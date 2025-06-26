#![feature(once_cell_get_mut)]

pub mod connection;
pub mod util;
pub mod protocol;
pub mod storage;

use core::panic;
use std::{u32, usize};

use mio::Token;
pub const SERVER: Token = Token(0);

pub trait Protocol {
    type Frame;

    fn parse(buf: &[u8]) -> Option<Self::Frame>;
    fn encode(frame: &Self::Frame) -> Vec<u8>;

    fn encode_many(frames: &[Self::Frame]) -> Vec<u8> {
        let mut v = Vec::new();
        for frame in frames {
            let enc = Self::encode(frame); 
            v.extend_from_slice(&enc);
        }
        v
    }
}

pub struct LengthPrefixed;

impl LengthPrefixed {
    pub const PREFIX_SIZE: usize = 4;
}

#[derive(Debug)]
pub struct LPFrame(pub String);

impl Protocol for LengthPrefixed {
    type Frame = LPFrame;

    fn parse(buf: &[u8]) -> Option<Self::Frame> {
        use std::str::from_utf8;

        fn get_u32(n: &[u8]) -> u32 {
            u32::from_be_bytes([n[0], n[1], n[2], n[3]])
        }

        if buf.len() < Self::PREFIX_SIZE {
            return None;
        }

        let len32 = get_u32(&buf[..Self::PREFIX_SIZE]);

        if buf.len() < Self::PREFIX_SIZE + len32 as usize {
            return None;
        }

        let strbuf = &buf[Self::PREFIX_SIZE..(Self::PREFIX_SIZE + len32 as usize)];
        let str = from_utf8(strbuf).expect("invalid utf8 while parsing");

        Some(LPFrame(str.into()))
    }

    fn encode(frame: &Self::Frame) -> Vec<u8> {
        let len = frame.0.len();
        if len > u32::MAX as usize {
            panic!("Frame string longer than u32::MAX");
        }

        let prefix = (len as u32).to_be_bytes();
        let mut v = Vec::with_capacity(size_of::<u32>() + len);
        v.extend_from_slice(&prefix);
        v.extend_from_slice(frame.0.as_bytes());
        v
    }
}
