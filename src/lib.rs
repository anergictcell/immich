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

pub mod takeout;

pub use album::{Album, Albums};
pub use api::requests::{AssetMoveError, MovedAsset};
pub use api::upload;
pub use asset::{Asset, AssetId, AssetRemoteStatus, AssetType};
pub use client::Client;
pub use utils::{DateTime, ImmichError, ImmichResult, User};
