#![doc = include_str!("../README.md")]

mod album;
mod api;
mod asset;
mod auth;
mod client;
mod host;
mod multipart;
mod url;
mod utils;

pub use album::{Album, Albums};
pub use api::upload;
pub use asset::{Asset, AssetRemoteStatus, AssetType};
pub use client::Client;
pub use utils::{DateTime, ImmichError, ImmichResult, User};
