use std::collections::HashSet;

use glam::Vec3;
use log::info;
use winit::event::{
  ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};

use crate::scene::Camera;

/// Other implementations:
/// * https://github.com/rukai/winit_input_helper/blob/main/src/current_input.rs
pub struct AppInput {
  pub close_requested: bool,
  pub key_held: HashSet<VirtualKeyCode>,
  pub mouse_buttons_held: HashSet<MouseButton>,
  pub scroll_delta_y: f32,
  pub is_minimized: bool,
  /// handle losing focus, cursor moving out of window etc.
  pub can_intercept_mouse_events: bool,
}

impl AppInput {
  pub fn new() -> Self {
    Self {
      close_requested: false,
      key_held: HashSet::new(),
      mouse_buttons_held: HashSet::new(),
      scroll_delta_y: 0.0,
      is_minimized: false,
      can_intercept_mouse_events: false, // wait to make sure we REALLY have mouse focus
    }
  }

  pub fn reset_transient_state(&mut self) {
    self.scroll_delta_y = 0.0;
  }

  pub fn handle_event<T>(&mut self, event: &Event<T>, imgui_intercepted: bool) {
    match &event {
      Event::WindowEvent { event, .. } => {
        self.handle_window_event(event, imgui_intercepted);
      }
      // Event::DeviceEvent { event, .. } => {
      // self.handle_device_event(event, imgui_intercepted);
      // }
      _ => (),
    }

    // prevents imgui intercepting `Release` events
    if imgui_intercepted {
      self.key_held.clear();
      self.reset_transient_state();
    }
  }

  fn handle_window_event(&mut self, event: &WindowEvent, imgui_intercepted: bool) {
    match event {
      // on clicked 'x'
      WindowEvent::CloseRequested => {
        self.close_requested = true;
      }
      // keyboard
      WindowEvent::KeyboardInput { input, .. } => match (input.state, input.virtual_keycode) {
        (_, Some(VirtualKeyCode::Escape)) => {
          self.close_requested = true;
        }
        (ElementState::Pressed, Some(key)) if !imgui_intercepted => {
          self.key_held.insert(key);
        }
        (ElementState::Released, Some(key)) => {
          // always handle, regardless of imgui
          self.key_held.remove(&key);
        }
        _ => {}
      },
      // mouse wheel
      WindowEvent::MouseWheel { delta, .. } if !imgui_intercepted => {
        if let MouseScrollDelta::LineDelta(_, delta_y) = delta {
          self.scroll_delta_y = delta_y.clone();
        }
      }
      // mouse buttons
      WindowEvent::MouseInput { button, state, .. } => {
        self.can_intercept_mouse_events = true;
        match *state {
          ElementState::Pressed if !imgui_intercepted => {
            self.mouse_buttons_held.insert(*button);
          }
          ElementState::Released => {
            // always handle, regardless of imgui
            self.mouse_buttons_held.remove(button);
          }
          _ => (),
        }
      }
      // window focus
      WindowEvent::Focused(is_focused) => {
        info!("Window focus change. Are we in focus: {:?}", is_focused);
        self.can_intercept_mouse_events = false;
      }
      WindowEvent::Resized(next_size) => {
        self.is_minimized = next_size.width == 0 && next_size.height == 0;
        info!(
          "Window resized. New size: {:?}, minimized: {}",
          next_size, self.is_minimized
        );
      }
      // cursor left
      WindowEvent::CursorLeft { .. } => {
        info!("Cursor left the window");
        self.can_intercept_mouse_events = false;
      }
      _ => {}
    }
  }

  fn is_pressed(&self, key: VirtualKeyCode) -> bool {
    self.key_held.contains(&key)
  }

  pub fn is_mouse_button_pressed(&self, btn: MouseButton) -> bool {
    self.can_intercept_mouse_events && self.mouse_buttons_held.contains(&btn)
  }

  /// Rust's `Winit` has problem with keyboard keys:
  /// "When user holds the key, winit emits `KEY_PRESS`, waits 0.5s and then
  /// starts emitting subsequent `KEY_PRESS` events".
  /// This results in initial camera movement stutter for 0.5s - bad!
  /// Update per-frame with local set of pressed keys instead.
  pub fn update_camera_position(&self, camera: &mut Camera) {
    let mut move_vector = Vec3::ZERO;

    if self.is_pressed(VirtualKeyCode::W) {
      move_vector.z = -1.0;
    }
    if self.is_pressed(VirtualKeyCode::S) {
      move_vector.z = 1.0;
    }
    if self.is_pressed(VirtualKeyCode::A) {
      move_vector.x = -1.0;
    }
    if self.is_pressed(VirtualKeyCode::D) {
      move_vector.x = 1.0;
    }
    if self.is_pressed(VirtualKeyCode::Z) {
      move_vector.y = -1.0;
    }
    if self.is_pressed(VirtualKeyCode::Space) {
      move_vector.y = 1.0;
    }

    camera.move_(move_vector);

    if self.scroll_delta_y != 0.0 {
      camera.move_forward(-self.scroll_delta_y);
    }
  }
}
