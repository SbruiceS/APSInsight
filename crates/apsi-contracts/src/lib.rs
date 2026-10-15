//! Integration contracts for ASPF products that are **planned and not yet created**:
//! AgriQL, KutajKutambi AI, AgriCUDA, APSNetwork and Nityaga RTOS, plus an optional
//! hardware-accelerator contract (for example GreenPlus FPGA).
//!
//! This crate defines, for each product:
//! - a versioned wire contract (serde types) with `validate()` rules,
//! - a port trait that a future adapter must implement,
//! - capability discovery through [`Adapter::discover`],
//! - a conformance harness ([`conformance`]) that any future adapter must pass.
//!
//! It deliberately contains **no** implementation of those products. The default
//! adapter for every product is [`Unavailable`], which reports itself as unavailable
//! and rejects every call with [`ContractError::Unavailable`]. APSInsight never
//! depends on these products being present.
//!
//! The one piece of real computation is [`agricuda::CpuFallback`]: the AgriCUDA
//! contract requires a CPU fallback with a numerical-tolerance contract, so the
//! fallback is implemented here and is what runs whenever no accelerator exists.

pub mod accelerator;
pub mod agricuda;
pub mod agriql;
pub mod apsnetwork;
pub mod common;
pub mod conformance;
pub mod kutajkutambi;
pub mod nityaga;

pub use common::{Adapter, Availability, CapabilityReport, ContractError, ContractVersion, Product, Unavailable, discover_all};
