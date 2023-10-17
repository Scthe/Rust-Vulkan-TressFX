#version 450

//@import _utils;

layout(set = 0, binding = 1)
uniform sampler2D texSampler;

layout(location = 0) in vec3 inColor;
layout(location = 0) out vec4 outputColor;

void main() {
  outputColor = oglTexture(texSampler, inColor.xy);
}