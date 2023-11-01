#version 450

precision highp float;
precision highp int;
precision highp sampler2D;

layout(push_constant) uniform Constants {
  // Direction of the blur:
  //   - First pass:   float2(1.0, 0.0)
  //   - Second pass:  float2(0.0, 1.0)
	vec4 u_sssDirection;
};

layout(binding = 1)
uniform sampler2D u_sourceTex;
layout(binding = 2)
uniform sampler2D u_linearDepthTex;

layout(location = 0) in vec2 v_position;
layout(location = 0) out vec4 outColor1;


//@import ./_config_ubo;
//@import ./_utils;


float SSSSS_sampleDepthLinear (sampler2D depthTex, vec2 texcoord) {
  float depthWorldUnits = texture(u_linearDepthTex, texcoord).r;
  return -depthWorldUnits; // to positive values
}

#define SSSS_GLSL_3 1
//@import ./_separableSSSSS;


void main() {
  // vec2 posTextureSpace = v_position;
  vec2 posTextureSpace = fixOpenGLTextureCoords_AxisY(v_position);

  vec4 result = SSSSBlurPS(
    posTextureSpace, // float2 texcoord,
    u_sourceTex, // SSSSTexture2D colorTex,
    u_linearDepthTex, // SSSSTexture2D depthTex,
    u_sssBlurWidth, // float sssWidth,
    u_sssDirection.xy, // float2 dir
    u_sssBlurFovy, u_sssBlurStrength, 
    u_sssBlurFollowSurface != 0 // replaced macros
  );

  outColor1 = vec4(result.rgb, 1.0);
}