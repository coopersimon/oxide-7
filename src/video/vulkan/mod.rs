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
            FixedSizeDescriptorSetsPool
        },
        pipeline_layout::PipelineLayoutAbstract
    },
    memory::pool::StdMemoryPool,
    buffer::cpu_pool::CpuBufferPoolChunk,
    buffer::ImmutableBuffer,
};

use vulkano_win::VkSurfaceBuild;

use winit::{
    Window,
    WindowBuilder,
    EventsLoop
};

use std::sync::Arc;

use super::{
    VRamRef,
    render::{
        Renderable,
        VideoMode
    }
};

use mem::MemoryCache;

// TODO: move these types elsewhere
// Types
type RenderPipeline = GraphicsPipeline<
    SingleBufferDefinition<Vertex>,
    Box<dyn PipelineLayoutAbstract + Send + Sync>,
    Arc<dyn RenderPassAbstract + Send + Sync>
>;

// TODO: move this and other data elsewhere
#[derive(Clone, Copy)]
enum Side {
    Left =  0 << 16,
    Right = 1 << 16
}

// Individual Vertex.
#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

vulkano::impl_vertex!(Vertex, position, data);

// Push constants used in BG shaders.
#[derive(Copy, Clone, Debug)]
pub struct BGPushConstants {
    pub tex_size:       [f32; 2],   // Size of individual tile texture in map.
    pub atlas_size:     [f32; 2],   // Size of texture atlas in tiles.
    pub tile_size:      [f32; 2],   // Width of tile relative to the viewport, height of line relative to the viewport.
    pub map_size:       [f32; 2],   // Size of tile map relative to the viewport.
    pub vertex_offset:  [f32; 2],   // Offset to apply to vertices for scrolling.   // TODO: use a different shader for sprites.
    pub palette_offset: u32,        // Offset for palette used by BG (in colours).
    pub palette_size:   u32,        // Size of palettes used.
    pub tex_pixel_height: f32       // Height of individual tile in pixels.
}

// Push constants used in Sprite shaders.
#[derive(Copy, Clone, Debug)]
pub struct ObjPushConstants {
    pub small_tex_size: [f32; 2],
    pub large_tex_size: [f32; 2],
}

type VertexBuffer = CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>;

// Data for a single render
struct RenderData {
    command_buffer: Option<AutoCommandBufferBuilder>,
    acquire_future: Box<dyn GpuFuture>,
    image_num:      usize,
    image_futures:  Vec<Box<dyn GpuFuture>>,

    bg_pipeline:    Arc<RenderPipeline>,
    obj_pipeline:   Arc<RenderPipeline>,
    //set0:           Arc<FixedSizeDescriptorSet<Arc<RenderPipeline>, (((), PersistentDescriptorSetImg<mem::patternmem::PatternImage>), PersistentDescriptorSetSampler)>>,
    //set1:           Arc<FixedSizeDescriptorSet<Arc<RenderPipeline>, ((), PersistentDescriptorSetBuf<mem::palette::PaletteBuffer>)>>
    bg_set_pool_0:  FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>,
    bg_set_pool_1:  FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>,
    obj_set_pool_0: FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>,
    obj_set_pool_1: FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>,
    
    debug_buffer:   Arc<ImmutableBuffer<[Vertex]>>,
}

