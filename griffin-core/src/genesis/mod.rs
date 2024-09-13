//! Utilities for blockchainchain genesis used by Tuxedo.

#[cfg(feature = "std")]
mod block_builder;

#[cfg(feature = "std")]
pub use block_builder::GriffinGenesisBlockBuilder;
