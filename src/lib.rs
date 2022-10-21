pub mod bench;
pub mod circuits;
pub mod cli;
pub mod foreign;
pub mod runtime;
pub mod test;
pub mod traits;

type Fr = halo2_proofs::pasta::Fp;
type G1Affine = halo2_proofs::pasta::EqAffine;

#[macro_use]
extern crate lazy_static;
extern crate downcast_rs;

// fn main() {
//     println!("Hello, world!");
// }
