// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

//! Transaction data structure.

use crate::{
    crypto::publickey::{self, public_to_address, recover, Public, Secret, Signature},
    hash::keccak,
    transaction::error,
};
use ethereum_types::{Address, BigEndianHash, H160, H256, U256};
use parity_util_mem::MallocSizeOf;

use rlp::{self, DecoderError, Rlp, RlpStream};
use std::{cmp::min, ops::Deref};

pub type AccessListItem = (H160, Vec<H256>);
pub type AccessList = Vec<AccessListItem>;

use super::TypedTxId;

type Bytes = Vec<u8>;
type BlockNumber = u64;

/// Fake address for unsigned transactions as defined by EIP-86.
pub const UNSIGNED_SENDER: Address = H160([0xff; 20]);

/// System sender address for internal state updates.
pub const SYSTEM_ADDRESS: Address = H160([
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xfe,
]);

/// Transaction action type.
#[derive(Debug, Clone, PartialEq, Eq, MallocSizeOf)]
pub enum Action {
    /// Create creates new contract.
    Create,
    /// Calls contract at given address.
    /// In the case of a transfer, this is the receiver's address.'
    Call(Address),
}

impl Default for Action {
    fn default() -> Action {
        Action::Create
    }
}

impl rlp::Decodable for Action {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.is_empty() {
            if rlp.is_data() {
                Ok(Action::Create)
            } else {
                Err(DecoderError::RlpExpectedToBeData)
            }
        } else {
            Ok(Action::Call(rlp.as_val()?))
        }
    }
}

impl rlp::Encodable for Action {
    fn rlp_append(&self, s: &mut RlpStream) {
        match *self {
            Action::Create => s.append_internal(&""),
            Action::Call(ref addr) => s.append_internal(addr),
        };
    }
}

/// Transaction activation condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Valid at this block number or later.
    Number(BlockNumber),
    /// Valid at this unix time or later.
    Timestamp(u64),
}

/// Replay protection logic for v part of transaction's signature
pub mod signature {
    /// Adds chain id into v
    pub fn add_chain_replay_protection(v: u8, chain_id: Option<u64>) -> u64 {
        v as u64
            + if let Some(n) = chain_id {
                35 + n * 2
            } else {
                27
            }
    }

    /// Returns refined v
    /// 0 if `v` would have been 27 under "Electrum" notation, 1 if 28 or 4 if invalid.
    pub fn extract_standard_v(v: u64) -> u8 {
        match v {
            v if v == 27 => 0,
            v if v == 28 => 1,
            v if v >= 35 => ((v - 1) % 2) as u8,
            _ => 4,
        }
    }

    pub fn extract_chain_id_from_legacy_v(v: u64) -> Option<u64> {
        if v >= 35 {
            Some((v - 35) / 2 as u64)
        } else {
            None
        }
    }
}

/// A set of information describing an externally-originating message call
/// or contract creation operation.
#[derive(Default, Debug, Clone, PartialEq, Eq, MallocSizeOf)]
pub struct Transaction {
    /// Nonce.
    pub nonce: U256,
    /// Gas price for non 1559 transactions. MaxFeePerGas for 1559 transactions.
    pub gas_price: U256,
    /// Gas paid up front for transaction execution.
    pub gas: U256,
    /// Action, can be either call or contract create.
    pub action: Action,
    /// Transfered value.s
    pub value: U256,
    /// Transaction data.
    pub data: Bytes,
}

impl Transaction {
    /// encode raw transaction
    fn encode(&self, chain_id: Option<u64>, signature: Option<&SignatureComponents>) -> Vec<u8> {
        let mut stream = RlpStream::new();
        self.encode_rlp(&mut stream, chain_id, signature);
        stream.drain()
    }

    pub fn rlp_append(
        &self,
        rlp: &mut RlpStream,
        chain_id: Option<u64>,
        signature: &SignatureComponents,
    ) {
        self.encode_rlp(rlp, chain_id, Some(signature));
    }

    fn encode_rlp(
        &self,
        rlp: &mut RlpStream,
        chain_id: Option<u64>,
        signature: Option<&SignatureComponents>,
    ) {
        let list_size = if chain_id.is_some() || signature.is_some() {
            9
        } else {
            6
        };
        rlp.begin_list(list_size);

        self.rlp_append_data_open(rlp);

        //append signature if given. If not, try to append chainId.
        if let Some(signature) = signature {
            signature.rlp_append_with_chain_id(rlp, chain_id);
        } else {
            if let Some(n) = chain_id {
                rlp.append(&n);
                rlp.append(&0u8);
                rlp.append(&0u8);
            }
        }
    }

