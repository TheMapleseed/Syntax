#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    InvalidLength,
    InvalidByte { index: usize, byte: u8 },
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn decode_value(byte: u8, index: usize) -> Result<u8, DecodeError> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'-' => Ok(62),
        b'_' => Ok(63),
        _ => Err(DecodeError::InvalidByte { index, byte }),
    }
}

pub fn encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    let mut i = 0usize;

    while i + 3 <= input.len() {
        let chunk = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(chunk & 0x3f) as usize] as char);
        i += 3;
    }

    let rem = input.len() - i;
    if rem == 1 {
        let chunk = (input[i] as u32) << 16;
        out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
    } else if rem == 2 {
        let chunk = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
    }

    out
}

pub fn decode(input: &str) -> Result<Vec<u8>, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity((bytes.len() * 3) / 4 + 2);
    let mut i = 0usize;

    while i + 4 <= bytes.len() {
        let a = decode_value(bytes[i], i)? as u32;
        let b = decode_value(bytes[i + 1], i + 1)? as u32;
        let c = decode_value(bytes[i + 2], i + 2)? as u32;
        let d = decode_value(bytes[i + 3], i + 3)? as u32;
        let chunk = (a << 18) | (b << 12) | (c << 6) | d;

        out.push(((chunk >> 16) & 0xff) as u8);
        out.push(((chunk >> 8) & 0xff) as u8);
        out.push((chunk & 0xff) as u8);
        i += 4;
    }

    let rem = bytes.len() - i;
    if rem == 2 {
        let a = decode_value(bytes[i], i)? as u32;
        let b = decode_value(bytes[i + 1], i + 1)? as u32;
        let chunk = (a << 18) | (b << 12);
        out.push(((chunk >> 16) & 0xff) as u8);
    } else if rem == 3 {
        let a = decode_value(bytes[i], i)? as u32;
        let b = decode_value(bytes[i + 1], i + 1)? as u32;
        let c = decode_value(bytes[i + 2], i + 2)? as u32;
        let chunk = (a << 18) | (b << 12) | (c << 6);
        out.push(((chunk >> 16) & 0xff) as u8);
        out.push(((chunk >> 8) & 0xff) as u8);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let input = b"hello_world";
        let encoded = encode(input);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn no_padding_in_output() {
        let encoded = encode(b"f");
        assert_eq!(encoded, "Zg");
    }

    #[test]
    fn invalid_length_rejected() {
        assert_eq!(decode("abcde"), Err(DecodeError::InvalidLength));
    }
}
