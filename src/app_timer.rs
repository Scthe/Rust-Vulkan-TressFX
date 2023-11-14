use std::{collections::VecDeque, time::Instant};

use crate::utils::get_simple_type_name;

// Delta times are filtered over _this many_ frames.
const DT_FILTER_WIDTH: usize = 20;

pub type FrameIdx = u64;

/// Heavily inspired by:
/// - https://github.com/EmbarkStudios/kajiya/blob/main/crates/lib/kajiya-simple/src/main_loop.rs#L329
/// - https://github.com/kayru/imgv/blob/main/src/main.rs#L918
pub struct AppTimer {
  frame_idx: FrameIdx,
  /// Provide fake `delta time` for  _this many_ initial frames.
  /// Smooths out simulation etc. at the start.
  fake_dt_for_initial_frames: i32,
  last_frame_start: Instant,
  delta_time: f32,
  /// Circular buffer for delta times
  dt_queue: VecDeque<f32>,
  // TODO Time spend on CPU (so not counting waiting on semaphors). In seconds.
  // cpu_draw_calls_duration: f32,
}

impl AppTimer {
  pub fn new() -> Self {
    Self {
      frame_idx: 0,
      fake_dt_for_initial_frames: 2 * (DT_FILTER_WIDTH as i32),
      last_frame_start: Instant::now(),
      delta_time: 0.0,
      dt_queue: VecDeque::with_capacity(DT_FILTER_WIDTH),
    }
  }

  #[allow(dead_code)]
  pub fn frame_idx(&self) -> FrameIdx {
    self.frame_idx - 1 // we start at 0 and immediately INC in `mark_start_frame` - fix it here
  }

  /// @return delta time in seconds
  pub fn mark_start_frame(&mut self) -> f32 {
    self.inc_frame_idx();

    let now = Instant::now();
    let dt_duration = now - self.last_frame_start;
    self.last_frame_start = now;

    let dt_raw = dt_duration.as_secs_f32();
    let delta_time = if self.fake_dt_for_initial_frames >= 0 {
      self.fake_dt_for_initial_frames -= 1;
      dt_raw.min(1.0 / 60.0)
    } else {
      while self.dt_queue.len() >= DT_FILTER_WIDTH {
        self.dt_queue.pop_front();
      }
      self.dt_queue.push_back(dt_raw);

      self.calc_average_frame_time()
    };

    self.delta_time = delta_time;
    self.delta_time
  }

  fn calc_average_frame_time(&self) -> f32 {
    let sum = self.dt_queue.iter().copied().sum::<f32>();
    let count = self.dt_queue.len();
    sum / (count as f32)
  }

  fn inc_frame_idx(&mut self) {
    match self.frame_idx.checked_add(1) {
      Some(e) => self.frame_idx = e,
      _ => panic!(
        "Integer overflow in {}.inc_frame_idx(). How long did the app run?!",
        get_simple_type_name::<Self>()
      ),
    }
  }

  pub fn delta_time_ms(&self) -> f32 {
    self.delta_time * 1000.0
  }
}
