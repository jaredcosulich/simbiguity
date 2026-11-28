//! The handle table: every live 3D view, by the `u32` handle JS holds.

use std::cell::RefCell;
use std::vec::Vec;

use simbiguity_circuits_view::CircuitView;

use crate::readouts::READOUT_COUNT;
use crate::status::{fail, ERR_HANDLE, ERR_INDEX, OK};

/// One view and the readouts buffer JS reads it through.
pub(crate) struct Slot {
    pub(crate) view: CircuitView,
    pub(crate) readouts: [f64; READOUT_COUNT],
    #[cfg(feature = "puzzle")]
    pub(crate) puzzle: Option<simbiguity_circuits_puzzle::PuzzleSession>,
}

thread_local! {
    static TABLE: RefCell<Vec<Option<Slot>>> = const { RefCell::new(Vec::new()) };
}

fn no_view(h: u32) -> i32 {
    fail(ERR_HANDLE, std::format!("no 3D view with handle {h}"))
}

/// Stores a new view; returns its handle (never `0`).
pub(crate) fn insert(view: CircuitView) -> u32 {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        t.push(Some(Slot {
            view,
            readouts: [0.0; READOUT_COUNT],
            #[cfg(feature = "puzzle")]
            puzzle: None,
        }));
        t.len() as u32
    })
}

/// Drops the view behind `h`; [`ERR_HANDLE`] if there is none.
pub(crate) fn remove(h: u32) -> i32 {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        match (h as usize).checked_sub(1).and_then(|i| t.get_mut(i)) {
            Some(slot @ Some(_)) => {
                *slot = None;
                OK
            }
            _ => no_view(h),
        }
    })
}

/// Runs `f` on the view behind `h`; [`ERR_HANDLE`] if there is none.
pub(crate) fn with_view(h: u32, f: impl FnOnce(&mut Slot) -> i32) -> i32 {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        match (h as usize)
            .checked_sub(1)
            .and_then(|i| t.get_mut(i))
            .and_then(Option::as_mut)
        {
            Some(slot) => f(slot),
            None => no_view(h),
        }
    })
}

/// Runs `f` on mesh `i` of view `h`; [`ERR_INDEX`] for a mesh that does not exist.
pub(crate) fn with_mesh(h: u32, i: u32, f: impl FnOnce(&simbiguity_render::Mesh) -> i32) -> i32 {
    with_view(h, |s| match s.view.scene().meshes.get(i as usize) {
        Some(m) => f(m),
        None => fail(ERR_INDEX, std::format!("no mesh {i}")),
    })
}
