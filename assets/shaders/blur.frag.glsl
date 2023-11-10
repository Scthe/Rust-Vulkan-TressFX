#version 450

precision highp float;
precision highp int;
precision highp usampler2D;


layout(push_constant) uniform Constants {
  // Direction of the blur:
  //   - First pass:   float2(1.0, 0.0)
  //   - Second pass:  float2(0.0, 1.0)
	vec2 u_direction;
  float u_blurRadius;
  float u_depthMaxDist;
  // The sigma value for the gaussian function: higher value means more blur
  // A good value for 9x9 is around 3 to 5
  // A good value for 7x7 is around 2.5 to 4
  // A good value for 5x5 is around 2 to 3.5
  float u_gaussSigma;
};

layout(binding = 1)
uniform sampler2D u_sourceTex;
layout(binding = 2)
uniform sampler2D u_linearDepthTex;


layout(location = 0) in vec2 v_position;
layout(location = 0) out vec4 outColor;


#pragma include ./_config_ubo;
#pragma include ./_utils;


float sampleLinearDepth(vec2 coord) {
  return textureLinearDepthIn_0_1_Range(u_linearDepthTex, coord, u_nearAndFar.x, u_nearAndFar.y).r;
}

vec4 sampleColorSource(vec2 uv) {
  uv = fixOpenGLTextureCoords_AxisY(uv);
  return texture(u_sourceTex, uv);
}

/** @param pixelsOffset - offset in pixels from center */
vec2 getSamplePointCoord (vec2 pixelsOffset) {
  vec2 sourceSize = vec2(textureSize(u_sourceTex, 0));
  // return (gl_FragCoord.xy + pixelsOffset) / sourceSize;
  return v_position + pixelsOffset / sourceSize;
}

/*
vec4 linearBlur() {
  float m = 1.0f / u_blurRadius;

  vec2 middleCoord = getSamplePointCoord(vec2(0.0, 0.0));
  vec4 sum = texture(u_sourceTex, middleCoord);
  float weightSum = 0.0;

  for (float i = 1.0; i <= u_blurRadius; i++) { // from 1 as 0 would be center pixel
    float weight =  1.0 - i * m; // linear
    vec2 sideNegCoord = getSamplePointCoord(-i * u_direction);
    vec2 sidePosCoord = getSamplePointCoord( i * u_direction);
    sum += texture(u_sourceTex, sideNegCoord) * weight;
    sum += texture(u_sourceTex, sidePosCoord) * weight;
    weightSum += 2.0 * weight;
  }

  return sum / weightSum;
}
*/


vec4 sampleWithDepthCompare (vec2 coord, float middleDepth, vec4 middleValue) {
  float sampleDepth = sampleLinearDepth(coord);
  float dist = abs(sampleDepth - middleDepth);
  if (dist < u_depthMaxDist) {
    return sampleColorSource(coord);
  } else {
    return middleValue;
  }
}

/** http://callumhay.blogspot.com/2010/09/gaussian-blur-shader-glsl.html
 *  https://github.com/genekogan/Processing-Shader-Examples/blob/master/TextureShaders/data/blur.glsl
 */
vec4 gaussianBlur() {
  vec3 incrementalGaussian;
  incrementalGaussian.x = 1.0 / (sqrt(2.0 * PI) * u_gaussSigma);
  incrementalGaussian.y = exp(-0.5 / (u_gaussSigma * u_gaussSigma));
  incrementalGaussian.z = incrementalGaussian.y * incrementalGaussian.y;

  vec4 sum = vec4(0.0, 0.0, 0.0, 0.0);
  float coefficientSum = 0.0;

  vec2 middleCoord = getSamplePointCoord(vec2(0.0, 0.0));
  float middleDepth = sampleLinearDepth(middleCoord).r;
  vec4 middleValue = sampleColorSource(middleCoord);
  sum += middleValue * incrementalGaussian.x;
  coefficientSum += incrementalGaussian.x;
  incrementalGaussian.xy *= incrementalGaussian.yz;

  for (float i = 1.0; i <= u_blurRadius; i++) { // from 1 as 0 would be center pixel
    vec2 sideNegCoord = getSamplePointCoord(-i * u_direction);
    vec2 sidePosCoord = getSamplePointCoord( i * u_direction);
    sum += sampleWithDepthCompare(sideNegCoord, middleDepth, middleValue) * incrementalGaussian.x;
    sum += sampleWithDepthCompare(sidePosCoord, middleDepth, middleValue) * incrementalGaussian.x;
    coefficientSum += 2.0 * incrementalGaussian.x;
    incrementalGaussian.xy *= incrementalGaussian.yz;
  }

  return sum / coefficientSum;
}



void main() {
  // outColor = linearBlur();
  outColor = gaussianBlur();
}