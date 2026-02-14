use core::panic;
use std::{
    fmt::Debug,
    io::{BufRead, BufReader, Read, Seek},
};

#[derive(Debug, PartialEq)]
pub enum BencodeType {
    Dictionary,
    Integer,
    List,
    String,
    Terminator,
}

pub trait BencodeKey {
    fn is_binary_field(&self) -> bool {
        false
    }
    fn is_string_field(&self) -> bool {
        false
    }
    fn is_integer_field(&self) -> bool {
        false
    }
    fn is_dictionary_field(&self) -> bool {
        false
    }

    fn is_list_field(&self) -> bool {
        false
    }

    fn is_list_of_strings(&self) -> bool {
        false
    }

    fn is_nested_list_string(&self) -> bool {
        false
    }

    fn is_list_of_bytes(&self) -> bool {
        false
    }

    fn from_str(s: &str) -> Self;
    fn as_str(&self) -> &str;
    fn is_unsupported_key(&self) -> bool;
}

pub trait BencodeParsable: Sized + Debug {
    type Key: BencodeKey + Debug + Clone;
    type R: Read + Seek;

    fn key_from_str(s: &str) -> Self::Key;

    fn on_string_or_bytes(&mut self, key: Self::Key, _value: Vec<u8>) {
        println!("on_string_or_bytes throwing away data for {}", key.as_str());
    }
    fn on_integer(&mut self, key: Self::Key, _value: usize) {
        println!("on_integer throwing away data for {}", key.as_str());
    }

    fn on_list_string(&mut self, key: Self::Key, _value: Vec<String>) {
        println!("on_list_string throwing away data for {}", key.as_str());
    }

    fn on_list_bytes(&mut self, key: Self::Key, _value: Vec<u8>) {
        println!("on_list_bytes throwing away data for {}", key.as_str());
    }

    fn on_nested_list_string(&mut self, key: Self::Key, _value: Vec<Vec<String>>) {
        println!(
            "on_nested_list_string throwing away data for {}",
            key.as_str()
        );
    }

    fn on_dictionary(&mut self, key: Self::Key, buf_reader: &mut BufReader<Self::R>) {
        println!("on_dictionary throwing away data for {}", key.as_str());
        decode_dictionary(self, buf_reader);
    }
}

fn consume_next_byte<R: Read + Seek>(buf_reader: &mut BufReader<R>) {
    buf_reader.read_exact(&mut [0u8]).unwrap();
}

/**
 * @TODO make sure that we have the minimum mandatory data according to torrent files BEP
 */

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

