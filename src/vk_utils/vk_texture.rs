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

pub struct VkTexture {
  // For debugging
  pub name: String,
  pub width: u32,
  pub height: u32,
  /// Native Vulkan image
  pub image: vk::Image,
  image_view: Option<vk::ImageView>, // TODO can we create it before write. Probably yes, so remove Option
  pub layout: vk::ImageLayout,
  pub allocation: vma::Allocation,
  // mapping
  mapped_pointer: Option<MemoryMapPointer>,
}

impl VkTexture {
  pub fn new(
    device: &ash::Device,
    allocator: &vma::Allocator,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    tiling: vk::ImageTiling, // always vk::ImageTiling::OPTIMAL?
    usage: vk::ImageUsageFlags,
    aspect: vk::ImageAspectFlags,
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
      required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
      ..Default::default()
    };

    let (image, allocation) = unsafe {
      allocator
        .create_image(&create_info, &alloc_info)
        .expect("Failed allocating GPU memory for texture")
    };

    let image_view = create_image_view(device, image, create_info.format, aspect);

    VkTexture {
      name,
      width: size.width,
      height: size.height,
      image,
      allocation,
      mapped_pointer: None,
      layout: create_info.initial_layout,
      image_view: Some(image_view),
    }
  }

  pub fn from_file(
    allocator: &vma::Allocator,
    app_init: &impl WithSetupCmdBuffer,
    path: &std::path::Path,
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
    let width = metadata.width as u32;
    let height = metadata.height as u32;

    // vulkan part starts here
    let create_info = vk::ImageCreateInfo::builder()
      .image_type(vk::ImageType::TYPE_2D)
      .extent(vk::Extent3D {
        width,
        height,
        depth: 1,
      })
      .format(vk::Format::R8G8B8A8_SRGB)
      .tiling(vk::ImageTiling::LINEAR) // Optimal if uploaded from staging buffer. Linear if written from CPU(!!!)
      .mip_levels(1)
      .array_layers(1)
      .usage(vk::ImageUsageFlags::SAMPLED)
      // https://stackoverflow.com/questions/76945200/how-to-properly-use-vk-image-layout-preinitialized
      .initial_layout(vk::ImageLayout::PREINITIALIZED) // VK_IMAGE_LAYOUT_GENERAL and ignore layouts? 
      .sharing_mode(vk::SharingMode::EXCLUSIVE)
      .samples(vk::SampleCountFlags::TYPE_1)
      .build();

    #[allow(deprecated)]
    let alloc_info = vma::AllocationCreateInfo {
      usage: vma::MemoryUsage::GpuOnly,
      required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
        | vk::MemoryPropertyFlags::HOST_COHERENT,
      ..Default::default()
    };

    let (image, allocation) = unsafe {
      allocator
        .create_image(&create_info, &alloc_info)
        .expect("Failed allocating GPU memory for texture")
    };

    let name = path.file_name().unwrap_or(OsStr::new(path));
    let mut texture = VkTexture {
      name: name.to_string_lossy().to_string(),
      width,
      height,
      image,
      allocation,
      mapped_pointer: None,
      image_view: None,
      layout: create_info.initial_layout,
    };

    // map image and copy content
    texture.map_memory(allocator);
    texture.write_to_mapped(&pixel_bytes);
    texture.unmap_memory(allocator);

    // change layout after write
    app_init.with_setup_cb(|device, cmd_buf| {
      let target_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
      let barrier = create_image_barrier(
        image,
        vk::ImageAspectFlags::COLOR,
        create_info.initial_layout,
        target_layout,
        vk::AccessFlags::empty(),     // src_access_mask
        vk::AccessFlags::SHADER_READ, // dst_access_mask
      );

      // https://vulkan-tutorial.com/Texture_mapping/Images#page_Transition-barrier-masks
      // as early as possible
      let source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
      // do not do any SHADER_READ in FRAGMENT_SHADER before this
      let destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;

      // barrier impl
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
      texture.layout = target_layout;

      // not part of cmd_buf, but too lazy to provide ash::Device to Vktexture::from_file
      let image_view = create_image_view(
        device,
        image,
        create_info.format,
        vk::ImageAspectFlags::COLOR,
      );
      texture.image_view = Some(image_view);
    });

    texture
  }

  pub fn image_view(&self) -> vk::ImageView {
    self
      .image_view
      .expect("Tried to access VkTexture.image_view before it was initialized")
  }

  pub unsafe fn delete(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    if let Some(iv) = self.image_view {
      device.destroy_image_view(iv, None);
    }
    allocator.destroy_image(self.image, &mut self.allocation)
  }
}

#[allow(dead_code)]
fn create_artificial_texture(w: u32, h: u32) -> Vec<u8> {
  let pixel_cnt = (w * h) as usize;
  let mut data: Vec<u8> = Vec::with_capacity(pixel_cnt * 4);

  (0..pixel_cnt).for_each(|idx| {
    if idx % 8 >= 4 {
      data.push(255u8); // red
      data.push(0u8);
      data.push(0u8);
    } else {
      data.push(0u8); // green
      data.push(255u8);
      data.push(0u8);
    }
    data.push(255u8);
  });

  data
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
