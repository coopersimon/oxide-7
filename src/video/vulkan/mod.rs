// Vulkan renderer and data caches.

mod mem;
mod shaders;

use vulkano::{
    instance::{
        Instance, PhysicalDevice
    },
    device::{
        Device, DeviceExtensions, Queue
    },
    framebuffer::{
        Framebuffer, Subpass, FramebufferAbstract, RenderPassAbstract
    },
    pipeline::{
        GraphicsPipeline,
        viewport::Viewport,
        vertex::SingleBufferDefinition
    },
    command_buffer::{
        AutoCommandBufferBuilder,
        AutoCommandBuffer,
        DynamicState
    },
    sampler::{
        Filter,
        MipmapMode,
        Sampler,
        SamplerAddressMode
    },
    swapchain::{
        Swapchain, Surface, SurfaceTransform, PresentMode, acquire_next_image
    },
    sync::{
        now, GpuFuture
    },
    descriptor::{
        descriptor_set::{
            PersistentDescriptorSetBuf,
            PersistentDescriptorSetImg,
            PersistentDescriptorSetSampler,
            FixedSizeDescriptorSet,
            FixedSizeDescriptorSetsPool
        },
        pipeline_layout::PipelineLayoutAbstract
    }
};

use vulkano_win::VkSurfaceBuild;

use winit::{
    EventsLoop,
    Window,
    WindowBuilder
};

use bitflags::bitflags;

use std::sync::Arc;

use super::{
    VRamRef,
    render::Renderable
};

use mem::MemoryCache;

// Types
type RenderPipeline = GraphicsPipeline<
    SingleBufferDefinition<Vertex>,
    Box<dyn PipelineLayoutAbstract + Send + Sync>,
    Arc<dyn RenderPassAbstract + Send + Sync>
>;

// Individual Vertex.
#[derive(Default, Copy, Clone)]
struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

vulkano::impl_vertex!(Vertex, position, data);

// Data for a single render
struct RenderData {
    command_buffer: Option<AutoCommandBufferBuilder>,
    acquire_future: Box<dyn GpuFuture>,
    image_num:      usize,
    image_future:   Box<dyn GpuFuture>,
    pipeline:       Arc<RenderPipeline>,
    //set0:           Arc<FixedSizeDescriptorSet<Arc<RenderPipeline>, (((), PersistentDescriptorSetImg<super::mem::TileImage>), PersistentDescriptorSetSampler)>>,
    //set1:           Arc<FixedSizeDescriptorSet<Arc<RenderPipeline>, ((), PersistentDescriptorSetBuf<super::mem::PaletteBuffer>)>>
}

pub struct Renderer {
    // Memory
    mem:            MemoryCache,
    // Core
    device:         Arc<Device>,
    queue:          Arc<Queue>,
    pipeline:       Arc<RenderPipeline>,
    render_pass:    Arc<dyn RenderPassAbstract + Send + Sync>,
    surface:        Arc<Surface<Window>>,
    // Uniforms
    sampler:        Arc<Sampler>,
    set_pools:      Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
    // Swapchain and frames
    swapchain:      Arc<Swapchain<Window>>,
    framebuffers:   Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    dynamic_state:  DynamicState,
    // Frame data
    previous_frame_future: Box<dyn GpuFuture>,
    render_data: Option<RenderData>
}

impl Renderer {
    // Create and initialise renderer.
    pub fn new(video_mem: VRamRef, events_loop: &EventsLoop) -> Self {
        // Make instance with window extensions.
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("Failed to create vulkan instance")
        };

        // Get graphics device.
        let physical = PhysicalDevice::enumerate(&instance).next()
            .expect("No device available");

        // Get graphics command queue family from graphics device.
        let queue_family = physical.queue_families()
            .find(|&q| q.supports_graphics())
            .expect("Could not find a graphical queue family");

        // Make software device and queue iterator of the graphics family.
        let (device, mut queues) = {
            let device_ext = DeviceExtensions{
                khr_swapchain: true,
                .. DeviceExtensions::none()
            };
            
            Device::new(physical, physical.supported_features(), &device_ext,
                        [(queue_family, 0.5)].iter().cloned())
                .expect("Failed to create device")
        };

