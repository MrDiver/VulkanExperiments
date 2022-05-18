use std::{collections::HashSet, sync::Arc};

use vulkano::{
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Features, Queue,
    },
    format::Format,
    swapchain::{ColorSpace, PresentMode, Surface, SurfaceCapabilities, SurfaceInfo},
    Version,
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

struct QueueFamilyIndices {
    graphics_family_id: Option<u32>,
    presentation_family_id: Option<u32>,
}

impl QueueFamilyIndices {
    fn new() -> Self {
        Self {
            graphics_family_id: None,
            presentation_family_id: None,
        }
    }

    fn is_complete(&self) -> bool {
        self.graphics_family_id.is_some() && self.presentation_family_id.is_some()
    }
}
struct HelloTriangleApplication {
    instance: Arc<Instance>,
    physical_device_index: usize,
    logical_device: Arc<Device>,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
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
        let physical_device_index = Self::pick_physical_device(&instance, &surface);
        let (logical_device, graphics_queue, present_queue) =
            Self::create_logical_device(physical_device_index, &instance, &surface);
        // println!("Physical_Device: {:?}", physical_device);
        // println!("Logical_Device: {:?}", logical_device);

        let event_loop = Some(event_loop);

        Self {
            instance,
            physical_device_index,
            logical_device,
            graphics_queue,
            present_queue,
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
            // let available_layers = layers_list().expect("Couldn't retrieve layers list");
            // for layer in available_layers {
            //     println!("{}", layer.name())
            // }

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
            // max_api_version: Some(Version::V1_3),
            ..Default::default()
        })
        .expect("Failed to create Instance");

        instance
    }

    fn find_queue_family_ids(
        physical_device: &PhysicalDevice,
        surface: &Arc<Surface<Window>>,
    ) -> QueueFamilyIndices {
        let mut family_ids = QueueFamilyIndices::new();
        let families = physical_device.queue_families();

        for family in families {
            if family.supports_graphics() {
                family_ids.graphics_family_id = Some(family.id())
            }
            if family
                .supports_surface(surface)
                .expect("Error while checking Surface drawing support")
            {
                family_ids.presentation_family_id = Some(family.id())
            }
            if family_ids.is_complete() {
                break;
            }
        }

        family_ids
    }

    fn query_swap_chain_support(
        physical_device_index: usize,
        instance: &Arc<Instance>,
        surface: &Arc<Surface<Window>>,
    ) -> (
        SurfaceCapabilities,
        Vec<(Format, ColorSpace)>,
        Vec<PresentMode>,
    ) {
        let physical_device = PhysicalDevice::from_index(instance, physical_device_index).unwrap();
        let surface_info = SurfaceInfo::default();
        let capabilities =
            physical_device.surface_capabilities(surface, surface_info.clone()).unwrap();
        let formats = physical_device.surface_formats(surface, surface_info).unwrap();
        let present_modes = physical_device.surface_present_modes(surface).unwrap();
        return (capabilities, formats, present_modes.collect());
    }

    fn is_device_suitable(
        physical_device: &PhysicalDevice,
        instance: &Arc<Instance>,
        surface: &Arc<Surface<Window>>,
    ) -> bool {
        let properties = physical_device.properties();
        let _features = physical_device.supported_features();
        let supported_extensions = physical_device.supported_extensions();
        let queue_family_ids = Self::find_queue_family_ids(physical_device, surface);
        let (_capabilities, formats, present_modes) =
            Self::query_swap_chain_support(physical_device.index(), instance, surface);

        let mut swap_chain_supported = false;
        if supported_extensions.khr_swapchain {
            swap_chain_supported = !formats.is_empty() && !present_modes.is_empty();
        }

        if (properties.device_type == PhysicalDeviceType::DiscreteGpu)
            && swap_chain_supported
            && queue_family_ids.is_complete()
            && supported_extensions.khr_swapchain
        {
            return true;
        }
        false
    }

    fn pick_physical_device(instance: &Arc<Instance>, surface: &Arc<Surface<Window>>) -> usize {
        let suitable_device: PhysicalDevice = PhysicalDevice::enumerate(instance)
            .filter(|device| Self::is_device_suitable(device, instance, surface))
            .next()
            .expect("No Physical device found");

        suitable_device.index()
    }

    fn create_logical_device(
        physical_device_index: usize,
        instance: &Arc<Instance>,
        surface: &Arc<Surface<Window>>,
    ) -> (Arc<Device>, Arc<Queue>, Arc<Queue>) {
        let physical_device = PhysicalDevice::from_index(instance, physical_device_index)
            .expect("Couldn't retrieve physical device by index while creating logical device");

        let queue_family_ids = Self::find_queue_family_ids(&physical_device, surface);

        let unique_family_ids: HashSet<u32> = vec![
            queue_family_ids.graphics_family_id.unwrap(),
            queue_family_ids.presentation_family_id.unwrap(),
        ]
        .into_iter()
        .collect();

        let queue_create_infos = unique_family_ids
            .into_iter()
            .map(|id| physical_device.queue_family_by_id(id).unwrap())
            .map(|family| QueueCreateInfo::family(family))
            .collect();

        let mut device_extensions = DeviceExtensions::none();
        device_extensions.khr_swapchain = true;

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: Features::none(),
                queue_create_infos,
                ..Default::default()
            },
        )
        .expect("Couldn't create device");

        let graphics_queue = queues.next().unwrap();
        let present_queue = queues.next().unwrap_or_else(|| graphics_queue.clone());

        (device, graphics_queue, present_queue)
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
        self.event_loop.take().expect("Window might not be initialized").run(
            move |event, _window_target, control_flow| {
                *control_flow = ControlFlow::Wait;

                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => (),
                }
            },
        );
    }
}

fn main() {
    let mut app = HelloTriangleApplication::new();
    // app.main_loop();
}
