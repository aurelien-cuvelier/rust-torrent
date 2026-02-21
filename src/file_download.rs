use std::{
    fs,
    io::{Read, Write},
};

use log::{debug, info};
use sha1::{Digest, Sha1};

use crate::{torrent_file::TorrentFile, tracker_data::TrackerData};
use hex;

pub struct FileHandler {
    pub file: fs::File,
    pub bitfield: Vec<u8>,
    pub has_missing_parts: bool,
}

fn get_local_file_bitfield(file: &mut fs::File, torrent_file: &TorrentFile) -> Vec<u8> {
    let total_pieces: usize = torrent_file
        .info
        .length
        .div_ceil(torrent_file.info.piece_length);

    info!("File has {} pieces", total_pieces);

    let mut bitfield = vec![0u8; total_pieces.div_ceil(8)];
    let mut buffer = vec![0u8; torrent_file.info.piece_length];
    let mut hasher = Sha1::new();
    let mut total_read: usize = 0;

    for i in 0..total_pieces {
        let read = file.read(&mut buffer).unwrap();
        total_read += read;

        hasher.write(&buffer[0..read]).unwrap();
        let hashed = hasher.finalize_reset();

        let matching_part = torrent_file.info.pieces[(i * 20)..(i * 20 + 20)].as_ref();

        let hashes_match = hashed.as_slice() == matching_part;
        let piece_index = i.div_euclid(8);

        //bits are stored in MSB order for a single piece
        let bit_index = 7 - i % 8;

        debug!(
            "piece #{i} ({total_read}/{}) bitfield index:{piece_index} bit index:{bit_index}\n\ncomputed: {}\nexpected: {}\nmatching: {hashes_match}",
            torrent_file.info.length,
            hex::encode(hashed),
            hex::encode(matching_part)
        );

        if hashes_match {
            //same as bitfield[piece_index] = bitfield[piece_index] | (1 << bit_index);
            bitfield[piece_index] |= 1 << bit_index;
        }

        if read != torrent_file.info.piece_length && i != total_pieces - 1 {
            panic!(
                "filled {read} bytes in buffer but iteration is not the last ({i}/{})",
                total_pieces - 1
            )
        }
    }

    debug!(
        "read {total_read} out of {}\n\nbitfield:\n{:?}",
        torrent_file.info.length, bitfield
    );

    return bitfield;
}

pub fn get_file_handlder(torrent_file: &TorrentFile, _tracker_data: &TrackerData) -> FileHandler {
    let path = format!("./downloads/{}", torrent_file.info.name);

    let exists_before_open = fs::exists(&path).unwrap();

    let mut handler = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();

    if !exists_before_open {
        debug!("created file {}", torrent_file.info.name);
        handler.set_len(torrent_file.info.length as u64).unwrap();
    } else {
        debug!("opened file {}", torrent_file.info.name);
    }

    let bitfield = get_local_file_bitfield(&mut handler, torrent_file);

    FileHandler {
        file: handler,
        bitfield,
        has_missing_parts: true,
    }
}
