use std::rc::Rc;
use specs::external_host_call_table::ExternalHostCallSignature;
use zkwasm_host_circuits::host::ForeignInst::JubjubSumResult;
use crate::foreign::error_code_helper::set_code_index;
use crate::runtime::host::ForeignContext;
use crate::runtime::host::host_env::HostEnv;

impl ForeignContext for ErrorCodeContext {}

pub struct ErrorCodeContext {}


impl ErrorCodeContext {
    pub fn new() -> Self {
        ErrorCodeContext {}
    }
}


pub fn register_error_code_foreign(env: &mut HostEnv) {
    let foreign_log_plugin = env
        .external_env
        .register_plugin("foreign_error_code", Box::new(ErrorCodeContext::new()));

    let record = Rc::new(
        |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
            let value: u64 = args.nth(0);
            let (code, index) = split_error_and_index(value);
            set_code_index(code, index);

            None
        },
    );
    let op_index = 100;
    env.external_env.register_function(
        "record_error_code",
        op_index,
        ExternalHostCallSignature::Argument,
        foreign_log_plugin,
        record,
    );
}

pub fn split_error_and_index(value: u64) -> (u32, u32) {
    let code = (value >> 32) as u32;
    let index = (value & 0xffffffff) as u32;
    (code, index)
}

pub fn merge_error_and_index(error_code: u32, index: u32) -> u64 {
    let code = (error_code as u64) << 32;
    let index = index as u64;
    code | index
}

#[test]
pub fn test_merge() {
    let error_code = 1;
    let index = 2;
    let value = merge_error_and_index(error_code, index);
    let (code, index) = split_error_and_index(value);
    assert_eq!(code, error_code);
    assert_eq!(index, index);
}