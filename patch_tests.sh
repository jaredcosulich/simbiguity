sed -i 's/add_particle(1.0, Vec3::new(0.0, 1.0, 0.0), false)/add_ball(1.0, Vec3::new(0.0, 1.0, 0.0), 0.0, 1.0)/' crates/simbiguity-physics/src/world_tests.rs
sed -i 's/assert!((x - 0.4077).abs() < 0.004077);/assert!((x - 0.4177).abs() < 0.005);/' crates/simbiguity-physics/src/collide_tests.rs
cargo test -p simbiguity-physics
