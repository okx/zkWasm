use std::rc::Rc;
use crate::runtime::host::{host_env::HostEnv, ForeignContext};
use zkwasm_host_circuits::host::merkle::MerkleTree;
use zkwasm_host_circuits::host::kvpair as kvpairhelper;

#[derive(Default)]
struct KVPairContext {
    pub address_limbs: Vec<u64>,
    pub value_limbs: Vec<u64>,
    pub result_limbs: Vec<u64>,
    pub input_cursor: usize,
    pub result_cursor: usize,
}

const ADDRESS_LIMBNB:usize = 2 + 1; //4 for db id and 1 for address
const VALUE_LIMBNB:usize = 4;
const KVPAIR_ADDR:usize= 7;
const KVPAIR_SET:usize= 8;
const KVPAIR_GET:usize= 9;


impl KVPairContext {
}

impl ForeignContext for KVPairContext {}

fn get_merkle_db_address(address_limbs: &Vec<u64>) -> ([u8; 32], u64) {
    let (id, address) = address_limbs.split_at(2);
    let mut id = id.iter().fold(vec![], |acc:Vec<u8>, x| {
        let mut v = acc.clone();
        let mut bytes: Vec<u8> = x.to_le_bytes().to_vec();
        v.append(&mut bytes);
        v
    });
    id.append(&mut [0u8;16].to_vec());
    (id.try_into().unwrap(), address[0])
}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_bn254pair_foreign(env: &mut HostEnv) {
    let foreign_kvpair_plugin = env
            .external_env
            .register_plugin("foreign_kvpair", Box::new(KVPairContext::default()));

    env.external_env.register_function(
        "kvpair_addr",
        KVPAIR_ADDR,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.input_cursor < ADDRESS_LIMBNB {
                    context.address_limbs.push(args.nth(0));
                    context.input_cursor += 1;
                } else {
                    context.input_cursor = 0;
                }
                None
            },
        ),
    );

    env.external_env.register_function(
        "kvpair_set",
        KVPAIR_SET,
        ExternalHostCallSignature::Argument,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.input_cursor < VALUE_LIMBNB {
                    context.value_limbs.push(args.nth(0));
                    context.input_cursor += 1;
                } else {
                    let (id, address) = get_merkle_db_address(&context.address_limbs);
                    let mut kv = kvpairhelper::MongoMerkle::construct(id.try_into().unwrap());
                    let bytes = context.value_limbs.iter().fold(vec![], |acc:Vec<u8>, x| {
                        let mut v = acc.clone();
                        let mut bytes: Vec<u8> = x.to_le_bytes().to_vec();
                        v.append(&mut bytes);
                        v
                    });
                    let index = address as u32;
                    kv.update_leaf_data_with_proof(index, &bytes)
                        .expect("Unexpected failure: update leaf with proof fail");
                    context.input_cursor = 0;

                }
                None
            },
        ),
    );


    env.external_env.register_function(
        "kv254pair_get",
        KVPAIR_GET,
        ExternalHostCallSignature::Return,
        foreign_kvpair_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<KVPairContext>().unwrap();
                if context.result_cursor == 0 {
                    let (id, address) = get_merkle_db_address(&context.address_limbs);
                    let kv = kvpairhelper::MongoMerkle::construct(id);
                    let index = address as u32;
                    let leaf = kv.get_leaf(index)
                        .expect("Unexpected failure: get leaf fail");
                    context.result_limbs = leaf.data_as_u64().to_vec();
                    context.input_cursor = 0;
                }
                let ret = Some(wasmi::RuntimeValue::I64(context.result_limbs[context.result_cursor] as i64));
                context.result_cursor += 1;
                ret
            },
        ),
    );
}
