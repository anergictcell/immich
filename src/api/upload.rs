use std::fmt::Display;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use serde::Deserialize;
use ureq::Response;

use crate::client::ImmichClient;
use crate::{multipart::MultipartBuilder, Asset, Client, ImmichResult};
use crate::{AssetId, ImmichError};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
/// Response status of an asset upload
pub enum Status {
    #[serde(rename(deserialize = "created"))]
    Created,
    #[serde(rename(deserialize = "duplicate"))]
    Duplicate,
    Failure,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Status::Created => "Created",
                Status::Duplicate => "Duplicate",
                Status::Failure => "Failure",
            }
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
/// Response of the Immich server for an uploaded asset
pub struct Uploaded {
    status: Status,
    id: AssetId,
    #[serde(default)]
    device_asset_id: String,
}

impl Uploaded {
    pub(crate) fn from_failure(device_asset_id: &str) -> Self {
        Self {
            status: Status::Failure,
            id: AssetId::default(),
            device_asset_id: String::from(device_asset_id),
        }
    }

    /// Returns the id of the uploaded/checked [`Asset`]
    pub fn id(&self) -> &AssetId {
        &self.id
    }

    /// Returns the client-side id of the uploaded/checked [`Asset`]
    pub fn device_asset_id(&self) -> &str {
        &self.device_asset_id
    }

    /// Returns the response status of the upload
    ///
    /// This can be either
    /// - `created`: The asset was uploaded successfully
    /// - `duplicate`: The asset did already exist on the Immich server and was not uploaded
    /// - `failure`: : The upload of the asset failed
    pub fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn device_asset_id_mut(&mut self) -> &mut String {
        self.device_asset_id.clear();
        &mut self.device_asset_id
    }
}

pub(crate) struct Upload {}

impl Upload {
    const URL: &str = "/assets";

    pub fn post(client: &Client, asset: &Asset) -> ImmichResult<Response> {
        let (content_type, data) = Upload::format_data(asset)?;

        let response = client
            .post(Upload::URL)
            .set("Content-Type", &content_type)
            .set("x-immich-checksum", &asset.checksum())
            .auth(client.auth())
            .send_bytes(&data)?;
        Ok(response)
    }

    fn format_data(asset: &Asset) -> ImmichResult<(String, Vec<u8>)> {
        Ok(MultipartBuilder::new()
            .add_text("deviceAssetId", asset.device_asset_id())?
            .add_text("deviceId", asset.device_id())?
            .add_text("fileCreatedAt", &asset.created_at().to_string())?
            .add_text("fileModifiedAt", &asset.modified_at().to_string())?
            .add_bytes(
                asset.asset_data(),
                "assetData",
                Some(asset.device_asset_id()),
            )?
            .finish()?)
    }
}

pub(crate) struct ParallelUpload {
    threads: usize,
}

impl Default for ParallelUpload {
    fn default() -> Self {
        Self::new(5)
    }
}

impl ParallelUpload {
    pub fn new(threads: usize) -> Self {
        Self { threads }
    }

    fn upload(
        &self,
        receiver: Receiver<Asset>,
        sender: Sender<Uploaded>,
        client: &Client,
    ) -> Vec<JoinHandle<()>> {
        (0..self.threads)
            .map(|_| {
                let rec = receiver.clone();
                let res = sender.clone();
                let client = client.clone();

                thread::spawn(move || {
                    while let Ok(mut asset) = rec.recv() {
                        let _ = match asset.upload(&client) {
                            Ok(response) => res.send(response),
                            Err(_) => res.send(Uploaded::from_failure(asset.device_asset_id())),
                        };
                    }
                })
            })
            .collect()
    }

    pub fn post<I: Iterator<Item = Asset>>(
        &self,
        client: &Client,
        assets: I,
        feedback: Option<Sender<Uploaded>>,
    ) -> ImmichResult<Vec<Uploaded>> {
        let (asset_sender, asset_receiver) = bounded::<Asset>(self.threads * 2);

        let (result_sender, result_receiver) = unbounded::<Uploaded>();

        let threads = self.upload(asset_receiver, result_sender, client);

        let results = thread::spawn(move || {
            let mut result = Vec::new();
            while let Ok(response) = result_receiver.recv() {
                result.push(response.clone());
                if let Some(channel) = &feedback {
                    channel
                        .send(response)
                        .expect("The feedback channel must remain open throughout");
                }
            }
            result
        });

        for asset in assets {
            asset_sender.send(asset)?
        }
        drop(asset_sender);

        for thread in threads {
            thread.join().map_err(|_| ImmichError::Multithread)?
        }

        results.join().map_err(|_| ImmichError::Multithread)
    }
}
