#version 450

precision highp float;
precision highp int;
precision highp sampler2D;


layout(binding = 1)
uniform sampler2D u_sceneDepthTex; // in perpective projection
layout(binding = 2)
uniform usampler2D u_normalTex;
layout(binding = 3)
uniform sampler2D u_noiseTex;
layout(binding = 4) 
uniform RandomSampleVectorsKernel {
  vec3 u_kernel[256];
};


layout(location = 0) in vec2 v_position;
layout(location = 0) out vec4 outColor1;


//@import ./_config_ubo;
//@import ./_utils;


vec3 positionVS_FromCoords(in vec2 texCoord) {
  float depth = texture(u_sceneDepthTex, texCoord).r;
  vec3 texSpace = vec3(texCoord, depth);
  vec4 clipSpace = vec4(to_neg1_1(texSpace), 1);
  vec4 viewPos = u_invProjectionMat * clipSpace;
  return viewPos.xyz / viewPos.w;
}


// @see https://learnopengl.com/Advanced-Lighting/SSAO
void main() {
  vec3 fragPosVS = positionVS_FromCoords(v_position);

  // vec3 normalWS = texture(u_normalTex, v_position).rgb;
  // normalWS = normalize(to_neg1_1(normalWS));
  vec3 normalWS = unpackNormal(u_normalTex, v_position);
  vec3 normalVS = (u_viewMat * vec4(normalWS, 1.0)).xyz;
  normalVS = normalize(normalVS);

  vec3 randomVec = texture(u_noiseTex, v_position * u_noiseScale).xyz;
  randomVec = normalize(randomVec);

  // Gram-Schmidt process
  // @see http://en.wikipedia.org/wiki/Gram%E2%80%93Schmidt_process
  vec3 tangent = normalize(randomVec - normalVS * dot(randomVec, normalVS));
  vec3 bitangent = cross(normalVS, tangent);
  mat3 TBN = mat3(tangent, bitangent, normalVS);

  float occlusion = 0.0;
  for(int i = 0; i < u_kernelSize; i++) {
    float radius = u_radius; // TODO make depth-independent

    // get sample position
    vec3 sampleVS = TBN * u_kernel[i]; // From tangent to view-space
    sampleVS = fragPosVS + sampleVS * radius;

    vec4 offset = vec4(sampleVS, 1.0);
    offset      = u_projection * offset;    // from view to clip-space
    offset.xyz /= offset.w;
    offset.xyz  = to_0_1(offset.xyz); // TODO ?

    float sampleDepth = positionVS_FromCoords(offset.xy).z;
    // occlusion += sampleDepth >= sampleVS.z + u_bias ? 1.0 : 0.0;
    float rangeCheck = smoothstep(0.0, 1.0, radius / abs(fragPosVS.z - sampleDepth));
    occlusion += (sampleDepth >= sampleVS.z + u_bias ? 1.0 : 0.0) * rangeCheck;
  }

  occlusion = occlusion / (float(u_kernelSize) - 1.0);
  outColor1 = vec4(vec3(1.0 - occlusion), 1.0);
}