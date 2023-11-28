use std::marker::{Send, Sync};

#[derive(PartialEq)]
pub enum VkMemoryPreference {
  /// Usage: vertex, index buffers, SSBO.
  GpuOnly,
  /// CPU-mapped memory that is read on GPU (e.g. uniforms).
  /// Will be persistently mapped.
  ///
  /// Usage: Uniform buffers.
  GpuMappable,
  /// Temporary allocation used when copying CPU data to GPU-only memory.
  /// No guarantee if it's CPU or GPU. Nor should you care.
  ///
  /// Will be persistently mapped.
  ScratchTransfer,
}

pub fn determine_gpu_allocation_info(
  memory_pref: &VkMemoryPreference,
) -> vma::AllocationCreateInfo {
  match memory_pref {
    VkMemoryPreference::GpuOnly => vma::AllocationCreateInfo {
      usage: vma::MemoryUsage::AutoPreferDevice,
      ..Default::default()
    },
    VkMemoryPreference::GpuMappable => vma::AllocationCreateInfo {
      usage: vma::MemoryUsage::AutoPreferDevice,
      flags: vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
        | vma::AllocationCreateFlags::MAPPED,
      ..Default::default()
    },
    VkMemoryPreference::ScratchTransfer => vma::AllocationCreateInfo {
      usage: vma::MemoryUsage::Auto,
      flags: vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
        | vma::AllocationCreateFlags::MAPPED,
      ..Default::default()
    },
  }
}

pub fn get_persistently_mapped_pointer(
  allocator: &vma::Allocator,
  allocation: &vma::Allocation,
) -> Option<MemoryMapPointer> {
  let alloc_info = allocator.get_allocation_info(&allocation);
  let ptr = alloc_info.mapped_data;
  if ptr.is_null() {
    None
  } else {
    Some(MemoryMapPointer(ptr))
  }
}

/// Wrapper over a raw pointer to make it moveable and accessible from other threads
pub struct MemoryMapPointer(pub *mut ::std::os::raw::c_void);
unsafe impl Send for MemoryMapPointer {}
unsafe impl Sync for MemoryMapPointer {}

impl Clone for MemoryMapPointer {
  #[inline]
  fn clone(&self) -> Self {
    MemoryMapPointer(self.0)
  }

  #[inline]
  fn clone_from(&mut self, source: &Self) {
    self.0 = source.0
  }
}

pub trait VkMemoryResource {
  fn get_name(&self) -> &String;
  fn get_long_name(&self) -> &String;
  fn get_mapped_pointer(&self) -> Option<MemoryMapPointer>;

  fn write_to_mapped(&self, bytes: &[u8]) {
    let mapped_pointer = self.get_mapped_pointer();
    let size = bytes.len();

    if let Some(pointer) = mapped_pointer {
      let slice = unsafe { std::slice::from_raw_parts_mut(pointer.0 as *mut u8, size) };
      slice.copy_from_slice(bytes);
    } else {
      let name = self.get_long_name();
      panic!("Tried to write {} bytes to unmapped '{}'", size, name)
    }
  }
}
