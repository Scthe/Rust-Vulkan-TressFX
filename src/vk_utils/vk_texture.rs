use jpeg_decoder::{Decoder, PixelFormat};
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use vma::Alloc;

use log::{info, trace};

use ash;
use ash::vk;

use crate::vk_utils::{create_image_barrier, create_image_view};

use super::{MemoryMapPointer, VkMemoryResource, WithSetupCmdBuffer};

const DEBUG_LAYOUT_TRANSITIONS: bool = false;

pub struct VkTexture {
  // For debugging
  name: String,
  pub width: u32,
  pub height: u32,
  /// Native Vulkan image
  image: vk::Image,
  image_view: vk::ImageView,
  aspect_flags: vk::ImageAspectFlags,
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

  /// * `tiling` -  `vk::ImageTiling::OPTIMAL` if uploaded from staging buffer. `vk::ImageTiling::LINEAR` if written from CPU (mapped).
  /// * `usage` - usually `vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED`
  ///     (or `vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT` for depth)
  /// * `aspect` - `vk::ImageAspectFlags::COLOR` or `vk::ImageAspectFlags::DEPTH`
  /// * `allocation_flags` - `vk::MemoryPropertyFlags::DEVICE_LOCAL` for GPU-allocated
  ///     or `vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT` for mapped
  pub fn empty(
    device: &ash::Device,
    allocator: &vma::Allocator,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    tiling: vk::ImageTiling, // always vk::ImageTiling::OPTIMAL?
    usage: vk::ImageUsageFlags,
    aspect: vk::ImageAspectFlags,
    allocation_flags: vk::MemoryPropertyFlags,
  ) -> VkTexture {
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
      // https://stackoverflow.com/questions/76945200/how-to-properly-use-vk-image-layout-preinitialized
      .initial_layout(vk::ImageLayout::PREINITIALIZED)
      // verbose properties, but vulkan requires
      .sharing_mode(vk::SharingMode::EXCLUSIVE)
      .samples(vk::SampleCountFlags::TYPE_1)
      .mip_levels(1)
      .array_layers(1)
      .build();

    #[allow(deprecated)]
    let alloc_info = vma::AllocationCreateInfo {
      usage: vma::MemoryUsage::GpuOnly,
      required_flags: allocation_flags,
      ..Default::default()
    };

    let (image, allocation) = unsafe {
      allocator
        .create_image(&create_info, &alloc_info)
        .expect("Failed allocating GPU memory for texture")
    };

    let image_view = create_image_view(device, image, create_info.format, aspect);

    VkTexture {
      name: create_texture_name(name, size.width, size.height),
      width: size.width,
      height: size.height,
      image,
      allocation,
      mapped_pointer: None,
      layout: create_info.initial_layout,
      image_view,
      aspect_flags: aspect,
      format,
    }
  }

