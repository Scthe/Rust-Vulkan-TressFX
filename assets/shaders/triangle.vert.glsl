#version 450

layout(binding = 1) 
uniform SceneUBO {
    mat4 vp;
} scene_ubo;

// https://www.khronos.org/opengl/wiki/Layout_Qualifier_(GLSL)
layout(location = 0) in vec4 inPosition; // Consumes 1 location, so next is 1
layout(location = 1) in vec4 inColor; // Consumes 1 location, so next would be 2

layout(location = 0) out vec3 fragColor; // Consumes 1 location

void main() {
  vec4 pos = vec4(inPosition.xy, 0.0, 1.0);
  gl_Position = scene_ubo.vp * pos;
  fragColor = inColor.rgb;
}