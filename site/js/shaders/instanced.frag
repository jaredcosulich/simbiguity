#version 300 es
// Self-lit particles: a bright core fading toward the rim (drawn additively).
precision highp float;

uniform vec4 u_color;
uniform vec4 u_eye;
uniform float u_emissive;

in vec3 v_normal;
in vec3 v_world;

out vec4 o_color;

void main() {
  vec3 n = normalize(v_normal);
  vec3 v = normalize(u_eye.xyz - v_world);
  float core = 0.35 + 0.65 * pow(max(dot(n, v), 0.0), 1.5);
  o_color = vec4(u_color.rgb * core * u_emissive, 1.0);
}
