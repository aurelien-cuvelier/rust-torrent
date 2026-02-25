use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{Read, Seek, Write},
};

use log::{debug, info};
use sha1::{Digest, Sha1};

use crate::{torrent_file::TorrentFile, tracker_data::TrackerData};
use hex;

pub struct FileHandler {
    pub file: fs::File,
    pub bitfield: Vec<u8>,

    /*
    Using VecDeque instead of Vector since we can/remove elements from the top
    with O(1) complexity, while Vector needs to re-organize the whole vector everytime
    */
    pub needed_pieces: VecDeque<usize>,
}

impl FileHandler {
    pub fn write_piece_to_file(&mut self, start_index: usize, piece: &[u8]) {
        self.file
            .seek(std::io::SeekFrom::Start(start_index as u64))
            .unwrap();

        self.file.write_all(piece).unwrap();
    }
}

fn get_local_file_bitfield(file: &mut fs::File, torrent_file: &TorrentFile) -> Vec<u8> {
    let mut bitfield = vec![0u8; torrent_file.pieces_amount.div_ceil(8)];
    let mut buffer = vec![0u8; torrent_file.info.piece_length];
    let mut hasher = Sha1::new();
    let mut total_read: usize = 0;

    for i in 0..torrent_file.pieces_amount {
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

        if read != torrent_file.info.piece_length && i != torrent_file.pieces_amount - 1 {
            panic!(
                "filled {read} bytes in buffer but iteration is not the last ({i}/{})",
                torrent_file.pieces_amount - 1
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

    println!("total pieces: {}", torrent_file.pieces_amount);

    let needed_pieces =
        bitfield
            .iter()
            .enumerate()
            .fold(VecDeque::new(), |mut acc, (index, pieces_bits)| {
                for shift in 0..=7 {
                    let piece_index = index * 8 + shift;
                    if piece_index >= torrent_file.pieces_amount {
                        break;
                    }
                    /*
                     * We shift left all the 8 pieces from the byte and apply
                     * a mask cancelling all bits except the MSB. If the resulting
                     * number is 0 then we need that piece, anything else than 0 is not needed.
                     */
                    let piece_is_needed = (pieces_bits << shift) & 0b10000000 == 0;

                    if piece_is_needed {
                        acc.push_back(piece_index);
                    }
                    println!("piece {} | needed: {piece_is_needed}", index * 8 + shift);
                }

                return acc;
            });

    FileHandler {
        file: handler,
        bitfield,
        needed_pieces,
    }
}
