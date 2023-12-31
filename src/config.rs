use ash;
use ash::vk;
use glam::{vec2, vec4, Vec2, Vec3, Vec4};

use crate::utils::color_hex_to_vec;

use self::tfx_simulation::TfxSimulation;
pub use self::{camera::*, color_grading::*, light::*, postfx::*, shadows::*, ssao::*, sss::*};

pub mod camera;
pub mod color_grading;
pub mod light;
pub mod postfx;
pub mod shadows;
pub mod ssao;
pub mod sss;
pub mod tfx_simulation;

// Must match consts in `present.frag.glsl`.
pub enum DisplayMode {
  Final = 0,
  Normals = 1,
  Luma = 2,
  SSAO = 3,
  LinearDepth = 4,
  ShadowMap = 5,
  SSSContribution = 6,
  SSSThickness = 7,
}

pub enum HairTechnique {
  PPLL = 0,
  Solid = 1,
}

/// Must match consts in `tfx_ppll_resolve.frag.glsl`
pub enum HairPPLLDisplayMode {
  Final = 0,
  Flat = 1,
  PpllOverlap = 2,
  Tangents = 3,
  Coverage = 4,
}

/// Must match consts in `tfx_forward.frag.glsl`.
pub enum HairSolidDisplayMode {
  Final = 0,
  Flat = 1,
  FollowGroups = 2,
  Strands = 3,
  RootTipPercentage = 4,
}

/// https://github.com/Scthe/WebFX/blob/master/src/Config.ts
pub struct Config {
  /// crash program after first frame to read init errors
  pub only_first_frame: bool,
  pub frames_in_flight: usize,
  /// build type: debug or release
  is_release: bool,
  /// run profiler
  pub profile_next_frame: bool,
  /// Ui has requested to reset simulation state to initial
  pub reset_tfx_simulation_next_frame: bool,
  /// show spheres where lights/shadows are
  pub show_debug_positions: bool,
  /// debug display mode
  pub display_mode: usize,
  pub linear_depth_preview_range: Vec2,
  // window
  pub window_width: f64,
  pub window_height: f64,
  vsync: bool,
  // clear colors
  pub clear_color: Vec3,
  pub clear_normal: [u32; 4],
  pub clear_depth: f32,
  pub clear_stencil: i8,
  // scene-related
  pub model_scale: f32,
  pub camera: CameraConfig,
  pub hair_technique: usize,
  pub hair_ppll_display_mode: usize,
  pub hair_solid_display_mode: usize,
  pub tfx_simulation: TfxSimulation,
  // lights
  pub light_ambient: LightAmbient,
  pub light0: LightCfg,
  pub light1: LightCfg,
  pub light2: LightCfg,
  // shadows
  pub shadows: ShadowsConfig,
  // sss
  pub sss_forward_scatter: SSSForwardScatterPassCfg,
  pub sss_blur: SSSBlurPassCfg,
  // ssao
  pub ssao: SSAOConfig,
  // postfx
  pub postfx: PostFxCfg,
  // misc
  // showDebugPositions = false;
  // useMSAA = true; // ok, technically it's brute force supersampling, but who cares?
  // center_of_gravity: vec3(0, 3.0, 0), // used for calulating hair normals (remember, no cards!)

  // TressFX collision spheres
  pub debug_collision_sphere0: Vec4,
  pub debug_collision_sphere1: Vec4,
  pub debug_collision_sphere2: Vec4,
  pub debug_collision_sphere3: Vec4,
}

impl Config {
  const ONLY_FIRST_FRAME: bool = false;
  const PROFILE_FIRST_FRAME: bool = false;
  pub const DEBUG_LAYOUT_TRANSITIONS: bool = false;

  /// Test transparency for swapchain image.
  /// Requires alpha channel added to "assets\shaders\present.frag.glsl"
  pub const TEST_ALPHA_COMPOSITE: bool = false;

  pub const STENCIL_BIT_SKIN: u32 = 1 << 0;
  pub const STENCIL_BIT_HAIR: u32 = 1 << 1;

