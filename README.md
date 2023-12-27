# Rust Vulkan TressFX

This is an implementation of [AMD's TressFX](https://gpuopen.com/tressfx/) hair rendering and simulation technology using Rust and Vulkan.



https://github.com/Scthe/Rust-Vulkan-TressFX/assets/9325337/d558c2cb-8164-47f0-903c-1346d2829c71

*Showcase adjusting real-time shadows using the UI*


[TressFX](https://gpuopen.com/tressfx/) is AMD's library used for the simulation and rendering of hair. It's been used in commercial games like the newest Tomb Raider titles and Deus Ex: Mankind Divided. The library itself is [open source](https://github.com/GPUOpen-Effects/TressFX) under GPUOpen initiative. Please visit the provided links to get more details.

Previously, I've already [ported TressFX to OpenGL with C++](https://github.com/Scthe/TressFX-OpenGL). It was mainly required to provide bindings for AMD's framework functions and translate HLSL into GLSL shader code. The app contained both the rendering and the simulation part. Later on, I created [WebFX](https://github.com/Scthe/WebFX) ([Demo](http://scthe.github.io/WebFX/dist)) - in browser viewer for TressFX files. Due to WebGL limitations (no compute shaders), it only contained the rendering part.




https://github.com/Scthe/Rust-Vulkan-TressFX/assets/9325337/3f99198f-425c-4eb9-94d4-65b18c593339

*TressFX simulation: adjusting the wind strength*


Based on this project, I've also written a series of Vulkan articles:

* ["Vulkan initialization"](https://www.sctheblog.com/blog/vulkan-initialization/)
* ["Vulkan synchronization"](https://www.sctheblog.com/blog/vulkan-synchronization/)
* ["Vulkan resources"](https://www.sctheblog.com/blog/vulkan-resources/)
* ["A typical Vulkan frame"](https://www.sctheblog.com/blog/vulkan-frame/)
* ["Debugging Vulkan using RenderDoc"](https://www.sctheblog.com/blog/debugging-vulkan-using-renderdoc/)



## Usage

Requires `glslc` in `PATH`. By default, debug data is added to shaders, which requires `glslangValidator`. If you want to skip this last step, set `ADD_DEBUG_DATA = False` in [compile_shaders.py](compile_shaders.py).

Run `make run` to:
1. Compile shaders (it just calls [compile_shaders.py](compile_shaders.py)) to SPIR-V
2. Build and run the main rust app (`cargo run`).

Use the `[W, S, A, D]` keys to move and `[Z, SPACEBAR]` to fly up or down. Click and drag to rotate the camera (be careful around the UI). All materials, effects, rendering and simulation techniques are configurable using the UI on the left side of the screen.

## FAQ

**Q: Which effects are implemented?**

- TressFX - both simulation and [Per-Pixel Linked Lists (PPLL)](https://www.cs.cornell.edu/~bkovacs/resources/TUBudapest-Barta-Pal.pdf).
- Kajiya-Kay hair shading (with small custom modifications) [Kajiya89](https://www.cs.drexel.edu/~david/Classes/CS586/Papers/p271-kajiya.pdf), [Scheuermann04](http://web.engr.oregonstate.edu/~mjb/cs519/Projects/Papers/HairRendering.pdf)
- PBR materials (small modifications to AO term to highlight details like collarbones, similar to micro shadow hack in [Uncharted4](http://advances.realtimerendering.com/other/2016/naughty_dog/NaughtyDog_TechArt_Final.pdf)) [Burley12](https://disney-animation.s3.amazonaws.com/library/s2012_pbs_disney_brdf_notes_v2.pdf), [Karis13](https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf), [Lagarde+2014](https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf), [in OpenGL](https://learnopengl.com/PBR/Theory)
  - Cook-Torrance model
  - Diffuse: Lambert
  - **F** Fresnel term: Schlick
  - **D** Normal distribution function: GGX
  - **G** Self-shadowing: GGX-Smith
- SSSSS - both forward scattering (remember [Nathan Drake in Uncharted 4?](https://www.reddit.com/r/gaming/comments/4jc38z/til_in_uncharted_4_under_certain_lighting_drakes/)) and the blur. [Jimenez+15](http://iryoku.com/separable-sss/) with [github](https://github.com/iryoku/separable-sss)
- Shadow Mapping - both [Percentage Closer Filter (PCF)](https://en.wikipedia.org/wiki/Texture_filtering#Percentage_Closer_filtering) and [Percentage-Closer Soft Shadows (PCSS)](http://developer.download.nvidia.com/shaderlibrary/docs/shadow_PCSS.pdf)
- HDR + Tonemapping (just please use ACES) [UE4 docs](https://docs.unrealengine.com/en-us/Engine/Rendering/PostProcessEffects/ColorGrading), [UE4 Feature Highlight video](https://www.youtube.com/watch?v=A-wectYNfRQ), [Wronski16](https://bartwronski.com/2016/08/29/localized-tonemapping/), [Hable10](http://filmicworlds.com/blog/filmic-tonemapping-operators/), [Nvidia - preparing for real HDR](https://developer.nvidia.com/preparing-real-hdr)
- Color Grading - based closely on Unreal Engine 4 implementation. [UE4 docs](https://docs.unrealengine.com/en-us/Engine/Rendering/PostProcessEffects/ColorGrading#colorcorrection), [Fry17](https://www.slideshare.net/DICEStudio/high-dynamic-range-color-grading-and-display-in-frostbite), [Hable17](http://filmicworlds.com/blog/minimal-color-grading-tools/)
- GPU dithering - [8x8 Bayer matrix dithering](https://en.wikipedia.org/wiki/Ordered_dithering)
- SSAO - [John Chapman's blog post](http://john-chapman-graphics.blogspot.com/2013/01/ssao-tutorial.html), [in OpenGL](https://learnopengl.com/Advanced-Lighting/SSAO)
- [FXAA](https://en.wikipedia.org/wiki/Fast_approximate_anti-aliasing) - [Lottes2009](http://developer.download.nvidia.com/assets/gamedev/files/sdk/11/FXAA_WhitePaper.pdf)


**Q: Where can I find ...?**

- [Vulkan initialization](src/vk_ctx/vk_ctx_initialize.rs)
- [GLSL shaders](assets/shaders)
- [Shader compilation script](compile_shaders.py) - handles includes and adds debug metadata for RenderDoc
- [Config file](src/config.rs) - requires recompile, but most of the options are available in UI anyway
- [Render graph](src/render_graph.rs#L143)
- [Render passess](src/render_graph)
- [TressFX simulation passess](src/render_graph/tfx_simulation.rs)
- [TressFX Per-Pixel Linked Lists rendering](src/render_graph/tfx_render.rs)
- [Low level Vulkan utils](src/vk_utils), [VkBuffer wrapper](src/vk_utils/vk_buffer.rs), [VkTexture wrapper](src/vk_utils/vk_texture.rs). For comparison, [VkCtx](src/vk_ctx/vk_ctx.rs) contains all the **instantiated** Vulkan objects (`VK_KHR_swapchain`, `VkPipelineCache`, `vma::Allocator`, synchronization for in-flight-frames etc.).
- [Scene loading, including reading TressFX asset](src/scene/mod.rs)
- [User input wrapper](src/app_input.rs) - fixes some bugs in `winit` when used in games
- [Mini GPU profiler](src/gpu_profiler.rs)


**Q: Why write this project? Isn't [TressFX-OpenGL](https://github.com/Scthe/TressFX-OpenGL) and [WebFX](https://github.com/Scthe/WebFX) [enough](https://www.youtube.com/watch?v=Hy8jp7JAmkg)?**

My main goal was to learn Vulkan. The API is infamous for requiring [1000 LOC to render a single triangle](https://www.reddit.com/r/vulkan/comments/512jvs/does_it_really_take_800_to_1000_lines_of_code/). One can hide tons of concepts and techniques in 1 thousand lines of code!
We can also combine all the goodness of [WebFX](https://github.com/Scthe/WebFX) rendering techniques with compute shader based simulation. As I'm no longer simply porting AMD's framework to OpenGL, I could rewrite and simplify a lot of code.

**Q: How to load new models?**

Authoring new models for TressFX is quite complicated. First, there is the scale. Certain simulation steps require models to have roughly comparable scales. For example, wind displacing a hair strand that has a length of 3 units is vastly different from a hair strand 300 units long. Sintel's model is about 50x50x50 blender units.

Another thing is tweaking all simulation and rendering parameters. All the constraints and forces are hard to debug and adjust.

There are also collision capsules that are hard to get right. When hair near the root intersects with a collision capsule, it is automatically 'pushed away'. Collision resolution has the highest priority. This results in colliding part of the hair strand just ignoring any other simulation forces. One could make the capsules smaller, but that leads to penetration of the object.

**Q: Where can I find TressFX Blender plugin?**

Simple Blender exporter can be found in my original [TressFX-OpenGL](https://github.com/Scthe/TressFX-OpenGL/blob/master/assets/sintel_lite_v2_1/tfx_exporter.py) project.

**Q: Sintel? How cool!**

Well, I made a rule to **not modify the official Sintel lite hair model**. As a result, there are a few issues that can probably be noticed when watching the animations above. The model was just not prepared to handle this kind of simulation.

We can compare this to models from commercial games that utilize TressFX:

- Adam Jensen (Deus Ex: Mankind Divided) has short hair.
- Lara Croft's (Tomb Raider 2013, Rise of the Tomb Raider) ponytail acts more like a ribbon that has simpler interactions with the rest of the model (though I assume it still was a nightmare to get the parameters right). The rest of the hairstyle is rather stiff in comparison. This is in stark contrast to Sintel, where the whole hair is purely under the control of the simulation.
  PS. Rise of The Tomb Raider used an evolution of TressFX 3.0 called [PureHair](https://www.youtube.com/watch?v=wrhSVcZF-1I). In the video, you can see what an experienced artist can do with a system like TressFX. Interestingly, not all hair is simulated, but only a few strands in key places (like bangs). It still gives a dynamic feeling.

**Q: Why Sintel?**

If you know me, you probably know why I like [Sintel](https://durian.blender.org/) so much.

## FAQ Vulkan

**Q: Did you use a render graph?**

No, all code is a straightforward Vulkan with a few utils functions. Render graph would make the code shorter, but someone would have to write it.

**Q: Is Vulkan as verbose as they say?**

Yes. In WebFX my pass to [write linear depth buffer](https://github.com/Scthe/WebFX/blob/master/src/webfx/passes/LinearDepthPass.ts) was 28LOC. In Vulkan [same pass](src/render_graph/linear_depth_pass.rs) is 223LOC. Around half of the Vulkan code is declarations of render targets, uniforms and pipelines. Some libraries can analyze SPIR-V to make a lot of this automatic (especially [VkPipelineLayout](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkPipelineLayout.html)). During the frame loop, this pass also has to set a lot of barriers (which is rare in OpenGL).

**Q: Which Vulkan concept is hardest?**

Around 1/3 of the project time was spent on synchronization. [VkAccessFlagBits](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAccessFlagBits.html) is efficient if you create an API, but a nightmare to work with. There is a lack of clear documentation about which `VkAccessFlagBits` are required by which command. E.g. `vkCmdFillBuffer` mentions only that it's a "transfer" operation (so probably `vk::AccessFlags2::TRANSFER_WRITE` with `vk::PipelineStageFlags2::TRANSFER`), but `vkCmdClearColorImage` has no such mention (it's `vk::AccessFlags2::TRANSFER_WRITE` with `vk::PipelineStageFlags2::CLEAR` according to `vk::AccessFlags2::TRANSFER_WRITE`).
I strongly recommend using `VK_KHR_synchronization2` extension (promoted to Vulkan 1.3). While all concepts stay the same, at least the API is a _tiny_ bit more organized.
Synchronization is also prevalent due to image layout transitions. `vkCmdPipelineBarrier` works differently inside a render pass, which in multithreaded env. would prohibit simplifications like `VkTexture.layout: vk::ImageLayout` to store the previous layout. Fortunately, this app is single-threaded.

**Q: What is your favorite Vulkan feature?**

Vulkan validation layers that intercept Vulkan calls and check provided parameters. Good for checking e.g. `VkPipelineStageFlagBits` vs `VkAccessFlagBits`. It also has best practices and basic synchronization guidelines.

## Honorable mentions

- [AMD TressFX](https://gpuopen.com/tressfx/)
- [AMD VulkanMemoryAllocator](https://github.com/GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator)
- [Ash](https://github.com/ash-rs/ash) - rust wrapper around Vulkan
- [EmbarkStudios's kajiya](https://github.com/EmbarkStudios/kajiya) ❤️ - good reference for Rust+Vulkan renderer
- [Sascha Willems' Vulkan samples](https://github.com/SaschaWillems/Vulkan/tree/master) - especially the [Order-independent transparency](https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L559) one
- [Arseny Kapoulkine's niagara](https://github.com/zeux/niagara) with corresponding [YouTube playlist](https://www.youtube.com/playlist?list=PL0JVLUVCkk-l7CWCn3-cdftR0oajugYvd)
- [imgui](https://github.com/ocornut/imgui) ❤️
- [RenderDoc](https://renderdoc.org/) ❤️
- [Blender](https://www.blender.org/), [Blender Institute](https://www.blender.org/institute/) ❤️
  Sintel's model under [CC 3.0](https://durian.blender.org/sharing/), the character was simplified into a bust. © copyright Blender Foundation | durian.blender.org
