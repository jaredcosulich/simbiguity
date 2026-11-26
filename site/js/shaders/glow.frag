#version 300 es
// An additive halo: strongest at the centre of the sphere as seen from the
// camera, fading to nothing at its silhouette.
precision highp float;

uniform vec4 u_color;
uniform vec4 u_eye;
uniform float u_emissive;

in vec3 v_world;
in vec3 v_normal;

out vec4 o_color;

void main() {
  vec3 n = normalize(v_normal);
  vec3 v = normalize(u_eye.xyz - v_world);
  float facing = max(dot(n, v), 0.0);
  float halo = pow(facing, 3.0) * u_emissive;
  o_color = vec4(u_color.rgb * halo, halo);
}
