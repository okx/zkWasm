#[cfg(test)]
mod tests {
    use crate::{
        foreign::wasm_input_helper::runtime::register_wasm_input_foreign,
        runtime::{host::HostEnv, WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
        Fr,
    };
    use std::{fs::File, io::Read, path::PathBuf};
    use wasmi::ImportsBuilder;

    #[test]
    fn test_binary_search_64() {
        let public_inputs = vec![3];

        let mut binary = vec![];

        let path = PathBuf::from("wasm/bsearch_64.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut binary).unwrap();

        let compiler = WasmInterpreter::new();
        let mut env = HostEnv::new();
        register_wasm_input_foreign(&mut env, public_inputs.clone(), vec![]);
        let imports = ImportsBuilder::new().with_resolver("env", &env);

        let compiled_module = compiler
            .compile(&binary, &imports, &env.function_plugin_lookup)
            .unwrap();
        let execution_log = compiler
            .run(
                &mut env,
                &compiled_module,
                "bsearch",
                public_inputs.clone(),
                vec![],
            )
            .unwrap();

        run_test_circuit::<Fr>(
            compiled_module.tables,
            execution_log.tables,
            public_inputs.into_iter().map(|v| Fr::from(v)).collect(),
        )
        .unwrap()
    }
}
