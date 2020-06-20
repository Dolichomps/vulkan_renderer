use ash::{
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk,
};
use winapi::{shared::windef::HWND, um::libloaderapi::GetModuleHandleW};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop)?;
    let mut fae = Fae::init(window)?;
    let mut cube = Model::cube();
    let mut angle = 0.2;
    let my_special_cube = cube.insert_visibly(InstanceData {
        model_matrix: (nalgebra::Matrix4::from_scaled_axis(nalgebra::Vector3::new(0.0, 0.0, angle))
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.0, 0.5, 0.0))
            * nalgebra::Matrix4::new_scaling(0.1))
        .into(),
        color: [0.0, 0.5, 0.0],
    });
    let my_special_cube_too = cube.insert_visibly(InstanceData {
        model_matrix: (nalgebra::Matrix4::from_scaled_axis(nalgebra::Vector3::new(0.0, 0.0, angle))
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.0, 0.5, 0.0))
            * nalgebra::Matrix4::new_scaling(0.1))
        .into(),
        color: [1.0, 0.0, 0.5],
    });
    cube.update_vertex_buffer(&fae.allocator)?;
    cube.update_instance_buffer(&fae.allocator)?;
    fae.models = vec![cube];
    use winit::event::{Event, WindowEvent};
    event_loop.run(move |event, _, controlflow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *controlflow = winit::event_loop::ControlFlow::Exit;
        }
        Event::MainEventsCleared => {
            // do some rotation
            angle += 0.01;
            fae.models[0]
                .get_mut(my_special_cube)
                .unwrap()
                .model_matrix = (nalgebra::Matrix4::from_scaled_axis(nalgebra::Vector3::new(0.0, 0.0, angle))
                * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.0, 0.5, 0.0))
                * nalgebra::Matrix4::new_scaling(0.1))
            .into();
            fae.models[0]
                .get_mut(my_special_cube_too)
                .unwrap()
                .model_matrix = (nalgebra::Matrix4::from_scaled_axis(nalgebra::Vector3::new(0.0, 0.0, angle * 2.0))
                * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.0, 0.5, 0.1))
                * nalgebra::Matrix4::new_scaling(0.1))
            .into();
            fae.window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            // aquire the next image
            let (image_index, _) = unsafe {
                fae.swapchain
                    .swapchain_loader
                    .acquire_next_image(
                        fae.swapchain.swapchain,
                        std::u64::MAX,
                        fae.swapchain.image_available[fae.swapchain.current_image],
                        vk::Fence::null(),
                    )
                    .expect("image aquisition trouble")
            };
            unsafe {
                // wait for fence
                fae.device
                    .wait_for_fences(
                        &[fae.swapchain.may_begin_drawing[fae.swapchain.current_image]],
                        true,
                        std::u64::MAX,
                    )
                    .expect("fence-waiting");
                // reset fence
                fae.device
                    .reset_fences(&[fae.swapchain.may_begin_drawing[fae.swapchain.current_image]])
                    .expect("resetting fences");
            };
            for m in &mut fae.models {
                m.update_instance_buffer(&fae.allocator);
            }
            //update command buffer
            fae
            .update_command_buffer(image_index as usize)
            .expect("updateing the command buffer");
            // command buffer setup info
            let semaphores_available = [fae.swapchain.image_available[fae.swapchain.current_image]];
            let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let semaphores_finished =
                [fae.swapchain.rendering_finished[fae.swapchain.current_image]];
            let command_buffers = [fae.command_buffers[image_index as usize]];
            let submit_info = [vk::SubmitInfo::builder()
                .wait_semaphores(&semaphores_available)
                .wait_dst_stage_mask(&waiting_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&semaphores_finished)
                .build()];
            // submit command buffer
            unsafe {
                fae.device
                    .queue_submit(
                        fae.queues.graphics_queue,
                        &submit_info,
                        fae.swapchain.may_begin_drawing[fae.swapchain.current_image],
                    )
                    .expect("queue submission");
            };
            // present image to screen
            let swapchains = [fae.swapchain.swapchain];
            let indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&semaphores_finished)
                .swapchains(&swapchains)
                .image_indices(&indices);
            unsafe {
                fae.swapchain
                    .swapchain_loader
                    .queue_present(fae.queues.graphics_queue, &present_info)
                    .expect("queue presentation");
            };
        }
        _ => {}
    });
}

