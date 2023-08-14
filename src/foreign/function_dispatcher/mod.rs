use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use downcast_rs::{Downcast, impl_downcast};
use specs::external_host_call_table::ExternalHostCallSignature;
use crate::runtime::host::ForeignContext;
use crate::runtime::host::host_env::HostEnv;

pub trait LiteFunction<T>: 'static {
    fn consume(&mut self, data: Vec<T>);
}

pub trait HostFunction: Downcast {
    fn consume(&mut self);
    fn store(&mut self, data: u64);
    fn cut(&mut self, size: usize);
    fn u64_size(&self) -> usize;
}
impl_downcast!(HostFunction);

pub struct BytesFunction<T: LiteFunction<u8>> {
    data: Vec<u8>,
    pub internal: T,
}

impl<T: LiteFunction<u8>> BytesFunction<T> {
    pub fn new(internal: T) -> Self {
        Self { data: Default::default(), internal }
    }
}

impl<T: LiteFunction<u8>> HostFunction for BytesFunction<T> {
    fn consume(&mut self) {
        self.internal.consume(self.data.clone());
    }

    fn store(&mut self, data: u64) {
        self.data.extend(data.to_le_bytes());
    }

    fn cut(&mut self, size: usize) {
        let l = self.data.len();
        self.data = self.data[0..(l - size)].to_vec();
    }

    fn u64_size(&self) -> usize {
        8
    }
}

pub struct U64Function<T: LiteFunction<u64>> {
    data: Vec<u64>,
    pub internal: T,
}

impl<T: LiteFunction<u64>> U64Function<T> {
    pub fn new(internal: T) -> Self {
        Self { data: Default::default(), internal }
    }
}

impl<T: LiteFunction<u64>> HostFunction for U64Function<T> {
    fn consume(&mut self) {
        self.internal.consume(self.data.clone());
    }

    fn store(&mut self, data: u64) {
        self.data.push(data);
    }

    fn cut(&mut self, _: usize) {
        unreachable!()
    }

    fn u64_size(&self) -> usize {
        1
    }
}

/*

pub fn dispatch(method: u64, v: impl Into<Vec<u8>>) {
    let (data, l) = array_to_u64(v);
    unsafe {
        dispatch_set_method(method);
        dispatch_set_length(l);
        data.into_iter()
            .for_each(|x| dispatch_receive(x));
    }
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

extern "C" {
    fn dispatch_set_method(method: u64);
    fn dispatch_set_length(l: u64);
    fn dispatch_receive(v: u64);
}
 */
#[derive(Default)]
pub struct FunctionContext {
    methods: HashMap<u64, FunctionWrapper>,
    method: u64,
}

pub struct FunctionWrapper {
    fun: Rc<RefCell<Box<dyn HostFunction>>>,
    length: u64,
    current_size: usize,
}

impl FunctionWrapper {
    pub fn new(fun: Rc<RefCell<Box<dyn HostFunction>>>) -> Self {
        Self { fun, length: 0, current_size: 0 }
    }
}


impl ForeignContext for FunctionContext {}

impl FunctionContext {
    pub fn set_length(&mut self, len: u64) {
        let wrapper = self.get_current_method();
        wrapper.length = len;
    }
    pub fn set_method(&mut self, method: u64) {
        self.method = method;
    }
    fn get_current_method(&mut self) -> &mut FunctionWrapper {
        let wrapper = self.methods.get_mut(&self.method).unwrap();
        wrapper
    }
    pub fn receive(&mut self, data: u64) {
        let wrapper = self.get_current_method();
        let mut method = (&wrapper.fun).borrow_mut();
        let size = method.u64_size();
        wrapper.current_size += size;
        method.store(data);
        if wrapper.current_size >= wrapper.length as usize {
            let delta = wrapper.current_size - wrapper.length as usize;
            if delta > 0 {
                method.cut(delta);
            }
            method.consume();

            wrapper.current_size = 0;
            wrapper.length = 0;
        }
    }

    pub fn register(&mut self, method: u64, func: Rc<RefCell<Box<dyn HostFunction>>>) {
        self.methods.insert(method, FunctionWrapper::new(func));
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
        100,
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
        101,
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
        102,
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
    use crate::foreign::function_dispatcher::{array_to_u64, BytesFunction, FunctionContext, HostFunction, LiteFunction};

    #[derive(Default)]
    pub struct Temp {
        v: Option<String>,
    }

    impl LiteFunction<u8> for Temp {
        fn consume(&mut self, data: Vec<u8>) {
            self.v = Some(String::from_utf8_lossy(data.as_slice()).to_string());
            println!("{:?}", &self.v);
        }
    }

    #[test]
    pub fn test_dynamic() {
        let mut ctx = FunctionContext::default();
        let temp: Rc<RefCell<Box<dyn HostFunction>>> = Rc::new(RefCell::new(Box::new(BytesFunction::new(Temp::default()))));
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
        let temp = binding.downcast_mut::<BytesFunction<Temp>>().unwrap();
        assert_eq!(temp.internal.v, Some("hello world".to_string()))
    }
}
