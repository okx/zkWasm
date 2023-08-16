use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use specs::external_host_call_table::ExternalHostCallSignature;

use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use zkwasm_host_circuits::host::ForeignInst::Log;

struct Context;
impl ForeignContext for Context {}

pub static mut  OUTPUT_CONTEXT:OutputContext=OutputContext::default();

pub struct OutputContext {
    pub output: Rc<RefCell<HashMap<u64, Vec<u64>>>>,
    pub current_key: u64,
}
impl ForeignContext for OutputContext {}

impl OutputContext {
    pub fn new(output: Rc<RefCell<HashMap<u64, Vec<u64>>>>) -> Self {
        OutputContext { output, current_key: 0 }
    }

    pub const fn default() -> OutputContext {
        OutputContext {
            output: Rc::new(RefCell::new(HashMap::new())),
            current_key: 0,
        }
    }

    pub fn switch_key(&mut self, k: u64) {
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
        if let Some(target) = output.get_mut(&self.current_key) {
            return target.pop().unwrap_or_default();
        }

        0
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

    env.external_env.register_function(
        "wasm_dbg",
        Log as usize,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin,
        print,
    );
}

pub fn register_log_output_foreign(env: &mut HostEnv) {
    let outputs =  env.log_outputs.clone();
    let foreign_output_plugin = env
        .external_env
        .register_plugin("foreign_log_output", Box::new(OutputContext::new(outputs)));
    unsafe {
        OUTPUT_CONTEXT.output= outputs.clone();
    }
    let push_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<OutputContext>().unwrap();
            let value: u64 = args.nth(0);
            context.push(value);

            log::debug!("internal output: {}", value);

            None
        },
    );

    let pop_output = Rc::new(
        |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<OutputContext>().unwrap();

            let ret = context.pop();

            log::debug!("pop internal output: {}", ret);

            Some(wasmi::RuntimeValue::I64(ret as i64))
        },
    );

    let switch_output = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let context = context.downcast_mut::<OutputContext>().unwrap();

            let value: u64 = args.nth(0);
            context.switch_key(value);

            log::debug!("switch internal output: {}", value);

            None
        },
    );

    env.external_env.register_function(
        "wasm_log_output",
        std::mem::variant_count::<zkwasm_host_circuits::host::ForeignInst>(),
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        push_output,
    );

    env.external_env.register_function(
        "wasm_log_output_pop",
        std::mem::variant_count::<zkwasm_host_circuits::host::ForeignInst>(),
        ExternalHostCallSignature::Return,
        foreign_output_plugin.clone(),
        pop_output,
    );

    env.external_env.register_function(
        "wasm_log_output_switch",
        std::mem::variant_count::<zkwasm_host_circuits::host::ForeignInst>(),
        ExternalHostCallSignature::Argument,
        foreign_output_plugin.clone(),
        switch_output,
    );
}

pub fn get_data(k:u64)->Option<u64>{
    unsafe {
        let ret=OUTPUT_CONTEXT.output.borrow_mut().get(&k);
        if let Some(mut ret)=ret{
            ret.pop()
        }
        None
    }
}