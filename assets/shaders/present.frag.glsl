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
uniform sampler2D u_tonemappedTex; // TODO usampler2D
layout(binding = 2)
uniform usampler2D u_normalsTex;
layout(binding = 3)
uniform sampler2D u_ssaoTex;
layout(binding = 4)
uniform sampler2D u_linearDepthTex;

const int DISPLAY_MODE_FINAL = 0;
const int DISPLAY_MODE_NORMALS = 1;
const int DISPLAY_MODE_LUMA = 2;
const int DISPLAY_MODE_SSAO = 3;
const int DISPLAY_MODE_LINEAR_DEPTH = 4;

layout(location = 0) in vec2 v_position;
layout(location = 0) out vec4 color1;

vec3 doFxaa (vec2 uv) {
  vec4 color;

  if (u_edgeThreshold == 0.0) {
    // color = vec4(readModelTexture_uint(u_tonemappedTex, uv), 1.0);
    color = vec4(texture(u_tonemappedTex, uv).rgb, 1.0);
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
    
    case DISPLAY_MODE_LUMA: {
      vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
      float luma = texture(u_tonemappedTex, uv).a;
      result = vec3(luma, luma, luma);
      break;
    }

    case DISPLAY_MODE_SSAO: {
      vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
      uv = v_position;
      float ssao = texture(u_ssaoTex, uv).r;
      result = vec3(ssao, ssao, ssao);
      break;
    }
    
    case DISPLAY_MODE_LINEAR_DEPTH: {
      vec2 uv = v_position;
      float depth = -texture(u_linearDepthTex, uv).r; // value is [0.1..100]
      vec2 nearAndFarPreview = -u_linear_depth_preview_range; // value is e.g. [5, 10]
      float d = nearAndFarPreview.y - nearAndFarPreview.x; // value for [5, 10] is 5
      float val = (depth - nearAndFarPreview.x) / d;
      result = vec3(saturate(val));
      break;
    }

    default:
    case DISPLAY_MODE_FINAL: {
      vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
      result = doFxaa(uv);
      break;
    }
  }
  
  color1 = vec4(result, 1.0f);
}