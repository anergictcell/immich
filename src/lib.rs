#![doc = include_str!("../README.md")]

mod api;
mod asset;
mod auth;
mod client;
mod host;
mod multipart;
mod url;
mod utils;

pub use api::upload;
pub use asset::{Album, Albums, Asset, AssetRemoteStatus, AssetType, Owner};
pub use client::Client;
pub use utils::{DateTime, ImmichError, ImmichResult};
