// NOTE: rust packing has problems with raw floats (packing?) - use vec4

const uint DISPLAY_MODE_FINAL = 0;
const uint DISPLAY_MODE_NORMALS = 1;
const uint DISPLAY_MODE_LUMA = 2;
const uint DISPLAY_MODE_SSAO = 3;
const uint DISPLAY_MODE_LINEAR_DEPTH = 4;
const uint DISPLAY_MODE_SHADOW_MAP = 5;
const uint DISPLAY_MODE_SSS_SCATTERING = 6;
const uint DISPLAY_MODE_SSS_THICKNESS = 7;


layout(binding = 0) 
uniform GlobalConfigUniformBuffer {
  vec4 u_cameraPositionAndDisplayMode; // [cameraPosition.xyz, u_displayMode]
  vec4 u_viewportAndNearFar; // [viewport.w,viewport.h, near,far]
  mat4 u_viewMat;
  mat4 u_projectionMat;
  mat4 u_invProjectionMat; // inverse projection matrix
  mat4 u_viewProjectionMat;
  vec4 u_tfxHairSettings; // [hairDisplayMode, u_tfxLinkedListPoolSize, -, -]
  // AO + Shadow
  mat4 u_directionalShadowMatrix_VP;
  vec4 u_shadowRadiusAndBias; // [u_shadowRadiusForwardShading, u_shadowBiasForwardShading, u_shadowRadiusTfx, u_shadowBiasTfx]
  vec4 u_directionalShadowCasterPosition; // [position.xyz, u_maxShadowContribution]
  vec4 u_aoSettings; // (u_aoStrength, u_aoExp, showDebugPositions+u_maxShadowContribution, -)
  // sss
  vec4 u_sssSettings; // [u_sssPosition, u_sssFarPlane]
  mat4 u_sssMatrix_VP;
  vec4 u_sssBlur; // [u_sssWidth, u_sssStrength, u_sssFovy+u_sssFollowSurface, -]
  // Lights
  vec4 u_lightAmbient;
  vec3 u_light0_Position;
  vec4 u_light0_Color;
  vec3 u_light1_Position;
  vec4 u_light1_Color;
  vec3 u_light2_Position;
  vec4 u_light2_Color;
  // SSAO
  vec4 u_ssao;
  vec4 u_ssao_and_misc;
  // FXAA
  vec4 u_fxaaSettings;
  // Color correction
  vec4 u_tonemapping;
  vec4 u_tonemapping2;
  // TONEMAPPING
  // vec2 u_colorCorrectionSettings;
  vec4 u_colorSaturation; // general
  vec4 u_colorContrast;
  vec4 u_colorGamma;
  vec4 u_colorGain;
  vec4 u_colorOffset;
  vec4 u_colorSaturationShadows; // shadows
  vec4 u_colorContrastShadows;
  vec4 u_colorGammaShadows;
  vec4 u_colorGainShadows;
  vec4 u_colorOffsetShadows;
  vec4 u_colorSaturationMidtones; // midtones
  vec4 u_colorContrastMidtones;
  vec4 u_colorGammaMidtones;
  vec4 u_colorGainMidtones;
  vec4 u_colorOffsetMidtones;
  vec4 u_colorSaturationHighlights; // highlights
  vec4 u_colorContrastHighlights;
  vec4 u_colorGammaHighlights;
  vec4 u_colorGainHighlights;
  vec4 u_colorOffsetHighlights;
};

// u_cameraPositionAndDisplayMode
#define u_cameraPosition (u_cameraPositionAndDisplayMode.xyz)
#define u_displayMode (readConfigUint(u_cameraPositionAndDisplayMode.w))
// u_viewportAndNearFar
#define u_viewport (u_viewportAndNearFar.xy)
#define u_nearAndFar (u_viewportAndNearFar.zw)
#define u_tfxDisplayMode (readConfigUint(u_tfxHairSettings.x))
#define u_tfxLinkedListPoolSize (readConfigUint(u_tfxHairSettings.y))

// Shadows
#define u_shadowRadiusForwardShading (readConfigInt(u_shadowRadiusAndBias.x))
#define u_shadowBiasForwardShading (u_shadowRadiusAndBias.y)
#define u_shadowRadiusTfx (readConfigInt(u_shadowRadiusAndBias.z))
#define u_shadowBiasTfx (u_shadowRadiusAndBias.w)
#define u_shadowsTechnique (readConfigUint(u_directionalShadowCasterPosition.w))

// AO + misc
#define u_aoStrength (u_aoSettings.r)
#define u_aoExp (u_aoSettings.g)
#define u_showDebugPositions (readConfigFlagFromSign(u_aoSettings.b))
#define u_maxShadowContribution (readConfigValueFromValueWithFlag(u_aoSettings.b))

// SSS (u_sssSettings)
#define u_sssPosition (u_sssSettings.xyz)
#define u_sssFarPlane (u_sssSettings.w)
// u_sssBlur
#define u_sssBlurWidth (u_sssBlur.x)
#define u_sssBlurStrength (u_sssBlur.y) // SSSS_STREGTH_SOURCE
#define u_sssBlurFovy (readConfigValueFromValueWithFlag(u_sssBlur.z)) // SSSS_FOVY 20.0
#define u_sssBlurFollowSurface (readConfigFlagFromSign(u_sssBlur.z) ? 1 : 0) // SSSS_FOLLOW_SURFACE: 0 or 1
// fxaa (u_fxaaSettings)
#define u_subpixel (u_fxaaSettings.x)
#define u_edgeThreshold (u_fxaaSettings.y)
#define u_edgeThresholdMin (u_fxaaSettings.z)
#define u_fxaa_luma_gamma (u_fxaaSettings.w)

// TONEMAPPING
// u_tonemapping
#define u_exposure (u_tonemapping.x)
#define u_whitePoint (u_tonemapping.y)
#define u_acesC (u_tonemapping.z)
#define u_acesS (u_tonemapping.w)
// u_tonemapping2
#define u_ditherStrength (u_tonemapping2.x)
#define u_tonemappingMode (readConfigUint(u_tonemapping2.y))
#define u_colorCorrectionShadowsMax (u_tonemapping2.z)
#define u_colorCorrectionHighlightsMin (u_tonemapping2.w)

// SSAO
// u_ssao
#define u_noiseScale (u_ssao.xy)
#define u_radius (u_ssao.z)
#define u_bias (u_ssao.w)
// u_ssao_and_misc
#define u_kernelSize (readConfigUint(u_ssao_and_misc.x))
#define u_linear_depth_preview_range (u_ssao_and_misc.yz)

uint readConfigUint(float value) { return uint(abs(value) + 0.5); }
int  readConfigInt (float value) { return int(abs(value) + 0.5); }

bool readConfigFlagFromSign(float value) {
  return value < 0.0f;
}

float readConfigValueFromValueWithFlag(float value) {
  return abs(value);
}