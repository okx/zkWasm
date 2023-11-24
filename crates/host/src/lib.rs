pub mod host;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use delphinus_zkwasm::foreign::context::runtime::register_context_foreign;
use delphinus_zkwasm::foreign::log_helper::register_external_output_foreign;
use delphinus_zkwasm::foreign::log_helper::register_log_foreign;
use delphinus_zkwasm::foreign::require_helper::register_require_foreign;
use delphinus_zkwasm::foreign::wasm_input_helper::runtime::register_wasm_input_foreign;
use delphinus_zkwasm::runtime::wasmi_interpreter::WasmRuntimeIO;

use delphinus_zkwasm::runtime::host::host_env::HostEnv;
use delphinus_zkwasm::runtime::host::ContextOutput;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::host::Sequence;
use specs::args::parse_args;
use std::sync::Arc;
use std::sync::Mutex;
use zkwasm_host_circuits::host::db::TreeDB;

pub struct ExecutionArg {
    /// Public inputs for `wasm_input(1)`
    pub public_inputs: Vec<u64>,
    /// Private inputs for `wasm_input(0)`
    pub private_inputs: Vec<u64>,
    /// Context inputs for `wasm_read_context()`
    pub context_inputs: Vec<u64>,
    /// Context outputs for `wasm_write_context()`
    pub context_outputs: Arc<Mutex<Vec<u64>>>,
    /// db src
    pub tree_db: Option<Rc<RefCell<dyn TreeDB>>>,
    /// external outputs for `wasm_external_output_push`
    pub external_outputs: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    pub trace_count: Option<Rc<RefCell<usize>>>,
}

impl ContextOutput for ExecutionArg {
    fn get_context_outputs(&self) -> Arc<Mutex<Vec<u64>>> {
        self.context_outputs.clone()
    }
}

impl From<Sequence> for ExecutionArg {
    fn from(seq: Sequence) -> ExecutionArg {
        let private_inputs = parse_args(seq.private_inputs.iter().map(|s| s.as_str()).collect());
        let public_inputs = parse_args(seq.public_inputs.iter().map(|s| s.as_str()).collect());
        let context_inputs = parse_args(seq.context_input.iter().map(|s| s.as_str()).collect());
        let context_outputs = Arc::new(Mutex::new(vec![]));
        let external_outputs = Rc::new(RefCell::new(HashMap::new()));
        ExecutionArg {
            private_inputs,
            public_inputs,
            context_inputs,
            context_outputs,
            tree_db: None,
            external_outputs,
            trace_count: None,
        }
    }
}

pub struct StandardHostEnvBuilder;

impl HostEnvBuilder for StandardHostEnvBuilder {
    type Arg = ExecutionArg;

    fn create_env_without_value() -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, vec![], Arc::new(Mutex::new(vec![])));
        host::hash_helper::poseidon::register_poseidon_foreign(&mut env);
        host::merkle_helper::merkle::register_merkle_foreign(&mut env, None);
        host::ecc_helper::bn254::sum::register_bn254sum_foreign(&mut env);
        host::ecc_helper::bn254::pair::register_bn254pair_foreign(&mut env);
        host::ecc_helper::jubjub::sum::register_babyjubjubsum_foreign(&mut env);
        host::witness_helper::register_witness_foreign(&mut env);
        register_external_output_foreign(&mut env, Rc::new(RefCell::new(HashMap::new())), None);
        env.finalize();

        (env, wasm_runtime_io)
    }

    fn create_env(arg: Self::Arg) -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io =
            register_wasm_input_foreign(&mut env, arg.public_inputs, arg.private_inputs);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, arg.context_inputs, arg.context_outputs);
        host::hash_helper::poseidon::register_poseidon_foreign(&mut env);
        host::merkle_helper::merkle::register_merkle_foreign(&mut env, arg.tree_db);
        host::ecc_helper::bn254::sum::register_bn254sum_foreign(&mut env);
        host::ecc_helper::bn254::pair::register_bn254pair_foreign(&mut env);
        host::ecc_helper::jubjub::sum::register_babyjubjubsum_foreign(&mut env);
        host::witness_helper::register_witness_foreign(&mut env);
        register_external_output_foreign(&mut env, arg.external_outputs, arg.trace_count);
        env.finalize();

        (env, wasm_runtime_io)
    }
}
