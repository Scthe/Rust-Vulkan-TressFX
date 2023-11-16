#define g_GravityMagnitude (0.0)
#define g_TimeStep (1.0/60.0)
#define g_NumOfStrandsPerThreadGroup (2) // TODO ?

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
