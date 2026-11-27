//! Status codes and the readable message behind the last failure.
//!
//! Every export that can fail returns an `i32`: `>= 0` is a result, `< 0` is
//! one of the codes below. After a negative code, `sb_last_error_ptr()` and
//! `sb_last_error_len()` locate a UTF-8 message in linear memory.

use std::cell::RefCell;
use std::string::String;

/// Success.
pub const OK: i32 = 0;
/// The handle is unknown or was freed.
pub const ERR_HANDLE: i32 = -1;
/// The index is out of range (a mesh that does not exist).
pub const ERR_INDEX: i32 = -5;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Records `message` as the last error and returns `code`.
pub fn fail(code: i32, message: String) -> i32 {
    LAST_ERROR.with(|e| *e.borrow_mut() = message);
    code
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
