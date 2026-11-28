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
    setup: (sim, params) => {
      const r = params.r || 470;
      const v = sim.battery(1, 0, 9.0);
      const res = sim.resistor(1, 2, r);
      const led = sim.led(2, 0, "red", 0.02, 1.5195e-17, 2.0);
      
      const parts = [
        { kind: 1, a: 1, b: 0, x: -2, y: 0, emf: 9.0 },
        { kind: 8, a: 1, b: 2, x: 0, y: 0, r: r },
        { kind: 6, a: 2, b: 0, x: 2, y: 0, ratedAmps: 0.02, is: 1.5195e-17, n: 2.0 }
      ];
      const wires = [
        { a: 0, b: 0, pts: [[-2,0], [-2,-2], [2,-2], [2,0]] },
        { a: 1, b: 1, pts: [[-2,0], [0,0]] },
        { a: 2, b: 2, pts: [[0,0], [2,0]] }
      ];
      const layout = { nodeCount: 3, parts, wires };
      
      const p1 = sim.addProbe(true, led, 2000);
      return { 
        circuit: sim, probes: [p1, null], layout, 
        readouts: () => {
          const res = sim.solve();
          const ledData = res.parts[2] || {};
          const isBlown = res.issues.some(i => i.kind === 'bulb-blown');
          return { "Current (A)": ledData.current || 0, "Vf (V)": ledData.across || 0, "Blown": isBlown, "Issue": isBlown ? res.issues.find(i=>i.kind==='bulb-blown').message : "" };
        },
        sampleTime: 1e-4
      };
    }
  },
  rectifier: {
    setup: (sim, params) => {
      const src = sim.signal(1, 0, 0, 10, 50, 0, 0);
      const diode = sim.diode(1, 2, 1e-14, 1.0);
      const res = sim.resistor(2, 0, 1000);
      
      const parts = [
        { kind: 7, a: 1, b: 0, x: -2, y: 0, waveform: 0, amplitude: 10, frequency: 50, phase: 0, offset: 0 },
        { kind: 5, a: 1, b: 2, x: 0, y: 0, is: 1e-14, n: 1.0 },
        { kind: 8, a: 2, b: 0, x: 2, y: 0, r: 1000 }
      ];
      const layout = { nodeCount: 3, parts, wires: [
        { a: 0, b: 0, pts: [[-2,0], [-2,-2], [2,-2], [2,0]] },
        { a: 1, b: 1, pts: [[-2,0], [0,0]] },
        { a: 2, b: 2, pts: [[0,0], [2,0]] }
      ]};
      
      const p1 = sim.addProbe(false, src, 2000);
      const p2 = sim.addProbe(false, res, 2000);
      
      return {
        circuit: sim, probes: [p1, p2], layout,
        readouts: () => {
          const resData = sim.solve();
          const rData = resData.parts[2] || {};
          return { "Peak Out (V)": rData.across || 0 };
        },
        sampleTime: 1/50/200
      };
    }
  },
  rlc: {
    setup: (sim, params) => {
      sim.node(); sim.node(); sim.node(); sim.node();
      const freq = params.freq || 1591.5;
      const src = sim.signal(1, 0, 0, 1, freq, 0, 0);
      const r = sim.resistor(1, 2, 10);
      const l = sim.inductor(2, 3, 0.010);
      const c = sim.capacitor(3, 0, 1e-6);
      const parts = [
        { kind: 7, a: 1, b: 0, x: -2, y: 0, waveform: 0, amplitude: 1, frequency: freq, phase: 0, offset: 0 },
        { kind: 8, a: 1, b: 2, x: 0, y: 0, r: 10 },
        { kind: 10, a: 2, b: 3, x: 2, y: 0, h: 0.010 },
        { kind: 9, a: 3, b: 0, x: 4, y: 0, f: 1e-6 }
      ];
      const wires = [
        { a: 0, b: 0, pts: [[-2,0], [-2,-2], [4,-2], [4,0]] },
        { a: 1, b: 1, pts: [[-2,0], [0,0]] },
        { a: 2, b: 2, pts: [[0,0], [2,0]] },
        { a: 3, b: 3, pts: [[2,0], [4,0]] }
      ];
      const layout = { nodeCount: 4, parts, wires };
      
      const p1 = sim.addProbe(true, src, 2000);
      return { circuit: sim, probes: [p1, null], layout, 
        readouts: () => {
          const resData = sim.solve();
          const srcData = resData.parts[0] || {};
          return { "Current Amp (A)": srcData.current || 0 }; 
        }, 
        sampleTime: 1/freq/200 
      };
    }
  },
  rc: {
    setup: (sim, params) => {
      const freq = params.freq || 159.15;
      const src = sim.signal(1, 0, 0, 1, freq, 0, 0);
      const r = sim.resistor(1, 2, 1000);
      const c = sim.capacitor(2, 0, 1e-6);
      
      const parts = [
        { kind: 7, a: 1, b: 0, x: -2, y: 0, waveform: 0, amplitude: 1, frequency: freq, phase: 0, offset: 0 },
        { kind: 8, a: 1, b: 2, x: 0, y: 0, r: 1000 },
        { kind: 9, a: 2, b: 0, x: 2, y: 0, f: 1e-6 }
      ];
      const wires = [
        { a: 0, b: 0, pts: [[-2,0], [-2,-2], [2,-2], [2,0]] },
        { a: 1, b: 1, pts: [[-2,0], [0,0]] },
        { a: 2, b: 2, pts: [[0,0], [2,0]] }
      ];
      const layout = { nodeCount: 3, parts, wires };
      
      const p1 = sim.addProbe(false, src, 2000);
      const p2 = sim.addProbe(false, c, 2000);
      return { circuit: sim, probes: [p1, p2], layout, 
        readouts: () => {
          const resData = sim.solve();
          return { "Output Amp (V)": (resData.parts[2] || {}).across || 0, "Phase Lag": 0 }; 
        }, 
        sampleTime: 1/freq/200 
      };
    }
  },
  square_rc: {
    setup: (sim, params) => {
      const src = sim.signal(1, 0, 1, 1, 10, 0, 0);
      const r = sim.resistor(1, 2, 1000);
      const c = sim.capacitor(2, 0, 1e-6);
      
      const parts = [
        { kind: 7, a: 1, b: 0, x: -2, y: 0, waveform: 1, amplitude: 1, frequency: 10, phase: 0, offset: 0 },
        { kind: 8, a: 1, b: 2, x: 0, y: 0, r: 1000 },
        { kind: 9, a: 2, b: 0, x: 2, y: 0, f: 1e-6 }
      ];
      const wires = [
        { a: 0, b: 0, pts: [[-2,0], [-2,-2], [2,-2], [2,0]] },
        { a: 1, b: 1, pts: [[-2,0], [0,0]] },
        { a: 2, b: 2, pts: [[0,0], [2,0]] }
      ];
      const layout = { nodeCount: 3, parts, wires };
      
      const p1 = sim.addProbe(false, src, 2000);
      const p2 = sim.addProbe(false, c, 2000);
      return { circuit: sim, probes: [p1, p2], layout, readouts: () => { return {}; }, sampleTime: 1/10/200 };
    }
  },
  ladder: {
    setup: (sim, params) => {
      const v = sim.battery(1, 0, 9);
      let parts = [
        { kind: 1, a: 1, b: 0, x: -2, y: 0, emf: 9 }
      ];
      let wires = [
        { a: 0, b: 0, pts: [[-2,0], [-2,-2]] },
        { a: 1, b: 1, pts: [[-2,0], [-2,2]] }
      ];
      
      let lastTop = 1;
      let nodeCount = 2;
      
      for(let i=0; i<1000; i++) {
        let top = sim.node();
        sim.resistor(lastTop, top, 1);
        sim.resistor(top, 0, 1);
        nodeCount++;
        
        parts.push({ kind: 8, a: lastTop, b: top, x: i*2, y: 2, r: 1 });
        parts.push({ kind: 8, a: top, b: 0, x: i*2, y: 0, r: 1 });
        
        wires.push({ a: lastTop, b: lastTop, pts: [[(i-1)*2, 2], [i*2, 2]] });
        wires.push({ a: top, b: top, pts: [[i*2, 2], [i*2, 0]] });
        
        lastTop = top;
      }
      
      const layout = { nodeCount, parts, wires };
      const p1 = sim.addProbe(true, v, 2000);
      return { circuit: sim, probes: [p1, null], layout, 
        readouts: () => {
          const t0 = performance.now();
          const resData = sim.solve();
          const t1 = performance.now();
          return { "Source Current (A)": (resData.parts[0] || {}).current || 0, "Solve Time (ms)": t1 - t0 };
        }, 
        sampleTime: 1e-4 
      };
    }
  }
};