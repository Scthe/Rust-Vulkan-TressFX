#version 450
precision highp float;
precision highp int;
precision highp usampler2D;
// precision highp sampler2D;


layout(binding = 4) uniform sampler2D u_directionalShadowDepthTex;
layout(binding = 5) uniform sampler2D u_aoTex;


layout(location = 0) flat in int v_hairInstanceId;
layout(location = 1) in float v_vertexRootToTipFactor;
layout(location = 2) in vec3 v_position;
layout(location = 3) in vec3 v_normal;
layout(location = 4) in vec3 v_tangent;
layout(location = 5) in vec4 v_positionLightShadowSpace;

layout(location = 0) out vec4 outColor1;
layout(location = 1) out vec4 outColor2;


//@import ./_config_ubo;
//@import ./_tressFXParams_ubo;
//@import ./_utils;
//@import ./_material; // for light struct
//@import ./_shadows;
//@import ./_kajiyakay;

// const int TFX_DISPLAY_MODE_FINAL = 0;
const int TFX_DISPLAY_MODE_FLAT = 1;
const int TFX_DISPLAY_MODE_FOLLOW_GROUPS = 2;
const int TFX_DISPLAY_MODE_ROOT_TIP_PERCENTAGE = 3;
// const int TFX_DISPLAY_MODE_SHADOW = 4;


vec3 getColorFromInstance (int instanceId) {
  switch (instanceId) {
    case 1: return vec3(0.0, 1.0, 0.0);
    case 2: return vec3(0.0, 0.0, 1.0);
    case 3: return vec3(1.0, 1.0, 0.0);
    case 4: return vec3(0.0, 1.0, 1.0);
    case 5: return vec3(1.0, 0.0, 1.0);
    case 6: return vec3(1.0, 1.0, 1.0);
    case 7: return vec3(0.0, 0.0, 0.0);
    case 8: return vec3(0.0, 0.5, 0.0);
    case 9: return vec3(0.5, 0.5, 0.5);
    case 10: return vec3(0.0, 0.0, 0.5);
    case 11: return vec3(0.5, 0.5, 0.0);
    case 12: return vec3(0.0, 0.5, 0.5);
    case 13: return vec3(0.5, 0.0, 0.5);

    default:
    case 0: return vec3(1.0, 0.0, 0.0);
  }
}



void main() {
  vec3 result = vec3(0.8);

  switch (u_tfxDisplayMode) {
    case TFX_DISPLAY_MODE_FOLLOW_GROUPS: {
      result = getColorFromInstance(v_hairInstanceId);
      break;
    }

    case TFX_DISPLAY_MODE_ROOT_TIP_PERCENTAGE: {
      result = vec3(v_vertexRootToTipFactor);
      // result = mix(vec3(0.0, 1.0, 1.0), vec3(0.0, 1.0, 0.0), v_vertexRootToTipFactor);
      // result += getColorFromInstance(v_hairInstanceId);
      break;
    }

    default: // TODO remove default
    case TFX_DISPLAY_MODE_FLAT: {
      result = vec3(0.8);
      break;
    }
    /*
    case TFX_DISPLAY_MODE_SHADOW: {
      float shadow = calculateShadow();
      result = vec3(shadow);
      break;
    }

    default:
    case TFX_DISPLAY_MODE_FINAL: {
      result = doShading(lights);
      break;
    }
    */
  }

  outColor1 = vec4(result, 1.0);
  outColor2 = uvec4(packNormal(v_normal), 255);
}