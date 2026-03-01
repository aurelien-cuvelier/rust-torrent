use core::panic;
use std::{
    fmt::Debug,
    io::{BufRead, BufReader, Read, Seek},
};

mod decode;

pub use decode::decode_dictionary;

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
