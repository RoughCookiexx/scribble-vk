use anyhow::Result;
use cgmath::AbsDiffEq;
use std::time::Instant;
use vulkanalia::prelude::v1_0::*;
use winit::window::Window;

use crate::config::Config;
use crate::types::{Line, Vec2, Vec3};
use crate::vulkan::buffer::{copy_buffer, create_buffers};
use crate::vulkan::context::VulkanContext;
use crate::vulkan::renderer::Renderer;

/// The main Vulkan application
pub struct App {
    context: VulkanContext,
    renderer: Renderer,

    // Scene resources (immutable for app lifetime)
    line_start: Option<Vec2>,
    lines: Vec<Vec<Line>>,
    new_lines: Vec<Line>,

    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    staging_buffer: vk::Buffer,
    staging_buffer_memory: vk::DeviceMemory,
    geometry_buffer: vk::Buffer,
    geometry_buffer_memory: vk::DeviceMemory,
    geometry_index_buffer: vk::Buffer,
    geometry_index_buffer_memory: vk::DeviceMemory,

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
        let (
            vertex_buffer,
            vertex_buffer_memory,
            staging_buffer,
            staging_buffer_memory,
            geometry_buffer,
            geometry_buffer_memory,
            geometry_index_buffer,
            geometry_index_buffer_memory,
        ) = create_buffers(
            &context.instance,
            &context.device,
            context.physical_device,
            context.graphics_queue,
            context.command_pool,
            config.vulkan.max_vertices,
            config.vulkan.staging_buffer_vertex_count,
        )?;

        // Create renderer
        let renderer = Renderer::create(window, &context, &config)?;

        let lines = vec![vec![]];
        let new_lines = vec![];

        // Copy lines to staging buffer
        Ok(Self {
            context,
            renderer,
            line_start: None,
            lines,
            new_lines,
            vertex_buffer,
            vertex_buffer_memory,
            staging_buffer,
            staging_buffer_memory,
            geometry_buffer,
            geometry_buffer_memory,
            geometry_index_buffer,
            geometry_index_buffer_memory,
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
            self.geometry_buffer,
            self.vertex_buffer,
            self.geometry_index_buffer,
            self.start,
            self.lines.len() as u32,
        )?;

        if self.resized {
            self.resized = false;
            self.renderer
                .recreate_swapchain(window, &self.context, &self.config)?;
        }

        Ok(())
    }

    pub unsafe fn append_vertex(&mut self, new_vertex: Vec2) -> Result<()> {
        match self.new_lines.last() {
            Some(last_element) => {
                // If the points are far enough apart, add a new line
                if !last_element.position.abs_diff_eq(&new_vertex, 1e-3) {
                    self.new_lines
                        .push(Line::new(last_element.position, new_vertex));
                }
            }
            None => match self.line_start {
                Some(line_start) => {
                    if !line_start.abs_diff_eq(&new_vertex, 1e-3) {
                        self.new_lines.push(Line::new(line_start, new_vertex));
                    }
                }
                None => {
                    self.line_start = Some(new_vertex);
                }
            },
        };

        Ok(())
    }

    pub unsafe fn commit_new_line(&mut self) -> Result<()> {
        /* let size = (std::mem::size_of::<Vertex>() * new_lines.len()) as u64;
        let dst_offset = (std::mem::size_of::<Vertex>() * self.lines.len()) as u64;
        std::ptr::copy_nonoverlapping(
            new_lines.as_ptr(),
            self.staging_buffer_memory_ptr,
            new_lines.len(),
        );
        crate::vulkan::buffer::copy_buffer(
            &self.context.device,
            self.context.graphics_queue,
            self.context.command_pool,
            self.staging_buffer,
            self.vertex_buffer,
            dst_offset,
            size,
        )?;
        */

        self.lines.push(self.new_lines.clone());
        self.new_lines = vec![];
        Ok(())
    }

    /// Destroys our Vulkan app
    pub unsafe fn destroy(&mut self) {
        self.context.device.device_wait_idle().unwrap();

        self.renderer.destroy(&self.context.device);

        self.context
            .device
            .free_memory(self.staging_buffer_memory, None);
        self.context
            .device
            .destroy_buffer(self.staging_buffer, None);

        self.context
            .device
            .free_memory(self.vertex_buffer_memory, None);
        self.context.device.destroy_buffer(self.vertex_buffer, None);

        self.context
            .device
            .free_memory(self.geometry_buffer_memory, None);
        self.context
            .device
            .destroy_buffer(self.geometry_buffer, None);

        self.context
            .device
            .free_memory(self.geometry_index_buffer_memory, None);
        self.context
            .device
            .destroy_buffer(self.geometry_index_buffer, None);

        self.context.destroy();
    }
}
