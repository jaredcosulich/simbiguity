import { loadCircuits } from "../../js/simbiguity.js";

async function main() {
    const { Circuit } = await loadCircuits(new URL("../../pkg/simbiguity_circuits_wasm.wasm", import.meta.url));
    const c = new Circuit();

    const n0 = c.node();
    const n1 = c.node();
    const n2 = c.node();
    const n3 = c.node();

    const battery = c.battery(n1, n0, 10.0);
    const sw = c.switch(n1, n2, true);
    const res = c.resistor(n2, n3, 100.0);
    const cap = c.capacitor(n3, n0, 0.1);
    
    let dt = 0.016;
    let history = [];

    const updateReadouts = () => {
        const { parts } = c.step(dt);
        if (!parts) return;
        const vcap = parts[cap * 4];
        const icap = parts[cap * 4 + 1];
        document.getElementById('vcap').textContent = (vcap || 0).toFixed(2);
        document.getElementById('icap').textContent = (icap || 0).toFixed(3);
        
        let r = parseFloat(document.getElementById('resistor').value);
        let capVal = parseFloat(document.getElementById('capacitor').value);
        document.getElementById('tau').textContent = (r * capVal).toFixed(3);
        
        history.push(vcap || 0);
        if (history.length > 200) history.shift();
        drawChart();
    };

    const drawChart = () => {
        const canvas = document.getElementById('chart');
        const ctx = canvas.getContext('2d');
        canvas.width = canvas.clientWidth;
        canvas.height = canvas.clientHeight;
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.beginPath();
        for (let i = 0; i < history.length; i++) {
            const x = (i / 200) * canvas.width;
            const y = canvas.height - (history[i] / 12) * canvas.height;
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.stroke();
    };

    document.getElementById('resistor').oninput = e => {
        c.setValue(res, parseFloat(e.target.value));
    };
    document.getElementById('capacitor').oninput = e => {
        c.setValue(cap, parseFloat(e.target.value));
    };
    document.getElementById('toggle').onclick = () => {
        c.toggle(sw);
    };

    setInterval(updateReadouts, 16);
}

main();