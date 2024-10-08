use franklin_crypto::alt_babyjubjub::AltJubjubBn256;
use franklin_crypto::bellman::bn256::Bn256;
use franklin_crypto::bellman::PrimeField;
use franklin_crypto::jubjub::edwards;
use franklin_crypto::jubjub::edwards::Point;
use franklin_crypto::jubjub::Unknown;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use crate::runtime::monitor::observer::Observer;
use zkwasm_host_circuits::host::ForeignInst::Log;
use zkwasm_host_circuits::host::ForeignInst::LogChar;

use crate::foreign::log_helper::ExternalOutputForeignInst::*;
use crate::runtime::host::ForeignPlugin;

lazy_static::lazy_static! {
    pub static ref JUBJUB_PARAMS: AltJubjubBn256 = AltJubjubBn256::new();
}

struct Context;
impl ForeignContext for Context {}

pub enum ExternalOutputForeignInst {
    ExternalUnpackPublicKey =
        std::mem::variant_count::<zkwasm_host_circuits::host::ForeignInst>() as isize,
    ExternalLogTraceCount,
    ExternalLogHostTrace,

    ExternalLogStdoutWrite,
    ExternalLogStdoutFlush,
}

pub struct ExternalOutputContext {
    pub output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    pub current_key: u64,

    pub foreigns: Vec<Rc<ForeignPlugin>>,
}
impl ForeignContext for ExternalOutputContext {}

impl ExternalOutputContext {
    pub fn new(
        output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
        foreigns: Vec<Rc<ForeignPlugin>>,
    ) -> Self {
        ExternalOutputContext {
            output,
            current_key: 0,
            foreigns,
        }
    }

    pub fn default() -> ExternalOutputContext {
        ExternalOutputContext {
            output: Rc::new(RefCell::new(HashMap::new())),
            current_key: 0,
            foreigns: vec![],
        }
    }

    pub fn unpack_public_key(&self, address: u64) {
        let mut output = self.output.borrow_mut();
        let target = output.get_mut(&address).unwrap();

        let data_len = target.len();
        assert!(data_len >= 4, "unpack public key len < 4, {:?}", target);

        let data = &mut target[data_len - 4..data_len];

        let mut packed = [0u64; 4];
        packed.copy_from_slice(data);
        let mut packed_le = [0u8; 32];
        primitive_types::U256(packed).to_little_endian(&mut packed_le);

        let r: std::io::Result<Point<Bn256, Unknown>> =
            edwards::Point::read(packed_le.as_slice(), &JUBJUB_PARAMS as &AltJubjubBn256);
        if let Ok(r) = r {
            let (x, _y) = r.into_xy();
            let x = x.into_repr();

            data.copy_from_slice(&x.0);
        } else {
            log::error!("unpack err: {:?}", r.err());
        }
    }

    pub fn log_trace_count(&self, address: u64, trace_count: usize) {
        let mut output = self.output.borrow_mut();
        if !output.contains_key(&address) {
            output.insert(address, vec![]);
        }
        let target = output.get_mut(&address).unwrap();

        target.push(trace_count as u64);
    }

    pub fn log_host_trace(&self, address: u64) {
        let mut m = HashMap::new();
        for v in &self.foreigns {
            let plugin_name = &v.name;

            if !m.contains_key(plugin_name) {
                if let Some(stat) = (v.ctx).as_ref().borrow().get_statics(22) {
                    // we don't need max_round, so we can use a const K here.
                    m.insert(plugin_name.clone(), stat.used_round);
                }
            }
        }
        let result = serde_json::to_string(&m).unwrap();

        let mut output = self.output.borrow_mut();
        if !output.contains_key(&address) {
            output.insert(address, vec![]);
        }
        let target = output.get_mut(&address).unwrap();
        for b in result.as_bytes() {
            target.push(*b as u64);
        }
    }

    pub fn log_stdout_write(&self, address: u64) {
        let mut output = self.output.borrow_mut();
        if !output.contains_key(&address) {
            return;
        }
        let target = output
            .get(&address)
            .unwrap()
            .iter()
            .map(|&b| {
                assert!(b <= u8::MAX as u64);
                b as u8
            })
            .collect::<Vec<u8>>();

        std::io::stdout().write_all(&target).unwrap();

        output.remove(&address);
    }
}

pub fn register_log_foreign(env: &mut HostEnv) {
    let foreign_log_plugin = env
        .external_env
        .register_plugin("foreign_print", Box::new(Context));

    let print = Rc::new(
        |_observer: &Observer, _context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u64 = args.nth(0);
            println!("{}", value);
            None
        },
    );

    let printchar = Rc::new(
        |_observer: &Observer, _context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u64 = args.nth(0);
            print!("{}", value as u8 as char);
            None
        },
    );

    env.external_env.register_function(
        "wasm_dbg",
        Log as usize,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin.clone(),
        print,
    );

    env.external_env.register_function(
        "wasm_dbg_char",
        LogChar as usize,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin,
        printchar,
    );
}

// It should be the last registered function in HostEnv.
pub fn register_external_output_foreign(
    env: &mut HostEnv,
    external_output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
) {
    let foreigns = env
        .external_env
        .functions
        .values()
        .map(|f| f.plugin.clone())
        .collect::<Vec<_>>();

    let foreign_output_plugin = env.external_env.register_plugin(
        "foreign_external_output",
        Box::new(ExternalOutputContext::new(external_output, foreigns)),
    );

    let unpack_public_key = Rc::new(
        |_: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let address: u64 = args.nth(0);
            context.unpack_public_key(address);

            None
        },
    );

    let log_trace_count = Rc::new(
        |ob: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let address: u64 = args.nth(0);
            context.log_trace_count(address, ob.counter);

            None
        },
    );

    let log_host_trace = Rc::new(
        |_ob: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let address: u64 = args.nth(0);
            context.log_host_trace(address);

            None
        },
    );

    let log_to_stdout_write = Rc::new(
        |_ob: &Observer, context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let address: u64 = args.nth(0);
            context.log_stdout_write(address);

            None
        },
    );

    let log_to_stdout_flush = Rc::new(
        |_ob: &Observer, _context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
            std::io::stdout().flush().unwrap();

            Some(wasmi::RuntimeValue::I64(0))
        },
    );

    env.external_env.register_function(
        "wasm_external_unpack_public_key",
        ExternalUnpackPublicKey as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        unpack_public_key,
    );

    env.external_env.register_function(
        "wasm_external_log_trace_count",
        ExternalLogTraceCount as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        log_trace_count,
    );

    env.external_env.register_function(
        "wasm_external_log_host_trace",
        ExternalLogHostTrace as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        log_host_trace,
    );

    env.external_env.register_function(
        "wasm_external_log_stdout_write",
        ExternalLogStdoutWrite as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        log_to_stdout_write,
    );

    env.external_env.register_function(
        "wasm_external_log_stdout_flush",
        ExternalLogStdoutFlush as usize,
        ExternalHostCallSignature::Return,
        foreign_output_plugin.clone(),
        log_to_stdout_flush,
    );
}
