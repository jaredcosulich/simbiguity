use crate::PuzzleSession;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareError {
    BadCharacter,
    TooShort,
    UnknownVersion,
    CrcMismatch,
    LevelMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub level_id: String,
    pub data: Vec<u8>,
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFFFFFFu32
}

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn encode_b64(data: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        if i + 1 < data.len() {
            out.push(B64[(n >> 6) as usize & 63] as char);
        }
        if i + 2 < data.len() {
            out.push(B64[n as usize & 63] as char);
        }
        i += 3;
    }
    out
}

pub fn decode_b64(s: &str) -> Result<Vec<u8>, ShareError> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.chars() {
        if c == '=' {
            continue;
        }
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '-' => 62,
            '_' => 63,
            _ => return Err(ShareError::BadCharacter),
        };
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

pub fn encode_solution(session: &PuzzleSession) -> String {
    let mut data = Vec::new();
    data.push(1);
    data.extend_from_slice(session.level.id.as_bytes());
    data.push(0);

    for w in &session.edits.wires {
        for &(x, y) in &w.points {
            data.push(x as u8);
            data.push(y as u8);
        }
        data.push(255);
    }

    let crc = crc32(&data);
    data.extend_from_slice(&crc.to_be_bytes());
    encode_b64(&data)
}

pub fn decode_solution(sol: &str, expected_level: &str) -> Result<Attempt, ShareError> {
    let data = decode_b64(sol)?;
    if data.len() < 6 {
        return Err(ShareError::TooShort);
    }
    let crc_given = u32::from_be_bytes(data[data.len() - 4..].try_into().unwrap_or([0, 0, 0, 0]));
    let crc_actual = crc32(&data[..data.len() - 4]);
    if crc_given != crc_actual {
        return Err(ShareError::CrcMismatch);
    }
    if data[0] != 1 {
        return Err(ShareError::UnknownVersion);
    }

    let mut level_id = String::new();
    let mut i = 1;
    while i < data.len() - 4 && data[i] != 0 {
        level_id.push(data[i] as char);
        i += 1;
    }
    if i == data.len() - 4 {
        return Err(ShareError::LevelMismatch);
    }
    if level_id != expected_level && !expected_level.is_empty() {
        return Err(ShareError::LevelMismatch);
    }
    i += 1;

    let attempt_data = data[i..data.len() - 4].to_vec();
    Ok(Attempt {
        level_id,
        data: attempt_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_format::Level;

    /// Verifies the CRC32 function against a known good value.
    #[test]
    fn crc32_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    /// Verifies Base64 encoding.
    #[test]
    fn base64_encode_test() {
        assert_eq!(encode_b64(b"hello"), "aGVsbG8");
    }

    /// Verifies Base64 decoding.
    #[test]
    fn base64_decode_test() {
        assert_eq!(decode_b64("aGVsbG8").unwrap(), b"hello");
    }

    /// Verifies invalid Base64 decoding.
    #[test]
    fn base64_decode_invalid() {
        assert_eq!(decode_b64("invalid!"), Err(ShareError::BadCharacter));
    }

    /// Verifies that an encoded solution round-trips correctly.
    #[test]
    fn encoded_solution_round_trips() {
        let mut session = PuzzleSession::new(Level {
            id: "level_1".into(),
            title: "".into(),
            hint: "".into(),
            grid: (10, 10),
            fixed_parts: alloc::vec![],
            palette: alloc::vec![],
            goals: alloc::vec![],
            solution_wires: alloc::vec![],
            solution_parts: alloc::vec![],
        });
        session.place(crate::level_format::LevelSolutionWire {
            points: alloc::vec![(0, 0), (1, 0)],
        });
        let encoded = encode_solution(&session);
        let attempt = decode_solution(&encoded, "level_1").unwrap();
        assert_eq!(attempt.level_id, "level_1");
    }

    /// Verifies various share error conditions.
    #[test]
    fn share_error_tests() {
        let session = PuzzleSession::new(Level {
            id: "level_1".into(),
            title: "".into(),
            hint: "".into(),
            grid: (10, 10),
            fixed_parts: alloc::vec![],
            palette: alloc::vec![],
            goals: alloc::vec![],
            solution_wires: alloc::vec![],
            solution_parts: alloc::vec![],
        });
        let encoded = encode_solution(&session);

        // bad character
        let mut bad_char = encoded.clone();
        bad_char.push('!');
        assert_eq!(
            decode_solution(&bad_char, "level_1"),
            Err(ShareError::BadCharacter)
        );

        // too short
        assert_eq!(decode_solution("AA", "level_1"), Err(ShareError::TooShort));

        // unknown version
        let mut data = decode_b64(&encoded).unwrap();
        data[0] = 99;
        let crc = crc32(&data[..data.len() - 4]);
        let data_len = data.len();
        data[data_len - 4..].copy_from_slice(&crc.to_be_bytes());
        assert_eq!(
            decode_solution(&encode_b64(&data), "level_1"),
            Err(ShareError::UnknownVersion)
        );

        // crc mismatch
        let mut bad_crc = decode_b64(&encoded).unwrap();
        let last = bad_crc.len() - 1;
        bad_crc[last] ^= 1;
        assert_eq!(
            decode_solution(&encode_b64(&bad_crc), "level_1"),
            Err(ShareError::CrcMismatch)
        );

        // level mismatch
        assert_eq!(
            decode_solution(&encoded, "level_5"),
            Err(ShareError::LevelMismatch)
        );
    }
}
