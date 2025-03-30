use std::{cell::RefCell, rc::Rc, sync::Arc};

use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo}, device::{physical::{self, PhysicalDevice}, Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags}, image::{view::ImageView, Image, ImageUsage}, instance::{Instance, InstanceCreateFlags, InstanceCreateInfo}, memory::allocator::{AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, MemoryTypeFilter, StandardMemoryAllocator}, pipeline::{graphics::{color_blend::{ColorBlendAttachmentState, ColorBlendState}, input_assembly::InputAssemblyState, multisample::MultisampleState, rasterization::RasterizationState, vertex_input::{Vertex, VertexDefinition}, viewport::{Viewport, ViewportState}, GraphicsPipelineCreateInfo}, layout::PipelineDescriptorSetLayoutCreateInfo, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass}, shader::ShaderModule, swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo}, sync::{self, GpuFuture}, Validated, VulkanError, VulkanLibrary};
use winit::{event_loop::EventLoop, window::{Window, WindowBuilder}};

use crate::GameObject;

pub struct RenderEngine {
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>, // TODO: See if free list allocator crashes after a while because of memory fragmentation,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    window: Arc<Window>,
    viewport: Viewport,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    recreate_swapchain: bool,
}

impl RenderEngine {
    pub fn new(event_loop: &EventLoop<()>) -> RenderEngine {
        let library = VulkanLibrary::new().expect("No Vulkan library found");
        let required_extensions = Surface::required_extensions(event_loop);
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            }
        )
        .expect("Failed to create instance");

        let window = Arc::new(WindowBuilder::new().build(event_loop).expect("Failed to create window"));
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

        let (swapchain, images) = Swapchain::new(
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

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0
        };

        let vertex_shader = vertex_shader::load(device.clone()).expect("Failed to load vertex shader");
        let fragment_shader = fragment_shader::load(device.clone()).expect("Failed to load fragment shader");

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(device.clone(), Default::default()));

        let pipeline = get_pipeline(
            device.clone(), 
            vertex_shader.clone(),
            fragment_shader.clone(),
            render_pass.clone(),
            viewport.clone()
        );

        Self {
            device,
            queue,
            swapchain,
            memory_allocator,
            command_buffer_allocator,
            pipeline,
            window,
            viewport,
            vertex_shader,
            fragment_shader,
            render_pass,
            framebuffers,
            recreate_swapchain: false
        }
    }

    pub fn draw(&mut self, game_objects: &Vec<Rc<RefCell<dyn GameObject>>>) {
        let squares:Vec<Square> = game_objects.iter()
            .map(|game_object| {
                Square { 
                    x: game_unit_to_render_unit(game_object.borrow().get_state().x) - 1.0, 
                    y: -1.0 * (game_unit_to_render_unit(game_object.borrow().get_state().y) - 1.0),
                    width: game_unit_to_render_unit(game_object.borrow().get_state().width),
                    height: game_unit_to_render_unit(game_object.borrow().get_state().height)
                }
            })
            .collect();

        self.render(squares);
    }

    fn render(&mut self, squares: Vec<Square>) {
        if self.recreate_swapchain {
            self.recreate_swapchain();
        }

        let (vertex_buffer, index_buffer) = get_square_buffers(squares, self.memory_allocator.clone());

        // TODO: no need to recreate index buffer
        let command_buffers = get_command_buffers(
            &self.command_buffer_allocator, 
            &self.queue,
            &self.pipeline, 
            &self.framebuffers, 
            &vertex_buffer,
            &index_buffer
        );

        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                },
                Err(e) => panic!("Failed to acquire next image {e}")
            };
        
        if suboptimal {
            self.recreate_swapchain = true;
        }

        let execution = sync::now(self.device.clone())
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffers[image_i as usize].clone())
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(), 
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i)
            )
            .then_signal_fence_and_flush();

        match execution.map_err(Validated::unwrap) {
            Ok(future) => {
                future.wait(None).unwrap();
            },
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
            },
            Err(e) => {
                println!("Failed to flush future: {e}");
            }
        }
    }

    pub fn on_window_resized(&mut self) {
        self.recreate_swapchain();

        let new_dimensions = self.window.inner_size();

        self.viewport.extent = new_dimensions.into();
        self.pipeline = get_pipeline(
            self.device.clone(), 
            self.vertex_shader.clone(),
            self.fragment_shader.clone(), 
            self.render_pass.clone(), 
            self.viewport.clone()
        );
    }

    fn recreate_swapchain(&mut self) {
        let new_dimensions = self.window.inner_size();

        let (new_swapchain, new_images) = self.swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: new_dimensions.into(),
                ..self.swapchain.create_info()
            })
            .expect("Failed to recreate swapchain");

        self.swapchain = new_swapchain;
        self.framebuffers = get_framebuffers(&new_images, &self.render_pass.clone())
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
    vertex_buffer: &Subbuffer<[MyVertex]>,
    index_buffer: &Subbuffer<[u32]>
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
                .bind_index_buffer(index_buffer.clone())
                .unwrap()
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
                .unwrap()
                .end_render_pass(SubpassEndInfo::default())
                .unwrap();

            builder.build().unwrap()
        })
        .collect()
}

fn get_square_buffers(squares: Vec<Square>, memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>) -> (Subbuffer<[MyVertex]>, Subbuffer<[u32]>) {
    let mut vertices: Vec<MyVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (i, square) in squares.iter().enumerate() {
        let base_index= (i * 4) as u32;

        let top_left = MyVertex {
            position: [square.x, square.y]
        };
        let top_right = MyVertex {
            position: [square.x + square.width, square.y]
        };
        let bottom_left = MyVertex {
            position: [square.x, square.y - square.height]
        };
        let bottom_right = MyVertex {
            position: [square.x + square.width, square.y - square.height]
        };

        vertices.push(top_left);
        vertices.push(top_right);
        vertices.push(bottom_left);
        vertices.push(bottom_right);

        indices.extend_from_slice(&[
            base_index + 0, base_index + 2, base_index + 3,
            base_index + 0, base_index + 1, base_index + 3,
        ]);
    }

    let vertex_buffer = Buffer::from_iter(
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
    .unwrap();

    let index_buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::INDEX_BUFFER,
            ..Default::default()
        }, 
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        }, 
        indices
    )
    .unwrap();

    (vertex_buffer, index_buffer)
}

#[derive(Debug)]
struct Square {
    x: f32,
    y: f32,
    width: f32,
    height: f32
}

fn game_unit_to_render_unit(game_unit: usize) -> f32 {
    (game_unit as f32) / 50.0 // TODO: generic scale and offset
}

