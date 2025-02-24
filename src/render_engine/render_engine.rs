use std::sync::Arc;

use vulkano::{device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo, QueueFlags}, instance::{Instance, InstanceCreateFlags, InstanceCreateInfo}, swapchain::Surface, VulkanLibrary};
use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}};

pub struct RenderEngine {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface>
}

impl RenderEngine {
    pub fn new() -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");

        let library = VulkanLibrary::new().expect("No Vulkan library found");
        let required_extensions = Surface::required_extensions(&event_loop).expect("Could not get required extensions");
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            }
        )
        .expect("Failed to create instance");

        let physical_device = instance.clone()
            .enumerate_physical_devices()
            .expect("Failed to enumerate physical devices")
            .next()
            .expect("No physical device found");

        let queue_family_index = physical_device
            .queue_family_properties()
            .iter()
            .position(|q| q.queue_flags.contains(QueueFlags::GRAPHICS))
            .expect("No graphical queue family found") as u32;

        let (device, mut queues) = Device::new(
            physical_device, 
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            }
        )
        .expect("Failed to create device");

        let queue = queues.next().unwrap();

        let window = Arc::new(event_loop.create_window(Default::default()).expect("Failed to create window"));
        let surface = Surface::from_window(instance, window).expect("Failed to create surface");

        event_loop.run(|event, event_loop| {
            match event {
                Event::WindowEvent { 
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    event_loop.exit();
                },
                _ => ()
            }
        })
        .unwrap();

        Self { device , queue, surface }
    }

    pub fn render() {
        todo!()
    }
}