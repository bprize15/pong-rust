use std::sync::Arc;

use vulkano::{device::{physical::{self, PhysicalDevice}, Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags}, instance::{Instance, InstanceCreateFlags, InstanceCreateInfo}, swapchain::Surface, VulkanLibrary};
use winit::{event::{Event, WindowEvent}, event_loop::EventLoop};

use crate::GameObject;

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

        let window = Arc::new(event_loop.create_window(Default::default()).expect("Failed to create window"));
        let surface = Surface::from_window(instance.clone(), window).expect("Failed to create surface");

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

    pub fn render(&self, game_object: &GameObject) {
        todo!()
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