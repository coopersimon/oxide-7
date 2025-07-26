mod debug;

use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler, dpi::{
        LogicalSize, Size, PhysicalSize
    }, event::{
        ElementState, WindowEvent
    }, event_loop::{
        EventLoop
    }, window::Window, keyboard::{PhysicalKey, KeyCode}
};

use cpal::traits::StreamTrait;

use clap::{clap_app, crate_version};

use oxide7::*;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct Vertex {
    position:   [f32; 2],
    tex_coord:  [f32; 2]
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

const FRAME_TIME: chrono::Duration = chrono::Duration::nanoseconds(1_000_000_000 / 60);

struct WindowState {
    window:         std::sync::Arc<Window>,
    surface:        wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

impl WindowState {
    fn resize_surface(&mut self, size: PhysicalSize<u32>, device: &wgpu::Device) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(device, &self.surface_config);
    }
}

struct App {
    window: Option<WindowState>,
    snes:   SNES,

    // WGPU params
    instance:        wgpu::Instance,
    adapter:         wgpu::Adapter,
    device:          wgpu::Device,
    queue:           wgpu::Queue,
    texture_extent:  wgpu::Extent3d,
    texture:         wgpu::Texture,
    bind_group:      wgpu::BindGroup,
    vertex_buffer:   wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,

    screen_buffer: Vec<u8>,
    last_frame_time: chrono::DateTime<chrono::Utc>,

    audio_stream: cpal::Stream
}

impl App {
    fn new(mut snes: SNES) -> Self {
        // Setup wgpu
        let instance = wgpu::Instance::new(&Default::default());

        let adapter = futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        })).expect("Failed to find appropriate adapter");

        let (device, queue) = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            ..Default::default()
        })).expect("Failed to create device");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None
                },
            ]
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[]
        });

        let texture_extent = wgpu::Extent3d {
            width: 512,
            height: 224,
            depth_or_array_layers: 1
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb]
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter:     wgpu::FilterMode::Nearest,
            min_filter:     wgpu::FilterMode::Linear,
            mipmap_filter:  wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler)
                }
            ],
            label: None
        });

        let vertices = vec![
            Vertex{position: [-1.0, -1.0], tex_coord: [0.0, 1.0]},
            Vertex{position: [1.0, -1.0], tex_coord: [1.0, 1.0]},
            Vertex{position: [-1.0, 1.0], tex_coord: [0.0, 0.0]},
            Vertex{position: [1.0, 1.0], tex_coord: [1.0, 0.0]},
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX
        });

        let shader_module = device.create_shader_module(wgpu::include_wgsl!("./shaders/shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 4 * 2,
                            shader_location: 1,
                        },
                    ]
                }],
                compilation_options: Default::default()
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                .. Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default()
            }),
            multiview: None,
            cache: None
        });

        let screen_tex_size = 512 * 224 * 4;
        let audio_stream = make_audio_stream(&mut snes);
        
        Self {
            window: None,
            snes,

            instance,
            adapter,
            device,
            queue,
            texture_extent,
            texture,
            bind_group,
            vertex_buffer,
            render_pipeline,

            screen_buffer: vec![0_u8; screen_tex_size],
            last_frame_time: chrono::Utc::now(),

            audio_stream: audio_stream
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = Window::default_attributes()
            .with_inner_size(Size::Logical(LogicalSize{width: 512_f64, height: 448_f64}))
            .with_title("Oxide-7: ".to_owned() + &self.snes.rom_name());
        let window = std::sync::Arc::new(event_loop.create_window(window_attrs).unwrap());

        // Setup wgpu
        let surface = self.instance.create_surface(window.clone()).expect("Failed to create surface");

        let size = window.inner_size();
        let surface_config = surface.get_default_config(&self.adapter, size.width, size.height).expect("Could not get default surface config");
        surface.configure(&self.device, &surface_config);

        self.window = Some(WindowState {
            window, surface, surface_config
        });

        self.last_frame_time = chrono::Utc::now();
    
        // AUDIO
        self.audio_stream.play().expect("Couldn't start audio stream");

        //let mut in_focus = true;
    }

    fn window_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            _window_id: winit::window::WindowId,
            event: WindowEvent,
        ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::Resized(size) => {
                self.window.as_mut().unwrap().resize_surface(size, &self.device);
            },
            WindowEvent::RedrawRequested => {
                let now = chrono::Utc::now();
                if now.signed_duration_since(self.last_frame_time) >= FRAME_TIME {
                    self.last_frame_time = now;
    
                    self.snes.frame(&mut self.screen_buffer);
    
                    self.queue.write_texture(
                        self.texture.as_image_copy(),
                        &self.screen_buffer, 
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * self.texture_extent.width),
                            rows_per_image: None,
                        },
                        self.texture_extent
                    );
    
                    let frame = self.window.as_ref().unwrap().surface.get_current_texture().expect("Timeout when acquiring next swapchain tex.");
                    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {label: None});
    
                    {
                        let view = frame.texture.create_view(&Default::default());
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                                resolve_target: None,
                            })],
                            depth_stencil_attachment: None,
                            ..Default::default()
                        });
                        rpass.set_pipeline(&self.render_pipeline);
                        rpass.set_bind_group(0, &self.bind_group, &[]);
                        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        rpass.draw(0..4, 0..1);
                    }
    
                    self.queue.submit([encoder.finish()]);
                    frame.present();
                }
                self.window.as_ref().unwrap().window.request_redraw();
            },
            WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                let pressed = match event.state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                };
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyG)        => self.snes.set_button(Button::A, pressed, 0),
                    PhysicalKey::Code(KeyCode::KeyX)        => self.snes.set_button(Button::A, pressed, 0),
                    PhysicalKey::Code(KeyCode::KeyZ)        => self.snes.set_button(Button::B, pressed, 0),
                    PhysicalKey::Code(KeyCode::KeyD)        => self.snes.set_button(Button::X, pressed, 0),
                    PhysicalKey::Code(KeyCode::KeyC)        => self.snes.set_button(Button::Y, pressed, 0),
                    PhysicalKey::Code(KeyCode::KeyA)        => self.snes.set_button(Button::L, pressed, 0),
                    PhysicalKey::Code(KeyCode::KeyS)        => self.snes.set_button(Button::R, pressed, 0),
                    PhysicalKey::Code(KeyCode::Space)       => self.snes.set_button(Button::Select, pressed, 0),
                    PhysicalKey::Code(KeyCode::Enter)       => self.snes.set_button(Button::Start, pressed, 0),
                    PhysicalKey::Code(KeyCode::ArrowUp)     => self.snes.set_button(Button::Up, pressed, 0),
                    PhysicalKey::Code(KeyCode::ArrowDown)   => self.snes.set_button(Button::Down, pressed, 0),
                    PhysicalKey::Code(KeyCode::ArrowLeft)   => self.snes.set_button(Button::Left, pressed, 0),
                    PhysicalKey::Code(KeyCode::ArrowRight)  => self.snes.set_button(Button::Right, pressed, 0),
                    _ => {},
                }
            }
            /*WindowEvent::Focused(focused) => {
                in_focus = focused;
                if !in_focus {
                    audio_stream.pause().expect("Couldn't pause audio stream");
                } else {
                    audio_stream.play().expect("Couldn't restart audio stream");
                }
            },*/
            _ => {}
        }
    }
}

