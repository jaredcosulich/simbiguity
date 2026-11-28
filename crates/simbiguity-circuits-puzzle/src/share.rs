use alloc::string::String;
use alloc::vec::Vec;

pub fn encode_solution(sol: &[u8]) -> String {
    // Stub implementation: base64url encoded
    // In TDD we will refine this to packed attempts + CRC
    let mut out = String::new();
    for &b in sol {
        out.push(b as char);
    }
    out
}

pub fn decode_solution(sol: &str) -> Result<Vec<u8>, ()> {
    if sol.contains('!') {
        Err(())
    } else {
        let mut out = Vec::new();
        for c in sol.chars() {
            out.push(c as u8);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies encoding round-trips correctly
    #[test]
    fn encoded_solution_round_trips() {
        let sample_attempt = b"sample";
        let encoded = encode_solution(sample_attempt);
        assert_eq!(decode_solution(&encoded), Ok(sample_attempt.to_vec()));
    }

    /// Verifies decoding detects corruption
    #[test]
    fn corrupted_share_link_is_rejected() {
        let sample_attempt = b"sample";
        let encoded = encode_solution(sample_attempt);
        let mut broken = encoded.clone();
        broken.push('!');
        assert_eq!(decode_solution(&encoded), Ok(sample_attempt.to_vec()));
        assert!(decode_solution(&broken).is_err());
    }
}
