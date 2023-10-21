#version 450
precision highp float;
precision highp int;


//@import ./_config_ubo;
//@import ./_forward_model_ubo;
//@i mport ./_forward_model_per_frame_ubo;


// https://www.khronos.org/opengl/wiki/Layout_Qualifier_(GLSL)
layout(location = 0) in vec3 in_Position;
layout(location = 1) in vec3 in_Normal;
layout(location = 2) in vec2 in_UV;

layout(location = 0) out vec3 v_Position; // global-space
layout(location = 1) out vec3 v_Normal;
layout(location = 2) out vec2 v_UV;
// layout(location = 3) out vec4 v_PositionLightShadowSpace;


void main() {
  vec4 pos = vec4(in_Position.xyz * 0.3f, 1.0); // magic scale TODO
  gl_Position = u_MVP * pos;
  v_Position = (u_M * pos).xyz;
  // v_PositionLightShadowSpace = u_directionalShadowMatrix_MVP * pos; // TODO
  v_Normal = in_Normal; // TODO technically we should have rotation matrix here, but not needed for app as simple as this
  // v_Normal = vec3(-0.5,1.0,-0.5);
  // v_Normal = vec3(0.0, 1.0, 0.0);
  v_UV = in_UV;
}