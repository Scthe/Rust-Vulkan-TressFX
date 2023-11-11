#version 450

// Closest pixels are put in special buffer and have larger weight on the outcome
#define KBUFFER_SIZE 8
// Max entries per pixel in ppll data list
#define MAX_FRAGMENTS 512
// const int RENDER_MODE_FILL_ONE_COLOR = 1;
// const int RENDER_MODE_PPLL_DEPTH = 2;

#define PPLL_HEAD_POINTERS_IMAGE_BINDING 0
#define PPLL_DATA_BUFFER_BINDING 1

#pragma include _tfx_ppll_shared;
// #pragma include _tfx_ppll_resolve_impl;


// uniform vec3 g_vEye;
// uniform int g_RenderMode;

layout(location = 0) out vec4 outColor;

layout(early_fragment_tests) in; // [earlydepthstencil]


vec3 getDebugColorForPpllDepth();


// TODO maybe just run full shading in build? In resolve we just blend.
//      Then detect hair display modes, so _build pass is cheaper.
//      Expensive..
void main () {
  /*
  if (g_RenderMode == RENDER_MODE_FILL_ONE_COLOR) {
    outColor = vec4(1,1,0,1);
  } else if (g_RenderMode == RENDER_MODE_PPLL_DEPTH) {
    outColor = vec4(getDebugColorForPpllDepth(), 1);
  } else {
    vec4 color = GatherLinkedList(gl_FragCoord.xy);
    // outColor = vec4(gammaFix(tonemapReinhard(color.rgb), GAMMA), color.a);
    // outColor = vec4(gammaFix(color.rgb, GAMMA), color.a);
    outColor = vec4(color.rgb, color.a);
  }
  */
  
  // WARNING: Blend mode means `outColor.a==0` will render nothing!
  outColor = vec4(getDebugColorForPpllDepth(), 1);
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