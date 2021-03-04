use ethereum_types::{Address, U256};
use v1::types::Transaction;

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Serialize)]
#[serde()]
pub enum EqFilterArgument<T: Eq> {
    eq(T),
    Nil,
}

impl<T: Eq> Default for EqFilterArgument<T> {
    fn default() -> Self { Self::Nil }
}

impl<T: Eq> EqFilterArgument<T> {
    fn matches(&self, value: &T) -> bool {
        match self {
            Self::eq(expected) => value == expected,
            Self::Nil => true,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Serialize)]
#[serde()]
pub enum ValueFilterArgument {
    eq(U256),
    lt(U256),
    gt(U256),
    Nil,
}

impl Default for ValueFilterArgument {
    fn default() -> Self { Self::Nil }
}

impl ValueFilterArgument {
    fn matches(&self, value: &U256) -> bool {
        match self {
            ValueFilterArgument::eq(expected) => value == expected,
            ValueFilterArgument::lt(threshold) => value < threshold,
            ValueFilterArgument::gt(threshold) => value > threshold,
            ValueFilterArgument::Nil => true,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TransactionFilter {
    from: EqFilterArgument<Address>,
    to: EqFilterArgument<Option<Address>>,
    gas: ValueFilterArgument,
    gas_price: ValueFilterArgument,
    value: ValueFilterArgument,
    nonce: ValueFilterArgument,
}

impl TransactionFilter {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        self.from.matches(&transaction.from)
            && self.to.matches(&transaction.to)
            && self.gas.matches(&transaction.gas)
            && self.gas_price.matches(&transaction.gas_price)
            && self.nonce.matches(&transaction.nonce)
            && self.value.matches(&transaction.value)
    }
}
