#version 450

// must match vertex shader
#define PPLL_HEAD_POINTERS_IMAGE_BINDING 4
#define PPLL_DATA_BUFFER_BINDING 5

#pragma include ../_utils;
#pragma include _tfx_ppll_shared;
// #pragma include TressFXRendering.coverage;

layout(location = 0) flat in int v_hairInstanceId;
layout(location = 1) in float v_vertexRootToTipFactor;
layout(location = 2) in vec3 v_position;
layout(location = 3) in vec3 v_normal;
layout(location = 4) in vec3 v_tangent;
layout(location = 5) in vec4 v_positionLightShadowSpace; // TODO not used?

layout(location = 0) out vec4 outColor1;

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
  if (newAddress < 0 || newAddress >= u_linkedListPoolSize){
    newAddress = FRAGMENT_LIST_NULL;
  }
  return newAddress;
}


float get_alpha () {
  // TODO [HIGH] compute pixel coverage from strand
  // uniform vec2 g_WinSize;
	// float coverage = ComputeCoverage(v_p0p1.xy, v_p0p1.zw, gl_FragCoord.xy, g_WinSize);
	// return coverage * v_strandColor.a;
  // return v_strandColor.a;
  return 0.5;
}

void main () {
	float alpha = get_alpha(); // 1.0;
	if (alpha < 1.0 / 255.0) {
		discard;
	}

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
			alpha, // coverage
			v_position.xyz // positionWorldSpace
		);
	}

  // outColor1 = vec4(1.0); // TODO debug that fragment shader is running. Then why stencil bit is not set?!
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