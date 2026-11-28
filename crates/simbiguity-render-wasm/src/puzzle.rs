#![allow(clippy::not_unsafe_ptr_arg_deref)]
use crate::slot::{insert, with_view, Slot};
use crate::status::{fail, OK};
use core::ffi::CStr;
use format;
use simbiguity_circuits::sim::CircuitSim;
use simbiguity_circuits_puzzle::level_format::{load_level, LevelSolutionPart, LevelSolutionWire};
use simbiguity_circuits_puzzle::share::{decode_solution, encode_solution};
use simbiguity_circuits_puzzle::{CircuitGoal, IssueKind, PartKind, PuzzleSession};
use simbiguity_circuits_view::CircuitView;
use simbiguity_core::Simulation;
use simbiguity_puzzle::Goal;
use std::cell::RefCell;
use std::string::String;
use Vec;

thread_local! {
    static LAST_OUT: RefCell<String> = const { RefCell::new(String::new()) };
}
pub fn set_out(message: String) {
    LAST_OUT.with(|e| *e.borrow_mut() = message);
}
#[no_mangle]
pub extern "C" fn sb_last_out_ptr() -> *const u8 {
    LAST_OUT.with(|e| e.borrow().as_ptr())
}
#[no_mangle]
pub extern "C" fn sb_last_out_len() -> u32 {
    LAST_OUT.with(|e| e.borrow().len() as u32)
}

fn build_puzzle_view(session: &PuzzleSession) -> CircuitView {
    let circuit = simbiguity_circuits_puzzle::build_circuit(&session.level, &session.edits);
    let mut parts = Vec::new();
    let mut wires = Vec::new();
    let mut cid = 0usize;

    for w in &session.edits.wires {
        for i in 0..w.points.len().saturating_sub(1) {
            let (x1, y1) = w.points[i];
            let (x2, y2) = w.points[i + 1];
            wires.push(simbiguity_circuits_view::board_layout::WireRoute {
                component: unsafe {
                    core::mem::transmute::<usize, simbiguity_circuits::ComponentId>(cid)
                },
                points: vec![(x1, y1), (x2, y2)],
            });
            cid += 1;
        }
    }

    let to_rot = |rot: i32| match rot {
        0 => simbiguity_circuits_view::board_layout::Rotation::Deg90,
        90 => simbiguity_circuits_view::board_layout::Rotation::Deg0,
        180 => simbiguity_circuits_view::board_layout::Rotation::Deg270,
        270 => simbiguity_circuits_view::board_layout::Rotation::Deg180,
        _ => simbiguity_circuits_view::board_layout::Rotation::Deg0,
    };

    for p in &session.level.fixed_parts {
        parts.push(simbiguity_circuits_view::board_layout::PartPlacement {
            component: unsafe {
                core::mem::transmute::<usize, simbiguity_circuits::ComponentId>(cid)
            },
            cell: (p.x, p.y),
            rotation: to_rot(p.rot),
        });
        cid += 1;
    }
    for p in &session.edits.parts {
        parts.push(simbiguity_circuits_view::board_layout::PartPlacement {
            component: unsafe {
                core::mem::transmute::<usize, simbiguity_circuits::ComponentId>(cid)
            },
            cell: (p.x, p.y),
            rotation: to_rot(p.rot),
        });
        cid += 1;
    }

    let layout = simbiguity_circuits_view::board_layout::BoardLayout { parts, wires };
    CircuitView::from_layout(circuit, layout).unwrap_or_else(|_| CircuitView::new(12.0, false))
}

fn update_view(slot: &mut Slot) {
    if let Some(session) = &slot.puzzle {
        slot.view = build_puzzle_view(session);
    }
}

