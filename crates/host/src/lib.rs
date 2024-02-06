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
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;
use zkwasm_host_circuits::host::db::TreeDB;
use zkwasm_host_circuits::proof::OpType;

pub struct ExecutionArg {
    /// Public inputs for `wasm_input(1)`
    pub public_inputs: Vec<u64>,
    /// Private inputs for `wasm_input(0)`
    pub private_inputs: Vec<u64>,
    /// Context inputs for `wasm_read_context()`
    pub context_inputs: Vec<u64>,
    /// Context outputs for `wasm_write_context()`
    pub context_outputs: Arc<Mutex<Vec<u64>>>,
    /// indexed witness context
    pub indexed_witness: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    /// db src
    pub tree_db: Option<Rc<RefCell<dyn TreeDB>>>,
}

// because it is singleton
unsafe impl Send for ExecutionArg{}
unsafe impl Sync for ExecutionArg{}

impl ContextOutput for ExecutionArg {
    fn get_context_outputs(&self) -> Arc<Mutex<Vec<u64>>> {
        self.context_outputs.clone()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HostEnvConfig {
    pub ops: Vec<OpType>,
}

impl Default for HostEnvConfig {
    fn default() -> Self {
        HostEnvConfig {
            ops: vec![
                OpType::POSEIDONHASH,
                OpType::MERKLE,
                OpType::JUBJUBSUM,
                OpType::KECCAKHASH,
                OpType::BN256SUM,
            ],
        }
    }
}

impl HostEnvConfig {
    fn register_ops(&self, env: &mut HostEnv, tree_db: Option<Rc<RefCell<dyn TreeDB>>>) {
        for op in &self.ops {
            match op {
                OpType::BLS381PAIR => host::ecc_helper::bls381::pair::register_blspair_foreign(env),
                OpType::BLS381SUM => host::ecc_helper::bls381::sum::register_blssum_foreign(env),
                OpType::BN256PAIR => host::ecc_helper::bn254::pair::register_bn254pair_foreign(env),
                OpType::BN256SUM => host::ecc_helper::bn254::sum::register_bn254sum_foreign(env),
                OpType::POSEIDONHASH => host::hash_helper::poseidon::register_poseidon_foreign(env),
                OpType::MERKLE => {
                    host::merkle_helper::merkle::register_merkle_foreign(env, tree_db.clone());
                    host::merkle_helper::datacache::register_datacache_foreign(
                        env,
                        tree_db.clone(),
                    );
                }
                OpType::JUBJUBSUM => {
                    host::ecc_helper::jubjub::sum::register_babyjubjubsum_foreign(env)
                }
                OpType::KECCAKHASH => host::hash_helper::keccak256::register_keccak_foreign(env),
            }
        }
    }
}

pub struct StandardHostEnvBuilder;

impl HostEnvBuilder for StandardHostEnvBuilder {
    type Arg = ExecutionArg;
    type HostConfig = HostEnvConfig;

    fn create_env_without_value(envconfig: &Self::HostConfig) -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, vec![], vec![]);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, vec![], Arc::new(Mutex::new(vec![])));
        envconfig.register_ops(&mut env, None);
        let external_output = Rc::new(RefCell::new(HashMap::new()));
        host::witness_helper::register_witness_foreign(&mut env, external_output.clone());
        register_external_output_foreign(&mut env, external_output);
        env.finalize();

        (env, wasm_runtime_io)
    }

    fn create_env(arg: Self::Arg, envconfig: &Self::HostConfig) -> (HostEnv, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io =
            register_wasm_input_foreign(&mut env, arg.public_inputs, arg.private_inputs);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_context_foreign(&mut env, arg.context_inputs, arg.context_outputs);
        host::witness_helper::register_witness_foreign(&mut env, arg.indexed_witness.clone());
        envconfig.register_ops(&mut env, arg.tree_db);
        register_external_output_foreign(&mut env, arg.indexed_witness);
        env.finalize();

        (env, wasm_runtime_io)
    }
}
