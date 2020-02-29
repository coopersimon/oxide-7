mod shaders;

use chrono::{
    Duration, Utc
};
use winit::{
    EventsLoop,
    Event,
    WindowEvent,
    WindowBuilder,
    ElementState,
    VirtualKeyCode
};

use vulkano::{
    instance::{
        Instance, PhysicalDevice
    },
    device::{
        Device, DeviceExtensions
    },
    framebuffer::{
        Framebuffer, Subpass, FramebufferAbstract, RenderPassAbstract
    },
    pipeline::{
        GraphicsPipeline,
        viewport::Viewport
    },
    command_buffer::{
        AutoCommandBufferBuilder,
        DynamicState
    },
    sampler::{
        Filter,
        MipmapMode,
        Sampler,
        SamplerAddressMode
    },
    swapchain::{
        Swapchain, SurfaceTransform, PresentMode, acquire_next_image, CompositeAlpha
    },
    sync::{
        now, GpuFuture
    },
    descriptor::{
        descriptor_set::FixedSizeDescriptorSetsPool,
    },
    buffer::{
        BufferUsage,
        ImmutableBuffer
    },
    image::{
        Dimensions,
        immutable::ImmutableImage
    },
    format::Format
};

use oxide7::*;

// Target output frame rate.
const TARGET_FRAME_RATE: usize = 60;
const FRAME_INTERVAL: f32 = 1.0 / TARGET_FRAME_RATE as f32;

use vulkano_win::VkSurfaceBuild;
use std::sync::Arc;

#[derive(Default, Debug, Clone)]
struct Vertex {
    position:   [f32; 2],
    tex_coord:  [f32; 2]
}

vulkano::impl_vertex!(Vertex, position, tex_coord);

