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
