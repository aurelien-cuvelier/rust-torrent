use std::io::Write;

use sha1::{Digest, Sha1};

use super::ConnectionHandler;
use super::message::{MessageType, REQUEST_PIECE_SIZE};

impl ConnectionHandler<'_> {
    pub(crate) fn handle_new_piece(&mut self, raw_msg: &[u8]) {
        let piece_index = u32::from_be_bytes(raw_msg[0..4].try_into().unwrap());
        let offset_inside_piece = u32::from_be_bytes(raw_msg[4..8].try_into().unwrap());

        let current_piece_index = self.current_piece.as_ref().unwrap().index;

        if current_piece_index != piece_index {
            self.log_err(
                format!(
                    "expected piece index {} but received {piece_index}",
                    current_piece_index
                )
                .as_str(),
            );
            return;
        }

        let block_data = &raw_msg[8..];

        let current_piece = self.current_piece.as_mut().unwrap();

        let min_required_length = (offset_inside_piece as usize) + block_data.len();
        if current_piece.data.len() < min_required_length {
            current_piece.data.resize(min_required_length, 0);
        }

        current_piece.data[(offset_inside_piece as usize)..min_required_length]
            .copy_from_slice(&block_data);

        current_piece.received_offsets.insert(offset_inside_piece);
        current_piece.missing_data -= block_data.len();

        /*
         * We shadow current_piece with a non-mut ref because otherwise we cannot
         * borrow self as non-mutable below. Removing the shadowing does not allow us to compile
         */

        let current_piece = self.current_piece.as_ref().unwrap();

        self.log_info(
                format!(
                    "received {} bytes for piece index {piece_index} & offset {offset_inside_piece} missing data: {}",
                    block_data.len(),
                    self.current_piece.as_ref().unwrap().missing_data
                )
                .as_str(),
        );

        if current_piece.missing_data > 0 {
            let max_offset_index_in_piece =
                (self.torrent_file.info.piece_length as u32).div_ceil(REQUEST_PIECE_SIZE);

            let next_offset = (0..max_offset_index_in_piece).find_map(|offset_index| {
                let offset = offset_index * REQUEST_PIECE_SIZE;
                if current_piece.received_offsets.contains(&offset) {
                    None
                } else {
                    Some(offset)
                }
            });

            if next_offset.is_none() {
                panic!(
                    "searched next offset but found none for piece index: {piece_index} received offsets: {:?}",
                    current_piece.received_offsets
                );
            }

            self.request_piece(piece_index, next_offset.unwrap());
            return;
        }

        let hash_data = Sha1::new()
            .chain_update(self.current_piece.as_ref().unwrap().data.as_slice())
            .finalize();
        let calculated_hash = hash_data.as_slice();

        let expected_hash = &self.torrent_file.info.pieces
            [(piece_index as usize * 20)..(piece_index as usize * 20) + 20];

        let hashes_match = calculated_hash == expected_hash;

        self.log_info(
            format!(
                "Piece index {} is done.\nDownloaded: {}\nExpected:   {}\nmatch: {hashes_match}",
                piece_index,
                hex::encode(calculated_hash),
                hex::encode(expected_hash)
            )
            .as_str(),
        );

        if hashes_match {
            self.file_handler.write_piece_to_file(
                piece_index as usize * self.torrent_file.info.piece_length,
                &current_piece.data,
            );

            self.send_have(piece_index);
        }

        self.current_piece = None;
    }

    pub(crate) fn handle_request_piece(&mut self, raw_msg: &[u8]) {
        let piece_index = u32::from_be_bytes(raw_msg[0..4].try_into().unwrap());
        let offset_inside_piece = u32::from_be_bytes(raw_msg[4..8].try_into().unwrap());
        let length = u32::from_be_bytes(raw_msg[8..].try_into().unwrap());

        let start_index =
            self.torrent_file.info.piece_length as u32 * piece_index + offset_inside_piece;

        let requested_data = self
            .file_handler
            .get_data_from_file(start_index as u64, length as usize);

        let body_len = 1 + 4 + 4 + requested_data.len(); // id + index + begin + block
        let mut piece_msg = vec![0u8; 4 + body_len];
        piece_msg[0..4].copy_from_slice(&(body_len as u32).to_be_bytes());
        piece_msg[4] = MessageType::Piece.to_byte();
        piece_msg[5..9].copy_from_slice(&piece_index.to_be_bytes());
        piece_msg[9..13].copy_from_slice(&offset_inside_piece.to_be_bytes());
        piece_msg[13..].copy_from_slice(&requested_data);

        self.log_info(
            format!(
                "sending piece {piece_index} offset {offset_inside_piece} length {}",
                requested_data.len()
            )
            .as_str(),
        );

        self.stream_mut().write_all(&piece_msg).unwrap();
        self.stream_mut().flush().unwrap();
    }
}
