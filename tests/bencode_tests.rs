use rust_torrent::torrent_file::TorrentFile;
use std::fs::File;

#[test]
fn test_unsupported_torrent_keys() {
    let file = File::open("./tests/test_minimal.torrent").unwrap();

    let _ = TorrentFile::from(file);
}
