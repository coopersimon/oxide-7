mod debug;

use winit::{
    dpi::{
        Size, LogicalSize
    },
    event::{
        Event, WindowEvent,
        ElementState,
        VirtualKeyCode,
    },
    event_loop::EventLoop,
    window::WindowBuilder
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
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_inner_size(Size::Logical(LogicalSize{width: 512_f64, height: 448_f64}))
            .with_title("Oxide-7: ".to_owned() + &snes.rom_name())
            .build(&event_loop).unwrap();

        // Setup wgpu
        let surface = wgpu::Surface::create(&window);

        let adapter = futures::executor::block_on(wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference:   wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface)
            },
            wgpu::BackendBit::PRIMARY
        )).unwrap();

        let (device, queue) = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default()
        }));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        component_type: wgpu::TextureComponentType::Uint,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                },
            ],
            label: None
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let texture_extent = wgpu::Extent3d {
            width: 512,
            height: 224,
            depth: 1
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: None,
        });
        let texture_view = texture.create_default_view();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter:     wgpu::FilterMode::Nearest,
            min_filter:     wgpu::FilterMode::Linear,
            mipmap_filter:  wgpu::FilterMode::Nearest,
            lod_min_clamp:  0.0,
            lod_max_clamp:  100.0,
            compare:        wgpu::CompareFunction::Undefined,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view)
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler)
                }
            ],
            label: None
        });

        let size = window.inner_size();

        let mut swapchain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo
        };

        let mut swapchain = device.create_swap_chain(&surface, &swapchain_desc);

        let vertices = vec![
            Vertex{position: [-1.0, -1.0], tex_coord: [0.0, 1.0]},
            Vertex{position: [1.0, -1.0], tex_coord: [1.0, 1.0]},
            Vertex{position: [-1.0, 1.0], tex_coord: [0.0, 0.0]},
            Vertex{position: [1.0, 1.0], tex_coord: [1.0, 0.0]},
        ];

        let vertex_buf = device.create_buffer_with_data(
            bytemuck::cast_slice(&vertices),
            wgpu::BufferUsage::VERTEX
        );

        let vs = include_bytes!("shaders/shader.vert.spv");
        let vs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).unwrap());

        let fs = include_bytes!("shaders/shader.frag.spv");
        let fs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).unwrap());

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float2,
                            offset: 4 * 2,
                            shader_location: 1,
                        },
                    ]
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let mut last_frame_time = chrono::Utc::now();
        let frame_time = chrono::Duration::nanoseconds(1_000_000_000 / 60);
    
        // AUDIO
        let audio_stream = make_audio_stream(&mut snes);
        audio_stream.play().expect("Couldn't start audio stream");

        let mut in_focus = true;
        
        event_loop.run(move |event, _, _| {
            match event {
                Event::MainEventsCleared if in_focus => window.request_redraw(),
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
                        is_synthetic: _,
                    } => {
                        let pressed = match k.state {
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        };
                        match k.virtual_keycode {
                            Some(VirtualKeyCode::G)         => snes.set_button(Button::A, pressed, 0),
                            Some(VirtualKeyCode::X)         => snes.set_button(Button::A, pressed, 0),
                            Some(VirtualKeyCode::Z)         => snes.set_button(Button::B, pressed, 0),
                            Some(VirtualKeyCode::D)         => snes.set_button(Button::X, pressed, 0),
                            Some(VirtualKeyCode::C)         => snes.set_button(Button::Y, pressed, 0),
                            Some(VirtualKeyCode::A)         => snes.set_button(Button::L, pressed, 0),
                            Some(VirtualKeyCode::S)         => snes.set_button(Button::R, pressed, 0),
                            Some(VirtualKeyCode::Space)     => snes.set_button(Button::Select, pressed, 0),
                            Some(VirtualKeyCode::Return)    => snes.set_button(Button::Start, pressed, 0),
                            Some(VirtualKeyCode::Up)        => snes.set_button(Button::Up, pressed, 0),
                            Some(VirtualKeyCode::Down)      => snes.set_button(Button::Down, pressed, 0),
                            Some(VirtualKeyCode::Left)      => snes.set_button(Button::Left, pressed, 0),
                            Some(VirtualKeyCode::Right)     => snes.set_button(Button::Right, pressed, 0),
                            _ => {},
                        }
                    },
                    WindowEvent::Resized(size) => {
                        swapchain_desc.width = size.width;
                        swapchain_desc.height = size.height;
                        swapchain = device.create_swap_chain(&surface, &swapchain_desc);
                    },
                    WindowEvent::Focused(focused) => {
                        in_focus = focused;
                        if !in_focus {
                            audio_stream.pause().expect("Couldn't pause audio stream");
                        } else {
                            audio_stream.play().expect("Couldn't restart audio stream");
                        }
                    },
                    _ => {}
                },
                Event::RedrawRequested(_) => {
                    let now = chrono::Utc::now();
                    if now.signed_duration_since(last_frame_time) >= frame_time {
                        last_frame_time = now;

                        let mut buf = device.create_buffer_mapped(&wgpu::BufferDescriptor {
                            label: None,
                            size: FRAME_BUFFER_SIZE as u64,
                            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_WRITE
                        });
        
                        snes.frame(&mut buf.data);
        
                        let tex_buffer = buf.finish();
        
                        let frame = swapchain.get_next_texture().expect("Timeout when acquiring next swapchain tex.");
                        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {label: None});
        
                        encoder.copy_buffer_to_texture(
                            wgpu::BufferCopyView {
                                buffer: &tex_buffer,
                                offset: 0,
                                bytes_per_row: 4 * texture_extent.width,
                                rows_per_image: 0
                            },
                            wgpu::TextureCopyView {
                                texture: &texture,
                                mip_level: 0,
                                array_layer: 0,
                                origin: wgpu::Origin3d::ZERO,
                            },
                            texture_extent
                        );
        
                        {
                            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                                    attachment: &frame.view,
                                    resolve_target: None,
                                    load_op: wgpu::LoadOp::Clear,
                                    store_op: wgpu::StoreOp::Store,
                                    clear_color: wgpu::Color::WHITE,
                                }],
                                depth_stencil_attachment: None,
                            });
                            rpass.set_pipeline(&render_pipeline);
                            rpass.set_bind_group(0, &bind_group, &[]);
                            rpass.set_vertex_buffer(0, &vertex_buf, 0, 0);
                            rpass.draw(0..4, 0..1);
                        }
        
                        queue.submit(&[encoder.finish()]);
                    }
                    
                },
                _ => {},
            }

        });
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