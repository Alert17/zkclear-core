pub mod error;
pub mod merkle;
pub mod nullifier;
pub mod prover;
pub mod snark;
pub mod stark;

#[cfg(feature = "winterfell")]
pub mod air;

#[cfg(feature = "arkworks")]
pub mod circuit;

#[cfg(feature = "arkworks")]
pub mod keys;

#[cfg(feature = "winterfell")]
pub mod stark_proof;

#[cfg(test)]
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
mod tests;

pub use error::ProverError;
pub use prover::{Prover, ProverConfig};
