# Deconstruct Plan

## crates/simbiguity-physics/src/collide.rs
- `resolve_ground_collision` mixes three concerns: position penetration resolution, normal impulse (bounce), and friction impulse. We can extract:
  - `apply_normal_impulse(...)`
  - `apply_friction_impulse(...)` (which encapsulates the exact v²/2μg fix)
- `find_pairs_broadphase` mixes grid construction and overlap checking. We can split it into:
  - `build_spatial_grid(particles, cell_size)`
  - `find_cell_overlaps(entries, particles)`

## crates/simbiguity-physics/src/gravity_tests.rs
- The crossing-detection logic in `run_orbit` is duplicated in `ellipse_period_and_aphelion` (which also tracks `max_r`). We should generalize `run_orbit` to track and return `max_r` as well, so both tests can use it.
- `ellipse_keplers_second_law` duplicates the area-sweeping loop for perihelion and aphelion. Extract a helper: `measure_swept_area(world, p_id, dt, steps)`.

## crates/simbiguity-physics/src/collide_tests.rs
- `test_ground_friction_slide_case` is already extracted cleanly.
- Minor repeated setup boilerplate (adding a ball to a zero-gravity or standard-gravity world) could be extracted into a test fixture helper, e.g., `setup_collision_world(gravity)`.
