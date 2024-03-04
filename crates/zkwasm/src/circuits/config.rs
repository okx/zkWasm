use std::env;
use std::sync::RwLock;

pub const POW_TABLE_POWER_START: u64 = 128;
pub const MIN_K: u32 = 18;

lazy_static! {
    static ref ZKWASM_K: RwLock<u32> =
        RwLock::new(env::var("ZKWASM_K").map_or(MIN_K, |k| k.parse().unwrap()));
}

pub fn set_zkwasm_k(k: u32) {
    println!("set set_zkwasm_k called here");
    assert!(k >= MIN_K);
    let mut zkwasm_k = ZKWASM_K.write().unwrap();
    *zkwasm_k = k;
}

pub fn zkwasm_k() -> u32 {
    *ZKWASM_K.read().unwrap()
}

pub fn init_zkwasm_runtime(k: u32) {
    set_zkwasm_k(k);
}

pub(crate) fn max_image_table_rows() -> u32 {
    8192
}
