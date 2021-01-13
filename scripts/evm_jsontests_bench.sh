#!/usr/bin/env bash

cargo build --release -p evmbin

./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmArithmeticTest
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmBitwiseLogicOperation
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmBlockInfoTest
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmEnvironmentalInfo
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmIOandFlowOperations
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmLogTest
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmPerformance
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmPushDupSwapTest
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmRandomTest
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmSha3Test
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmSystemOperations
./target/release/openethereum-evm stats-jsontests-vm ./ethcore/res/json_tests/VMTests/vmTests
