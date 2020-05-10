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
    event_loop::{
        ControlFlow, EventLoop
    },
    window::WindowBuilder
};

use crossbeam_channel::{
    unbounded,
    Receiver
};

use oxide7::*;

// Target output frame rate.
const TARGET_FRAME_RATE: usize = 60;
const FRAME_INTERVAL: f32 = 1.0 / TARGET_FRAME_RATE as f32;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct Vertex {
    position:   [f32; 2],
    tex_coord:  [f32; 2]
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

struct ButtonEvent {
    button: Button,
    pressed: bool
}

fn main() {
    let cart_path = std::env::args().nth(1).expect("Expected ROM file path as first argument!");

    let debug_mode = std::env::args().nth(2).is_some();

    let save_file_name = make_save_name(&cart_path);

    if debug_mode {
        let mut snes = SNES::new(&cart_path, &save_file_name);

        #[cfg(feature = "debug")]
        debug::debug_mode(&mut snes);
    } else {
        let mut frame_tex = Box::new([0_u8; FRAME_BUFFER_SIZE]);

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_inner_size(Size::Logical(LogicalSize{width: 512_f64, height: 448_f64}))
            .with_title("Oxide-7")
            .build(&event_loop).unwrap();

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
                anisotropic_filtering: false, // TODO: need this?
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

        let mut loop_helper = spin_sleep::LoopHelper::builder()
            .native_accuracy_ns(1_000_000)
            .report_interval_s(1.0)
            .build_with_target_rate(60.0);

        let (send, mut recv) = unbounded();

        std::thread::spawn(move || {
            let mut snes = SNES::new(&cart_path, &save_file_name);

            loop {
                let _ = loop_helper.loop_start();

                read_events(&mut recv, &mut snes);
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

                /*if let Some(fps) = loop_helper.report_rate() {
                    println!("Current fps: {}", fps.round());
                }*/

                loop_helper.loop_sleep();
            }
        });

        event_loop.run(move |event, _, _| {
            match event {
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
                            Some(VirtualKeyCode::X)         => send.send(ButtonEvent{ button: Button::A, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::Z)         => send.send(ButtonEvent{ button: Button::B, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::D)         => send.send(ButtonEvent{ button: Button::X, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::C)         => send.send(ButtonEvent{ button: Button::Y, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::A)         => send.send(ButtonEvent{ button: Button::L, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::S)         => send.send(ButtonEvent{ button: Button::R, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::Space)     => send.send(ButtonEvent{ button: Button::Select, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::Return)    => send.send(ButtonEvent{ button: Button::Start, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::Up)        => send.send(ButtonEvent{ button: Button::Up, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::Down)      => send.send(ButtonEvent{ button: Button::Down, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::Left)      => send.send(ButtonEvent{ button: Button::Left, pressed: pressed }).unwrap(),
                            Some(VirtualKeyCode::Right)     => send.send(ButtonEvent{ button: Button::Right, pressed: pressed }).unwrap(),
                            _ => {},
                        }
                    },
                    _ => {}
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

fn read_events(event_queue: &mut Receiver<ButtonEvent>, snes: &mut SNES) {
    while let Ok(e) = event_queue.try_recv() {
        snes.set_button(e.button, e.pressed, 0);
    }
}
