pub struct Level {
    pub id: String,
    pub hint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that a level contains the correct ID.
    #[test]
    fn test_level() {
        let level = Level {
            id: "1".into(),
            hint: "test".into(),
        };
        assert_eq!(level.id, "1");
    }
}
