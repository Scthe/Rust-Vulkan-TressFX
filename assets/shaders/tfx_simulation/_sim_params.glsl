#pragma include ../_config_ubo;

//
// Constants

// There is an assert inside loader that each asset has to have 32 vertices per strand.
// All simulation shaders use 64 threads workgroups. So 2 strands per workgroup in by-vertex mode.
// Ofc. if we run thread per strand, this setting has no sense
#define g_NumOfStrandsPerThreadGroup (2)

const int NUM_COLLISION_CAPSULES = 4;


//
// Uniforms

#define g_GravityMagnitude (u_tfxHairSettings.z)
#define g_TimeStep (u_tfxHairSettings.w)
#define g_Wind (u_tfxWind)
#define g_LengthStiffness (u_tfxConstraints.y)






// Used durring Verlet integration. Can be though of as inertia.
// We calculate `delta = position_now - position_prev` and then
// multiply by damping:
//   * if damping == 0, then delta affects outcome (seemingly another acceleration)
//   * if damping == 1, then delta is nullified and verlet only calculates
//       basing on forces/gravity
float GetDamping() { return u_tfxShape.x; }


//
// Global Shape Constraints (GSC)

// AMD:
// If the forces/gravity are not strong enough to overcome
// this, the strands will not move.
//
// TL;DR: 'Nudge' final position toward initial position by 'that much'.
float GetGlobalStiffness() { return u_tfxShape.z; }

// AMD:
// By default, Global Shape Constraints affect only `global_range * vertices_in_strand`
// vertices:
//   * globalRange == 0.0, only root affected by GSC
//   * globalRange == 0.5, only half of strand (near tip)
//       is affected by forces/gravity
//   * globalRange == 1.0, then whole strand is affected by forces/gravity
// Also known as 'globalShapeMatchingEffectiveRange'
//
// TL;DR: If the number is small, the tips will be very 'bouncy' (it will affect only near root).
// If it's high, the hair will be 'static'.
float GetGlobalRange() { return u_tfxShape.w; }



//
// Local Shape Constraints

// * stiffness == 0, then no local shape preservation
// * stiffness == 1, then ignore forces/gravity VSP etc.
float GetLocalStiffness() { return u_tfxShape.y; }



//
// Length Constraints

int GetLengthConstraintIterations() {
  return max(0, readConfigInt(u_tfxConstraints.x));
}