// external function call to setup validation layer callbacks
unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);
    vk::FALSE
}

fn init_instance(
    entry: &ash::Entry,
    layer_names: &[&str],
) -> Result<ash::Instance, ash::InstanceError> {
    // setup varaibles for ApplicationInfo
    let engine_name = std::ffi::CString::new("GameEngine").unwrap();
    let app_name = std::ffi::CString::new("Rusty VK").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(vk::make_version(0, 0, 1))
        .engine_name(&engine_name)
        .engine_version(vk::make_version(0, 1, 0))
        .api_version(vk::make_version(1, 0, 106));

    // load validation layers and enable DebugUtils extension
    let layer_names_c: Vec<std::ffi::CString> = layer_names
        .iter()
        .map(|&ln| std::ffi::CString::new(ln).unwrap())
        .collect();
    let layer_name_pointers: Vec<*const i8> = layer_names_c
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();
    let extension_name_pointers: Vec<*const i8> = vec![
        ash::extensions::ext::DebugUtils::name().as_ptr(),
        ash::extensions::khr::Surface::name().as_ptr(),
        ash::extensions::khr::Win32Surface::name().as_ptr(),
    ];

    // setup debug create info
    let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback));

    // setup instance creation info
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_create_info)
        .application_info(&app_info)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_extension_names(&extension_name_pointers);

    unsafe { entry.create_instance(&instance_create_info, None) }
}

struct FaeDebug {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl FaeDebug {
    fn init(entry: &ash::Entry, instance: &ash::Instance) -> Result<FaeDebug, vk::Result> {
        // setup debug messeges
        let debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(vulkan_debug_utils_callback));

        let loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let messenger = unsafe { loader.create_debug_utils_messenger(&debug_create_info, None)? };

        Ok(FaeDebug { loader, messenger })
    }
}

impl Drop for FaeDebug {
    fn drop(&mut self) {
        unsafe {
            self.loader
                .destroy_debug_utils_messenger(self.messenger, None)
        };
    }
}

struct FaeSurface {
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
}

impl FaeSurface {
    fn init(
        window: &winit::window::Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<FaeSurface, vk::Result> {
        // TODO: make cross platform is currently windows only
        // TODO: use conditional compilation
        // create window surface
        use winit::platform::windows::WindowExtWindows;
        let hwnd = window.hwnd() as HWND;
        let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) as *const std::ffi::c_void };
        let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
            s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: Default::default(),
            hinstance: h_instance,
            hwnd: hwnd as *const std::ffi::c_void,
        };
        let win32_surface_loader = ash::extensions::khr::Win32Surface::new(entry, instance);
        let surface =
            unsafe { win32_surface_loader.create_win32_surface(&win32_create_info, None) }?;
        let surface_loader = ash::extensions::khr::Surface::new(entry, instance);
        Ok(FaeSurface {
            surface,
            surface_loader,
        })
    }
    fn get_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
        }
    }
    fn get_present_modes(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)
        }
    }
    fn get_formats(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device, self.surface)
        }
    }
    fn get_physical_device_surface_support(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: usize,
    ) -> Result<bool, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_support(
                physical_device,
                queue_family_index as u32,
                self.surface,
            )
        }
    }
}

impl Drop for FaeSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

fn init_physical_device_and_properties(
    instance: &ash::Instance,
) -> Result<
    (
        vk::PhysicalDevice,
        vk::PhysicalDeviceProperties,
        vk::PhysicalDeviceFeatures,
    ),
    vk::Result,
> {
    // pick gpu to use (in this case it the discrete gpu)
    let phys_devs = unsafe { instance.enumerate_physical_devices()? };
    let mut chosen = None;
    for p in phys_devs {
        let properties = unsafe { instance.get_physical_device_properties(p) };
        let features = unsafe { instance.get_physical_device_features(p) };
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            chosen = Some((p, properties, features));
        }
    }
    Ok(chosen.unwrap())
}

