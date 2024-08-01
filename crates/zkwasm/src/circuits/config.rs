use std::env;
use std::sync::atomic::{AtomicU32, Ordering};

pub const POW_TABLE_POWER_START: u64 = 128;

pub const MIN_K: u32 = 18;
const MAX_K: u32 = 22;

lazy_static! {
    static ref ZKWASM_K: AtomicU32 =
        AtomicU32::new(env::var("ZKWASM_K").map_or(MIN_K, |k| k.parse().unwrap()));
}

pub(crate) fn set_zkwasm_k(k: u32) {
    assert!(k >= MIN_K);
    assert!(k <= MAX_K);

    ZKWASM_K.store(k, Ordering::Relaxed);
}

pub(in crate::circuits) fn zkwasm_k() -> u32 {
    ZKWASM_K.load(Ordering::Relaxed)
}

pub(crate) fn init_zkwasm_runtime(k: u32) {
    set_zkwasm_k(k);
}

pub(crate) fn common_range(k: u32) -> u32 {
    (1 << k) - 256
}

pub(crate) fn common_range_max(k: u32) -> u32 {
    common_range(k) - 1
}
