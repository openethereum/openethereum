// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

struct Code;
struct Address;
struct MerklePatriciaTree<K,V>;
struct CodeBuffer(Vec<u8>);

struct Account {
  nonce: U256,
  balance: U256,
  storage: MerklePatriciaTree<U256, U256>,
  code: CodeBuffer
}

struct Transaction;

struct Block;
struct Blockchain;

struct WorldState;