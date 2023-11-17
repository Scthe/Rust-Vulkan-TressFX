#version 450
// https://github.com/Scthe/TressFX-OpenGL/blob/master/src/shaders/gl-tfx/sim3_LengthConstraintsWindAndCollision.comp.glsl

#define BINDING_INDEX_POSITIONS 0
#define BINDING_INDEX_POSITIONS_PREV 1
#define BINDING_INDEX_POSITIONS_INITIAL 2
#define BINDING_INDEX_TANGENTS 3

// @return true if vec4 from `sharedPos` is movable.
bool sharedPosIsMovable(vec4 particle0) {
  return particle0.w > 0.5; // it can be only 0.0 or 1.0, so 0.5 just in case 
}

#pragma include ./_sim_params;
#pragma include ./_sim_common;
#pragma include ./_sim_buffers;
#pragma include ./_sim_params;
#pragma include ./_sim_capsule_collision;
// #pragma include "sim/_SimQuat.comp.glsl"

// THREAD_GROUP_SIZE <- 64 (2 strands, 32 vertices each)
// indexing: [vert0_strand0, vert0_strand1, vert1_strand0, vert1_strand1, ... , vert31_strand0, vert31_strand1]
#define getSharedIndex(VERTEX_ID) ((VERTEX_ID) * numOfStrandsPerThreadGroup + vertData.strandId)

shared vec4 sharedPos[THREAD_GROUP_SIZE];
// Distance from current vertex to the next one. Is 0 for last vertex in strand.
shared float sharedLength[THREAD_GROUP_SIZE];



float GetInitalLength(inout PerVertexData vertData, uint numVerticesInTheStrand) {
  if (vertData.vertexId == numVerticesInTheStrand - 1) { // tip
    return 0;
  }
  vec4 pos0 = g_InitialHairPositions[vertData.vertexId_global];
  vec4 pos1 = g_InitialHairPositions[vertData.vertexId_global + 1];
  return length(pos0.xyz - pos1.xyz);
}

vec2 ConstraintMultiplier(vec4 particle0, vec4 particle1) {
  bool can_move0 = particle0.w > 0.5; // it can be only 0.0 or 1.0, so 0.5 just in case
  bool can_move1 = particle1.w > 0.5; // it can be only 0.0 or 1.0, so 0.5 just in case
  if ( can_move0 &&  can_move1) { return vec2(0.5, 0.5); } // move equally
  if ( can_move0 && !can_move1) { return vec2(1.0, 0.0); } // move only 1st
  if (!can_move0 &&  can_move1) { return vec2(0.0, 1.0); } // move only 2nd
  return vec2(0, 0); // can't move either
}

void ApplyDistanceConstraint(uint idx0, uint idx1, float expectedLength) {
  vec4 pos0 = sharedPos[idx0];
  vec4 pos1 = sharedPos[idx1];

  vec3 delta = pos1.xyz - pos0.xyz;
  float distance = max(length(delta), 1e-7);
  // expectedLength / distance:
  //  * > 1 if real distance if TOO SHORT and we need to ELONGATE
  //  * < 1 if real distance is BIGGER than expected and we need to SHORTEN
  //  stretching:
  //  * < 0 if we need to ELONGATE
  //  * > 0 if we need to SHORTEN
  float stretching = 1 - expectedLength / distance;
  delta = stretching * delta;

  // how much we scale movement of each vertex
  vec2 multiplier = ConstraintMultiplier(pos0, pos1);

  sharedPos[idx0].xyz += multiplier[0] * delta * LENGTH_STIFFNESS;
  sharedPos[idx1].xyz -= multiplier[1] * delta * LENGTH_STIFFNESS;
}

