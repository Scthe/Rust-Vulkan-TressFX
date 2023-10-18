layout(binding = 1) 
uniform ForwardModelUBO {
  mat4 u_M; // model matrix
  // material
  float u_specular;
  float u_specularMul;
  int u_materialFlags;
  float u_sssTransluency;
  float u_sssWidth;
  float u_sssBias;
  float u_sssGain;
  float u_sssStrength;
  // vec3 u_sssPosition;
};