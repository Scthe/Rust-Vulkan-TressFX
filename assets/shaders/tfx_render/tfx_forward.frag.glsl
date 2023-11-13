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
layout(location = 6) flat in uint v_strandId;

layout(location = 0) out vec4 outColor1;
layout(location = 1) out uvec4 outColor2;


#pragma include ../_config_ubo;
#pragma include ../_utils;
#pragma include ./_tfx_params_ubo;
#pragma include ../materials/_hair;

const int TFX_DISPLAY_MODE_FINAL = 0;
const int TFX_DISPLAY_MODE_FLAT = 1;
const int TFX_DISPLAY_MODE_FOLLOW_GROUPS = 2;
const int TFX_DISPLAY_MODE_STRANDS = 3;
const int TFX_DISPLAY_MODE_ROOT_TIP_PERCENTAGE = 4;


vec3 getColorFromInstance (int instanceId) {
  switch (instanceId % 14) {
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



vec4 debugModeOverride(vec3 shadingResult, float shadow){
  vec3 result = vec3(0);
  float mixFac = 1;

  // global debug mode
  switch (u_displayMode) {
    case DISPLAY_MODE_SHADOW_MAP: {
      float shadow2 = 1.0 - shadow;
      return vec4(shadow2,shadow2,shadow2, 1);
    }
  }

  // hair debug mode
  switch (u_tfxDisplayMode) {
    case TFX_DISPLAY_MODE_FOLLOW_GROUPS: {
      result = getColorFromInstance(v_hairInstanceId);
      break;
    }
    case TFX_DISPLAY_MODE_STRANDS: {
      result = getColorFromInstance(int(v_strandId));
      break;
    }
    case TFX_DISPLAY_MODE_ROOT_TIP_PERCENTAGE: {
      result = vec3(v_vertexRootToTipFactor);
      break;
    }
    case TFX_DISPLAY_MODE_FLAT: {
      result = debugHairFlatColor();
      break;
    }
    default: {
      mixFac = 0;
      break;
    }
  }

  return vec4(result, mixFac);
}

void main() {
  Light lights[3];
  lights[0] = unpackLight(u_light0_Position, u_light0_Color);
  lights[1] = unpackLight(u_light1_Position, u_light1_Color);
  lights[2] = unpackLight(u_light2_Position, u_light2_Color);

  float ao = calculateHairAO(u_aoTex);
  float shadow = calculateHairShadow(
    u_directionalShadowDepthTex,
    v_position.xyz,
    normalize(v_normal),
    v_positionLightShadowSpace
  );
  vec3 result = doHairShading(
    lights, ao, shadow,
    v_position, normalize(v_normal), normalize(v_tangent)
  );

  vec4 colorDebug = debugModeOverride(result, shadow);
  result = mix(result, colorDebug.rgb, colorDebug.a);
  outColor1 = vec4(result, 1.0);
  outColor2 = uvec4(packNormal(v_normal), 255);
}