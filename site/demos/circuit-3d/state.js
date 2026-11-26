// The 3D demo's state and its URL-hash form, so any view is linkable and
// captures are pixel-stable: `#closed&v=16&t=2.5&cam=60,25,9`. Pure: no DOM.
//
//   closed      the switch starts closed
//   v=<volts>   battery voltage (clamped to the slider)
//   t=<secs>    freeze animation at exactly this time (for screenshots)
//   cam=<yaw>,<pitch>,<distance>   camera angles in degrees, distance in board units

export const BULB_VOLTS = 12;
export const VOLTS_MIN = 0;
export const VOLTS_MAX = 24;

/** The opening camera (matches simbiguity_circuits_view::default_camera). */
export const DEFAULT_CAMERA = Object.freeze({ yaw: 25, pitch: 38, distance: 9.5 });

const num = (s) => (s === undefined || s === "" ? NaN : Number(s));

/**
 * @param {string} hash with or without the leading `#`
 * @returns {{ closed: boolean, volts: number, fixedTime: number | null, camera: {yaw:number,pitch:number,distance:number} | null }}
 */
export function stateFromHash(hash) {
  // Anything after a `?` inside the hash (a cache-buster some tools append)
  // is not ours.
  const tokens = hash.replace(/^#/, "").split("?")[0].split("&").filter(Boolean);
  const value = (key) => tokens.find((t) => t.startsWith(`${key}=`))?.slice(key.length + 1);
  const v = num(value("v"));
  const t = num(value("t"));
  const cam = (value("cam") ?? "").split(",").map(num);
  const camera = cam.length === 3 && cam.every(Number.isFinite)
    ? { yaw: cam[0], pitch: cam[1], distance: cam[2] }
    : null;
  return {
    closed: tokens.includes("closed"),
    volts: Number.isFinite(v) ? Math.min(VOLTS_MAX, Math.max(VOLTS_MIN, v)) : BULB_VOLTS,
    fixedTime: Number.isFinite(t) && t >= 0 ? t : null,
    camera,
  };
}

const round = (x, d) => Number(x.toFixed(d));

/**
 * The hash for a state: `""` for the default, otherwise tokens that
 * `stateFromHash` reads back.
 */
export function hashFromState({ closed, volts, fixedTime = null, camera = null }) {
  const parts = [];
  if (closed) parts.push("closed");
  if (volts !== BULB_VOLTS) parts.push(`v=${volts}`);
  if (fixedTime !== null) parts.push(`t=${fixedTime}`);
  if (camera) parts.push(`cam=${round(camera.yaw, 1)},${round(camera.pitch, 1)},${round(camera.distance, 2)}`);
  return parts.length ? `#${parts.join("&")}` : "";
}
