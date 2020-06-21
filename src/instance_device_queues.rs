use crate::surface::FaeSurface;
use ash::{
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk,
};

pub fn init_instance(
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
        .pfn_user_callback(Some(crate::debug::vulkan_debug_utils_callback));

    // setup instance creation info
    let instance_create_info = vk::InstanceCreateInfo::builder()
        .push_next(&mut debug_create_info)
        .application_info(&app_info)
        .enabled_layer_names(&layer_name_pointers)
        .enabled_extension_names(&extension_name_pointers);

    unsafe { entry.create_instance(&instance_create_info, None) }
}

pub fn init_physical_device_and_properties(
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

pub struct QueueFamilies {
    pub graphics_q_index: Option<u32>,
    pub transfer_q_index: Option<u32>,
}
impl QueueFamilies {
    pub fn init(
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

pub struct Queues {
    pub graphics_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
}

pub fn init_device_and_queues(
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
