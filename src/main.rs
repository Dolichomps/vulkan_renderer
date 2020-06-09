use ash::{
    extensions::khr::{Surface, Swapchain, Win32Surface},
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk, Entry,
};
use winapi::{shared::windef::HWND, um::libloaderapi::GetModuleHandleW};
use winit::{event_loop::EventLoop, platform::windows::WindowExtWindows, window::Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // winit eventloop and window initialization
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop)?;

    // vulkan entry
    let entry = Entry::new()?;

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
    let layer_names: Vec<std::ffi::CString> =
        vec![std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
    let layer_name_pointers: Vec<*const i8> = layer_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();
    let extension_name_pointers: Vec<*const i8> = vec![
        ash::extensions::ext::DebugUtils::name().as_ptr(),
        Surface::name().as_ptr(),
        Win32Surface::name().as_ptr(),
    ];

    // setup debug create info
    let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
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

    // setup instance creation info
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_create_info)
        .application_info(&app_info)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_extension_names(&extension_name_pointers);

    // create vulkan instance
    let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

    // create debug messenger instance
    let debug_utils = ash::extensions::ext::DebugUtils::new(&entry, &instance);
    let utils_messenger =
        unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None)? };

    // TODO: make cross platform is currently windows only
    // TODO: use conditional compilation
    // create window surface
    let hwnd = window.hwnd() as HWND;
    let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) as *const std::ffi::c_void };
    let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
        s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
        p_next: std::ptr::null(),
        flags: Default::default(),
        hinstance: h_instance,
        hwnd: hwnd as *const std::ffi::c_void,
    };
    let win32_surface_loader = Win32Surface::new(&entry, &instance);
    let surface = unsafe { win32_surface_loader.create_win32_surface(&win32_create_info, None) }?;
    let surface_loader = Surface::new(&entry, &instance);

    // pick gpu to use (in this case it the discrete gpu)
    let phys_devs = unsafe { instance.enumerate_physical_devices()? };
    let (physical_device, physical_device_properties) = {
        let mut chosen = None;
        for p in phys_devs {
            let properties = unsafe { instance.get_physical_device_properties(p) };
            if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                chosen = Some((p, properties));
            }
        }
        chosen.unwrap()
    };

    // TODO: this could be done like in the ash examples to make sure
    // physical device is surface and graphics capable
    // find queue family indices
    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
    let q_fam_indicies = {
        let mut found_grapics_q_index = None;
        let mut found_transfer_q_index = None;
        for (index, q_fam) in queue_family_properties.iter().enumerate() {
            if q_fam.queue_count > 0
                && q_fam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && unsafe {
                    surface_loader.get_physical_device_surface_support(
                        physical_device,
                        index as u32,
                        surface,
                    )?
                }
            {
                found_grapics_q_index = Some(index as u32);
            }
            if q_fam.queue_count > 0 && q_fam.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                if found_transfer_q_index.is_none()
                    || !q_fam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                {
                    found_transfer_q_index = Some(index as u32);
                }
            }
        }
        (
            found_grapics_q_index.unwrap(),
            found_transfer_q_index.unwrap(),
        )
    };

    // create a logical device as primary interface to gpu
    let priorities = [1.0f32];
    let queue_infos = [
        // GRAPHICS QUEUE
        // TODO: use only one DeviceQueueCreateInfo
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(q_fam_indicies.0)
            .queue_priorities(&priorities)
            .build(),
        // TRANSFER QUEUE
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(q_fam_indicies.1)
            .queue_priorities(&priorities)
            .build(),
    ];

    // device creation
    let device_extension_name_pointers: Vec<*const i8> = vec![Swapchain::name().as_ptr()];
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extension_name_pointers)
        .enabled_layer_names(&layer_name_pointers);
    let logical_device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };
    let graphics_queue = unsafe { logical_device.get_device_queue(q_fam_indicies.0, 0) };
    let transfer_queue = unsafe { logical_device.get_device_queue(q_fam_indicies.1, 0) };

    // query surface information
    let surface_capabilites = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
    };
    let surface_present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?
    };
    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface)? };

    // create swapchain
    let queue_families = [q_fam_indicies.0];
    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface)
        .min_image_count(
            3.max(surface_capabilites.min_image_count)
                .min(surface_capabilites.max_image_count),
        )
        .image_format(surface_formats.first().unwrap().format)
        .image_color_space(surface_formats.first().unwrap().color_space)
        .image_extent(surface_capabilites.current_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .queue_family_indices(&queue_families)
        .pre_transform(surface_capabilites.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(vk::PresentModeKHR::FIFO);
    let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &logical_device);
    let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

    // get Vec of vkImages
    let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

    //create ImageViews
    let mut swapchain_image_views: Vec<vk::ImageView> = Vec::with_capacity(swapchain_images.len());
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

    // cleanup
    unsafe {
        for image_view in &swapchain_image_views {
            logical_device.destroy_image_view(*image_view, None)
        }
        swapchain_loader.destroy_swapchain(swapchain, None);
        surface_loader.destroy_surface(surface, None);
        logical_device.destroy_device(None);
        debug_utils.destroy_debug_utils_messenger(utils_messenger, None);
        instance.destroy_instance(None)
    };

    Ok(())
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
