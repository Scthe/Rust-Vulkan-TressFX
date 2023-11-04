// NOTE: rust packing has problems with raw floats (packing?) - use vec4

layout(binding = 3) 
uniform TfxParamsUniformBuffer {
  mat4 u_modelMatrix;
  vec4 u_generalSettings;
  // geometry
  vec4 u_geometry;
  vec4 u_centerOfGravity;
  // material
  vec4 u_albedo;
  vec4 u_specular1;
  vec4 u_specular2;
  vec4 u_material;
} TfxParamsUbo;

// u_specular1, u_specular2
#define u_specularColor1 (TfxParamsUbo.u_specular1.rgb)
#define u_specularColor2 (TfxParamsUbo.u_specular2.rgb)
#define u_specularPower1 (TfxParamsUbo.u_specular1.a)
#define u_specularPower2 (TfxParamsUbo.u_specular2.a)
// u_geometry
#define u_thinTip (TfxParamsUbo.u_geometry.x)
#define u_fiberRadius (TfxParamsUbo.u_geometry.y)
#define u_followHairSpreadRoot (TfxParamsUbo.u_geometry.z)
#define u_followHairSpreadTip (TfxParamsUbo.u_geometry.w)
// u_material
#define u_primaryShift (TfxParamsUbo.u_material.x)
#define u_secondaryShift (TfxParamsUbo.u_material.y)
#define u_specularStrength1 (TfxParamsUbo.u_material.z)
#define u_specularStrength2 (TfxParamsUbo.u_material.w)
// u_generalSettings
#define u_tfxDisplayMode (readConfigUint(TfxParamsUbo.u_generalSettings.x))
#define u_numVerticesPerStrand (readConfigUint(TfxParamsUbo.u_generalSettings.y))
#define u_tfxAoStrength (TfxParamsUbo.u_generalSettings.z)
#define u_tfxAoExp (TfxParamsUbo.u_generalSettings.w)
