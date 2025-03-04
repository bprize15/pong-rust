use std::sync::Arc;

use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{self, allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo}, device::{physical::{self, PhysicalDevice}, Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags}, format, image::{view::ImageView, Image, ImageUsage}, instance::{Instance, InstanceCreateFlags, InstanceCreateInfo}, memory::allocator::{AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, MemoryTypeFilter, StandardMemoryAllocator}, pipeline::{graphics::{color_blend::{ColorBlendAttachmentState, ColorBlendState}, input_assembly::InputAssemblyState, multisample::MultisampleState, rasterization::RasterizationState, vertex_input::{Vertex, VertexDefinition}, viewport::{Viewport, ViewportState}, GraphicsPipelineCreateInfo}, layout::PipelineDescriptorSetLayoutCreateInfo, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}, shader::ShaderModule, swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo}, sync::{self, GpuFuture}, Validated, VulkanError, VulkanLibrary};
use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};

use crate::GameObject;

pub struct RenderEngine {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<Image>>,
    memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>, // TODO: See if free list allocator crashes after a while because of memory fragmentation,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>
}

impl RenderEngine {
    pub fn new() -> () {
        let event_loop = EventLoop::new();

        let library = VulkanLibrary::new().expect("No Vulkan library found");
        let required_extensions = Surface::required_extensions(&event_loop);
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            }
        )
        .expect("Failed to create instance");

        let window = Arc::new(WindowBuilder::new().build(&event_loop).expect("Failed to create window"));
        let surface = Surface::from_window(instance.clone(), window.clone()).expect("Failed to create surface");

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = select_physical_device(
            &instance,
            &surface,
            &device_extensions
        );

        let (device, mut queues) = Device::new(
            physical_device.clone(), 
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            }
        )
        .expect("Failed to create device");

        let queue = queues.next().unwrap();

        let caps = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("Failed to get surface capabilities");

        let dimensions = window.inner_size();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = physical_device
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

        let (mut swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count + 1,
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                ..Default::default()
            }
        )
        .unwrap();

        let render_pass = get_render_pass(device.clone(), &swapchain);
        let framebuffers = get_framebuffers(&images, &render_pass);

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0
        };

        let vertex_shader = vertex_shader::load(device.clone()).expect("Failed to load vertex shader");
        let fragment_shader = fragment_shader::load(device.clone()).expect("Failed to load fragment shader");

        let mut window_resized = false;
        let mut recreate_swapchain = false;

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(device.clone(), Default::default()));

        let top_left = MyVertex {
            position: [-1.0, 1.0]
        };
        let top_right = MyVertex {
            position: [0.0, 1.0]
        };
        let bottom_left = MyVertex {
            position: [-1.0, -1.0]
        };
        let bottom_right = MyVertex {
            position: [0.0, -1.0]
        };
        let initial_vertex_buffer = get_vertex_buffer([top_left, top_right, bottom_left, bottom_right], memory_allocator.clone());

        let pipeline = get_pipeline(
            device.clone(), 
            vertex_shader.clone(),
            fragment_shader.clone(),
            render_pass.clone(),
            viewport.clone()
        );

        let mut command_buffers = get_command_buffers(
            &command_buffer_allocator, 
            &queue, 
            &pipeline, 
            &framebuffers, 
            &initial_vertex_buffer
        );

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { 
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                },
                Event::WindowEvent { 
                    event: WindowEvent::Resized(..),
                    ..
                } => {
                    window_resized = true;
                },
                Event::MainEventsCleared => {
                    if window_resized || recreate_swapchain {
                        recreate_swapchain = false;

                        let new_dimensions = window.inner_size();

                        let (new_swapchain, new_images) = swapchain
                            .recreate(SwapchainCreateInfo {
                                image_extent: new_dimensions.into(),
                                ..swapchain.create_info()
                            })
                            .expect("Failed to recreate swapchain {e}");
                        swapchain = new_swapchain;
                        let new_framebuffers = get_framebuffers(&new_images, &render_pass);

                        if window_resized {
                            window_resized = false;

                            viewport.extent = new_dimensions.into();
                            let new_pipeline = get_pipeline(
                                device.clone(), 
                                vertex_shader.clone(), 
                                fragment_shader.clone(), 
                                render_pass.clone(), 
                                viewport.clone()
                            );
                            command_buffers = get_command_buffers(
                                &command_buffer_allocator, 
                                &queue,
                                &new_pipeline, 
                                &new_framebuffers, 
                                &initial_vertex_buffer
                            );

                            let (image_i, suboptimal, acquire_future) =
                                match swapchain::acquire_next_image(swapchain.clone(), None)
                                    .map_err(Validated::unwrap)
                                {
                                    Ok(r) => r,
                                    Err(VulkanError::OutOfDate) => {
                                        recreate_swapchain = true;
                                        return;
                                    },
                                    Err(e) => panic!("Failed to acquire next image {e}")
                                };

                            if suboptimal {
                                recreate_swapchain = true;
                            }

                            let execution = sync::now(device.clone())
                                .join(acquire_future)
                                .then_execute(queue.clone(), command_buffers[image_i as usize].clone())
                                .unwrap()
                                .then_swapchain_present(
                                    queue.clone(), 
                                    SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_i)
                                )
                                .then_signal_fence_and_flush();

                            match execution.map_err(Validated::unwrap) {
                                Ok(future) => {
                                    future.wait(None).unwrap();
                                },
                                Err(VulkanError::OutOfDate) => {
                                    recreate_swapchain = true;
                                },
                                Err(e) => {
                                    println!("Failed to flush future: {e}");
                                }
                            }
                        }
                    }
                },
                _ => ()
            }
        });

        // Self { device , queue, surface, swapchain, images, memory_allocator, command_buffer_allocator }
    }

    pub fn render(&self, game_object: &GameObject) {
        let top_left = MyVertex {
            position: [-1.0, 1.0]
        };
        let top_right = MyVertex {
            position: [0.0, 1.0]
        };
        let bottom_left = MyVertex {
            position: [-1.0, -1.0]
        };
        let bottom_right = MyVertex {
            position: [0.0, -1.0]
        };
        // let vertex_buffer = get_vertex_buffer([top_left, top_right, bottom_left, bottom_right], self.memory_allocator.clone());
        todo!()
    }
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2]
}

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 460

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
    }
}

