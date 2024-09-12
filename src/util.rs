use std::u16;
use std::u64;

pub fn parse_u16(s: &str) -> Result<u16, String> {
    if s.starts_with("0x") {
        match u16::from_str_radix(&s[2..], 16) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.to_string())
        }
    } else if s.starts_with("0b") {
        match u16::from_str_radix(&s[2..], 2) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.to_string())
        }
    } else {
        match u16::from_str_radix(s, 10) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.to_string())
        }
    }
}

pub fn parse_u64(s: &str) -> Result<u64, String> {
    if s.starts_with("0x") {
        match u64::from_str_radix(&s[2..], 16) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.to_string())
        }
    } else if s.starts_with("0b") {
        match u64::from_str_radix(&s[2..], 2) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.to_string())
        }
    } else {
        match u64::from_str_radix(s, 10) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.to_string())
        }
    }
}