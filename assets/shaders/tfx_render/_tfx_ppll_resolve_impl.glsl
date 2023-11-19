// https://github.com/Scthe/TressFX-OpenGL/blob/master/src/shaders/gl-tfx/lib/TressFXPPLL.resolve.glsl

// @returns `true` if `DEPTH_BBB` is further than `DEPTH_AAA`
#define DEPTH_IS_FURTHER(DEPTH_AAA, DEPTH_BBB) ((DEPTH_AAA) < (DEPTH_BBB))
#define DEPTH_RESET_TO_CLOSE (0)


void ClearKBuffer(inout PPLLFragmentData kBuffer[KBUFFER_SIZE]) {
  for (int t = 0; t < KBUFFER_SIZE; ++t) {
    kBuffer[t].depth = 100000.0;
    kBuffer[t].tangentAndCoverage = vec4(0); // coverage 0.0 means ignored. Though we will use FRAGMENT_LIST_NULL to detect end of the list
  }
}

PPLLFragmentData unpackPPLLFragment(uint pointer) {
  PPLLFragmentData frag;
  frag.tangentAndCoverage = parseTangentAndCoverage(NODE_TANGENT_COV(pointer));
  frag.positionWorldSpace = NODE_POSITION(pointer);
  frag.depth = NODE_DEPTH(pointer);
  return frag;
}

void swapPPLLFragmentsData(inout PPLLFragmentData fragA, inout PPLLFragmentData fragB) {
  vec4 tangentAndCoverage = fragA.tangentAndCoverage;
  vec3 positionWorldSpace = fragA.positionWorldSpace;
  float depth = fragA.depth;
  fragA.tangentAndCoverage = fragB.tangentAndCoverage;
  fragA.positionWorldSpace = fragB.positionWorldSpace;
  fragA.depth = fragB.depth;
  fragB.tangentAndCoverage = tangentAndCoverage;
  fragB.positionWorldSpace = positionWorldSpace;
  fragB.depth = depth;
}

uint FillFirstKBuffferElements (inout PPLLFragmentData kBuffer[KBUFFER_SIZE], uint pointer, inout uint linkedListElements) {
  for (int p = 0; p < KBUFFER_SIZE; ++p) {
    if (pointer != FRAGMENT_LIST_NULL) {
      kBuffer[p] = unpackPPLLFragment(pointer);
      pointer = NODE_NEXT(pointer);
      ++linkedListElements;
    }
  }
  return pointer;
}

int FindFurthestKBufferEl (inout PPLLFragmentData kBuffer[KBUFFER_SIZE], inout float max_depth) {
  int id = 0;
  for (int i = 0; i < KBUFFER_SIZE; i++) {
    float fDepth = kBuffer[i].depth;
    if (DEPTH_IS_FURTHER(max_depth, fDepth)) {
      // `fDepth` is further than `max_depth`
      max_depth = fDepth;
      id = i;
    }
  }
  return id;
}


// https://github.com/GPUOpen-Effects/TressFX/blob/ba0bdacdfb964e38522fda812bf23169bc5fa603/src/Shaders/TressFXPPLL.hlsl#L252
vec4 GatherLinkedList(vec2 vfScreenAddress, inout PPLLFragmentData closestFragment) {
  uint pointer = getListHeadPointer(vfScreenAddress).r;
  if (pointer == FRAGMENT_LIST_NULL) {
    discard;
  }

  // kBuffer - local tmp buffer for first K values from PPLL.
  //
  // create kBuffer to hold intermediary values. We are going to fill it with
  // KBUFFER_SIZE of PPLL_STRUCTs that are closest to the camera. The closest
  // linked list elements have special treatment in blending
  PPLLFragmentData kBuffer[KBUFFER_SIZE];
  ClearKBuffer(kBuffer);
  uint linkedListElements = 0; // count of traversed elements
  pointer = FillFirstKBuffferElements(kBuffer, pointer, linkedListElements);

  vec4 fcolor = vec4(0, 0, 0, 1); // final fragment color

  // TAIL := all vertices that are not in kBuffer
  // If the node in the linked list is nearer than the furthest one in the local array,
  // exchange the node in the local array for the one in the linked list.
  for (int iFragment = 0; iFragment < MAX_FRAGMENTS; ++iFragment) {
    if (pointer == FRAGMENT_LIST_NULL) break;

    // find id of node to be exchanged (one with kbufferFurthestDepth)
    float kbufferFurthestDepth = DEPTH_RESET_TO_CLOSE;
    int kBufferFurthestIdx = FindFurthestKBufferEl(kBuffer, kbufferFurthestDepth);

    // fetch data for this iteration of linked list elements
    PPLLFragmentData furthestFragment = unpackPPLLFragment(pointer);

    // kBuffer collects linked list elements closest to the eye. If element
    // under pointer is closer then furthest kBuffer element, then exchange
    if (DEPTH_IS_FURTHER(furthestFragment.depth, kbufferFurthestDepth)) {
      // `kbufferFurthestDepth` is further than `furthestFragment.depth`
      swapPPLLFragmentsData(furthestFragment, kBuffer[kBufferFurthestIdx]);
    }

    // add the element to accumulating value
    vec4 fragmentColor = TFX_SHADING_FAR_FN(vfScreenAddress, furthestFragment);
    float alpha = fragmentColor.a;
    fcolor.rgb = fcolor.rgb * (1.0 - alpha) + (fragmentColor.rgb * alpha) * alpha;
    fcolor.a *= (1.0 - alpha);

    pointer = NODE_NEXT(pointer);
    ++linkedListElements;
  }


  // Blend the k nearest layers of fragments from back to front, where k = KBUFFER_SIZE
  // ofc if linked list has <KBUFFER_SIZE elements we can stop early
  for (int j = 0; j < min(KBUFFER_SIZE, linkedListElements); j++) {
    float kbufferFurthestDepth = DEPTH_RESET_TO_CLOSE;
    int kBufferFurthestIdx = FindFurthestKBufferEl(kBuffer, kbufferFurthestDepth);

    // Use high quality shading for the nearest k fragments
    vec4 fragmentColor = TFX_SHADING_CLOSE_FN(vfScreenAddress, kBuffer[kBufferFurthestIdx]);
    closestFragment = kBuffer[kBufferFurthestIdx];

    // Blend in the fragment color
    float alpha = fragmentColor.a;
    fcolor.rgb = fcolor.rgb * (1.0 - alpha) + (fragmentColor.rgb * alpha) * alpha;
    fcolor.a *= (1.0 - alpha);

    // take this node out of the next search (will fail FindFurthestKBufferEl)
    kBuffer[kBufferFurthestIdx].depth = DEPTH_RESET_TO_CLOSE;
  }

  return fcolor;
}
