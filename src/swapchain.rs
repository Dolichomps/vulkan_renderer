use crate::instance_device_queues::QueueFamilies;
use crate::surface::FaeSurface;
use ash::{version::DeviceV1_0, vk};

pub struct FaeSwapchain {
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    _images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    depth_image: vk::Image,
    _depth_image_allocation: vk_mem::Allocation,
    _depth_image_allocation_info: vk_mem::AllocationInfo,
    depth_image_view: vk::ImageView,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub surface_format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
    pub image_available: Vec<vk::Semaphore>,
    pub rendering_finished: Vec<vk::Semaphore>,
    pub may_begin_drawing: Vec<vk::Fence>,
    pub amount_of_images: u32,
    pub current_image: usize,
}

impl FaeSwapchain {
    pub fn init(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        logical_device: &ash::Device,
        surfaces: &FaeSurface,
        q_families: &QueueFamilies,
        allocator: &vk_mem::Allocator,
    ) -> Result<FaeSwapchain, vk::Result> {
        // query surface information
        let surface_capabilites = surfaces.get_capabilities(physical_device)?;
        let extent = surface_capabilites.current_extent;
        let _surface_present_modes = surfaces.get_present_modes(physical_device)?;
        let surface_format = *surfaces.get_formats(physical_device)?.first().unwrap();
        let queue_families = [q_families.graphics_q_index.unwrap()];
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surfaces.surface)
            .min_image_count(
                3.max(surface_capabilites.min_image_count)
                    .min(surface_capabilites.max_image_count),
            )
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_families)
            .pre_transform(surface_capabilites.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, logical_device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        // get Vec of vkImages
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let amount_of_images = swapchain_images.len() as u32;
        //create ImageViews
        let mut swapchain_image_views = Vec::with_capacity(swapchain_images.len());
        for image in &swapchain_images {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let image_view_create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .subresource_range(*subresource_range);
            let image_view =
                unsafe { logical_device.create_image_view(&image_view_create_info, None) }?;
            swapchain_image_views.push(image_view);
        }

        let extent3d = vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        };
        let depth_image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .extent(extent3d)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_families);
        let allocation_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::GpuOnly,
            ..Default::default()
        };
        let (depth_image, _depth_image_allocation, _depth_image_allocation_info) = allocator
            .create_image(&depth_image_info, &allocation_info)
            .unwrap();

        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::DEPTH)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(depth_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(*subresource_range);
        let depth_image_view =
            unsafe { logical_device.create_image_view(&image_view_create_info, None) }?;

        // use semaphores for syncing image views
        let mut image_available = vec![];
        let mut rendering_finished = vec![];
        let mut may_begin_drawing = vec![];
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        // use fence to sync cpu and gpu
        // set fence to signaled state
        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        // sync each image
        for _ in 0..amount_of_images {
            let semaphore_available =
                unsafe { logical_device.create_semaphore(&semaphore_create_info, None) }?;
            let semaphore_finished =
                unsafe { logical_device.create_semaphore(&semaphore_create_info, None) }?;
            image_available.push(semaphore_available);
            rendering_finished.push(semaphore_finished);
            let fence = unsafe { logical_device.create_fence(&fence_create_info, None) }?;
            may_begin_drawing.push(fence);
        }

        Ok(FaeSwapchain {
            swapchain_loader,
            swapchain,
            _images: swapchain_images,
            image_views: swapchain_image_views,
            depth_image,
            _depth_image_allocation,
            _depth_image_allocation_info,
            depth_image_view,
            framebuffers: vec![],
            surface_format,
            extent,
            amount_of_images,
            current_image: 0,
            image_available,
            rendering_finished,
            may_begin_drawing,
        })
    }

    pub fn create_framebuffers(
        &mut self,
        logical_device: &ash::Device,
        render_pass: vk::RenderPass,
    ) -> Result<(), vk::Result> {
        for iv in &self.image_views {
            let i_view = [*iv, self.depth_image_view];
            let framebuffer_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&i_view)
                .width(self.extent.width)
                .height(self.extent.height)
                .layers(1);
            let fb = unsafe { logical_device.create_framebuffer(&framebuffer_info, None) }?;
            self.framebuffers.push(fb);
        }
        Ok(())
    }

    pub unsafe fn cleanup(&mut self, logical_device: &ash::Device, allocator: &vk_mem::Allocator) {
        logical_device.destroy_image_view(self.depth_image_view, None);
        allocator
            .destroy_image(self.depth_image, &self._depth_image_allocation)
            .unwrap();
        for fence in &self.may_begin_drawing {
            logical_device.destroy_fence(*fence, None);
        }
        for semaphore in &self.image_available {
            logical_device.destroy_semaphore(*semaphore, None);
        }
        for semaphore in &self.rendering_finished {
            logical_device.destroy_semaphore(*semaphore, None);
        }
        for fb in &self.framebuffers {
            logical_device.destroy_framebuffer(*fb, None);
        }
        for iv in &self.image_views {
            logical_device.destroy_image_view(*iv, None);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None)
    }
}
