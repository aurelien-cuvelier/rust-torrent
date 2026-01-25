use core::panic;
use std::{
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
};

use crate::torrent_file::{TorrentFile, TorrentKeys};

#[derive(Debug, PartialEq)]
enum BencodeType {
    Dictionary,
    Integer,
    List,
    String,
    Terminator,
}

static SEMI_COLON: u8 = b':';

fn parse_string_to_usize(str: String) -> usize {
    let parse_res = str.parse::<usize>();

    if parse_res.is_err() {
        let err_msg = format!("Cannot parse {} to usize", str);
        panic!("{}", err_msg);
    }

    return parse_res.unwrap();
}

fn is_parseable_stringified_number(str_bytes: Vec<u8>) -> bool {
    let string_res = String::from_utf8(str_bytes);

    if string_res.is_err() {
        return false;
    }

    let parse_res = string_res.unwrap().parse::<usize>();

    return parse_res.is_ok();
}

fn get_incoming_string_length(buf_reader: &mut BufReader<File>) -> Result<usize, String> {
    let mut string_length_buf = Vec::new();

    buf_reader
        .read_until(SEMI_COLON, &mut string_length_buf)
        .unwrap();

    string_length_buf.pop(); //Removing the semicolon

    if !is_parseable_stringified_number(string_length_buf.clone()) {
        return Err(format!(
            "cannot parse {} to usize",
            String::from_utf8_lossy(&string_length_buf)
        ));
    }

    Ok(parse_string_to_usize(
        String::from_utf8(string_length_buf).unwrap(),
    ))
}

fn decode_string(buf_reader: &mut BufReader<File>) -> String {
    let string_length = get_incoming_string_length(buf_reader).unwrap();

    let mut string_value = vec![0u8; string_length];

    let read_res = buf_reader.read(&mut string_value);

    if read_res.is_err() {
        panic!("error reading in string {}", read_res.unwrap_err())
    }

    return String::from_utf8(string_value).unwrap();
}

fn decode_next_torrent_key(buf_reader: &mut BufReader<File>) -> TorrentKeys {
    let potential_torrent_key = decode_string(buf_reader);

    return TorrentKeys::from_str(potential_torrent_key.as_str()).unwrap();
}

fn handle_binary_field(
    torrent_file: &mut TorrentFile,
    buf_reader: &mut BufReader<File>,
    key: TorrentKeys,
) {
    let data_length = get_incoming_string_length(buf_reader).unwrap();

    match key {
        TorrentKeys::Pieces => {
            let mut data = vec![0u8; data_length];
            buf_reader.read(&mut data).unwrap();
            torrent_file.info.pieces = data
        }
        TorrentKeys::Sha1 => {
            let mut data = [0u8; 20];
            buf_reader.read_exact(&mut data).unwrap();
            torrent_file.info.sha1 = Some(data);
        }
        TorrentKeys::Sha256 => {
            let mut data = [0u8; 32];
            buf_reader.read_exact(&mut data).unwrap();
            torrent_file.info.sha256 = Some(data);
        }
        _ => panic!("unsupported key in hande_binary: {}", key.as_str()),
    }
}

/**
 * Extract the next type without consuming the 1st byte representing the type or the first length digit for string/bytes.
 */
fn extract_next_type(buf_reader: &mut BufReader<File>) -> Option<BencodeType> {
    let internal_buf = buf_reader.fill_buf().unwrap();

    if internal_buf.len() == 0 {
        return None;
    }

    if is_parseable_stringified_number(vec![internal_buf[0]]) {
        return Some(BencodeType::String);
    }

    return Some(match char::from(internal_buf[0]) {
        'd' => BencodeType::Dictionary,
        'l' => BencodeType::List,
        'i' => BencodeType::Integer,
        'e' => BencodeType::Terminator,
        _ => panic!("cannot match bencode type 0X{:2X}", internal_buf[0]),
    });
}

fn handle_string_field(
    torrent_file: &mut TorrentFile,
    buf_reader: &mut BufReader<File>,
    key: TorrentKeys,
) {
    assert_next_type(buf_reader, BencodeType::String);

    let value = decode_string(buf_reader);

    match key {
        TorrentKeys::Announce => torrent_file.announce = value,
        TorrentKeys::Comment => torrent_file.comment = value,
        TorrentKeys::CreatedBy => torrent_file.created_by = value,
        TorrentKeys::Md5Sum => torrent_file.info.md5sum = Some(value),
        TorrentKeys::Name => torrent_file.info.name = value,
        _ => panic!("handle_string_field unsupported key: {}", key.as_str()),
    }
}

fn handle_integer_field(
    torrent_file: &mut TorrentFile,
    buf_reader: &mut BufReader<File>,
    key: TorrentKeys,
) {
    buf_reader.consume(1); //consuming the i;

    let mut int_buf = Vec::<u8>::with_capacity(10);

    let read_bytes = buf_reader.read_until(b'e', &mut int_buf);
    println!("read bytes: {:?}", read_bytes);

    int_buf.pop(); //removing the read 'e'

    let uint_value = parse_string_to_usize(String::from_utf8(int_buf).unwrap());

    match key {
        TorrentKeys::CreationDate => torrent_file.creation_date = uint_value,
        TorrentKeys::Length => torrent_file.info.length = uint_value,
        TorrentKeys::PieceLength => torrent_file.info.piece_length = uint_value,

        _ => {
            println!("handle_integer_field unsupported key: {}", key.as_str());
        }
    }
}

