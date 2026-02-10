use std::{mem::size_of, ptr::copy_nonoverlapping as memcpy};

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::helpers::{begin_single_time_commands, end_single_time_commands, get_memory_type_index};
use crate::types::{Line, RECT, RECT_INDICES};

//================================================
// Generic Buffer Creation
//================================================

unsafe fn create_buffer(
    instance: &Instance,
    device: &Device,
    physical_device: vk::PhysicalDevice,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    // Buffer
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = device.create_buffer(&buffer_info, None)?;

    // Memory
    let requirements = device.get_buffer_memory_requirements(buffer);

    let memory_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(get_memory_type_index(
            instance,
            physical_device,
            properties,
            requirements,
        )?);

    let buffer_memory = device.allocate_memory(&memory_info, None)?;

    device.bind_buffer_memory(buffer, buffer_memory, 0)?;

    Ok((buffer, buffer_memory))
}

pub unsafe fn copy_buffer(
    device: &Device,
    graphics_queue: vk::Queue,
    command_pool: vk::CommandPool,
    source: vk::Buffer,
    destination: vk::Buffer,
    dst_offset: u64,
    size: vk::DeviceSize,
) -> Result<()> {
    let command_buffer = begin_single_time_commands(device, command_pool)?;

    let regions = vk::BufferCopy::builder().dst_offset(dst_offset).size(size);
    device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);

    end_single_time_commands(device, graphics_queue, command_pool, command_buffer)?;

    Ok(())
}

//================================================
// Create Scribble Buffers
//================================================

pub unsafe fn create_buffers(
    instance: &Instance,
    device: &Device,
    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    command_pool: vk::CommandPool,
    max_vertices: u32,
    staging_buffer_vertex_count: u32,
) -> Result<(
    vk::Buffer,
    vk::DeviceMemory,
    vk::Buffer,
    vk::DeviceMemory,
    vk::Buffer,
    vk::DeviceMemory,
    vk::Buffer,
    vk::DeviceMemory,
)> {
    // Create vertex buffers
    let (vertex_buffer, vertex_buffer_memory, staging_buffer, staging_buffer_memory) =
        create_vertex_buffers(
            instance,
            device,
            physical_device,
            max_vertices,
            staging_buffer_vertex_count,
        )?;

    // Create instance buffer
    let (instance_buffer, instance_buffer_memory) = create_instance_buffers(
        instance,
        device,
        physical_device,
        graphics_queue,
        command_pool,
    )?;

    // Create index buffer
    let (instance_index_buffer, instance_index_buffer_memory) = create_index_buffers(
        instance,
        device,
        physical_device,
        graphics_queue,
        command_pool,
    )?;

    Ok((
        vertex_buffer,
        vertex_buffer_memory,
        staging_buffer,
        staging_buffer_memory,
        instance_buffer,
        instance_buffer_memory,
        instance_index_buffer,
        instance_index_buffer_memory,
    ))
}

pub unsafe fn create_vertex_buffers(
    instance: &Instance,
    device: &Device,
    physical_device: vk::PhysicalDevice,
    max_vertices: u32,
    staging_buffer_vertex_count: u32,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::Buffer, vk::DeviceMemory)> {
    let vertex_buffer_size = (size_of::<Line>() * max_vertices as usize) as u64;
    let staging_buffer_size = (size_of::<Line>() * staging_buffer_vertex_count as usize) as u64;

    // Create staging buffer
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        staging_buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    // Create vertex buffer
    let (vertex_buffer, vertex_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        vertex_buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    Ok((
        vertex_buffer,
        vertex_buffer_memory,
        staging_buffer,
        staging_buffer_memory,
    ))
}

pub unsafe fn create_instance_buffers(
    instance: &Instance,
    device: &Device,
    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    command_pool: vk::CommandPool,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_size = (size_of::<f32>() * RECT.len()) as u64;

    // Create staging buffer
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    // Create vertex buffer
    let (vertex_buffer, vertex_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    copy_buffer(
        device,
        graphics_queue,
        command_pool,
        staging_buffer,
        vertex_buffer,
        0,
        buffer_size,
    )?;

    let memory = device.map_memory(
        staging_buffer_memory,
        0,
        buffer_size,
        vk::MemoryMapFlags::empty(),
    )?;
    memcpy(RECT.as_ptr(), memory.cast(), RECT.len());
    device.unmap_memory(staging_buffer_memory);

    copy_buffer(
        device,
        graphics_queue,
        command_pool,
        staging_buffer,
        vertex_buffer,
        0,
        buffer_size,
    )?;
    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);

    Ok((vertex_buffer, vertex_buffer_memory))
}

pub unsafe fn create_index_buffers(
    instance: &Instance,
    device: &Device,
    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    command_pool: vk::CommandPool,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_size = (size_of::<u16>() * RECT_INDICES.len()) as u64;

    // Create staging buffer
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    // Create index buffer
    let (index_buffer, index_buffer_memory) = create_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let memory = device.map_memory(
        staging_buffer_memory,
        0,
        buffer_size,
        vk::MemoryMapFlags::empty(),
    )?;

    memcpy(RECT_INDICES.as_ptr(), memory.cast(), RECT_INDICES.len());
    device.unmap_memory(staging_buffer_memory);

    copy_buffer(
        device,
        graphics_queue,
        command_pool,
        staging_buffer,
        index_buffer,
        0,
        buffer_size,
    )?;
    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);

    Ok((index_buffer, index_buffer_memory))
}
