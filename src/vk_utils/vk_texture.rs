use jpeg_decoder::{Decoder, PixelFormat};
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use vma::Alloc;

use log::{info, trace};

use ash;
use ash::vk;

use crate::vk_utils::create_image_view;

use super::{
  determine_gpu_allocation_info, get_persistently_mapped_pointer, MemoryMapPointer,
  VkMemoryPreference, VkMemoryResource, WithSetupCmdBuffer,
};

pub struct VkTexture {
  /// For debugging. User-set name
  name: String,
  /// For debugging. Includes size etc.
  long_name: String,
  pub width: u32,
  pub height: u32,
  /// Native Vulkan image
  pub image: vk::Image,
  image_view: vk::ImageView,
  pub aspect_flags: vk::ImageAspectFlags,
  pub layout: vk::ImageLayout,
  allocation: vma::Allocation,
  format: vk::Format,
  // mapping
  mapped_pointer: Option<MemoryMapPointer>,
}

impl VkTexture {
  /// vk::Format for textures that contain raw data e.g. specular, normal, hairShadow.
  /// As opposed to diffuse/albedo texture that are _SRGB.
  pub const RAW_DATA_TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UINT;

  /// ### Params
  /// * `tiling` -  `vk::ImageTiling::OPTIMAL` if uploaded from staging buffer. `vk::ImageTiling::LINEAR` if written from CPU (mapped).
  /// * `usage` - usually `vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED`
  ///     (or `vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT` for depth)
  /// * `allocation_flags` - `vk::MemoryPropertyFlags::DEVICE_LOCAL` for GPU-allocated
  ///     or `vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT` for mapped
  pub fn empty(
    device: &ash::Device,
    allocator: &vma::Allocator,
    with_setup_cb: &impl WithSetupCmdBuffer,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    memory_pref: VkMemoryPreference,
    initial_layout: vk::ImageLayout,
  ) -> Self {
    let create_info = vk::ImageCreateInfo::builder()
      .image_type(vk::ImageType::TYPE_2D)
      .extent(vk::Extent3D {
        width: size.width,
        height: size.height,
        depth: 1,
      })
      .format(format)
      .tiling(tiling)
      .usage(usage)
      // https://vulkan.lunarg.com/doc/view/1.3.261.1/windows/1.3-extensions/vkspec.html#VUID-VkImageCreateInfo-initialLayout-00993
      // has to be VK_IMAGE_LAYOUT_UNDEFINED or VK_IMAGE_LAYOUT_PREINITIALIZED
      // https://stackoverflow.com/questions/76945200/how-to-properly-use-vk-image-layout-preinitialized
      .initial_layout(vk::ImageLayout::PREINITIALIZED) // required by validation layers
      // verbose properties, but vulkan requires
      .sharing_mode(vk::SharingMode::EXCLUSIVE)
      .samples(vk::SampleCountFlags::TYPE_1)
      .mip_levels(1)
      .array_layers(1)
      .build();

    let alloc_create_info = determine_gpu_allocation_info(&memory_pref);
    let (image, allocation) = unsafe {
      allocator
        .create_image(&create_info, &alloc_create_info)
        .expect("Failed allocating GPU memory for texture")
    };
    let mapped_pointer = get_persistently_mapped_pointer(allocator, &allocation);

    let aspect = get_image_aspect_from_format(format);
    let image_view = create_image_view(device, image, create_info.format, aspect);
    let mut texture = Self {
      name: name.clone(),
      long_name: get_texture_long_name(name, size.width, size.height),
      width: size.width,
      height: size.height,
      image,
      allocation,
      mapped_pointer,
      layout: create_info.initial_layout,
      image_view,
      aspect_flags: aspect,
      format,
    };

    if initial_layout != vk::ImageLayout::PREINITIALIZED {
      texture.set_initial_image_layout(with_setup_cb, initial_layout);
    }
    texture
  }

