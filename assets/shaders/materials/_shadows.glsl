/** returns inShadow(1.0) or notInShadow(0.0) */
/*
float calculateDirectionalShadow(vec4 lightPosInterp, vec3 normal, vec3 toShadowCaster) {
  vec3 lightPosProj = lightPosInterp.xyz / lightPosInterp.w; // not build-in gl_Position. Useless for ORTHO, only PERSP.
  lightPosProj = lightPosProj * 0.5 + 0.5; // from opengl [-1, 1] to depth-texture-like [0..1]

  // depth from shadow map
  float shadowMapDepth = texture(u_directionalShadowDepthTex, lightPosProj.xy).r;
  // depth of current fragment (we multiplied by light-shadow matrix
  // in vert. shader, did w-divide here)
  float fragmentDepth = lightPosProj.z;

  // GDC_Poster_NormalOffset.png
  float bias = max(0.05 * (1.0 - dot(normal, toShadowCaster)), 0.005);

  // There are following cases:
  //  * fragmentDepth > shadowMapDepth
  //      there exist some object that is closer to shadow source than object
  //      Means object is IN SHADOW
  //  * fragmentDepth == shadowMapDepth
  //      this is the object that casts the shadow
  //      Means NO SHADOW
  //  * fragmentDepth < shadowMapDepth
  //      would probably happen if object is not shadow-caster
  //      Means NO SHADOW
  return fragmentDepth - bias > shadowMapDepth  ? 1.0 : 0.0;
}
*/

const uint SHADOW_TECHNIQUE_BINARY_DEBUG = 0;
const uint SHADOW_TECHNIQUE_PFC = 1;
const uint SHADOW_TECHNIQUE_PCSS = 2;

const float IN_SHADOW = 1.0f;
const float NOT_IN_SHADOW = 0.0f;

// settings
const float PCSS_PENUMBRA_WIDTH = 10.0;
const int PCSS_PENUMBRA_BASE = 1; // we want at least some blur

/** Simplest possible shadow implementation - just a binary in shadow or not */
float shadowTestSimple(
  sampler2D directionalShadowDepthTex,
  vec4 lightPosInterp, vec3 normal, vec3 toShadowCaster, float shadowBiasFromUi
) {
  // position of fragment as rendered from light POV
  vec3 lightPosProj = lightPosInterp.xyz / lightPosInterp.w; // Useless for ORTHO, only PERSP.
  vec2 uv = to_0_1(lightPosProj.xy); // from opengl [-1, 1] to depth-texture-like [0..1]
  float fragmentDepth = lightPosProj.z;
  uv.y = 1.0 - uv.y;

  // Special case if we went beyond the far plane of the frustum.
  // Mark no shadow, cause it's better than dark region
  // far away (or whatever relative light-camera postion is)
  if (fragmentDepth > 1.0) {
    return NOT_IN_SHADOW;
    // return vec3(1, 0, 0); // debug: red
  }
  // would cause 'invalid' sampling, mark as no shadow too.
  if (outOfScreen(uv.xy)) {
    return NOT_IN_SHADOW;
    // return vec3(0, 1, 0); // debug: green
  }

  // GDC_Poster_NormalOffset.png
  float bias = max(shadowBiasFromUi * (1.0 - dot(normal, toShadowCaster)), 0.005);

  float shadowMapDepth = texture(directionalShadowDepthTex, uv).r;
  float shadow = fragmentDepth - bias > shadowMapDepth  ? IN_SHADOW : NOT_IN_SHADOW;
  // return vec3(0, 0, shadow); // debug: blue
  return 1.0 - shadow;
}


////////////////

float sampleShadowMap (
  sampler2D directionalShadowDepthTex,
  int sampleRadius, vec3 lightPosProj, float bias
) {
  // depth of current fragment (we multiplied by light-shadow matrix
  // in vert. shader, did w-divide here)
  float fragmentDepth = lightPosProj.z;

  float shadow = 0.0;
  vec2 texelSize = 1.0 / vec2(textureSize(directionalShadowDepthTex, 0));

  for (int x = -sampleRadius; x <= sampleRadius; ++x) {
    for (int y = -sampleRadius; y <= sampleRadius; ++y) {
      // depth from shadow map
      float shadowMapDepth = texture(
        directionalShadowDepthTex,
        lightPosProj.xy + vec2(x, y) * texelSize
      ).r;

      // There are following cases:
      //  * fragmentDepth > shadowMapDepth
      //      there exist some object that is closer to shadow source than object
      //      Means object is IN SHADOW
      //  * fragmentDepth == shadowMapDepth
      //      this is the object that casts the shadow
      //      Means NO SHADOW
      //  * fragmentDepth < shadowMapDepth
      //      would probably happen if object is not shadow-caster
      //      Means NO SHADOW
      shadow += fragmentDepth - bias > shadowMapDepth  ? IN_SHADOW : NOT_IN_SHADOW;
    }
  }

  float pcfTmp = float(sampleRadius * 2 + 1);
  return shadow /= pcfTmp * pcfTmp;
}

float calculateDirectionalShadow(
  sampler2D directionalShadowDepthTex,
  vec4 lightPosInterp, 
  vec3 normal, 
  vec3 toShadowCaster,
  float shadowBiasFromUi,
  int sampleRadius
) {
  // position of fragment as rendered from light POV
  vec3 lightPosProj = lightPosInterp.xyz / lightPosInterp.w; // Useless for ORTHO, only PERSP.
  lightPosProj = vec3(to_0_1(lightPosProj.xy), lightPosProj.z); // from opengl [-1, 1] to depth-texture-like [0..1]
  lightPosProj.y = 1.0 - lightPosProj.y;

  // Special case if we went beyond the far plane of the frustum.
  // Mark no shadow, cause it's better than dark region
  // far away (or whatever relative light-camera postion is)
  if (lightPosProj.z > 1.0) {
    return NOT_IN_SHADOW;
  }
  // would cause 'invalid' sampling, mark as no shadow too.
  if (outOfScreen(lightPosProj.xy)) {
    return NOT_IN_SHADOW;
  }

  // GDC_Poster_NormalOffset.png
  float bias = max(shadowBiasFromUi * (1.0 - dot(normal, toShadowCaster)), 0.005);

  switch(u_shadowsTechnique) {
    case SHADOW_TECHNIQUE_BINARY_DEBUG: {
      // discard variables above and use simplest possible way
      return shadowTestSimple(
        directionalShadowDepthTex,
        lightPosInterp, normal, toShadowCaster, shadowBiasFromUi
      );
    }
    case SHADOW_TECHNIQUE_PFC: {
      return 1.0 - sampleShadowMap(directionalShadowDepthTex, sampleRadius, lightPosProj, bias);
    }
    default:
    case SHADOW_TECHNIQUE_PCSS: {
      float fragmentDepth = lightPosProj.z; // current fragment - depth buffer from light position
      float shadowMapDepth = texture(directionalShadowDepthTex, lightPosProj.xy).r; // sample center

      // Subtraction is reversed for RH coordinate system.
      // Fine for ortho projection.
      float depthDiff = max(shadowMapDepth - fragmentDepth, 0.0);

      int sampleRadius = PCSS_PENUMBRA_BASE + int(depthDiff / shadowMapDepth * PCSS_PENUMBRA_WIDTH);
      return 1.0 - sampleShadowMap(
        directionalShadowDepthTex, sampleRadius, lightPosProj, bias
      );
    }
  }
}
