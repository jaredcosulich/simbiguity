import { findTrigger, toPixels } from './scope.js';
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

describe("findTrigger", () => {
  it("finds trigger when crossing level from below", () => {
    assert.equal(findTrigger([0, 0.5, 1.5, 2], 1.0), 2);
  });

  it("returns -1 for empty samples", () => {
    assert.equal(findTrigger([], 1.0), -1);
  });

  it("returns -1 if no crossing", () => {
    assert.equal(findTrigger([0, 0.5, 0.9], 1.0), -1);
  });

  it("finds trigger when first value is below and next is exactly level", () => {
    assert.equal(findTrigger([0.5, 1.0, 1.5], 1.0), 1);
  });
});

describe("toPixels", () => {
  it("returns empty array for empty samples", () => {
    assert.deepEqual(toPixels([], {}, 100, 100), []);
  });

  it("converts samples to x, y coordinate pairs", () => {
    const samples = [0, 1];
    const settings = { timeDiv: 0.1, voltsDiv: 1, triggerIndex: 0, sampleTime: 0.01 };
    const pts = toPixels(samples, settings, 100, 100);
    assert.deepEqual(pts, [0, 50, 1, 37.5]);
  });
  
  it("applies triggerIndex correctly", () => {
    const samples = [0, 1, 2];
    const settings = { timeDiv: 0.1, voltsDiv: 1, triggerIndex: 1, sampleTime: 0.01 };
    const pts = toPixels(samples, settings, 100, 100);
    assert.deepEqual(pts, [0, 37.5, 1, 25]);
  });

  it("stops processing if x is greater than w", () => {
    const samples = [0, 1, 2];
    const settings = { timeDiv: 0.1, voltsDiv: 1, triggerIndex: 0, sampleTime: 1 };
    const pts = toPixels(samples, settings, 100, 100);
    assert.deepEqual(pts, [0, 50, 100, 37.5]);
  });
});
