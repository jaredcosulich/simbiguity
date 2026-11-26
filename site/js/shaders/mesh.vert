#version 300 es
// Shared vertex shader for the unlit, lit and glow programs.
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;

uniform mat4 u_viewProj;
uniform mat4 u_model;

out vec3 v_world;
out vec3 v_normal;

void main() {
  vec4 world = u_model * vec4(a_position, 1.0);
  v_world = world.xyz;
  // Inverse-transpose keeps normals perpendicular under non-uniform scale.
  v_normal = transpose(inverse(mat3(u_model))) * a_normal;
  gl_Position = u_viewProj * world;
}
