use simbiguity_circuits::Circuit;
use simbiguity_circuits_view::CircuitView;
use simbiguity_math::DenseMatrix;
use simbiguity_math::Lu;
use std::time::Instant;

pub struct BenchResult {
    pub name: String,
    pub median_us: f64,
    pub p95_us: f64,
}

pub fn run_all() -> Vec<BenchResult> {
    let mut results = Vec::new();

    results.push(bench("hello circuit solve", || {
        let mut c = Circuit::new();
        let plus = c.node();
        let minus = c.node();
        let top = c.node();
        c.add_battery(plus, minus, 12.0);
        c.add_switch(plus, top, true);
        c.add_bulb(top, minus, 12.0, 6.0);
        c.solve();
    }));

    for n in [10, 100, 500] {
        results.push(bench(&format!("{n}-resistor ladder"), || {
            let mut c = Circuit::new();
            let mut top = c.node();
            let mut bot = c.node();
            c.add_battery(top, bot, 9.0);
            for _ in 0..n {
                let next_top = c.node();
                let next_bot = c.node();
                c.add_resistor(top, next_top, 100.0);
                c.add_resistor(next_top, next_bot, 100.0);
                c.add_resistor(next_bot, bot, 100.0);
                top = next_top;
                bot = next_bot;
            }
            c.solve();
        }));
    }

    results.push(bench("10x10 resistor grid", || {
        let mut c = Circuit::new();
        let nodes = vec![vec![c.node(); 10]; 10];
        for i in 0..10 {
            for j in 0..10 {
                if i > 0 {
                    c.add_resistor(nodes[i - 1][j], nodes[i][j], 10.0);
                }
                if j > 0 {
                    c.add_resistor(nodes[i][j - 1], nodes[i][j], 10.0);
                }
            }
        }
        c.add_battery(nodes[0][0], nodes[9][9], 12.0);
        c.solve();
    }));

    results.push(bench("CircuitView advance + encode (600 frames)", || {
        let mut view = CircuitView::new(12.0, true);
        for _ in 0..600 {
            view.advance(1.0 / 60.0);
            view.encode(640, 480);
        }
    }));

    for n in [10, 50, 200] {
        results.push(bench(&format!("Lu::decompose (n={n})"), || {
            let mut m = DenseMatrix::zeros(n, n);
            for i in 0..n {
                m[(i, i)] = 10.0;
                if i > 0 {
                    m[(i, i - 1)] = 1.0;
                }
                if i + 1 < n {
                    m[(i, i + 1)] = 1.0;
                }
            }
            let _ = Lu::decompose(&m);
        }));
    }

    results
}

fn bench(name: &str, mut f: impl FnMut()) -> BenchResult {
    for _ in 0..3 {
        f();
    }
    let mut times = Vec::new();
    let iters = if name.contains("600 frames") { 10 } else { 100 };
    for _ in 0..iters {
        let start = Instant::now();
        f();
        times.push(start.elapsed().as_micros() as f64);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    BenchResult {
        name: name.to_string(),
        median_us: times[times.len() / 2],
        p95_us: times[(times.len() as f64 * 0.95) as usize],
    }
}

pub fn to_json(results: &[BenchResult]) -> String {
    let mut out = String::new();
    out.push_str("[\n");
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!(
            "  {{ \"name\": \"{}\", \"medianUs\": {}, \"p95Us\": {} }}{}\n",
            r.name,
            r.median_us,
            r.p95_us,
            if i + 1 < results.len() { "," } else { "" }
        ));
    }
    out.push_str("]\n");
    out
}
