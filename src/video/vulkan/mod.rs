// Vulkan renderer and data caches.

mod mem;
mod shaders;
mod uniforms;
mod types;

use vulkano::{
    instance::{
        Instance, PhysicalDevice
    },
    device::{
        Device, DeviceExtensions, Queue
    },
    format::Format::D16Unorm,   // TODO: is this the best format?
    framebuffer::{
        Framebuffer, Subpass, FramebufferAbstract, RenderPassAbstract
    },
    image::attachment::AttachmentImage,
    pipeline::{
        GraphicsPipeline,
        viewport::Viewport,
    },
    command_buffer::{
        AutoCommandBufferBuilder,
        AutoCommandBuffer,
        DynamicState
    },
    swapchain::{
        Swapchain, Surface, SurfaceTransform, PresentMode, acquire_next_image
    },
    sync::{
        now, GpuFuture
    },
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
use types::*;

// Data for a single render
struct RenderData {
    command_buffer: Option<AutoCommandBufferBuilder>,

    bg_cb:          Option<AutoCommandBufferBuilder>,
    obj_cb:         Option<AutoCommandBufferBuilder>,

    acquire_future: Box<dyn GpuFuture>,
    image_num:      usize,
    image_futures:  Vec<Box<dyn GpuFuture>>,

    bg_pipeline:    Arc<RenderPipeline>,
    obj_pipeline:   Arc<RenderPipeline>,
    
    //debug_buffer:   Arc<ImmutableBuffer<[Vertex]>>,
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
    // Swapchain and frames
    swapchain:      Arc<Swapchain<Window>>,
    framebuffers:   Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    dynamic_state:  DynamicState,
    // Frame data
    previous_frame_future: Box<dyn GpuFuture>,
    render_data: Option<RenderData>,
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

        let surface = WindowBuilder::new()
            .with_dimensions((512, 448).into())
            .with_title("Oxide-7")
            .build_vk_surface(&events_loop, instance.clone())
            .expect("Couldn't create surface");

        // Get a swapchain and images for use with the swapchain, as well as the dynamic state.
        let ((swapchain, images), dynamic_state, depth_buffer) = {

            let caps = surface.capabilities(physical)
                    .expect("Failed to get surface capabilities");
            let dimensions = caps.current_extent.unwrap_or([512, 448]);

            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;

            (Swapchain::new(device.clone(), surface.clone(),
                3, format, dimensions, 1, caps.supported_usage_flags, &queue,
                SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None
            ).expect("Failed to create swapchain"),
            DynamicState {
                viewports: Some(vec![Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0 .. 1.0,
                }]),
                .. DynamicState::none()
            },
            AttachmentImage::transient(
                device.clone(),
                dimensions,
                D16Unorm
            ).unwrap())
        };

        // Make the render pass to insert into the command queue.
        let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: D16Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        ).unwrap()) as Arc<dyn RenderPassAbstract + Send + Sync>;

        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .add(depth_buffer.clone()).unwrap()
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
            .depth_stencil_simple_depth()
            //.blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        // Assemble sprite shaders.
        let obj_vs = shaders::obj_vs::Shader::load(device.clone()).expect("failed to create obj vertex shader");
        let obj_fs = shaders::obj_fs::Shader::load(device.clone()).expect("failed to create obj fragment shader");

        // Make pipeline.
        let obj_pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(obj_vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(obj_fs.main_entry_point(), ())
            //.blend_alpha_blending()
            .depth_stencil_simple_depth()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        let uniform_cache = uniforms::UniformCache::new(&device, &bg_pipeline, &obj_pipeline);

        Renderer {
            mem:            MemoryCache::new(video_mem, &device, &queue, uniform_cache),

            device:         device.clone(),
            queue:          queue,
            bg_pipeline:    bg_pipeline,
            obj_pipeline:   obj_pipeline,
            render_pass:    render_pass,
            surface:        surface,

            swapchain:      swapchain,
            framebuffers:   framebuffers,
            dynamic_state:  dynamic_state,

            previous_frame_future:  Box::new(now(device)),
            render_data:            None,
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

        let depth_buffer = AttachmentImage::transient(self.device.clone(), dimensions, D16Unorm).unwrap();

        self.dynamic_state.viewports = Some(vec![viewport]);

        self.framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(self.render_pass.clone())
                    .add(image.clone()).unwrap()
                    .add(depth_buffer.clone()).unwrap()
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
            .begin_render_pass(self.framebuffers[image_num].clone(), true, vec![[0.0, 0.0, 0.0, 1.0].into(), 1.0.into()]).unwrap();

        let bg_command_buf = AutoCommandBufferBuilder::secondary_graphics(self.device.clone(), self.queue.family(), Subpass::from(self.render_pass.clone(), 0).unwrap()).unwrap();
        let obj_command_buf = AutoCommandBufferBuilder::secondary_graphics(self.device.clone(), self.queue.family(), Subpass::from(self.render_pass.clone(), 0).unwrap()).unwrap();

        /*let (debug_buffer, debug_future) = ImmutableBuffer::from_iter(
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
        ];*/

        self.render_data = Some(RenderData{
            command_buffer: Some(command_buffer_builder),
            bg_cb:          Some(bg_command_buf),
            obj_cb:         Some(obj_command_buf),

            acquire_future: Box::new(acquire_future),
            image_num:      image_num,
            image_futures:  Vec::new(),

            bg_pipeline:    self.bg_pipeline.clone(),
            obj_pipeline:   self.obj_pipeline.clone(),

            // Uncomment the below for debug:
            //image_futures:  vec![Box::new(debug_future) as Box<_>],
            //bg_pipeline:    debug_pipeline,
            //debug_buffer:   debug_buffer
        });
    }

    fn draw_line(&mut self, y: u16) {
        if let Some(render_data) = &mut self.render_data {
            if !self.mem.in_fblank() {
                self.mem.init();

                match self.mem.get_mode() {
                    VideoMode::_0 => render_data.draw_mode_0(&mut self.mem, &self.dynamic_state, y),
                    VideoMode::_1 => render_data.draw_mode_1(&mut self.mem, &self.dynamic_state, y),
                    VideoMode::_2 => panic!("Mode 2 not supported."),
                    VideoMode::_3 => panic!("Mode 3 not supported."),
                    VideoMode::_4 => panic!("Mode 4 not supported."),
                    VideoMode::_5 => panic!("Mode 5 not supported."),
                    VideoMode::_6 => panic!("Mode 6 not supported."),
                    VideoMode::_7 => panic!("Mode 7 not supported."),
                }
                //render_data.draw_pattern_mem(&mut self.mem, &self.sampler, &self.dynamic_state, y, 1);
            }
        }
    }

    // TODO: move this into frame start.
    fn frame_end(&mut self) {
        let render_data = std::mem::replace(&mut self.render_data, None);

        if let Some(render_data) = render_data {
            // Finish command buffer.
            let (command_buffer, acquire_future, mut image_futures, image_num) = render_data.finish_drawing();

            // Cleanup old frame.
            self.previous_frame_future.cleanup_finished();

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
                Ok(future) => {
                    //future.wait(None).unwrap();
                    self.previous_frame_future = Box::new(future) as Box<_>
                },
                Err(e) => println!("Err: {:?}", e),
            }
        }
    }
}

