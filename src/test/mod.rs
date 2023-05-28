use crate::circuits::config::zkwasm_k;
use crate::circuits::utils::table_entry::MemoryWritingTable;
use crate::circuits::TestCircuit;
use crate::profile::Profiler;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::ExecutionResult;
use crate::runtime::WasmInterpreter;

#[cfg(feature = "checksum")]
use crate::image_hasher::ImageHasher;

use anyhow::Result;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::dev::MockProver;
use halo2_proofs::pairing::bn256::Fr as Fp;
use wasmi::ImportsBuilder;
use wasmi::RuntimeValue;

#[cfg(test)]
mod test_wasm_instructions;

mod spec;
mod test_binary_search;
mod test_fibonacci;
mod test_rlp;
mod test_rlp_simple;
mod test_start;

#[cfg(feature = "checksum")]
mod test_uniform_verifier;

fn test_circuit(
    mut env: HostEnv,
    wasm: Vec<u8>,
    function_name: &str,
    public_inputs: Vec<Fp>,
) -> Result<ExecutionResult<RuntimeValue>> {
    let module = wasmi::Module::from_buffer(&wasm).expect("failed to load wasm");

    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(
            &module,
            &imports,
            &env.function_description_table(),
            function_name,
        )
        .unwrap();

    let execution_result = compiled_module.run(&mut env)?;

    run_test_circuit::<Fp>(execution_result.clone(), public_inputs)?;

    Ok(execution_result)
}

fn test_circuit_noexternal_function(textual_repr: &str, function_name: &str) -> Result<()> {
    let wasm = wabt::wat2wasm(&textual_repr).expect("failed to parse wat");

    let mut env = HostEnv::new();
    env.finalize();

    test_circuit(env, wasm, function_name, vec![]).unwrap();

    Ok(())
}

pub fn test_circuit_noexternal(textual_repr: &str) -> Result<()> {
    test_circuit_noexternal_function(textual_repr, "test").unwrap();

    Ok(())
}

pub fn test_circuit_with_env(
    env: HostEnv,
    wasm: Vec<u8>,
    function_name: &str,
    public_inputs: Vec<Fp>,
) -> Result<ExecutionResult<RuntimeValue>> {
    test_circuit(env, wasm, function_name, public_inputs)
}

pub fn run_test_circuit<F: FieldExt>(
    execution_result: ExecutionResult<wasmi::RuntimeValue>,
    mut public_inputs: Vec<F>,
) -> Result<()> {
    let mut instances = vec![];

    #[cfg(feature = "checksum")]
    instances.push(execution_result.tables.compilation_tables.hash());

    // TODO

    instances.append(&mut public_inputs);

    execution_result.tables.write_json(None);
    let memory_writing_table: MemoryWritingTable = execution_result
        .tables
        .execution_tables
        .mtable
        .clone()
        .into();
    memory_writing_table.write_json(None);

    execution_result.tables.profile_tables();

    let circuit = TestCircuit::<F>::new(execution_result.tables);
    let prover = MockProver::run(zkwasm_k(), &circuit, vec![instances])?;
    assert_eq!(prover.verify(), Ok(()));

    Ok(())
}
