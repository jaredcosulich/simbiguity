sed -i 's/-> Vec<f64>/-> (Vec<f64>, f64)/' crates/simbiguity-plot/src/ticks.rs
sed -i 's/ticks$/(ticks, nice_step)/' crates/simbiguity-plot/src/ticks.rs
sed -i 's/let ticks = NiceTicks::compute(0.0, 10.0, 5);/let (ticks, _) = NiceTicks::compute(0.0, 10.0, 5);/' crates/simbiguity-plot/src/ticks.rs
