#version 300 es
// Lambert diffuse + Blinn-Phong highlight + emissive self-light.
precision highp float;

uniform vec4 u_color;        // rgb + alpha
uniform vec4 u_lightDir;     // toward the light
uniform vec4 u_lightColor;   // rgb + ambient fraction in w
uniform vec4 u_eye;
uniform float u_emissive;

in vec3 v_world;
in vec3 v_normal;

out vec4 o_color;

void main() {
  vec3 n = normalize(v_normal);
  vec3 l = normalize(u_lightDir.xyz);
  vec3 v = normalize(u_eye.xyz - v_world);
  vec3 h = normalize(l + v);
  float diffuse = max(dot(n, l), 0.0);
  float spec = diffuse > 0.0 ? pow(max(dot(n, h), 0.0), 48.0) : 0.0;
  // A little sky-from-above fill so shadowed faces are not flat black.
  float fill = 0.5 + 0.5 * n.y;
  vec3 base = u_color.rgb;
  vec3 lit = base * (u_lightColor.w * (0.6 + 0.4 * fill) + diffuse * u_lightColor.rgb)
    + spec * 0.35 * u_lightColor.rgb
    + base * u_emissive;
  o_color = vec4(lit, u_color.a);
}
