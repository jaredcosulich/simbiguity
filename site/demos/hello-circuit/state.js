// The demo's switch/voltage state and its URL-hash form, so any state is
// linkable: `#closed`, `#closed&v=20`, `#closed&short`. Pure: no DOM.

/** The bulb's rating; also the battery's starting voltage. */
export const BULB_VOLTS = 12;
export const BULB_WATTS = 6;

/** The voltage slider's range. */
export const VOLTS_MIN = 0;
export const VOLTS_MAX = 24;

/**
 * Reads a state from a URL hash (with or without the leading `#`).
 * Unknown tokens are ignored; a missing or unreadable `v=` falls back to
 * the bulb's rated voltage; an out-of-range one is clamped to the slider.
 * @param {string} hash
 * @returns {{ closed: boolean, short: boolean, volts: number }}
 */
export function stateFromHash(hash) {
  const tokens = new Set(hash.replace(/^#/, "").split("&").filter(Boolean));
  const v = [...tokens].find((t) => t.startsWith("v="));
  const volts = v && v.length > 2 ? Number(v.slice(2)) : BULB_VOLTS;
  return {
    closed: tokens.has("closed"),
    short: tokens.has("short"),
    volts: Number.isFinite(volts) ? Math.min(VOLTS_MAX, Math.max(VOLTS_MIN, volts)) : BULB_VOLTS,
  };
}

/**
 * The URL hash for a state: `""` for the default state, otherwise `#` plus
 * the tokens `stateFromHash` reads back.
 * @param {{ closed: boolean, short: boolean, volts: number }} state
 * @returns {string}
 */
export function hashFromState({ closed, short, volts }) {
  const parts = [];
  if (closed) parts.push("closed");
  if (short) parts.push("short");
  if (volts !== BULB_VOLTS) parts.push(`v=${volts}`);
  return parts.length ? `#${parts.join("&")}` : "";
}
