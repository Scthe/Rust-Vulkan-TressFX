#version 450
// https://github.com/Scthe/TressFX-OpenGL/blob/master/src/shaders/gl-tfx/sim0_IntegrationAndGlobalShapeConstraints.comp.glsl

#define BINDING_INDEX_POSITIONS 0
#define BINDING_INDEX_POSITIONS_PREV 1
#define BINDING_INDEX_POSITIONS_PREV_PREV 2
#define BINDING_INDEX_POSITIONS_INITIAL 3

#pragma include ./_sim_params;
#pragma include ./_sim_common;
#pragma include ./_sim_buffers;
// #pragma include ./_sim_quat;

shared vec4 sharedPos[THREAD_GROUP_SIZE];

// Uses Verlet integration to calculate the new position for the current time step
vec4 Integrate(
  vec4 curPosition, vec4 oldPosition,
  vec4 force, float dampingCoeff
) {
  vec4 outputPos = curPosition;

  force.xyz += g_GravityMagnitude * vec3(0, -1.0f, 0);
  vec3 posDelta = curPosition.xyz - oldPosition.xyz;
  outputPos.xyz = curPosition.xyz
                  + (1.0 - dampingCoeff) * posDelta
                  + force.xyz * g_TimeStep * g_TimeStep;

  return outputPos;
}

// Updates the hair vertex positions based on the physics simulation
void UpdateFinalVertexPositions(
  vec4 oldPosition,
  vec4 newPosition,
  uint globalVertexIndex
) {
  g_HairVertexPositionsPrevPrev[globalVertexIndex] = g_HairVertexPositionsPrev[globalVertexIndex];
  g_HairVertexPositionsPrev[globalVertexIndex] = oldPosition;
  g_HairVertexPositions[globalVertexIndex] = newPosition;
}

// TODO [LOW] why does AMD store this in shared memory? Macro to make the code more readable.
#define nextPosition sharedPos[vertData.localId]


// Compute shader to simulate the gravitational force with integration
// and to maintain the global shape constraints.
//   1) Apply skinning
//   2) Integrate using forces (only gravity ATM)
//   3) Try to go back to initial position (global shape constaint)
//   4) Write to all g_HairVertexPositions* SSBOs
//
// One thread computes one vertex.
//
layout (local_size_x = THREAD_GROUP_SIZE) in; // [numthreads(THREAD_GROUP_SIZE, 1, 1)]
void main() {
  uint numVerticesInTheStrand; // 32
  PerVertexData vertData = GetPerVertexData(
    gl_LocalInvocationIndex, // index in workgroup [0, THREAD_GROUP_SIZE)
    gl_WorkGroupID.x, // if of the workgroup, [0, SCHEDULED_JOBS / THREAD_GROUP_SIZE)
    numVerticesInTheStrand
  );

  // Apply bone skinning to initial position.
  // TODO [LOW] add Model matrix here. Gravity should point global down.
  // vec4 bone_quat;
  vec4 initialPos = g_InitialHairPositions[vertData.vertexId_global]; // rest position
  // initialPos.xyz = ApplyVertexBoneSkinning(initialPos.xyz, /*skinningData,*/ bone_quat);
  // we temporarily use g_HairVertexTangents to hold bone quaternion data compute in ApplyVertexBoneSkinning.
  // g_HairVertexTangents[vertData.strandId_global] = bone_quat; // TODO needed?

  // position when this step starts. In other words, a position from the last step.
  vec4 currentPos = nextPosition = g_HairVertexPositions[vertData.vertexId_global];

  GroupMemoryBarrierWithGroupSync();


  // Integrate
  vec4 oldPos = g_HairVertexPositionsPrev[vertData.vertexId_global];
  vec4 force = vec4(0, 0, 0, 0);
  bool isMoveable = IsMovable(vertData.vertexId);
  if (isMoveable){
    float damping = GetDamping(); // 1.0f;
    nextPosition = Integrate(
      currentPos, oldPos, force, damping
    );
  } else {
    nextPosition = initialPos;
  }

  
  // Global Shape Constraints
  float stiffnessForGlobalShapeMatching = GetGlobalStiffness();
  bool hasStiffness = stiffnessForGlobalShapeMatching > 0;// && globalShapeMatchingEffectiveRange;
  // 
  float globalShapeMatchingEffectiveRange = GetGlobalRange();
  bool closeEnoughToRoot = float(vertData.vertexId) < globalShapeMatchingEffectiveRange * float(numVerticesInTheStrand);

  if (hasStiffness && isMoveable && closeEnoughToRoot) {
    // (Calc delta to initial position and move in that direction)
    vec3 delta = (initialPos - nextPosition).xyz;
    nextPosition.xyz += stiffnessForGlobalShapeMatching * delta;
  }

  // update global position buffers
  // vec4 newPosition = currentPos + vec4(0, 0.01, 0, 0); // Test: hair flies up!
  UpdateFinalVertexPositions(
    currentPos, nextPosition, vertData.vertexId_global
  );
}

/*
vec3 ApplyVertexBoneSkinning(vec3 vertexPos, BoneSkinningData skinningData, inout vec4 bone_quat) {
  vec3 newVertexPos;

  {
    // Interpolate world space bone matrices using weights.
    row_major mat4 bone_matrix = g_BoneSkinningMatrix[skinningData.boneIndex[0]] * skinningData.boneWeight[0];
    float weight_sum = skinningData.boneWeight[0];

    for (int i = 1; i < 4; i++) {
      if (skinningData.boneWeight[i] > 0) {
        bone_matrix += g_BoneSkinningMatrix[skinningData.boneIndex[i]] * skinningData.boneWeight[i];
        weight_sum += skinningData.boneWeight[i];
      }
    }

    bone_matrix /= weight_sum;
    bone_quat = MakeQuaternion(bone_matrix);

    newVertexPos = mul(vec4(vertexPos, 1), bone_matrix).xyz;
  }

  return newVertexPos;
}
*/