//
// 1) wind
// 2) length constraints
// 3) capsule collisions
// 4) update tangents
// 5) write back to g_HairVertexPositions
//
// One thread computes one vertex.
//
layout (local_size_x = THREAD_GROUP_SIZE) in; // [numthreads(THREAD_GROUP_SIZE, 1, 1)]
void main() {
  g_Capsules[0] = vec4(0,0,0, 0);
  g_Capsules[1] = vec4(0,0,0, 0);
  g_Capsules[2] = vec4(0,0,0, 0);
  g_Capsules[3] = vec4(0,0,0, 0);


  const uint numOfStrandsPerThreadGroup = g_NumOfStrandsPerThreadGroup;
  uint numVerticesInTheStrand; // 32
  PerVertexData vertData = GetPerVertexData(
    gl_LocalInvocationIndex, // index in workgroup [0, THREAD_GROUP_SIZE)
    gl_WorkGroupID.x, // if of the workgroup, [0, SCHEDULED_JOBS / THREAD_GROUP_SIZE)
    numVerticesInTheStrand
  );

  // Copy data into shared memory
  float isMovable = IsMovable(vertData.vertexId) ? 1.0 : 0.0;
  sharedPos[vertData.localId] = vec4(g_HairVertexPositions[vertData.vertexId_global].xyz, isMovable);
  sharedLength[vertData.localId] = GetInitalLength(vertData, numVerticesInTheStrand);
  GroupMemoryBarrierWithGroupSync();


  // Wind
  if (IsMovable(vertData.vertexId)) {
    uint sharedIndex      = getSharedIndex(vertData.vertexId);
    uint sharedIndex_next = getSharedIndex(vertData.vertexId + 1);
    // vector(next_vertex -> this_vertex), NOT NORMALIZED
    vec3 from_next_vert = sharedPos[sharedIndex].xyz - sharedPos[sharedIndex_next].xyz;

    // add small value to make it not panic during cross().
    vec3 windDirection =  normalize(g_Wind.xyz + vec3(0.00001));
    float windStrength = length(from_next_vert) * g_Wind.w; // longer edge means more force applied
    from_next_vert = normalize(from_next_vert);
    // make wind perpendicular to strand.
    // vec3 force = cross(cross(from_next_vert, windDirection), from_next_vert);
    // force *= windStrength;
    vec3 force = windDirection * windStrength;

    // apply wind
    sharedPos[sharedIndex].xyz += force * g_TimeStep * g_TimeStep;
  }
  GroupMemoryBarrierWithGroupSync();


  // Enforce length constraints
  // https://github.com/GPUOpen-Effects/TressFX/blob/master/src/Shaders/TressFXSimulation.hlsl#L918
  uint maxVertexId_n_n1 = uint(floor(float(numVerticesInTheStrand) / 2.0f)); // 16
  uint maxVertexId_n1_n2 = uint(floor(float(numVerticesInTheStrand - 1) / 2.0f)); // 15
  int nLengthContraintIterations = GetLengthConstraintIterations();

  // we re-adjust positions several times, getting more accurate results with each iter.
  // In each iter. we operate on 2 distances between 3 diffrent consecutive vertices
  for (int jitteration = 0; jitteration < nLengthContraintIterations; jitteration++) {
    // vert0 strand0: 2*0*2 + 0 = 0  | n1=2  | n2=4
    // vert0 strand1: 2*0*2 + 1 = 1  | n1=3  | n2=5
    // vert1 strand0: 2*1*2 + 0 = 4  | n1=6  | n2=8
    // vert1 strand1: 2*1*2 + 1 = 5  | n1=7  | n2=9
    // vert8 strand0: 2*8*2 + 0 = 32 | n1=34 | n2=36
    // vert8 strand1: 2*8*2 + 1 = 33 | n1=35 | n2=37
    uint sharedIndex    = 2 * vertData.vertexId * numOfStrandsPerThreadGroup + vertData.strandId;
    uint sharedIndex_n1 = sharedIndex + numOfStrandsPerThreadGroup;
    uint sharedIndex_n2 = sharedIndex + numOfStrandsPerThreadGroup * 2;

    // length constraint: 
    if (vertData.vertexId < maxVertexId_n_n1) {
      float expectedLength = sharedLength[sharedIndex].x;
      ApplyDistanceConstraint(sharedIndex, sharedIndex_n1, expectedLength);
    }
    GroupMemoryBarrierWithGroupSync();

    if (vertData.vertexId < maxVertexId_n1_n2) {
      float expectedLength = sharedLength[sharedIndex_n1].x;
      ApplyDistanceConstraint(sharedIndex_n1, sharedIndex_n2, expectedLength);
    }
    GroupMemoryBarrierWithGroupSync();
  }


  // Collision handling with capsule objects
  vec4 oldPos = g_HairVertexPositionsPrev[vertData.vertexId_global];
  bool bAnyColDetected = ResolveCapsuleCollisions(sharedPos[vertData.localId], oldPos);
  GroupMemoryBarrierWithGroupSync();


  // Compute tangent
  // tangent := normalize(vertex -> next_vertex)
  // If this is the last vertex in the strand, we can't get tangent from subtracting from the next vertex, need to use previous instead
  uint nextVertexLocalIdMod = (vertData.vertexId == numVerticesInTheStrand - 1) ? -numOfStrandsPerThreadGroup : numOfStrandsPerThreadGroup;
  vec3 tangent = sharedPos[vertData.localId + nextVertexLocalIdMod].xyz
               - sharedPos[vertData.localId].xyz;
  g_HairVertexTangents[vertData.vertexId_global].xyz = normalize(tangent);

  // update global position buffers
  g_HairVertexPositions[vertData.vertexId_global] = sharedPos[vertData.localId];
  // update previous frame data as it has led to collision. Not sure 
  if (bAnyColDetected) {
    g_HairVertexPositionsPrev[vertData.vertexId_global] = sharedPos[vertData.localId];
  }
}
