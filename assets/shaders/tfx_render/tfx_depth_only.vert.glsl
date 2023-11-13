#version 450
precision highp float;
precision highp int;

#pragma include ../_config_ubo;
#pragma include ../_utils;
#pragma include ./_tfx_params_ubo;
#pragma include ./_tfx_vertex_resolve;


void main() {
  TressFXParams tfxParams = createTfxParams();
  TressFXVertex tressfxVert = getExpandedTressFXVert(tfxParams);
  gl_Position = tressfxVert.position;
}
