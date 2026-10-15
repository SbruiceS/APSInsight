//! APSInsight core engine.
//!
//! The feature register (3,100 features) is the cross product of 100 agricultural
//! and platform domains with 31 capabilities. This crate implements each capability
//! once, generically, for every entity kind in [`catalog::catalog`].

pub mod audit;
pub mod auth;
pub mod bulk;
pub mod catalog;
pub mod dashboard;
pub mod docs;
pub mod engine;
pub mod error;
pub mod exchange;
pub mod ids;
pub mod metrics;
pub mod migration;
pub mod notify;
pub mod quality;
pub mod record;
pub mod retention;
pub mod schema;
pub mod search;
pub mod traceability;
pub mod validation;
pub mod workflow;

pub use engine::Engine;
pub use error::{ApsiError, Result};
