use crate::PartKind;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelError {
    ParseError,
    InvalidGoal,
    InvalidLayout,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelFixedPart {
    pub kind: PartKind,
    pub x: i32,
    pub y: i32,
    pub rot: i32,
    pub v1: f64,
    pub v2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelSolutionWire {
    pub points: Vec<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelSolutionPart {
    pub kind: PartKind,
    pub x: i32,
    pub y: i32,
    pub rot: i32,
    pub v1: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub id: String,
    pub title: String,
    pub hint: String,
    pub grid: (i32, i32),
    pub fixed_parts: Vec<LevelFixedPart>,
    pub palette: Vec<(PartKind, usize)>,
    pub goals: Vec<String>,
    pub solution_wires: Vec<LevelSolutionWire>,
    pub solution_parts: Vec<LevelSolutionPart>,
}

pub fn parse_part_kind(s: &str) -> Result<PartKind, LevelError> {
    match s {
        "battery" => Ok(PartKind::Battery),
        "bulb" => Ok(PartKind::Bulb),
        "resistor" => Ok(PartKind::Resistor),
        "switch" => Ok(PartKind::Switch),
        "wire" => Ok(PartKind::Wire),
        _ => Err(LevelError::ParseError),
    }
}

pub fn parse_fixed_part(parts: &[&str]) -> Result<LevelFixedPart, LevelError> {
    if parts.len() < 6 {
        return Err(LevelError::ParseError);
    }
    let kind = parse_part_kind(parts[1])?;
    let x = parts
        .get(2)
        .and_then(|s| s.parse().ok())
        .ok_or(LevelError::ParseError)?;
    let y = parts
        .get(3)
        .and_then(|s| s.parse().ok())
        .ok_or(LevelError::ParseError)?;
    let rot = parts
        .get(5)
        .and_then(|s| s.parse().ok())
        .ok_or(LevelError::ParseError)?;
    let v1 = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let v2 = parts.get(9).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Ok(LevelFixedPart {
        kind,
        x,
        y,
        rot,
        v1,
        v2,
    })
}

pub fn parse_palette_entry(parts: &[&str]) -> Result<(PartKind, usize), LevelError> {
    if parts.len() < 3 {
        return Err(LevelError::ParseError);
    }
    let kind = parse_part_kind(parts[1])?;
    let count = parts
        .get(2)
        .and_then(|s| s.parse().ok())
        .ok_or(LevelError::ParseError)?;
    Ok((kind, count))
}

pub fn parse_solution_part(parts: &[&str]) -> Result<LevelSolutionPart, LevelError> {
    if parts.len() < 6 {
        return Err(LevelError::ParseError);
    }
    let kind = parse_part_kind(parts[2])?;
    let x = parts
        .get(3)
        .and_then(|s| s.parse().ok())
        .ok_or(LevelError::ParseError)?;
    let y = parts
        .get(4)
        .and_then(|s| s.parse().ok())
        .ok_or(LevelError::ParseError)?;
    let rot = parts
        .get(6)
        .and_then(|s| s.parse().ok())
        .ok_or(LevelError::ParseError)?;
    let v1 = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Ok(LevelSolutionPart {
        kind,
        x,
        y,
        rot,
        v1,
    })
}

pub fn load_level(id: &str) -> Result<Level, LevelError> {
    let content = match id {
        "1" => include_str!("../levels/level_1.txt"),
        "2" => include_str!("../levels/level_2.txt"),
        "3" => include_str!("../levels/level_3.txt"),
        "4" => include_str!("../levels/level_4.txt"),
        "5" => include_str!("../levels/level_5.txt"),
        "6" => include_str!("../levels/level_6.txt"),
        "7" => include_str!("../levels/level_7.txt"),
        "8" => include_str!("../levels/level_8.txt"),
        _ => return Err(LevelError::ParseError),
    };
    parse_level(content)
}

pub fn parse_level(content: &str) -> Result<Level, LevelError> {
    let mut level = Level {
        id: String::new(),
        title: String::new(),
        hint: String::new(),
        grid: (10, 10),
        fixed_parts: Vec::new(),
        palette: Vec::new(),
        goals: Vec::new(),
        solution_wires: Vec::new(),
        solution_parts: Vec::new(),
    };

    let mut current_wire: Option<Vec<(i32, i32)>> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "id" => level.id = parts[1].into(),
            "title" => level.title = parts[1..].join(" "),
            "hint" => level.hint = parts[1..].join(" "),
            "grid" => {
                level.grid = (
                    parts
                        .get(1)
                        .and_then(|s| s.parse().ok())
                        .ok_or(LevelError::ParseError)?,
                    parts
                        .get(2)
                        .and_then(|s| s.parse().ok())
                        .ok_or(LevelError::ParseError)?,
                );
            }
            "fixed" => {
                level.fixed_parts.push(parse_fixed_part(&parts)?);
            }
            "palette" => {
                level.palette.push(parse_palette_entry(&parts)?);
            }
            "goal" => {
                level.goals.push(parts[1..].join(" "));
            }
            "solution" => {
                if parts[1] == "wire" {
                    let x1 = parts
                        .get(2)
                        .and_then(|s| s.parse().ok())
                        .ok_or(LevelError::ParseError)?;
                    let y1 = parts
                        .get(3)
                        .and_then(|s| s.parse().ok())
                        .ok_or(LevelError::ParseError)?;
                    let x2 = parts
                        .get(4)
                        .and_then(|s| s.parse().ok())
                        .ok_or(LevelError::ParseError)?;
                    let y2 = parts
                        .get(5)
                        .and_then(|s| s.parse().ok())
                        .ok_or(LevelError::ParseError)?;

                    if let Some(mut w) = current_wire.take() {
                        if w.last() == Some(&(x1, y1)) {
                            w.push((x2, y2));
                            current_wire = Some(w);
                        } else {
                            level.solution_wires.push(LevelSolutionWire { points: w });
                            current_wire = Some(alloc::vec![(x1, y1), (x2, y2)]);
                        }
                    } else {
                        current_wire = Some(alloc::vec![(x1, y1), (x2, y2)]);
                    }
                } else if parts[1] == "part" {
                    level.solution_parts.push(parse_solution_part(&parts)?);
                }
            }
            _ => return Err(LevelError::ParseError),
        }
    }
    if let Some(w) = current_wire {
        level.solution_wires.push(LevelSolutionWire { points: w });
    }
    if level.id.is_empty() {
        return Err(LevelError::ParseError);
    }
    Ok(level)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that parsing a valid part kind string works.
    #[test]
    fn parse_part_kind_valid() {
        assert_eq!(parse_part_kind("battery"), Ok(PartKind::Battery));
        assert_eq!(parse_part_kind("bulb"), Ok(PartKind::Bulb));
        assert_eq!(parse_part_kind("resistor"), Ok(PartKind::Resistor));
        assert_eq!(parse_part_kind("switch"), Ok(PartKind::Switch));
        assert_eq!(parse_part_kind("wire"), Ok(PartKind::Wire));
    }

    /// Verifies that parsing an invalid part kind string returns an error.
    #[test]
    fn parse_part_kind_invalid() {
        assert_eq!(parse_part_kind("unknown"), Err(LevelError::ParseError));
    }

    /// Verifies parsing a fixed part line array.
    #[test]
    fn parse_fixed_part_valid() {
        let parts = [
            "fixed", "battery", "1", "2", "rot", "90", "v1", "12.0", "v2", "0.0",
        ];
        let fixed = parse_fixed_part(&parts).unwrap();
        assert_eq!(fixed.kind, PartKind::Battery);
        assert_eq!(fixed.x, 1);
        assert_eq!(fixed.y, 2);
        assert_eq!(fixed.rot, 90);
        assert_eq!(fixed.v1, 12.0);
        assert_eq!(fixed.v2, 0.0);
    }

    /// Verifies parsing a palette entry line array.
    #[test]
    fn parse_palette_entry_valid() {
        let parts = ["palette", "wire", "10"];
        let pal = parse_palette_entry(&parts).unwrap();
        assert_eq!(pal, (PartKind::Wire, 10));
    }

    /// Verifies parsing a solution part line array.
    #[test]
    fn parse_solution_part_valid() {
        let parts = [
            "solution", "part", "switch", "3", "4", "rot", "180", "v1", "1.0",
        ];
        let sol = parse_solution_part(&parts).unwrap();
        assert_eq!(sol.kind, PartKind::Switch);
        assert_eq!(sol.x, 3);
        assert_eq!(sol.y, 4);
        assert_eq!(sol.rot, 180);
        assert_eq!(sol.v1, 1.0);
    }
}
