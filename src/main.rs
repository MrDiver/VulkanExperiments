use std::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    sync::Arc,
};

use vulkano::{
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Features, Queue,
    },
    swapchain::Surface,
};
use vulkano::{
    device::{
        physical::{PhysicalDevice, QueueFamily},
        QueueCreateInfo,
    },
    instance::{
        debug::{DebugCallback, Message, MessageSeverity, MessageType},
        layers_list, Instance, InstanceCreateInfo, InstanceExtensions,
    },
};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
};

struct HelloTriangleApplication {
    instance: Arc<Instance>,
    physical_device_index: usize,
    logical_device: Arc<Device>,
    device_queues: Vec<Arc<Queue>>,
    debug_callback: Option<DebugCallback>,
    event_loop: Option<EventLoop<()>>,
    surface: Arc<Surface<Window>>,
}

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

#[cfg(all(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

impl HelloTriangleApplication {
    pub fn new() -> Self {
        let instance: Arc<Instance> = Self::create_instance();
        let (event_loop, surface) = Self::init_window(instance.clone());
        let debug_callback = Self::setup_debug_callback(&instance);
        let physical_device = Self::pick_physical_device(&instance);
        let physical_device_index = physical_device.index();
        let (logical_device, device_queues) = Self::create_logical_device(physical_device);

        let event_loop = Some(event_loop);

        Self {
            instance,
            physical_device_index,
            logical_device,
            device_queues,
            debug_callback,
            event_loop,
            surface,
        }
    }

    fn required_extensions() -> InstanceExtensions {
        let mut extensions = vulkano_win::required_extensions();

        if ENABLE_VALIDATION_LAYERS {
            extensions.ext_debug_utils = true;
        }

        let supported_extensions: InstanceExtensions =
            InstanceExtensions::supported_by_core().unwrap();

        if !supported_extensions.is_superset_of(&extensions) {
            let not_supported = extensions.difference(&supported_extensions);
            panic!(
                "Not supported Extensions but required:get_required_extensions {:?}",
                not_supported
            );
        }

        extensions
    }

    fn validation_layers() -> Vec<std::string::String> {
        let mut layers: Vec<std::string::String> = Vec::new();
        if ENABLE_VALIDATION_LAYERS {
            let available_layers = layers_list().expect("Couldn't retrieve layers list");
            for layer in available_layers {
                println!("{}", layer.name())
            }

            layers.push("VK_LAYER_KHRONOS_validation".into());
        }
        layers
    }

    fn setup_debug_callback(instance: &Arc<Instance>) -> Option<DebugCallback> {
        if !ENABLE_VALIDATION_LAYERS {
            return None;
        }

        let msg_types = MessageType {
            general: true,
            validation: true,
            performance: true,
        };

        let msg_severity = MessageSeverity {
            error: true,
            warning: true,
            information: true,
            verbose: true,
        };

        let callback = DebugCallback::new(instance, msg_severity, msg_types, |msg| {
            println!("{:?}", msg.description);
        })
        .expect("Couldn't create DebugCallback");
        Some(callback)
    }

    fn create_instance() -> Arc<Instance> {
        /* Create instance */
        let instance = Instance::new(InstanceCreateInfo {
            application_name: Some("My Vulkan Triangle".into()),
            enabled_extensions: Self::required_extensions(),
            enabled_layers: Self::validation_layers(),
            ..Default::default()
        })
        .expect("Failed to create Instance");

        instance
    }

    fn is_suitable_queue(queue_family: &QueueFamily) -> bool {
        queue_family.supports_graphics()
    }

    fn get_suitable_queue_ids(physical_device: &PhysicalDevice) -> Vec<u32> {
        physical_device
            .queue_families()
            .filter(Self::is_suitable_queue)
            .map(|qf| qf.id())
            .collect()
    }

    fn is_device_suitable(device: &PhysicalDevice) -> bool {
        let properties = device.properties();
        let features = device.supported_features();
        let has_suitable_queues = !Self::get_suitable_queue_ids(device).is_empty();

        if (properties.device_type == PhysicalDeviceType::DiscreteGpu)
            && features.geometry_shader
            && has_suitable_queues
        {
            return true;
        }
        false
    }

    fn pick_physical_device(instance: &Arc<Instance>) -> PhysicalDevice {
        let suitable_device: PhysicalDevice = PhysicalDevice::enumerate(instance)
            .filter(Self::is_device_suitable)
            .next()
            .expect("No Physical device found");

        suitable_device
    }

    fn create_logical_device(physical_device: PhysicalDevice) -> (Arc<Device>, Vec<Arc<Queue>>) {
        let family = physical_device
            .queue_family_by_id(Self::get_suitable_queue_ids(&physical_device)[0])
            .expect("No suitable queue family id found in physical device");

        let mut queue_create_infos = Vec::new();
        queue_create_infos.push(QueueCreateInfo::family(family));

        let (device, queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: DeviceExtensions::none(),
                enabled_features: Features::none(),
                queue_create_infos,
                ..Default::default()
            },
        )
        .expect("Couldn't create device");

        (device, queues.collect())
    }

    fn init_window(instance: Arc<Instance>) -> (EventLoop<()>, Arc<Surface<Window>>) {
        let event_loop = EventLoop::new();
        let surface = WindowBuilder::new()
            .with_title("My Vulkan Triangle")
            .with_inner_size(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
            .build_vk_surface(&event_loop, instance)
            .expect("Failed to create Surface");

        (event_loop, surface)
    }

    pub fn main_loop(&mut self) {
        self.event_loop
            .take()
            .expect("Window might not be initialized")
            .run(move |event, _window_target, control_flow| {
                *control_flow = ControlFlow::Wait;

                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => (),
                }
            });
    }
}

fn main() {
    let mut app = HelloTriangleApplication::new();
    app.main_loop();
}
