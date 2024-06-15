use log::info;
use std::default;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::image::Image;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use vulkano::{Version, VulkanLibrary};
use vulkano_win::create_surface_from_winit;
use winit::window::{Window, WindowAttributes};

fn get_instance() -> Arc<Instance> {
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let extensions = vulkano_win::required_extensions(&library);
    Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions: extensions,
            application_name: Some("Hello Triangle".into()),
            engine_name: Some("Ant Engine".into()),
            engine_version: 0.into(),
            max_api_version: Some(Version::V1_1),
            ..Default::default()
        },
    )
    .expect("failed to create Vulkan instance")
}

fn create_surface(instance: Arc<Instance>) -> Arc<Surface> {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = Window::new(&event_loop).expect("failed to create window");
    let surface =
        create_surface_from_winit(Arc::new(window), instance).expect("failed to create surface");
    surface
}

fn get_physical_device(
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    device_extensions: DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .expect("failed to enumerate physical devices")
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    // TODO: choose the best queue family
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|i| (p, i as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        })
        .expect("no suitable physical device found");
    (physical_device, queue_family_index)
}

fn create_device(
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
    device_extensions: DeviceExtensions,
) -> (
    Arc<Device>,
    impl Iterator<Item = Arc<Queue>> + ExactSizeIterator,
) {
    Device::new(
        physical_device,
        DeviceCreateInfo {
            enabled_extensions: device_extensions,
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .expect("failed to create device")
}

fn create_swapchain(
    device: Arc<Device>,
    surface: Arc<Surface>,
    queue: Arc<Queue>,
) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
    let capabilities = device
        .physical_device()
        .surface_capabilities(&surface, Default::default())
        .expect("failed to get surface capabilities");

    let usage = capabilities.supported_usage_flags;
    let alpha = capabilities
        .supported_composite_alpha
        .into_iter()
        .nth(0) // TODO: choose the best alpha mode
        .unwrap();

    let image_format = device
        .physical_device()
        .surface_formats(&surface, Default::default())
        .expect("failed to get surface formats")
        .iter()
        .nth(0) //TODO: choose the best format
        .unwrap()
        .0;

    let window = surface
        .object()
        .expect("failed to get surface handle")
        .downcast_ref::<Window>()
        .expect("failed to get window handle");
    let image_extent: [u32; 2] = window.inner_size().into();

    Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: capabilities.min_image_count,
            image_format,
            image_extent,
            image_usage: usage,
            composite_alpha: alpha,
            ..Default::default()
        },
    )
    .expect("failed to create swapchain")
}

fn initialize() -> (Arc<Instance>, Arc<Device>, Arc<Queue>) {
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };
    let instance = get_instance();
    let surface = create_surface(instance.clone());
    let (physical_device, queue_family_index) =
        get_physical_device(instance.clone(), surface.clone(), device_extensions);
    let (device, mut queues) =
        create_device(physical_device, queue_family_index, device_extensions);
    let queue = queues.next().expect("no queue found");
    let (swapchain, images) = create_swapchain(device.clone(), surface.clone(), queue.clone());

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
    let (device, mut queues) = Device::new(
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
    (instance, device, queue)
}

fn move_stuff(memory_allocator: Arc<StandardMemoryAllocator>, device: Arc<Device>) {
    let source_content: Vec<i32> = (0..64).collect();
    let source = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        source_content,
    )
    .expect("failed to create source buffer");

    let destination_content = vec![0; 64];
    let destination = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
        destination_content,
    )
    .expect("failed to create destination buffer");

    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );
}

fn main() {
    env_logger::init();
    let (_instance, device, _queue) = initialize();
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
}
