use ash::vk;
use winapi::{shared::windef::HWND, um::libloaderapi::GetModuleHandleW};

pub struct FaeSurface {
    pub surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
}

impl FaeSurface {
    pub fn init(
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
    pub fn get_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
        }
    }
    pub fn get_present_modes(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)
        }
    }
    pub fn get_formats(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device, self.surface)
        }
    }
    pub fn get_physical_device_surface_support(
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
