#version 450
precision highp float;
precision highp int;
precision highp usampler2D;
// precision highp sampler2D;


// layout(location = 0) flat in int v_hairInstanceId;
// layout(location = 1) in float v_vertexRootToTipFactor;
// layout(location = 2) in vec3 v_position;
// layout(location = 3) in vec3 v_normal;
// layout(location = 4) in vec3 v_tangent;
// layout(location = 5) in vec4 v_positionLightShadowSpace;

layout(location = 0) out vec4 outColor1;
layout(location = 1) out vec4 outColor2;


void main() {
  vec3 result = vec3(0.8);

  outColor1 = vec4(result, 1.0);
  outColor2 = vec4(0.0, 1.0, 0.0, 1.0);
}