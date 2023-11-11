#version 450
precision highp float;
precision highp int;

#pragma include ../_config_ubo;
#pragma include ../_utils;
#pragma include ./_tfx_params_ubo;
#pragma include ./_tfx_vertex_resolve;


layout(location = 0) flat out int v_hairInstanceId;
layout(location = 1) out float v_vertexRootToTipFactor;
layout(location = 2) out vec3 v_position;
layout(location = 3) out vec3 v_normal;
layout(location = 4) out vec3 v_tangent;
layout(location = 5) out vec4 v_positionLightShadowSpace;

// Funny this shader, is same as 'tfx_forward.vert.glsl'
void main() {
  TressFXParams tfxParams = createTfxParams();
  TressFXVertex tressfxVert = getExpandedTressFXVert(tfxParams);

  gl_Position = tressfxVert.position;

  v_hairInstanceId = gl_InstanceIndex;
  v_vertexRootToTipFactor = tressfxVert.vertexRootToTipFactor;
  v_positionLightShadowSpace = u_directionalShadowMatrix_VP * tressfxVert.positionWorldSpace;
  v_position = tressfxVert.positionWorldSpace.xyz;
  v_normal = tressfxVert.normal;
  v_tangent = tressfxVert.tangent;
}
