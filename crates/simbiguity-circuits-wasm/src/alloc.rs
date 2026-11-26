//! Byte buffers JavaScript can fill and hand back.
//!
//! JS cannot allocate inside WASM linear memory by itself, so it asks:
//! `ptr = sb_alloc(len)`, writes `len` bytes at `memory.buffer[ptr..]`, passes
//! `(ptr, len)` to an export, then `sb_free(ptr, len)`.

use std::vec::Vec;

/// Allocates `len` bytes in linear memory and returns their address. The
/// bytes are uninitialised; free them with [`sb_free`] and the same `len`.
#[no_mangle]
pub extern "C" fn sb_alloc(len: u32) -> *mut u8 {
    let mut buf: Vec<u8> = Vec::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr
}

/// Releases a buffer from [`sb_alloc`].
///
/// # Safety
/// `ptr` must come from `sb_alloc(len)` with this exact `len`, and must not
/// have been freed already.
#[no_mangle]
pub unsafe extern "C" fn sb_free(ptr: *mut u8, len: u32) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: the caller guarantees `ptr` came from `sb_alloc(len)`, whose
    // Vec had capacity `len` and length 0 bytes initialised that we drop.
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len as usize) });
}
