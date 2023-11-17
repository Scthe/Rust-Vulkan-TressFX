#version 450
// https://github.com/Scthe/TressFX-OpenGL/blob/master/src/shaders/gl-tfx/sim2_LocalShapeConstraints.comp.glsl

#define BINDING_INDEX_POSITIONS 0
#define BINDING_INDEX_POSITIONS_INITIAL 1

#pragma include ./_sim_params;
#pragma include ./_sim_common;
#pragma include ./_sim_buffers;
// #pragma include ./_sim_quat;

// Compute shader to maintain the local shape constraints.
// for each vertex in strand (excluding root vertex):
//   1) get initial (frame 0) vector: (vertex -> next_vertex)
//   2) calculate where, according to this vector, would next_vertex lie
//   3) compare this with current next_vertex position after gravity, shock propagation etc.
//   4) adjust g_HairVertexPositions_[i], g_HairVertexPositions_[i-1] based on
//      local shape constraint param
//
// One thread computes one strand.
//
layout (local_size_x = THREAD_GROUP_SIZE) in; // [numthreads(THREAD_GROUP_SIZE, 1, 1)]
void main() {
  uint globalStrandIndex, numVerticesInTheStrand, globalRootVertexIndex;
  CalcIndicesInStrandLevelMaster(
    gl_LocalInvocationIndex, gl_WorkGroupID.x,
    globalStrandIndex, numVerticesInTheStrand, globalRootVertexIndex
  );

  // stiffness for local shape constraints
  float stiffnessForLocalShapeMatching = GetLocalStiffness();
  // 1.0 for stiffness makes things unstable sometimes.
  stiffnessForLocalShapeMatching = min(stiffnessForLocalShapeMatching, 0.95f);

  // Local shape constraint for bending/twisting
  vec4 pos_prev = g_HairVertexPositions[globalRootVertexIndex];
  vec4 pos_init_prev = g_InitialHairPositions[globalRootVertexIndex];

  // iterate starting from child vertex 1 (which means first closest to the root)
  // in strand all the way to the tip
  for (uint i = 1; i < numVerticesInTheStrand - 1; i++ ) {
    uint globalVertexIndex = globalRootVertexIndex + i;
    // pos of previous vertex in strand
    vec4 pos = g_HairVertexPositions[globalVertexIndex];
    vec4 pos_init = g_HairVertexPositions[globalVertexIndex];

    // delta from current_vert -> prev_vert - expected local shape (think curly hair)
    vec4 delta_init = pos_init - pos_init_prev;
    vec4 delta_now = pos - pos_prev;
    vec4 delta_diff = delta_init - delta_now; // delta from now->expected 
    // 0.5 cause we move both current and prev vert
    delta_diff = stiffnessForLocalShapeMatching * delta_diff * 0.5;
    
    // apply to go back to local shape
    if (IsMovable(i)){ pos.xyz += delta_diff.xyz; }
    if (IsMovable(i - 1)){ pos_prev.xyz -= delta_diff.xyz; }

    // write back
    g_HairVertexPositions[globalVertexIndex].xyz = pos.xyz;
    g_HairVertexPositions[globalVertexIndex - 1].xyz = pos_prev.xyz;

    // move forward with iteration
    pos_prev = pos;
    pos_init_prev = pos_init;
  }
}