#version 450

// Closest pixels are put in special buffer and have larger weight on the outcome
#define KBUFFER_SIZE 4
// Max entries per pixel in ppll data list
#define MAX_FRAGMENTS 512
const int PPLL_DISPLAY_MODE_FINAL = 0;
const int PPLL_DISPLAY_MODE_FLAT = 1;
const int PPLL_DISPLAY_MODE_OVERLAP = 2;


// includes
#pragma include ../_config_ubo;
#pragma include ../_utils;

#define PPLL_HEAD_POINTERS_IMAGE_BINDING 1
#define PPLL_DATA_BUFFER_BINDING 2
#pragma include _tfx_ppll_shared;
#pragma include ./_tfx_params_ubo; // binding 3
#pragma include ../materials/_hair;

// intra-shader stuff
layout(location = 0) out vec4 outColor;

layout(early_fragment_tests) in; // [earlydepthstencil]


///////////////////////
// TressFX Shading
struct PPLLFragmentData {
  vec4 tangentAndCoverage;
  vec3 positionWorldSpace;
  float depth;
};

#define PPLL_DBG_COVERAGE (0.8)

vec4 tfxCalculateFarFragmentsColor(vec2 pixelCoord, inout PPLLFragmentData frag) {
  // return vec4(0,0,1, PPLL_DBG_COVERAGE);
  // float coverage = frag.tangentAndCoverage.w; // TODO use alpha? Send coverage in PPLL
  return vec4(TfxParamsUbo.u_albedo.rgb, PPLL_DBG_COVERAGE);
}

vec4 tfxCalculateCloseFragmentsColor(vec2 pixelCoord, inout PPLLFragmentData frag) {
  // return vec4(1,0,0, PPLL_DBG_COVERAGE);
  vec3 positionWorld = frag.positionWorldSpace;
  float coverage = frag.tangentAndCoverage.w; // TODO use alpha? Send coverage in PPLL
  vec3 tangent = normalize(frag.tangentAndCoverage.xyz);
  vec3 normal = normalize(positionWorld.xyz - TfxParamsUbo.u_centerOfGravity.xyz);

  Light lights[3]; // makes you wonder how much registers this uses. Better not to dwell!
  lights[0] = unpackLight(u_light0_Position, u_light0_Color);
  lights[1] = unpackLight(u_light1_Position, u_light1_Color);
  lights[2] = unpackLight(u_light2_Position, u_light2_Color);

  float ao = 1.0; // TODO Hide both AO and shadow under UI flag
  float shadow = 0.0; // TODO
  vec3 result = doHairShading(
    lights, ao, shadow,
    positionWorld, normal, tangent
  );

  // TODO support debug modes. Maybe inout last `PPLLFragmentData` from `GatherLinkedList`?
  //      The we could try depth write from frag. shader (gl_FragDepth) maybe.. (prob not working)
  return vec4(result, PPLL_DBG_COVERAGE);
  // return vec4(abs(normal), 1.0);
  // return vec4(abs(tangent), 1.0);
}

#define TFX_SHADING_FAR_FN tfxCalculateFarFragmentsColor
#define TFX_SHADING_CLOSE_FN tfxCalculateCloseFragmentsColor
#pragma include _tfx_ppll_resolve_impl.glsl;




///////////////////////
// fwd decl.
vec3 getDebugColorForPpllDepth();
vec4 debugModeOverride(vec3 shadingResult);


// TODO maybe just run full shading in build? In resolve we just blend.
//      Then detect hair display modes, so _build pass is cheaper.
//      Expensive..
void main () {
  vec4 result = GatherLinkedList(gl_FragCoord.xy);
  
  // WARNING: Blend mode means `outColor.a==0` will render nothing!
  
  vec4 colorDebug = debugModeOverride(result.rgb);
  result.rgb = mix(result.rgb, colorDebug.rgb, colorDebug.a);
  // TODO alpha for blending? `mix(originalResult.w, 1.0, colorDebug.a);`, unless debug mode also has alpha? (e.g. coverage. tho coverage just black-white..)
  outColor = vec4(result.rgb, result.a);
}

vec4 debugModeOverride(vec3 shadingResult){
  vec3 result = vec3(0);
  float mixFac = 1;

  switch (u_tfxDisplayMode) {
    case PPLL_DISPLAY_MODE_OVERLAP: {
      result = getDebugColorForPpllDepth();
      break;
    }
    case PPLL_DISPLAY_MODE_FLAT: {
      result = vec3(0.8); // TODO use debugHairFlatColor()
      break;
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