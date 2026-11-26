#version 300 es
// One small mesh drawn many times: per-instance world offset at location 2.
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec3 a_offset;

uniform mat4 u_viewProj;
uniform float u_scale;

out vec3 v_normal;
out vec3 v_world;

void main() {
  vec3 world = a_position * u_scale + a_offset;
  v_world = world;
  v_normal = a_normal;
  gl_Position = u_viewProj * vec4(world, 1.0);
}
