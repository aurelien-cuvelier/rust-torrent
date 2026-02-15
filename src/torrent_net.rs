use reqwest;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use urlencoding::encode_binary;

use crate::client;
use crate::torrent_file::TorrentFile;
use crate::tracker_data::TrackerData;

#[derive(Debug)]
enum MessageType {
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
    fn from_byte(value: u8) -> Option<Self> {
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

pub fn get_announce(torrent_file: TorrentFile) {
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
        println!("server answered with error: {:#?}", res.unwrap_err());
        return;
    }

    let res_data = res.unwrap();

    println!("{:#?}", res_data);

    let body = res_data.bytes().unwrap();

    println!(
        "{:?}",
        String::from_utf8_lossy(body.clone().to_vec().as_slice())
    );

    let tracker_data = TrackerData::from(Vec::from(body));

    println!("{:?}", tracker_data);

    connect_to_peer(&torrent_file, &tracker_data.peers_str[0]);
}

fn get_handshake_data(info_hash: &[u8; 20]) -> [u8; 68] {
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

pub fn connect_to_peer(torrent_file: &TorrentFile, peer: &str) {
    //handshakes are 68 bytes long
    let mut stream = TcpStream::connect(peer).unwrap();

    let handshake_data = get_handshake_data(&torrent_file.info_hash);
    println!(
        "\n\nsending handshake to {} data {:?}",
        peer,
        String::from_utf8_lossy(&handshake_data)
    );
    stream.write_all(&handshake_data).unwrap();

    let mut handshake_response = [0u8; 68];
    stream.read_exact(&mut handshake_response).unwrap();

    println!(
        "\n\nreceived handshake response: {:?}",
        String::from_utf8_lossy(&handshake_response)
    );

    let peer_id = &handshake_response[48..68];
    let info_hash = &handshake_response[28..48];

    let info_hash_match = torrent_file.info_hash.eq(info_hash);

    println!(
        "\n\n{peer} - {}\ninfo hash match: {}",
        String::from_utf8(peer_id.to_vec()).unwrap(),
        info_hash_match
    );

    loop {
        //4 first bytes is the payload length
        let mut payload_length_raw = [0u8; 4];
        match stream.read_exact(&mut payload_length_raw) {
            Ok(()) => {}
            Err(e) => {
                if e.kind() == ErrorKind::UnexpectedEof {
                    eprintln!("stream read OEF -> peer closed the connection");
                } else {
                    eprintln!("stream read error: {}", e);
                }
                break;
            }
        }

        let payload_length = u32::from_be_bytes(payload_length_raw);

        if payload_length == 0 {
            println!("received 0 length msg (keep-alive)");
            continue;
        }

        let mut full_payload = vec![0u8; payload_length.try_into().unwrap()];
        stream.read_exact(&mut full_payload).unwrap();

        let msg_id = full_payload[0];

        let msg_type = match MessageType::from_byte(msg_id) {
            None => {
                println!("unknown message type: {}", msg_id);
                continue;
            }
            Some(m) => m,
        };

        println!("received message type: {:?}", msg_type);

        match msg_type {
            MessageType::Choke => {}
            MessageType::Unchoke => {}
            MessageType::Interested => {}
            MessageType::NotInterested => {}
            MessageType::Have => {}
            MessageType::Bitfield => {}
            MessageType::Request => {}
            MessageType::Piece => {}
            MessageType::Cancel => {}
            MessageType::Port => {}
        }
    }

    stream.shutdown(Shutdown::Both).unwrap();
}
