// NOTE: rust packing has problems with raw floats (packing?) - use vec4

layout(binding = 0) 
uniform GlobalConfigUniformBuffer {
  vec3 u_cameraPosition;
  vec4 u_viewportAndNearFar;
  mat4 u_viewMat;
  mat4 u_projection;
  mat4 u_invProjectionMat; // inverse projection matrix
  // AO + Shadow
  mat4 u_directionalShadowMatrix_VP;
  vec4 u_shadowMiscSettings; // [u_directionalShadowSampleRadius, -, -, -]
  vec4 u_directionalShadowCasterPosition; // [position.xyz, bias (negative if pcss)]
  vec4 u_aoAndShadowContrib; // (u_aoStrength, u_aoExp, u_maxShadowContribution, u_directionalShadowSampleRadius)
  // sss
  // float u_sssFarPlane;
  // mat4 u_sssMatrix_VP;
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

#define u_viewport (u_viewportAndNearFar.xy)
#define u_nearAndFar (u_viewportAndNearFar.zw)
// AO + shadows
#define u_directionalShadowSampleRadius (readConfigUint(u_shadowMiscSettings.x))
#define u_shadowBias (abs(u_directionalShadowCasterPosition.w))
#define u_usePCSS_Shadows (u_directionalShadowCasterPosition.w < 0.0f)
#define u_aoStrength (u_aoAndShadowContrib.r)
#define u_aoExp (u_aoAndShadowContrib.g)
#define u_maxShadowContribution (u_aoAndShadowContrib.b)
// fxaa
#define u_subpixel (u_fxaaSettings.x)
#define u_edgeThreshold (u_fxaaSettings.y)
#define u_edgeThresholdMin (u_fxaaSettings.z)
#define u_fxaa_luma_gamma (u_fxaaSettings.w)
// TONEMAPPING
#define u_exposure (u_tonemapping.x)
#define u_whitePoint (u_tonemapping.y)
#define u_acesC (u_tonemapping.z)
#define u_acesS (u_tonemapping.w)
#define u_ditherStrength (u_tonemapping2.x)
#define u_tonemappingMode (readConfigUint(u_tonemapping2.y))
#define u_colorCorrectionShadowsMax (u_tonemapping2.z)
#define u_colorCorrectionHighlightsMin (u_tonemapping2.w)
// SSAO
#define u_noiseScale (u_ssao.xy)
#define u_radius (u_ssao.z)
#define u_bias (u_ssao.w)
#define u_kernelSize (readConfigUint(u_ssao_and_misc.x))
#define u_linear_depth_preview_range (u_ssao_and_misc.yz)

uint readConfigUint(float value) {
  return uint(abs(value) + 0.5);
}