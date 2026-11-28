use simbiguity_circuits::Circuit;
use simbiguity_circuits_view::CircuitView;
use simbiguity_math::DenseMatrix;
use simbiguity_math::Lu;
use simbiguity_physics::World;
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
        let mut nodes = Vec::new();
        for _ in 0..10 {
            let mut row = Vec::new();
            for _ in 0..10 {
                row.push(c.node());
            }
            nodes.push(row);
        }
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

    results.push(bench("ball_pit_1000_one_second", || {
        let mut world = World::new();
        world.setup_deterministic_ball_pit(1000);
        for _ in 0..60 {
            // 60 steps for 1 second at 60Hz
            world.step(1.0 / 60.0).unwrap();
        }
    }));

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
    let (median_us, p95_us) = compute_stats(times);
    BenchResult {
        name: name.to_string(),
        median_us,
        p95_us,
    }
}

pub fn compute_stats(mut times: Vec<f64>) -> (f64, f64) {
    if times.is_empty() {
        return (0.0, 0.0);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[times.len() / 2];
    let p95 = times[(times.len() as f64 * 0.95) as usize];
    (median, p95)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// JSON representation has the required fields.
    #[test]
    fn to_json_shape() {
        let results = vec![
            BenchResult {
                name: "A".to_string(),
                median_us: 10.0,
                p95_us: 20.0,
            },
            BenchResult {
                name: "B".to_string(),
                median_us: 100.0,
                p95_us: 200.0,
            },
        ];
        let json = to_json(&results);
        assert!(json.contains(r#"  { "name": "A", "medianUs": 10, "p95Us": 20 },"#));
        assert!(json.contains(r#"  { "name": "B", "medianUs": 100, "p95Us": 200 }"#));
        assert!(json.starts_with("[\n"));
        assert!(json.ends_with("]\n"));
    }

    /// Median and p95 compute the correct indices.
    #[test]
    fn stats_calculations() {
        let times: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let (median, p95) = compute_stats(times);
        assert_eq!(median, 51.0); // times[50] is 51
        assert_eq!(p95, 96.0); // times[95] is 96

        assert_eq!(compute_stats(vec![]), (0.0, 0.0));
    }
}
