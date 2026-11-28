import { test, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";

test("demo.js history.replaceState hash write", async () => {
    const elements = {};
    global.document = {
        getElementById: (id) => {
            if (!elements[id]) {
                elements[id] = {
                    getContext: () => ({
                        enable: () => {}, cullFace: () => {}, createShader: () => ({}),
                        shaderSource: () => {}, compileShader: () => {},
                        getShaderParameter: () => true, createProgram: () => ({}),
                        attachShader: () => {}, linkProgram: () => {},
                        getProgramParameter: () => true, getUniformLocation: () => ({}),
                        createVertexArray: () => {}, bindVertexArray: () => {},
                        createBuffer: () => {}, bindBuffer: () => {}, bufferData: () => {},
                        enableVertexAttribArray: () => {}, vertexAttribPointer: () => {},
                        disableVertexAttribArray: () => {}, vertexAttribDivisor: () => {},
                        drawElementsInstanced: () => {}, drawElements: () => {},
                        clearColor: () => {}, clear: () => {}, viewport: () => {},
                        useProgram: () => {}, uniformMatrix4fv: () => {},
                        uniform4fv: () => {}, uniform1f: () => {},
                        disable: () => {}, blendFunc: () => {}, depthMask: () => {}
                    }),
                    clientWidth: 100, clientHeight: 100, value: '',
                    style: {}, addEventListener: () => {}, replaceChildren: () => {},
                    appendChild: () => {}
                };
            }
            return elements[id];
        },
        createElement: () => ({ style: {}, getContext: () => ({}) }),
        body: { dataset: {} }
    };
    global.window = { devicePixelRatio: 1, addEventListener: () => {} };
    global.location = { hash: '#closed&v=10', pathname: '/demo', search: '' };
    global.history = { replaceState: mock.fn() };
    global.performance = { now: () => 0 };
    global.requestAnimationFrame = () => {};
    global.ResizeObserver = class { observe() {} };
    
    const originalFetch = global.fetch;
    global.fetch = async (url) => {
        const path = new URL(url).pathname;
        if (path.endsWith(".wasm")) {
            return new Response(fs.readFileSync(path), { headers: { "content-type": "application/wasm" } });
        }
        return new Response(fs.readFileSync(path, "utf-8"));
    };

    try {
        await import("./demo.js");
        // Wait for main() async calls to resolve
        await new Promise(r => setTimeout(r, 100));
        
        elements["toggle"].onclick();
        
        assert.strictEqual(global.history.replaceState.mock.callCount(), 1, "history.replaceState was called");
        const call = global.history.replaceState.mock.calls[0];
        assert.strictEqual(call.arguments[2], "/demo#v=10");
    } finally {
        global.fetch = originalFetch;
    }
});
