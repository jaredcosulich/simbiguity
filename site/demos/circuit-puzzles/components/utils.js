export const $ = (id) => document.getElementById(id);

export const stringFromWasm = (w) => {
    const ptr = w.sb_last_out_ptr();
    const len = w.sb_last_out_len();
    return new TextDecoder().decode(new Uint8Array(w.memory.buffer, ptr, len));
};