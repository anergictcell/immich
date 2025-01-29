use std::thread;

use crossbeam_channel::unbounded;
use immich::{upload::Uploaded, Asset, Client};

fn main() {
    let mut args = std::env::args();
    if args.len() < 5 {
        println!("Usage:");
        println!("multithread_upload <URL> <EMAIL> <PASSWORD> <PATH_TO_FOLDER>");
    }
    let _ = args.next();
    let url = args.next().expect("No URL specified");
    println!("{url}");
    let email = args.next().expect("No email specified");
    let password = args.next().expect("No password specified");
    let path = args.next().expect("No folder specified");

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

    client
        .parallel_upload_with_progress(5, assets, result_sender)
        .expect("Parallel upload works");
}
