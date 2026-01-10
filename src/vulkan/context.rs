use anyhow::Result;
use vulkanalia::loader::{LIBRARY, LibloadingLoader};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::ExtDebugUtilsExtensionInstanceCommands;
use vulkanalia::vk::KhrSurfaceExtensionInstanceCommands;
use winit::window::Window;

use super::instance::create_instance;
use super::logical_device::create_logical_device;
use super::physical_device::pick_physical_device;
use crate::config::Config;

/// Core Vulkan objects that live for the entire application lifetime
pub struct VulkanContext {
    pub entry: vulkanalia::Entry,
    pub instance: Instance,
    pub device: Device,
    pub physical_device: vk::PhysicalDevice,
    pub surface: vk::SurfaceKHR,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub command_pool: vk::CommandPool,
}

impl VulkanContext {
    /// Creates a new Vulkan context
    pub unsafe fn create(window: &Window, config: &Config) -> Result<Self> {
        let loader = LibloadingLoader::new(LIBRARY)?;
        let entry = vulkanalia::Entry::new(loader).map_err(|b| anyhow::anyhow!("{}", b))?;

        let (instance, messenger) = create_instance(window, &entry, &config.window)?;
        let surface = vulkanalia::window::create_surface(&instance, window, window)?;
        let physical_device = pick_physical_device(&instance, surface)?;
        let (device, graphics_queue, present_queue) =
            create_logical_device(&entry, &instance, surface, physical_device)?;

        let command_pool =
            super::command::create_command_pool(&instance, &device, surface, physical_device)?;

        Ok(Self {
            entry,
            instance,
            device,
            physical_device,
            surface,
            graphics_queue,
            present_queue,
            messenger,
            command_pool,
        })
    }

    /// Destroys the Vulkan context
    pub unsafe fn destroy(&self) {
        self.device.destroy_command_pool(self.command_pool, None);
        self.device.destroy_device(None);
        self.instance.destroy_surface_khr(self.surface, None);

        if !self.messenger.is_null() {
            self.instance
                .destroy_debug_utils_messenger_ext(self.messenger, None);
        }

        self.instance.destroy_instance(None);
    }
}
