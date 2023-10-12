use ash;
use ash::vk;
use bytemuck;
use glam::Mat4;

static mut SCENE_UNIFORM_BUFFER_LAYOUT: Option<vk::DescriptorSetLayout> = None;

// TODO rename to GlobalSharedUniformBuffer - config overwritten by UI anyway..
// TODO rename to render_graph/shared/_.* ?
// TODO add check when compiling shader if .glsl is newer than .spv. Then panic and say to recompile shaders

#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct SceneUniformBuffer {
  // view projection matrix for current camera
  pub u_vp: Mat4,
}

unsafe impl bytemuck::Zeroable for SceneUniformBuffer {}
unsafe impl bytemuck::Pod for SceneUniformBuffer {}

impl SceneUniformBuffer {
  // must be same as in shader!
  pub const BINDING_INDEX: u32 = 0;
  // TODO this is tmp, texture should not be part of this shader
  pub const TMP_TEXTURE_BINDING_INDEX: u32 = 1;

  pub fn get_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    if let Some(layout) = unsafe { SCENE_UNIFORM_BUFFER_LAYOUT } {
      return layout;
    } else {
      let layout = SceneUniformBuffer::create_layout(device);
      unsafe { SCENE_UNIFORM_BUFFER_LAYOUT = Some(layout) };
      return layout;
    }
  }

  fn create_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    let binding = vk::DescriptorSetLayoutBinding::builder()
      .binding(SceneUniformBuffer::BINDING_INDEX)
      .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
      .descriptor_count(1)
      .stage_flags(vk::ShaderStageFlags::VERTEX)
      .build();

    let tmp_tex_binding = vk::DescriptorSetLayoutBinding::builder()
      .binding(SceneUniformBuffer::TMP_TEXTURE_BINDING_INDEX)
      .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
      .descriptor_count(1)
      .stage_flags(vk::ShaderStageFlags::FRAGMENT)
      .build();

    // TODO all below is per-shader?
    let ubo_descriptors_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
      .bindings(&[binding, tmp_tex_binding])
      .build();

    unsafe {
      device
        .create_descriptor_set_layout(&ubo_descriptors_create_info, None)
        .expect("Failed to create DescriptorSetLayout")
    }
  }

  pub unsafe fn destroy_layout(device: &ash::Device) {
    if let Some(layout) = unsafe { SCENE_UNIFORM_BUFFER_LAYOUT } {
      device.destroy_descriptor_set_layout(layout, None);
      SCENE_UNIFORM_BUFFER_LAYOUT = None;
    }
  }
}
