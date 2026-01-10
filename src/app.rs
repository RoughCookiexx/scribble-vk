use anyhow::Result;
use std::time::Instant;
use vulkanalia::prelude::v1_0::*;
use winit::window::Window;

use crate::config::Config;
use crate::types::{Vec2, Vertex};
use crate::vulkan::buffer::create_vertex_buffers;
use crate::vulkan::context::VulkanContext;
use crate::vulkan::renderer::Renderer;

/// The main Vulkan application
pub struct App {
    context: VulkanContext,
    renderer: Renderer,

    // Scene resources (immutable for app lifetime)
    vertices: Vec<Vertex>,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,

    // App state
    pub resized: bool,
    start: Instant,
    config: Config,
}

impl App {
    /// Creates our Vulkan app
    pub unsafe fn create(window: &Window) -> Result<Self> {
        let config = Config::load()?;

        // Create core Vulkan context
        let context = VulkanContext::create(window, &config)?;

        // Create vertex and index buffers
        let (vertex_buffer, vertex_buffer_memory, staging_buffer, staging_buffer_memory) =
            create_vertex_buffers(
                &context.instance,
                &context.device,
                context.physical_device,
                context.graphics_queue,
                context.command_pool,
                config.vulkan.max_vertices,
                config.vulkan.staging_buffer_vertex_count,
            )?;

        // Create renderer
        let renderer = Renderer::create(window, &context, &config, vertex_buffer)?;

        //     let vertices = Vec::new();
        let vertices = vec![
            Vertex {
                pos: Vec2::new(0.0, 0.5),
            },
            Vertex {
                pos: Vec2::new(-0.5, -0.5),
            },
            Vertex {
                pos: Vec2::new(0.5, -0.5),
            },
        ];

        // Copy vertices to staging buffer
        let size = (std::mem::size_of::<Vertex>() * vertices.len()) as u64;
        let memory = context.device.map_memory(
            staging_buffer_memory,
            0,
            size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(vertices.as_ptr(), memory.cast(), vertices.len());
        context.device.unmap_memory(staging_buffer_memory);

        // Copy from staging to device-local buffer
        crate::vulkan::buffer::copy_buffer(
            &context.device,
            context.graphics_queue,
            context.command_pool,
            staging_buffer,
            vertex_buffer,
            size,
        )?;

        // Clean up staging buffer (we don't need it anymore for now)
        context.device.destroy_buffer(staging_buffer, None);
        context.device.free_memory(staging_buffer_memory, None);

        Ok(Self {
            context,
            renderer,
            vertices,
            vertex_buffer,
            vertex_buffer_memory,
            resized: false,
            start: Instant::now(),
            config,
        })
    }

    /// Renders a frame for our Vulkan app
    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        let needs_recreate = self.renderer.render(
            window,
            &self.context,
            &self.config,
            self.vertex_buffer,
            self.start,
        )?;

        if self.resized {
            self.resized = false;
            self.renderer
                .recreate_swapchain(window, &self.context, &self.config)?;
        }

        Ok(())
    }

    /// Destroys our Vulkan app
    pub unsafe fn destroy(&mut self) {
        self.context.device.device_wait_idle().unwrap();

        self.renderer.destroy(&self.context.device);

        self.context
            .device
            .free_memory(self.vertex_buffer_memory, None);
        self.context.device.destroy_buffer(self.vertex_buffer, None);

        self.context.destroy();
    }
}
