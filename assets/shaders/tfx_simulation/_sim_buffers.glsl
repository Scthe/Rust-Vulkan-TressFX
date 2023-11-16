//
// UAVs (read-write resources)
//

layout(std430, binding=0)
buffer g_HairVertexPositionsBuffer { // RWStructuredBuffer<vec4> g_HairVertexPositions;
  vec4 g_HairVertexPositions[];
};

layout(std430, binding=1)
buffer g_HairVertexPositionsPrevBuffer { // RWStructuredBuffer<vec4> g_HairVertexPositionsPrev;
  vec4 g_HairVertexPositionsPrev[];
};

layout(std430, binding=2)
buffer g_HairVertexPositionsPrevPrevBuffer { // RWStructuredBuffer<vec4> g_HairVertexPositionsPrevPrev;
  vec4 g_HairVertexPositionsPrevPrev[];
};



//
// SRVs (read resources)
//

layout(std430, binding=3)
readonly buffer g_InitialHairPositionsBuffer { // StructuredBuffer<vec4> g_InitialHairPositions;
  vec4 g_InitialHairPositions[];
};
