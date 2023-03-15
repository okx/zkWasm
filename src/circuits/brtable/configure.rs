use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Expression, VirtualCells},
};
use std::marker::PhantomData;

use super::BrTableConfig;
use crate::{circuits::traits::ConfigureLookupTable, curr};

impl<F: FieldExt> BrTableConfig<F> {
    pub(in crate::circuits) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        Self {
            col: meta.advice_column(),
            _mark: PhantomData,
        }
    }
}

impl<F: FieldExt> ConfigureLookupTable<F> for BrTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), curr!(meta, self.col))]);
    }
}
