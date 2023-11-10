#version 450

precision highp float;
precision highp int;
precision highp usampler2D;


layout(binding = 1)
uniform sampler2D u_tonemappedTex; // TODO usampler2D
layout(binding = 2)
uniform usampler2D u_normalsTex;
layout(binding = 3)
uniform sampler2D u_ssaoTex;
layout(binding = 4)
uniform sampler2D u_depthTex;
layout(binding = 5)
uniform sampler2D u_directionalShadowDepthTex;
layout(binding = 6)
uniform sampler2D u_rawForwardPassResult;
layout(binding = 7)
uniform sampler2D u_linearDepthTex;


//@import _config_ubo;
//@import _utils;
//@import postfx/_fxaa;


layout(location = 0) in vec2 v_position;
layout(location = 0) out vec4 color1;

vec3 doFxaa (vec2 uv) {
  vec4 color;

  if (u_edgeThreshold == 0.0) {
    // color = vec4(readModelTexture_uint(u_tonemappedTex, uv), 1.0);
    color = vec4(texture(u_tonemappedTex, uv).rgb, 1.0);
  } else {
    color = FxaaPixelShader(
      uv, // in [0-1]
      u_tonemappedTex,
      u_tonemappedTex,
      vec2(1.0) / u_viewport,
      u_subpixel,
      u_edgeThreshold,
      u_edgeThresholdMin
    );
  }

  return color.rgb;
}

vec3 getNormal() {
  // v_position as `readModelTexture_uint` already does `fixOpenGLTextureCoords_AxisY`
  return unpackNormal(u_normalsTex, v_position).xyz;
}

float sampleLinearDepth(){
  vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
  return texture(u_linearDepthTex, uv).r;
}

vec4 sampleRawDiffuseTexture() {
  vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
  return texture(u_rawForwardPassResult, uv).rgba;
}

vec4 getWorldSpacePosition() {
  vec2 uv = v_position;
  mat4 invVP_matrix = inverse(calcViewProjectionMatrix(u_viewMat, u_projectionMat));
  return reprojectFromDepthBuffer(u_depthTex, v_position, invVP_matrix);
}

/** Returns distance along the ray (negative if behind). Returns -1.0 if miss.
  * `vec3 rayHitWorldPos = rayDir * rayHit + rayOrigin;`
  */
float sphIntersect(vec3 rayOrigin, vec3 rayDir, vec4 sph){
    vec3 oc = rayOrigin - sph.xyz;
    float b = dot(oc, rayDir);
    float c = dot(oc, oc) - sph.w * sph.w;
    float h = b * b - c;
    if(h < 0.0){ return -1.0; }
    h = sqrt(h);
    return -b - h;
}

const float DEBUG_SPHERE_RADIUS = 1.5;

#define DRAW_DEBUG_SPHERE(position, color) {\
  float rayHit = sphIntersect(rayOrigin, rayDir, vec4(position, DEBUG_SPHERE_RADIUS)); \
  if (rayHit > 0 && rayHit < closestRayHit) { closestRayHit = rayHit; sphereColor = color; } \
}

vec4 drawDebugSpheres(){
  const vec4 SKIP_DRAW = vec4(0,0,0, 0);
  if (!u_showDebugPositions) { return SKIP_DRAW; }

  vec4 fragPositionWorldSpace = getWorldSpacePosition();
  vec3 rayDir = normalize(fragPositionWorldSpace.xyz - u_cameraPosition.xyz);
  vec3 rayOrigin = u_cameraPosition.xyz;
  float closestRayHit = 99999;
  vec3 sphereColor = vec3(0.0);

  DRAW_DEBUG_SPHERE(u_directionalShadowCasterPosition.xyz, vec3(0.2));
  DRAW_DEBUG_SPHERE(u_light0_Position, u_light0_Color.rgb);
  DRAW_DEBUG_SPHERE(u_light1_Position, u_light1_Color.rgb);
  DRAW_DEBUG_SPHERE(u_light2_Position, u_light2_Color.rgb);
  DRAW_DEBUG_SPHERE(u_sssPosition, vec3(0.87, 0.53, 0.36)); // #de875d

  if (closestRayHit > 0 && closestRayHit < 99999) {
    return vec4(sphereColor, 1);
  }
  return SKIP_DRAW;
}


// Gamma not needed as swapchain image is in SRGB
void main() {
  vec3 result;

  switch(u_displayMode) {
    case DISPLAY_MODE_NORMALS: {
      // v_position as `readModelTexture_uint` already does `fixOpenGLTextureCoords_AxisY`
      vec3 normal = getNormal();
      result = abs(normal);
      break;
    }
    
    case DISPLAY_MODE_LUMA: {
      vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
      float luma = texture(u_tonemappedTex, uv).a;
      result = vec3(luma, luma, luma);
      break;
    }

    case DISPLAY_MODE_SSAO: {
      vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
      float ssao = texture(u_ssaoTex, uv).r;
      result = vec3(ssao);
      break;
    }
    
    case DISPLAY_MODE_LINEAR_DEPTH: {
      // TODO use `textureLinearDepthIn_0_1_Range`
      float depth = -sampleLinearDepth(); // value is [0.1..100]
      vec2 nearAndFarPreview = -u_linear_depth_preview_range; // value is e.g. [5, 10]
      float d = nearAndFarPreview.y - nearAndFarPreview.x; // value for [5, 10] is 5
      float val = (depth - nearAndFarPreview.x) / d;
      result = vec3(saturate(val));
      break;
    }

    case DISPLAY_MODE_SHADOW_MAP: {
      result = sampleRawDiffuseTexture().rgb;
      break;
    }

    case DISPLAY_MODE_SSS_SCATTERING: {
      result = sampleRawDiffuseTexture().rgb;
      break;
    }

    case DISPLAY_MODE_SSS_THICKNESS: {
      result = sampleRawDiffuseTexture().rgb;
      break;
    }

    default:
    case DISPLAY_MODE_FINAL: {
      vec2 uv = fixOpenGLTextureCoords_AxisY(v_position);
      result = doFxaa(uv);
      break;
    }
  }
  
  vec4 colDpgSpheres = drawDebugSpheres();
  result = mix(result, colDpgSpheres.rgb, colDpgSpheres.a);
  color1 = vec4(result, 1.0f);
}