struct QueueFamilies {
    graphics_q_index: Option<u32>,
    transfer_q_index: Option<u32>,
}
impl QueueFamilies {
    fn init(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surfaces: &FaeSurface,
    ) -> Result<QueueFamilies, vk::Result> {
        // TODO: this could be done like in the ash examples to make sure
        // physical device is surface and graphics capable
        // find queue family indices
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let mut found_graphics_q_index = None;
        let mut found_transfer_q_index = None;
        for (index, q_fam) in queue_family_properties.iter().enumerate() {
            if q_fam.queue_count > 0
                && q_fam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && surfaces.get_physical_device_surface_support(physical_device, index)?
            {
                found_graphics_q_index = Some(index as u32);
            }
            if q_fam.queue_count > 0 && q_fam.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                if found_transfer_q_index.is_none()
                    || !q_fam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                {
                    found_transfer_q_index = Some(index as u32);
                }
            }
        }
        Ok(QueueFamilies {
            graphics_q_index: found_graphics_q_index,
            transfer_q_index: found_transfer_q_index,
        })
    }
}

struct Queues {
    graphics_queue: vk::Queue,
    transfer_queue: vk::Queue,
}

fn init_device_and_queues(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_families: &QueueFamilies,
    layer_names: &[&str],
) -> Result<(ash::Device, Queues), vk::Result> {
    // setup validation layers again it's a compromise
    // TODO: try to think of a better way to do this
    let layer_names_c: Vec<std::ffi::CString> = layer_names
        .iter()
        .map(|&ln| std::ffi::CString::new(ln).unwrap())
        .collect();
    let layer_name_pointers: Vec<*const i8> = layer_names_c
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();

    // create a logical device as primary interface to gpu
    let priorities = [1.0f32];
    let queue_infos = [
        // GRAPHICS QUEUE
        // TODO: use only one DeviceQueueCreateInfo
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.graphics_q_index.unwrap())
            .queue_priorities(&priorities)
            .build(),
        // TRANSFER QUEUE
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.transfer_q_index.unwrap())
            .queue_priorities(&priorities)
            .build(),
    ];

    let features = vk::PhysicalDeviceFeatures::builder().fill_mode_non_solid(true);
    // device creation
    let device_extension_name_pointers: Vec<*const i8> =
        vec![ash::extensions::khr::Swapchain::name().as_ptr()];
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extension_name_pointers)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_features(&features);
    let logical_device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };
    let graphics_queue =
        unsafe { logical_device.get_device_queue(queue_families.graphics_q_index.unwrap(), 0) };
    let transfer_queue =
        unsafe { logical_device.get_device_queue(queue_families.transfer_q_index.unwrap(), 0) };
    Ok((
        logical_device,
        Queues {
            graphics_queue,
            transfer_queue,
        },
    ))
}

struct FaeSwapchain {
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    depth_image: vk::Image,
    depth_image_allocation: vk_mem::Allocation,
    depth_image_allocation_info: vk_mem::AllocationInfo,
    depth_image_view: vk::ImageView,
    framebuffers: Vec<vk::Framebuffer>,
    surface_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    image_available: Vec<vk::Semaphore>,
    rendering_finished: Vec<vk::Semaphore>,
    may_begin_drawing: Vec<vk::Fence>,
    amount_of_images: u32,
    current_image: usize,
}

