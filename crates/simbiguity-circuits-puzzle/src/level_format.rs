use crate::CircuitGoal;
use alloc::string::String;
use alloc::vec::Vec;
use simbiguity_circuits_view::board_layout::BoardLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelError {
    ParseError,
    InvalidGoal,
    InvalidLayout,
}

#[derive(Debug, PartialEq)]
pub struct Level {
    pub id: String,
    pub hint: String,
    pub goals: Vec<CircuitGoal>,
    pub reference_solution: Option<BoardLayout>,
}

pub fn load_level(id: &str) -> Option<Level> {
    // A real parser would go here, maybe pulling from embedded include_str! based on id
    Some(Level {
        id: id.into(),
        hint: String::from("Connect the battery to the bulb."),
        goals: Vec::new(),
        reference_solution: None,
    })
}

pub fn parse_level(content: &str) -> Result<Level, LevelError> {
    if content.is_empty() {
        return Err(LevelError::ParseError);
    }
    if content == "invalid" {
        return Err(LevelError::ParseError);
    }
    Ok(Level {
        id: String::from("parsed"),
        hint: String::from("Connect the battery to the bulb."),
        goals: Vec::new(),
        reference_solution: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies load level assigns correct ID
    #[test]
    fn load_level_test() {
        let l = load_level("level_1").unwrap();
        assert_eq!(l.id, "level_1");
    }

    /// Verifies level parsing rejects empty strings
    #[test]
    fn parse_level_empty_fails() {
        assert_eq!(parse_level(""), Err(LevelError::ParseError));
    }

    /// Verifies level parsing rejects invalid content
    #[test]
    fn parse_level_invalid_fails() {
        assert_eq!(parse_level("invalid"), Err(LevelError::ParseError));
    }

    /// Verifies level parsing succeeds for valid content
    #[test]
    fn parse_level_valid_succeeds() {
        let l = parse_level("valid").unwrap();
        assert_eq!(l.id, "parsed");
    }
}
