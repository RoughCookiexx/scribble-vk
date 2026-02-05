use anyhow::Result;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::KhrSwapchainExtensionDeviceCommands;
use winit::window::Window;

use super::command::{create_command_buffers, create_command_pools};
use super::context::VulkanContext;
use super::pipeline::{create_framebuffers, create_pipeline, create_render_pass};
use super::swapchain::{create_swapchain, create_swapchain_image_views};
use crate::{config::Config, types::RECT};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

/// Manages swapchain-dependent rendering resources
pub struct Renderer {
    // Swapchain
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,

    // Pipeline
    pub render_pass: vk::RenderPass,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,

    // Framebuffers
    pub framebuffers: Vec<vk::Framebuffer>,

    // Command buffers
    pub command_pools: Vec<vk::CommandPool>,
    pub command_buffers: Vec<vk::CommandBuffer>,

    // Sync objects
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,

    pub frame: usize,
}

impl Renderer {
    /// Creates a new renderer with all swapchain-dependent resources
    pub unsafe fn create(
        window: &Window,
        context: &VulkanContext,
        config: &Config,
    ) -> Result<Self> {
        // Create swapchain
        let (swapchain, swapchain_images, swapchain_format, swapchain_extent) = create_swapchain(
            window,
            &context.instance,
            &context.device,
            context.surface,
            context.physical_device,
        )?;

        let swapchain_image_views =
            create_swapchain_image_views(&context.device, &swapchain_images, swapchain_format)?;

        // Create render pass and pipeline
        let render_pass = create_render_pass(&context.device, swapchain_format)?;

        let (pipeline, pipeline_layout) = create_pipeline(
            &context.device,
            swapchain_extent,
            render_pass,
            &config.shaders,
        )?;

        // Create framebuffers
        let framebuffers = create_framebuffers(
            &context.device,
            &swapchain_image_views,
            swapchain_extent,
            render_pass,
        )?;

        // Create command pools and buffers
        let command_pools = create_command_pools(
            &context.instance,
            &context.device,
            context.surface,
            context.physical_device,
            swapchain_images.len(),
        )?;

        let command_buffers = create_command_buffers(&context.device, &command_pools)?;

        // Create sync objects
        let (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
        ) = super::sync::create_sync_objects(
            &context.device,
            MAX_FRAMES_IN_FLIGHT,
            swapchain_images.len(),
        )?;

        Ok(Self {
            swapchain,
            swapchain_images,
            swapchain_image_views,
            swapchain_format,
            swapchain_extent,
            render_pass,
            pipeline_layout,
            pipeline,
            framebuffers,
            command_pools,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            frame: 0,
        })
    }

    /// Renders a frame
    pub unsafe fn render(
        &mut self,
        window: &Window,
        context: &VulkanContext,
        config: &Config,
        rect_buffer: vk::Buffer,
        line_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        start_time: std::time::Instant,
        line_count: u32,
    ) -> Result<bool> {
        let in_flight_fence = self.in_flight_fences[self.frame];

        context
            .device
            .wait_for_fences(&[in_flight_fence], true, u64::MAX)?;

        let result = context.device.acquire_next_image_khr(
            self.swapchain,
            u64::MAX,
            self.image_available_semaphores[self.frame],
            vk::Fence::null(),
        );

        let image_index = match result {
            Ok((image_index, _)) => image_index as usize,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(window, context, config)?;
                return Ok(false);
            }
            Err(e) => return Err(anyhow::anyhow!(e)),
        };

        let image_in_flight = self.images_in_flight[image_index];
        if !image_in_flight.is_null() {
            context
                .device
                .wait_for_fences(&[image_in_flight], true, u64::MAX)?;
        }

        self.images_in_flight[image_index] = in_flight_fence;

        self.update_command_buffer(
            context,
            image_index,
            rect_buffer,
            line_buffer,
            index_buffer,
            start_time,
            line_count,
        )?;

