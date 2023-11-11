#version 450
precision highp float;
precision highp int;
precision highp usampler2D;
// precision highp sampler2D;


#pragma include ./_config_ubo;
#pragma include ./_forward_model_ubo;
// #pragma include ./_forward_model_per_frame_ubo;


// material
layout(binding = 2) uniform sampler2D u_albedoTexture;
layout(binding = 3) uniform usampler2D u_specularTexture;
layout(binding = 4) uniform usampler2D u_hairShadowTexture;
layout(binding = 5) uniform sampler2D u_directionalShadowDepthTex;
layout(binding = 6) uniform sampler2D u_sssDepthTex;
layout(binding = 7) uniform sampler2D u_aoTex;


// input-output variables
layout(location = 0) in vec3 v_Position;
layout(location = 1) in vec3 v_Normal;
layout(location = 2) in vec2 v_UV;
layout(location = 3) in vec4 v_PositionLightShadowSpace;

layout(location = 0) out vec4 outColor1;
layout(location = 1) out uvec4 outColor2;


// required by SSSSS import, but not used here (used in SSS blur)
float SSSSS_sampleDepthLinear (sampler2D depthTex, vec2 texcoord) {
  return 0.0;
}



#pragma include ./_utils;
#pragma include ./materials/_material;
#pragma include ./materials/_pbr;
#pragma include ./materials/_shadows;
#define SSSS_GLSL_3 1
#pragma include ./materials/_separableSSSSS;
// #pragma include ./_skin; // not imported even in WebFx?


const int FLAG_IS_METALIC = 1;
const int FLAG_USE_SPECULAR_TEXTURE = 2;
const int FLAG_USE_HAIR_SHADOW_TEXTURE = 4;


float readSpecular() {
  // we are going to pretend that specular is same as smoothness. Probably is not, but..
  if (isFlag(u_materialFlags, FLAG_USE_SPECULAR_TEXTURE)) {
    return readModelTexture_uint(u_specularTexture, v_UV).r;
  } else {
    return u_specular;
  }
}

float readHairShadow() {
  if (isFlag(u_materialFlags, FLAG_USE_HAIR_SHADOW_TEXTURE)) {
    // special code for this demo
    // the texture is square, so we have to adjust UVs
    vec2 adjustedUV = vec2(v_UV.x * 2.0 - 1.0, v_UV.y);
    if (outOfScreen(adjustedUV)) {
      return NOT_IN_SHADOW;
    }
    return readModelTexture_uint(u_hairShadowTexture, adjustedUV).r;
  } else {
    return NOT_IN_SHADOW;
  }
}


Material createMaterial() {
  Material material;
  material.normal = normalize(v_Normal); // normalize here as it was interpolated between 3 vertices and is no longer normalized
  material.toEye = normalize(u_cameraPosition - v_Position);
  material.albedo = readModelTexture_srgb(u_albedoTexture, v_UV);
  material.positionWS = v_Position;
  material.isMetallic = isFlag(u_materialFlags, FLAG_IS_METALIC) ? 1.0 : 0.0;
  material.specularMul = u_specularMul;
  material.ao = texture(u_aoTex, gl_FragCoord.xy / u_viewport).r;
  // convert specular/smoothness -> roughness
  material.roughness = 1.0 - readSpecular();

  vec3 toCaster = normalize(u_directionalShadowCasterPosition.xyz - v_Position);
  material.shadow = 1.0 - calculateDirectionalShadow(
    u_directionalShadowDepthTex,
    v_PositionLightShadowSpace, material.normal, toCaster,
    u_shadowBiasForwardShading,
    u_shadowRadiusForwardShading
  );
  material.hairShadow = readHairShadow();

  return material;
}

vec4 calculateSSSForwardScattering(Material material) {
  vec3 sssL = normalize(u_sssPosition - material.positionWS);
  return SSSSTransmittance(
    u_sssTransluency, // float translucency,
    u_sssWidth, // float sssWidth,
    material.positionWS, // float3 worldPosition,
    material.normal, // float3 worldNormal,
    sssL, // float3 light,
    u_sssDepthTex, // SSSSTexture2D shadowMap, linear cause ortho projection
    u_sssMatrix_VP, // float4x4 lightViewProjection,
    u_sssFarPlane, // float lightFarPlane
    u_sssBias, u_sssGain
  );
}

