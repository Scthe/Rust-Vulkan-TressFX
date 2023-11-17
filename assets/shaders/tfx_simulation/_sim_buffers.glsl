//
// UAVs (read-write resources)
//

#ifdef BINDING_INDEX_POSITIONS
layout(std430, binding=BINDING_INDEX_POSITIONS)
buffer g_HairVertexPositionsBuffer { // RWStructuredBuffer<vec4> g_HairVertexPositions;
  vec4 g_HairVertexPositions[];
};
#endif

#ifdef BINDING_INDEX_POSITIONS_PREV
layout(std430, binding=BINDING_INDEX_POSITIONS_PREV)
buffer g_HairVertexPositionsPrevBuffer { // RWStructuredBuffer<vec4> g_HairVertexPositionsPrev;
  vec4 g_HairVertexPositionsPrev[];
};
#endif

#ifdef BINDING_INDEX_POSITIONS_PREV_PREV
layout(std430, binding=BINDING_INDEX_POSITIONS_PREV_PREV)
buffer g_HairVertexPositionsPrevPrevBuffer { // RWStructuredBuffer<vec4> g_HairVertexPositionsPrevPrev;
  vec4 g_HairVertexPositionsPrevPrev[];
};
#endif



//
// SRVs (read resources)
//

#ifdef BINDING_INDEX_POSITIONS_INITIAL
layout(std430, binding=BINDING_INDEX_POSITIONS_INITIAL)
readonly buffer g_InitialHairPositionsBuffer { // StructuredBuffer<vec4> g_InitialHairPositions;
  vec4 g_InitialHairPositions[];
};
#endif
