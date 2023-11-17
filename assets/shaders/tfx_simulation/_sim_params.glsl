#define g_GravityMagnitude (0.0)
#define g_TimeStep (1.0/60.0)
#define g_NumOfStrandsPerThreadGroup (2) // TODO ?
// TODO this is per model, not a global const. push consts?
// And `g_NumOfStrandsPerThreadGroup` too, though loader already checks it's 32 verts/strand
vec4 g_Capsules[4];
#define g_Wind (vec4(-1,0,0, 10))
const float LENGTH_STIFFNESS = 1.0;

// No reason to make uniform tbh. Can just vec4(,,,0) to ignore
// TODO debug view may not be accurate due to sim. not using model_matrix?
//      so set model scale to 1.0, test-try-preview capsules, then revert scale?
const int NUM_COLLISION_CAPSULES = 4;


// Used durring Verlet integration. Can be though of as inertia.
// We calculate `delta = position_now - position_prev` and then
// multiply by damping:
//   * if damping == 0, then delta affects outcome (seemingly another acceleration)
//   * if damping == 1, then delta is nullified and verlet only calculates
//       basing on forces/gravity
float GetDamping() {
  // return g_Shape.x; // 1.0f;
  return 1.0f;
}


//
// Global Shape Constraints (GSC)

// AMD:
// If the forces/gravity are not strong enough to overcome
// this, the strands will not move.
//
// TL;DR: 'Nudge' final position toward initial position by 'that much'.
float GetGlobalStiffness() {
  // return g_Shape.z; //0.05;
  return 0.05;
}

// AMD:
// By default, Global Shape Constraints affect only `global_range * vertices_in_strand`
// vertices:
//   * globalRange == 0.0, then whole strand is affected by forces/gravity
//   * globalRange == 0.5, only half of strand (the one closer to root)
//       is affected by forces/gravity
//   * globalRange == 1.0, then strand tips will be affected
//       by GSC, which wolud negate forces/gravity etc.
// Also known as 'globalShapeMatchingEffectiveRange'
//
// TL;DR: If the number is small, the tips will be very 'bouncy' (it will affect only near root).
// If it's high, the hair will be 'static'.
float GetGlobalRange() {
  // return g_Shape.w; // 0.3;
  return 0.3;
}



//
// Local Shape Constraints

// * stiffness == 0, then no local shape preservation
// * stiffness == 1, then ignore forces/gravity VSP etc.
float GetLocalStiffness() {
  // return g_Shape.y; // 0.9;
  return 0.9;
}



//
// Length Constraints

int GetLengthConstraintIterations() {
  // return int(g_SimInts.x); //1;
  return 1;
}
