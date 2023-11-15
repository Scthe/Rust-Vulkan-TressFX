use std::iter;

use ash::vk;
use log::trace;

use crate::vk_ctx::VkCtx;

pub type ScopeId = u32;
/// Name of the scope and timing in milliseconds.
pub type TimesScope = (String, f32);
/// List of scopes and their times
pub type GpuProfilerReport = Vec<TimesScope>;

#[derive(Debug)]
struct ProfilerScope {
  name: String,
}

/// Big amount of queries to never have to carry about it
const MAX_QUERY_COUNT: u32 = 1024;
/// Each pass has BEGIN and END timestamp query
const QUERIES_PER_PASS: u32 = 2;
const TOTAL_MAX_QUERIES: u32 = MAX_QUERY_COUNT * QUERIES_PER_PASS;
/// Magic value to disable recording the timestamps for this scope
const SCOPE_ID_IGNORED: ScopeId = MAX_QUERY_COUNT * 10;

/// ### Docs
/// - https://nikitablack.github.io/post/how_to_use_vulkan_timestamp_queries/ <- very good!
/// - https://docs.vulkan.org/spec/latest/chapters/queries.html#queries-timestamps
///
/// ### References
/// - https://github.com/h3r2tic/gpu-profiler/blob/main/src/shared.rs
pub struct GpuProfiler {
  enabled: bool,
  query_pool: vk::QueryPool,
  scopes: Vec<ProfilerScope>,
  pub last_report: Option<GpuProfilerReport>,
}

impl GpuProfiler {
  pub fn new(vk_ctx: &VkCtx) -> Self {
    let device = vk_ctx.vk_device();
    let pool_info = vk::QueryPoolCreateInfo::builder()
      .query_type(vk::QueryType::TIMESTAMP)
      .query_count(TOTAL_MAX_QUERIES);
    let query_pool = unsafe {
      device
        .create_query_pool(&pool_info, None)
        .expect("Failed to create query pool")
    };

    Self {
      enabled: false,
      query_pool,
      scopes: Vec::new(),
      last_report: None,
    }
  }

  pub unsafe fn destroy(&mut self, device: &ash::Device) {
    device.destroy_query_pool(self.query_pool, None);
  }

  pub fn set_enabled(&mut self, enabled: bool) {
    self.enabled = enabled
  }

  /// Start profiling - will be very slow as we wait after each frame to readback the results
  pub fn begin_frame(&mut self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
    self.scopes.clear();

    unsafe {
      device.cmd_reset_query_pool(command_buffer, self.query_pool, 0, TOTAL_MAX_QUERIES);
    }
  }

  /// End profiling and wait for result.
  pub fn end_frame(&mut self, device: &ash::Device) {
    if self.enabled {
      self.last_report = self.create_queries_report(device);
    }
  }

  pub fn begin_scope(
    &mut self,
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    name: &str,
  ) -> ScopeId {
    if !self.enabled {
      return SCOPE_ID_IGNORED;
    }

    let query_id: u32 = self.scopes.len() as _;
    self.scopes.push(ProfilerScope {
      name: name.to_string(),
    });

    unsafe {
      device.cmd_write_timestamp(
        command_buffer,
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        self.query_pool,
        query_id * QUERIES_PER_PASS,
      );
    }

    query_id
  }

  pub fn end_scope(
    &self,
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    scope_id: ScopeId,
  ) {
    if !self.enabled || scope_id == SCOPE_ID_IGNORED {
      return;
    }

    let query_id: u32 = scope_id;

    unsafe {
      device.cmd_write_timestamp(
        command_buffer,
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        self.query_pool,
        query_id * QUERIES_PER_PASS + 1,
      );
    }
  }

  fn create_queries_report(&self, device: &ash::Device) -> Option<GpuProfilerReport> {
    if self.scopes.is_empty() {
      trace!("No profiling scopes added to the frame");
      return None;
    }

    trace!("Reading profiling result for {:?}", self.scopes);
    let scopes_count = self.scopes.len() as u32;
    let query_count = scopes_count * QUERIES_PER_PASS;

    let mut durations: Vec<u64> = iter::repeat(0u64).take(query_count as _).collect();
    unsafe {
      device
        .get_query_pool_results(
          self.query_pool,
          0,
          query_count,
          durations.as_mut_slice(),
          vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
        )
        .expect("Failed to get profiler results")
    };

    let report: GpuProfilerReport = self
      .scopes
      .iter()
      .enumerate()
      .map(|(scope_idx, scope)| {
        let base_query = scope_idx * (QUERIES_PER_PASS as usize);
        let time_start = durations[base_query];
        let time_end = durations[base_query + 1];
        let duration_nano = time_end - time_start;
        let duration_ms = (duration_nano as f32) * 0.000001;
        trace!("'{}' took {:.2}ms", scope.name, duration_ms);
        (scope.name.clone(), duration_ms)
      })
      .collect();

    Some(report)
  }
}