  pub fn new() -> Config {
    let clear_col: u8 = 93;

    Config {
      only_first_frame: Self::ONLY_FIRST_FRAME,
      frames_in_flight: 2,
      is_release: false, // TODO [CRITICAL] from Cargo build type or cmd line args. Apply to `compile_shaders.py` too
      profile_next_frame: Self::PROFILE_FIRST_FRAME,
      reset_tfx_simulation_next_frame: false,
      show_debug_positions: false,
      display_mode: DisplayMode::Final as _,
      linear_depth_preview_range: vec2(-2.0, -15.0),
      // window
      window_width: 1280f64,
      window_height: 720f64,
      vsync: true,
      // clear colors
      clear_color: color_hex_to_vec(clear_col, clear_col, clear_col),
      clear_normal: [0, 0, 0, 0], // or [1,1,1,1] for performance reasons
      clear_depth: 1.0,
      clear_stencil: 0,
      // scene
      model_scale: 0.3,
      camera: CameraConfig::default(),
      hair_technique: HairTechnique::PPLL as _,
      hair_ppll_display_mode: HairPPLLDisplayMode::Final as _,
      hair_solid_display_mode: HairSolidDisplayMode::Final as _,
      tfx_simulation: TfxSimulation::default(),
      // lights
      light_ambient: LightAmbient::default(),
      light0: LightCfg::light0(),
      light1: LightCfg::light1(),
      light2: LightCfg::light2(),
      // material + lights
      ssao: SSAOConfig::default(),
      shadows: ShadowsConfig::default(),
      sss_forward_scatter: SSSForwardScatterPassCfg::default(),
      sss_blur: SSSBlurPassCfg::default(),
      // postfx
      postfx: PostFxCfg::default(),
      // TressFX collision spheres
      debug_collision_sphere0: vec4(0.0, 0.0, 0.0, 0.0),
      debug_collision_sphere1: vec4(0.0, 0.0, 0.0, 0.0),
      debug_collision_sphere2: vec4(0.0, 0.0, 0.0, 0.0),
      debug_collision_sphere3: vec4(0.0, 0.0, 0.0, 0.0),
    }
  }

  pub fn get_viewport_size(&self) -> vk::Extent2D {
    vk::Extent2D {
      width: self.window_width as u32,
      height: self.window_height as u32,
    }
  }

  pub fn vsync(&self) -> bool {
    self.vsync
  }

  pub fn is_release(&self) -> bool {
    self.is_release
  }

  pub fn get_ssao_viewport_size(&self) -> vk::Extent2D {
    vk::Extent2D {
      width: (self.window_width as u32) / self.ssao.texture_size_div,
      height: (self.window_height as u32) / self.ssao.texture_size_div,
    }
  }

  pub fn clear_color(&self) -> vk::ClearValue {
    let cc = self.clear_color;
    vk::ClearValue {
      color: vk::ClearColorValue {
        float32: [cc[0], cc[1], cc[2], 1f32],
      },
    }
  }

  pub fn clear_normals(&self) -> vk::ClearValue {
    vk::ClearValue {
      color: vk::ClearColorValue {
        uint32: self.clear_normal,
      },
    }
  }

  pub fn clear_depth_stencil(&self) -> vk::ClearValue {
    vk::ClearValue {
      depth_stencil: vk::ClearDepthStencilValue {
        depth: self.clear_depth,
        stencil: self.clear_stencil as u32,
      },
    }
  }

  pub fn clear_swapchain_color(&self) -> vk::ClearValue {
    vk::ClearValue {
      color: vk::ClearColorValue {
        uint32: [0, 0, 0, 0],
      },
    }
  }

  /// Some debug display modes write to forward pass result.
  /// Avoid overriding them.
  pub fn preserve_original_forward_pass_result(&self) -> bool {
    let preserve_org_result = [
      DisplayMode::ShadowMap as usize,
      DisplayMode::SSSContribution as usize,
      DisplayMode::SSSThickness as usize,
    ];
    preserve_org_result.contains(&self.display_mode)
  }

  pub fn fxaa_edge_threshold(&self) -> f32 {
    if self.postfx.use_fxaa {
      self.postfx.edge_threshold
    } else {
      0.0
    }
  }

  pub fn get_camera_fov_y(&self) -> f32 {
    (self.camera.fov_dgr / self.window_width as f32) * self.window_height as f32
  }

  pub fn is_hair_using_ppll(&self) -> bool {
    self.hair_technique != (HairTechnique::Solid as _)
  }

  pub fn get_hair_display_mode(&self) -> usize {
    let allow_debug_mode = self.display_mode == (DisplayMode::Final as _);
    if allow_debug_mode {
      if self.is_hair_using_ppll() {
        return self.hair_ppll_display_mode;
      } else {
        return self.hair_solid_display_mode;
      }
    }

    // Force render final pixel value, so 'general' debug modes can use it.
    // This includes e.g. luma (takes render final output and greyscales it) etc.
    if self.is_hair_using_ppll() {
      return HairPPLLDisplayMode::Final as _;
    } else {
      return HairSolidDisplayMode::Final as _;
    }
  }
}
