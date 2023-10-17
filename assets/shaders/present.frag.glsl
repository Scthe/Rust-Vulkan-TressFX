#version 450

layout(set = 0, binding = 0)
uniform sampler2D texSampler;

layout(location = 0) in vec2 uvCoord;

layout(location = 0) out vec4 outputColor;

void main() {
  vec2 texCoord = vec2(uvCoord.x, 1.0f - uvCoord.y);
  outputColor = texture(texSampler, texCoord);
}