#version 450

precision highp float;
precision highp int;
precision highp sampler2D;

// depth in perspective projection, will be converted to linear later
layout(binding = 1)
uniform sampler2D u_depthBufferTex;


layout(location = 0) in vec2 v_position; // TexCoords
// NOTE: this will be in [zNear...zFar], not [0..1] !!!
layout(location = 0) out vec4 outDepth;

//@import ./_config_ubo;
//@import ./_utils;


void main() {
  float depth = texture(u_depthBufferTex, v_position).r;
  // TODO? or inverse_projection? Then remove from `_utils.glsl`.
  // float linearDepth = linearizeDepth(depth, u_nearAndFar);
  vec4 clipSpace = vec4(to_neg1_1(v_position), depth, 1);
  vec4 viewPos = u_invProjectionMat * clipSpace;
  viewPos.xyz /= viewPos.w;
  /// Values are negative due to Vulkan coordinate system
  outDepth = vec4(vec3(viewPos.z), 1.0);
}