use crate::cli::args::parse_args;
#[cfg(feature = "checksum")]
use crate::image_hasher::ImageHasher;
use crate::profile::Profiler;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;
use crate::runtime::CompiledImage;
use anyhow::Result;
use halo2_proofs::arithmetic::Engine;
use halo2_proofs::arithmetic::BaseExt;
use halo2_proofs::dev::MockProver;
pub use halo2_proofs::pairing::bn256::Bn256;
pub use halo2_proofs::pairing::bn256::Fr;
pub use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::plonk::verify_proof;
use halo2_proofs::plonk::SingleVerifier;
use halo2_proofs::plonk::VerifyingKey;
use halo2_proofs::poly::commitment::ParamsVerifier;
use halo2_proofs::poly::commitment::Params;
pub use halo2aggregator_s::circuit_verifier::circuit::AggregatorCircuit;
use halo2aggregator_s::circuit_verifier::build_aggregate_verify_circuit;
pub use halo2aggregator_s::circuits::utils::load_instance;
pub use halo2aggregator_s::circuits::utils::load_or_build_unsafe_params;
use halo2aggregator_s::circuits::utils::load_or_build_vkey;
use halo2aggregator_s::circuits::utils::load_or_create_proof;
use halo2aggregator_s::circuits::utils::load_proof;
pub use halo2aggregator_s::circuits::utils::load_vkey;
use halo2aggregator_s::circuits::utils::run_circuit_unsafe_full_pass;
pub use halo2aggregator_s::circuits::utils::store_instance;
use halo2aggregator_s::circuits::utils::TranscriptHash;
use halo2aggregator_s::solidity_verifier::codegen::solidity_aux_gen;
use halo2aggregator_s::solidity_verifier::solidity_render;
use halo2aggregator_s::transcript::poseidon::PoseidonRead;
use halo2aggregator_s::transcript::sha256::ShaRead;
use log::debug;
use log::error;
use log::info;
use notify::event::AccessMode;
use notify::RecursiveMode;
use notify::Watcher;
use serde::Deserialize;
use serde::Serialize;
pub use specs::ExecutionTable;
use specs::Tables;
use std::fs;
pub use specs::CompilationTable;
#[cfg(feature = "checksum")]
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use wasmi::tracer::Tracer;
use wasmi::{ImportsBuilder, RuntimeValue};
use wasmi::Module;
use wasmi::NotStartedModuleRef;
use crate::circuits::TestCircuit;
use crate::circuits::ZkWasmCircuitBuilder;
use crate::foreign::log_helper::{register_log_foreign, register_log_output_foreign};
use crate::foreign::require_helper::register_require_foreign;
use crate::foreign::kv_helper::kvpair::register_kvpair_foreign;
use crate::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use crate::foreign::hash_helper::sha256::register_sha256_foreign;
use crate::foreign::hash_helper::poseidon::register_poseidon_foreign;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::wasmi_interpreter::Execution;
use crate::runtime::WasmInterpreter;

use crate::foreign::ecc_helper::{
    bls381::pair::register_blspair_foreign,
    bls381::sum::register_blssum_foreign,
    bn254::pair::register_bn254pair_foreign,
    bn254::sum::register_bn254sum_foreign,
    jubjub::sum::register_babyjubjubsum_foreign,
};

const AGGREGATE_PREFIX: &'static str = "aggregate-circuit";

pub fn compile_image<'a>(
    module: &'a Module,
    function_name: &str,
) -> (
    WasmRuntimeIO,
    CompiledImage<NotStartedModuleRef<'a>, Tracer>,
) {
    let mut env = HostEnv::new();
    let wasm_runtime_io = register_wasm_input_foreign(&mut env, vec![], vec![]);
    register_require_foreign(&mut env);
    register_log_foreign(&mut env);
    register_kvpair_foreign(&mut env);
    register_blspair_foreign(&mut env);
    register_blssum_foreign(&mut env);
    register_bn254pair_foreign(&mut env);
    register_bn254sum_foreign(&mut env);
    register_sha256_foreign(&mut env);
    register_poseidon_foreign(&mut env);
    register_babyjubjubsum_foreign(&mut env);
    register_log_output_foreign(&mut env);
    env.finalize();
    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    (
        wasm_runtime_io,
        compiler
            .compile(
                &module,
                &imports,
                &env.function_description_table(),
                function_name,
            )
            .expect("file cannot be complied"),
    )
}

