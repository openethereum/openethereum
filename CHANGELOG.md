## OpenEthereum v3.2.0-rc.1

Bug fixes:
* Update EWF's chains with Istanbul transition block numbers (#11482) (#254)
* fix Supplied instant is later than self (#169)
  
Enhancements:
* Berlin hardfork blocks: mainnet (12,244,000), goerli (4,460,644), rinkeby (8,290,928) and ropsten (9,812,189)
* yolo3x spec (#241)
* EIP-2930 RPC support
* Remove eth/63 protocol version (#252)
* Snapshot manifest block added to prometheus (#232)
* EIP-1898: Allow default block parameter to be blockHash
* Change ProtocolId to U64
* Update ethereum/tests