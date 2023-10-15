#version 450

layout(set = 0, binding = 0)
uniform sampler2D texSampler;

layout(location = 0) in vec2 uvCoord;

layout(location = 0) out vec4 outputColor;

void main() {
  outputColor = texture(texSampler, uvCoord.xy);
}