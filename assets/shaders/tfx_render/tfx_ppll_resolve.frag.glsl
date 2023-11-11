#version 450

// Closest pixels are put in special buffer and have larger weight on the outcome
#define KBUFFER_SIZE 8
// Max entries per pixel in ppll data list
#define MAX_FRAGMENTS 512
const int PPLL_DISPLAY_MODE_FINAL = 0;
const int PPLL_DISPLAY_MODE_FLAT = 1;
const int PPLL_DISPLAY_MODE_OVERLAP = 2;


// includes
#pragma include ../_config_ubo;
#define PPLL_HEAD_POINTERS_IMAGE_BINDING 1
#define PPLL_DATA_BUFFER_BINDING 2
#pragma include _tfx_ppll_shared;


// intra-shader stuff
layout(location = 0) out vec4 outColor;

layout(early_fragment_tests) in; // [earlydepthstencil]


// fwd decl.
vec3 getDebugColorForPpllDepth();
vec4 debugModeOverride(vec3 shadingResult);


// TODO maybe just run full shading in build? In resolve we just blend.
//      Then detect hair display modes, so _build pass is cheaper.
//      Expensive..
void main () {
  // vec4 color = GatherLinkedList(gl_FragCoord.xy);
  // outColor = vec4(color.rgb, color.a);
  
  vec3 result = getDebugColorForPpllDepth();
  
  // WARNING: Blend mode means `outColor.a==0` will render nothing!
  
  vec4 colorDebug = debugModeOverride(result);
  result = mix(result, colorDebug.rgb, colorDebug.a);
  outColor = vec4(result, 1.0); // TODO alpha for blending?
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