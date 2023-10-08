use log::trace;

use ash;
use ash::vk;

// https://github.com/zeux/niagara/blob/master/src/shaders.cpp

fn load_shader_module(device: &ash::Device, path: &std::path::Path) -> vk::ShaderModule {
  trace!("Loading shader from {}", path.to_string_lossy());

  let mut file =
    std::fs::File::open(path).expect(&format!("Could not open file '{}'", path.to_string_lossy()));
  let spirv_code = ash::util::read_spv(&mut file).unwrap();
  let create_info = vk::ShaderModuleCreateInfo::builder()
    .code(&spirv_code)
    .build();

  let shader_module = unsafe {
    device
      .create_shader_module(&create_info, None)
      .expect(&format!(
        "Failed to create shader module from file '{}'",
        path.to_string_lossy()
      ))
  };

  shader_module
}

pub fn load_shader(
  device: &ash::Device,
  stage: vk::ShaderStageFlags,
  path: &std::path::Path,
) -> (vk::ShaderModule, vk::PipelineShaderStageCreateInfo) {
  let shader_fn_name = unsafe { std::ffi::CStr::from_ptr("main\0".as_ptr() as *const i8) };

  let shader_module = load_shader_module(device, path);

  let stage_stage = vk::PipelineShaderStageCreateInfo::builder()
    .stage(stage)
    .module(shader_module)
    .name(shader_fn_name)
    .build();
  trace!("Shader {:?} loaded from {}", stage, path.to_string_lossy());

  (shader_module, stage_stage)
}