        // Get a queue from the iterator.
        let queue = queues.next().unwrap();

        // Make a surface.
        let surface = WindowBuilder::new()
            .with_dimensions((512, 448).into())
            .with_title("Oxide-7")
            .build_vk_surface(&events_loop, instance.clone())
            .expect("Couldn't create surface");

        // Make the sampler for the texture.
        let sampler = Sampler::new(
            device.clone(),
            Filter::Nearest,
            Filter::Nearest,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0, 1.0, 0.0, 0.0
        ).expect("Couldn't create sampler!");

        // Get a swapchain and images for use with the swapchain, as well as the dynamic state.
        let ((swapchain, images), dynamic_state) = {

            let caps = surface.capabilities(physical)
                    .expect("Failed to get surface capabilities");
            let dimensions = caps.current_extent.unwrap_or([160, 144]);

            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;

            (Swapchain::new(device.clone(), surface.clone(),
                caps.min_image_count, format, dimensions, 1, caps.supported_usage_flags, &queue,
                SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None
            ).expect("Failed to create swapchain"),
            DynamicState {
                viewports: Some(vec![Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0 .. 1.0,
                }]),
                .. DynamicState::none()
            })
        };

        // Make the render pass to insert into the command queue.
        let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),//Format::R8G8B8A8Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()) as Arc<dyn RenderPassAbstract + Send + Sync>;

        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        // Assemble
        let vs = shaders::vs::Shader::load(device.clone()).expect("failed to create vertex shader");
        let fs = shaders::fs::Shader::load(device.clone()).expect("failed to create fragment shader");

        // Make pipeline.
        let pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        // Make descriptor set pools.
        let set_pools = vec![
            FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(pipeline.clone(), 1)
        ];

        Renderer {
            mem:            MemoryCache::new(video_mem),

            device:         device.clone(),
            queue:          queue,
            pipeline:       pipeline,
            render_pass:    render_pass,
            surface:        surface,

            sampler:        sampler,
            set_pools:      set_pools,

            swapchain:      swapchain,
            framebuffers:   framebuffers,
            dynamic_state:  dynamic_state,

            previous_frame_future: Box::new(now(device.clone())),
            render_data: None
        }
    }

    // Re-create the swapchain and framebuffers.
    pub fn create_swapchain(&mut self) {
        let window = self.surface.window();
        let dimensions = if let Some(dimensions) = window.get_inner_size() {
            let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            return;
        };

        // Get a swapchain and images for use with the swapchain.
        let (new_swapchain, images) = self.swapchain.recreate_with_dimension(dimensions).unwrap();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0 .. 1.0,
        };

        self.dynamic_state.viewports = Some(vec![viewport]);

        self.framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(self.render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        self.swapchain = new_swapchain;
    }
}

impl Renderable for Renderer {
    fn frame_start(&mut self) {
        // Get current framebuffer index from the swapchain.
        let (image_num, acquire_future) = acquire_next_image(self.swapchain.clone(), None)
            .expect("Didn't get next image");

        // Make image with current texture.
        let (image, write_future) = video_mem.get_tile_atlas(&self.device, &self.queue);

        // Make descriptor set to bind texture atlas.
        let set0 = Arc::new(self.set_pools[0].next()
            .add_sampled_image(image, self.sampler.clone()).unwrap()
            .build().unwrap());

        // Make descriptor set for palette.
        let set1 = Arc::new(self.set_pools[1].next()
            .add_buffer(video_mem.get_palette_buffer().clone()).unwrap()
            .build().unwrap());
        
        // Start building command buffer using pipeline and framebuffer, starting with the background vertices.
        let command_buffer_builder = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family()).unwrap()
            .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into()]).unwrap();

        self.render_data = Some(RenderData{
            command_buffer: Some(command_buffer_builder),
            acquire_future: Box::new(acquire_future),
            image_num:      image_num,
            image_future:   write_future,
            pipeline:       self.pipeline.clone(),
            //set0:           set0,
            //set1:           set1
        });
    }

    fn draw_line(&mut self) {

    }

    fn frame_end(&mut self) {

    }
}