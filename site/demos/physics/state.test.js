import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseState } from './state.js';

test('parses preset and integrator', () => {
    const state = parseState('#preset=spring&integrator=explicit');
    assert.equal(state.preset, 'spring');
    assert.equal(state.integrator, 'explicit');
});