pub struct Renderer {
    // Memory
    mem:            MemoryCache,
    // Core
    device:         Arc<Device>,
    queue:          Arc<Queue>,
    bg_pipeline:    Arc<RenderPipeline>,    // Pipeline used for rendering backgrounds.
    obj_pipeline:   Arc<RenderPipeline>,    // Pipeline used for rendering sprites.
    render_pass:    Arc<dyn RenderPassAbstract + Send + Sync>,
    surface:        Arc<Surface<Window>>,
    // Uniforms
    sampler:        Arc<Sampler>,
    bg_set_pools:   Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
    obj_set_pools:  Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
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
    pub fn new(video_mem: VRamRef, events_loop: &EventsLoop/*, instance: Arc<Instance>, surface: Arc<Surface<Window>>*/) -> Self {
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

        let surface = WindowBuilder::new()
            .with_dimensions((512, 448).into())
            .with_title("Oxide-7")
            .build_vk_surface(&events_loop, instance.clone())
            .expect("Couldn't create surface");

        // Get a swapchain and images for use with the swapchain, as well as the dynamic state.
        let ((swapchain, images), dynamic_state) = {

            let caps = surface.capabilities(physical)
                    .expect("Failed to get surface capabilities");
            let dimensions = caps.current_extent.unwrap_or([512, 448]);

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

        // Assemble BG shaders.
        let bg_vs = shaders::bg_vs::Shader::load(device.clone()).expect("failed to create bg vertex shader");
        let bg_fs = shaders::bg_fs::Shader::load(device.clone()).expect("failed to create bg fragment shader");

        // Make pipeline.
        let bg_pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(bg_vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(bg_fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        // Make descriptor set pools.
        let bg_set_pools = vec![
            FixedSizeDescriptorSetsPool::new(bg_pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(bg_pipeline.clone(), 1)
        ];

        // Assemble sprite shaders.
        let obj_vs = shaders::obj_vs::Shader::load(device.clone()).expect("failed to create obj vertex shader");
        let obj_fs = shaders::obj_fs::Shader::load(device.clone()).expect("failed to create obj fragment shader");

        // Make pipeline.
        let obj_pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(obj_vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(obj_fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        // Make descriptor set pools.
        let obj_set_pools = vec![
            FixedSizeDescriptorSetsPool::new(obj_pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(obj_pipeline.clone(), 1)
        ];

        Renderer {
            mem:            MemoryCache::new(video_mem, &device, &queue),

            device:         device.clone(),
            queue:          queue,
            bg_pipeline:    bg_pipeline,
            obj_pipeline:   obj_pipeline,
            render_pass:    render_pass,
            surface:        surface,

            sampler:        sampler,
            bg_set_pools:   bg_set_pools,
            obj_set_pools:  obj_set_pools,

            swapchain:      swapchain,
            framebuffers:   framebuffers,
            dynamic_state:  dynamic_state,

            previous_frame_future: Box::new(now(device)),
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
        
        // Start building command buffer using framebuffer.
        let command_buffer_builder = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family()).unwrap()
            .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into()]).unwrap();

        let (debug_buffer, debug_future) = ImmutableBuffer::from_iter(
            vec![
                Vertex{ position: [-1.0, -1.0], data: 0 },
                Vertex{ position: [1.0, -1.0], data: 1 },
                Vertex{ position: [-1.0, 1.0], data: 2 },
                Vertex{ position: [1.0, -1.0], data: 1 },
                Vertex{ position: [-1.0, 1.0], data: 2 },
                Vertex{ position: [1.0, 1.0], data: 3 },
            ].iter().cloned(),
            vulkano::buffer::BufferUsage::vertex_buffer(),
            self.queue.clone()
        ).unwrap();

        // Assemble
        let vs = shaders::debug_vs::Shader::load(self.device.clone()).expect("failed to create vertex shader");
        let fs = shaders::debug_fs::Shader::load(self.device.clone()).expect("failed to create fragment shader");

        // Make pipeline.
        let debug_pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(self.render_pass.clone(), 0).unwrap())
            .build(self.device.clone())
            .unwrap()
        );

        // Make descriptor set pools.
        let set_pools = vec![
            FixedSizeDescriptorSetsPool::new(debug_pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(debug_pipeline.clone(), 1)
        ];

        self.render_data = Some(RenderData{
            command_buffer: Some(command_buffer_builder),
            acquire_future: Box::new(acquire_future),
            image_num:      image_num,
            image_futures:  Vec::new(),

            bg_pipeline:    self.bg_pipeline.clone(),
            obj_pipeline:   self.obj_pipeline.clone(),

            bg_set_pool_0:  self.bg_set_pools[0].clone(),
            bg_set_pool_1:  self.bg_set_pools[1].clone(),
            obj_set_pool_0: self.obj_set_pools[0].clone(),
            obj_set_pool_1: self.obj_set_pools[1].clone(),

            // Uncomment the below for debug:
            //image_futures:  vec![Box::new(debug_future) as Box<_>],
            //bg_pipeline:    debug_pipeline,
            //bg_set_pool_0:  set_pools[0].clone(),
            //bg_set_pool_1:  set_pools[1].clone(),
            debug_buffer:   debug_buffer
        });
    }

    fn draw_line(&mut self, y: u16) {
        if let Some(render_data) = &mut self.render_data {
            if !self.mem.in_fblank() {
                self.mem.init();

                match self.mem.get_mode() {
                    VideoMode::_0 => render_data.draw_mode_0(&mut self.mem, &self.sampler, &self.dynamic_state, y),
                    VideoMode::_1 => render_data.draw_mode_1(&mut self.mem, &self.sampler, &self.dynamic_state, y),
                    VideoMode::_2 => {},
                    VideoMode::_3 => {},
                    VideoMode::_4 => {},
                    VideoMode::_5 => {},
                    VideoMode::_6 => {},
                    VideoMode::_7 => {},
                }
                //render_data.draw_pattern_mem(&mut self.mem, &self.sampler, &self.dynamic_state, y, 1);
            }
        }
    }

    fn frame_end(&mut self) {
        let render_data = std::mem::replace(&mut self.render_data, None);

        if let Some(render_data) = render_data {
            // Finish command buffer.
            let (command_buffer, acquire_future, mut image_futures, image_num) = render_data.finish_drawing();

            // Wait until previous frame is done.
            let mut now_future = Box::new(now(self.device.clone())) as Box<dyn GpuFuture>;
            std::mem::swap(&mut self.previous_frame_future, &mut now_future);

            // Wait until previous frame is done,
            // _and_ the framebuffer has been acquired,
            // _and_ the textures have been uploaded.
            let init_future = Box::new(now_future.join(acquire_future)) as Box<dyn GpuFuture>;
            let future = image_futures.drain(..).fold(init_future, |all, f| Box::new(all.join(f)) as Box<dyn GpuFuture>)
                .then_execute(self.queue.clone(), command_buffer).unwrap()                      // Run the commands (pipeline and render)
                .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)  // Present newly rendered image.
                .then_signal_fence_and_flush();                                                 // Signal done and flush the pipeline.

            match future {
                Ok(future) => self.previous_frame_future = Box::new(future) as Box<_>,
                Err(e) => println!("Err: {:?}", e),
            }

            self.previous_frame_future.cleanup_finished();
        }
    }
}

// Internal
impl RenderData {
    fn draw_mode_0(
        &mut self,
        mem:            &mut MemoryCache,
        sampler:        &Arc<Sampler>,
        dynamic_state:  &DynamicState,
        y:              u16
        ) {

        let mut command_buffer = std::mem::replace(&mut self.command_buffer, None).unwrap();

        // Make descriptor sets for palettes.
        let bg_set1 = Arc::new(self.bg_set_pool_1.next()
            .add_buffer(mem.get_bg_palette_buffer()).unwrap()
            .build().unwrap());

        let obj_set1 = Arc::new(self.obj_set_pool_1.next()
            .add_buffer(mem.get_obj_palette_buffer()).unwrap()
            .build().unwrap());

        // Make descriptor set to bind texture atlases for patterns.
        let bg_4_set0 = {
            let (image, write_future) = mem.get_bg_image(3);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let bg_3_set0 = {
            let (image, write_future) = mem.get_bg_image(2);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let bg_2_set0 = {
            let (image, write_future) = mem.get_bg_image(1);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let bg_1_set0 = {
            let (image, write_future) = mem.get_bg_image(0);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let obj_0_set0 = {
            let (image, write_future) = mem.get_sprite_image_0();
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.obj_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let obj_n_set0 = {
            let (image, write_future) = mem.get_sprite_image_n();
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.obj_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        // Push constants
        let bg_4_push_constants = mem.get_bg_push_constants(3);
        let bg_3_push_constants = mem.get_bg_push_constants(2);
        let bg_2_push_constants = mem.get_bg_push_constants(1);
        let bg_1_push_constants = mem.get_bg_push_constants(0);
        let obj_push_constants = mem.get_obj_push_constants();

        // Draw
        let bg_4_y = mem.calc_y_line(3, y);
        if let Some(bg_4_vertices) = mem.get_bg_lo_vertices(3, bg_4_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_4_vertices,
                (bg_4_set0.clone(), bg_set1.clone()),
                bg_4_push_constants.clone()
            ).unwrap();
        }

        let bg_3_y = mem.calc_y_line(2, y);
        if let Some(bg_3_vertices) = mem.get_bg_lo_vertices(2, bg_3_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_3_vertices,
                (bg_3_set0.clone(), bg_set1.clone()),
                bg_3_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_0) = mem.get_sprite_vertices_0(0, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_0,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_0) = mem.get_sprite_vertices_n(0, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_0,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(bg_4_vertices) = mem.get_bg_hi_vertices(3, bg_4_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_4_vertices,
                (bg_4_set0, bg_set1.clone()),
                bg_4_push_constants
            ).unwrap();
        }

        if let Some(bg_3_vertices) = mem.get_bg_hi_vertices(2, bg_3_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_3_vertices,
                (bg_3_set0, bg_set1.clone()),
                bg_3_push_constants
            ).unwrap();
        }

        if let Some(sprites_1) = mem.get_sprite_vertices_0(1, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_1,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_1) = mem.get_sprite_vertices_n(1, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_1,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        let bg_2_y = mem.calc_y_line(1, y);
        if let Some(bg_2_vertices) = mem.get_bg_lo_vertices(1, bg_2_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_2_vertices,
                (bg_2_set0.clone(), bg_set1.clone()),
                bg_2_push_constants.clone()
            ).unwrap();
        }

        let bg_1_y = mem.calc_y_line(0, y);
        if let Some(bg_1_vertices) = mem.get_bg_lo_vertices(0, bg_1_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_1_vertices,
                (bg_1_set0.clone(), bg_set1.clone()),
                bg_1_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_2) = mem.get_sprite_vertices_0(2, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_2,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_2) = mem.get_sprite_vertices_n(2, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_2,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(bg_2_vertices) = mem.get_bg_hi_vertices(1, bg_2_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_2_vertices,
                (bg_2_set0, bg_set1.clone()),
                bg_2_push_constants
            ).unwrap();
        }

        if let Some(bg_1_vertices) = mem.get_bg_hi_vertices(0, bg_1_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_1_vertices,
                (bg_1_set0, bg_set1.clone()),
                bg_1_push_constants
            ).unwrap();
        }

        if let Some(sprites_3) = mem.get_sprite_vertices_0(3, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_3,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_3) = mem.get_sprite_vertices_n(3, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_3,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        self.command_buffer = Some(command_buffer);
    }

    fn draw_mode_1(
        &mut self,
        mem:            &mut MemoryCache,
        sampler:        &Arc<Sampler>,
        dynamic_state:  &DynamicState,
        y:              u16
        ) {

        let mut command_buffer = std::mem::replace(&mut self.command_buffer, None).unwrap();

        // Make descriptor set for palettes.
        let bg_set1 = Arc::new(self.bg_set_pool_1.next()
            .add_buffer(mem.get_bg_palette_buffer()).unwrap()
            .build().unwrap());

        let obj_set1 = Arc::new(self.obj_set_pool_1.next()
            .add_buffer(mem.get_obj_palette_buffer()).unwrap()
            .build().unwrap());

        // Make descriptor set to bind texture atlases for patterns.
        let bg_3_set0 = {
            let (image, write_future) = mem.get_bg_image(2);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let bg_2_set0 = {
            let (image, write_future) = mem.get_bg_image(1);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let bg_1_set0 = {
            let (image, write_future) = mem.get_bg_image(0);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let obj_0_set0 = {
            let (image, write_future) = mem.get_sprite_image_0();
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.obj_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        let obj_n_set0 = {
            let (image, write_future) = mem.get_sprite_image_n();
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.obj_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        // Push constants
        let bg_3_push_constants = mem.get_bg_push_constants(2);
        let bg_2_push_constants = mem.get_bg_push_constants(1);
        let bg_1_push_constants = mem.get_bg_push_constants(0);
        let obj_push_constants = mem.get_obj_push_constants();

        // Draw
        let bg_3_y = mem.calc_y_line(2, y);
        if let Some(bg_3_vertices) = mem.get_bg_lo_vertices(2, bg_3_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_3_vertices,
                (bg_3_set0.clone(), bg_set1.clone()),
                bg_3_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_0) = mem.get_sprite_vertices_0(0, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_0,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_0) = mem.get_sprite_vertices_n(0, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_0,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if !mem.get_bg3_priority() {
            if let Some(bg_3_vertices) = mem.get_bg_hi_vertices(2, bg_3_y) {
                command_buffer = command_buffer.draw(
                    self.bg_pipeline.clone(),
                    dynamic_state,
                    bg_3_vertices,
                    (bg_3_set0.clone(), bg_set1.clone()),
                    bg_3_push_constants.clone()
                ).unwrap();
            }
        }

        if let Some(sprites_1) = mem.get_sprite_vertices_0(1, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_1,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_1) = mem.get_sprite_vertices_n(1, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_1,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        let bg_2_y = mem.calc_y_line(1, y);
        if let Some(bg_2_vertices) = mem.get_bg_lo_vertices(1, bg_2_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_2_vertices,
                (bg_2_set0.clone(), bg_set1.clone()),
                bg_2_push_constants.clone()
            ).unwrap();
        }

        let bg_1_y = mem.calc_y_line(0, y);
        if let Some(bg_1_vertices) = mem.get_bg_lo_vertices(0, bg_1_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_1_vertices,
                (bg_1_set0.clone(), bg_set1.clone()),
                bg_1_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_2) = mem.get_sprite_vertices_0(2, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_2,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_2) = mem.get_sprite_vertices_n(2, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_2,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(bg_2_vertices) = mem.get_bg_hi_vertices(1, bg_2_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_2_vertices,
                (bg_2_set0, bg_set1.clone()),
                bg_2_push_constants
            ).unwrap();
        }

        if let Some(bg_1_vertices) = mem.get_bg_hi_vertices(0, bg_1_y) {
            command_buffer = command_buffer.draw(
                self.bg_pipeline.clone(),
                dynamic_state,
                bg_1_vertices,
                (bg_1_set0, bg_set1.clone()),
                bg_1_push_constants
            ).unwrap();
        }

        if let Some(sprites_3) = mem.get_sprite_vertices_0(3, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_3,
                (obj_0_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if let Some(sprites_3) = mem.get_sprite_vertices_n(3, y) {
            command_buffer = command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites_3,
                (obj_n_set0.clone(), obj_set1.clone()),
                obj_push_constants.clone()
            ).unwrap();
        }

        if mem.get_bg3_priority() {
            if let Some(bg_3_vertices) = mem.get_bg_hi_vertices(2, bg_3_y) {
                command_buffer = command_buffer.draw(
                    self.bg_pipeline.clone(),
                    dynamic_state,
                    bg_3_vertices,
                    (bg_3_set0, bg_set1.clone()),
                    bg_3_push_constants
                ).unwrap();
            }
        }

        self.command_buffer = Some(command_buffer);
    }

    fn finish_drawing(self) -> (AutoCommandBuffer, Box<dyn GpuFuture>, Vec<Box<dyn GpuFuture>>, usize) {
        (
            self.command_buffer.unwrap().end_render_pass().unwrap().build().unwrap(),
            self.acquire_future,
            self.image_futures,
            self.image_num
        )
    }
}

// Debug
impl RenderData {
    fn draw_pattern_mem(
        &mut self,
        mem:            &mut MemoryCache,
        sampler:        &Arc<Sampler>,
        dynamic_state:  &DynamicState,
        y:              u16,
        bg_num:         usize
        ) {

        let mut command_buffer = std::mem::replace(&mut self.command_buffer, None).unwrap();

        // Make descriptor set for palettes.
        let set1 = Arc::new(self.bg_set_pool_1.next()
            .add_buffer(mem.get_bg_palette_buffer()).unwrap()
            .build().unwrap());

        // Make descriptor set to bind texture atlases for patterns.

        let bg_set0 = {
            let (image, write_future) = mem.get_sprite_image_0();   // Change pattern here to see all tiles.
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }

            // Make descriptor set to bind texture atlas.
            Arc::new(self.bg_set_pool_0.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap())
        };

        // Draw
        command_buffer = command_buffer.draw(
            self.bg_pipeline.clone(),
            dynamic_state,
            self.debug_buffer.clone(),
            (bg_set0, set1.clone()),
            ()
        ).unwrap();

        self.command_buffer = Some(command_buffer);
    }
}