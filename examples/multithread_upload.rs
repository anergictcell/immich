use std::thread;

use crossbeam_channel::unbounded;
use immich::{upload::Uploaded, Album, Asset, Client};

fn main() {
    let mut args = std::env::args();
    if args.len() < 5 {
        println!("Usage:");
        println!("multithread_upload <URL> <EMAIL> <PASSWORD> <PATH_TO_FOLDER> [<ALBUM_NAME>]");
    }
    let _ = args.next();
    let url = args.next().expect("No URL specified");
    println!("{url}");
    let email = args.next().expect("No email specified");
    let password = args.next().expect("No password specified");
    let path = args.next().expect("No folder specified");
    let album = args.next();

    let client =
        Client::with_email(&url, &email, &password).expect("Unable to connect to specified host");

    let assets = std::fs::read_dir(path)
        .expect("path is a readable directory")
        .filter_map(|entry| {
            let entry = entry.expect("File must be actual file");
            let path = entry.path();
            if path.is_dir() {
                None
            } else {
                Asset::try_from(path).ok()
            }
        });

    let (result_sender, result_receiver) = unbounded::<Uploaded>();

    thread::spawn(move || {
        while let Ok(result) = result_receiver.recv() {
            println!("{}: {}", result.status(), result.device_asset_id())
        }
    });

    if let Some(album_name) = album {
        let album = Album::get_or_create(&client, album_name).expect("Can't find or crate album");
        let result = client
            .upload_to_album(5, assets, &album, Some(result_sender))
            .expect("Uploading to album works");
        println!("{} assets uploaded and moved", result.len());
    } else {
        let result = client
            .upload(5, assets, Some(result_sender))
            // .parallel_upload(5, assets, None)
            .expect("Parallel upload works");
        println!("{} assets uploaded and moved", result.len());
    }
}
