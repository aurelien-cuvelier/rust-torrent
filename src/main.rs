use std::{env::args, fs::File, process};

use dotenvy;
use log::{error, info};
use rust_torrent::{file_handler, torrent_file::TorrentFile, tracker};

fn main() {
    dotenvy::dotenv().unwrap();
    env_logger::init();

    let args: Vec<String> = args().collect();

    if args.len() != 2 {
        panic!("No file path given in args")
    }

    let file_name = &args[1];

    let file_res = File::open(file_name);

    if file_res.is_err() {
        let err = file_res.unwrap_err();
        error!("Error opening file {}: {}", file_name, err);
        process::exit(1);
    }

    let file = file_res.unwrap();
    let torrent = TorrentFile::from(file);

    let res_tracker_data = tracker::get_tracker_data(&torrent);

    if res_tracker_data.is_err() {
        error!(
            "error getting tracker data: {}",
            res_tracker_data.unwrap_err()
        );
        process::exit(1)
    }

    let tracker_data = res_tracker_data.unwrap();

    info!("{:?}", tracker_data);

    let mut file_handler = file_handler::get_file_handler(&torrent, &tracker_data);

    tracker::get_connections_handler(&torrent, &tracker_data, &mut file_handler, Some(1));

    println!("END");
}
