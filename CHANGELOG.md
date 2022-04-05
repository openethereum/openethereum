## OpenEthereum v3.3.5

Enhancements:
* Support for POSDAO contract hardfork (#633)
* Update rpc server (#619)

## OpenEthereum v3.3.4

Enhancements:
* EIP-712: Update logos and rewrite type parser (now builds on Rust 1.58.1) (#463)
* Handling of incoming transactions with maxFeePerGas lower than current baseFee (#604)
* Update transaction replacement (#607)

## OpenEthereum v3.3.3

Enhancements:
* Implement eip-3607 (#593)

Bug fixes:
* Add type field for legacy transactions in RPC calls (#580)
* Makes eth_mining to return False if not is not allowed to seal (#581)
* Made nodes data concatenate as RLP sequences instead of bytes (#598)

## OpenEthereum v3.3.2

Enhancements:
* London hardfork block: Sokol (24114400)

Bug fixes:
* Fix for maxPriorityFeePerGas overflow

## OpenEthereum v3.3.1

Enhancements:
* Add eth_maxPriorityFeePerGas implementation (#570)
* Add a bootnode for Kovan

Bug fixes:
* Fix for modexp overflow in debug mode (#578)

## OpenEthereum v3.3.0

Enhancements:
* Add `validateServiceTransactionsTransition` spec option to be able to enable additional checking of zero gas price transactions by block verifier

## OpenEthereum v3.3.0-rc.15

* Revert eip1559BaseFeeMinValue activation on xDai at London hardfork block

## OpenEthereum v3.3.0-rc.14

Enhancements:
* Add eip1559BaseFeeMinValue and eip1559BaseFeeMinValueTransition spec options
* Activate eip1559BaseFeeMinValue on xDai at London hardfork block (19040000), set it to 20 GWei
* Activate eip1559BaseFeeMinValue on POA Core at block 24199500 (November 8, 2021), set it to 10 GWei
* Delay difficulty bomb to June 2022 for Ethereum Mainnet (EIP-4345)

## OpenEthereum v3.3.0-rc.13

Enhancements:
* London hardfork block: POA Core (24090200)

## OpenEthereum v3.3.0-rc.12

Enhancements:
* London hardfork block: xDai (19040000)

## OpenEthereum v3.3.0-rc.11

Bug fixes:
* Ignore GetNodeData requests only for non-AuRa chains

## OpenEthereum v3.3.0-rc.10

Enhancements:
* Add eip1559FeeCollector and eip1559FeeCollectorTransition spec options

## OpenEthereum v3.3.0-rc.9

Bug fixes:
* Add service transactions support for EIP-1559
* Fix MinGasPrice config option for POSDAO and EIP-1559

Enhancements:
* min_gas_price becomes min_effective_priority_fee
* added version 4 for TxPermission contract

## OpenEthereum v3.3.0-rc.8

Bug fixes:
* Ignore GetNodeData requests (#519)

## OpenEthereum v3.3.0-rc.7

Bug fixes:
* GetPooledTransactions is sent in invalid form (wrong packet id)

## OpenEthereum v3.3.0-rc.6

Enhancements:
* London hardfork block: kovan (26741100) (#502)

## OpenEthereum v3.3.0-rc.4

Enhancements:
* London hardfork block: mainnet (12,965,000) (#475)
* Support for eth/66 protocol version (#465)
* Bump ethereum/tests to v9.0.3
* Add eth_feeHistory

Bug fixes:
* GetNodeData from eth63 is missing (#466)
* Effective gas price not omitting (#477)
* London support in openethereum-evm (#479)
* gasPrice is required field for Transaction object (#481)

## OpenEthereum v3.3.0-rc.3

Bug fixes:
* Add effective_gas_price to eth_getTransactionReceipt #445 (#450)
* Update eth_gasPrice to support EIP-1559 #449 (#458)
* eth_estimateGas returns "Requires higher than upper limit of X" after London Ropsten Hard Fork #459 (#460)

## OpenEthereum v3.3.0-rc.2

Enhancements:
* EIP-1559: Fee market change for ETH 1.0 chain
* EIP-3198: BASEFEE opcode
* EIP-3529: Reduction in gas refunds
* EIP-3541: Reject new contracts starting with the 0xEF byte
* Delay difficulty bomb to December 2021 (EIP-3554)
* London hardfork blocks: goerli (5,062,605), rinkeby (8,897,988), ropsten (10,499,401)
* Add chainspecs for aleut and baikal
* Bump ethereum/tests to v9.0.2

## OpenEthereum v3.2.6

Enhancement:
* Berlin hardfork blocks: poacore (21,364,900), poasokol (21,050,600)

## OpenEthereum v3.2.5

Bug fixes:
* Backport: Block sync stopped without any errors. #277 (#286)
* Strict memory order (#306)

Enhancements:
* Executable queue for ancient blocks inclusion (#208)
* Backport AuRa commits for xdai (#330)
* Add Nethermind to clients that accept service transactions (#324)
* Implement the filter argument in parity_pendingTransactions (#295)
* Ethereum-types and various libs upgraded (#315)
* [evmbin] Omit storage output, now for std-json (#311)
* Freeze pruning while creating snapshot (#205)
* AuRa multi block reward (#290)
* Improved metrics. DB read/write. prometheus prefix config (#240)
* Send RLPx auth in EIP-8 format (#287)
* rpc module reverted for RPC JSON api (#284)
* Revert "Remove eth/63 protocol version (#252)"
* Support for eth/65 protocol version (#366)
* Berlin hardfork blocks: kovan (24,770,900), xdai (16,101,500)
* Bump ethereum/tests to v8.0.3

devops:
* Upgrade docker alpine to `v1.13.2`. for rust `v1.47`.
* Send SIGTERM instead of SIGHUP to OE daemon (#317)

## OpenEthereum v3.2.4

* Fix for Typed transaction broadcast.

## OpenEthereum v3.2.3

* Hotfix for berlin consensus error.

## OpenEthereum v3.2.2-rc.1

Bug fixes:
* Backport: Block sync stopped without any errors. #277 (#286)
* Strict memory order (#306)

Enhancements:
* Executable queue for ancient blocks inclusion (#208)
* Backport AuRa commits for xdai (#330)
* Add Nethermind to clients that accept service transactions (#324)
* Implement the filter argument in parity_pendingTransactions (#295) 
* Ethereum-types and various libs upgraded (#315)
* Bump ethereum/tests to v8.0.2
* [evmbin] Omit storage output, now for std-json (#311)
* Freeze pruning while creating snapshot (#205)
* AuRa multi block reward (#290)
* Improved metrics. DB read/write. prometheus prefix config (#240)
* Send RLPx auth in EIP-8 format (#287)
* rpc module reverted for RPC JSON api (#284)
* Revert "Remove eth/63 protocol version (#252)"

devops:
* Upgrade docker alpine to `v1.13.2`. for rust `v1.47`.
* Send SIGTERM instead of SIGHUP to OE daemon (#317)

## OpenEthereum v3.2.1

Hot fix issue, related to initial sync:
* Initial sync gets stuck. (#318)

## OpenEthereum v3.2.0

Bug fixes:
* Update EWF's chains with Istanbul transition block numbers (#11482) (#254)
* fix Supplied instant is later than self (#169)
* ethcore/snapshot: fix double-lock in Service::feed_chunk (#289)

Enhancements:
* Berlin hardfork blocks: mainnet (12,244,000), goerli (4,460,644), rinkeby (8,290,928) and ropsten (9,812,189)
* yolo3x spec (#241)
* EIP-2930 RPC support
* Remove eth/63 protocol version (#252)
* Snapshot manifest block added to prometheus (#232)
* EIP-1898: Allow default block parameter to be blockHash
* Change ProtocolId to U64
* Update ethereum/tests
