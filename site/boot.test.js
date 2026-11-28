const { test } = require('node:test');
const assert = require('node:assert');
const { boot } = require('./js/boot.js');
// Shows message when WebGL2 is missing

test('boot.js shows message when WebGL2 is missing', async () => {
    let replacedWith = null;
    let prependedWith = null;
    let getContextCalled = false;
    
    global.document = {
        createElement: (tag) => {
            if (tag === "canvas") {
                return {
                    getContext: (type) => {
                        getContextCalled = true;
                        return type === 'webgl2' ? null : {};
                    }
                };
            }
            return {
                style: {},
                innerHTML: ''
            };
        },
        querySelector: (sel) => {
            if (sel === 'canvas') return null;
        },
        body: {
            prepend: (el) => { prependedWith = el; }
        }
    };
    
    let mainCalled = false;
    await boot(async () => {
        mainCalled = true;
    });
    
    assert.strictEqual(mainCalled, false, 'mainFn should not be called if WebGL2 is missing');
    assert.ok(getContextCalled, 'canvas.getContext should be called');
    assert.ok(prependedWith, 'Error div should be prepended');
    assert.match(prependedWith.innerHTML, /Your browser doesn't support WebGL2\./);
// Renders error message on rejected promise
    
    delete global.document;
});

test('boot.js renders error message on rejected promise', async () => {
    let prependedWith = null;
    
    global.document = {
        createElement: (tag) => {
            if (tag === "canvas") {
                return {
                    getContext: (type) => {
                        return type === 'webgl2' ? {} : null;
                    }
                };
            }
            return {
                style: {},
                innerHTML: ''
            };
        },
        querySelector: (sel) => {
            if (sel === 'canvas') return null;
        },
        body: {
            prepend: (el) => { prependedWith = el; }
        }
    };
    
    // Stub WebAssembly
    global.WebAssembly = {};
    
    let mainCalled = false;
    await boot(async () => {
        mainCalled = true;
        throw new Error("test rejection message");
    });
    
    assert.strictEqual(mainCalled, true, 'mainFn should be called');
    assert.ok(prependedWith, 'Error div should be prepended');
    assert.match(prependedWith.innerHTML, /test rejection message/);
    
    delete global.document;
    delete global.WebAssembly;
});
