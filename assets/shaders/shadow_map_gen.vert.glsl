#version 450
precision highp float;
precision highp int;

layout(push_constant) uniform Constants {
  mat4 u_MVP;
};

layout(location = 0) in vec3 in_Position;
layout(location = 1) in vec3 in_Normal;
layout(location = 2) in vec2 in_UV;

void main() {
  vec4 pos = vec4(in_Position.xyz, 1.0);
  /// Values are negative ([-near, -far]) due to Vulkan coordinate system
  gl_Position = u_MVP * pos;
}