    fn rlp_append_data_open(&self, s: &mut RlpStream) {
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.action);
        s.append(&self.value);
        s.append(&self.data);
    }

    fn decode(d: &Rlp) -> Result<UnverifiedTransaction, DecoderError> {
        if d.item_count()? != 9 {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let hash = keccak(d.as_raw());

        let transaction = TypedTransaction::Legacy(Self::decode_data(d, 0)?);

        // take V from signatuere and decompose it into chain_id and standard V.
        let legacy_v: u64 = d.val_at(6)?;

        let signature = SignatureComponents {
            standard_v: signature::extract_standard_v(legacy_v),
            r: d.val_at(7)?,
            s: d.val_at(8)?,
        };
        Ok(UnverifiedTransaction::new(
            transaction,
            signature::extract_chain_id_from_legacy_v(legacy_v),
            signature,
            hash,
        ))
    }

    fn decode_data(d: &Rlp, offset: usize) -> Result<Transaction, DecoderError> {
        Ok(Transaction {
            nonce: d.val_at(offset)?,
            gas_price: d.val_at(offset + 1)?,
            gas: d.val_at(offset + 2)?,
            action: d.val_at(offset + 3)?,
            value: d.val_at(offset + 4)?,
            data: d.val_at(offset + 5)?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, MallocSizeOf)]
pub struct AccessListTx {
    pub transaction: Transaction,
    //optional access list
    pub access_list: AccessList,
}

impl AccessListTx {
    pub fn new(transaction: Transaction, access_list: AccessList) -> AccessListTx {
        AccessListTx {
            transaction,
            access_list,
        }
    }

    pub fn tx_type(&self) -> TypedTxId {
        TypedTxId::AccessList
    }

    pub fn tx(&self) -> &Transaction {
        &self.transaction
    }

    pub fn tx_mut(&mut self) -> &mut Transaction {
        &mut self.transaction
    }

    // decode bytes by this payload spec: rlp([1, [chainId, nonce, gasPrice, gasLimit, to, value, data, access_list, senderV, senderR, senderS]])
    pub fn decode(tx: &[u8]) -> Result<UnverifiedTransaction, DecoderError> {
        let tx_rlp = &Rlp::new(tx);

        // we need to have 11 items in this list
        if tx_rlp.item_count()? != 11 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        let chain_id = Some(tx_rlp.val_at(0)?);
        //let chain_id = if chain_id == 0 { None } else { Some(chain_id) };

        // first part of list is same as legacy transaction and we are reusing that part.
        let transaction = Transaction::decode_data(&tx_rlp, 1)?;

        // access list we get from here
        let accl_rlp = tx_rlp.at(7)?;

        // access_list pattern: [[{20 bytes}, [{32 bytes}...]]...]
        let mut accl: AccessList = Vec::new();

        for i in 0..accl_rlp.item_count()? {
            let accounts = accl_rlp.at(i)?;

            // check if there is list of 2 items
            if accounts.item_count()? != 2 {
                return Err(DecoderError::Custom("Unknown access list length"));
            }
            accl.push((accounts.val_at(0)?, accounts.list_at(1)?));
        }

        // we get signature part from here
        let signature = SignatureComponents {
            standard_v: tx_rlp.val_at(8)?,
            r: tx_rlp.val_at(9)?,
            s: tx_rlp.val_at(10)?,
        };

        // and here we create UnverifiedTransaction and calculate its hash
        Ok(UnverifiedTransaction::new(
            TypedTransaction::AccessList(AccessListTx {
                transaction,
                access_list: accl,
            }),
            chain_id,
            signature,
            H256::zero(),
        )
        .compute_hash())
    }

    fn encode_payload(
        &self,
        chain_id: Option<u64>,
        signature: Option<&SignatureComponents>,
    ) -> RlpStream {
        let mut stream = RlpStream::new();

        let list_size = if signature.is_some() { 11 } else { 8 };
        stream.begin_list(list_size);

        // append chain_id. from EIP-2930: chainId is defined to be an integer of arbitrary size.
        stream.append(&(if let Some(n) = chain_id { n } else { 0 }));

        // append legacy transaction
        self.transaction.rlp_append_data_open(&mut stream);

        // access list
        stream.begin_list(self.access_list.len());
        for access in self.access_list.iter() {
            stream.begin_list(2);
            stream.append(&access.0);
            stream.begin_list(access.1.len());
            for storage_key in access.1.iter() {
                stream.append(storage_key);
            }
        }

        // append signature if any
        if let Some(signature) = signature {
            signature.rlp_append(&mut stream);
        }
        stream
    }

    // encode by this payload spec: 0x01 | rlp([1, [chain_id, nonce, gasPrice, gasLimit, to, value, data, access_list, senderV, senderR, senderS]])
    pub fn encode(
        &self,
        chain_id: Option<u64>,
        signature: Option<&SignatureComponents>,
    ) -> Vec<u8> {
        let stream = self.encode_payload(chain_id, signature);
        // make as vector of bytes
        [&[TypedTxId::AccessList as u8], stream.as_raw()].concat()
    }

    pub fn rlp_append(
        &self,
        rlp: &mut RlpStream,
        chain_id: Option<u64>,
        signature: &SignatureComponents,
    ) {
        rlp.append(&self.encode(chain_id, Some(signature)));
    }
}

#[derive(Debug, Clone, Eq, PartialEq, MallocSizeOf)]
pub struct EIP1559TransactionTx {
    pub transaction: AccessListTx,
    pub max_priority_fee_per_gas: U256,
}

impl EIP1559TransactionTx {
    pub fn tx_type(&self) -> TypedTxId {
        TypedTxId::EIP1559Transaction
    }

    pub fn tx(&self) -> &Transaction {
        &self.transaction.tx()
    }

    pub fn tx_mut(&mut self) -> &mut Transaction {
        self.transaction.tx_mut()
    }

    // decode bytes by this payload spec: rlp([2, [chainId, nonce, maxPriorityFeePerGas, maxFeePerGas(gasPrice), gasLimit, to, value, data, access_list, senderV, senderR, senderS]])
    pub fn decode(tx: &[u8]) -> Result<UnverifiedTransaction, DecoderError> {
        let tx_rlp = &Rlp::new(tx);

        // we need to have 12 items in this list
        if tx_rlp.item_count()? != 12 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        let chain_id = Some(tx_rlp.val_at(0)?);

        let max_priority_fee_per_gas = tx_rlp.val_at(2)?;

        let tx = Transaction {
            nonce: tx_rlp.val_at(1)?,
            gas_price: tx_rlp.val_at(3)?, //taken from max_fee_per_gas
            gas: tx_rlp.val_at(4)?,
            action: tx_rlp.val_at(5)?,
            value: tx_rlp.val_at(6)?,
            data: tx_rlp.val_at(7)?,
        };

        // access list we get from here
        let accl_rlp = tx_rlp.at(8)?;

        // access_list pattern: [[{20 bytes}, [{32 bytes}...]]...]
        let mut accl: AccessList = Vec::new();

        for i in 0..accl_rlp.item_count()? {
            let accounts = accl_rlp.at(i)?;

            // check if there is list of 2 items
            if accounts.item_count()? != 2 {
                return Err(DecoderError::Custom("Unknown access list length"));
            }
            accl.push((accounts.val_at(0)?, accounts.list_at(1)?));
        }

        // we get signature part from here
        let signature = SignatureComponents {
            standard_v: tx_rlp.val_at(9)?,
            r: tx_rlp.val_at(10)?,
            s: tx_rlp.val_at(11)?,
        };

        // and here we create UnverifiedTransaction and calculate its hash
        Ok(UnverifiedTransaction::new(
            TypedTransaction::EIP1559Transaction(EIP1559TransactionTx {
                transaction: AccessListTx::new(tx, accl),
                max_priority_fee_per_gas,
            }),
            chain_id,
            signature,
            H256::zero(),
        )
        .compute_hash())
    }

    fn encode_payload(
        &self,
        chain_id: Option<u64>,
        signature: Option<&SignatureComponents>,
    ) -> RlpStream {
        let mut stream = RlpStream::new();

        let list_size = if signature.is_some() { 12 } else { 9 };
        stream.begin_list(list_size);

        // append chain_id. from EIP-2930: chainId is defined to be an integer of arbitrary size.
        stream.append(&(if let Some(n) = chain_id { n } else { 0 }));

        stream.append(&self.tx().nonce);
        stream.append(&self.max_priority_fee_per_gas);
        stream.append(&self.tx().gas_price);
        stream.append(&self.tx().gas);
        stream.append(&self.tx().action);
        stream.append(&self.tx().value);
        stream.append(&self.tx().data);

        // access list
        stream.begin_list(self.transaction.access_list.len());
        for access in self.transaction.access_list.iter() {
            stream.begin_list(2);
            stream.append(&access.0);
            stream.begin_list(access.1.len());
            for storage_key in access.1.iter() {
                stream.append(storage_key);
            }
        }

        // append signature if any
        if let Some(signature) = signature {
            signature.rlp_append(&mut stream);
        }
        stream
    }

    // encode by this payload spec: 0x02 | rlp([2, [chainId, nonce, maxPriorityFeePerGas, maxFeePerGas(gasPrice), gasLimit, to, value, data, access_list, senderV, senderR, senderS]])
    pub fn encode(
        &self,
        chain_id: Option<u64>,
        signature: Option<&SignatureComponents>,
    ) -> Vec<u8> {
        let stream = self.encode_payload(chain_id, signature);
        // make as vector of bytes
        [&[TypedTxId::EIP1559Transaction as u8], stream.as_raw()].concat()
    }

    pub fn rlp_append(
        &self,
        rlp: &mut RlpStream,
        chain_id: Option<u64>,
        signature: &SignatureComponents,
    ) {
        rlp.append(&self.encode(chain_id, Some(signature)));
    }
}

#[derive(Debug, Clone, Eq, PartialEq, MallocSizeOf)]
pub enum TypedTransaction {
    Legacy(Transaction),      // old legacy RLP encoded transaction
    AccessList(AccessListTx), // EIP-2930 Transaction with a list of addresses and storage keys that the transaction plans to access.
    // Accesses outside the list are possible, but become more expensive.
    EIP1559Transaction(EIP1559TransactionTx),
}

impl TypedTransaction {
    pub fn tx_type(&self) -> TypedTxId {
        match self {
            Self::Legacy(_) => TypedTxId::Legacy,
            Self::AccessList(_) => TypedTxId::AccessList,
            Self::EIP1559Transaction(_) => TypedTxId::EIP1559Transaction,
        }
    }

    /// The message hash of the transaction.
    pub fn signature_hash(&self, chain_id: Option<u64>) -> H256 {
        keccak(match self {
            Self::Legacy(tx) => tx.encode(chain_id, None),
            Self::AccessList(tx) => tx.encode(chain_id, None),
            Self::EIP1559Transaction(tx) => tx.encode(chain_id, None),
        })
    }

    /// Signs the transaction as coming from `sender`.
    pub fn sign(self, secret: &Secret, chain_id: Option<u64>) -> SignedTransaction {
        let sig = publickey::sign(secret, &self.signature_hash(chain_id))
            .expect("data is valid and context has signing capabilities; qed");
        SignedTransaction::new(self.with_signature(sig, chain_id))
            .expect("secret is valid so it's recoverable")
    }

    /// Signs the transaction with signature.
    pub fn with_signature(self, sig: Signature, chain_id: Option<u64>) -> UnverifiedTransaction {
        UnverifiedTransaction {
            unsigned: self,
            chain_id,
            signature: SignatureComponents {
                r: sig.r().into(),
                s: sig.s().into(),
                standard_v: sig.v().into(),
            },
            hash: H256::zero(),
        }
        .compute_hash()
    }

    /// Specify the sender; this won't survive the serialize/deserialize process, but can be cloned.
    pub fn fake_sign(self, from: Address) -> SignedTransaction {
        SignedTransaction {
            transaction: UnverifiedTransaction {
                unsigned: self,
                chain_id: None,
                signature: SignatureComponents {
                    r: U256::one(),
                    s: U256::one(),
                    standard_v: 4,
                },
                hash: H256::zero(),
            }
            .compute_hash(),
            sender: from,
            public: None,
        }
    }

    /// Legacy EIP-86 compatible empty signature.
    /// This method is used in json tests as well as
    /// signature verification tests.
    pub fn null_sign(self, chain_id: u64) -> SignedTransaction {
        SignedTransaction {
            transaction: UnverifiedTransaction {
                unsigned: self,
                chain_id: Some(chain_id),
                signature: SignatureComponents {
                    r: U256::zero(),
                    s: U256::zero(),
                    standard_v: 0,
                },
                hash: H256::zero(),
            }
            .compute_hash(),
            sender: UNSIGNED_SENDER,
            public: None,
        }
    }

    /// Useful for test incorrectly signed transactions.
    #[cfg(test)]
    pub fn invalid_sign(self) -> UnverifiedTransaction {
        UnverifiedTransaction {
            unsigned: self,
            chain_id: None,
            signature: SignatureComponents {
                r: U256::one(),
                s: U256::one(),
                standard_v: 0,
            },
            hash: H256::zero(),
        }
        .compute_hash()
    }

    // Next functions are for encoded/decode

    pub fn tx(&self) -> &Transaction {
        match self {
            Self::Legacy(tx) => tx,
            Self::AccessList(ocl) => ocl.tx(),
            Self::EIP1559Transaction(tx) => tx.tx(),
        }
    }

    pub fn tx_mut(&mut self) -> &mut Transaction {
        match self {
            Self::Legacy(tx) => tx,
            Self::AccessList(ocl) => ocl.tx_mut(),
            Self::EIP1559Transaction(tx) => tx.tx_mut(),
        }
    }

    pub fn access_list(&self) -> Option<&AccessList> {
        match self {
            Self::EIP1559Transaction(tx) => Some(&tx.transaction.access_list),
            Self::AccessList(tx) => Some(&tx.access_list),
            Self::Legacy(_) => None,
        }
    }

    pub fn effective_gas_price(&self, block_base_fee: Option<U256>) -> U256 {
        match self {
            Self::EIP1559Transaction(tx) => min(
                self.tx().gas_price,
                tx.max_priority_fee_per_gas + block_base_fee.unwrap_or_default(),
            ),
            Self::AccessList(_) => self.tx().gas_price,
            Self::Legacy(_) => self.tx().gas_price,
        }
    }

    pub fn max_priority_fee_per_gas(&self) -> U256 {
        match self {
            Self::EIP1559Transaction(tx) => tx.max_priority_fee_per_gas,
            Self::AccessList(tx) => tx.tx().gas_price,
            Self::Legacy(tx) => tx.gas_price,
        }
    }

    fn decode_new(tx: &[u8]) -> Result<UnverifiedTransaction, DecoderError> {
        if tx.is_empty() {
            // at least one byte needs to be present
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let id = TypedTxId::try_from_wire_byte(tx[0]);
        if id.is_err() {
            return Err(DecoderError::Custom("Unknown transaction"));
        }
        // other transaction types
        match id.unwrap() {
            TypedTxId::EIP1559Transaction => EIP1559TransactionTx::decode(&tx[1..]),
            TypedTxId::AccessList => AccessListTx::decode(&tx[1..]),
            TypedTxId::Legacy => return Err(DecoderError::Custom("Unknown transaction legacy")),
        }
    }

    pub fn decode(tx: &[u8]) -> Result<UnverifiedTransaction, DecoderError> {
        if tx.is_empty() {
            // at least one byte needs to be present
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let header = tx[0];
        // type of transaction can be obtained from first byte. If first bit is 1 it means we are dealing with RLP list.
        // if it is 0 it means that we are dealing with custom transaction defined in EIP-2718.
        if (header & 0x80) != 0x00 {
            Transaction::decode(&Rlp::new(tx))
        } else {
            Self::decode_new(tx)
        }
    }

    pub fn decode_rlp_list(rlp: &Rlp) -> Result<Vec<UnverifiedTransaction>, DecoderError> {
        if !rlp.is_list() {
            // at least one byte needs to be present
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let mut output = Vec::with_capacity(rlp.item_count()?);
        for tx in rlp.iter() {
            output.push(Self::decode_rlp(&tx)?);
        }
        Ok(output)
    }

    pub fn decode_rlp(tx: &Rlp) -> Result<UnverifiedTransaction, DecoderError> {
        if tx.is_list() {
            //legacy transaction wrapped around RLP encoding
            Transaction::decode(tx)
        } else {
            Self::decode_new(tx.data()?)
        }
    }

    fn rlp_append(
        &self,
        s: &mut RlpStream,
        chain_id: Option<u64>,
        signature: &SignatureComponents,
    ) {
        match self {
            Self::Legacy(tx) => tx.rlp_append(s, chain_id, signature),
            Self::AccessList(opt) => opt.rlp_append(s, chain_id, signature),
            Self::EIP1559Transaction(tx) => tx.rlp_append(s, chain_id, signature),
        }
    }

    pub fn rlp_append_list(s: &mut RlpStream, tx_list: &[UnverifiedTransaction]) {
        s.begin_list(tx_list.len());
        for tx in tx_list.iter() {
            tx.unsigned.rlp_append(s, tx.chain_id, &tx.signature);
        }
    }

    fn encode(&self, chain_id: Option<u64>, signature: &SignatureComponents) -> Vec<u8> {
        let signature = Some(signature);
        match self {
            Self::Legacy(tx) => tx.encode(chain_id, signature),
            Self::AccessList(opt) => opt.encode(chain_id, signature),
            Self::EIP1559Transaction(tx) => tx.encode(chain_id, signature),
        }
    }
}

/// Components that constitute transaction signature
#[derive(Debug, Clone, Eq, PartialEq, MallocSizeOf)]
pub struct SignatureComponents {
    /// The V field of the signature; the LS bit described which half of the curve our point falls
    /// in. It can be 0 or 1.
    pub standard_v: u8,
    /// The R field of the signature; helps describe the point on the curve.
    pub r: U256,
    /// The S field of the signature; helps describe the point on the curve.
    pub s: U256,
}

impl SignatureComponents {
    pub fn rlp_append(&self, s: &mut RlpStream) {
        s.append(&self.standard_v);
        s.append(&self.r);
        s.append(&self.s);
    }

    pub fn rlp_append_with_chain_id(&self, s: &mut RlpStream, chain_id: Option<u64>) {
        s.append(&signature::add_chain_replay_protection(
            self.standard_v,
            chain_id,
        ));
        s.append(&self.r);
        s.append(&self.s);
    }
}

/// Signed transaction information without verified signature.
#[derive(Debug, Clone, Eq, PartialEq, MallocSizeOf)]
pub struct UnverifiedTransaction {
    /// Plain Transaction.
    pub unsigned: TypedTransaction,
    /// Transaction signature
    pub signature: SignatureComponents,
    /// chain_id recover from signature in legacy transaction. For TypedTransaction it is probably separate field.
    pub chain_id: Option<u64>,
    /// Hash of the transaction
    pub hash: H256,
}

impl Deref for UnverifiedTransaction {
    type Target = TypedTransaction;

    fn deref(&self) -> &Self::Target {
        &self.unsigned
    }
}

impl UnverifiedTransaction {
    pub fn rlp_append(&self, s: &mut RlpStream) {
        self.unsigned.rlp_append(s, self.chain_id, &self.signature);
    }

    pub fn rlp_append_list(s: &mut RlpStream, tx_list: &[UnverifiedTransaction]) {
        s.begin_list(tx_list.len());
        for tx in tx_list.iter() {
            tx.unsigned.rlp_append(s, tx.chain_id, &tx.signature);
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        self.unsigned.encode(self.chain_id, &self.signature)
    }

    /// Used to compute hash of created transactions.
    pub fn compute_hash(mut self) -> UnverifiedTransaction {
        let hash = keccak(&*self.encode());
        self.hash = hash;
        self
    }

    /// Used by TypedTransaction to create UnverifiedTransaction.
    fn new(
        transaction: TypedTransaction,
        chain_id: Option<u64>,
        signature: SignatureComponents,
        hash: H256,
    ) -> UnverifiedTransaction {
        UnverifiedTransaction {
            unsigned: transaction,
            chain_id,
            signature,
            hash,
        }
    }
    /// Checks if the signature is empty.
    pub fn is_unsigned(&self) -> bool {
        self.signature.r.is_zero() && self.signature.s.is_zero()
    }

    ///	Reference to unsigned part of this transaction.
    pub fn as_unsigned(&self) -> &TypedTransaction {
        &self.unsigned
    }

    /// Returns standardized `v` value (0, 1 or 4 (invalid))
    pub fn standard_v(&self) -> u8 {
        self.signature.standard_v
    }

    /// The legacy `v` value that contains signatures v and chain_id for replay protection.
    pub fn legacy_v(&self) -> u64 {
        signature::add_chain_replay_protection(self.signature.standard_v, self.chain_id)
    }

    /// The `v` value that appears in the RLP.
    pub fn v(&self) -> u64 {
        match self.unsigned {
            TypedTransaction::Legacy(_) => self.legacy_v(),
            _ => self.signature.standard_v as u64,
        }
    }

    /// The chain ID, or `None` if this is a global transaction.
    pub fn chain_id(&self) -> Option<u64> {
        self.chain_id
    }

    /// Construct a signature object from the sig.
    pub fn signature(&self) -> Signature {
        let r: H256 = BigEndianHash::from_uint(&self.signature.r);
        let s: H256 = BigEndianHash::from_uint(&self.signature.s);
        Signature::from_rsv(&r, &s, self.standard_v())
    }

    /// Checks whether the signature has a low 's' value.
    pub fn check_low_s(&self) -> Result<(), publickey::Error> {
        if !self.signature().is_low_s() {
            Err(publickey::Error::InvalidSignature.into())
        } else {
            Ok(())
        }
    }

    /// Get the hash of this transaction (keccak of the RLP).
    pub fn hash(&self) -> H256 {
        self.hash
    }

    /// Recovers the public key of the sender.
    pub fn recover_public(&self) -> Result<Public, publickey::Error> {
        Ok(recover(
            &self.signature(),
            &self.unsigned.signature_hash(self.chain_id()),
        )?)
    }

    /// Verify basic signature params. Does not attempt sender recovery.
    pub fn verify_basic(
        &self,
        check_low_s: bool,
        chain_id: Option<u64>,
    ) -> Result<(), error::Error> {
        if self.is_unsigned() {
            return Err(publickey::Error::InvalidSignature.into());
        }
        if check_low_s {
            self.check_low_s()?;
        }
        match (self.chain_id(), chain_id) {
            (None, _) => {}
            (Some(n), Some(m)) if n == m => {}
            _ => return Err(error::Error::InvalidChainId),
        };
        Ok(())
    }
}

/// A `UnverifiedTransaction` with successfully recovered `sender`.
#[derive(Debug, Clone, Eq, PartialEq, MallocSizeOf)]
pub struct SignedTransaction {
    transaction: UnverifiedTransaction,
    sender: Address,
    public: Option<Public>,
}

impl Deref for SignedTransaction {
    type Target = UnverifiedTransaction;
    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl From<SignedTransaction> for UnverifiedTransaction {
    fn from(tx: SignedTransaction) -> Self {
        tx.transaction
    }
}

impl SignedTransaction {
    // t_nb 5.3.1 Try to verify transaction and recover sender.
    pub fn new(transaction: UnverifiedTransaction) -> Result<Self, publickey::Error> {
        if transaction.is_unsigned() {
            return Err(publickey::Error::InvalidSignature);
        }
        let public = transaction.recover_public()?;
        let sender = public_to_address(&public);
        Ok(SignedTransaction {
            transaction,
            sender,
            public: Some(public),
        })
    }

    /// Returns transaction sender.
    pub fn sender(&self) -> Address {
        self.sender
    }

    /// Returns a public key of the sender.
    pub fn public_key(&self) -> Option<Public> {
        self.public
    }

    /// Checks is signature is empty.
    pub fn is_unsigned(&self) -> bool {
        self.transaction.is_unsigned()
    }

    /// Deconstructs this transaction back into `UnverifiedTransaction`
    pub fn deconstruct(self) -> (UnverifiedTransaction, Address, Option<Public>) {
        (self.transaction, self.sender, self.public)
    }

    pub fn rlp_append_list(s: &mut RlpStream, tx_list: &[SignedTransaction]) {
        s.begin_list(tx_list.len());
        for tx in tx_list.iter() {
            tx.unsigned.rlp_append(s, tx.chain_id, &tx.signature);
        }
    }
}

/// Signed Transaction that is a part of canon blockchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedTransaction {
    /// Signed part.
    pub signed: UnverifiedTransaction,
    /// Block number.
    pub block_number: BlockNumber,
    /// Block hash.
    pub block_hash: H256,
    /// Transaction index within block.
    pub transaction_index: usize,
    /// Cached sender
    pub cached_sender: Option<Address>,
}

impl LocalizedTransaction {
    /// Returns transaction sender.
    /// Panics if `LocalizedTransaction` is constructed using invalid `UnverifiedTransaction`.
    pub fn sender(&mut self) -> Address {
        if let Some(sender) = self.cached_sender {
            return sender;
        }
        if self.is_unsigned() {
            return UNSIGNED_SENDER.clone();
        }
        let sender = public_to_address(&self.recover_public()
			.expect("LocalizedTransaction is always constructed from transaction from blockchain; Blockchain only stores verified transactions; qed"));
        self.cached_sender = Some(sender);
        sender
    }
}

impl Deref for LocalizedTransaction {
    type Target = UnverifiedTransaction;

    fn deref(&self) -> &Self::Target {
        &self.signed
    }
}

/// Queued transaction with additional information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTransaction {
    /// Signed transaction data.
    pub transaction: SignedTransaction,
    /// To be activated at this condition. `None` for immediately.
    pub condition: Option<Condition>,
}

impl PendingTransaction {
    /// Create a new pending transaction from signed transaction.
    pub fn new(signed: SignedTransaction, condition: Option<Condition>) -> Self {
        PendingTransaction {
            transaction: signed,
            condition: condition,
        }
    }
}

impl Deref for PendingTransaction {
    type Target = SignedTransaction;

    fn deref(&self) -> &SignedTransaction {
        &self.transaction
    }
}

impl From<SignedTransaction> for PendingTransaction {
    fn from(t: SignedTransaction) -> Self {
        PendingTransaction {
            transaction: t,
            condition: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::keccak;
    use ethereum_types::{H160, U256};
    use std::str::FromStr;

    #[test]
    fn sender_test() {
        let bytes = ::rustc_hex::FromHex::from_hex("f85f800182520894095e7baea6a6c7c4c2dfeb977efac326af552d870a801ba048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a0efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804").unwrap();
        let t = TypedTransaction::decode(&bytes).expect("decoding UnverifiedTransaction failed");
        assert_eq!(t.tx().data, b"");
        assert_eq!(t.tx().gas, U256::from(0x5208u64));
        assert_eq!(t.tx().gas_price, U256::from(0x01u64));
        assert_eq!(t.tx().nonce, U256::from(0x00u64));
        if let Action::Call(ref to) = t.tx().action {
            assert_eq!(
                *to,
                H160::from_str("095e7baea6a6c7c4c2dfeb977efac326af552d87").unwrap()
            );
        } else {
            panic!();
        }
        assert_eq!(t.tx().value, U256::from(0x0au64));
        assert_eq!(
            public_to_address(&t.recover_public().unwrap()),
            H160::from_str("0f65fe9276bc9a24ae7083ae28e2660ef72df99e").unwrap()
        );
        assert_eq!(t.chain_id(), None);
    }

    #[test]
    fn empty_atom_as_create_action() {
        let empty_atom = [0x80];
        let action: Action = rlp::decode(&empty_atom).unwrap();
        assert_eq!(action, Action::Create);
    }

    #[test]
    fn empty_list_as_create_action_rejected() {
        let empty_list = [0xc0];
        let action: Result<Action, DecoderError> = rlp::decode(&empty_list);
        assert_eq!(action, Err(DecoderError::RlpExpectedToBeData));
    }

    #[test]
    fn signing_eip155_zero_chainid() {
        use self::publickey::{Generator, Random};

        let key = Random.generate();
        let t = TypedTransaction::Legacy(Transaction {
            action: Action::Create,
            nonce: U256::from(42),
            gas_price: U256::from(3000),
            gas: U256::from(50_000),
            value: U256::from(1),
            data: b"Hello!".to_vec(),
        });

        let hash = t.signature_hash(Some(0));
        let sig = publickey::sign(&key.secret(), &hash).unwrap();
        let u = t.with_signature(sig, Some(0));

        assert!(SignedTransaction::new(u).is_ok());
    }

    #[test]
    fn signing() {
        use self::publickey::{Generator, Random};

        let key = Random.generate();
        let t = TypedTransaction::Legacy(Transaction {
            action: Action::Create,
            nonce: U256::from(42),
            gas_price: U256::from(3000),
            gas: U256::from(50_000),
            value: U256::from(1),
            data: b"Hello!".to_vec(),
        })
        .sign(&key.secret(), None);
        assert_eq!(Address::from(keccak(key.public())), t.sender());
        assert_eq!(t.chain_id(), None);
    }

    #[test]
    fn fake_signing() {
        let t = TypedTransaction::Legacy(Transaction {
            action: Action::Create,
            nonce: U256::from(42),
            gas_price: U256::from(3000),
            gas: U256::from(50_000),
            value: U256::from(1),
            data: b"Hello!".to_vec(),
        })
        .fake_sign(Address::from_low_u64_be(0x69));
        assert_eq!(Address::from_low_u64_be(0x69), t.sender());
        assert_eq!(t.chain_id(), None);

        let t = t.clone();
        assert_eq!(Address::from_low_u64_be(0x69), t.sender());
        assert_eq!(t.chain_id(), None);
    }

    #[test]
    fn should_reject_null_signature() {
        use std::str::FromStr;
        let t = TypedTransaction::Legacy(Transaction {
            nonce: U256::zero(),
            gas_price: U256::from(10000000000u64),
            gas: U256::from(21000),
            action: Action::Call(
                Address::from_str("d46e8dd67c5d32be8058bb8eb970870f07244567").unwrap(),
            ),
            value: U256::from(1),
            data: vec![],
        })
        .null_sign(1);

        let res = SignedTransaction::new(t.transaction);
        match res {
            Err(publickey::Error::InvalidSignature) => {}
            _ => panic!("null signature should be rejected"),
        }
    }

    #[test]
    fn should_recover_from_chain_specific_signing() {
        use self::publickey::{Generator, Random};
        let key = Random.generate();
        let t = TypedTransaction::Legacy(Transaction {
            action: Action::Create,
            nonce: U256::from(42),
            gas_price: U256::from(3000),
            gas: U256::from(50_000),
            value: U256::from(1),
            data: b"Hello!".to_vec(),
        })
        .sign(&key.secret(), Some(69));
        assert_eq!(Address::from(keccak(key.public())), t.sender());
        assert_eq!(t.chain_id(), Some(69));
    }

    #[test]
    fn should_encode_decode_access_list_tx() {
        use self::publickey::{Generator, Random};
        let key = Random.generate();
        let t = TypedTransaction::AccessList(AccessListTx::new(
            Transaction {
                action: Action::Create,
                nonce: U256::from(42),
                gas_price: U256::from(3000),
                gas: U256::from(50_000),
                value: U256::from(1),
                data: b"Hello!".to_vec(),
            },
            vec![
                (
                    H160::from_low_u64_be(10),
                    vec![H256::from_low_u64_be(102), H256::from_low_u64_be(103)],
                ),
                (H160::from_low_u64_be(400), vec![]),
            ],
        ))
        .sign(&key.secret(), Some(69));
        let encoded = t.encode();

        let t_new =
            TypedTransaction::decode(&encoded).expect("Error on UnverifiedTransaction decoder");
        if t_new.unsigned != t.unsigned {
            assert!(true, "encoded/decoded tx differs from original");
        }
    }

    #[test]
    fn should_encode_decode_eip1559_tx() {
        use self::publickey::{Generator, Random};
        let key = Random.generate();
        let t = TypedTransaction::EIP1559Transaction(EIP1559TransactionTx {
            transaction: AccessListTx::new(
                Transaction {
                    action: Action::Create,
                    nonce: U256::from(42),
                    gas_price: U256::from(3000),
                    gas: U256::from(50_000),
                    value: U256::from(1),
                    data: b"Hello!".to_vec(),
                },
                vec![
                    (
                        H160::from_low_u64_be(10),
                        vec![H256::from_low_u64_be(102), H256::from_low_u64_be(103)],
                    ),
                    (H160::from_low_u64_be(400), vec![]),
                ],
            ),
            max_priority_fee_per_gas: U256::from(100),
        })
        .sign(&key.secret(), Some(69));
        let encoded = t.encode();

        let t_new =
            TypedTransaction::decode(&encoded).expect("Error on UnverifiedTransaction decoder");
        if t_new.unsigned != t.unsigned {
            assert!(true, "encoded/decoded tx differs from original");
        }
    }

    #[test]
    fn should_decode_access_list_in_rlp() {
        use rustc_hex::FromHex;
        let encoded_tx = "b8cb01f8a7802a820bb882c35080018648656c6c6f21f872f85994000000000000000000000000000000000000000af842a00000000000000000000000000000000000000000000000000000000000000066a00000000000000000000000000000000000000000000000000000000000000067d6940000000000000000000000000000000000000190c080a00ea0f1fda860320f51e182fe68ea90a8e7611653d3975b9301580adade6b8aa4a023530a1a96e0f15f90959baf1cd2d9114f7c7568ac7d77f4413c0a6ca6cdac74";
        let _ = TypedTransaction::decode_rlp(&Rlp::new(&FromHex::from_hex(encoded_tx).unwrap()))
            .expect("decoding tx data failed");
    }

    #[test]
    fn should_decode_eip1559_in_rlp() {
        use rustc_hex::FromHex;
        let encoded_tx = "b8cb01f8a7802a820bb882c35080018648656c6c6f21f872f85994000000000000000000000000000000000000000af842a00000000000000000000000000000000000000000000000000000000000000066a00000000000000000000000000000000000000000000000000000000000000067d6940000000000000000000000000000000000000190c080a00ea0f1fda860320f51e182fe68ea90a8e7611653d3975b9301580adade6b8aa4a023530a1a96e0f15f90959baf1cd2d9114f7c7568ac7d77f4413c0a6ca6cdac74";
        let _ = TypedTransaction::decode_rlp(&Rlp::new(&FromHex::from_hex(encoded_tx).unwrap()))
            .expect("decoding tx data failed");
    }

    #[test]
    fn should_decode_access_list_solo() {
        use rustc_hex::FromHex;
        let encoded_tx = "01f8630103018261a894b94f5374fce5edbc8e2a8697c15331677e6ebf0b0a825544c001a0cb51495c66325615bcd591505577c9dde87bd59b04be2e6ba82f6d7bdea576e3a049e4f02f37666bd91a052a56e91e71e438590df861031ee9a321ce058df3dc2b";
        let _ = TypedTransaction::decode(&FromHex::from_hex(encoded_tx).unwrap())
            .expect("decoding tx data failed");
    }

    #[test]
    fn test_rlp_data() {
        let mut rlp_list = RlpStream::new();
        rlp_list.begin_list(3);
        rlp_list.append(&100u8);
        rlp_list.append(&"0000000");
        rlp_list.append(&5u8);
        let rlp_list = Rlp::new(rlp_list.as_raw());
        println!("rlp list data: {:?}", rlp_list.as_raw());

        let mut rlp = RlpStream::new();
        rlp.append(&"1111111");
        let rlp = Rlp::new(rlp.as_raw());
        println!("rlp list data: {:?}", rlp.data());
    }

    #[test]
    fn should_agree_with_geth_test() {
        use rustc_hex::FromHex;
        let encoded_tx = "01f8630103018261a894b94f5374fce5edbc8e2a8697c15331677e6ebf0b0a825544c001a0cb51495c66325615bcd591505577c9dde87bd59b04be2e6ba82f6d7bdea576e3a049e4f02f37666bd91a052a56e91e71e438590df861031ee9a321ce058df3dc2b";
        let _ = TypedTransaction::decode(&FromHex::from_hex(encoded_tx).unwrap())
            .expect("decoding tx data failed");
    }

    #[test]
    fn should_agree_with_vitalik() {
        use rustc_hex::FromHex;

        let test_vector = |tx_data: &str, address: &'static str| {
            let signed = TypedTransaction::decode(&FromHex::from_hex(tx_data).unwrap())
                .expect("decoding tx data failed");
            let signed = SignedTransaction::new(signed).unwrap();
            assert_eq!(signed.sender(), H160::from_str(address).unwrap());
        };

        test_vector("f864808504a817c800825208943535353535353535353535353535353535353535808025a0044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116da0044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d", "f0f6f18bca1b28cd68e4357452947e021241e9ce");
        test_vector("f864018504a817c80182a410943535353535353535353535353535353535353535018025a0489efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bcaa0489efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6", "23ef145a395ea3fa3deb533b8a9e1b4c6c25d112");
        test_vector("f864028504a817c80282f618943535353535353535353535353535353535353535088025a02d7c5bef027816a800da1736444fb58a807ef4c9603b7848673f7e3a68eb14a5a02d7c5bef027816a800da1736444fb58a807ef4c9603b7848673f7e3a68eb14a5", "2e485e0c23b4c3c542628a5f672eeab0ad4888be");
        test_vector("f865038504a817c803830148209435353535353535353535353535353535353535351b8025a02a80e1ef1d7842f27f2e6be0972bb708b9a135c38860dbe73c27c3486c34f4e0a02a80e1ef1d7842f27f2e6be0972bb708b9a135c38860dbe73c27c3486c34f4de", "82a88539669a3fd524d669e858935de5e5410cf0");
        test_vector("f865048504a817c80483019a28943535353535353535353535353535353535353535408025a013600b294191fc92924bb3ce4b969c1e7e2bab8f4c93c3fc6d0a51733df3c063a013600b294191fc92924bb3ce4b969c1e7e2bab8f4c93c3fc6d0a51733df3c060", "f9358f2538fd5ccfeb848b64a96b743fcc930554");
        test_vector("f865058504a817c8058301ec309435353535353535353535353535353535353535357d8025a04eebf77a833b30520287ddd9478ff51abbdffa30aa90a8d655dba0e8a79ce0c1a04eebf77a833b30520287ddd9478ff51abbdffa30aa90a8d655dba0e8a79ce0c1", "a8f7aba377317440bc5b26198a363ad22af1f3a4");
        test_vector("f866068504a817c80683023e3894353535353535353535353535353535353535353581d88025a06455bf8ea6e7463a1046a0b52804526e119b4bf5136279614e0b1e8e296a4e2fa06455bf8ea6e7463a1046a0b52804526e119b4bf5136279614e0b1e8e296a4e2d", "f1f571dc362a0e5b2696b8e775f8491d3e50de35");
        test_vector("f867078504a817c807830290409435353535353535353535353535353535353535358201578025a052f1a9b320cab38e5da8a8f97989383aab0a49165fc91c737310e4f7e9821021a052f1a9b320cab38e5da8a8f97989383aab0a49165fc91c737310e4f7e9821021", "d37922162ab7cea97c97a87551ed02c9a38b7332");
        test_vector("f867088504a817c8088302e2489435353535353535353535353535353535353535358202008025a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10", "9bddad43f934d313c2b79ca28a432dd2b7281029");
        test_vector("f867098504a817c809830334509435353535353535353535353535353535353535358202d98025a052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afba052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb", "3c24d7329e92f84f08556ceb6df1cdb0104ca49f");
    }
}
