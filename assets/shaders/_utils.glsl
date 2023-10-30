const float PI = 3.14159265359;

vec2 fixOpenGLTextureCoords_AxisY(vec2 uv) {
  return vec2(uv.x, 1.0 - uv.y);
}

// srgb handled by hardware (we defined texture format in rust as such)
vec3 readModelTexture_srgb(sampler2D tex, vec2 coords) {
  coords = fixOpenGLTextureCoords_AxisY(coords);
  return texture(tex, coords).rgb; // as uint [0-255]
}

vec3 readModelTexture_uint(usampler2D tex, vec2 coords) {
  coords = fixOpenGLTextureCoords_AxisY(coords);
  uvec3 value = texture(tex, coords).rgb; // as uint [0-255]
  return vec3(value) / 255.0;
}

/** Preferably use *_SRGB attachment textures to auto apply gamma */
float doGamma (float color, float gammaValue) {
  return pow(color, 1.0 / gammaValue);
}
/** Preferably use *_SRGB attachment textures to auto apply gamma */
vec3 doGamma (vec3 color, float gammaValue) {
  return pow(color, vec3(1.0 / gammaValue));
}

float sRGBtoLinear (float color, float gammaValue) {
  // http://renderwonk.com/blog/index.php/archive/adventures-with-gamma-correct-rendering/
  if (color > 0.04045) {
    float n = color + 0.055;
    return pow(n / 1.055, gammaValue);
  }
  return color / 12.92;
}
vec3 sRGBtoLinear (vec3 color, float gammaValue) {
  return vec3(
    sRGBtoLinear(color.r, gammaValue),
    sRGBtoLinear(color.g, gammaValue),
    sRGBtoLinear(color.b, gammaValue)
  );
}
/*
// OR: https://github.com/EpicGames/UnrealEngine/blob/release/Engine/Shaders/Private/GammaCorrectionCommon.ush
half3 sRGBToLinear( half3 Color ) {
	Color = max(6.10352e-5, Color); // minimum positive non-denormal (fixes black problem on DX11 AMD and NV)
	return Color > 0.04045 ? pow( Color * (1.0 / 1.055) + 0.0521327, 2.4 ) : Color * (1.0 / 12.92);
}*/


float toLuma_fromGamma (vec3 rgbCol) {
  vec3 toLumaCoef = vec3(0.299f, 0.587f, 0.114f);
  return dot(toLumaCoef, rgbCol);
}

float toLuma_fromLinear(vec3 rgbCol) {
  vec3 toLumaCoef = vec3(0.2126729f,  0.7151522f, 0.0721750f);
  return dot(toLumaCoef, rgbCol);
}


float dotMax0 (vec3 n, vec3 toEye){
  return max(0.0, dot(n, toEye));
}

bool outOfScreen (vec2 coord) {
  return coord.x < 0.0 ||
         coord.x > 1.0 ||
         coord.y < 0.0 ||
         coord.y > 1.0;
}


// [0..1] -> [-1..1]
float to_neg1_1 (float v) { return 2.0 * v - 1.0; }
vec2  to_neg1_1 (vec2  v) { return 2.0 * v - 1.0; }
vec3  to_neg1_1 (vec3  v) { return 2.0 * v - 1.0; }
vec4  to_neg1_1 (vec4  v) { return 2.0 * v - 1.0; }

// [-1..1] -> [0..1]
float to_0_1 (float v) { return 0.5 * v + 0.5; }
vec2 to_0_1  (vec2  v) { return 0.5 * v + 0.5; }
vec3 to_0_1  (vec3  v) { return 0.5 * v + 0.5; }
vec4 to_0_1  (vec4  v) { return 0.5 * v + 0.5; }

// [-x..x] -> [0..1]
float saturate (float v) { return clamp(v, 0.0, 1.0); }
vec2  saturate (vec2  v) { return clamp(v, vec2(0.0, 0.0), vec2(1.0, 1.0)); }
vec3  saturate (vec3  v) { return clamp(v, vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)); }
vec4  saturate (vec4  v) { return clamp(v, vec4(0.0, 0.0, 0.0, 0.0), vec4(1.0, 1.0, 1.0, 1.0)); }

float max3(vec3 v){ return max(v.x, max(v.y, v.z)); }
float max4(vec4 v){ return max(v.w, max3(v.xyz)); }

float min3(vec3 v){ return min(v.x, min(v.y, v.z)); }
float min4(vec4 v){ return min(v.w, min3(v.xyz)); }

/** returns something random */
vec3 hash(vec3 a) {
  a = fract(a * vec3(.8, .8, .8));
  a += dot(a, a.yxz + 19.19);
  return fract((a.xxy + a.yxx) * a.zyx);
}


/**
 * Example usage:
 * uniform int u_optionFlags;
 * const int FLAG_USE_GAUSS = 1;
 * const int FLAG_USE_ROUGHNESS = 2;
 * ...
 * isFlag(u_optionFlags, FLAG_USE_ROUGHNESS) ? .. : ..;
*/
//
bool isFlag(int flags, int flagValue) {
  return (flags & flagValue) > 0;
}

uvec3 packNormal(vec3 normal) {
  vec3 n = to_0_1(normal);
  return uvec3(n * 255);
}

vec3 unpackNormal(usampler2D tex, vec2 coords) {
  vec3 normal = readModelTexture_uint(tex, coords).xyz;
  return normalize(to_neg1_1(normal));
}

float textureDepth(sampler2D tex, vec2 uv) {
  uv = fixOpenGLTextureCoords_AxisY(uv);
  return texture(tex, uv).r;
}

vec4 toClipSpace(vec2 uv, float depth) {
  return vec4(to_neg1_1(uv), depth, 1.0);
}


////////////
// Matrices

mat4 calcViewProjectionMatrix(mat4 v, mat4 p){ return p * v; }
mat4 calcModelViewProjectionMatrix(mat4 m, mat4 v, mat4 p){ return p * v * m; }


////////////
// Depth reprojection

vec4 reprojectFromDepthBuffer(float depthBufferZ, vec2 uv, mat4 reprojMat) {
  vec4 clipSpace = toClipSpace(uv, depthBufferZ);
  // vec4 clipSpace = vec4(to_neg1_1(vec3(texCoord, depthBufferZ)), 1);
  
  vec4 pos = reprojMat * clipSpace;
  return pos.xyzw / pos.w;
}

vec4 reprojectFromDepthBuffer(sampler2D depthTex, vec2 uv, mat4 reprojMat) {
  float depthBufferZ = textureDepth(depthTex, uv);
  return reprojectFromDepthBuffer(depthBufferZ, uv, reprojMat);
}
