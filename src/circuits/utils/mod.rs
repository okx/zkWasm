use halo2_proofs::{arithmetic::FieldExt, circuit::Region};
use num_bigint::BigUint;

pub mod bitvalue;
pub mod bytes8;
pub mod row_diff;
pub mod u16;
pub mod u32;
pub mod u64;
pub mod u8;

pub struct Context<'a, F: FieldExt> {
    pub region: Box<Region<'a, F>>,
    pub offset: usize,
    records: Vec<usize>,
}

impl<'a, F: FieldExt> Context<'a, F> {
    pub fn new(region: Region<'a, F>) -> Self {
        Self {
            region: Box::new(region),
            offset: 0usize,
            records: vec![],
        }
    }

    pub fn next(&mut self) {
        self.offset += 1;
    }

    pub fn reset(&mut self) {
        self.offset = 0;
        self.records.clear();
    }

    pub fn push(&mut self) {
        self.records.push(self.offset)
    }

    pub fn pop(&mut self) {
        self.offset = self.records.pop().unwrap();
    }
}

pub fn field_to_bn<F: FieldExt>(f: &F) -> BigUint {
    BigUint::from_bytes_le(f.to_repr().as_ref())
}

pub fn bn_to_field<F: FieldExt>(bn: &BigUint) -> F {
    let mut bytes = bn.to_bytes_le();
    bytes.resize(64, 0);
    F::from_bytes_wide(&bytes.try_into().unwrap())
}

#[macro_export]
macro_rules! curr {
    ($meta: expr, $x: expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation::cur())
    };
}

#[macro_export]
macro_rules! prev {
    ($meta: expr, $x: expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation::prev())
    };
}

#[macro_export]
macro_rules! next {
    ($meta: expr, $x: expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation::next())
    };
}

#[macro_export]
macro_rules! nextn {
    ($meta: expr, $x: expr, $n:expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation($n))
    };
}

#[macro_export]
macro_rules! fixed_curr {
    ($meta: expr, $x: expr) => {
        $meta.query_fixed($x, halo2_proofs::poly::Rotation::cur())
    };
}

#[macro_export]
macro_rules! instance_curr {
    ($meta: expr, $x: expr) => {
        $meta.query_instance($x, halo2_proofs::poly::Rotation::cur())
    };
}

#[macro_export]
macro_rules! fixed_prev {
    ($meta: expr, $x: expr) => {
        $meta.query_fixed($x, halo2_proofs::poly::Rotation::prev())
    };
}

#[macro_export]
macro_rules! fixed_next {
    ($meta: expr, $x: expr) => {
        $meta.query_fixed($x, halo2_proofs::poly::Rotation::next())
    };
}

#[macro_export]
macro_rules! constant_from {
    ($x: expr) => {
        halo2_proofs::plonk::Expression::Constant(F::from($x as u64))
    };
}

#[macro_export]
macro_rules! constant_from_bn {
    ($x: expr) => {
        halo2_proofs::plonk::Expression::Constant(bn_to_field($x))
    };
}

#[macro_export]
macro_rules! constant {
    ($x: expr) => {
        halo2_proofs::plonk::Expression::Constant($x)
    };
}
