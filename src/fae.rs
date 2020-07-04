use crate::*;

pub struct Fae {
    pub window: winit::window::Window,
    _entry: ash::Entry,
    instance: ash::Instance,
    debug: std::mem::ManuallyDrop<FaeDebug>,
    surfaces: std::mem::ManuallyDrop<FaeSurface>,
    _physical_device: vk::PhysicalDevice,
    _physical_device_properties: vk::PhysicalDeviceProperties,
    _queue_families: QueueFamilies,
    pub queues: Queues,
    pub device: ash::Device,
    pub swapchain: FaeSwapchain,
    render_pass: vk::RenderPass,
    pipeline: Pipeline,
    pools: Pools,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub allocator: vk_mem::Allocator,
    pub models: Vec<Model<model::VertexData, InstanceData>>,
    pub uniform_buffer: Buffer,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl Fae {
    pub fn init(window: winit::window::Window) -> Result<Fae, Box<dyn std::error::Error>> {
        // create vulkan entry
        let entry = ash::Entry::new()?;
        // layer names to enable
        let layer_names = vec!["VK_LAYER_KHRONOS_validation"];
        // create vulkan instance
        let instance = init_instance(&entry, &layer_names)?;
        // create debug messenger instance
        let debug = FaeDebug::init(&entry, &instance)?;
        // create surface instance
        let surfaces = FaeSurface::init(&window, &entry, &instance)?;
        // init physical rendering device and properties
        let (physical_device, physical_device_properties, _physical_device_features) =
            init_physical_device_and_properties(&instance)?;
        // create queue family instance
        let queue_families = QueueFamilies::init(&instance, physical_device, &surfaces)?;
        // create logical device and device queues
        let (logical_device, queues) =
            init_device_and_queues(&instance, physical_device, &queue_families, &layer_names)?;
        // create allocator
        let allocator_create_info = vk_mem::AllocatorCreateInfo {
            physical_device,
            device: logical_device.clone(),
            instance: instance.clone(),
            ..Default::default()
        };
        let allocator = vk_mem::Allocator::new(&allocator_create_info)?;

        let _allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuToGpu,
            ..Default::default()
        };
        // create swapchain
        let mut swapchain = FaeSwapchain::init(
            &instance,
            physical_device,
            &logical_device,
            &surfaces,
            &queue_families,
            &allocator,
        )?;
        // create render pass
        let render_pass = init_render_pass(&logical_device, swapchain.surface_format.format)?;
        // create framebuffers
        swapchain.create_framebuffers(&logical_device, render_pass)?;
        // create pipeline
        let pipeline = Pipeline::init(&logical_device, &swapchain, &render_pass)?;
        // create command pools
        let pools = Pools::init(&logical_device, &queue_families)?;
        // create command buffers
        let command_buffers =
            create_command_buffers(&logical_device, &pools, swapchain.amount_of_images)?;

        // create uniform buffer
        let mut uniform_buffer = Buffer::new(
            &allocator,
            128,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk_mem::MemoryUsage::CpuToGpu,
        )?;
        let camera_transform: [[[f32; 4]; 4]; 2] = [
            nalgebra::Matrix4::identity().into(),
            nalgebra::Matrix4::identity().into(),
        ];
        uniform_buffer.fill(&allocator, &camera_transform)?;

        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: swapchain.amount_of_images,
        }];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(swapchain.amount_of_images)
            .pool_sizes(&pool_sizes);
        let descriptor_pool =
            unsafe { logical_device.create_descriptor_pool(&descriptor_pool_info, None) }?;

        let desc_layouts =
            vec![pipeline.descriptor_set_layouts[0]; swapchain.amount_of_images as usize];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desc_layouts);
        let descriptor_sets =
            unsafe { logical_device.allocate_descriptor_sets(&descriptor_set_allocate_info) }?;

        for descset in descriptor_sets.iter() {
            let buffer_infos = [vk::DescriptorBufferInfo {
                buffer: uniform_buffer.buffer,
                offset: 0,
                range: 128,
            }];
            let desc_sets_write = [vk::WriteDescriptorSet::builder()
                .dst_set(*descset)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
                .build()];
            unsafe { logical_device.update_descriptor_sets(&desc_sets_write, &[]) };
        }

        Ok(Fae {
            window,
            _entry: entry,
            instance,
            debug: std::mem::ManuallyDrop::new(debug),
            surfaces: std::mem::ManuallyDrop::new(surfaces),
            _physical_device: physical_device,
            _physical_device_properties: physical_device_properties,
            _queue_families: queue_families,
            queues,
            device: logical_device,
            swapchain,
            render_pass,
            pipeline,
            pools,
            command_buffers,
            allocator,
            models: vec![],
            uniform_buffer,
            descriptor_pool,
            descriptor_sets,
        })
    }

    pub fn update_command_buffer(&mut self, index: usize) -> Result<(), vk::Result> {
        let command_buffer = self.command_buffers[index];
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();
        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)?;
        }
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.08, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.swapchain.framebuffers[index])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            })
            .clear_values(&clear_values);
        unsafe {
            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline,
            );
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &[self.descriptor_sets[index]],
                &[],
            );
            for m in &self.models {
                m.draw(&self.device, command_buffer);
            }
            self.device.cmd_end_render_pass(command_buffer);
            self.device.end_command_buffer(command_buffer)?;
        }
        Ok(())
    }
}

impl Drop for Fae {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("something wrong while waiting");
            self.allocator
                .destroy_buffer(self.uniform_buffer.buffer, &self.uniform_buffer.allocation)
                .unwrap();
            for m in &self.models {
                if let Some(vb) = &m.vertex_buffer {
                    self.allocator
                        .destroy_buffer(vb.buffer, &vb.allocation)
                        .expect("problem with buffer destruction")
                }
                if let Some(ib) = &m.instance_buffer {
                    self.allocator
                        .destroy_buffer(ib.buffer, &ib.allocation)
                        .expect("problem with buffer destruction");
                }
                if let Some(ib) = &m.index_buffer {
                    self.allocator
                        .destroy_buffer(ib.buffer, &ib.allocation)
                        .expect("problem with buffer destruction");
                }
            }
            self.pools.cleanup(&self.device);
            self.pipeline.cleanup(&self.device);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.swapchain.cleanup(&self.device, &self.allocator);
            self.allocator.destroy();
            self.device.destroy_device(None);
            std::mem::ManuallyDrop::drop(&mut self.surfaces);
            std::mem::ManuallyDrop::drop(&mut self.debug);
            self.instance.destroy_instance(None);
        };
    }
}
