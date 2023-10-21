layout(binding = 0) 
uniform GlobalConfigUniformBuffer {
  vec3 u_cameraPosition;
  vec4 u_viewportAndNearFar;
  // ao
  // float u_aoStrength;
  // float u_aoExp;
  // combined as rust packing has problems with raw floats?
  vec4 u_aoAndShadowContrib;
  // Shadow
  // vec4 u_directionalShadowCasterPosition; // [position.xyz, bias (negative if pcss)]
  // int u_directionalShadowSampleRadius;
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
  // FXAA
  vec3 u_fxaaSettings;
};

#define u_viewport (u_viewportAndNearFar.xy)
#define u_nearAndFar (u_viewportAndNearFar.zw)
#define u_aoStrength (u_aoAndShadowContrib.r)
#define u_aoExp (u_aoAndShadowContrib.g)
#define u_maxShadowContribution (u_aoAndShadowContrib.b)
// #define BIAS_FROM_UI (u_directionalShadowCasterPosition.w)
// #define USE_PCSS_SHADOWS (u_directionalShadowCasterPosition.w < 0.0f)
#define u_subpixel (u_fxaaSettings.x)
#define u_edgeThreshold (u_fxaaSettings.y)
#define u_edgeThresholdMin (u_fxaaSettings.z)