impl FaeSwapchain {
    fn init(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        logical_device: &ash::Device,
        surfaces: &FaeSurface,
        q_families: &QueueFamilies,
        queues: &Queues,
        allocator: &vk_mem::Allocator,
    ) -> Result<FaeSwapchain, vk::Result> {
        // query surface information
        let surface_capabilites = surfaces.get_capabilities(physical_device)?;
        let extent = surface_capabilites.current_extent;
        let surface_present_modes = surfaces.get_present_modes(physical_device)?;
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
        let (depth_image, depth_image_allocation, depth_image_allocation_info) = allocator
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
            images: swapchain_images,
            image_views: swapchain_image_views,
            depth_image,
            depth_image_allocation,
            depth_image_allocation_info,
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

    fn create_framebuffers(
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

    unsafe fn cleanup(&mut self, logical_device: &ash::Device, allocator: &vk_mem::Allocator) {
        logical_device.destroy_image_view(self.depth_image_view, None);
        allocator.destroy_image(self.depth_image, &self.depth_image_allocation).unwrap();
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

fn init_render_pass(
    logical_device: &ash::Device,
    format: vk::Format,
) -> Result<vk::RenderPass, vk::Result> {
    // create attachments (essentially render-target)
    let attachments = [
        vk::AttachmentDescription::builder()
            // format must be same as swapchain
            .format(format)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .samples(vk::SampleCountFlags::TYPE_1)
            .build(),
        //attachment for depth
        vk::AttachmentDescription::builder()
            .format(vk::Format::D32_SFLOAT)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .samples(vk::SampleCountFlags::TYPE_1)
            .build(),
    ];

    let color_attachment_references = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];

    let depth_attachment_reference = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    // define subpasses
    let subpasses = [vk::SubpassDescription::builder()
        .color_attachments(&color_attachment_references)
        .depth_stencil_attachment(&depth_attachment_reference)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .build()];

    let subpass_dependencies = [vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_subpass(0)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        )
        .build()];

    let render_pass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&subpass_dependencies);
    let render_pass = unsafe { logical_device.create_render_pass(&render_pass_create_info, None)? };

    Ok(render_pass)
}

struct Pipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Pipeline {
    fn init(
        logical_device: &ash::Device,
        swapchain: &FaeSwapchain,
        render_pass: &vk::RenderPass,
    ) -> Result<Pipeline, vk::Result> {
        // create vertex shader module
        let vertex_shader_create_info = vk::ShaderModuleCreateInfo::builder()
            .code(vk_shader_macros::include_glsl!("./shaders/shader.vert"));
        let vertex_shader_module =
            unsafe { logical_device.create_shader_module(&vertex_shader_create_info, None)? };
        // create fragment shader module
        let fragment_shader_create_info = vk::ShaderModuleCreateInfo::builder()
            .code(vk_shader_macros::include_glsl!("./shaders/shader.frag"));
        let fragment_shader_module =
            unsafe { logical_device.create_shader_module(&fragment_shader_create_info, None)? };
        // define what functiuon should be used as the entry point in the shader
        let main_function_name = std::ffi::CString::new("main").unwrap();
        // shader stage creation info
        let vertex_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&main_function_name);
        let fragment_shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(&main_function_name);
        // create shader stages
        let shader_stages = vec![
            vertex_shader_stage_create_info.build(),
            fragment_shader_stage_create_info.build(),
        ];

        //setup data to pass to vertex shader
        let vertex_attrib_descs = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                offset: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 1,
                offset: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 2,
                offset: 16,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 3,
                offset: 32,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 4,
                offset: 48,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 5,
                offset: 64,
                format: vk::Format::R32G32B32A32_SFLOAT,
            },
        ];

        let vertex_binding_descs = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: 12,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            vk::VertexInputBindingDescription {
                binding: 1,
                stride: 76,
                input_rate: vk::VertexInputRate::INSTANCE,
            },
        ];
        // shader input creation info
        let vertex_input_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_attrib_descs)
            .vertex_binding_descriptions(&vertex_binding_descs);
        // is topology points or triangles
        let input_assembly_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        // define what part of screen to correspond to internal coordinates
        let viewports = [vk::Viewport {
            x: 0.,
            y: 0.,
            width: swapchain.extent.width as f32,
            height: swapchain.extent.height as f32,
            min_depth: 0.,
            max_depth: 1.,
        }];
        // define area that we can draw in
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent,
        }];
        // viewport creation info
        let viewport_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        // rasterizer creation info
        let rasterizer_creation_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE)
            .polygon_mode(vk::PolygonMode::FILL);

        // multisampler creation info
        let multisampler_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // color blending and transparency
        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .build()];
        let color_blend_create_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        // data to pass to pipeline not attached to verticies
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder();
        let pipeline_layout =
            unsafe { logical_device.create_pipeline_layout(&pipeline_layout_info, None) }?;
        
        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
        // pipeline creation info
        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_create_info)
            .input_assembly_state(&input_assembly_create_info)
            .viewport_state(&viewport_create_info)
            .rasterization_state(&rasterizer_creation_info)
            .multisample_state(&multisampler_create_info)
            .depth_stencil_state(&depth_stencil_info)
            .color_blend_state(&color_blend_create_info)
            .layout(pipeline_layout)
            .render_pass(*render_pass)
            .subpass(0);
        let graphics_pipeline = unsafe {
            logical_device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[pipeline_create_info.build()],
                    None,
                )
                .expect("A problem occured with pipeline creation")
        }[0];

        // cleanup shader modules they are no loger needed after the pipeline creation
        unsafe {
            logical_device.destroy_shader_module(fragment_shader_module, None);
            logical_device.destroy_shader_module(vertex_shader_module, None);
        }

        Ok(Pipeline {
            pipeline: graphics_pipeline,
            layout: pipeline_layout,
        })
    }

    fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_pipeline(self.pipeline, None);
            logical_device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

