use std::cell::RefCell;
use std::string::String;

pub const OK: i32 = 0;
pub const ERR_HANDLE: i32 = -1;
pub const ERR_INDEX: i32 = -5;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

pub(crate) fn fail(code: i32, message: String) -> i32 {
    LAST_ERROR.with(|e| *e.borrow_mut() = message);
    code
}

#[no_mangle]
pub extern "C" fn sb_last_error_ptr() -> *const u8 {
    LAST_ERROR.with(|e| e.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn sb_last_error_len() -> u32 {
    LAST_ERROR.with(|e| e.borrow().len() as u32)
}
