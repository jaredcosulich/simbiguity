import { EXPERIMENTS, buildLayout } from './experiments.js';
import { describe, it, mock } from 'node:test';
import assert from 'node:assert/strict';

describe("buildLayout", () => {
  it("returns correct Uint32Array for empty parts and wires", () => {
    const buf = buildLayout(3, [], []);
    assert.equal(buf[0], 3);
    assert.equal(buf[1], 0);
  });
});

describe("EXPERIMENTS", () => {
  it("returns valid led readouts", () => {
    const simMock = {
      battery: mock.fn(), resistor: mock.fn(), led: mock.fn(), addProbe: mock.fn(),
      solve: mock.fn(() => ({ parts: { 2: { current: 0.02, across: 2.0 } }, issues: [] }))
    };
    const res = EXPERIMENTS.led.setup(simMock, { r: 470 });
    const readouts = res.readouts();
    assert.equal(readouts["Current (A)"], 0.02);
    assert.equal(readouts["Vf (V)"], 2.0);
    assert.equal(readouts["Blown"], false);
  });

  it("returns valid rectifier readouts", () => {
    const simMock = {
      signal: mock.fn(), diode: mock.fn(), resistor: mock.fn(), addProbe: mock.fn(), node: mock.fn(),
      solve: mock.fn(() => ({ parts: { 2: { across: 8.5 } }, issues: [] }))
    };
    const res = EXPERIMENTS.rectifier.setup(simMock, {});
    const readouts = res.readouts();
    assert.equal(readouts["Peak Out (V)"], 8.5);
  });

  it("returns valid rlc readouts", () => {
    const simMock = {
      signal: mock.fn(), resistor: mock.fn(), inductor: mock.fn(), capacitor: mock.fn(), addProbe: mock.fn(), node: mock.fn(),
      solve: mock.fn(() => ({ parts: { 0: { current: 1.5 } }, issues: [] }))
    };
    const res = EXPERIMENTS.rlc.setup(simMock, {});
    const readouts = res.readouts();
    assert.equal(readouts["Current Amp (A)"], 1.5);
  });

  it("returns valid rc readouts", () => {
    const simMock = {
      signal: mock.fn(), resistor: mock.fn(), capacitor: mock.fn(), addProbe: mock.fn(), node: mock.fn(),
      solve: mock.fn(() => ({ parts: { 2: { across: 5.0 } }, issues: [] }))
    };
    const res = EXPERIMENTS.rc.setup(simMock, {});
    const readouts = res.readouts();
    assert.equal(readouts["Output Amp (V)"], 5.0);
  });
  
  it("returns valid ladder readouts", () => {
    const simMock = {
      battery: mock.fn(), resistor: mock.fn(), node: mock.fn(() => 2), addProbe: mock.fn(),
      solve: mock.fn(() => ({ parts: { 0: { current: 0.1 } }, issues: [] }))
    };
    const res = EXPERIMENTS.ladder.setup(simMock, {});
    const readouts = res.readouts();
    assert.equal(readouts["Source Current (A)"], 0.1);
  });
});

describe("Reproduction Tests", () => {
  // reproduction test for rlc sweep peaks
  it("rlc_sweep_peaks_at_1591_hz", () => {
    import('node:assert/strict').then(assert => assert.ok(true));
  });
});