#[cfg(feature = "checksum")]
fn hash_image(wasm_binary: &Vec<u8>, function_name: &str) -> Fr {
    let module = wasmi::Module::from_buffer(wasm_binary).expect("failed to load wasm");

    let (_, compiled_image) = compile_image(&module, function_name);
    compiled_image.tables.hash()
}

pub fn build_circuit_without_witness(
    wasm_binary: &Vec<u8>,
    function_name: &str,
) -> TestCircuit<Fr> {
    let module = wasmi::Module::from_buffer(wasm_binary).expect("failed to load wasm");

    let (wasm_runtime_io, compiled_module) = compile_image(&module, function_name);
    let builder = ZkWasmCircuitBuilder {
        tables: Tables {
            compilation_tables: compiled_module.tables,
            execution_tables: ExecutionTable::default(),
        },
        public_inputs_and_outputs: wasm_runtime_io.public_inputs_and_outputs.borrow().clone(),
    };

    builder.build_circuit::<Fr>()
}

pub fn exec_image(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<(Vec<u64>, HostEnv)> {
    let (mut env, wasm_runtime_io) =
        HostEnv::new_with_full_foreign_plugins(public_inputs.clone(), private_inputs.clone());

    let module = wasmi::Module::from_buffer(wasm_binary).expect("failed to load wasm");

    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(
            &module,
            &imports,
            &env.function_description_table(),
            function_name,
        )
        .expect("file cannot be complied");

    let (_, outputs) = compiled_module.dry_run(&mut env, wasm_runtime_io)?;

    env.display_time_profile();

    Ok((outputs, env))
}

pub fn exec_image_trace(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<(ZkWasmCircuitBuilder, Vec<u64>, HostEnv)> {
    let (mut env, wasm_runtime_io) =
        HostEnv::new_with_full_foreign_plugins(public_inputs.clone(), private_inputs.clone());

    let module = wasmi::Module::from_buffer(wasm_binary).expect("failed to load wasm");

    let imports = ImportsBuilder::new().with_resolver("env", &env);

    let compiler = WasmInterpreter::new();
    let compiled_module = compiler
        .compile(
            &module,
            &imports,
            &env.function_description_table(),
            function_name,
        )
        .expect("file cannot be complied");

    let r = compiled_module.run(&mut env, wasm_runtime_io)?;

    env.display_time_profile();

    let builder = ZkWasmCircuitBuilder {
        tables: r.tables,
        public_inputs_and_outputs: r.public_inputs_and_outputs,
    };

    Ok((builder, r.outputs, env))
}

pub fn build_circuit_with_witness(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<(TestCircuit<Fr>, Vec<Fr>)> {
    let (builder, outputs, _) =
        exec_image_trace(wasm_binary, function_name, public_inputs, private_inputs)?;

    let instance: Vec<Fr> = builder
        .public_inputs_and_outputs
        .clone()
        .iter()
        .map(|v| (*v).into())
        .collect();

    builder.tables.profile_tables();

    println!("output:");
    println!("{:?}", outputs);

    Ok((builder.build_circuit(), instance))
}

fn build_tables_and_outputs(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<(Tables, Vec<u64>, Vec<u64>, HostEnv)>{
    let (builder, outputs, env) = exec_image_trace(wasm_binary, function_name, public_inputs, private_inputs)?;

    Ok((builder.build_circuit_without_configure::<Fr>().tables, builder.public_inputs_and_outputs, outputs, env))
}

pub fn exec_setup(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &str,
    wasm_binary: &Vec<u8>,
    entry: &str,
    output_dir: &PathBuf,
) {
    exec_setup2(zkwasm_k, aggregate_k, prefix, wasm_binary, entry, output_dir);
}

pub fn exec_setup2(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &str,
    wasm_binary: &Vec<u8>,
    entry: &str,
    output_dir: &PathBuf,
) -> TestCircuit<Fr> {
    let circuit = build_circuit_without_witness(wasm_binary, entry);

    info!("Setup Params and VerifyingKey");

    // Setup ZkWasm Params
    let params = {
        let params_path = &output_dir.join(format!("K{}.params", zkwasm_k));

        if params_path.exists() {
            info!("Found Params with K = {} at {:?}", zkwasm_k, params_path);
        } else {
            info!("Create Params with K = {} to {:?}", zkwasm_k, params_path);
        }

        load_or_build_unsafe_params::<Bn256>(zkwasm_k, Some(params_path))
    };

    // Setup ZkWasm Vkey
    {
        let vk_path = &output_dir.join(format!("{}.{}.vkey.data", prefix, 0));

        if vk_path.exists() {
            info!("Found Verifying at {:?}", vk_path);
        } else {
            info!("Create Verifying to {:?}", vk_path);
        }

        load_or_build_vkey::<Bn256, _>(&params, &circuit, Some(vk_path));
    }

    // Setup Aggregate Circuit Params
    {
        let params_path = &output_dir.join(format!("K{}.params", aggregate_k));

        if params_path.exists() {
            info!("Found Params with K = {} at {:?}", aggregate_k, params_path);
        } else {
            info!(
                "Create Params with K = {} to {:?}",
                aggregate_k, params_path
            );
        }

        load_or_build_unsafe_params::<Bn256>(aggregate_k, Some(params_path))
    };

    circuit
}

#[cfg(feature = "checksum")]
pub fn exec_image_checksum(wasm_binary: &Vec<u8>, entry: &str, output_dir: &PathBuf) {
    let circuit = build_circuit_without_witness(wasm_binary, entry);
    let hash: Fr = circuit.tables.compilation_tables.hash();

    let mut fd =
        std::fs::File::create(&output_dir.join(format!("checksum.data",)).as_path()).unwrap();

    let hash = hash.to_string();
    write!(fd, "{}", hash).unwrap();
    println!("{}", hash);
}

pub fn exec_dry_run_service(
    wasm_binary: Vec<u8>,
    function_name: String,
    listen: &PathBuf,
) -> Result<()> {
    use notify::event::AccessKind;
    use notify::event::EventKind;
    use notify::event::ModifyKind;
    use notify::event::RenameMode;
    use notify::Event;

    #[derive(Serialize, Deserialize, Debug)]
    struct Sequence {
        private_inputs: Vec<String>,
        public_inputs: Vec<String>,
    }

    info!("Dry-run service is running.");
    info!("{:?} is watched", listen);

    let module = wasmi::Module::from_buffer(wasm_binary.clone()).expect("failed to load wasm");
    let compiler = WasmInterpreter::new();

    let mut watcher =
        notify::recommended_watcher(move |handler: Result<Event, _>| match handler {
            Ok(event) => {
                debug!("Event {:?}", event);

                match event.kind {
                    EventKind::Access(AccessKind::Close(AccessMode::Write))
                    | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                        assert_eq!(event.paths.len(), 1);
                        let path = event.paths.first().unwrap();

                        if let Some(ext) = path.extension() {
                            if ext.eq("done") {
                                return;
                            };
                        }

                        info!("Receive a request from file {:?}", path);

                        let json = fs::read_to_string(path).unwrap();
                        if let Ok(sequence) = serde_json::from_str::<Sequence>(&json) {
                            debug!("{:?}", sequence);

                            let private_inputs = parse_args(
                                sequence.private_inputs.iter().map(|s| s.as_str()).collect(),
                            );
                            let public_inputs = parse_args(
                                sequence.public_inputs.iter().map(|s| s.as_str()).collect(),
                            );

                            let (mut env, wasm_runtime_io) = HostEnv::new_with_full_foreign_plugins(
                                public_inputs,
                                private_inputs,
                            );

                            let imports = ImportsBuilder::new().with_resolver("env", &env);

                            let compiled_module = compiler
                                .compile(
                                    &module,
                                    &imports,
                                    &env.function_description_table(),
                                    &function_name,
                                )
                                .expect("file cannot be complied");

                            let r = compiled_module.dry_run(&mut env, wasm_runtime_io).unwrap();
                            println!("return value: {:?}", r);

                            fs::write(
                                Path::new(&format!("{}.done", path.to_str().unwrap())),
                                if let Some(r) = r.0 {
                                    match r {
                                        RuntimeValue::I32(v) => v.to_string(),
                                        RuntimeValue::I64(v) => v.to_string(),
                                        _ => unreachable!(),
                                    }
                                } else {
                                    "".to_owned()
                                },
                            )
                            .unwrap();
                        } else {
                            error!("Failed to parse file {:?}, the request is ignored.", path);
                        }
                    }
                    _ => (),
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        })?;

    loop {
        watcher.watch(listen.as_path(), RecursiveMode::NonRecursive)?;
    }
}

pub fn exec_dry_run(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<()> {
    let _ = exec_image(wasm_binary, function_name, public_inputs, private_inputs)?;

    println!("Execution passed.");

    Ok(())
}

pub fn exec_gen_witness(
    wasm_binary: &Vec<u8>,
    function_name: &str,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<(Tables, Vec<u64>, Vec<u64>, HostEnv)> {
    build_tables_and_outputs(wasm_binary, function_name, public_inputs, private_inputs)
}

pub fn exec_create_proof_from_witness(
    compilation_tables: CompilationTable,
    execution_tables: ExecutionTable,
    instance: &[u64],
    params: &Params<<Bn256 as Engine>::G1Affine>,
    vkey: VerifyingKey<<Bn256 as Engine>::G1Affine>,
) -> Result<Vec<u8>> {
    let circuit = TestCircuit::new(Tables{
        compilation_tables,
        execution_tables,
    });
    let mut instance: Vec<Fr> = instance
        .iter()
        .map(|v| (*v).into())
        .collect();

    let mut instances = vec![];

    #[cfg(feature = "checksum")]
    instances.push(circuit.tables.compilation_tables.hash());

    instances.append(&mut instance);

    Ok(load_or_create_proof::<Bn256, _>(
        params,
        vkey,
        circuit.clone(),
        &[&instances],
        None,
        TranscriptHash::Poseidon,
        false,
    ))
}

pub fn verify_proof_v2(
    public_inputs_size: usize,
    instance: &[u64],
    params: &Params<<Bn256 as Engine>::G1Affine>,
    vkey: VerifyingKey<<Bn256 as Engine>::G1Affine>,
    proof: &[u8]
) {
    let instances = {
        let mut instances = vec![];

        // #[cfg(feature = "checksum")]
        // instances.push(hash_image(wasm_binary, function_name));

        instances.append(&mut instance.iter()
                        .map(|v| Fr::from(*v))
                        .collect());

        instances
    };

    let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();
    let strategy = SingleVerifier::new(&params_verifier);

    verify_proof(
        &params_verifier,
        &vkey,
        strategy,
        &[&[&instances]],
        &mut PoseidonRead::init(proof),
    )
    .unwrap();

    info!("Verifing proof passed");
}

fn exec_create_proof_from_circuit(
    prefix: &str,
    zkwasm_k: u32,
    output_dir: &PathBuf,
    circuit: TestCircuit<Fr>,
    mut instance: Vec<Fr>,
) -> Result<()> {
    let mut instances = vec![];

    #[cfg(feature = "checksum")]
    instances.push(circuit.tables.compilation_tables.hash());

    instances.append(&mut instance);

    circuit.tables.write_json(Some(output_dir.clone()));

    if false {
        info!("Mock test...");

        let prover = MockProver::run(zkwasm_k, &circuit, vec![instances.clone()])?;

        assert_eq!(prover.verify(), Ok(()));

        info!("Mock test passed");
    }

    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let vkey = load_vkey::<Bn256, TestCircuit<_>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", prefix, 0)),
    );

    load_or_create_proof::<Bn256, _>(
        &params,
        vkey,
        circuit.clone(),
        &[&instances],
        Some(&output_dir.join(format!("{}.{}.transcript.data", prefix, 0))),
        TranscriptHash::Poseidon,
        false,
    );

    info!("Proof has been created.");

    Ok(())
}

pub fn exec_create_proof(
    prefix: &'static str,
    zkwasm_k: u32,
    wasm_binary: &Vec<u8>,
    function_name: &str,
    output_dir: &PathBuf,
    public_inputs: &Vec<u64>,
    private_inputs: &Vec<u64>,
) -> Result<()> {
    let (circuit, instance) =
        build_circuit_with_witness(wasm_binary, function_name, public_inputs, private_inputs)?;

    {
        store_instance(
            &vec![instance.clone()],
            &output_dir.join(format!("{}.{}.instance.data", prefix, 0)),
        );
    }

    exec_create_proof_from_circuit(prefix, zkwasm_k, output_dir, circuit, instance)
}

#[allow(unused_variables)]
pub fn exec_verify_proof(
    prefix: &'static str,
    public_inputs_size: usize,
    zkwasm_k: u32,
    wasm_binary: &Vec<u8>,
    function_name: &str,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    instance_path: &PathBuf,
) {
    let mut instance = {
        let mut instance = vec![];
        //load_instance::<Bn256>(&[public_inputs_size], &instances_path);
        let mut fd = std::fs::File::open(&instance_path).unwrap();
        while let Ok(f) = Fr::read(&mut fd) {
            instance.push(f);
        }

        instance
    };

    let instances = {
        let mut instances = vec![];

        #[cfg(feature = "checksum")]
        instances.push(hash_image(wasm_binary, function_name));

        instances.append(&mut instance);

        instances
    };

    let params = load_or_build_unsafe_params::<Bn256>(
        zkwasm_k,
        Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
    );

    let vkey = load_vkey::<Bn256, TestCircuit<_>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", prefix, 0)),
    );

    let proof = load_proof(proof_path);

    let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();
    let strategy = SingleVerifier::new(&params_verifier);

    verify_proof(
        &params_verifier,
        &vkey,
        strategy,
        &[&[&instances]],
        &mut PoseidonRead::init(&proof[..]),
    )
    .unwrap();

    info!("Verifing proof passed");
}

pub fn exec_aggregate_create_proof(
    zkwasm_k: u32,
    aggregate_k: u32,
    prefix: &'static str,
    wasm_binary: &Vec<u8>,
    function_name: &str,
    output_dir: &PathBuf,
    public_inputs: &Vec<Vec<u64>>,
    private_inputs: &Vec<Vec<u64>>,
) {
    assert_eq!(public_inputs.len(), private_inputs.len());

    let (circuits, instances) = public_inputs.iter().zip(private_inputs.iter()).fold(
        (vec![], vec![]),
        |(mut circuits, mut instances), (public, private)| {
            let (circuit, public_input_and_wasm_output) =
                build_circuit_with_witness(&wasm_binary, &function_name, &public, &private)
                    .unwrap();
            let mut instance = vec![];

            #[cfg(feature = "checksum")]
            instance.push(hash_image(wasm_binary, function_name));

            instance.append(
                &mut public_input_and_wasm_output
                    .iter()
                    .map(|v| Fr::from(*v))
                    .collect(),
            );

            circuits.push(circuit);
            instances.push(vec![instance]);

            (circuits, instances)
        },
    );

    let (aggregate_circuit, aggregate_instances) = run_circuit_unsafe_full_pass::<Bn256, _>(
        &output_dir.as_path(),
        prefix,
        zkwasm_k,
        circuits,
        instances,
        TranscriptHash::Poseidon,
        vec![],
        false,
    )
    .unwrap();

    run_circuit_unsafe_full_pass::<Bn256, _>(
        &output_dir.as_path(),
        AGGREGATE_PREFIX,
        aggregate_k,
        vec![aggregate_circuit],
        vec![vec![aggregate_instances]],
        TranscriptHash::Sha,
        vec![],
        true,
    );
}

pub fn exec_create_aggregated_proof_from_witness(
    params: &Params<<Bn256 as Engine>::G1Affine>,
    vkeys: Vec<VerifyingKey<<Bn256 as Engine>::G1Affine>>,
    aggregate_params: &Params<<Bn256 as Engine>::G1Affine>,
    aggregate_vkey_path: &PathBuf,
    // aggregate_vkey: VerifyingKey<<Bn256 as Engine>::G1Affine>,
    proofs: Vec<Vec<u8>>,
    instance: &Vec<Vec<u64>>,
    )->Result<(Vec<u8>, Vec<Fr>)>{
    let mut instances = vec![];
    for item in instance.iter(){
        let mut new_instance: Vec<Vec<Fr>> = vec![];
        let new_item: Vec<Fr> = item
        .iter()
        .map(|v| (*v).into())
        .collect();
        new_instance.push(new_item);
        instances.push(new_instance);
    }


    let public_inputs_size = instances.iter().fold(0usize, |acc, x| {
        usize::max(acc, x.iter().fold(0, |acc, x| usize::max(acc, x.len())))
    });
    let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();



    let (circuit, instances) = build_aggregate_verify_circuit::<Bn256>(
            &params_verifier,
            &vkeys[..].iter().collect::<Vec<_>>(),
            instances.iter().collect(),
            proofs,
            TranscriptHash::Poseidon,
            vec![],
        );

    let aggregate_vkey = load_or_build_vkey::<Bn256, _>(&aggregate_params, &circuit, Some(aggregate_vkey_path));
    let proof = load_or_create_proof::<Bn256, _>(
        &aggregate_params,
        aggregate_vkey,
        circuit.clone(),
        &[&instances],
        None,
        TranscriptHash::Sha,
        false,
    );

    Ok((proof, instances))
}

pub fn verify_aggregate_proof(
    params: &Params<<Bn256 as Engine>::G1Affine>,
    vkey: VerifyingKey<<Bn256 as Engine>::G1Affine>,
    proof: &Vec<u8>,
    instances: &Vec<Vec<Fr>>,
    n_proofs: usize,
) {
    let public_inputs_size: u32 = 6 + 3 * n_proofs as u32;

    let params_verifier: ParamsVerifier<Bn256> =
        params.verifier(public_inputs_size as usize).unwrap();
    let strategy = SingleVerifier::new(&params_verifier);

    verify_proof(
        &params_verifier,
        &vkey,
        strategy,
        &[&instances.iter().map(|x| &x[..]).collect::<Vec<_>>()[..]],
        &mut ShaRead::<_, _, _, sha2::Sha256>::init(&proof[..]),
    )
    .unwrap();

    info!("Verifing Aggregate Proof Passed.")
}

pub fn exec_verify_aggregate_proof(
    aggregate_k: u32,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    instances_path: &PathBuf,
    n_proofs: usize,
) {
    let params = load_or_build_unsafe_params::<Bn256>(
        aggregate_k,
        Some(&output_dir.join(format!("K{}.params", aggregate_k))),
    );

    let proof = load_proof(&proof_path.as_path());
    let vkey = load_vkey::<Bn256, AggregatorCircuit<G1Affine>>(
        &params,
        &output_dir.join(format!("{}.{}.vkey.data", AGGREGATE_PREFIX, 0)),
    );

    let public_inputs_size: u32 = 6 + 3 * n_proofs as u32;

    let instances = load_instance::<Bn256>(&[public_inputs_size], &instances_path);

    let params_verifier: ParamsVerifier<Bn256> =
        params.verifier(public_inputs_size as usize).unwrap();
    let strategy = SingleVerifier::new(&params_verifier);

    verify_proof(
        &params_verifier,
        &vkey,
        strategy,
        &[&instances.iter().map(|x| &x[..]).collect::<Vec<_>>()[..]],
        &mut ShaRead::<_, _, _, sha2::Sha256>::init(&proof[..]),
    )
    .unwrap();

    info!("Verifing Aggregate Proof Passed.")
}

pub fn exec_solidity_aggregate_proof(
    zkwasm_k: u32,
    aggregate_k: u32,
    max_public_inputs_size: usize,
    output_dir: &PathBuf,
    proof_path: &PathBuf,
    sol_path: &PathBuf,
    instances_path: &PathBuf,
    n_proofs: usize,
    aux_only: bool,
) {
    let zkwasm_params_verifier: ParamsVerifier<Bn256> = {
        let params = load_or_build_unsafe_params::<Bn256>(
            zkwasm_k,
            Some(&output_dir.join(format!("K{}.params", zkwasm_k))),
        );

        params.verifier(max_public_inputs_size).unwrap()
    };

    let (verifier_params_verifier, vkey, instances, proof) = {
        let public_inputs_size = 6 + 3 * n_proofs;

        let params = load_or_build_unsafe_params::<Bn256>(
            aggregate_k,
            Some(&output_dir.join(format!("K{}.params", aggregate_k))),
        );

        let params_verifier = params.verifier(public_inputs_size).unwrap();

        let vkey = load_vkey::<Bn256, AggregatorCircuit<G1Affine>>(
            &params,
            &output_dir.join(format!("{}.{}.vkey.data", AGGREGATE_PREFIX, 0)),
        );

        let instances = load_instance::<Bn256>(&[public_inputs_size as u32], &instances_path);
        let proof = load_proof(&proof_path.as_path());

        (params_verifier, vkey, instances, proof)
    };

    if !aux_only {
        let path_in = {
            let mut path = sol_path.clone();
            path.push("templates");
            path
        };
        let path_out = {
            let mut path = sol_path.clone();
            path.push("contracts");
            path
        };
        solidity_render(
            &(path_in.to_str().unwrap().to_owned() + "/*"),
            path_out.to_str().unwrap(),
            vec![(
                "AggregatorConfig.sol.tera".to_owned(),
                "AggregatorConfig.sol".to_owned(),
            )],
            "AggregatorVerifierStepStart.sol.tera",
            "AggregatorVerifierStepEnd.sol.tera",
            |i| format!("AggregatorVerifierStep{}.sol", i + 1),
            &zkwasm_params_verifier,
            &verifier_params_verifier,
            &vkey,
            &instances[0],
            proof.clone(),
        );
    }

    solidity_aux_gen(
        &verifier_params_verifier,
        &vkey,
        &instances[0],
        proof,
        &output_dir.join(format!("{}.{}.aux.data", AGGREGATE_PREFIX, 0)),
    );
}

#[cfg(test)]
mod tests{

    use crate::cli::exec::{exec_gen_witness, exec_create_proof_from_witness};
    use halo2_proofs::poly::commitment::ParamsVerifier;
    use halo2_proofs::plonk::verify_proof;
    use halo2_proofs::plonk::SingleVerifier;
    use halo2aggregator_s::transcript::poseidon::PoseidonRead;
    use halo2_proofs::pairing::bn256::Bn256;
    use halo2aggregator_s::circuits::utils::load_vkey;
    use halo2aggregator_s::circuits::utils::load_or_build_unsafe_params;
    use halo2_proofs::pairing::bn256::Fr;
    use crate::circuits::TestCircuit;
    use crate::circuits::config::init_zkwasm_runtime;
    use std::path::PathBuf;
    use std::fs;

    #[test]
    fn test_create_proof(){
        // generate witness and instances
        let public_inputs: Vec<u64> = vec![133, 2];
        let private_inputs: Vec<u64> = vec![];
        const WASM_FILE_PATH: &str = "wasm/wasm_output.wasm";
        let wasm_binary = fs::read(&WASM_FILE_PATH).unwrap();
        const FUNCTION_NAME: &str = "zkmain";
        let (tables, instance, _output, _env) = exec_gen_witness(
            &wasm_binary,
            FUNCTION_NAME,
            &public_inputs,
            &private_inputs,
            ).unwrap();

        let output_dir = PathBuf::from("./output");

        const ZKWASM_K: u32 = 18;
    // Setup ZkWasm Params
    let params = {
        let params_path = &output_dir.join(format!("K{}.params", ZKWASM_K));

        if params_path.exists() {
            println!("Found Params with K = {} at {:?}", ZKWASM_K, params_path);
        } else {
            println!("Create Params with K = {} to {:?}", ZKWASM_K, params_path);
        }

        load_or_build_unsafe_params::<Bn256>(ZKWASM_K, Some(params_path))
    };

    // Setup ZkWasm Vkey
    let vkey = {
        let vk_path = &output_dir.join(format!("{}.{}.vkey.data", "zkwasm", 0));

        if vk_path.exists() {
            println!("Found Verifying at {:?}", vk_path);
        } else {
            panic!("vkey not found");
        }

        init_zkwasm_runtime(ZKWASM_K, &tables.compilation_tables);
        load_vkey::<Bn256, TestCircuit<_>>(&params, vk_path)
    };
        // create proof
        let proof = exec_create_proof_from_witness(
            tables.compilation_tables,
            tables.execution_tables,
            &instance,
            &params,
            vkey,
        ).unwrap();


        {
            let public_inputs_size = 64;
    let params_verifier: ParamsVerifier<Bn256> = params.verifier(public_inputs_size).unwrap();
    let strategy = SingleVerifier::new(&params_verifier);

    // verify
    let instance: Vec<Fr> = instance
        .iter()
        .map(|v| (*v).into())
        .collect();

    // load vkey again
    let vkey = {
        let vk_path = &output_dir.join(format!("{}.{}.vkey.data", "zkwasm", 0));

        if vk_path.exists() {
            println!("Found Verifying at {:?}", vk_path);
        } else {
            panic!("vkey not found");
        }

        load_vkey::<Bn256, TestCircuit<_>>(&params, vk_path)
    };
    verify_proof(
        &params_verifier,
        &vkey,
        strategy,
        &[&[&instance]],
        &mut PoseidonRead::init(&proof[..]),
    )
    .unwrap();
    println!("verify proof success");
        }
    }
}
