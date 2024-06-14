use log::info;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::VulkanLibrary;

fn main() {
    env_logger::init();
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let instance = Instance::new(library, InstanceCreateInfo::default())
        .expect("failed to create Vulkan instance");

    let mut physical_devices = instance
        .enumerate_physical_devices()
        .expect("could not enumerate physical devices");
    info!("Found {} compatible physical devices", physical_devices.len());

    let physical_device = physical_devices
        .next()
        .expect("no physical device available");

    for family in physical_device.queue_family_properties() {
        info!("Found a queue family with: {:?} queue(s)", family);
    }
}
