use log::info;
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, QueueFlags, Queue};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::VulkanLibrary;
use std::sync::Arc;

fn initialize() -> (Arc<Instance>, Arc<Queue>) {
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let instance = Instance::new(library, InstanceCreateInfo::default())
        .expect("failed to create Vulkan instance");
    let mut physical_devices = instance
        .enumerate_physical_devices()
        .expect("could not enumerate physical devices");
    info!(
        "Found {} compatible physical devices",
        physical_devices.len()
    );
    let physical_device = physical_devices
        .next()
        .expect("no physical device available");
    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        .position(|(_queue_family_index, queue_family_properties)| {
            queue_family_properties
                .queue_flags
                .contains(QueueFlags::GRAPHICS)
        })
        .expect("could not find a queue family supporting graphics")
        as u32;
    let (_device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            // the desired queue family by index
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .expect("failed to create device");
    let queue = queues.next().expect("no queue found");
    (instance, queue)
}

fn main() {
    env_logger::init();
    let (_instance, _queue) = initialize();
}
