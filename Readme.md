# Immich Client for Rust

This is an (early stage and inofficial) Rust client for [Immich](https://immich.app/).

This library provides a simple interface to connect to the Immich REST API for basic operations, such as uploading images or videos.
At the moment, this is very much work in progress and I'm currently focussing on the features I needd myself.

This is currently a toy weekend and evening project for me, so there is no guarantee that things will work as expected - although they currently do work quite well.

## Features

- Get a list of all albums on the server
- Upload a single image or video
- Upload many images or videos in parallel

## Some examples

### List all albums on the server
```rust
use immich::{Asset, Client};

fn example() {
    let client = Client::with_email(
        "https://immich-web-url/api",
        "email@somewhere",
        "s3cr3tpassword"
    ).unwrap();

    for album in client.albums().unwrap() {
        println!("{}: {} assets", album.name(), album.len());
    }
}
```

### Upload a single image or video
```rust
use immich::{Asset, Client};

fn example() {
    let image = "/path/to/image or video";
    let mut asset: Asset = std::path::PathBuf::from(image).try_into().unwrap();

    let client = Client::with_email(
        "https://immich-web-url/api",
        "email@somewhere",
        "s3cr3tpassword"
    ).unwrap();

    let upload_status = client.upload(&mut asset).unwrap();

    println!(
        "{}: {} [Remote ID: {}]",
        upload_status.device_asset_id(),
        upload_status.status(),
        upload_status.id()
    );
}
```

### Upload a whole folder, partily in parallel with live progress update

```rust
use crossbeam_channel::unbounded;
use immich::{Asset, Client};

fn example() {
    let client = Client::with_email(
        "https://immich-web-url/api",
        "email@somewhere",
        "s3cr3tpassword"
    ).unwrap();

    let path = "/path/to/folder/with/images or videos";

    let asset_iterator = std::fs::read_dir(path).unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
                if path.is_dir() {
                    None
                } else {
                    Asset::try_from(path).ok()
                }
        });

    let (sender, receiver) = unbounded::<immich::upload::Uploaded>();

    std::thread::spawn(move || {
        while let Ok(result) = receiver.recv() {
            println!("File uploaded: {}: {}", result.status(), result.device_asset_id())
        }
    });

    client.parallel_upload_with_progress(5, asset_iterator, sender)
        .expect("Parallel upload works");
}
```


## Disclaimer
This is not an official `immich` client and is not supported or endorsed by any `Immich` developer. 

I chose the name for the crate, because it was free. I am happy to hand over the name to an official Rust Immich client, if needed. I could also transfer the library ownership to the immich project, if they find it interesting enough.