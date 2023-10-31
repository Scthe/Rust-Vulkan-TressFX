#version 450

precision highp float;
precision highp int;
precision highp sampler2D;

/*
uniform float u_sssWidth;
// Direction of the blur:
//   - First pass:   float2(1.0, 0.0)
//   - Second pass:  float2(0.0, 1.0)
uniform vec2 u_sssDirection;
// replaced macros:
uniform float u_sssStrength; // SSSS_STREGTH_SOURCE
uniform int u_sssFollowSurface; // SSSS_FOLLOW_SURFACE: 0 or 1
uniform float u_sssFovy; // SSSS_FOVY 20.0
*/

layout(binding = 1)
uniform sampler2D u_sourceTex;
layout(binding = 2)
uniform sampler2D u_linearDepthTex;

layout(location = 0) in vec2 v_position;
layout(location = 0) out vec4 outColor1;


//@import ./_config_ubo;
//@import ./_utils;
#define SSSS_GLSL_3 1
//@i mport ./_separableSSSSS;

float SSSSS_sampleDepthLinear (sampler2D depthTex, vec2 texcoord) {
  return texture(u_linearDepthTex, texcoord).r;
}


void main() {
  /*
  // vec2 posTextureSpace = v_position;
  vec2 posTextureSpace = fixOpenGLTextureCoords_AxisY(v_position);

  vec4 result = SSSSBlurPS(
    posTextureSpace, // float2 texcoord,
    u_sourceTex, // SSSSTexture2D colorTex,
    u_linearDepthTex, // SSSSTexture2D depthTex,
    u_sssWidth, // float sssWidth,
    u_sssDirection, // float2 dir
    u_sssFovy, u_sssStrength, u_sssFollowSurface != 0 // replaced macros
  );

  outColor1 = vec4(result.rgb, 1.0);
  */
  vec2 posTextureSpace = fixOpenGLTextureCoords_AxisY(v_position);
  vec3 result = texture(u_sourceTex, posTextureSpace).rgb;
  outColor1 = vec4(result, 1.0);
}