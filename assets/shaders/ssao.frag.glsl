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
  // texCoord = fixOpenGLTextureCoords_AxisY(texCoord);
  float depth = texture(u_sceneDepthTex, texCoord).r;
  // vec3 texSpace = vec3(texCoord, depth);
  vec4 clipSpace = vec4(to_neg1_1(texCoord), depth, 1);
  // vec4 clipSpace = vec4(to_neg1_1(vec3(texCoord, depth)), 1);
  vec4 viewPos = u_invProjectionMat * clipSpace;
  return viewPos.xyz / viewPos.w;
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
  vec3 normalVS = (u_viewMat * vec4(normalWS, 1.0)).xyz;
  normalVS = normalize(normalVS);

  // Get a random vector using a noise lookup
	// ivec2 texDim = ivec2(400, 300); // textureSize(samplerPositionDepth, 0); 
	// ivec2 noiseDim = textureSize(u_noiseTex, 0);
	// vec2 noiseScale = vec2(float(texDim.x)/float(noiseDim.x), float(texDim.y)/(noiseDim.y));  
  // vec3 randomVec = texture(u_noiseTex, v_position * u_noiseScale).xyz;
  // vec3 randomVec = texture(u_noiseTex, v_position * noiseScale).xyz;
  vec3 randomVec = vec3(0.0, 1.0, 0.0);
  // vec3 randomVec = to_neg1_1(hash(vec3(v_position.xy, v_position.x * v_position.y)));
  randomVec = normalize(randomVec);

  // Gram-Schmidt process
  // @see http://en.wikipedia.org/wiki/Gram%E2%80%93Schmidt_process
  vec3 tangent = normalize(randomVec - normalVS * dot(randomVec, normalVS));
  vec3 bitangent = cross(normalVS, tangent);
  mat3 TBN = mat3(tangent, bitangent, normalVS);
  // TBN = inverse(TBN);

  float occlusion = 0.0;
  float DBG_MAGIC = 0;
  int kernel_size = u_kernelSize;
  for(int i = 0; i < kernel_size; i++) {
    float radius = u_radius; // TODO make depth-independent. Closer==smaller radius, Further==bigger radius?

    // get position of the sampled point, especially how far it is from camera
    // project `samplePointVS` to clip space (perspective)
    vec3 sampleDirectionNS = u_kernel[i]; // normal/tangent space
    vec3 sampleDirectionVS = TBN * sampleDirectionNS; // From tangent to view-space vector
    vec3 samplePointVS = fragPosVS + sampleDirectionVS * radius;
    float samplePointDepth = samplePointVS.z;
    // samplePointDepth = clampToNearFar(samplePointDepth);

    // project the point onto depth texture (from our scene) and read distance from camera
    vec4 sampledPointClipS = vec4(samplePointVS, 1.0);
    sampledPointClipS      = u_projection * sampledPointClipS;    // from view to clip-space
    vec2 sampledPointUV = sampledPointClipS.xy / sampledPointClipS.w; // clip space[-1,1] -> NDC
    sampledPointUV  = to_0_1(sampledPointUV); // NDC -> uv[0,1]
    // sample the XY-coordinates to get the real depth
    float sampleSceneDepth = positionVS_FromCoords(sampledPointUV).z;
    // sampleSceneDepth = clampToNearFar(sampleSceneDepth);

    // if the sampled point is occluded by depth buffer depth
    occlusion += sampleSceneDepth > samplePointDepth ? 1.0 : 0.0;
    if (sampleSceneDepth > samplePointDepth) {
      DBG_MAGIC += 0.0001;
    }
    // occlusion += sampleSceneDepth >= samplePointVS.z + u_bias ? 1.0 : 0.0;
    // float rangeCheck = smoothstep(0.0, 1.0, radius / abs(fragPosVS.z - sampleSceneDepth));
    // occlusion += (sampleSceneDepth >= samplePointVS.z + u_bias ? 1.0 : 0.0) * rangeCheck;
  }

  occlusion = occlusion / float(u_kernelSize);
  outColor1 = 1.0 - occlusion + DBG_MAGIC;
  // outColor1 = abs(normalWS.x);
}