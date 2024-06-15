use log::info;
use std::sync::Arc;
use std::time::Duration;
use std::{default, thread};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo,
    SubpassContents,
};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::image::view::ImageView;
use vulkano::image::Image;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, Version, VulkanError, VulkanLibrary};
use vulkano_win::create_surface_from_winit;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{self, ControlFlow};
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

fn create_surface(instance: Arc<Instance>) -> (Arc<Surface>, event_loop::EventLoop<()>) {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = Window::new(&event_loop).expect("failed to create window");
    let surface =
        create_surface_from_winit(Arc::new(window), instance).expect("failed to create surface");
    (surface, event_loop)
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

fn get_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
        },
        pass: {
            color: [color],
            depth_stencil: {},
        },
    )
    .expect("failed to create render pass")
}

fn window_size_dependent_setup(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).expect("failed to create image view");
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .expect("failed to create framebuffer")
        })
        .collect::<Vec<_>>()
}

fn main() {
    env_logger::init();
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };
    let instance = get_instance();
    let (surface, event_loop) = create_surface(instance.clone());
    let (physical_device, queue_family_index) =
        get_physical_device(instance.clone(), surface.clone(), device_extensions);
    let (device, mut queues) =
        create_device(physical_device, queue_family_index, device_extensions);
    let queue = queues.next().expect("no queue found");
    let (mut swapchain, images) = create_swapchain(device.clone(), surface.clone(), queue.clone());
    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(device.clone(), Default::default());
    // Shaders would go here
    let render_pass = get_render_pass(device.clone(), swapchain.clone());
    // Setup the Graphics Pipeline
    let mut viewport = Viewport {
        offset: [0.0, 0.0],
        extent: [0.0, 0.0],
        depth_range: (0.0..=1.0).into(),
    };
    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

    let mut recreate_swapchain = false;
    let mut previous_frame_end =
        Some(Box::new(sync::now(device.clone())) as Box<dyn sync::GpuFuture>);

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                // release unused resources
                previous_frame_end
                    .as_mut()
                    .take()
                    .unwrap()
                    .cleanup_finished();
                // check if swapchain needs to be recreated
                if recreate_swapchain {
                    let window = surface
                        .object()
                        .expect("failed to get surface handle")
                        .downcast_ref::<Window>()
                        .expect("failed to get window handle");
                    let image_extent: [u32; 2] = window.inner_size().into();

                    let (new_swapchain, new_images) =
                        match swapchain.recreate(SwapchainCreateInfo {
                            image_extent,
                            ..swapchain.create_info()
                        }) {
                            Ok(r) => r,
                            Err(e) => {
                                panic!("failed to recreate swapchain: {:?}", e);
                            }
                        };

                    swapchain = new_swapchain;
                    framebuffers = window_size_dependent_setup(
                        &new_images,
                        render_pass.clone(),
                        &mut viewport,
                    );
                    recreate_swapchain = false;
                }

                let (image_index, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(Validated::Error(VulkanError::OutOfDate)) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => {
                            panic!("failed to acquire next image: {:?}", e);
                        }
                    };
                if suboptimal {
                    recreate_swapchain = true;
                }
                let clear_values = vec![Some([0.0, 0.0, 1.0, 1.0].into())];
                let mut cmd_buffer_builder = AutoCommandBufferBuilder::primary(
                    &command_buffer_allocator,
                    queue.queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();

                cmd_buffer_builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values,
                            ..RenderPassBeginInfo::framebuffer(
                                framebuffers[image_index as usize].clone(),
                            )
                        },
                        SubpassBeginInfo {
                            contents: SubpassContents::Inline,
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .end_render_pass(vulkano::command_buffer::SubpassEndInfo {
                        ..Default::default()
                    })
                    .unwrap();

                let command_buffer = cmd_buffer_builder.build().unwrap();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(
                        queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_index),
                    )
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(Box::new(future) as Box<_>);
                    }
                    Err(Validated::Error(VulkanError::OutOfDate)) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                }
            }
            _ => {}
        }
    });
}
