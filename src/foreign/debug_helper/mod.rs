use std::rc::Rc;
use specs::external_host_call_table::ExternalHostCallSignature;
use crate::runtime::host::ForeignContext;
use crate::runtime::host::host_env::HostEnv;

pub enum DebugType {
    String = 0,
    Bytes = 1,
}
impl Default for DebugType{
    fn default() -> Self {
        Self::String
    }
}

impl DebugType {
    pub fn from_array(&mut self, data: &[u8]) {
        match self {
            DebugType::String => {
                println!("{:?}", String::from_utf8_lossy(data));
            }
            DebugType::Bytes => {
                println!("{:?}", data);
            }
        }
    }
}

#[derive(Default)]
pub struct DebugContext {
    pub debug_type: DebugType,
    pub data: Vec<u8>,
    pub length: u64,
}

impl ForeignContext for DebugContext {}

impl DebugContext {
    pub fn init(&mut self, debug_type: u64, l: u64) {
        self.debug_type = match debug_type {
            0 => DebugType::String,
            1 => DebugType::Bytes,
            _ => panic!("invalid debug type"),
        };
        self.length = l
    }
    pub fn push(&mut self, d: u64) {
        self.data.extend(d.to_le_bytes());
        if self.data.len() >= self.length as usize {
            let data = self.data[0..self.length as usize].to_vec();
            self.debug_type.from_array(data.as_slice());
            self.data.clear();
        }
    }
}

pub fn array_to_u64(v: impl Into<Vec<u8>>) -> (Vec<u64>, u64) {
    let mut  data = v.into();
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

pub fn register_debug_foreign(env: &mut HostEnv) {
    let plugin = env
        .external_env
        .register_plugin("debug_plugin", Box::new(DebugContext::default()));
    env.external_env.register_function(
        "debug_init",
        26 as usize,
        ExternalHostCallSignature::Argument,
        plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<DebugContext>().unwrap();
                context.init(args.nth(0),args.nth(1));
                None
            },
        ),
    );

    env.external_env.register_function(
        "push",
        27 as usize,
        ExternalHostCallSignature::Argument,
        plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<DebugContext>().unwrap();
                context.push(args.nth(0));
                None
            },
        ),
    );
}

#[test]
pub fn test_debug() {
    let msg = "hello world";
    let mut context = DebugContext::default();
    let (data, l) = array_to_u64(msg);
    context.init(DebugType::String as u64,l);
    for d in data {
        context.push(d)
    }
}