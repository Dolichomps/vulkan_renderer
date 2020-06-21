use ash::vk;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
    size_in_bytes: u64,
    buffer_usage: vk::BufferUsageFlags,
    memory_usage: vk_mem::MemoryUsage,
}
impl Buffer {
    pub fn new(
        allocator: &vk_mem::Allocator,
        size_in_bytes: u64,
        buffer_usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Result<Buffer, vk_mem::error::Error> {
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            ..Default::default()
        };

        // create buffer
        let (buffer, allocation, _allocation_info) = allocator.create_buffer(
            &ash::vk::BufferCreateInfo::builder()
                .size(size_in_bytes)
                .usage(buffer_usage)
                .build(),
            &allocation_create_info,
        )?;
        Ok(Buffer {
            buffer,
            allocation,
            size_in_bytes,
            buffer_usage,
            memory_usage,
        })
    }

    pub fn fill<T: Sized>(
        &mut self,
        allocator: &vk_mem::Allocator,
        data: &[T],
    ) -> Result<(), vk_mem::error::Error> {
        let bytes_to_write = (data.len() * std::mem::size_of::<T>()) as u64;
        if bytes_to_write > self.size_in_bytes {
            allocator.destroy_buffer(self.buffer, &self.allocation)?;
            let new_buffer = Buffer::new(
                allocator,
                bytes_to_write,
                self.buffer_usage,
                self.memory_usage,
            )?;
            *self = new_buffer;
        }
        let data_ptr = allocator.map_memory(&self.allocation)? as *mut T;
        unsafe { data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len()) };
        allocator.unmap_memory(&self.allocation)?;
        Ok(())
    }
}
