with open("crates/simbiguity-physics/src/collide_tests.rs", "r") as f:
    lines = f.readlines()

def add_comment(test_name, comment):
    for i, line in enumerate(lines):
        if line.strip().startswith(f"fn {test_name}()"):
            # Check if there's already a #[test] above it, which there is.
            # We want to put the comment ABOVE the #[test] attribute.
            j = i - 1
            while j >= 0 and lines[j].strip() == "":
                j -= 1
            if lines[j].strip() == "#[test]":
                lines.insert(j, f"    /// {comment}\n")
            else:
                lines.insert(i, f"    /// {comment}\n")
            break

add_comment("elastic_collision_one_kg_into_three_kg_bounces_back_at_minus_one", "Verifies that an elastic collision between different masses results in correct post-collision velocities.")
add_comment("perfectly_inelastic_collision", "Verifies that a perfectly inelastic collision results in both objects sticking together with zero restitution.")
add_comment("equal_masses_elastic_head_on", "Verifies that two equal masses with an elastic head-on collision swap their velocities.")
add_comment("newtons_cradle", "Verifies Newton's cradle momentum transfer behavior across multiple aligned balls.")
add_comment("bounce_drop", "Verifies that a ball dropped on a restitution 0.5 ground bounces to exactly the expected height.")
add_comment("ground_friction_slide", "Verifies that kinematic friction applied to a sliding ball reduces its velocity correctly over time.")
add_comment("balls_in_box_overlap_and_momentum", "Verifies momentum conservation for a large number of randomly overlapping balls inside a box.")
add_comment("broadphase_equivalence", "Verifies that the broadphase collision pair generation matches the brute-force O(N^2) pair generation exactly.")

with open("crates/simbiguity-physics/src/collide_tests.rs", "w") as f:
    f.writelines(lines)
