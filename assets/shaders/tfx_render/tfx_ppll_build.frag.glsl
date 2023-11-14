#version 450

// must match vertex shader
#define PPLL_HEAD_POINTERS_IMAGE_BINDING 4
#define PPLL_DATA_BUFFER_BINDING 5

#pragma include ../_config_ubo;
#pragma include ../_utils;
#pragma include _tfx_ppll_shared;
// #pragma include TressFXRendering.coverage;

layout(location = 0) flat in int v_hairInstanceId;
layout(location = 1) in float v_vertexRootToTipFactor;
layout(location = 2) in vec3 v_position;
layout(location = 3) in vec3 v_normal;
layout(location = 4) in vec3 v_tangent;
layout(location = 5) in vec4 v_p0p1;

// NOTE: very important
// Force early depth tests
//
// Remember we are not writing fragment color, but for all fragments we write
// to SSBO / image2d. Normally, depth stencil can be done whatever (exp. when
// fragment's depth is modified in pixel shader). But in our case, if depth/stencil
// is done too late, we already written to SSBO etc. So yeah, do early
// depth/stencil here
layout(early_fragment_tests) in; // [earlydepthstencil]


layout(binding = 6) buffer LinkedListNextHeadCounter {
  // index of next free entry in `u_linkedListDataBuffer`
  uint u_linkedListNextHeadCounter;
};


// Allocate a new fragment location in fragment color, depth, and link buffers
uint allocateFragment(ivec2 vScreenAddress) {
  uint newAddress = atomicAdd(u_linkedListNextHeadCounter, 1); //LinkedListUAV.IncrementCounter();
  if (newAddress < 0 || newAddress >= u_tfxLinkedListPoolSize){
    newAddress = FRAGMENT_LIST_NULL;
  }
  return newAddress;
}

// TODO [LOW] Check how well this works
// Calculate the pixel coverage of a hair strand by computing the hair width
// p0, p1, pixelLoc are in d3d clip space (-1 to 1)x(-1 to 1)
//
// @param vec2 p0 - position of 'left' vertex after perspective projection (-1 to 1)
// @param vec2 p1 - position of 'right' vertex after perspective projection (-1 to 1)
float computeCoverage(vec2 p0, vec2 p1, vec2 pixelLoc, vec2 winSize) {
  vec4 dbg = vec4(p0,p1);
  vec4 positionProj = u_viewProjectionMat * vec4(v_position, 1);
  vec4 positionProjAfterW = positionProj / positionProj.w;

  // Scale positions so 1.f = half pixel width
  p0 *= winSize;
  p1 *= winSize;

  float p0dist = length(p0 - pixelLoc);
  float p1dist = length(p1 - pixelLoc);
  float hairWidth = length(p0 - p1); // distance to center of the pixel

  // if outside, set sign to -1, else set sign to 1
  bool outside = p0dist > hairWidth || p1dist > hairWidth;
  float sign = outside ? -1 : 1;

  // signed distance (positive if inside hair, negative if outside hair)
  float relDist = sign * saturate(min(p0dist, p1dist));

  // returns coverage based on the relative distance
  // 0, if completely outside hair edge
  // 1, if completely inside hair edge
  return (relDist + 1.f) * 0.5f;
}


// https://github.com/GPUOpen-Effects/TressFX/blob/ba0bdacdfb964e38522fda812bf23169bc5fa603/src/Shaders/TressFXPPLL.hlsl#L116
void main () {
	float coverage = computeCoverage(v_p0p1.xy, v_p0p1.zw, gl_FragCoord.xy, u_viewport);
	// if (coverage < 1.0 / 255.0) {
		// discard;
	// }

	// Allocate a new fragment in heads texture
	ivec2 vScreenAddress = ivec2(gl_FragCoord.xy);
	uint nNewFragmentAddress = allocateFragment(vScreenAddress);

	if (nNewFragmentAddress != FRAGMENT_LIST_NULL) {
		uint nOldFragmentAddress = makeFragmentLink(vScreenAddress, nNewFragmentAddress);
		writeFragmentAttributes(
			nNewFragmentAddress,
			nOldFragmentAddress,
			v_position.z, // depth
			v_tangent.xyz, // tangent
			coverage, // coverage
			v_position.xyz // positionWorldSpace
		);
	}
}

// paging algo breakdown(very rough draft):
// 1) pointer = curr_head[x,y]
// 2) for i in [:PAGE] // or we can store in pointer current page offset
//      if linked_list[pointer+i] is unused: (should this be atomic?)
//        write here; break;
//      else:
//        addres = atomic_inc * 4
//        do the swap in head_texture
//        write to linked_list, update old pointer
//