  /// * `format` - usually `vk::Format::R8G8B8A8_SRGB` for diffuse, but raw data
  ///     (no `_SRGB` or `VkTexture::RAW_DATA_TEXTURE_FORMAT`) for specular/normals
  pub fn from_file(
    device: &ash::Device,
    allocator: &vma::Allocator,
    with_setup_cb: &impl WithSetupCmdBuffer,
    path: &std::path::Path,
    format: vk::Format,
  ) -> Self {
    // load image from file
    info!("Loading texture from '{}'", path.to_string_lossy());
    let file = File::open(path).expect("Failed to open file");
    let mut decoder = Decoder::new(BufReader::new(file));
    let pixel_bytes_rgb = decoder.decode().expect("Failed to decode image");
    let metadata = decoder.info().unwrap();
    trace!("File meta: {:?}", metadata);

    assert!(
      metadata.pixel_format == PixelFormat::RGB24,
      "Texture '{}' has pixel format {:?}, expected PixelFormat::RGB24",
      path.display(),
      metadata.pixel_format
    );

    let pixel_bytes = covert_rgb_to_rgba(&pixel_bytes_rgb);
    let size = vk::Extent2D {
      width: metadata.width as _,
      height: metadata.height as _,
    };

    // create texture
    let name = path.file_name().unwrap_or(OsStr::new(path));
    let name_str = name.to_string_lossy().to_string();
    let mut texture = Self::empty(
      device,
      allocator,
      with_setup_cb,
      name_str,
      size,
      format,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
      VkMemoryPreference::GpuOnly,
      vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    );

    Self::write_initial_data(device, allocator, with_setup_cb, &pixel_bytes, &texture);
    texture.set_initial_image_layout(with_setup_cb, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    texture
  }

  pub fn from_data(
    device: &ash::Device,
    allocator: &vma::Allocator,
    with_setup_cb: &impl WithSetupCmdBuffer,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    data_bytes: &Vec<u8>,
  ) -> Self {
    let pixel_cnt = (size.width * size.height) as usize;
    if data_bytes.len() % pixel_cnt != 0 {
      panic!(
        "Tried to create VkTexture::from_data with dimensions {}x{}px. Provided data ({} bytes) does not allign with the dimensions ({} bytes per pixel).",
        size.width,
        size.height,
        data_bytes.len(),
        (data_bytes.len() as f32) / (pixel_cnt as f32)
      );
    }

    // create texture
    let mut texture = Self::empty(
      device,
      allocator,
      with_setup_cb,
      name,
      size,
      format,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
      VkMemoryPreference::GpuOnly,
      vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    );

    Self::write_initial_data(device, allocator, with_setup_cb, data_bytes, &texture);
    texture.set_initial_image_layout(with_setup_cb, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    texture
  }

  fn write_initial_data(
    device: &ash::Device,
    allocator: &vma::Allocator,
    with_setup_cb: &impl WithSetupCmdBuffer,
    pixel_bytes: &Vec<u8>,
    dst_texture: &VkTexture,
  ) {
    let mut scratch_texture = Self::empty(
      device,
      allocator,
      with_setup_cb,
      format!("{}-scratch-texture", dst_texture.name),
      dst_texture.size(),
      dst_texture.format,
      vk::ImageTiling::LINEAR,
      vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
      VkMemoryPreference::ScratchTransfer,
      vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
    );
    scratch_texture.write_to_mapped(&pixel_bytes);

    // copy content
    with_setup_cb.with_setup_cb(|device, cb| unsafe {
      let offset_zero = vk::Offset3D { x: 0, y: 0, z: 0 };
      let subresources = vk::ImageSubresourceLayers {
        aspect_mask: dst_texture.aspect_flags, // same for both
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
      };
      let img_copy = ash::vk::ImageCopy::builder()
        .src_offset(offset_zero)
        .src_subresource(subresources)
        .dst_offset(offset_zero)
        .dst_subresource(subresources)
        .extent(vk::Extent3D {
          width: dst_texture.width,
          height: dst_texture.height,
          depth: 1,
        })
        .build();
      device.cmd_copy_image(
        cb,
        scratch_texture.image,
        scratch_texture.layout,
        dst_texture.image,
        dst_texture.layout,
        &[img_copy],
      );
    });

    unsafe { scratch_texture.delete(device, allocator) };
  }

  pub fn image_view(&self) -> vk::ImageView {
    self.image_view
  }

  /// If you need extra image view. Needed for depth-stencil textures if we want
  /// to read depth only.
  pub fn create_extra_image_view(
    &self,
    device: &ash::Device,
    aspect_mask_flags: vk::ImageAspectFlags,
  ) -> vk::ImageView {
    create_image_view(device, self.image, self.format, aspect_mask_flags)
  }

  pub fn is_color(&self) -> bool {
    self.aspect_flags == vk::ImageAspectFlags::COLOR
  }

  pub fn is_depth_stencil(&self) -> bool {
    self.aspect_flags == (vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
  }
  pub fn is_depth(&self) -> bool {
    self.aspect_flags == vk::ImageAspectFlags::DEPTH
  }

  pub fn size(&self) -> vk::Extent2D {
    vk::Extent2D {
      width: self.width,
      height: self.height,
    }
  }

  pub unsafe fn delete(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    device.destroy_image_view(self.image_view, None);
    allocator.destroy_image(self.image, &mut self.allocation)
  }

  pub fn create_texture_bytes<GetBytes>(size: vk::Extent2D, mut f: GetBytes) -> Vec<u8>
  where
    GetBytes: FnMut(u32, u32, u32) -> Vec<f32>,
  {
    let pixel_cnt = (size.width * size.height) as usize;
    let mut result: Vec<f32> = Vec::with_capacity(pixel_cnt * 4);

    (0..size.height).for_each(|y| {
      (0..size.width).for_each(|x| {
        let idx = y * size.width + x;
        let values = f(x, y, idx);
        values.iter().for_each(|v| result.push(*v));
      });
    });

    let bytes_slice: &[u8] = bytemuck::cast_slice(&result[..]);
    bytes_slice.iter().map(|v| *v).collect()
  }
}

pub fn get_image_aspect_from_format(format: vk::Format) -> vk::ImageAspectFlags {
  match format {
    vk::Format::D24_UNORM_S8_UINT => vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
    vk::Format::D32_SFLOAT => vk::ImageAspectFlags::DEPTH,
    vk::Format::R8G8B8A8_SRGB
    | vk::Format::R8G8B8A8_UINT
    | vk::Format::R32G32B32A32_SFLOAT
    | vk::Format::R32_UINT
    | vk::Format::R32_SFLOAT => vk::ImageAspectFlags::COLOR,
    _ => panic!("Cannot determine image aspect for {:?}", format),
  }
}

// Used cause vk::Format::R8G8B8_SRGB are not supported on my GPU
fn covert_rgb_to_rgba(data_rgb: &Vec<u8>) -> Vec<u8> {
  let pixel_cnt = data_rgb.len() / 3;
  let mut data: Vec<u8> = Vec::with_capacity(pixel_cnt * 4);
  // trace!("covert_rgb_to_rgba: bytes({:?}), pixel_cnt({})", data_rgb.len(), pixel_cnt);

  (0..pixel_cnt).for_each(|pixel_id| {
    let offset = pixel_id * 3;
    data.push(data_rgb[offset]);
    data.push(data_rgb[offset + 1]);
    data.push(data_rgb[offset + 2]);
    data.push(255u8);
  });

  data
}

impl VkMemoryResource for VkTexture {
  fn get_name(&self) -> &String {
    &self.name
  }

  fn get_long_name(&self) -> &String {
    &self.long_name
  }

  fn get_mapped_pointer(&self) -> Option<MemoryMapPointer> {
    self.mapped_pointer.clone()
  }
}

fn get_texture_long_name(name: String, width: u32, height: u32) -> String {
  format!("VkTexture({}, {}x{})", name, width, height)
}
