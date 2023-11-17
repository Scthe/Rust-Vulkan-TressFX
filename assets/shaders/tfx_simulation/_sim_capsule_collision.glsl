bool CapsuleCollision(vec4 curPosition, inout vec3 newPosition, vec4 capsule) {
  const float radius1 = capsule.w;
  newPosition = curPosition.xyz;

  if (!sharedPosIsMovable(curPosition)) {
    return false;
  }

  vec3 delta1 = capsule.xyz - curPosition.xyz;
  if (dot(delta1, delta1) < radius1 * radius1) {
    // inside capsule - move to outer shell
    vec3 n = normalize(-delta1); // 'normal': center->impact position
    newPosition = capsule.xyz + radius1 * n;
    return true;
  }

  return false;
}

// Resolve hair vs capsule collisions.
bool ResolveCapsuleCollisions(inout vec4 curPosition, vec4 oldPos) {
  bool bAnyColDetected = false;
  vec3 newPos;

  for (int i = 0; i < NUM_COLLISION_CAPSULES; i++) {
    bool bColDetected = CapsuleCollision(curPosition, newPos, g_Capsules[i]);
    bAnyColDetected = bColDetected || bAnyColDetected;

    if (bColDetected) {
      curPosition.xyz = newPos;
    }
  }

  return bAnyColDetected;
}