vec3 doShading(Material material, Light lights[3]) {
  vec3 ambient = u_lightAmbient.rgb * u_lightAmbient.a * material.ao;
  vec3 radianceSum = vec3(0.0);

  for (uint i = 0u; i < 3u; i++) {
    Light light = lights[i];

    vec3 contrib = pbr(material, light);

    /* // OR instead of PBR:
    vec3 L = normalize(light.position - material.positionWS); // wi in integral
    float NdotL = dotMax0(material.normal, L);
    vec3 radiance = light.color * light.intensity; // incoming color from light
    vec3 contrib = material.albedo * radiance * NdotL;
    */

    radianceSum += contrib;
  }

  // not PBR, but we need this to highlight some details like collarbones etc.
  float aoRadianceFactor = getCustom_AO(material.ao, u_aoStrength, u_aoExp);
  radianceSum *= aoRadianceFactor;

  vec4 contribSSS = calculateSSSForwardScattering(material);
  vec3 sssForwardScattering = contribSSS.rgb * radianceSum * u_sssStrength;

  float shadow = max(material.shadow, material.hairShadow);
  float shadowContrib = clamp(shadow, 0.0, u_maxShadowContribution);
  radianceSum = radianceSum * (1.0 - shadowContrib);
  return ambient + radianceSum + sssForwardScattering;
}

vec4 debugModeOverride(Material material, vec3 shadingResult){
  vec4 result = vec4(0);

  switch(u_displayMode) {
    case DISPLAY_MODE_SHADOW_MAP: {
      vec3 c = mix(shadingResult, vec3(1 - material.shadow), 0.8);
      result = vec4(c, 1);
      break;
    }
    case DISPLAY_MODE_SSS_SCATTERING: {
      vec4 sss = calculateSSSForwardScattering(material);
      result = vec4(sss.rgb, 1);
      break;
    }
    case DISPLAY_MODE_SSS_THICKNESS: {
      vec4 sss = calculateSSSForwardScattering(material);
      result = vec4(vec3(sss.a), 1);
      break;
    }
  }

  return result;
}

void main() {
  Light lights[3];
  lights[0] = unpackLight(u_light0_Position, u_light0_Color);
  lights[1] = unpackLight(u_light1_Position, u_light1_Color);
  lights[2] = unpackLight(u_light2_Position, u_light2_Color);

  vec3 color;
  Material material = createMaterial();
  // SkinParams skinParams = createSkinParams();
  // material.skin = skinShader(material, skinParams);
  color = doShading(material, lights);

  vec4 colorDebug = debugModeOverride(material, color);
  color = mix(color, colorDebug.rgb, colorDebug.a);
  outColor1 = vec4(color, 1.0);
  // outColor2 = vec4(packNormal(material.normal), 1.0);
  outColor2 = uvec4(packNormal(material.normal), 255);
  
  /* DEBUG:
  // vec3 n = vec3(0.0, 0.5, 1.0);
  // vec3 n = material.normal;
  vec3 toCaster = normalize(u_directionalShadowCasterPosition.xyz - v_Position);
  vec4 positionShadowSpace = u_directionalShadowMatrix_MVP * vec4(v_Position, 1);
  float shadowSim = shadowTestSimple(positionShadowSpace, material.normal, toCaster);
  color = mix(
    material.albedo,
    vec3(shadowSim),
    0.3
  );  
  // outColor1 = vec4(color, 1.0);
  // outColor1 = vec4(vec3(material.shadow), 1.0);

  // outColor2 = vec4(n, 1.0);
  // outColor2 = uvec4(0, 128, 255, 255);
  // outColor2 = vec4(to_0_1(n), 1.0);
  // outColor2 = vec4(abs(material.normal), 1.0);
  // outColor2 = vec4(abs(normalize(v_Normal)), 1.0);
  // outColor2 = vec4(v_Normal, 1.0);
  // outColor2 = vec4(v_Normal, 1.0);
  */
}