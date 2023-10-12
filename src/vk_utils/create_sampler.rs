use ash;
use ash::vk;

/// OMG so much fluff for simple sampler.
/// And filtering will be always vk::Filter::LINEAR anyway.
pub fn create_sampler(
  device: &ash::Device,
  mag_filter: vk::Filter,
  min_filter: vk::Filter,
) -> vk::Sampler {
  let create_info = vk::SamplerCreateInfo::builder()
    .mag_filter(mag_filter)
    .min_filter(min_filter)
    .address_mode_u(vk::SamplerAddressMode::REPEAT)
    .address_mode_v(vk::SamplerAddressMode::REPEAT)
    .address_mode_w(vk::SamplerAddressMode::REPEAT)
    .anisotropy_enable(false) // TODO turn on anisotropy in samplers
    .max_anisotropy(8f32) // 1050TI handles 16
    .compare_enable(false)
    .compare_op(vk::CompareOp::ALWAYS)
    .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
    .unnormalized_coordinates(false) // address with [0, 1) instead of [0, tex_width)
    // mipmaps:
    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
    .mip_lod_bias(0f32)
    .min_lod(0f32)
    .max_lod(0f32)
    .build();

  unsafe {
    device
      .create_sampler(&create_info, None)
      .expect("Failed creating sampler")
  }
}
