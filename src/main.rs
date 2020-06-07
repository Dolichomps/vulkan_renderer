use ash::{
    version::{EntryV1_0, InstanceV1_0},
    vk, Device, Entry, Instance,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new()?;
    let instance: Instance = unsafe { entry.create_instance(&Default::default(), None)? };
    unsafe { instance.destroy_instance(None) };
    Ok(())
}