  /// * `format` - usually `vk::Format::R8G8B8A8_SRGB` for diffuse, but raw data
  ///     (no `_SRGB` or `VkTexture::RAW_DATA_TEXTURE_FORMAT`) for specular/normals
  pub fn from_file(
    device: &ash::Device,
    allocator: &vma::Allocator,
    app_init: &impl WithSetupCmdBuffer,
    path: &std::path::Path,
    format: vk::Format,
  ) -> VkTexture {
    // load image from file
    info!("Loading texture from '{}'", path.to_string_lossy());
    let file = File::open(path).expect("Failed to open file");
    let mut decoder = Decoder::new(BufReader::new(file));
    let pixel_bytes_rgb = decoder.decode().expect("Failed to decode image");
    let metadata = decoder.info().unwrap();
    trace!("File meta: {:?}", metadata);

    if metadata.pixel_format != PixelFormat::RGB24 {
      panic!(
        "Texture '{}' has pixel format {:?}, expected PixelFormat::RGB24",
        path.display(),
        metadata.pixel_format
      );
    }
    let pixel_bytes = covert_rgb_to_rgba(&pixel_bytes_rgb);
    let size = vk::Extent2D {
      width: metadata.width as _,
      height: metadata.height as _,
    };

    // create texture
    let name = path.file_name().unwrap_or(OsStr::new(path));
    let name_str = name.to_string_lossy().to_string();
    let mut texture = VkTexture::empty(
      device,
      allocator,
      name_str,
      size,
      format,
      vk::ImageTiling::LINEAR,
      vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
      vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    // map image and copy content
    texture.map_memory(allocator);
    texture.write_to_mapped(&pixel_bytes);
    texture.unmap_memory(allocator);

    // change layout after write
    texture.force_image_layout(app_init, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

    texture
  }

  pub fn from_data(
    device: &ash::Device,
    allocator: &vma::Allocator,
    app_init: &impl WithSetupCmdBuffer,
    name: String,
    format: vk::Format,
    size: vk::Extent2D,
    data_bytes: &Vec<u8>,
  ) -> VkTexture {
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
    let mut texture = VkTexture::empty(
      device,
      allocator,
      name,
      size,
      format,
      vk::ImageTiling::LINEAR,
      vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
      vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    // map image and copy content
    texture.map_memory(allocator);
    texture.write_to_mapped(&data_bytes);
    texture.unmap_memory(allocator);

    // change layout after write
    texture.force_image_layout(app_init, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

    texture
  }

  pub fn force_image_layout(
    &mut self,
    app_init: &impl WithSetupCmdBuffer,
    target_layout: vk::ImageLayout,
  ) {
    let barrier = self.barrier_prepare_for_layout_transition(
      target_layout,
      vk::AccessFlags::empty(),     // src_access_mask
      vk::AccessFlags::SHADER_READ, // dst_access_mask
    );

    // https://vulkan-tutorial.com/Texture_mapping/Images#page_Transition-barrier-masks
    // as early as possible
    let source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
    // do not do any SHADER_READ in FRAGMENT_SHADER before this
    let destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;

    app_init.with_setup_cb(|device, cmd_buf| {
      unsafe {
        device.cmd_pipeline_barrier(
          cmd_buf,
          source_stage,
          destination_stage,
          vk::DependencyFlags::empty(),
          &[],
          &[],
          &[barrier],
        )
      };
    });
  }

  pub fn image_view(&self) -> vk::ImageView {
    self.image_view
  }

  /// If you need extra image view. Needed for depth-stencil textures if we want
  /// read depth only.
  pub fn create_extra_image_view(
    &self,
    device: &ash::Device,
    aspect_mask_flags: vk::ImageAspectFlags,
  ) -> vk::ImageView {
    create_image_view(device, self.image, self.format, aspect_mask_flags)
  }

  /// The `srcStageMask` marks the stages to wait for in previous commands
  /// before allowing the stages given in `dstStageMask` to execute
  /// in subsequent commands.
  ///
  /// ## Docs
  /// * https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples-(Legacy-synchronization-APIs)
  /// * https://www.khronos.org/blog/understanding-vulkan-synchronization
  /// * https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAccessFlagBits.html
  /// * https://vulkan-tutorial.com/Texture_mapping/Images#page_Transition-barrier-masks
  ///
  /// ## Params:
  /// * `new_layout` - next layout to set to e.g. `COLOR_ATTACHMENT_OPTIMAL`
  ///     or `SHADER_READ_ONLY_OPTIMAL`
  /// * `src_access_mask` - previous op e.g. `COLOR_ATTACHMENT_WRITE`
  /// * `dst_access_mask` - op we will do e.g. `COLOR_ATTACHMENT_READ`
  pub fn barrier_prepare_for_layout_transition(
    &mut self,
    new_layout: vk::ImageLayout,
    src_access_mask: vk::AccessFlags,
    dst_access_mask: vk::AccessFlags,
  ) -> vk::ImageMemoryBarrier {
    if DEBUG_LAYOUT_TRANSITIONS {
      trace!(
        "VkTexture::LayoutTransition '{}' ({:?} -> {:?})",
        self.get_name(),
        self.layout,
        new_layout
      );
    }

    let barrier = create_image_barrier(
      self.image,
      self.aspect_flags,
      self.layout,
      new_layout,
      src_access_mask,
      dst_access_mask,
    );

    self.layout = new_layout;
    barrier
  }

  pub fn is_color(&self) -> bool {
    self.aspect_flags == vk::ImageAspectFlags::COLOR
  }

  pub fn is_depth_stencil(&self) -> bool {
    self.aspect_flags == (vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
  }

  pub fn barrier_prepare_attachment_for_shader_read(&mut self) -> vk::ImageMemoryBarrier {
    if self.is_color() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE, // prev op
        vk::AccessFlags::SHADER_READ,            // our op
      )
    } else if self.is_depth_stencil() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, // prev op
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,  // our op
      )
    } else {
      panic!("Tried to transition texture {} for shader read, but it's neither color or depth-stencil texture.", self.get_name());
    }
  }

  pub fn barrier_prepare_attachment_for_write(&mut self) -> vk::ImageMemoryBarrier {
    if self.is_color() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::SHADER_READ,            // prev op
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE, // our op
      )
    } else if self.is_depth_stencil() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ, // prev op
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, // our op
      )
    } else {
      panic!("Tried to transition texture {} for shader write, but it's neither color or depth-stencil texture.", self.get_name());
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

  fn get_allocation(&mut self) -> &mut vma::Allocation {
    &mut self.allocation
  }

  fn get_mapped_pointer(&self) -> Option<MemoryMapPointer> {
    self.mapped_pointer.clone()
  }
  fn set_mapped_pointer(&mut self, next_ptr: Option<MemoryMapPointer>) {
    self.mapped_pointer = next_ptr;
  }
}

fn create_texture_name(name: String, width: u32, height: u32) -> String {
  format!("VkTexture({}, {}x{})", name, width, height)
}
