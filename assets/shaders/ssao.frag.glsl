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
layout(location = 0) out float outColor1;


//@import ./_config_ubo;
//@import ./_utils;


/** Get view space coordinates for the depth map point */
vec3 positionVS_FromCoords(in vec2 texCoord) {
  vec2 uv = texCoord;
  return reprojectFromDepthBuffer(u_sceneDepthTex, uv, u_invProjectionMat).xyz;
}

float clampToNearFar(in float depthViewSpace) {
  float d = clamp(-depthViewSpace, u_nearAndFar.x, u_nearAndFar.y);
  return -d;
}


// @see https://learnopengl.com/Advanced-Lighting/SSAO
// @see https://github.com/SaschaWillems/Vulkan/blob/master/shaders/glsl/ssao/ssao.frag
void main() {
  vec3 fragPosVS = positionVS_FromCoords(v_position);

  vec3 normalWS = unpackNormal(u_normalTex, v_position);
  vec3 normalVS = normalize((u_viewMat * vec4(normalWS, 1.0)).xyz);
  normalVS = normalize(normalVS);

  // Get a random vector using a noise lookup
  vec3 randomVec = texture(u_noiseTex, v_position * u_noiseScale).xyz;
  randomVec = normalize(randomVec);


  // Gram-Schmidt process
  // @see http://en.wikipedia.org/wiki/Gram%E2%80%93Schmidt_process
  vec3 tangent = normalize(randomVec - normalVS * dot(randomVec, normalVS));
  vec3 bitangent = cross(normalVS, tangent);
  mat3 TBN = mat3(tangent, bitangent, normalVS);
  // TBN = inverse(TBN);

  float occlusion = 0.0;
  uint kernel_size = u_kernelSize;
  for(uint i = 0; i < kernel_size; i++) {
    float radius = u_radius; // TODO make depth-independent. Closer==smaller radius, Further==bigger radius?

    // get position of the sampled point, especially how far it is from camera
    // project `samplePointVS` to clip space (perspective)
    vec3 sampleDirectionNS = u_kernel[i]; // normal/tangent space
    vec3 sampleDirectionVS = TBN * sampleDirectionNS; // From tangent to view-space vector
    vec3 samplePointVS = fragPosVS + sampleDirectionVS * radius;
    float samplePointDepth = samplePointVS.z;

    // project the point onto depth texture (from our scene) and read distance from camera
    vec4 sampledPointClipS = u_projectionMat * vec4(samplePointVS, 1.0); // from view to clip-space
    vec2 sampledPointUV = sampledPointClipS.xy / sampledPointClipS.w; // clip space[-1,1] -> NDC
    sampledPointUV  = to_0_1(sampledPointUV); // NDC -> uv[0,1]
    // sample the XY-coordinates to get the real depth
    float sampleSceneDepth = positionVS_FromCoords(sampledPointUV).z;

    // if the sampled point is occluded by depth buffer depth
    float rangeCheck = smoothstep(0.0, 1.0, radius / abs(fragPosVS.z - sampleSceneDepth));
    occlusion += (sampleSceneDepth >= samplePointVS.z + u_bias ? 1.0 : 0.0) * rangeCheck;
  }

  occlusion = occlusion / float(u_kernelSize);
  outColor1 = 1.0 - occlusion;
}