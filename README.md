# pathrs
Path tracer using Vulkan in Rust 

TODO list:
- [x] Draw a triangle
- [x] Draw a fullscreen quad in a Windowed frontend
- [x] Implement simple resource management for loading shaders, textures, models
- [x] Implement a basic compute shader that fills offscreen texture
- [x] Add [egui](https://github.com/emilk/egui) for debugging purposes
- [ ] Headless mode (without window)
  - [x] Offscreen rendering to a texture
  - [ ] Temporal accumulation 
- [x] Implement a basic path tracing algorithm in the compute shader for spheres:
  - [x] Sphere intersection
  - [x] Light scattering
  - [x] Temporal accumulation
  - [x] Antialiasing
  - [ ] Materials (diffuse, metal, dielectric)

### References

Vulkan tutorials and examples:
- [Vulkan tutorial](https://vulkan-tutorial.com/Introduction) by Alexander Overvoorde;
- [Vulkan Guide](https://vkguide.dev/) by Victor Blanco;
- [Repository of Vulkan Examples](https://github.com/SaschaWillems/Vulkan) by Sascha Willems (and others);
- [Rust oriented another Vulkan tutorial](https://kylemayes.github.io/vulkanalia) by Kyle Mayes.

Raytracing tutorials and examples:
- [Ray Tracing in One Weekend](https://raytracing.github.io/books/RayTracingInOneWeekend.html) by Peter Shirley;
- [Ray Tracing: The Next Week](https://raytracing.github.io/books/RayTracingTheNextWeek.html) by Peter Shirley;
- [Ray Tracing: The Rest of Your Life](https://raytracing.github.io/books/RayTracingTheRestOfYourLife.html) by Peter Shirley;
- [Smallpt: Global Illumination in 99 lines of C++](https://www.kevinbeason.com/smallpt/) by Kevin Beason;
- [A Vulkan path tracer in C++](https://github.com/Zydak/Vulkan-Path-Tracer) by Zydak;
- [YouTube | I wrote a Ray Tracer from scratch... in a Year](https://www.youtube.com/watch?v=wzZJzyX0UkI) by Jacob Gordiak.

And an amazing ray / path-tracing series by Sebastian Lague:
- [YouTube | Coding Adventure: Ray Tracing](https://www.youtube.com/watch?v=Qz0KTGYJtUk): Implementation of BSDF, antialiasing and DOF;
- [YouTube | Coding Adventure: More Ray Tracing!](https://www.youtube.com/watch?v=C1H4zIiCOaI): BVH acceleration structure;
- [YouTube | Coding Adventure: Ray-Tracing Glass and Caustics](https://www.youtube.com/watch?v=wA1KVZ1eOuA): Glass, absorption, and caustics.
- [YouTube | Coding Adventure: Clouds](https://www.youtube.com/watch?v=4QOcCGI6xOU): Light scattering and ray marching.
- [YouTube | Coding Adventure: Atmosphere](https://www.youtube.com/watch?v=DxfEbulyFcY): Light scattering and fog