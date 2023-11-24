use franklin_crypto::alt_babyjubjub::AltJubjubBn256;
use franklin_crypto::bellman::bn256::Bn256;
use franklin_crypto::bellman::PrimeField;
use franklin_crypto::jubjub::edwards;
use franklin_crypto::jubjub::edwards::Point;
use franklin_crypto::jubjub::Unknown;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmi::tracer::Tracer;

use specs::external_host_call_table::ExternalHostCallSignature;

use crate::foreign::log_helper::ExternalOutputForeignInst::*;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use zkwasm_host_circuits::host::ForeignInst::Log;
use zkwasm_host_circuits::host::ForeignInst::LogChar;

lazy_static! {
    pub static ref JUBJUB_PARAMS: AltJubjubBn256 = AltJubjubBn256::new();
}

struct Context;
impl ForeignContext for Context {}

pub enum ExternalOutputForeignInst {
    ExternalOutputPush =
        std::mem::variant_count::<zkwasm_host_circuits::host::ForeignInst>() as isize,
    ExternalOutputPop,
    ExternalOutputAddress,

    ExternalUnpackPublicKey,

    ExternalLogTraceCount,
}

pub struct ExternalOutputContext {
    pub output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    pub current_key: u64,
    pub trace_counter: Option<Rc<RefCell<Option<Rc<RefCell<Tracer>>>>>>,
}
impl ForeignContext for ExternalOutputContext {}

impl ExternalOutputContext {
    pub fn new(
        output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
        trace_counter: Option<Rc<RefCell<Option<Rc<RefCell<Tracer>>>>>>,
    ) -> Self {
        ExternalOutputContext {
            output,
            current_key: 0,
            trace_counter,
        }
    }

    pub fn default() -> ExternalOutputContext {
        ExternalOutputContext {
            output: Rc::new(RefCell::new(HashMap::new())),
            current_key: 0,
            trace_counter: None,
        }
    }

    pub fn address(&mut self, k: u64) {
        self.current_key = k;
    }

    pub fn push(&self, v: u64) {
        let mut output = self.output.borrow_mut();
        if !output.contains_key(&self.current_key) {
            output.insert(self.current_key.clone(), vec![]);
        }
        let target = output.get_mut(&self.current_key).unwrap();
        target.push(v);
    }

    pub fn pop(&self) -> u64 {
        let mut output = self.output.borrow_mut();
        let target = output
            .get_mut(&self.current_key)
            .expect(&format!("can't get vec at key {}", self.current_key));

        target
            .pop()
            .expect(&format!("can't pop vec at key {}", self.current_key))
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

    pub fn log_trace_count(&self, address: u64) {
        let mut output = self.output.borrow_mut();
        if !output.contains_key(&address) {
            output.insert(address, vec![]);
        }
        let target = output.get_mut(&address).unwrap();

        let trace_count = if let Some(trace_counter) = &self.trace_counter {
            if trace_counter.borrow().is_some() {
                trace_counter
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .get_trace_count()
            } else {
                0
            }
        } else {
            0
        };

        target.push(trace_count as u64);
    }
}

pub fn register_log_foreign(env: &mut HostEnv) {
    let foreign_log_plugin = env
        .external_env
        .register_plugin("foreign_print", Box::new(Context));

    let print = Rc::new(
        |_context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u64 = args.nth(0);
            println!("{}", value);
            None
        },
    );

    let printchar = Rc::new(
        |_context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
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

pub fn register_external_output_foreign(
    env: &mut HostEnv,
    external_output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    trace_count: Option<Rc<RefCell<usize>>>,
) {
    let trace_counter = if trace_count.is_some() {
        Some(env.make_only_count_trace())
    } else {
        None
    };

    let foreign_output_plugin = env.external_env.register_plugin(
        "foreign_external_output",
        Box::new(ExternalOutputContext::new(external_output, trace_counter)),
    );

    let push_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();
            let value: u64 = args.nth(0);
            context.push(value);

            log::debug!("external output push: {}", value);

            None
        },
    );

    let pop_output = Rc::new(
        |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let ret = context.pop();

            log::debug!("external output pop: {}", ret);

            Some(wasmi::RuntimeValue::I64(ret as i64))
        },
    );

    let address_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let value: u64 = args.nth(0);
            context.address(value);

            log::debug!("external output address: {}", value);

            None
        },
    );

    let unpack_public_key = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let address: u64 = args.nth(0);
            context.unpack_public_key(address);

            None
        },
    );

    let log_trace_count = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<ExternalOutputContext>().unwrap();

            let address: u64 = args.nth(0);
            context.log_trace_count(address);

            None
        },
    );

    env.external_env.register_function(
        "wasm_external_output_push",
        ExternalOutputPush as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        push_output,
    );

    env.external_env.register_function(
        "wasm_external_output_pop",
        ExternalOutputPop as usize,
        ExternalHostCallSignature::Return,
        foreign_output_plugin.clone(),
        pop_output,
    );

    env.external_env.register_function(
        "wasm_external_output_address",
        ExternalOutputAddress as usize,
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        address_output,
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
}