// Internal
impl RenderData {
    fn draw_mode_0(
        &mut self,
        mem:            &mut MemoryCache,
        dynamic_state:  &DynamicState,
        y:              u16
        ) {

        let mut bg_command_buffer = std::mem::replace(&mut self.bg_cb, None).unwrap();
        let mut obj_command_buffer = std::mem::replace(&mut self.obj_cb, None).unwrap();

        // Make descriptor sets for palettes.
        let bg_palettes = mem.get_bg_palette_buffer();

        // Draw
        bg_command_buffer = self.draw_bg(bg_command_buffer, 3, mem, dynamic_state, y, &bg_palettes, [0.95, 0.8]);
        bg_command_buffer = self.draw_bg(bg_command_buffer, 2, mem, dynamic_state, y, &bg_palettes, [0.9, 0.7]);
        bg_command_buffer = self.draw_bg(bg_command_buffer, 1, mem, dynamic_state, y, &bg_palettes, [0.5, 0.2]);
        bg_command_buffer = self.draw_bg(bg_command_buffer, 0, mem, dynamic_state, y, &bg_palettes, [0.4, 0.1]);

        obj_command_buffer = self.draw_objects(obj_command_buffer, mem, dynamic_state, y, [0.85, 0.6, 0.3, 0.0]);

        self.bg_cb = Some(bg_command_buffer);
        self.obj_cb = Some(obj_command_buffer);
    }

