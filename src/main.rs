use std::{fs::File, process};
mod bencode;
mod torrent_file;

fn main() {
    let file_name = "./libreoffice-help-25.8.4.2.tar.xz.torrent";

    let file_res = File::open(file_name);

    if file_res.is_err() {
        let err = file_res.unwrap_err();
        println!("Error opening file {}: {}", file_name, err);
        process::exit(1);
    }

    let file = file_res.unwrap();

    _ = bencode::parse_torrent_file(file)
}