struct Pools {
    command_pool_graphics: vk::CommandPool,
    command_pool_transfer: vk::CommandPool,
}

impl Pools {
    fn init(
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

    fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_command_pool(self.command_pool_graphics, None);
            logical_device.destroy_command_pool(self.command_pool_transfer, None);
        }
    }
}

fn create_command_buffers(
    logical_device: &ash::Device,
    pools: &Pools,
    amount: u32,
) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(pools.command_pool_graphics)
        .command_buffer_count(amount as u32);
    unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info) }
}

struct Buffer {
    buffer: vk::Buffer,
    allocation: vk_mem::Allocation,
    size_in_bytes: u64,
    buffer_usage: vk::BufferUsageFlags,
    memory_usage: vk_mem::MemoryUsage,
}
impl Buffer {
    fn new(
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
        let (buffer, allocation, allocation_info) = allocator.create_buffer(
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

    fn fill<T: Sized>(
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

#[derive(Debug, Clone)]
struct InvalidHandle;
impl std::fmt::Display for InvalidHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "invalid handle")
    }
}
impl std::error::Error for InvalidHandle {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

struct Model<V, I> {
    vertex_data: Vec<V>,
    handle_to_index: std::collections::HashMap<usize, usize>,
    handles: Vec<usize>,
    instances: Vec<I>,
    first_invisible: usize,
    next_handle: usize,
    vertex_buffer: Option<Buffer>,
    instance_buffer: Option<Buffer>,
}

impl<V, I> Model<V, I> {
    fn get(&self, handle: usize) -> Option<&I> {
        if let Some(&index) = self.handle_to_index.get(&handle) {
            self.instances.get(index)
        } else {
            None
        }
    }
    fn get_mut(&mut self, handle: usize) -> Option<&mut I> {
        if let Some(&index) = self.handle_to_index.get(&handle) {
            self.instances.get_mut(index)
        } else {
            None
        }
    }
    fn swap_by_handle(&mut self, handle1: usize, handle2: usize) -> Result<(), InvalidHandle> {
        if handle1 == handle2 {
            return Ok(());
        }
        if let (Some(&index1), Some(&index2)) = (
            self.handle_to_index.get(&handle1),
            self.handle_to_index.get(&handle2),
        ) {
            self.handles.swap(index1, index2);
            self.instances.swap(index1, index2);
            self.handle_to_index.insert(index1, handle2);
            self.handle_to_index.insert(index2, handle1);
            Ok(())
        } else {
            Err(InvalidHandle)
        }
    }
    fn swap_by_index(&mut self, index1: usize, index2: usize) {
        if index1 == index2 {
            return;
        }
        let handle1 = self.handles[index1];
        let handle2 = self.handles[index2];
        self.handles.swap(index1, index2);
        self.instances.swap(index1, handle2);
        self.handle_to_index.insert(index1, handle2);
        self.handle_to_index.insert(index2, handle1);
    }
    fn is_visible(&self, handle: usize) -> Result<bool, InvalidHandle> {
        if let Some(index) = self.handle_to_index.get(&handle) {
            Ok(index < &self.first_invisible)
        } else {
            Err(InvalidHandle)
        }
    }
    fn make_visible(&mut self, handle: usize) -> Result<(), InvalidHandle> {
        // if already visible do nothing
        if let Some(&index) = self.handle_to_index.get(&handle) {
            if index < self.first_invisible {
                return Ok(());
            }
            // else move position first_invisible and increase value of first_invisible
            self.swap_by_index(index, self.first_invisible);
            self.first_invisible += 1;
            Ok(())
        } else {
            Err(InvalidHandle)
        }
    }
    fn make_invisible(&mut self, handle: usize) -> Result<(), InvalidHandle> {
        // if already invisible do nothing
        if let Some(&index) = self.handle_to_index.get(&handle) {
            if index >= self.first_invisible {
                return Ok(());
            }
            // else move position before first_invisible and decrease value of first_invisible
            self.swap_by_index(index, self.first_invisible - 1);
            self.first_invisible -= 1;
            Ok(())
        } else {
            Err(InvalidHandle)
        }
    }
    fn insert(&mut self, element: I) -> usize {
        let handle = self.next_handle;
        self.next_handle += 1;
        let index = self.instances.len();
        self.instances.push(element);
        self.handles.push(handle);
        self.handle_to_index.insert(handle, index);
        handle
    }
    fn insert_visibly(&mut self, element: I) -> usize {
        let new_handle = self.insert(element);
        self.make_visible(new_handle).ok(); //cant go wrong, see previous line
        new_handle
    }
    fn remove(&mut self, handle: usize) -> Result<I, InvalidHandle> {
        if let Some(&index) = self.handle_to_index.get(&handle) {
            if index < self.first_invisible {
                self.swap_by_index(index, self.first_invisible - 1);
                self.first_invisible -= 1;
            }
            self.swap_by_index(self.first_invisible, self.instances.len() - 1);
            self.handles.pop();
            self.handle_to_index.remove(&handle);
            //must be Some(), otherwise we couldnt have found and index
            Ok(self.instances.pop().unwrap())
        } else {
            Err(InvalidHandle)
        }
    }
    fn update_vertex_buffer(
        &mut self,
        allocator: &vk_mem::Allocator,
    ) -> Result<(), vk_mem::error::Error> {
        if let Some(buffer) = &mut self.vertex_buffer {
            buffer.fill(allocator, &self.vertex_data)?;
            Ok(())
        } else {
            let bytes = (self.vertex_data.len() * std::mem::size_of::<V>()) as u64;
            let mut buffer = Buffer::new(
                &allocator,
                bytes,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk_mem::MemoryUsage::CpuToGpu,
            )?;
            buffer.fill(allocator, &self.vertex_data)?;
            self.vertex_buffer = Some(buffer);
            Ok(())
        }
    }
    fn update_instance_buffer(
        &mut self,
        allocator: &vk_mem::Allocator,
    ) -> Result<(), vk_mem::error::Error> {
        if let Some(buffer) = &mut self.instance_buffer {
            buffer.fill(allocator, &self.instances[0..self.first_invisible])?;
            Ok(())
        } else {
            let bytes = (self.first_invisible * std::mem::size_of::<I>()) as u64;
            let mut buffer = Buffer::new(
                &allocator,
                bytes,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk_mem::MemoryUsage::CpuToGpu,
            )?;
            buffer.fill(allocator, &self.instances[0..self.first_invisible])?;
            self.instance_buffer = Some(buffer);
            Ok(())
        }
    }
    fn draw(&self, logical_device: &ash::Device, command_buffer: vk::CommandBuffer) {
        if let Some(vertex_buffer) = &self.vertex_buffer {
            if let Some(instance_buffer) = &self.instance_buffer {
                if self.first_invisible > 0 {
                    unsafe {
                        logical_device.cmd_bind_vertex_buffers(
                            command_buffer,
                            0,
                            &[vertex_buffer.buffer],
                            &[0],
                        );
                        logical_device.cmd_bind_vertex_buffers(
                            command_buffer,
                            1,
                            &[instance_buffer.buffer],
                            &[0],
                        );
                        logical_device.cmd_draw(
                            command_buffer,
                            self.vertex_data.len() as u32,
                            self.first_invisible as u32,
                            0,
                            0,
                        );
                    }
                }
            }
        }
    }
}

impl Model<[f32; 3], InstanceData> {
    fn cube() -> Model<[f32; 3], InstanceData> {
        let lbf = [-1.0, 1.0, 0.0]; //lbf: left-bottom-front
        let lbb = [-1.0, 1.0, 1.0];
        let ltf = [-1.0, -1.0, 0.0];
        let ltb = [-1.0, -1.0, 1.0];
        let rbf = [1.0, 1.0, 0.0];
        let rbb = [1.0, 1.0, 1.0];
        let rtf = [1.0, -1.0, 0.0];
        let rtb = [1.0, -1.0, 1.0];
        Model {
            vertex_data: vec![
                lbf, lbb, rbb, lbf, rbb, rbf, //bottom
                ltf, rtb, ltb, ltf, rtf, rtb, //top
                lbf, rtf, ltf, lbf, rbf, rtf, //front
                lbb, ltb, rtb, lbb, rtb, rbb, //back
                lbf, ltf, lbb, lbb, ltf, ltb, //left
                rbf, rbb, rtf, rbb, rtb, rtf, //right
            ],
            handle_to_index: std::collections::HashMap::new(),
            handles: Vec::new(),
            instances: Vec::new(),
            first_invisible: 0,
            next_handle: 0,
            vertex_buffer: None,
            instance_buffer: None,
        }
    }
}

#[repr(C)]
struct InstanceData {
    model_matrix: [[f32; 4]; 4],
    color: [f32; 3],
}

struct Fae {
    window: winit::window::Window,
    entry: ash::Entry,
    instance: ash::Instance,
    debug: std::mem::ManuallyDrop<FaeDebug>,
    surfaces: std::mem::ManuallyDrop<FaeSurface>,
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    queue_families: QueueFamilies,
    queues: Queues,
    device: ash::Device,
    swapchain: FaeSwapchain,
    render_pass: vk::RenderPass,
    pipeline: Pipeline,
    pools: Pools,
    command_buffers: Vec<vk::CommandBuffer>,
    allocator: vk_mem::Allocator,
    models: Vec<Model<[f32; 3], InstanceData>>,
}

impl Fae {
    fn init(window: winit::window::Window) -> Result<Fae, Box<dyn std::error::Error>> {
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
        let (physical_device, physical_device_properties, physical_device_features) =
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

        let allocation_create_info = vk_mem::AllocationCreateInfo {
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
            &queues,
            &allocator,
        )?;
        // create render pass
        let render_pass = init_render_pass(
            &logical_device,
            swapchain.surface_format.format,
        )?;
        // create framebuffers
        swapchain.create_framebuffers(&logical_device, render_pass)?;
        // create pipeline
        let pipeline = Pipeline::init(&logical_device, &swapchain, &render_pass)?;
        // create command pools
        let pools = Pools::init(&logical_device, &queue_families)?;
        // create command buffers
        let command_buffers =
            create_command_buffers(&logical_device, &pools, swapchain.amount_of_images)?;

        Ok(Fae {
            window,
            entry,
            instance,
            debug: std::mem::ManuallyDrop::new(debug),
            surfaces: std::mem::ManuallyDrop::new(surfaces),
            physical_device,
            physical_device_properties,
            queue_families,
            queues,
            device: logical_device,
            swapchain,
            render_pass,
            pipeline,
            pools,
            command_buffers,
            allocator,
            models: vec![],
        })
    }

    fn update_command_buffer(&mut self, index: usize) -> Result<(), vk::Result> {
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
            self.device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);
            self.device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
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
            }
            self.pools.cleanup(&self.device);
            self.pipeline.cleanup(&self.device);
            self.device.destroy_render_pass(self.render_pass, None);
            self.swapchain.cleanup(&self.device, &self.allocator);
            self.allocator.destroy();
            self.device.destroy_device(None);
            std::mem::ManuallyDrop::drop(&mut self.surfaces);
            std::mem::ManuallyDrop::drop(&mut self.debug);
            self.instance.destroy_instance(None);
        };
    }
}
