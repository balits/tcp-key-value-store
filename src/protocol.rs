use thiserror::Error;

pub const MAX_ARGS: usize = 32 << 20;

#[derive(Error, Debug)]
pub enum ParseError {
    /// Should try to read more after this
    #[error("not enough bytes (want: {want}, got: {got}")]
    NotEnoughBytes { want: usize, got: usize },

    /// Should close connection after this
    #[error("protocol error (likely exceeding MAX_ARGS)")]
    ProtocolError,
}

pub mod request {
    pub const RES_NX: i32 = 1;
    pub const RES_OK: i32 = 0;
    pub const RES_ERR: i32 = -1;

    pub fn serialize(status_code: i32, data: &[u8], buf: &mut Vec<u8>) {
        let len = if data.is_empty() { 8 } else { 8 + data.len() };
        buf.reserve(len);

        buf.extend_from_slice(&(len as u32).to_be_bytes());
        buf.extend_from_slice(&status_code.to_be_bytes());

        if !data.is_empty() {
            buf.extend_from_slice(data);
        }
    }

    pub fn handle_and_encode_request(cmd: Vec<String>, buf: &mut Vec<u8>) {
        let mut map = crate::storage::MAP.lock().unwrap();
        match cmd.len() {
            2 if cmd[0] == "get" => {
                if let Some(v) = map.get(&cmd[1]) {
                    serialize(RES_OK, v.as_bytes(), buf)
                } else {
                    serialize(RES_NX, &[], buf)
                }
            }
            2 if cmd[0] == "del" => {
                if let Some(s) = map.remove(cmd[1].as_str()) {
                    serialize(RES_OK, s.as_bytes(), buf)
                } else {
                    serialize(RES_NX, &[], buf)
                };

            }
            3 if cmd[0] == "set" => {
                map.entry(cmd[1].clone())
                    .and_modify(|v| *v = cmd[2].clone())
                    .or_insert(cmd[2].clone());

                serialize(RES_OK, cmd[2].as_bytes(), buf)
            }
            _ => serialize(RES_OK, &[], buf),
        }
    }
}

pub fn parse_request(src: &[u8]) -> Result<(Vec<String>, usize), ParseError> {
    let mut cursor = 0;
    let num_str = get_u32(src, cursor)? as usize;
    if num_str > MAX_ARGS {
        return Err(ParseError::ProtocolError);
    }
    cursor += 4;

    let mut dst = Vec::with_capacity(num_str);

    for _ in 0..num_str {
        let len = get_u32(&src, cursor)? as usize;

        if len > MAX_ARGS {
            return Err(ParseError::ProtocolError);
        }
        cursor += 4;

        let s = get_str(&src, cursor, cursor + len)?;
        dst.push(s.into());

        cursor += len;
    }

    Ok((dst, cursor))
}

fn get_str<'a>(src: &'a [u8], start: usize, end: usize) -> Result<&'a str, ParseError> {
    if src.len() < start {
        // want read
        Err(ParseError::NotEnoughBytes {
            want: start,
            got: src.len(),
        })
    } else if src.len() < end {
        Err(ParseError::NotEnoughBytes {
            want: end,
            got: src.len(),
        })
    } else {
        std::str::from_utf8(&src[start..end]).map_err(|_| ParseError::ProtocolError)
    }
}

fn get_u32(src: &[u8], start: usize) -> Result<u32, ParseError> {
    if src.len() < start + 4 {
        Err(ParseError::NotEnoughBytes {
            want: 4,
            got: src.len(),
        })
    } else {
        Ok(to_u32(&src[start..start + 4]))
    }
}

fn to_u32(n: &[u8]) -> u32 {
    u32::from_be_bytes([n[0], n[1], n[2], n[3]])
}