fn main() {
    let cart_path = std::env::args().nth(1).expect("Expected ROM file path as first argument!");

    let debug_mode = std::env::args().nth(2).is_some();

    let mut events_loop = EventsLoop::new();
    let mut snes = SNES::new(&cart_path, "");

    //let mut now = Utc::now();
    let frame_duration = Duration::microseconds((FRAME_INTERVAL * 1_000_000.0) as i64);

    let mut frame_tex = [0_u8; 256 * 224 * 4];

    if debug_mode {
        //#[cfg(feature = "debug")]
        //debug::debug_mode(&mut rustboy);
    } else {
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
            .with_title("Super Rust Boy")
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
            let dimensions = caps.current_extent.unwrap_or([512, 448]);

            //let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            //println!("{:?}", caps.supported_formats);
            let format = caps.supported_formats[0].0;

            (Swapchain::new(device.clone(), surface.clone(),
                caps.min_image_count, format, dimensions, 1, caps.supported_usage_flags, &queue,
                SurfaceTransform::Identity, CompositeAlpha::Opaque, PresentMode::Fifo, true, None
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
            .triangle_strip()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        // Make descriptor set pools.
        let mut set_pool = FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0);

        let (vertices, vertex_future) = ImmutableBuffer::from_iter(
            vec![
                Vertex{position: [-1.0, -1.0], tex_coord: [0.0, 0.0]},
                Vertex{position: [1.0, -1.0], tex_coord: [1.0, 0.0]},
                Vertex{position: [-1.0, 1.0], tex_coord: [0.0, 1.0]},
                Vertex{position: [1.0, 1.0], tex_coord: [1.0, 1.0]},
            ].into_iter(),
            BufferUsage::vertex_buffer(),
            queue.clone()
        ).unwrap();

        let mut previous_frame_future = Box::new(vertex_future) as Box<dyn GpuFuture>;

        loop {
            //println!("Frame");
            let frame = Utc::now();

            read_events(&mut events_loop, &mut snes);
            snes.frame(&mut frame_tex);

            /*for pix in frame_tex.chunks(4) {
                println!("r: {}, g: {}, b: {}", pix[0], pix[1], pix[2]);
            }*/

            // Get current framebuffer index from the swapchain.
            let (image_num, acquire_future) = acquire_next_image(swapchain.clone(), None).expect("Didn't get next image");

            // Get image with current texture.
            let (image, image_future) = ImmutableImage::from_iter(
                frame_tex.iter().cloned(),
                Dimensions::Dim2d { width: 256, height: 224 },
                Format::R8G8B8A8Uint,
                queue.clone()
            ).expect("Couldn't create image.");

            // Make descriptor set to bind texture.
            let set0 = Arc::new(set_pool.next()
                .add_sampled_image(image, sampler.clone()).unwrap()
                .build().unwrap());

            // Start building command buffer using pipeline and framebuffer, starting with the background vertices.
            let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
                .begin_render_pass(framebuffers[image_num].clone(), false, vec![[1.0, 0.0, 0.0, 1.0].into()]).unwrap()
                .draw(
                    pipeline.clone(),
                    &dynamic_state,
                    vertices.clone(),
                    set0.clone(),
                    ()
                ).unwrap().end_render_pass().unwrap().build().unwrap();

            // Wait until previous frame is done.
            let mut now_future = Box::new(now(device.clone())) as Box<dyn GpuFuture>;
            std::mem::swap(&mut previous_frame_future, &mut now_future);

            // Wait until previous frame is done,
            // _and_ the framebuffer has been acquired,
            // _and_ the texture has been uploaded.
            let future = now_future.join(acquire_future)
                .join(image_future)
                .then_execute(queue.clone(), command_buffer).unwrap()                   // Run the commands (pipeline and render)
                .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)    // Present newly rendered image.
                .then_signal_fence_and_flush();                                         // Signal done and flush the pipeline.

            match future {
                Ok(future) => previous_frame_future = Box::new(future) as Box<_>,
                Err(e) => println!("Err: {:?}", e),
            }

            previous_frame_future.cleanup_finished();

            //averager.add((Utc::now() - frame).num_milliseconds());
            //println!("Frame t: {}ms", averager.get_avg());

            while (Utc::now() - frame) < frame_duration {}  // Wait until next frame.
        }
    }
}

/*fn make_save_name(cart_name: &str) -> String {
    match cart_name.find(".") {
        Some(pos) => cart_name[0..pos].to_string() + ".sav",
        None      => cart_name.to_string() + ".sav"
    }
}*/

fn read_events(events_loop: &mut EventsLoop, snes: &mut SNES) {
    events_loop.poll_events(|e| {
        match e {
            Event::WindowEvent {
                window_id: _,
                event: w,
            } => match w {
                WindowEvent::CloseRequested => {
                    ::std::process::exit(0);
                },
                WindowEvent::KeyboardInput {
                    device_id: _,
                    input: k,
                } => {
                    let pressed = match k.state {
                        ElementState::Pressed => true,
                        ElementState::Released => false,
                    };
                    match k.virtual_keycode {
                        Some(VirtualKeyCode::X)         => snes.set_button(Button::A, pressed, 0),
                        Some(VirtualKeyCode::Z)         => snes.set_button(Button::B, pressed, 0),
                        Some(VirtualKeyCode::Space)     => snes.set_button(Button::Select, pressed, 0),
                        Some(VirtualKeyCode::Return)    => snes.set_button(Button::Start, pressed, 0),
                        Some(VirtualKeyCode::Up)        => snes.set_button(Button::Up, pressed, 0),
                        Some(VirtualKeyCode::Down)      => snes.set_button(Button::Down, pressed, 0),
                        Some(VirtualKeyCode::Left)      => snes.set_button(Button::Left, pressed, 0),
                        Some(VirtualKeyCode::Right)     => snes.set_button(Button::Right, pressed, 0),
                        _ => {},
                    }
                },
                _ => {}
            },
            _ => {},
        }
    });
}
