use bytemuck::{Pod, Zeroable};
use log::info;
use nalgebra_glm::TMat4;
use std::sync::Arc;
use std::time::Duration;
use std::{default, thread};
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo,
    SubpassContents,
};
use vulkano::descriptor_set::allocator::{
    StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo,
};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::image::view::ImageView;
use vulkano::image::Image;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{
    BuffersDefinition, VertexDefinition, VertexInputState,
};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, Version, VulkanError, VulkanLibrary};
use vulkano_win::create_surface_from_winit;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{self, ControlFlow};
use winit::window::{Window, WindowAttributes};

#[derive(Debug, Clone)]
struct MVP {
    model: TMat4<f32>,
    view: TMat4<f32>,
    projection: TMat4<f32>,
}

impl MVP {
    fn new() -> Self {
        Self {
            model: TMat4::identity(),
            view: TMat4::identity(),
            projection: TMat4::identity(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}
vulkano::impl_vertex!(Vertex, position, color);

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 color;

            layout(location = 0) out vec3 out_color;

            layout(set = 0, binding = 0) uniform MVP {
                mat4 model;
                mat4 view;
                mat4 projection;
            } uniforms;

            void main() {
                mat4 worldview = uniforms.view * uniforms.model;
                gl_Position = vec4(position, 1.0) * worldview * uniforms.projection;
                out_color = color;
            }
        "
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) in vec3 in_color;

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(in_color, 1.0);
            }
        "
    }
}

fn get_shader(device: Arc<Device>) -> (Arc<ShaderModule>, Arc<ShaderModule>) {
    let vs = vs::load(device.clone()).expect("failed to create shader module");
    let fs = fs::load(device.clone()).expect("failed to create shader module");
    (vs, fs)
}

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

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
    let describtor_set_allocator = StandardDescriptorSetAllocator::new(
        device.clone(),
        StandardDescriptorSetAllocatorCreateInfo::default(),
    );

    let vertices = [
        Vertex {
            position: [-0.5, 0.5, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [0.0, -0.5, 0.0],
            color: [0.0, 0.0, 1.0],
        },
    ];

    let vertex_buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        vertices,
    )
    .expect("failed to create vertex buffer");

    let render_pass = get_render_pass(device.clone(), swapchain.clone());
    let subpass = Subpass::from(render_pass.clone(), 0).expect("failed to get subpass");

    let (vs, fs) = get_shader(device.clone());
    let vs_entry_point = vs.entry_point("main").expect("failed to get entry point");
    let fs_entry_point = fs.entry_point("main").expect("failed to get entry point");

    let uniform_buffer = SubbufferAllocator::new(
        memory_allocator.clone(),
        SubbufferAllocatorCreateInfo {
            buffer_usage: BufferUsage::UNIFORM_BUFFER,
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
    );

    let vertex_input_state =
        <Vertex as vulkano::pipeline::graphics::vertex_input::Vertex>::per_vertex()
            .definition(&vs_entry_point.info().input_interface)
            .expect("failed to get vertex input state");

    let stages = [
        PipelineShaderStageCreateInfo::new(vs_entry_point),
        PipelineShaderStageCreateInfo::new(fs_entry_point),
    ];

    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())
            .expect("failed to create pipeline layout"),
    )
    .expect("failed to create pipeline layout");
    let pipeline = GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState::default()),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState::default(),
            )),
            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .expect("failed to create pipeline");

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

                let uniform_subbuffer = {
                    let mvp = MVP::new();
                    let uniform_data = vs::MVP {
                        model: mvp.model.into(),
                        view: mvp.view.into(),
                        projection: mvp.projection.into(),
                    };

                    let uniform_subbuffer = uniform_buffer
                        .allocate_sized()
                        .expect("failed to allocate uniform buffer");
                    *uniform_subbuffer.write().unwrap() = uniform_data;

                    uniform_subbuffer
                };
                let layout = pipeline
                    .layout()
                    .set_layouts()
                    .get(0)
                    .expect("failed to get layout 0");
                let set = PersistentDescriptorSet::new(
                    &describtor_set_allocator,
                    layout.clone(),
                    [WriteDescriptorSet::buffer(0, uniform_subbuffer)],
                    [],
                )
                .expect("failed to create descriptor set");

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
                    .set_viewport(0, vec![viewport.clone()].into())
                    .expect("failed to set viewport")
                    .bind_pipeline_graphics(pipeline.clone())
                    .expect("failed to bind pipeline")
                    .bind_descriptor_sets(
                        vulkano::pipeline::PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        0,
                        set.clone(),
                    )
                    .expect("failed to bind descriptor set")
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .expect("failed to bind vertex buffer")
                    .draw(vertex_buffer.len() as u32, 1, 0, 0)
                    .expect("failed to draw")
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
