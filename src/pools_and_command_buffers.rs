use crate::instance_device_queues::QueueFamilies;
use ash::{version::DeviceV1_0, vk};
pub struct Pools {
    command_pool_graphics: vk::CommandPool,
    command_pool_transfer: vk::CommandPool,
}

impl Pools {
    pub fn init(
        logical_device: &ash::Device,
        queue_families: &QueueFamilies,
    ) -> Result<Pools, vk::Result> {
        let graphics_command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.graphics_q_index.unwrap())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool_graphics = unsafe {
            logical_device.create_command_pool(&graphics_command_pool_create_info, None)
        }?;
        let transfer_command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.transfer_q_index.unwrap())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool_transfer = unsafe {
            logical_device.create_command_pool(&transfer_command_pool_create_info, None)
        }?;

        Ok(Pools {
            command_pool_graphics,
            command_pool_transfer,
        })
    }

    pub fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_command_pool(self.command_pool_graphics, None);
            logical_device.destroy_command_pool(self.command_pool_transfer, None);
        }
    }
}

pub fn create_command_buffers(
    logical_device: &ash::Device,
    pools: &Pools,
    amount: u32,
) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(pools.command_pool_graphics)
        .command_buffer_count(amount as u32);
    unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info) }
}
