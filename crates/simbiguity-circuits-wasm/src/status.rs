//! Status codes and the readable message behind the last failure.
//!
//! Every export that can fail returns an `i32`: `>= 0` is a result, `< 0` is
//! one of the codes below. After a negative code, `sb_last_error_ptr()` and
//! `sb_last_error_len()` locate a UTF-8 message in linear memory.

use std::cell::RefCell;
use std::string::String;

/// Success.
pub const OK: i32 = 0;
/// The circuit handle is unknown or was freed.
pub const ERR_HANDLE: i32 = -1;
/// The node id does not belong to this circuit.
pub const ERR_NODE: i32 = -2;
/// The component id does not belong to this circuit.
pub const ERR_COMPONENT: i32 = -3;
/// The component is the wrong kind for this call (toggling a bulb).
pub const ERR_KIND: i32 = -4;
/// The index is past the end of the list (an issue that does not exist).
pub const ERR_INDEX: i32 = -5;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
    static OUT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Records `message` as the last error and returns `code`.
pub(crate) fn fail(code: i32, message: impl Into<String>) -> i32 {
    LAST_ERROR.with(|e| *e.borrow_mut() = message.into());
    code
}

/// Puts `s` in the shared output string and returns its length in bytes.
pub(crate) fn output(s: String) -> i32 {
    let len = s.len() as i32;
    OUT.with(|o| *o.borrow_mut() = s);
    len
}

/// The last error message, for native callers and tests.
pub fn last_error() -> String {
    LAST_ERROR.with(|e| e.borrow().clone())
}

/// Address of the last error message (UTF-8).
#[no_mangle]
pub extern "C" fn sb_last_error_ptr() -> *const u8 {
    LAST_ERROR.with(|e| e.borrow().as_ptr())
}

/// Length in bytes of the last error message.
#[no_mangle]
pub extern "C" fn sb_last_error_len() -> u32 {
    LAST_ERROR.with(|e| e.borrow().len() as u32)
}

/// Address of the string the last string-returning export produced.
#[no_mangle]
pub extern "C" fn sb_out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

/// Length in bytes of that string.
#[no_mangle]
pub extern "C" fn sb_out_len() -> u32 {
    OUT.with(|o| o.borrow().len() as u32)
}
