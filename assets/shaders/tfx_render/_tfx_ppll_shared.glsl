////////////////// 
// STRUCTS

struct PerPixelListEntryData {
  uint depth;
  uint data;
  uint color;
  uint uNext;
};
#define KBUFFER_TYPE uvec4

#define FRAGMENT_LIST_NULL (0xffffffff)

uint PackFloat4IntoUint(vec4 vValue) {
  return ((uint(vValue.x * 255)) << 24) |
         ((uint(vValue.y * 255)) << 16) |
         ((uint(vValue.z * 255)) << 8) |
           uint(vValue.w * 255);
}

vec4 UnpackUintIntoFloat4(uint uValue) {
  uvec4 v = uvec4(
    (uValue & 0xFF000000) >> 24,
    (uValue & 0x00FF0000) >> 16,
    (uValue & 0x0000FF00) >> 8,
    (uValue & 0x000000FF)
  );
  return vec4(v) / 255.0;
}


////////////////// 
// PPLL UNIFORMS

// 2D texture to store head pointers
// HLSL: RWTexture2D<int> u_linkedListHeadPointersImage;
layout(binding = PPLL_HEAD_POINTERS_IMAGE_BINDING, r32ui)
uniform uimage2D u_linkedListHeadPointersImage;

// SSBO to store nodes
// HLSL: RWStructuredBuffer<PerPixelListEntryData> LinkedListUAV;
layout(binding = PPLL_DATA_BUFFER_BINDING)
buffer LinkedListDataBuffer {
  PerPixelListEntryData u_linkedListDataBuffer[];
};

// size of `u_linkedListDataBuffer`
// value: width * height * AVG_FRAGS_PER_PIXEL(4)
// uniform int u_linkedListPoolSize; // TODO hardcoded
#define u_linkedListPoolSize (800*600*4)


////////////////// 
// PPLL INSERT

// Insert a new fragment at the head of the list. The old list head becomes the
// the second fragment in the list and so on. Return the address of the *old* head.
uint makeFragmentLink(ivec2 vScreenAddress, uint nNewHeadAddress) {
    // int nOldHeadAddress;
    // InterlockedExchange(u_linkedListHeadPointersImage[vScreenAddress], nNewHeadAddress, nOldHeadAddress);
    uint nOldHeadAddress = imageAtomicExchange(u_linkedListHeadPointersImage, vScreenAddress, nNewHeadAddress);
    return nOldHeadAddress;
}


// Write fragment attributes to list location.
void writeFragmentAttributes(uint nAddress, uint nPreviousLink, vec4 vData, vec3 vColor3, float fDepth) {
    u_linkedListDataBuffer[nAddress].data  = PackFloat4IntoUint(vData);
    u_linkedListDataBuffer[nAddress].color = PackFloat4IntoUint(vec4(vColor3, 0));
    u_linkedListDataBuffer[nAddress].depth = uint(fDepth * 255.0); //uint(saturate(fDepth)); or gl_FragCoord.z; ?
    u_linkedListDataBuffer[nAddress].uNext = nPreviousLink;
}

////////////////// 
// PPLL READ

uint getListHeadPointer(vec2 vfScreenAddress) {
  return imageLoad(u_linkedListHeadPointersImage, ivec2(vfScreenAddress)).r;
}

#define NODE_DATA(x)  (u_linkedListDataBuffer[x].data)
#define NODE_NEXT(x)  (u_linkedListDataBuffer[x].uNext)
#define NODE_DEPTH(x) (u_linkedListDataBuffer[x].depth) // was multiplied by 255 in build stage
#define NODE_COLOR(x) (u_linkedListDataBuffer[x].color)