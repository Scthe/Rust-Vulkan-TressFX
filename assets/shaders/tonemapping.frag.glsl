#version 450

precision highp float;
precision highp int;
precision highp usampler2D;

layout(binding = 1)
uniform sampler2D u_source;

layout(location = 0) in vec2 v_position; // TexCoords
layout(location = 0) out vec4 outColor;


#pragma include ./_config_ubo;
#pragma include ./_utils;
#pragma include ./postfx/_dither;
#pragma include ./postfx/_tonemappers;
#pragma include ./postfx/_color_grading;

const uint TONEMAP_LINEAR = 0;
const uint TONEMAP_REINHARD = 1;
const uint TONEMAP_U2 = 2;
const uint TONEMAP_PHOTOGRAPHIC = 3;
const uint TONEMAP_ACES = 4;


vec3 doTonemapping(uint tonemapMode, vec3 hdrColor) {
  switch (tonemapMode) {
    case TONEMAP_U2: return Uncharted2Tonemap(hdrColor);
    case TONEMAP_LINEAR: return tonemapLinear(hdrColor);
    case TONEMAP_PHOTOGRAPHIC: return tonemapPhotographic(hdrColor);
    case TONEMAP_REINHARD: return tonemapReinhard(hdrColor);
    default:
    case TONEMAP_ACES: return tonemapACES(hdrColor);
  }
}


void main() {
  vec2 pixelTS = fixOpenGLTextureCoords_AxisY(v_position);
  vec3 colorHDR = texture(u_source, pixelTS).rgb;

  // do dithering to break up banding
  colorHDR = doDither(colorHDR, u_ditherStrength);

  // color grade raw HDR
  // In old days we used LUTs for this, but LUTs require conversion to LDR.
  // Since HDR displays are now available, we do color grading in HDR,
  // skipping LDR conversion. This, and also cause we can.
  vec3 colorAfterColorGrading = colorCorrectAll(colorHDR);

  outColor.rgb = saturate(
    doTonemapping(u_tonemappingMode, colorAfterColorGrading)
  );

  float luma = toLuma_fromLinear(outColor.rgb);
  // just SOME gamma, does not matter exact. We need to convert into SOME perceptual space
  outColor.a = doGamma(luma, u_fxaa_luma_gamma);
}