fn get_incoming_string_length<R: Read + Seek>(
    buf_reader: &mut BufReader<R>,
) -> Result<usize, String> {
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

fn decode_bytes<R: Read + Seek>(buf_reader: &mut BufReader<R>) -> Vec<u8> {
    let data_length = get_incoming_string_length(buf_reader).unwrap();

    let mut data_value = vec![0u8; data_length];

    let read_res = buf_reader.read(&mut data_value);

    if read_res.is_err() {
        panic!("error reading in string {}", read_res.unwrap_err())
    }

    return data_value;
}

/**
 * Extract the next type without consuming the 1st byte representing the type or the first length digit for string/bytes.
 */
pub fn extract_next_type<R: Read + Seek>(buf_reader: &mut BufReader<R>) -> Option<BencodeType> {
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

fn decode_integer<R: Read + Seek>(buf_reader: &mut BufReader<R>) -> usize {
    consume_next_byte(buf_reader); //consuming the i;

    let mut int_buf = Vec::<u8>::with_capacity(10);

    let _read_bytes = buf_reader.read_until(b'e', &mut int_buf);

    int_buf.pop(); //removing the read 'e'

    parse_string_to_usize(String::from_utf8(int_buf).unwrap())
}

/**
 * Will panic if the next type is None (buffer EOF) or if the type is not the expected one
 */
pub fn assert_next_type<R: Read + Seek>(buf_reader: &mut BufReader<R>, expected_type: BencodeType) {
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

fn decode_list_string<R: Read + Seek>(buf_reader: &mut BufReader<R>) -> Vec<String> {
    let mut strings = Vec::<String>::new();

    loop {
        let next_type = extract_next_type(buf_reader);

        if next_type.is_none() {
            panic!("reached EOF");
        }

        match next_type.unwrap() {
            BencodeType::Terminator => {
                consume_next_byte(buf_reader);
                break;
            }
            _ => {}
        }

        let data_size = get_incoming_string_length(buf_reader).unwrap();

        let mut data = vec![0u8; data_size];

        buf_reader.read_exact(&mut data).unwrap();

        strings.push(String::from_utf8(data).unwrap());
    }

    return strings;
}

fn decode_nested_list_string<R: Read + Seek>(buf_reader: &mut BufReader<R>) -> Vec<Vec<String>> {
    let mut nested = Vec::new();
    loop {
        let next_type = extract_next_type(buf_reader).unwrap();
        match next_type {
            BencodeType::List => {
                consume_next_byte(buf_reader);
                nested.push(decode_list_string(buf_reader));
            }
            BencodeType::Terminator => {
                consume_next_byte(buf_reader);
                break;
            }
            _ => panic!("received type {:?} in nested list string", next_type),
        }
    }
    nested
}

pub fn decode_dictionary<P: BencodeParsable + Debug>(
    target: &mut P,
    buf_reader: &mut BufReader<P::R>,
) where
    P::Key: Debug + Clone,
{
    //consuming the first byte (d)
    consume_next_byte(buf_reader);

    let mut current_key: Option<P::Key> = None;

    loop {
        let next_type = extract_next_type(buf_reader);

        if next_type.is_none() {
            //EOF
            break;
        }

        let next_type = next_type.unwrap();

        if next_type == BencodeType::Terminator {
            consume_next_byte(buf_reader);
            break;
        }

        if current_key.is_none() && next_type != BencodeType::String {
            panic!(
                "decode_dictionary: current key is empty but next type is {:?}",
                next_type
            );
        }

        if current_key.is_none() {
            current_key = Some(P::key_from_str(
                str::from_utf8(decode_bytes(buf_reader).as_slice()).unwrap(),
            ));
            println!("New key: {:?}", current_key);
            continue;
        }

        let unwrapped_current_key = current_key.unwrap();

        match next_type {
            BencodeType::String => {
                //in reality BencodeType::String work for both string & raw bytes

                let decoded_bytes = decode_bytes(buf_reader);
                target.on_string_or_bytes(unwrapped_current_key.clone(), decoded_bytes);
            }
            BencodeType::Integer => {
                let decoded_integer = decode_integer(buf_reader);
                target.on_integer(unwrapped_current_key.clone(), decoded_integer);
            }
            BencodeType::Dictionary => {
                target.on_dictionary(unwrapped_current_key.clone(), buf_reader);
            }
            BencodeType::List => {
                consume_next_byte(buf_reader); // consume 'l'
                match () {
                    _ if unwrapped_current_key.is_list_of_strings() => {
                        let strings = decode_list_string(buf_reader);
                        target.on_list_string(unwrapped_current_key.clone(), strings);
                    }
                    _ if unwrapped_current_key.is_nested_list_string() => {
                        let nested_strings = decode_nested_list_string(buf_reader);
                        target.on_nested_list_string(unwrapped_current_key.clone(), nested_strings);
                    }
                    _ => {}
                }
            }
            BencodeType::Terminator => {
                consume_next_byte(buf_reader);
                return;
            }
        };

        println!("{:?}\n\n", target);
        println!("Deleting current key: {:?}\n", unwrapped_current_key);
        current_key = None;
    }

    println!("{:?}\n\n", target);

    return;
}
