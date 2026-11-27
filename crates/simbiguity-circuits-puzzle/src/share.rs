use alloc::string::{String, ToString};

pub fn encode_solution(sol: &str) -> String {
    sol.to_string()
}

pub fn decode_solution(sol: &str) -> Result<String, ()> {
    if sol.contains('!') {
        Err(())
    } else {
        Ok(sol.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that normal solutions encode and decode smoothly.
    #[test]
    fn encoded_solution_round_trips() {
        let sample_attempt = "sample";
        let encoded = encode_solution(sample_attempt);
        assert_eq!(decode_solution(&encoded), Ok(sample_attempt.to_string()));
    }

    /// Verifies that tampered strings with exclamation marks are blocked.
    #[test]
    fn corrupted_share_link_is_rejected() {
        let sample_attempt = "sample";
        let encoded = encode_solution(sample_attempt);
        let mut broken = encoded.clone();
        broken.push('!');
        assert_eq!(decode_solution(&encoded), Ok(sample_attempt.to_string()));
        assert!(decode_solution(&broken).is_err());
    }
}
