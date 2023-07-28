use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use downcast_rs::{Downcast, impl_downcast};
use specs::external_host_call_table::ExternalHostCallSignature;
use crate::runtime::host::ForeignContext;
use crate::runtime::host::host_env::HostEnv;

pub trait HostFunction: Downcast {
    fn consume(&mut self, data: Vec<u8>);
}
impl_downcast!(HostFunction);

#[derive(Default)]
pub struct FunctionContext {
    methods: HashMap<u64, Rc<RefCell<Box<dyn HostFunction>>>>,
    length: u64,
    method: u64,
    data: Vec<u8>,

}

impl ForeignContext for FunctionContext {}

impl FunctionContext {
    pub fn set_length(&mut self, len: u64) {
        self.length = len;
    }
    pub fn set_method(&mut self, method: u64) {
        self.method = method;
    }
    pub fn receive(&mut self, data: u64) {
        self.data.extend(data.to_le_bytes());
        if self.data.len() >= self.length as usize {
            let data = self.data[0..self.length as usize].to_vec();
            let method = self.methods.get_mut(&self.method).unwrap();
            method.borrow_mut().consume(data);
            self.data.clear();
        }
    }
    pub fn register(&mut self, method: u64, func: Rc<RefCell<Box<dyn HostFunction>>>) {
        self.methods.insert(method, func);
    }
}

pub fn register_dispatch_foreign(env: &mut HostEnv, methods: Vec<(u64, Rc<RefCell<Box<dyn HostFunction>>>)>) {
    let mut context = FunctionContext::default();
    for (method, func) in methods {
        context.register(method, func);
    }
    let plugin = env
        .external_env
        .register_plugin("foreign_function_plugin", Box::new(context));


    // TODO: op_index should not be constant
    env.external_env.register_function(
        "dispatch_set_method",
        26,
        ExternalHostCallSignature::Argument,
        plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<FunctionContext>().unwrap();
                context.set_method(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "dispatch_set_length",
        27,
        ExternalHostCallSignature::Argument,
        plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<FunctionContext>().unwrap();
                context.set_length(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "dispatch_receive",
        28,
        ExternalHostCallSignature::Argument,
        plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<FunctionContext>().unwrap();
                context.receive(args.nth(0));
                None
            },
        ),
    );
}

pub fn array_to_u64(v: impl Into<Vec<u8>>) -> (Vec<u64>, u64) {
    let mut data = v.into();
    let len = data.len();
    let delta = len % 8;
    if delta != 0 {
        data.append(&mut vec![0; 8 - delta]);
    }
    let data: Vec<u64> = data
        .chunks_exact(8)
        .map(|x| u64::from_le_bytes(x.try_into().unwrap()))
        .collect();
    (data, len as u64)
}


#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::foreign::function_dispatcher::{array_to_u64, FunctionContext, HostFunction};

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
    pub fn test_dynamic() {
        let mut ctx = FunctionContext::default();
        let temp: Rc<RefCell<Box<dyn HostFunction>>> = Rc::new(RefCell::new(Box::new(Temp::default())));
        ctx.register(0, temp.clone());
        let data = "hello world";
        let (v, l) = array_to_u64(data);
        ctx.set_method(0);
        ctx.set_length(l);
        for d in v {
            ctx.receive(d);
        }
        let binding = temp.clone();
        let mut binding = binding.borrow_mut();
        let temp = binding.downcast_mut::<Temp>().unwrap();
        assert_eq!(temp.v, Some("hello world".to_string()))
    }
}
