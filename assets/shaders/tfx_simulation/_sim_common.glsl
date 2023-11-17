// HLSL -> GLSL notes
// (from: https://anteru.net/blog/2016/mapping-between-hlsl-and-glsl/index.html)
//  * SV_DispatchThreadID gl_GlobalInvocationID
//  * SV_GroupID gl_WorkGroupID
//  * SV_GroupIndex gl_LocalInvocationIndex


// If you change the value below, you must change it in `TfxSim0Pass.rs` etc. as well.
#define THREAD_GROUP_SIZE 64

// ?Old skinning method?
#define USE_MESH_BASED_HAIR_TRANSFORM 0

// Toggle capsule collisions
#ifndef TRESSFX_COLLISION_CAPSULES
#define TRESSFX_COLLISION_CAPSULES 0
#endif

struct PerVertexData {
  uint localId; // [0-64] unique indexForSharedMem
  uint strandId; // {0,1}, localStrandIndex (each workgroup operates on 2 strands)
  uint strandId_global; // globalStrandIndex
  uint vertexId; // [0-32], localVertexIndex
  uint vertexId_global; // globalVertexIndex
};

PerVertexData GetPerVertexData(uint local_id, uint group_id, inout uint numVerticesInTheStrand) {
  numVerticesInTheStrand = (THREAD_GROUP_SIZE / g_NumOfStrandsPerThreadGroup);

  PerVertexData d;
  d.localId = local_id;
  d.strandId = local_id % g_NumOfStrandsPerThreadGroup;
  d.strandId_global = group_id * g_NumOfStrandsPerThreadGroup + d.strandId;
  d.vertexId = (local_id - d.strandId) / g_NumOfStrandsPerThreadGroup;
  d.vertexId_global = d.strandId_global * numVerticesInTheStrand + d.vertexId;
  return d;
}

bool IsMovable(uint vertexInStrandId) {
  return vertexInStrandId > 1; // verts 0, 1 are not movable
}

void CalcIndicesInStrandLevelMaster(
  uint local_id, uint group_id,
  inout uint globalStrandIndex,
  inout uint numVerticesInTheStrand,
  inout uint globalRootVertexIndex
) {
  globalStrandIndex = THREAD_GROUP_SIZE * group_id + local_id;
  numVerticesInTheStrand = (THREAD_GROUP_SIZE / g_NumOfStrandsPerThreadGroup);
  globalRootVertexIndex = globalStrandIndex * numVerticesInTheStrand;
}

void GroupMemoryBarrierWithGroupSync () {
  memoryBarrierShared();
  barrier();
}
