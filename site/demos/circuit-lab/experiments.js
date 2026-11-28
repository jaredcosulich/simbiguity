export function buildLayout(nodeCount, parts, wires) {
  const buf = new Uint32Array(2000);
  let pos = 0;
  buf[pos++] = nodeCount;
  buf[pos++] = parts.length + wires.length;

  const putF64 = (v) => {
    const arr = new Float64Array([v]);
    const u32 = new Uint32Array(arr.buffer);
    buf[pos++] = u32[0];
    buf[pos++] = u32[1];
  };

  for (const p of parts) {
    buf[pos++] = p.kind;
    buf[pos++] = p.a;
    buf[pos++] = p.b;
    buf[pos++] = p.x >>> 0;
    buf[pos++] = p.y >>> 0;
    buf[pos++] = p.rot || 0;
    
    if (p.kind === 1) { putF64(p.emf); }
    else if (p.kind === 2) { buf[pos++] = p.closed ? 1 : 0; }
    else if (p.kind === 3) { putF64(p.volts); putF64(p.watts); }
    else if (p.kind === 5) { putF64(p.is || 1e-14); putF64(p.n || 1.0); }
    else if (p.kind === 6) { putF64(p.ratedAmps || 0.02); putF64(p.is || 1.5195e-17); putF64(p.n || 2.0); }
    else if (p.kind === 7) { buf[pos++] = p.waveform; putF64(p.amplitude); putF64(p.frequency); putF64(p.phase); putF64(p.offset); }
    else if (p.kind === 8) { putF64(p.r); }
    else if (p.kind === 9) { putF64(p.f); }
    else if (p.kind === 10) { putF64(p.h); }
  }

  for (const w of wires) {
    buf[pos++] = 4;
    buf[pos++] = w.a;
    buf[pos++] = w.b;
    buf[pos++] = w.pts.length;
    for (const [x, y] of w.pts) {
      buf[pos++] = x >>> 0;
      buf[pos++] = y >>> 0;
    }
  }

  return buf.subarray(0, pos);
}

export const EXPERIMENTS = {
  led: {
    setup: (params) => {
      const parts = [
        { kind: 1, a: 1, b: 0, x: -2, y: 0, emf: 9.0 },
        { kind: 8, a: 1, b: 2, x: 0, y: 0, r: params.r || 470 }, // Resistor
        { kind: 6, a: 2, b: 0, x: 2, y: 0, ratedAmps: 0.02, is: 1.5195e-17, n: 2.0 } // LED
      ];
      const wires = [
        { a: 1, b: 1, pts: [[-2, 0], [0, 0]] }, // actually we don't strictly need wires if we just want a circuit simulation, but board_layout requires all connected. Wait, board_layout.validate checks wires! "DanglingWire".
      ];
      // Let's just create a valid layout.
      return { nodeCount: 3, parts, wires: [
        { a: 0, b: 0, pts: [[-2,0], [-2,-2], [2,-2], [2,0]] },
        { a: 1, b: 1, pts: [[-2,0], [0,0]] },
        { a: 2, b: 2, pts: [[0,0], [2,0]] }
      ] };
    }
  }
};