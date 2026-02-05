use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

//================================================
// Synchronization Objects
//================================================

pub unsafe fn create_sync_objects(
    device: &Device,
    max_frames_in_flight: usize,
    swapchain_image_count: usize,
) -> Result<(
    Vec<vk::Semaphore>,
    Vec<vk::Semaphore>,
    Vec<vk::Fence>,
    Vec<vk::Fence>,
)> {
    let semaphore_info = vk::SemaphoreCreateInfo::builder();
    let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

    let mut image_available_semaphores = Vec::new();
    let mut render_finished_semaphores = Vec::new();
    let mut in_flight_fences = Vec::new();

    for _ in 0..swapchain_image_count {
        image_available_semaphores.push(device.create_semaphore(&semaphore_info, None)?);
        render_finished_semaphores.push(device.create_semaphore(&semaphore_info, None)?);
    }

    for _ in 0..max_frames_in_flight {
        in_flight_fences.push(device.create_fence(&fence_info, None)?);
    }

    let images_in_flight = vec![vk::Fence::null(); swapchain_image_count];

    Ok((
        image_available_semaphores,
        render_finished_semaphores,
        in_flight_fences,
        images_in_flight,
    ))
}
