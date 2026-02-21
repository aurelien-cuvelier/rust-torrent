use reqwest;
use urlencoding::encode_binary;

use crate::client;
use crate::connection_handler::ConnectionHandler;
use crate::torrent_file::TorrentFile;
use crate::tracker_data::TrackerData;

#[derive(Debug)]
pub enum MessageType {
    //https://wiki.theory.org/BitTorrentSpecification
    Choke = 0,         // (choke): Peer notifies that it will not send data.
    Unchoke = 1,       // (unchoke): Peer notifies that it will send data.
    Interested = 2,    // (interested): Peer expresses interest in obtaining data.
    NotInterested = 3, // (not interested): Peer expresses no interest in data.
    Have = 4,          // (have): Tells peers that a piece has been downloaded.
    Bitfield = 5, // (bitfield): Sent immediately after handshake to show all pieces a peer has.
    Request = 6,  // (request): Requests a block of data.
    Piece = 7,    // (piece): Delivers a block of data.
    Cancel = 8,   // (cancel): Cancels a previously sent request.
    Port = 9,     // (port): Used for DHT tracker connectivity.
}

impl MessageType {
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Choke),
            1 => Some(Self::Unchoke),
            2 => Some(Self::Interested),
            3 => Some(Self::NotInterested),
            4 => Some(Self::Have),
            5 => Some(Self::Bitfield),
            6 => Some(Self::Request),
            7 => Some(Self::Piece),
            8 => Some(Self::Cancel),
            9 => Some(Self::Port),
            _ => None,
        }
    }
}

pub fn get_tracker_data(torrent_file: &TorrentFile) -> Result<TrackerData, String> {
    println!("Announcing to tracker...");
    let req = format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact=1",
        torrent_file.announce,
        encode_binary(torrent_file.info_hash.as_slice()),
        client::PEER_ID,
        6881,
        0,
        0,
        torrent_file.info.length,
        "started"
    );

    let res = reqwest::blocking::get(req);

    if res.is_err() {
        return Err(format!("server answered with: {:?}", res.unwrap_err()));
    }

    let res_data = res.unwrap();

    println!("{:#?}", res_data);

    let body = res_data.bytes().unwrap();

    println!(
        "{:?}",
        String::from_utf8_lossy(body.clone().to_vec().as_slice())
    );

    let tracker_data = TrackerData::from(Vec::from(body));

    return Ok(tracker_data);
}

pub fn get_handshake_data(info_hash: &[u8; 20]) -> [u8; 68] {
    /*
    * Bytes	Content
        1	Protocol string length = 19
        19	BitTorrent protocol (literal ASCII)
        8	Reserved (all zeros: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00)
        20	Info hash (your torrent’s 20-byte info hash)
        20	Peer ID (your 20-byte peer id)
    */
    let mut buf = [0u8; 68];

    buf[0] = 19;
    buf[1..20].copy_from_slice(b"BitTorrent protocol");
    //20 => 28 is alrweady 0;
    buf[28..48].copy_from_slice(info_hash);
    buf[48..68].copy_from_slice(client::PEER_ID.as_ref());
    return buf;
}

pub fn get_connections_handler<'a>(
    tracker_data: &'a TrackerData,
    max_peers: Option<usize>,
) -> Vec<ConnectionHandler<'a>> {
    let mut connections = Vec::<ConnectionHandler<'a>>::new();
    let max_peers = match max_peers {
        Some(max) => max,
        _ => 4,
    };

    for (peer_index, peer) in tracker_data.peers_str.iter().enumerate() {
        if peer_index >= max_peers {
            break;
        }

        connections.push(ConnectionHandler::new(peer));
    }

    return connections;
}
