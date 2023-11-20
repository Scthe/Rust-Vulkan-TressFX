#version 450

// Closest pixels are put in special buffer and have larger weight on the outcome
#define KBUFFER_SIZE 4
// Max entries per pixel in ppll data list
#define MAX_FRAGMENTS 512
const int PPLL_DISPLAY_MODE_FINAL = 0;
const int PPLL_DISPLAY_MODE_FLAT = 1;
const int PPLL_DISPLAY_MODE_OVERLAP = 2;
const int PPLL_DISPLAY_MODE_TANGENTS = 3;
const int PPLL_DISPLAY_MODE_COVERAGE = 4;


// includes
#pragma include ../_config_ubo;
#pragma include ../_utils;

#define PPLL_HEAD_POINTERS_IMAGE_BINDING 1
#define PPLL_DATA_BUFFER_BINDING 2
#pragma include _tfx_ppll_shared;
#pragma include ./_tfx_params_ubo; // binding 3
#pragma include ../materials/_hair;

// intra-shader stuff
layout(location = 0) out vec4 outColor1;
layout(location = 1) out uvec4 outColor2;

layout(binding = 4) uniform sampler2D u_aoTex;
layout(binding = 5) uniform sampler2D u_directionalShadowDepthTex;


layout(early_fragment_tests) in; // [earlydepthstencil]



///////////////////////
// TressFX Shading
#pragma include ../materials/_material; // for light struct

float PrecalcAmbientOcclusion;
Light GlobalLightsArray[3];

struct PPLLFragmentData {
  vec4 tangentAndCoverage;
  vec3 positionWorldSpace;
  float depth;
};

float calculateShadowForPPLLFragment(inout PPLLFragmentData frag, vec3 normal) {
  vec4 positionShadowProjected = u_directionalShadowMatrix_VP * vec4(frag.positionWorldSpace, 1);
  return calculateHairShadow (
    u_directionalShadowDepthTex,
    frag.positionWorldSpace,
    normal,
    positionShadowProjected
  );
}

vec4 tfxCalculateFarFragmentsColor(vec2 pixelCoord, inout PPLLFragmentData frag) {
  float coverage = frag.tangentAndCoverage.w;
  if (u_tfxDisplayMode == PPLL_DISPLAY_MODE_COVERAGE) {
    return vec4(coverage,coverage,coverage, 1);
  }
  return vec4(TfxParamsUbo.u_albedo.rgb, u_tfxOpacity);
}

vec4 tfxCalculateCloseFragmentsColor(vec2 pixelCoord, inout PPLLFragmentData frag) {
  // return vec4(TfxParamsUbo.u_albedo.rgb, u_tfxOpacity);
  vec3 positionWorld = frag.positionWorldSpace;
  float coverage = frag.tangentAndCoverage.w;
  vec3 tangent = frag.tangentAndCoverage.xyz;
  vec3 normal = calculateHairNormal(positionWorld.xyz);

  if (u_tfxDisplayMode == PPLL_DISPLAY_MODE_COVERAGE) {
    return vec4(coverage,coverage,coverage, 1);
  }

  float ao = PrecalcAmbientOcclusion;
  float shadow = calculateShadowForPPLLFragment(frag, normal); // TODO [LOW] can be expensive, though only for `KBUFFER_SIZE`, so not *that* bad?
  vec3 result = doHairShading(
    GlobalLightsArray, ao, shadow,
    positionWorld, normal, tangent
  );

  return vec4(result, u_tfxOpacity);
}

#define TFX_SHADING_FAR_FN tfxCalculateFarFragmentsColor
#define TFX_SHADING_CLOSE_FN tfxCalculateCloseFragmentsColor
#pragma include _tfx_ppll_resolve_impl.glsl;




///////////////////////
// fwd decl.
vec3 getDebugColorForPpllDepth();
vec4 debugModeOverride(vec3 shadingResult, inout PPLLFragmentData closestFragment, vec3 normal);


void main () {
  GlobalLightsArray[0] = unpackLight(u_light0_Position, u_light0_Color);
  GlobalLightsArray[1] = unpackLight(u_light1_Position, u_light1_Color);
  GlobalLightsArray[2] = unpackLight(u_light2_Position, u_light2_Color);
  // shared value based on last-frame's closest fragment
  PrecalcAmbientOcclusion = calculateHairAO(u_aoTex);

  PPLLFragmentData closestFragment;
  vec4 result = GatherLinkedList(gl_FragCoord.xy, closestFragment);
  
  
  // gather debug output
  vec3 normal = calculateHairNormal(closestFragment.positionWorldSpace.xyz);
  vec4 colorDebug = debugModeOverride(result.rgb, closestFragment, normal);
  result = mix(result.rgba, colorDebug.rgba, colorDebug.a);

  // write color + normals
  // WARNING: Blend mode means `outColor1.a==0` may render nothing!
  // Normals have blend=OFF, so no worry there, only color can be a problem.
  outColor1 = result;
  outColor2 = uvec4(packNormal(normal), 255);
}

vec4 debugModeOverride(vec3 shadingResult, inout PPLLFragmentData closestFragment, vec3 normal){
  vec3 result = vec3(0);
  float mixFac = 1;

  // global debug mode
  switch (u_displayMode) {
    case DISPLAY_MODE_SHADOW_MAP: {
      float shadow = 1 - calculateShadowForPPLLFragment(closestFragment, normal);
      return vec4(shadow,shadow,shadow, 1);
    }
  }

  // hair debug mode
  switch (u_tfxDisplayMode) {
    case PPLL_DISPLAY_MODE_OVERLAP: {
      result = getDebugColorForPpllDepth();
      break;
    }
    case PPLL_DISPLAY_MODE_FLAT: {
      result = debugHairFlatColor();
      break;
    }
    case PPLL_DISPLAY_MODE_TANGENTS: {
      result = abs(closestFragment.tangentAndCoverage.xyz);
      break;
    }
    case PPLL_DISPLAY_MODE_COVERAGE: {
      result = shadingResult;
    }
    default: {
      mixFac = 0;
      break;
    }
  }

  return vec4(result, mixFac);
}

uint countListNodesForPixel (vec2 vfScreenAddress) {
  uint result = 0;
  uint pointer = getListHeadPointer(vfScreenAddress).r;
  uint iter = 0;

  while (pointer != FRAGMENT_LIST_NULL && iter < MAX_FRAGMENTS) {
    pointer = NODE_NEXT(pointer);
    ++result;
    ++iter;
  }

  return result;
}

vec3 getDebugColorForPpllDepth() {
  const float MAX_DEBUG_LIST_DEPTH = 12;
  const vec3 ColorFragmentsZero = vec3(0,0,1);
  const vec3 ColorFragmentsFull = vec3(1,0,0);

  uint depth = countListNodesForPixel(gl_FragCoord.xy);
  float fac = clamp(float(depth) / MAX_DEBUG_LIST_DEPTH, 0, 1);
  return mix(ColorFragmentsZero, ColorFragmentsFull, fac);
}