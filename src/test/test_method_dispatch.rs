use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::runtime::host::host_env::{EnvHook, HostEnv};
use crate::runtime::ExecutionResult;

use anyhow::Result;
use std::fs::{self};

use super::compile_then_execute_wasm;


fn build_test(hook: &Option<EnvHook>) -> Result<ExecutionResult<wasmi::RuntimeValue>> {
    let public_inputs = vec![3];

    let wasm = fs::read("wasm/method.wasm").unwrap();

    let mut env = HostEnv::default();
    if let Some(hook) = hook {
        hook.hook(&mut env)
    }
    let wasm_runtime_io = register_wasm_input_foreign(&mut env, public_inputs, vec![]);
    let a=env.external_env.get_current_function_len();
    println!("{:?}",a);
    env.finalize();

    compile_then_execute_wasm(env, wasm_runtime_io, wasm, "zkmain")
}

mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::foreign::function_dispatcher::{HostFunction, register_dispatch_foreign};
    use crate::test::test_circuit_mock;
    use super::*;
    use halo2_proofs::pairing::bn256::Fr as Fp;

    #[derive(Default)]
    pub struct Temp {
        v: Option<String>,
    }

    impl HostFunction for Temp {
        fn consume(&mut self, data: Vec<u8>) {
            self.v = Some(String::from_utf8_lossy(data.as_slice()).to_string());
            println!("{:?}", &self.v);
        }
    }

    #[test]
    fn test_wasm_hook() {
        let temp: Rc<RefCell<Box<dyn HostFunction>>> = Rc::new(RefCell::new(Box::new(Temp::default())));
        let binding = |env: &mut HostEnv| {
            register_dispatch_foreign(env, vec![(0, temp.clone())]);
        };
        let hook: &Option<EnvHook> = &Some(EnvHook(&binding));
        let trace = build_test(hook).unwrap();
        test_circuit_mock::<Fp>(trace).unwrap();
    }
}