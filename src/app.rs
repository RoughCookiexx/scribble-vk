use anyhow::Result;
use cgmath::AbsDiffEq;
use std::time::Instant;
use vulkanalia::prelude::v1_0::*;
use winit::window::Window;

use crate::config::Config;
use crate::types::{Line, Vec2};
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
    staging_buffer_ptr: *mut Line,
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

        // Persistently map staging buffer for efficient updates
        let staging_buffer_ptr = context.device.map_memory(
            staging_buffer_memory,
            0,
            vk::WHOLE_SIZE,
            vk::MemoryMapFlags::empty(),
        )? as *mut Line;

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
            staging_buffer_ptr,
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
        let new_line_count = if !self.new_lines.is_empty() {
            let lines_to_copy = self
                .new_lines
                .len()
                .min(self.config.vulkan.staging_buffer_vertex_count as usize);
            std::ptr::copy_nonoverlapping(
                self.new_lines.as_ptr(),
                self.staging_buffer_ptr,
                lines_to_copy,
            );
            lines_to_copy as u32
        } else {
            0
        };

        let line_count = self.lines.iter().map(|v| v.len()).sum::<usize>() as u32;

        let needs_recreate = self.renderer.render(
            window,
            &self.context,
            &self.config,
            self.geometry_buffer,
            self.vertex_buffer,
            self.staging_buffer,
            self.geometry_index_buffer,
            self.start,
            line_count,
            new_line_count,
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
                // Calculate the endpoint of the last line (position + dir/2)
                let last_end_point = last_element.position + last_element.dir / 2.0;
                // If the points are far enough apart, add a new line
                if !last_end_point.abs_diff_eq(&new_vertex, 1e-3) {
                    self.new_lines.push(Line::new(last_end_point, new_vertex));
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

        if self.new_lines.len() >= self.config.vulkan.staging_buffer_vertex_count as usize {
            self.commit_new_line()?;
        }

        Ok(())
    }

    pub unsafe fn commit_new_line(&mut self) -> Result<()> {
        if self.new_lines.is_empty() {
            self.line_start = None;
            return Ok(());
        }

        let new_line_count = if !self.new_lines.is_empty() {
            let lines_to_copy = self
                .new_lines
                .len()
                .min(self.config.vulkan.staging_buffer_vertex_count as usize);
            std::ptr::copy_nonoverlapping(
                self.new_lines.as_ptr(),
                self.staging_buffer_ptr,
                lines_to_copy,
            );
            lines_to_copy as u32
        } else {
            0
        };

        // Safety check: ensure we don't exceed staging buffer capacity
        let lines_to_copy = self
            .new_lines
            .len()
            .min(self.config.vulkan.staging_buffer_vertex_count as usize);
        let size = (std::mem::size_of::<Line>() * lines_to_copy) as u64;
        let current_line_count = self.lines.iter().map(|v| v.len()).sum::<usize>();
        let dst_offset = (std::mem::size_of::<Line>() * current_line_count) as u64;

        // GPU copy from staging buffer to device-local buffer
        // (staging buffer already contains the data from render() updates)
        copy_buffer(
            &self.context.device,
            self.context.graphics_queue,
            self.context.command_pool,
            self.staging_buffer,
            self.vertex_buffer,
            dst_offset,
            size,
        )?;

        // Update CPU-side tracking (only add the lines we actually copied)
        if lines_to_copy < self.new_lines.len() {
            self.lines.push(self.new_lines[..lines_to_copy].to_vec());
            self.new_lines = self.new_lines[lines_to_copy..].to_vec();
        } else {
            self.lines.push(self.new_lines.clone());
            self.new_lines.clear();
            self.line_start = None;
        }

        Ok(())
    }

    pub fn undo(&mut self) {
        // Remove the last committed stroke if there is one
        if self.lines.len() > 1 {
            self.lines.pop();
        }
    }

    /// Destroys our Vulkan app
    pub unsafe fn destroy(&mut self) {
        self.context.device.device_wait_idle().unwrap();

        self.renderer.destroy(&self.context.device);

        // Unmap persistently mapped staging buffer
        self.context.device.unmap_memory(self.staging_buffer_memory);

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
