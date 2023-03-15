use crate::curr;

use super::{config::max_imtable_rows, utils::bn_to_field};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use specs::{
    encode::init_memory_table::encode_init_memory_table_entry, imtable::InitMemoryTable,
    mtable::LocationType,
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct InitMemoryTableConfig<F: FieldExt> {
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InitMemoryTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        Self {
            col: meta.advice_column(),
            _mark: PhantomData,
        }
    }

    pub fn encode(
        &self,
        is_mutable: Expression<F>,
        ltype: Expression<F>,
        offset: Expression<F>,
        value: Expression<F>,
    ) -> Expression<F> {
        encode_init_memory_table_entry(ltype, is_mutable, offset, value)
    }

    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), curr!(meta, self.col))]);
    }
}

pub struct MInitTableChip<F: FieldExt> {
    config: InitMemoryTableConfig<F>,
}

impl<F: FieldExt> MInitTableChip<F> {
    pub fn new(config: InitMemoryTableConfig<F>) -> Self {
        MInitTableChip { config }
    }

    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        memory_init_table: &InitMemoryTable,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        let mut ret = vec![];

        layouter.assign_region(
            || "memory init",
            |mut table| {
                table.assign_advice_from_constant(
                    || "memory init table",
                    self.config.col,
                    0,
                    F::zero(),
                )?;

                let heap_entries = memory_init_table.filter(LocationType::Heap);
                let global_entries = memory_init_table.filter(LocationType::Global);

                let mut idx = 1;

                for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                    let cell = table.assign_advice(
                        || "memory init table",
                        self.config.col,
                        idx,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;

                    ret.push(cell);

                    idx += 1;
                }

                while idx < max_imtable_rows() as usize {
                    let cell = table.assign_advice(
                        || "memory init table",
                        self.config.col,
                        idx,
                        || Ok(F::zero()),
                    )?;

                    ret.push(cell);

                    idx += 1;
                }

                Ok(())
            },
        )?;

        Ok(ret)
    }
}
