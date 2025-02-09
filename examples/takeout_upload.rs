use std::thread;

use crossbeam_channel::unbounded;
use immich::{
    takeout,
    upload::{Status, Uploaded},
    Client,
};

fn main() {
    let mut args = std::env::args();
    if args.len() < 5 {
        println!("Usage:");
        println!("multithread_upload <URL> <EMAIL> <PASSWORD> <PATH_TO_TAKEOUT_FILE>");
    }
    let _ = args.next();
    let url = args.next().expect("No URL specified");
    let email = args.next().expect("No email specified");
    let password = args.next().expect("No password specified");
    let path = args.next().expect("No path to Takeout specified");

    let client =
        Client::with_email(&url, &email, &password).expect("Unable to connect to specified host");

    println!("Scanning Takeout archive. This might take a while");
    let mut takeout = takeout::Uploader::new(path).unwrap();

    let total = takeout.len();

    let (result_sender, result_receiver) = unbounded::<Uploaded>();

    thread::spawn(move || {
        let mut created = 0;
        let mut duplicate = 0;
        let mut failure = 0;
        while let Ok(result) = result_receiver.recv() {
            match result.status() {
                Status::Created => created += 1,
                Status::Duplicate => duplicate += 1,
                Status::Failure => failure += 1,
            };
            print!(
                "\rCreated: {created} | Duplicate: {duplicate} | Failure: {failure} | Total: {}/{total} | [{}: {}]             ",
                created + duplicate + failure,
                result.status(),
                result.device_asset_id()
            )
        }
    });

    let res = takeout
        .upload(&client, 5, result_sender, |_record| true)
        .unwrap();
    println!("Uploaded and moved {} assets", res.len());
}
