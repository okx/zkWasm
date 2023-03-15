use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter},
    plonk::{Advice, Column, ConstraintSystem, Error, Fixed, Instance},
};

use super::CircuitConfigure;

// The target is to implement uniform wasm image verifier,
// The key is to make image-related fixed value into advice,
// Then hash(itable, imtable, brtable, static_frame_table, start, first_consecutive_zero_memory_offset) into a checksum
// And expose the checksum as an instance

#[derive(Clone)]
pub struct CheckSumConfig<F> {
    pub(crate) _checksum: Column<Instance>,
    pub(crate) _data: Column<Advice>,
    _sel: Column<Fixed>,
    _mark: PhantomData<F>,
}

pub struct CheckSumChip<F: FieldExt> {
    pub(crate) _config: CheckSumConfig<F>,
}

impl<F: FieldExt> CheckSumConfig<F> {
    fn _configure(meta: &mut ConstraintSystem<F>) -> Self {
        CheckSumConfig {
            _checksum: meta.instance_column(),
            _data: meta.advice_column(),
            _sel: meta.fixed_column(),
            _mark: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let config = Self::_configure(meta);
        // TODO: Add constraints between data with checksum

        config
    }
}

impl<F: FieldExt> CheckSumChip<F> {
    pub fn new(config: CheckSumConfig<F>) -> Self {
        Self { _config: config }
    }
    pub fn assign(
        self,
        _layouter: &mut impl Layouter<F>,
        _instructions: Vec<AssignedCell<F, F>>,
        _init_memory: Vec<AssignedCell<F, F>>,
        _br_entries: Vec<AssignedCell<F, F>>,
        _circuit_configure: &CircuitConfigure,
    ) -> Result<(), Error> {
        // TODO
        Ok(())
    }
}
