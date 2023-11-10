#version 450
precision highp float;
precision highp int;


#pragma include ./_config_ubo;
#pragma include ./_forward_model_ubo;


// https://www.khronos.org/opengl/wiki/Layout_Qualifier_(GLSL)
layout(location = 0) in vec3 in_Position;
layout(location = 1) in vec3 in_Normal;
layout(location = 2) in vec2 in_UV;

layout(location = 0) out vec3 v_Position; // global-space
layout(location = 1) out vec3 v_Normal;
layout(location = 2) out vec2 v_UV;
layout(location = 3) out vec4 v_PositionLightShadowSpace;


void main() {
  vec4 pos = vec4(in_Position.xyz, 1.0);
  gl_Position = u_MVP * pos;
  v_Position = (u_M * pos).xyz;
  v_PositionLightShadowSpace = u_directionalShadowMatrix_MVP * pos;
  v_Normal = in_Normal; // TODO technically we should have rotation matrix here, but not needed for app as simple as this
  v_UV = in_UV;
}