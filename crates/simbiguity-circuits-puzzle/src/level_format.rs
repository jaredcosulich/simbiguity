use alloc::string::String;

pub struct Level {
    pub id: String,
    pub hint: String,
}

pub fn load_level(id: &str) -> Option<Level> {
    Some(Level {
        id: id.into(),
        hint: String::from("Connect the battery to the bulb."),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that load level returns a valid level.
    #[test]
    fn load_level_test() {
        let l = load_level("level_1").unwrap();
        assert_eq!(l.id, "level_1");
        assert_eq!(l.hint, "Connect the battery to the bulb.");
    }
}
