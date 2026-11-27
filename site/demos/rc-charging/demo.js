import { loadCircuits } from "../../js/simbiguity.js";
import { renderLabels } from "../../js/plot-labels.js";
import { WebGL2Executor } from "../../js/webgl2-executor.js";

async function main() {
    const { Circuit, exports: w } = await loadCircuits(new URL("../../pkg/simbiguity_circuits_wasm.wasm", import.meta.url));
    
    // We need WebGL2 Executor for meshes
    const plotContainer = document.getElementById("plot-container");
    const canvas = document.createElement("canvas");
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    plotContainer.appendChild(canvas);
    
    let executor;
    try {
        const gl = canvas.getContext('webgl2');
        executor = await WebGL2Executor.create(gl);;
    } catch (e) {
        console.error("WebGL2 error:", e);
        return;
    }

    let c;
    let cap, res, ind, sw, bat, bulb;

    const setupPreset = (isLC) => {
        if (c) c.free();
        c = new Circuit();
        const n0 = c.node();
        const n1 = c.node();
        const n2 = c.node();
        const n3 = c.node();

        bat = c.battery(n1, n0, 10.0);
        sw = c.switch(n1, n2, true);
        res = c.resistor(n2, n3, parseFloat(document.getElementById('resistor').value));
        cap = c.capacitor(n3, n0, parseFloat(document.getElementById('capacitor').value));
        
        if (isLC) {
            ind = c.inductor(n2, n0, 1.0);
            c.setValue(res, 0.001); // minimal resistance
        } else {
            bulb = c.bulb(n3, n0, 6.0, 3.0);
        }
        
        let method = parseInt(document.getElementById('integrator').value);
        c.setIntegrator(method);
    };

    setupPreset(false);

    let dt = 0.016;

    const updateReadouts = () => {
        const { parts } = c.step(dt);
        if (!parts) return;
        const vcap = parts[cap * 4];
        const icap = parts[cap * 4 + 1];
        
        // Push plot point
        c.plotPush(vcap || 0);

        document.getElementById('vcap').textContent = (vcap || 0).toFixed(2);
        document.getElementById('icap').textContent = (icap || 0).toFixed(3);
        
        let r = parseFloat(document.getElementById('resistor').value);
        let capVal = parseFloat(document.getElementById('capacitor').value);
        document.getElementById('tau').textContent = (r * capVal).toFixed(3);
        
        // Draw Plot
        const width = plotContainer.clientWidth;
        const height = plotContainer.clientHeight;
        if (executor && width > 0 && height > 0) {
            canvas.width = width * window.devicePixelRatio;
            canvas.height = height * window.devicePixelRatio;
            
            c.plotDraw(width, height);
            
            const meshCount = w.circuit_plot_mesh_count(c.handle);
            for (let i = 0; i < meshCount; i++) {
                const vptr = w.circuit_plot_mesh_vertices_ptr(c.handle, i);
                const vlen = w.circuit_plot_mesh_vertices_len(c.handle, i);
                const iptr = w.circuit_plot_mesh_indices_ptr(c.handle, i);
                const ilen = w.circuit_plot_mesh_indices_len(c.handle, i);
                
                const vertices = new Float32Array(w.memory.buffer, vptr, vlen);
                const indices = new Uint32Array(w.memory.buffer, iptr, ilen);
                executor.uploadMesh(i, vertices, indices);
            }
            
            const cptr = w.circuit_plot_commands_ptr(c.handle);
            const clen = w.circuit_plot_commands_len(c.handle);
            const commands = new Uint32Array(w.memory.buffer, cptr, clen);
            executor.execute(commands);
            
            const lptr = w.circuit_plot_labels_ptr(c.handle);
            const llen = w.circuit_plot_labels_len(c.handle);
            const utf8 = new TextDecoder();
            const labelsStr = utf8.decode(new Uint8Array(w.memory.buffer, lptr, llen));
            let labels = [];
            try { labels = JSON.parse(labelsStr); } catch (e) {}
            renderLabels(plotContainer, labels);
        }
    };

    document.getElementById('preset').onchange = e => {
        setupPreset(e.target.value === 'lc');
    };
    document.getElementById('integrator').onchange = e => {
        c.setIntegrator(parseInt(e.target.value));
    };
    document.getElementById('resistor').oninput = e => {
        if (res !== undefined) c.setValue(res, parseFloat(e.target.value));
    };
    document.getElementById('capacitor').oninput = e => {
        c.setValue(cap, parseFloat(e.target.value));
    };
    document.getElementById('toggle').onclick = () => {
        c.toggle(sw);
    };
    document.getElementById('reset').onclick = () => {
        setupPreset(document.getElementById('preset').value === 'lc');
    };

    setInterval(updateReadouts, 16);
}

main();
