#version 450
precision highp float;
precision highp int;


layout(location = 0) flat out int v_hairInstanceId;
layout(location = 1) out float v_vertexRootToTipFactor;
layout(location = 2) out vec3 v_position;
layout(location = 3) out vec3 v_normal;
layout(location = 4) out vec3 v_tangent;
layout(location = 5) out vec4 v_positionLightShadowSpace;


//@import ../_config_ubo;
//@import ../_utils;
//@import ./_tfx_params_ubo;
//@import ./_tfx_vertex_resolve;



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