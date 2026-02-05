# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Vulkan renderer written in Rust using the `vulkanalia` bindings (v0.33.0). The project renders 3D models with textures using modern Vulkan features including MSAA, depth buffering, and mipmapping.

## Build Commands

```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the application
cargo run

# Run with logging enabled
RUST_LOG=debug cargo run

# Check code without building
cargo check
```

## Shader Compilation

Shaders are in GLSL and must be compiled to SPIR-V:

```bash
cd shaders
./compile.sh
```

The compile script uses `glslc` (from the Vulkan SDK) to compile:
- `shader.vert` → `vert.spv`
- `shader.frag` → `frag.spv`

## Architecture

### Two-Layer Architecture

The application follows a clear separation between lifetime tiers:

1. **VulkanContext** (`src/vulkan/context.rs`) - Application lifetime resources
   - Vulkan instance, device, and physical device
   - Surface and queues (graphics and present)
   - Debug messenger
   - Command pool
   - Lives for the entire application lifetime

2. **Renderer** (`src/vulkan/renderer.rs`) - Swapchain-dependent resources
   - Swapchain and its image views
   - Render pass, pipeline, and framebuffers
   - MSAA color attachments and depth buffers
   - Uniform buffers and descriptor sets
   - Command buffers and sync objects
   - Recreated on window resize/swapchain invalidation

3. **App** (`src/app.rs`) - Application state and scene resources
   - Owns both VulkanContext and Renderer
   - Scene data (vertices, indices, textures) - immutable after creation
   - Application state (resize flag, frame timing)
   - Coordinates rendering and swapchain recreation

### Vulkan Module Organization

The `src/vulkan/` directory contains specialized modules:
- `instance.rs` - Instance creation with validation layers
- `physical_device.rs` - Physical device selection
- `logical_device.rs` - Logical device and queue creation
- `device.rs` - Queue family indices and swapchain support utilities
- `swapchain.rs` - Swapchain creation and management
- `pipeline.rs` - Graphics pipeline and render pass
- `buffer.rs` - Vertex, index, and uniform buffer creation
- `image.rs` - Image creation (textures, depth, color attachments)
- `texture.rs` - Texture loading and mipmap generation
- `descriptors.rs` - Descriptor sets and layouts
- `command.rs` - Command pool and buffer creation
- `sync.rs` - Synchronization objects (semaphores, fences)
- `helpers.rs` - Utility functions

### Configuration System

The app uses `config.toml` for runtime configuration (loaded via `src/config.rs`):
- Window settings (title, dimensions)
- Vulkan settings (validation, max frames in flight)
- Shader paths
- Resource paths (models, textures)
- Camera settings
- Demo settings

If `config.toml` is missing, embedded defaults from `src/config.rs` are used.

### Resource Management

**Ownership pattern:**
- VulkanContext owns application-lifetime Vulkan objects
- Renderer owns swapchain-lifetime objects
- App owns scene resources (buffers, textures) that persist across swapchain recreation
- Each owner implements its own `destroy()` method with proper cleanup order

**IMPORTANT:** When destroying resources, always call `device.device_wait_idle()` first to ensure no resources are in use.

### Rendering Pipeline

1. `main.rs` creates window and event loop
2. App initializes VulkanContext, loads model/texture, creates buffers
3. App creates Renderer with swapchain-dependent resources
4. Each frame:
   - Renderer acquires swapchain image
   - Updates uniform buffers (view/projection matrices)
   - Records command buffer with model transforms (push constants)
   - Submits to graphics queue
   - Presents to screen
5. On window resize:
   - App sets `resized` flag
   - Renderer recreates swapchain and all dependent resources
   - Scene buffers/textures are preserved

### Shader Interface

**Vertex Shader** (`shader.vert`):
- Input: position (vec3), color (vec3), tex coords (vec2)
- Uniform: view and projection matrices
- Push constant: model matrix (mat4)
- Output: transformed position, color, tex coords

**Fragment Shader** (`shader.frag`):
- Input: tex coords from vertex shader
- Uniform: texture sampler
- Push constant (offset 64): opacity (float)
- Output: textured color with alpha

Push constants allow per-draw-call data without updating descriptor sets.

### Vertex Structure

Defined in `src/types.rs`:
```rust
struct Vertex {
    pos: Vec3,      // Position
    color: Vec3,    // Color (currently unused in fragment shader)
    tex_coord: Vec2 // Texture coordinates
}
```

Vertices are loaded from models (see `src/vulkan/model.rs`).

## Key Implementation Notes

- **Unsafe Code**: Most Vulkan operations are `unsafe`. The codebase uses `#![allow(unsafe_op_in_unsafe_fn)]` for brevity
- **Error Handling**: Uses `anyhow::Result` throughout
- **Frame-in-Flight**: Supports 2 frames in flight (MAX_FRAMES_IN_FLIGHT constant in renderer.rs)
- **MSAA**: Uses multi-sampling anti-aliasing (sample count determined from physical device)
- **Mipmaps**: Textures use automatic mipmap generation
- **Coordinate System**: Uses cgmath with GLM-style right-handed coordinates; applies correction matrix for Vulkan's clip space

## Common Pitfalls

1. **Resource Destruction Order**: Always destroy child resources before parents (e.g., image views before images)
2. **Swapchain Recreation**: Must recreate render pass, pipeline, framebuffers, and command buffers when swapchain is recreated
3. **Push Constant Offsets**: Fragment shader push constant starts at offset 64 (after the vertex shader's mat4)
4. **Validation Layers**: Enable via `validation_enabled = true` in config.toml for debugging
5. **Command Pool Reset**: Use per-image command pools to allow parallel command buffer recording

## Dependencies

- `vulkanalia` - Vulkan bindings (pinned to v0.33.0)
- `winit` - Window creation and event handling
- `cgmath` - Math library for vectors and matrices
- `anyhow` - Error handling
- `log` + `pretty_env_logger` - Logging infrastructure
- `serde` + `toml` - Configuration parsing

Requires Vulkan SDK and Vulkan-capable GPU to run.
