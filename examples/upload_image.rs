use std::path::PathBuf;

use immich::Client;

fn main() {
    let mut args = std::env::args();
    if args.len() < 5 {
        println!("Usage:");
        println!("upload_image <URL> <EMAIL> <PASSWORD> <IMAGE>");
    }
    let _ = args.next();
    let url = args.next().expect("No URL specified");
    println!("{url}");
    let email = args.next().expect("No email specified");
    let password = args.next().expect("No password specified");
    let image = args.next().expect("No image specified");

    let client =
        Client::with_email(&url, &email, &password).expect("Unable to connect to specified host");

    let mut asset = PathBuf::from(image).try_into().expect("Cant read image");

    let res = client.upload(&mut asset);

    println!("{:?}", res);
}
