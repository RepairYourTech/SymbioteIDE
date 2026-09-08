//! Runtime integration boundary. Neither adapters nor providers own canonical completion.
mod adapter;
mod capabilities;
pub mod events;
pub mod provider;

pub use adapter::*;
pub use capabilities::*;

pub const SDK_VERSION: u32 = 1;
