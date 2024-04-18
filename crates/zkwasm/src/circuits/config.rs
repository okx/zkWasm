use std::env;
use std::sync::atomic::{AtomicU32, Ordering};

pub const POW_TABLE_POWER_START: u64 = 128;

pub const MIN_K: u32 = 18;
const MAX_K: u32 = 25;

lazy_static! {
    static ref ZKWASM_K: AtomicU32 =
        AtomicU32::new(env::var("ZKWASM_K").map_or(MIN_K, |k| k.parse().unwrap()));
}

pub fn set_zkwasm_k(k: u32) {
    assert!(k >= MIN_K);
    assert!(k <= MAX_K);

    ZKWASM_K.store(k, Ordering::Relaxed);
}

pub fn zkwasm_k() -> u32 {
    ZKWASM_K.load(Ordering::Relaxed)
}

pub fn init_zkwasm_runtime(k: u32) {
    set_zkwasm_k(k);
}

pub(crate) fn max_image_table_rows() -> u32 {
    8192
}
