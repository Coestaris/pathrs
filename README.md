# pathrs
Path tracer using Vulkan in Rust 

TODO list:
- [ ] Draw a fullscreen quad in a Windowed frontend
- [ ] Implement simple resource management for loading shaders, textures, models
- [ ] Implement a basic compute shader that fills offscreen texture
- [ ] Add egui for debugging purposes
- [ ] Implement a basic path tracing algorithm in the compute shader for spheres:
  - [ ] Intersection tests
  - [ ] Material handling
  - [ ] Multisampling
- [ ] Headless mode (without window)

References:
- https://github.com/Zydak/Vulkan-Path-Tracer?tab=readme-ov-file
- https://kylemayes.github.io/vulkanalia
- https://vulkan-tutorial.com/Introduction
- https://github.com/SaschaWillems/Vulkan
- 