        let wait_semaphores = &[self.image_available_semaphores[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.command_buffers[image_index]];
        let signal_semaphores = &[self.render_finished_semaphores[self.frame]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        context.device.reset_fences(&[in_flight_fence])?;

        context
            .device
            .queue_submit(context.graphics_queue, &[submit_info], in_flight_fence)?;

        let swapchains = &[self.swapchain];
        let image_indices = &[image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        let result = context
            .device
            .queue_present_khr(context.present_queue, &present_info);
        let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

        let needs_recreate = if changed {
            self.recreate_swapchain(window, context, config)?;
            true
        } else if let Err(e) = result {
            return Err(anyhow::anyhow!(e));
        } else {
            false
        };

        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(needs_recreate)
    }

    /// Updates a command buffer
    unsafe fn update_command_buffer(
        &mut self,
        context: &VulkanContext,
        image_index: usize,
        rect_buffer: vk::Buffer,
        line_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        start_time: std::time::Instant,
        line_count: u32,
    ) -> Result<()> {
        let command_pool = self.command_pools[image_index];
        context
            .device
            .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())?;

        let command_buffer = self.command_buffers[image_index];

        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        context.device.begin_command_buffer(command_buffer, &info)?;

        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(self.swapchain_extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let clear_values = &[color_clear_value];
        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[image_index])
            .render_area(render_area)
            .clear_values(clear_values);

        context
            .device
            .cmd_begin_render_pass(command_buffer, &info, vk::SubpassContents::INLINE);

        // Bind pipeline
        context.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );

        context.device.cmd_bind_index_buffer(
            command_buffer,
            index_buffer,
            0,
            vk::IndexType::UINT16,
        );

        context
            .device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[rect_buffer], &[0]);
        context
            .device
            .cmd_bind_vertex_buffers(command_buffer, 1, &[line_buffer], &[0]);

        context
            .device
            .cmd_draw_indexed(command_buffer, RECT.len() as u32, line_count, 0, 0, 0);

        context.device.cmd_end_render_pass(command_buffer);
        context.device.end_command_buffer(command_buffer)?;

        Ok(())
    }

    /// Recreates the swapchain and dependent resources
    pub unsafe fn recreate_swapchain(
        &mut self,
        window: &Window,
        context: &VulkanContext,
        config: &Config,
    ) -> Result<()> {
        context.device.device_wait_idle()?;
        self.destroy_swapchain(&context.device);

        let (swapchain, swapchain_images, swapchain_format, swapchain_extent) = create_swapchain(
            window,
            &context.instance,
            &context.device,
            context.surface,
            context.physical_device,
        )?;
        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.swapchain_format = swapchain_format;
        self.swapchain_extent = swapchain_extent;

        self.swapchain_image_views = create_swapchain_image_views(
            &context.device,
            &self.swapchain_images,
            self.swapchain_format,
        )?;

        self.render_pass = create_render_pass(&context.device, self.swapchain_format)?;

        let (pipeline, pipeline_layout) = create_pipeline(
            &context.device,
            self.swapchain_extent,
            self.render_pass,
            &config.shaders,
        )?;

        self.pipeline = pipeline;
        self.pipeline_layout = pipeline_layout;

        self.framebuffers = create_framebuffers(
            &context.device,
            &self.swapchain_image_views,
            self.swapchain_extent,
            self.render_pass,
        )?;

        let command_buffers = create_command_buffers(&context.device, &self.command_pools)?;
        self.command_buffers = command_buffers;

        self.images_in_flight
            .resize(self.swapchain_images.len(), vk::Fence::null());

        Ok(())
    }

    /// Destroys swapchain-dependent resources
    unsafe fn destroy_swapchain(&self, device: &Device) {
        self.framebuffers
            .iter()
            .for_each(|f| device.destroy_framebuffer(*f, None));
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_render_pass(self.render_pass, None);
        self.swapchain_image_views
            .iter()
            .for_each(|v| device.destroy_image_view(*v, None));
        device.destroy_swapchain_khr(self.swapchain, None);
    }

    /// Destroys all renderer resources
    pub unsafe fn destroy(&self, device: &Device) {
        self.destroy_swapchain(device);

        self.in_flight_fences
            .iter()
            .for_each(|f| device.destroy_fence(*f, None));
        self.render_finished_semaphores
            .iter()
            .for_each(|s| device.destroy_semaphore(*s, None));
        self.image_available_semaphores
            .iter()
            .for_each(|s| device.destroy_semaphore(*s, None));
        self.command_pools
            .iter()
            .for_each(|p| device.destroy_command_pool(*p, None));
    }
}
