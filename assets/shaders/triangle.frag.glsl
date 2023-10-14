#version 450

layout(set = 0, binding = 1)
uniform sampler2D texSampler;

layout(location = 0) in vec3 inColor;

layout(location = 0) out vec4 outputColor;

void main() {
  // outputColor = vec4(0.0, 0.5, 0.5, 1.0);
  // outputColor = vec4(inColor, 1.0);
  outputColor = texture(texSampler, inColor.xy);
}