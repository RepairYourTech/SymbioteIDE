//! Version-one transport contracts and pure state transitions, not a Host implementation.
//! Aggregate deserialization validates replay and dispatch invariants. Cross-record
//! authentication/provenance remain Host responsibilities; wire data is never authorization.

mod dependency;
mod dispatch;
mod ids;
mod lease;
mod lifecycle;
mod model;
mod team;
mod work;

pub use dependency::*;
pub use dispatch::*;
pub use ids::*;
pub use lease::*;
pub use lifecycle::*;
pub use model::*;
pub use team::*;
pub use work::*;

/// Bump for incompatible wire changes. Version one has no persistent migration yet.
pub const SCHEMA_VERSION: u32 = 1;