fn main() {
    let app = clap_app!(oxide7 =>
        (version: crate_version!())
        (author: "Simon Cooper")
        (about: "Super Nintendo Entertainment System emulator.")
        (@arg CART: "The path to the game cart to use.")
        (@arg debug: -d "Enter debug mode.")
        (@arg save: -s +takes_value "Save file path.")
        (@arg dsprom: -r +takes_value "DSP ROM path. Needed for DSP games (e.g. Super Mario Kart, Pilotwings)")
    );

    let cmd_args = app.get_matches();

    let cart_path = match cmd_args.value_of("CART") {
        Some(c) => c.to_string(),
        None => panic!("Usage: oxide7 [cart name]. Run with --help for more options."),
    };

    let save_file_path = match cmd_args.value_of("save") {
        Some(c) => c.to_string(),
        None => make_save_name(&cart_path),
    };

    let mut snes = SNES::new(&cart_path, &save_file_path, cmd_args.value_of("dsprom"));

    if cmd_args.is_present("debug") {
        //#[cfg(feature = "debug")]
        debug::debug_mode(&mut snes);
    } else {
        let event_loop = EventLoop::new().expect("Failed to create event loop");

        let mut app = App::new(snes);
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(&mut app).unwrap();
    }
}

fn make_save_name(cart_name: &str) -> String {
    match cart_name.find(".") {
        Some(pos) => cart_name[0..pos].to_string() + ".sav",
        None      => cart_name.to_string() + ".sav"
    }
}

fn make_audio_stream(snes: &mut SNES) -> cpal::Stream {
    use cpal::traits::{
        DeviceTrait,
        HostTrait
    };

    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device available.");

    let config = pick_output_config(&device).with_max_sample_rate();
    let sample_rate = config.sample_rate().0 as f64;
    println!("Audio sample rate {}", sample_rate);
    let mut audio_handler = snes.enable_audio(sample_rate);

    device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _| {
            audio_handler.get_audio_packet(data);
        },
        move |err| {
            println!("Error occurred: {}", err);
        }
    ).unwrap()
}

fn pick_output_config(device: &cpal::Device) -> cpal::SupportedStreamConfigRange {
    use cpal::traits::DeviceTrait;

    const MIN: u32 = 32_000;

    let supported_configs_range = device.supported_output_configs()
        .expect("error while querying configs");

    for config in supported_configs_range {
        let cpal::SampleRate(v) = config.max_sample_rate();
        if v >= MIN {
            return config;
        }
    }

    device.supported_output_configs()
        .expect("error while querying formats")
        .next()
        .expect("No supported config")
}