use halo2_proofs::{arithmetic::FieldExt, plonk::Expression};
use num_bigint::BigUint;
use std::ops::{Add, Mul};

pub mod opcode;

pub trait FromBn: Sized + Add<Self, Output = Self> + Mul<Self, Output = Self> {
    fn zero() -> Self;
    fn from_bn(bn: &BigUint) -> Self;
}

impl FromBn for BigUint {
    fn zero() -> Self {
        BigUint::from(0u64)
    }

    fn from_bn(bn: &BigUint) -> Self {
        bn.clone()
    }
}

impl<F: FieldExt> FromBn for Expression<F> {
    fn from_bn(bn: &BigUint) -> Self {
        let mut bytes = bn.to_bytes_le();
        bytes.resize(64, 0);
        let f = F::from_bytes_wide(&bytes.try_into().unwrap());
        halo2_proofs::plonk::Expression::Constant(f)
    }

    fn zero() -> Self {
        halo2_proofs::plonk::Expression::Constant(F::zero())
    }
}