/**
 * Will panic if the next type is None (buffer EOF) or if the type is not the expected one
 */
fn assert_next_type(buf_reader: &mut BufReader<File>, expected_type: BencodeType) {
    let next_type_result = extract_next_type(buf_reader);

    if next_type_result.is_none() {
        panic!("next type is None while expected {:?}", expected_type);
    }

    let next_type = next_type_result.unwrap();

    if next_type != expected_type {
        panic!(
            "received type {:?} while expected {:?}",
            next_type, expected_type
        );
    }
}

fn extract_string_list(buf_reader: &mut BufReader<File>) -> Vec<String> {
    buf_reader.consume(1);
    assert_next_type(buf_reader, BencodeType::String);

    let mut strings = Vec::<String>::new();

    loop {
        let string_element = decode_string(buf_reader);

        strings.push(string_element);

        let next_type = extract_next_type(buf_reader).unwrap();

        if next_type == BencodeType::Terminator {
            buf_reader.consume(1);
            break;
        }
    }

    return strings;
}

fn handle_string_list_field(
    torrent_file: &mut TorrentFile,
    buf_reader: &mut BufReader<File>,
    key: TorrentKeys,
) {
    let extracted_list = extract_string_list(buf_reader);

    match key {
        TorrentKeys::Sources => torrent_file.sources = Some(extracted_list),
        TorrentKeys::UrlList => torrent_file.url_list = Some(extracted_list),
        _ => println!("handle_string_list_field unsupported key: {}", key.as_str()),
    }
}

fn handle_announce_list(torrent_file: &mut TorrentFile, buf_reader: &mut BufReader<File>) {
    assert_next_type(buf_reader, BencodeType::List);

    let mut announce_list = Vec::<Vec<String>>::new();

    buf_reader.consume(1); //consuming the main list delimiter;

    loop {
        let next_type = extract_next_type(buf_reader).unwrap();

        if next_type == BencodeType::Terminator {
            buf_reader.consume(1);
            break;
        }

        if next_type == BencodeType::List {
            // buf_reader.consume(1);
            announce_list.push(extract_string_list(buf_reader));
            continue;
        }
    }

    torrent_file.announce_list = Some(announce_list);
}

fn decode_dictionary_field(torrent_file: &mut TorrentFile, buf_reader: &mut BufReader<File>) {
    //consuming the first byte (d)
    buf_reader.consume(1);

    let mut current_key: Option<TorrentKeys> = None;
    loop {
        println!("Torrent file: {:?}\n", torrent_file);
        let next_type = extract_next_type(buf_reader);

        if next_type.is_none() {
            break;
        }

        let next_type = next_type.unwrap();
        println!(
            "current key: {:?}  |  next type: {:?}",
            current_key, next_type
        );

        if next_type == BencodeType::Terminator {
            buf_reader.consume(1);
            return;
        }

        if current_key.is_none() && next_type != BencodeType::String {
            panic!(
                "decode_dictionary: current key is empty but next type is {:?}",
                next_type
            );
        }

        if current_key.is_none() {
            current_key = Some(decode_next_torrent_key(buf_reader));
            continue;
        }

        let current_torrent_key = current_key.unwrap();

        println!(
            "Handling {:?} with key {} | is_binary: {}",
            next_type,
            current_torrent_key.as_str(),
            current_torrent_key.is_binary_field()
        );
        match next_type {
            BencodeType::String => {
                if current_torrent_key.is_binary_field() {
                    handle_binary_field(torrent_file, buf_reader, current_torrent_key);
                } else {
                    handle_string_field(torrent_file, buf_reader, current_torrent_key);
                }
            }
            BencodeType::Integer => {
                handle_integer_field(torrent_file, buf_reader, current_torrent_key);
            }
            BencodeType::Dictionary => {
                decode_dictionary_field(torrent_file, buf_reader);
            }
            BencodeType::List => {
                if current_torrent_key == TorrentKeys::AnnounceList {
                    handle_announce_list(torrent_file, buf_reader);
                } else {
                    handle_string_list_field(torrent_file, buf_reader, current_torrent_key);
                }
            }
            BencodeType::Terminator => {}
        };

        current_key = None;
    }
}

pub fn parse_torrent_file(file: File) -> Result<TorrentFile, String> {
    let mut torrent_file = TorrentFile::default();
    let mut buf_reader = BufReader::new(file);
    let next_type_res = extract_next_type(&mut buf_reader);

    if next_type_res.is_none() {
        return Err(format!("file is empty"));
    }

    let next_type = next_type_res.unwrap();

    if next_type != BencodeType::Dictionary {
        return Err(format!("file must start with a bencode dictionary"));
    }

    decode_dictionary_field(&mut torrent_file, &mut buf_reader);

    let current_pos = buf_reader.seek(SeekFrom::Current(0)).unwrap();
    let end = buf_reader.seek(SeekFrom::End(0)).unwrap();
    let remaining_bytes = end - current_pos;

    if remaining_bytes != 0 {
        panic!(
            "done parsing but some bytes havent been read: {}",
            remaining_bytes
        );
    }

    return Ok(torrent_file);
}
