use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::device::QueueFamilyIndices;

//================================================
// Command Pools
//================================================

pub unsafe fn create_command_pool(
    instance: &Instance,
    device: &Device,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
) -> Result<vk::CommandPool> {
    let indices = QueueFamilyIndices::get(instance, surface, physical_device)?;

    let info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(indices.graphics);

    Ok(device.create_command_pool(&info, None)?)
}

pub unsafe fn create_command_pools(
    instance: &Instance,
    device: &Device,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    swapchain_image_count: usize,
) -> Result<Vec<vk::CommandPool>> {
    // Per-framebuffer command pools
    let mut command_pools = Vec::new();
    for _ in 0..swapchain_image_count {
        let pool = create_command_pool(instance, device, surface, physical_device)?;
        command_pools.push(pool);
    }

    Ok(command_pools)
}

//================================================
// Command Buffers
//================================================

pub unsafe fn create_command_buffers(
    device: &Device,
    command_pools: &[vk::CommandPool],
) -> Result<Vec<vk::CommandBuffer>> {
    let mut command_buffers = Vec::new();

    for &command_pool in command_pools {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = device.allocate_command_buffers(&allocate_info)?[0];
        command_buffers.push(command_buffer);
    }

    Ok(command_buffers)
}
