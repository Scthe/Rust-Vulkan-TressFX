/** Vulkan and OpenGL have different coordinate system. I'm used to OpenGL's. */
vec4 oglTexture(sampler2D texSampler, vec2 texCoord) {
  vec2 fixedTexCoord = vec2(texCoord.x, 1.0f - texCoord.y);
  return texture(texSampler, fixedTexCoord);
}