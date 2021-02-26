// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

mod state;

// #[cfg(test)]
// mod tests {

//     use ethcore::{client::{Executive, test_client::get_temp_state_db}, test_helpers::get_temp_state_with_factory};
//     use evm::{ActionParams, EnvInfo, Factory, Schedule, VMType};

//     #[test]
//     fn test_vm_creation() {
//         let action = ActionParams::default();
//         let schedule = Schedule::new_berlin();
//         let vm = Factory::new(VMType::Interpreter, 1024 * 32);
//         let journaldb = get_temp_state_db();
//         let env = EnvInfo::default();
//         let mut exec = Executive::new(&mut state, env, machine, &schedule);
//         //let exec = vm.create(action, &schedule, 1024);
//     }
// }
