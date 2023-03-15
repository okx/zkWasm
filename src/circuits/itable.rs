use crate::{circuits::config::max_itable_rows, curr};

use super::utils::bn_to_field;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use specs::itable::InstructionTable;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct InstructionTableConfig<F: FieldExt> {
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InstructionTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        InstructionTableConfig {
            col: meta.advice_column(),
            _mark: PhantomData,
        }
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

#[derive(Clone)]
pub struct InstructionTableChip<F: FieldExt> {
    config: InstructionTableConfig<F>,
}

impl<F: FieldExt> InstructionTableChip<F> {
    pub fn new(config: InstructionTableConfig<F>) -> Self {
        InstructionTableChip { config }
    }

    pub fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        instructions: &InstructionTable,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        let max_rows = max_itable_rows() as usize;

        assert!(instructions.entries().len() + 1 <= max_rows);

        let mut assigned_instructions = vec![];
        layouter.assign_region(
            || "instruction table",
            |mut table| {
                table.assign_advice_from_constant(
                    || "memory init table",
                    self.config.col,
                    0,
                    F::zero(),
                )?;

                for (i, v) in instructions.entries().iter().enumerate() {
                    let c = table.assign_advice(
                        || "instruction",
                        self.config.col,
                        i + 1,
                        || Ok(bn_to_field::<F>(&v.encode())),
                    )?;
                    assigned_instructions.push(c)
                }

                // padding
                for i in instructions.entries().len() + 1..max_rows {
                    let c = table.assign_advice(
                        || "instruction",
                        self.config.col,
                        i + 1,
                        || Ok(F::zero()),
                    )?;
                    assigned_instructions.push(c)
                }

                Ok(())
            },
        )?;

        Ok(assigned_instructions)
    }
}
