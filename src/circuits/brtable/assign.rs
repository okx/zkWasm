use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter},
    plonk::Error,
};
use specs::brtable::{BrTable, ElemTable};

use crate::circuits::{config::max_brtable_rows, utils::bn_to_field};

use super::BrTableChip;

impl<F: FieldExt> BrTableChip<F> {
    pub(in crate::circuits) fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        br_table_init: &BrTable,
        elem_table: &ElemTable,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        let mut ret = vec![];
        layouter.assign_region(
            || "br table",
            |mut table| {
                table.assign_advice_from_constant(
                    || "br table init",
                    self.config.col,
                    0,
                    F::zero(),
                )?;

                let mut offset = 1;

                for e in br_table_init.entries() {
                    let cell = table.assign_advice(
                        || "br table init",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&e.encode())),
                    )?;

                    ret.push(cell);

                    offset += 1;
                }

                for e in elem_table.entries() {
                    let cell = table.assign_advice(
                        || "call indirect init",
                        self.config.col,
                        offset,
                        || Ok(bn_to_field::<F>(&e.encode())),
                    )?;

                    ret.push(cell);

                    offset += 1;
                }

                while offset < max_brtable_rows() as usize {
                    let cell = table.assign_advice(
                        || "call indirect init",
                        self.config.col,
                        offset,
                        || Ok(F::zero()),
                    )?;

                    ret.push(cell);

                    offset += 1;
                }

                Ok(())
            },
        )?;

        Ok(ret)
    }
}
