use crate::runtime::host::host_env::HostEnv;
use crate::runtime::host::ForeignContext;
use num_bigint::BigUint;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Instant;

use zkwasm_host_circuits::host::ForeignInst::JubjubSumNew;
use zkwasm_host_circuits::host::ForeignInst::JubjubSumPush;
use zkwasm_host_circuits::host::ForeignInst::JubjubSumResult;

use zkwasm_host_circuits::host::jubjub;

use super::babyjubjub_fq_to_limbs;
use super::fetch_g1;
use super::LIMBNB;

fn fetch_biguint(_limbs: &Vec<u64>) -> BigUint {
    BigUint::from_bytes_le(
        _limbs
            .iter()
            .map(|x| x.to_le_bytes())
            .flatten()
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

pub struct BabyJubjubSumContext {
    pub acc: jubjub::Point,
    pub limbs: Vec<u64>,
    pub coeffs: Vec<u64>,
    pub result_limbs: Option<Vec<u64>>,
    pub result_cursor: usize,
    pub input_cursor: usize,
}

static JUBJUB_SUM_TIMER: AtomicUsize = AtomicUsize::new(0);

pub fn print_jubjub_sum_profile() {
    println!("jubjub sum time elapsed:{:?}micros", JUBJUB_SUM_TIMER);
    JUBJUB_SUM_TIMER.store(0, Ordering::Relaxed);
}

static JUBJUB_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn print_jubjub_count() {
    println!("jubjub count:{:?}", JUBJUB_COUNT);
    JUBJUB_COUNT.store(0, Ordering::Relaxed);
}

impl BabyJubjubSumContext {
    pub fn default() -> Self {
        BabyJubjubSumContext {
            acc: jubjub::Point::identity(),
            limbs: vec![],
            coeffs: vec![],
            result_limbs: None,
            result_cursor: 0,
            input_cursor: 0,
        }
    }

    pub fn babyjubjub_sum_new(&mut self, new: usize) {
        JUBJUB_COUNT.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();
        self.result_limbs = None;
        self.result_cursor = 0;
        self.limbs = vec![];
        self.input_cursor = 0;
        self.coeffs = vec![];
        if new != 0 {
            self.acc = jubjub::Point::identity();
        }
        let elapsed = start.elapsed().as_micros();
        JUBJUB_SUM_TIMER.fetch_add(elapsed as usize, Ordering::Relaxed);
    }

    pub fn babyjubjub_sum_push(&mut self, v: u64) {
        let start = Instant::now();
        if self.input_cursor < LIMBNB * 2 {
            self.limbs.push(v);
            self.input_cursor += 1;
        } else if self.input_cursor < LIMBNB * 2 + 4 {
            self.coeffs.push(v);
            self.input_cursor += 1;
            if self.input_cursor == LIMBNB * 2 + 4 {
                self.input_cursor = 0;
            }
        }
        let elapsed = start.elapsed().as_micros();
        JUBJUB_SUM_TIMER.fetch_add(elapsed as usize, Ordering::Relaxed);
    }

    pub fn babyjubjub_sum_finalize(&mut self) -> u64 {
        let start = Instant::now();
        let limbs = self.result_limbs.clone();
        match limbs {
            None => {
                assert!(self.limbs.len() == LIMBNB * 2);
                let coeff = fetch_biguint(&self.coeffs.to_vec());
                let g1 = fetch_g1(&self.limbs.to_vec());
                self.acc = self
                    .acc
                    .projective()
                    .add(&g1.mul_scalar(&coeff).projective())
                    .affine();
                self.babyjubjub_result_to_limbs(self.acc.clone());
            }
            _ => (),
        };
        let ret = self.result_limbs.as_ref().unwrap()[self.result_cursor];
        self.result_cursor += 1;
        let elapsed = start.elapsed().as_micros();
        JUBJUB_SUM_TIMER.fetch_add(elapsed as usize, Ordering::Relaxed);
        ret
    }
}

impl BabyJubjubSumContext {
    fn babyjubjub_result_to_limbs(&mut self, g: jubjub::Point) {
        let mut limbs = vec![];
        babyjubjub_fq_to_limbs(&mut limbs, g.x);
        babyjubjub_fq_to_limbs(&mut limbs, g.y);
        self.result_limbs = Some(limbs);
    }
}

impl ForeignContext for BabyJubjubSumContext {}

use specs::external_host_call_table::ExternalHostCallSignature;
pub fn register_babyjubjubsum_foreign(env: &mut HostEnv) {
    let foreign_babyjubjubsum_plugin = env.external_env.register_plugin(
        "foreign_babyjubjubsum",
        Box::new(BabyJubjubSumContext::default()),
    );

    env.external_env.register_function(
        "babyjubjub_sum_new",
        JubjubSumNew as usize,
        ExternalHostCallSignature::Argument,
        foreign_babyjubjubsum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BabyJubjubSumContext>().unwrap();
                context.babyjubjub_sum_new(args.nth::<u64>(0) as usize);
                None
            },
        ),
    );

    env.external_env.register_function(
        "babyjubjub_sum_push",
        JubjubSumPush as usize,
        ExternalHostCallSignature::Argument,
        foreign_babyjubjubsum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BabyJubjubSumContext>().unwrap();
                context.babyjubjub_sum_push(args.nth(0));
                None
            },
        ),
    );

    env.external_env.register_function(
        "babyjubjub_sum_finalize",
        JubjubSumResult as usize,
        ExternalHostCallSignature::Return,
        foreign_babyjubjubsum_plugin.clone(),
        Rc::new(
            |context: &mut dyn ForeignContext, _args: wasmi::RuntimeArgs| {
                let context = context.downcast_mut::<BabyJubjubSumContext>().unwrap();
                let ret = Some(wasmi::RuntimeValue::I64(
                    context.babyjubjub_sum_finalize() as i64
                ));
                ret
            },
        ),
    );
}
