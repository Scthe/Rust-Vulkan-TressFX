#version 450
precision highp float;
precision highp int;
precision highp usampler2D;
// precision highp sampler2D;


//@import ./_config_ubo;
//@import ./_forward_model_ubo;
//@i mport ./_forward_model_per_frame_ubo;


// material
layout(binding = 2) uniform sampler2D u_albedoTexture;
layout(binding = 3) uniform sampler2D u_specularTexture;
// layout(binding = 5) uniform sampler2D u_hairShadowTexture;
// layout(binding = 6) uniform sampler2D u_sssDepthTex;
// ao
// layout(binding = 7) uniform sampler2D u_aoTex;
// Shadow
// layout(binding = 8) uniform sampler2D u_directionalShadowDepthTex;


// input-output variables
layout(location = 0) in vec3 v_Position;
layout(location = 1) in vec3 v_Normal;
layout(location = 2) in vec2 v_UV;
// layout(location = 3) in vec4 v_PositionLightShadowSpace;

layout(location = 0) out vec4 outColor1;
// layout(location = 1) out vec4 outColor2; // TODO normals


// required by SSSSS import, but not used here (used in SSS blur)
float SSSSS_sampleDepthLinear (sampler2D depthTex, vec2 texcoord) {
  return 0.0;
}



//@import ./_utils;
//@import ./_material;
//@import ./_pbr;
//@i mport ./_skin; // not imported even in WebFx?
//@i mport ./_shadows;
const float IN_SHADOW = 1.0f;
const float NOT_IN_SHADOW = 0.0f;
// #define SSSS_GLSL_3 1
//@i mport ./_separableSSSSS;


const int FLAG_IS_METALIC = 1;
const int FLAG_USE_SPECULAR_TEXTURE = 2;
const int FLAG_USE_HAIR_SHADOW_TEXTURE = 4;


vec3 readModelTexture(sampler2D tex, vec2 coords) {
  coords = fixOpenGLTextureCoords_AxisY(coords);
  return texture(tex, coords).rgb; // as uint [0-255]
}

float readSpecular() {
  // we are going to pretend that specular is same as smoothness. Probably is not, but..
  if (isFlag(u_materialFlags, FLAG_USE_SPECULAR_TEXTURE)) {
    return readModelTexture(u_specularTexture, v_UV).r;
  } else {
    return u_specular;
  }
}

/*
TODO float readHairShadow() {
  if (isFlag(u_materialFlags, FLAG_USE_HAIR_SHADOW_TEXTURE)) {
    // special code for this demo
    // the texture is square, so we have to adjust UVs
    vec2 adjustedUV = vec2(v_UV.x * 2.0 - 1.0, v_UV.y);
    if (outOfScreen(adjustedUV)) {
      return NOT_IN_SHADOW;
    }
    float hairShadowVal = readModelTexture(u_hairShadowTexture, adjustedUV).r;
    return hairShadowVal;
  } else {
    return NOT_IN_SHADOW;
  }
}
*/


Material createMaterial() {
  Material material;
  material.normal = v_Normal;
  material.toEye = normalize(u_cameraPosition - v_Position);
  material.albedo = readModelTexture(u_albedoTexture, v_UV);
  material.positionWS = v_Position;
  material.isMetallic = isFlag(u_materialFlags, FLAG_IS_METALIC) ? 1.0 : 0.0;
  material.specularMul = u_specularMul;
  material.ao = 1.0f; // TODO texture(u_aoTex, gl_FragCoord.xy / u_viewport).r;
  // convert specular/smoothness -> roughness
  material.roughness = 1.0 - readSpecular();

  /* TODO restore
  vec3 toCaster = normalize(u_directionalShadowCasterPosition.xyz - v_Position);
  material.shadow = 1.0 - calculateDirectionalShadow(
    v_PositionLightShadowSpace, material.normal, toCaster
  );
  material.hairShadow = 1.0 - readHairShadow();*/
  material.shadow = NOT_IN_SHADOW;
  material.hairShadow = NOT_IN_SHADOW;

  return material;
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

  /*
  // TODO ambient occlusion
  // not PBR, I know, but we need this to highlight some details like collarbones etc.
  float aoRadianceFactor = getCustom_AO(material.ao, u_aoStrength, u_aoExp);
  radianceSum *= aoRadianceFactor;

  // TODO add SSSSS forward scattering - transluency
  vec3 sssL = normalize(u_sssPosition - material.positionWS);
  vec3 contribSSS = SSSSTransmittance(
    u_sssTransluency, // float translucency,
    u_sssWidth, // float sssWidth,
    material.positionWS, // float3 worldPosition,
    material.normal, // float3 worldNormal,
    sssL, // float3 light,
    u_sssDepthTex, // SSSSTexture2D shadowMap,
    u_sssMatrix_VP, // float4x4 lightViewProjection,
    u_sssFarPlane, // float lightFarPlane
    u_sssBias, u_sssGain
  );
  contribSSS = contribSSS * radianceSum * u_sssStrength;

  // TODO add shadow, combine
  float shadow = min(material.shadow, material.hairShadow);
  radianceSum = radianceSum * clamp(shadow, 1.0 - u_maxShadowContribution, 1.0);
  return ambient + radianceSum + contribSSS;
  */

  return ambient + radianceSum;
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

  outColor1 = vec4(color, 1.0);
  // outColor2 = vec4(to_0_1(material.normal), 1.0);
}