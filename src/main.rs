use std::{cmp, collections::HashSet, sync::Arc};

use vulkano::{
    device::{
        self,
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{self, ImageUsage, SwapchainImage},
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
            DebugUtilsMessengerCreateInfo,
        },
        Instance, InstanceCreateInfo, InstanceExtensions,
    },
    swapchain::{
        ColorSpace, PresentMode, Surface, SurfaceCapabilities, SurfaceInfo, Swapchain,
        SwapchainCreateInfo,
    },
    sync::Sharing,
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

fn clamp<T: Ord>(val: T, min: T, max: T) -> T {
    cmp::max(cmp::min(val, max), min)
}

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
    swapchain: Arc<Swapchain<Window>>,
    images: Vec<Arc<SwapchainImage<Window>>>,
    image_format: Format,
    image_extent: [u32; 2],
    debug_callback: Option<DebugUtilsMessenger>,
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
        let (swapchain, images, image_format, image_extent) =
            Self::create_swap_chain(physical_device_index, &logical_device, &instance, &surface);
        // println!("Physical_Device: {:?}", physical_device);
        // println!("Logical_Device: {:?}", logical_device);

        let event_loop = Some(event_loop);

        Self {
            instance,
            physical_device_index,
            logical_device,
            graphics_queue,
            present_queue,
            swapchain,
            images,
            image_format,
            image_extent,
            debug_callback,
            event_loop,
            surface,
        }
    }

    fn required_extensions() -> InstanceExtensions {
        let mut extensions = vulkano_win::required_extensions(); // already has surface caps 2
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

    fn setup_debug_callback(instance: &Arc<Instance>) -> Option<DebugUtilsMessenger> {
        if !ENABLE_VALIDATION_LAYERS {
            return None;
        }

        let message_severity = DebugUtilsMessageSeverity {
            error: true,
            warning: true,
            information: true,
            verbose: true,
        };

        let callback = unsafe {
            DebugUtilsMessenger::new(
                instance.clone(),
                DebugUtilsMessengerCreateInfo {
                    message_severity,
                    message_type: DebugUtilsMessageType::all(),
                    ..DebugUtilsMessengerCreateInfo::user_callback(Arc::new(|msg| {
                        println!("{}:{}", msg.layer_prefix.unwrap(), msg.description);
                    }))
                },
            )
            .expect("Couldn't create Debug Utils Messenger")
        };

        // let callback = DebugCallback::new(instance, message_severity, message_type, |msg| {
        //     println!("{:?}", msg.description);
        // })
        // .expect("Couldn't create DebugCallback");
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
                enabled_features: device::Features::none(),
                queue_create_infos,
                ..Default::default()
            },
        )
        .expect("Couldn't create device");

        let graphics_queue = queues.next().unwrap();
        let present_queue = queues.next().unwrap_or_else(|| graphics_queue.clone());

        (device, graphics_queue, present_queue)
    }

    fn choose_swap_surface_format(
        available_formats: &Vec<(vulkano::format::Format, ColorSpace)>,
    ) -> (Format, ColorSpace) {
        available_formats
            .into_iter()
            .find(|&&format| {
                format.0 == Format::B8G8R8A8_SRGB && format.1 == ColorSpace::SrgbNonLinear
            })
            .unwrap_or(&available_formats[0])
            .to_owned()
    }

    fn choose_swap_present_modes(available_modes: &Vec<PresentMode>) -> PresentMode {
        available_modes
            .into_iter()
            .find(|&&mode| mode == PresentMode::Mailbox)
            .unwrap_or(&PresentMode::Fifo)
            .to_owned()
    }

    fn choose_swap_extent(
        capabilities: &SurfaceCapabilities,
        surface: &Arc<Surface<Window>>,
    ) -> [u32; 2] {
        let PhysicalSize { width, height } = surface.window().inner_size();
        let width = clamp(
            width,
            capabilities.min_image_extent[0],
            capabilities.max_image_extent[0],
        );
        let height = clamp(
            height,
            capabilities.min_image_extent[1],
            capabilities.max_image_extent[1],
        );
        [width, height]
    }

    fn create_swap_chain(
        physical_device_index: usize,
        logical_device: &Arc<Device>,
        instance: &Arc<Instance>,
        surface: &Arc<Surface<Window>>,
    ) -> (
        Arc<Swapchain<Window>>,
        Vec<Arc<SwapchainImage<Window>>>,
        Format,
        [u32; 2],
    ) {
        let (capabilities, formats, present_modes) =
            Self::query_swap_chain_support(physical_device_index, instance, surface);
        let (image_format, image_color_space) = Self::choose_swap_surface_format(&formats);
        let present_mode = Self::choose_swap_present_modes(&present_modes);
        let image_extent = Self::choose_swap_extent(&capabilities, surface);

        let min_image_count = capabilities.min_image_count + 1;
        let min_image_count = if capabilities.max_image_count.is_some()
            && min_image_count > capabilities.max_image_count.unwrap()
        {
            capabilities.max_image_count.unwrap()
        } else {
            min_image_count
        };

        let image_usage = ImageUsage {
            color_attachment: true,
            ..ImageUsage::none()
        };
        let pre_transform = capabilities.current_transform;

        let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();

        let physical_device = PhysicalDevice::from_index(instance, physical_device_index).unwrap();
        let queue_family_ids = Self::find_queue_family_ids(&physical_device, surface);

        let image_sharing = if queue_family_ids.graphics_family_id.unwrap()
            == queue_family_ids.presentation_family_id.unwrap()
        {
            Sharing::Exclusive
        } else {
            Sharing::Concurrent(
                [
                    queue_family_ids.graphics_family_id.unwrap(),
                    queue_family_ids.presentation_family_id.unwrap(),
                ][..]
                    .into(),
            )
        };

        // Create the swapchain and its images.
        let (swapchain, images) = Swapchain::new(
            logical_device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count,
                image_format: Some(image_format),
                image_color_space,
                image_extent,
                image_usage,
                pre_transform,
                composite_alpha,
                present_mode,
                image_sharing,
                ..Default::default()
            },
        )
        .expect("Couldn't create Swapchain");

        (swapchain, images, image_format, image_extent)
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