fn select_physical_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("Failed to enumerate physical devices")
        .filter(|device| device.supported_extensions().contains(&device_extensions))
        .filter_map(|device| {
            device.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS) && device.surface_support(i as u32, &surface).unwrap()
                })
                .map(|q| (device, q as u32))
        })
        .min_by_key(|(device, _)| match device.properties().device_type {
            physical::PhysicalDeviceType::DiscreteGpu => 0,
            physical::PhysicalDeviceType::IntegratedGpu => 1,
            physical::PhysicalDeviceType::VirtualGpu => 2,
            physical::PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("No physical device found")
}

fn get_render_pass(device: Arc<Device>, swapchain: &Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store   
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )
    .unwrap()
}

fn get_framebuffers(images: &[Arc<Image>], render_pass: &Arc<RenderPass>) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                }
            )
            .unwrap()
        })
        .collect()
}

fn get_pipeline(
    device: Arc<Device>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport
) -> Arc<GraphicsPipeline> {
    let vertex_shader = vertex_shader.entry_point("main").unwrap();
    let fragment_shader = fragment_shader.entry_point("main").unwrap();

    let vertex_input_state = MyVertex::per_vertex()
        .definition(&vertex_shader.info().input_interface)
        .unwrap();

    let stages = [
        PipelineShaderStageCreateInfo::new(vertex_shader),
        PipelineShaderStageCreateInfo::new(fragment_shader)
    ];

    let layout = PipelineLayout::new(
        device.clone(), 
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())
            .unwrap()
    )
    .unwrap();

    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    GraphicsPipeline::new(
        device.clone(), 
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState {
                viewports: [viewport].into_iter().collect(),
                ..Default::default()
            }),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(), 
                ColorBlendAttachmentState::default()
            )),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        }
    )
    .unwrap()
}

fn get_command_buffers(
    command_buffer_allocator: &StandardCommandBufferAllocator,
    queue: &Arc<Queue>,
    pipeline: &Arc<GraphicsPipeline>,
    framebuffers: &[Arc<Framebuffer>],
    vertex_buffer: &Subbuffer<[MyVertex]>
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    framebuffers
        .iter()
        .map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                command_buffer_allocator, 
                queue.queue_family_index(), 
                CommandBufferUsage::MultipleSubmit
            )
            .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into())],
                        ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                    },
                    SubpassBeginInfo {
                        contents: SubpassContents::Inline,
                        ..Default::default()
                    }
                )
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap()
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass(SubpassEndInfo::default())
                .unwrap();

            builder.build().unwrap()
        })
        .collect()
}

fn get_vertex_buffer(vertices: [MyVertex; 4], memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>) -> Subbuffer<[MyVertex]> {
    Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        }, 
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        }, 
        vertices
    )
    .unwrap()
}