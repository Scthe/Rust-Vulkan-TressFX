#pragma include ./_material; // for light struct
#pragma include ./_shadows;
#pragma include ./_kajiyakay;

float calculateHairShadow (
  sampler2D directionalShadowDepthTex,
  vec3 positionWorld,
  vec3 normal,
  vec4 positionShadowProjected
) {
  vec3 positionShadowCaster = u_directionalShadowCasterPosition.xyz;
  vec3 toCaster = normalize(positionShadowCaster - positionWorld);
  vec3 normal2 = normalize(normal); // TODO use tangent per http://developer.amd.com/wordpress/media/2012/10/Scheuermann_HairRendering.pdf s7?
  return 1.0 - calculateDirectionalShadow(
    directionalShadowDepthTex,
    positionShadowProjected, normal2, toCaster,
    u_shadowBiasTfx,
    u_shadowRadiusTfx
  );
}

float calculateHairAO(sampler2D aoTex) {
  return texture(aoTex, gl_FragCoord.xy / u_viewport).r;
}


KajiyaKayParams createKajiyakayParams(
  vec3 positionWorld,
  vec3 normal,
  vec3 tangent
) {
  KajiyaKayParams params;
  params.V = normalize(u_cameraPosition - positionWorld); // viewDir
  params.T = tangent; // tangentDir
  params.N = normal; // normalDir
  // params.L // filled later

  params.shift = 0.0; // TODO
  params.primaryShift = u_primaryShift;
  params.secondaryShift = u_secondaryShift;
  params.specularPower1 = u_specularPower1;
  params.specularPower2 = u_specularPower2;
  return params;
}


vec3 doHairShading(
  Light lights[3],
  float ao,
  float shadow,
  vec3 positionWorld,
  vec3 normal, // assumed normalized
  vec3 tangent // assumed normalized
) {
  vec3 ambient = u_lightAmbient.rgb * u_lightAmbient.a;
  vec3 radianceSum = vec3(0.0);
  KajiyaKayParams params = createKajiyakayParams(
    positionWorld, normal, tangent
  );

  for (uint i = 0u; i < 3u; i++) {
    Light light = lights[i];
    vec3 L = normalize(light.position - positionWorld); // wi in integral
    // float NdotL = dotMax0(normalize(normal), L); // no, cause it's hair
    float NdotL = dotMax0(tangent, L);
    vec3 radiance = light.color * light.intensity; // incoming color from light

    // specular
    params.L = L;
    vec2 specularHighlight = kajiyakay(params);
    vec3 specular1 = specularHighlight.x * u_specularColor1 * u_specularStrength1;
    vec3 specular2 = specularHighlight.y * u_specularColor2 * u_specularStrength2;

    // combine
    // NOTE: this is different then usual Kajiya-Kay, I like it more
    vec3 fr = TfxParamsUbo.u_albedo.rgb * NdotL + specular1 + specular2;
    radianceSum += fr * radiance;

    // debug:
    // radianceSum += u_albedo * NdotL * radiance;
    // radianceSum += NdotL;
    // radianceSum += specularHighlight.x;
    // radianceSum += specularHighlight.y;
    // radianceSum += specular1;
    // radianceSum += specular2;
    // radianceSum += specular1 + specular2;
  }

  // ambient occlusion
  float aoRadianceFactor = getCustom_AO(ao, u_tfxAoStrength, u_tfxAoExp);
  radianceSum *= aoRadianceFactor;
  ambient *= aoRadianceFactor;

  float shadowContrib = clamp(shadow, 0.0, u_maxShadowContribution);
  radianceSum = radianceSum * (1.0 - shadowContrib);
  return ambient + radianceSum;
}

vec3 debugHairFlatColor(){ return vec3(0.8); }