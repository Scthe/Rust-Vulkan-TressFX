#version 450
precision highp float;
precision highp int;

//@import ../_config_ubo;
//@import ../_utils;
//@import ./_tfx_params_ubo;
//@import ./_tfx_vertex_resolve;

layout(push_constant) uniform Constants {
  mat4 u_MVP;
  mat4 u_modelMat;
  vec4 u_shadowCameraPosition; // [cameraPosition.xyz, u_fiberRadius]
  vec4 u_shadowViewport;
};

void main() {
  TressFXParams tfxParams = createTfxParams();
  tfxParams.eye = u_shadowCameraPosition.xyz;
  tfxParams.modelMat = u_modelMat;
  tfxParams.viewProjMat = u_directionalShadowMatrix_VP;
  tfxParams.viewportSize = u_shadowViewport.xy;
  tfxParams.fiberRadius = u_fiberRadius * u_shadowCameraPosition.w;
  
  TressFXVertex tressfxVert = getExpandedTressFXVert(tfxParams);

  gl_Position = tressfxVert.position;
}