#version 450

// must match fragment shader
#define PPLL_HEAD_POINTERS_IMAGE_BINDING 4
#define PPLL_DATA_BUFFER_BINDING 5

#pragma include ../_config_ubo;
#pragma include ../_utils;
#pragma include ./_tfx_params_ubo;
#pragma include ./_tfx_vertex_resolve;


layout(location = 0) out vec4 v_position; // position world space
layout(location = 1) out vec4 v_tangent;
layout(location = 2) out vec4 v_strandColor;
// layout(location = 3) out vec4 v_p0p1;

void main(void) {
  TressFXParams tfxParams = createTfxParams();
  TressFXVertex tressfxVert = getExpandedTressFXVert(tfxParams);

  gl_Position = tressfxVert.position;
  v_position = tressfxVert.position;
  v_tangent  = vec4(tressfxVert.tangent, 1);
  // v_p0p1     = tressfxVert.p0p1;
  // v_strandColor = tressfxVert.strandColor;
  v_strandColor = vec4(1,0,0, 0.5); // TODO [HIGH] hardcoded half-transparent red
}
