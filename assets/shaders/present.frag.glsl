#version 450

precision highp float;
precision highp int;
precision highp usampler2D;

//@import _config_ubo;
//@import _utils;
//@import _fxaa;

layout(push_constant) uniform Constants {
	int u_displayMode;
};


layout(binding = 1)
uniform sampler2D u_tonemappedTex;
layout(binding = 2)
uniform usampler2D u_normalsTex;
// layout(binding = 3)
// uniform sampler2D u_linearDepthTex;
// layout(binding = 4)
// uniform sampler2D u_ssaoTex;

const int DISPLAY_MODE_FINAL = 0;
const int DISPLAY_MODE_NORMALS = 1;
// const int DISPLAY_MODE_LINEAR_DEPTH = 2;
// const int DISPLAY_MODE_SSAO = 3;

layout(location = 0) in vec2 v_position;
layout(location = 0) out vec4 color1;


vec3 doFxaa (vec2 uv) {
  vec4 color;

  if (u_edgeThreshold == 0.0) {
    color = texture(u_tonemappedTex, uv);
  } else {
    color = FxaaPixelShader(
      uv, // in [0-1]
      u_tonemappedTex,
      u_tonemappedTex,
      vec2(1.0) / u_viewport,
      u_subpixel,
      u_edgeThreshold,
      u_edgeThresholdMin
    );
  }

  return color.rgb;
}


// Gamma not needed as swapchain image is in SRGB
void main() {
  vec3 result;

  switch(u_displayMode) {
    case DISPLAY_MODE_NORMALS: {
      // v_position as `readModelTexture_uint` already does `fixOpenGLTextureCoords_AxisY`
      vec3 normal = unpackNormal(u_normalsTex, v_position).xyz;
      result = abs(normal);
      break;
    }
    /*
    case DISPLAY_MODE_LINEAR_DEPTH: {
      float depth = texture(u_linearDepthTex, uv).r;
      float d = u_nearAndFar.y - u_nearAndFar.x;
      result = vec3(depth / d);
      break;
    }


    case DISPLAY_MODE_SSAO: {
      float ssao = texture(u_ssaoTex, uv).r;
      result = vec3(ssao);
      break;
    }
    */

    default:
    case DISPLAY_MODE_FINAL: {
      vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
      result = doFxaa(uv);
      break;
    }
  }
  
  color1 = vec4(result, 1.0f);
}