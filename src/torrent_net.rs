use reqwest;
use urlencoding::encode_binary;

use crate::client::PEER_ID;
use crate::torrent_file::TorrentFile;
use crate::tracker_data::TrackerData;

pub fn get_announce(torrent_file: TorrentFile) {
    println!("Announcing to tracker...");
    let req = format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact=1",
        torrent_file.announce,
        encode_binary(torrent_file.info_hash.as_slice()),
        PEER_ID,
        6881,
        0,
        0,
        torrent_file.info.length,
        "started"
    );

    let res = reqwest::blocking::get(req);

    if res.is_err() {
        println!("server answered with error: {:#?}", res.unwrap_err());
        return;
    }

    let res_data = res.unwrap();

    println!("{:#?}", res_data);

    let body = res_data.bytes().unwrap();

    println!("{:?}", body.clone());

    let tracker_data = TrackerData::from(Vec::from(body));

    println!("{:?}", tracker_data);
}
