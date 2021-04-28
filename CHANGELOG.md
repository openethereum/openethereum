## OpenEthereum v3.2.5-rc.1

Enhancements:
* Support for eth/65 protocol version (#366)
* Berlin hardfork blocks: kovan (24,770,900), xdai (16,101,500)
* Bump ethereum/tests to v8.0.3

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