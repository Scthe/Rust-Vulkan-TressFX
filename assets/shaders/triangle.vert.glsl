#version 450

layout(set = 0, binding = 0) 
uniform SceneUBO {
    mat4 vp;
} scene_ubo;

// https://www.khronos.org/opengl/wiki/Layout_Qualifier_(GLSL)
layout(location=0) in vec3 in_Position;
layout(location=1) in vec3 in_Normal;
layout(location=2) in vec2 in_UV;

layout(location = 0) out vec3 fragColor; // Consumes 1 location

void main() {
  vec4 pos = vec4(in_Position.xyz, 1.0);
  gl_Position = scene_ubo.vp * pos;
  fragColor = vec3(in_UV.xy, in_Normal.x);
}