fn kind_from_int(kind: i32) -> Option<PartKind> {
    match kind {
        1 => Some(PartKind::Battery),
        2 => Some(PartKind::Switch),
        3 => Some(PartKind::Bulb),
        4 => Some(PartKind::Wire),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn puzzle_load(id_ptr: *const core::ffi::c_char) -> u32 {
    let id_str = unsafe { CStr::from_ptr(id_ptr) }.to_str().unwrap_or("");
    match load_level(id_str) {
        Ok(level) => {
            let session = PuzzleSession::new(level);
            let view = build_puzzle_view(&session);
            let h = insert(view);
            crate::slot::with_view(h, |slot| {
                slot.puzzle = Some(session);
                OK
            });
            h
        }
        Err(_) => {
            fail(-1, "failed to load level".into());
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn puzzle_place(h: u32, kind: i32, x: i32, z: i32, rot: i32) -> i32 {
    with_view(h, |slot| {
        let Some(session) = &mut slot.puzzle else {
            return -1;
        };
        let Some(k) = kind_from_int(kind) else {
            return -1;
        };
        if k == PartKind::Wire {
            let (dx, dy) = match rot {
                0 => (1, 0),
                90 => (0, 1),
                180 => (-1, 0),
                270 => (0, -1),
                _ => (1, 0),
            };
            session.place(LevelSolutionWire {
                points: vec![(x, z), (x + dx, z + dy)],
            });
        } else {
            use simbiguity_core::Snapshot;
            let snap = session.snapshot();
            let mut v1 = 0.0;
            if k == PartKind::Battery {
                v1 = 9.0;
            }
            if k == PartKind::Bulb {
                v1 = 9.0;
            }
            session.edits.parts.push(LevelSolutionPart {
                kind: k,
                x,
                y: z,
                rot,
                v1,
            });
            session.stack.push(snap);
        }
        update_view(slot);
        OK
    })
}

#[no_mangle]
pub extern "C" fn puzzle_remove(h: u32, x: i32, z: i32) -> i32 {
    with_view(h, |slot| {
        let Some(session) = &mut slot.puzzle else {
            return -1;
        };
        use simbiguity_core::Snapshot;
        let snap = session.snapshot();
        let mut changed = false;

        if let Some(pos) = session
            .edits
            .parts
            .iter()
            .position(|p| p.x == x && p.y == z)
        {
            session.edits.parts.remove(pos);
            changed = true;
        } else if let Some(pos) = session.edits.wires.iter().position(|w| {
            w.points.contains(&(x, z))
                || w.points.contains(&(x - 1, z))
                || w.points.contains(&(x, z - 1))
        }) {
            // Actually just any wire that passes through this coordinate
            // We'll just check if it contains the exact endpoint or startpoint
            session.edits.wires.remove(pos);
            changed = true;
        }

        if changed {
            session.stack.push(snap);
            update_view(slot);
        }
        OK
    })
}

#[no_mangle]
pub extern "C" fn puzzle_undo(h: u32) -> i32 {
    with_view(h, |slot| {
        if let Some(session) = &mut slot.puzzle {
            session.undo();
            update_view(slot);
        }
        OK
    })
}

#[no_mangle]
pub extern "C" fn puzzle_redo(h: u32) -> i32 {
    with_view(h, |slot| {
        if let Some(session) = &mut slot.puzzle {
            session.redo();
            update_view(slot);
        }
        OK
    })
}

#[no_mangle]
pub extern "C" fn puzzle_reset(h: u32) -> i32 {
    with_view(h, |slot| {
        if let Some(session) = &mut slot.puzzle {
            // clear edits and stack
            session.edits.parts.clear();
            session.edits.wires.clear();
            session.stack = simbiguity_puzzle::UndoStack::new();
            update_view(slot);
        }
        OK
    })
}

fn parse_goal(s: &str) -> Option<CircuitGoal> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    match parts[0] {
        "lit_bulbs" => {
            let n = parts.get(1)?.parse().ok()?;
            Some(CircuitGoal::LitBulbs(n))
        }
        "no_issue" => {
            let kind = match *parts.get(1)? {
                "short_circuit" => IssueKind::ShortCircuit,
                "bulb_blown" => IssueKind::BulbBlown,
                "invalid_value" => IssueKind::InvalidValue,
                "battery_loop" => IssueKind::VoltageSourceLoop,
                _ => return None,
            };
            Some(CircuitGoal::NoIssue(kind))
        }
        "part_budget" => {
            let kind = match *parts.get(1)? {
                "wire" => PartKind::Wire,
                _ => return None,
            };
            let n = parts.get(2)?.parse().ok()?;
            Some(CircuitGoal::PartBudget { kind, max: n })
        }
        _ => None,
    }
}

fn format_goal(s: &str) -> String {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return String::new();
    }
    match parts[0] {
        "lit_bulbs" => format!(
            "Light {} bulb{}",
            parts.get(1).unwrap_or(&"0"),
            if parts.get(1) == Some(&"1") { "" } else { "s" }
        ),
        "no_issue" => match *parts.get(1).unwrap_or(&"") {
            "short_circuit" => "No short circuits".into(),
            "bulb_blown" => "No blown bulbs".into(),
            _ => format!("No {}", parts.get(1).unwrap_or(&"issue")),
        },
        "part_budget" => format!(
            "Use at most {} {}",
            parts.get(2).unwrap_or(&"0"),
            parts.get(1).unwrap_or(&"parts")
        ),
        _ => s.into(),
    }
}

#[no_mangle]
pub extern "C" fn puzzle_goal_count(h: u32) -> i32 {
    let mut count = -1;
    with_view(h, |slot| {
        if let Some(session) = &slot.puzzle {
            count = session.level.goals.len() as i32;
        }
        OK
    });
    count
}

#[no_mangle]
pub extern "C" fn puzzle_goal_text(h: u32, i: u32) -> i32 {
    with_view(h, |slot| {
        if let Some(session) = &slot.puzzle {
            if let Some(text) = session.level.goals.get(i as usize) {
                set_out(format_goal(text));
                return OK;
            }
        }
        fail(-1, "goal not found".into())
    })
}

#[no_mangle]
pub extern "C" fn puzzle_goal_status(h: u32, i: u32) -> i32 {
    let mut status = 0;
    with_view(h, |slot| {
        if let Some(session) = &slot.puzzle {
            if let Some(text) = session.level.goals.get(i as usize) {
                if let Some(goal) = parse_goal(text) {
                    let circuit =
                        simbiguity_circuits_puzzle::build_circuit(&session.level, &session.edits);
                    let mut sim = CircuitSim::new(circuit);
                    // run a few steps to let it settle
                    for _ in 0..10 {
                        sim.step(simbiguity_core::Seconds(0.01));
                    }
                    if goal.check(&sim) {
                        status = 1;
                    }
                }
            }
        }
        OK
    });
    status
}

#[no_mangle]
pub extern "C" fn puzzle_issue_text(h: u32) -> i32 {
    with_view(h, |slot| {
        if let Some(session) = &slot.puzzle {
            let circuit = simbiguity_circuits_puzzle::build_circuit(&session.level, &session.edits);
            let mut sim = CircuitSim::new(circuit);
            sim.step(simbiguity_core::Seconds(0.01));
            let state = sim.state();
            if let Some(issue) = state.issues().first() {
                let msg = match issue {
                    simbiguity_circuits::CircuitIssue::ShortCircuit { .. } => {
                        "Short Circuit! The battery is connected directly to itself."
                    }
                    simbiguity_circuits::CircuitIssue::BulbBlown { .. } => {
                        "Bulb Blown! Too much voltage."
                    }
                    simbiguity_circuits::CircuitIssue::OpenCircuit { .. } => "Open circuit.",
                    simbiguity_circuits::CircuitIssue::FloatingNode { .. } => "Floating node.",
                    _ => "Circuit Issue",
                };
                set_out(msg.into());
                return 1;
            }
        }
        set_out("".into());
        0
    })
}

#[no_mangle]
pub extern "C" fn puzzle_share(h: u32) -> i32 {
    with_view(h, |slot| {
        if let Some(session) = &slot.puzzle {
            let code = encode_solution(session);
            set_out(code);
            OK
        } else {
            fail(-1, "no session".into())
        }
    })
}

#[no_mangle]
pub extern "C" fn puzzle_load_share(h: u32, s_ptr: *const core::ffi::c_char) -> i32 {
    let code = unsafe { CStr::from_ptr(s_ptr) }.to_str().unwrap_or("");
    with_view(h, |slot| {
        if let Some(session) = &mut slot.puzzle {
            match decode_solution(code, &session.level.id) {
                Ok(attempt) => {
                    // Attempt data is raw bytes of wires.
                    // Instead of parsing bytes directly here, wait!
                    // decode_solution just checks CRC. How do we apply the attempt?
                    // Oh, wait, the `share.rs` we read earlier had:
                    // `attempt_data` = the wires. But wait, `PuzzleSession` doesn't have a `load_attempt` function!
                    // Let's implement reading the data right here.
                    let mut i = 0;
                    let data = &attempt.data;
                    session.edits.wires.clear();
                    session.edits.parts.clear();
                    while i < data.len() {
                        let mut w = LevelSolutionWire { points: Vec::new() };
                        while i < data.len() && data[i] != 255 {
                            if i + 1 < data.len() {
                                w.points.push((data[i] as i32, data[i + 1] as i32));
                            }
                            i += 2;
                        }
                        if !w.points.is_empty() {
                            session.edits.wires.push(w);
                        }
                        i += 1; // skip 255
                    }
                    update_view(slot);
                    OK
                }
                Err(e) => {
                    let msg = match e {
                        simbiguity_circuits_puzzle::share::ShareError::BadCharacter => {
                            "Bad character in share link."
                        }
                        simbiguity_circuits_puzzle::share::ShareError::TooShort => {
                            "Share link is too short."
                        }
                        simbiguity_circuits_puzzle::share::ShareError::UnknownVersion => {
                            "Unknown share link version."
                        }
                        simbiguity_circuits_puzzle::share::ShareError::CrcMismatch => {
                            "Share link is corrupted."
                        }
                        simbiguity_circuits_puzzle::share::ShareError::LevelMismatch => {
                            "Share link is for a different level."
                        }
                    };
                    fail(-1, msg.into())
                }
            }
        } else {
            fail(-1, "no session".into())
        }
    })
}

#[no_mangle]
pub extern "C" fn puzzle_level_count() -> i32 {
    8
}

#[no_mangle]
pub extern "C" fn puzzle_level_title(i: u32) -> i32 {
    match load_level(&format!("{}", i + 1)) {
        Ok(l) => {
            set_out(l.title);
            OK
        }
        Err(_) => fail(-1, "level not found".into()),
    }
}

#[no_mangle]
pub extern "C" fn puzzle_pick_cell(
    h: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    out_x: *mut i32,
    out_z: *mut i32,
) -> i32 {
    crate::slot::with_view(h, |s| {
        let ray = s.view.camera.ray(x, y, width, height);
        if ray.dir.y < -1e-6 {
            let t = -ray.origin.y / ray.dir.y;
            let px = ray.origin.x + t * ray.dir.x;
            let pz = ray.origin.z + t * ray.dir.z;
            unsafe {
                *out_x = (px / 2.0).round() as i32;
                *out_z = (pz / 2.0).round() as i32;
            }
            return 1;
        }
        0
    })
}

#[no_mangle]
pub extern "C" fn puzzle_part_cell(
    h: u32,
    cid_plus_1: u32,
    out_x: *mut i32,
    out_z: *mut i32,
) -> i32 {
    crate::slot::with_view(h, |s| {
        if let Some(session) = &s.puzzle {
            let target_cid = cid_plus_1 as usize - 1;
            let mut cid = 0;
            for w in &session.edits.wires {
                for i in 0..w.points.len().saturating_sub(1) {
                    if cid == target_cid {
                        unsafe {
                            *out_x = w.points[i].0;
                            *out_z = w.points[i].1;
                        }
                        return 1;
                    }
                    cid += 1;
                }
            }
            for p in &session.level.fixed_parts {
                if cid == target_cid {
                    unsafe {
                        *out_x = p.x;
                        *out_z = p.y;
                    }
                    return 1;
                }
                cid += 1;
            }
            for p in &session.edits.parts {
                if cid == target_cid {
                    unsafe {
                        *out_x = p.x;
                        *out_z = p.y;
                    }
                    return 1;
                }
                cid += 1;
            }
        }
        0
    })
}
