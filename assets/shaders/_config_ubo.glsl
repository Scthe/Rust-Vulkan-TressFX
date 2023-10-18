layout(binding = 0) 
uniform GlobalConfigUniformBuffer {
  mat4 u_MVP;
  vec3 u_cameraPosition;
  vec2 u_viewport;
  // ao
  // float u_aoStrength;
  // float u_aoExp;
  // Shadow
  // vec4 u_directionalShadowCasterPosition; // [position.xyz, bias (negative if pcss)]
  // int u_directionalShadowSampleRadius;
  // float u_maxShadowContribution;
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
};


// #define BIAS_FROM_UI (u_directionalShadowCasterPosition.w)
// #define USE_PCSS_SHADOWS (u_directionalShadowCasterPosition.w < 0.0f)
