pub fn encode_solution(sol: &str) -> String {
    sol.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that a solution string can be encoded.
    #[test]
    fn test_encode() {
        assert_eq!(encode_solution("A"), "A");
    }
}