    fn draw_mode_1(
        &mut self,
        mem:            &mut MemoryCache,
        dynamic_state:  &DynamicState,
        y:              u16
        ) {

        let mut bg_command_buffer = std::mem::replace(&mut self.bg_cb, None).unwrap();
        let mut obj_command_buffer = std::mem::replace(&mut self.obj_cb, None).unwrap();

        // Make descriptor sets for palettes.
        let bg_palettes = mem.get_bg_palette_buffer();

        // Draw
        bg_command_buffer = self.draw_bg(bg_command_buffer, 2, mem, dynamic_state, y, &bg_palettes, [0.95, if mem.get_bg3_priority() {0.0} else {0.8}]);
        bg_command_buffer = self.draw_bg(bg_command_buffer, 1, mem, dynamic_state, y, &bg_palettes, [0.6, 0.3]);
        bg_command_buffer = self.draw_bg(bg_command_buffer, 0, mem, dynamic_state, y, &bg_palettes, [0.5, 0.2]);

        obj_command_buffer = self.draw_objects(obj_command_buffer, mem, dynamic_state, y, [0.9, 0.7, 0.4, 0.1]);

        self.bg_cb = Some(bg_command_buffer);
        self.obj_cb = Some(obj_command_buffer);
    }

    fn finish_drawing(mut self) -> (AutoCommandBuffer, Box<dyn GpuFuture>, Vec<Box<dyn GpuFuture>>, usize) {
        let bg_command_buffer = std::mem::replace(&mut self.bg_cb, None).unwrap().build().unwrap();
        let obj_command_buffer = std::mem::replace(&mut self.obj_cb, None).unwrap().build().unwrap();
        unsafe {
            (
                self.command_buffer.unwrap()
                    .execute_commands(bg_command_buffer).unwrap()
                    .execute_commands(obj_command_buffer).unwrap()
                    .end_render_pass().unwrap().build().unwrap(),
                self.acquire_future,
                self.image_futures,
                self.image_num
            )
        }
    }
}

// Individual draws
impl RenderData {
    fn draw_bg(&mut self,
        command_buffer: AutoCommandBufferBuilder,
        bg_num:         usize,
        mem:            &mut MemoryCache,
        dynamic_state:  &DynamicState,
        y:              u16,
        palettes:       &mem::palette::PaletteDescriptorSet,
        priorities:     [f32; 2]
    ) -> AutoCommandBufferBuilder {

        let tiles = {
            let (image, write_future) = mem.get_bg_image(bg_num);
            if let Some(future) = write_future {
                self.image_futures.push(future);
            }
            image
        };

        let push_constants = mem.get_bg_push_constants(bg_num, priorities);

        let scrolled_y = mem.calc_y_line(bg_num, y);
        let vertices = mem.get_bg_vertex_buffer(bg_num, scrolled_y);

        command_buffer.draw(
            self.bg_pipeline.clone(),
            dynamic_state,
            vertices,
            (tiles, palettes.clone()),
            push_constants
        ).unwrap()
    }

    fn draw_objects(&mut self,
        command_buffer: AutoCommandBufferBuilder,
        mem:            &mut MemoryCache,
        dynamic_state:  &DynamicState,
        y:              u16,
        priorities:     [f32; 4]
    ) -> AutoCommandBufferBuilder {
        if let Some(sprites) = mem.get_sprite_vertices(y) {
            let palettes = mem.get_obj_palette_buffer();

            let tiles = {
                let (image, write_future) = mem.get_sprite_images();
                if let Some(future) = write_future {
                    self.image_futures.push(future);
                }
                image
            };

            let push_constants = mem.get_obj_push_constants(priorities);

            command_buffer.draw(
                self.obj_pipeline.clone(),
                dynamic_state,
                sprites,
                (tiles, palettes),
                push_constants
            ).unwrap()
        } else {
            command_buffer
        }
    }
}

// Debug
/*impl RenderData {
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
        let palettes = Arc::new(self.bg_set_pool_1.next()
            .add_buffer(mem.get_bg_palette_buffer()).unwrap()
            .build().unwrap());

        // Make descriptor set to bind texture atlases for patterns.

        let bg_set0 = {
            let (image, write_future) = mem.get_bg_image(1);   // Change pattern here to see all tiles.
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
            (bg_set0, palettes.clone()),
            ()
        ).unwrap();

        self.command_buffer = Some(command_buffer);
    }
}*/