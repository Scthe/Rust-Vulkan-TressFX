#version 450

// must match vertex shader
#define PPLL_HEAD_POINTERS_IMAGE_BINDING 4
#define PPLL_DATA_BUFFER_BINDING 5

//@import ../_utils.glsl;
//@import _tfx_ppll_shared.glsl;
//@ import TressFXRendering.coverage.glsl;

layout(location = 0) in vec4 v_position;
layout(location = 1) in vec4 v_tangent;
layout(location = 2) in vec4 v_strandColor;
// layout(location = 3) in vec4 v_p0p1;

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
  return v_strandColor.a;
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
			vec4(to_0_1(v_tangent.xyz), alpha), // data
			v_strandColor.rgb, // color
			v_position.z // depth
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