use std::{cell::RefCell, rc::Rc};

use cgmath::*;

use winit::{
    event::{Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use rand::Rng;

use bytemuck::{Pod, Zeroable};

use wgpu::{BindGroup, BindingResource, Buffer, Device, Queue, RenderPipeline, SwapChain, util::DeviceExt};

pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

fn move_camera(cam_pos_x: f64, cam_pos_y: f64, position_x: f64, _position_y: f64) -> (f64, f64) {
    (cam_pos_x + (position_x / 100000.0), cam_pos_y)
}

fn get_matrix(aspect_ratio: f64, cam_pos: (f64, f64)) -> Matrix4<f32> {
    assert!(aspect_ratio > 0.0f64);

    let perspective : PerspectiveFov<f32> = PerspectiveFov::<f32> {
        fovy: Rad::<f32>::from(Deg::<f32>(90.0)),
        aspect: aspect_ratio as f32,
        near: 0.01,
        far: 1000.0,
    };

    let projection_matrix = 
        Matrix4::<f32>::from(perspective.to_perspective());

    let distance = 3.0f32;

    let transformation_matrix = Matrix4::look_at(
        Point3::new((cam_pos.0 as f32).cos() * distance, 0.0 as f32, (cam_pos.0 as f32).sin() * distance),
        Point3::new(0f32, 0.0, 0.0),
        Vector3::unit_y(),
    );

    OPENGL_TO_WGPU_MATRIX * projection_matrix * transformation_matrix
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 3],
    _color: [f32; 4],
}

fn alter_buffer(device: &Device, rc_ref_cell_vertex_data: Rc<RefCell<Vec<Vertex>>>) -> wgpu::Buffer {
    let mut vertex_data = rc_ref_cell_vertex_data.borrow_mut();
    vertex_data.clear();

    let max = 20;
    let mut rng = rand::thread_rng();

    for _i in 0..max {
        let x = rng.gen_range(-1.0, 1.0);
        let y = rng.gen_range(-1.0, 1.0);
        let z = rng.gen_range(-1.0, 1.0);

        let color_1 = rng.gen_range(0.0, 1.0);
        let color_2 = rng.gen_range(0.0, 1.0);
        let color_3 = rng.gen_range(0.0, 1.0);

        vertex_data.push(Vertex {
            _pos: [x, y, z],
            _color: [color_1, color_2, color_3, 1.0],
        });
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsage::VERTEX,
    });

    vertex_buffer
}

fn draw(
    swap_chain: &mut SwapChain,
    device: &Device,
    render_pipeline: &RenderPipeline,
    queue: &Queue,
    rc_refcell_vertex_data: Rc<RefCell<Vec<Vertex>>>,
    vertex_buffer: &Buffer,
    uniform_bind_group: &BindGroup,
    ratio: f64,
    uniform_buffer: &Buffer,
    cam_pos: (f64, f64)
) {

    let vertex_data = rc_refcell_vertex_data.borrow();
    let frame = swap_chain
        .get_current_frame()
        .expect("Failed to acquire next swap chain texture")
        .output;

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

        let vertex_count = vertex_data.len() as u32;

        let mx_total = get_matrix(ratio, cam_pos);
        let mx_ref: &[f32; 16] = mx_total.as_ref();

        queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(mx_ref)
        );

        render_pass.draw(0..vertex_count, 0..1);
    }

    queue.submit(Some(encoder.finish()));
}   

async fn run(event_loop: EventLoop<()>, window: Window, swapchain_format: wgpu::TextureFormat) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let vs_module = device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));

    let fs_module = device.create_shader_module(wgpu::include_spirv!("shader.frag.spv"));

    let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None,
                },
                count: None,
            }
        ],
        label: Some("uniform_bind_group_layout")
    });

    let (mut cam_pos_x, mut cam_pos_y) = (0.0f64, -4.0f64);

    let mx_total = get_matrix(1.0, (cam_pos_x, cam_pos_y));
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(mx_ref),
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(uniform_buffer.slice(..))
            },
        ],
        label: Some("uniform_bind_group"),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&uniform_bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        // Use the default rasterizer state: no culling, no depth bias
        rasterization_state: None,
        primitive_topology: wgpu::PrimitiveTopology::PointList,
        color_states: &[swapchain_format.into()],
        depth_stencil_state: None,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float4],
            }],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);
    let mut ratio = 1.0f64;

    ///////////////////////////////
    let vertex_data: Rc<RefCell<Vec<Vertex>>> = 
    {
        let mut vertex_data_inner: Vec<Vertex> = vec![];
        let mut rng = rand::thread_rng();
        let max = 50;
        for i in 0..max {
            let percent = i as f32 / max as f32;
            let (sin, cos) = (percent * 2.0 * std::f32::consts::PI).sin_cos();
            let (r, g, b) = 
                (
                    rng.gen_range(0.0, 1.0),
                    rng.gen_range(0.0, 1.0),
                    rng.gen_range(0.0, 1.0)
                );

                vertex_data_inner.push(Vertex {
                _pos: [1.0 * cos, 1.0 * sin, 0.99],
                _color: [r, g, b, 1.0],
            });
        }

        Rc::new(RefCell::new(vertex_data_inner))
    };

    /////////////////////////////////////
    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (
            &instance,
            &adapter,
            &vs_module,
            &fs_module,
            &pipeline_layout,
        );

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Recreate the swap chain with the new size
                sc_desc.width = size.width;
                sc_desc.height = size.height;
                ratio = (size.width as f64) / (size.height as f64);
                swap_chain = device.create_swap_chain(&surface, &sc_desc);
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput { .. } => {}

                WindowEvent::MouseInput { button, .. } => {
                    if button == MouseButton::Right {

                        let vertex_buffer = alter_buffer(&device, vertex_data.clone());

                        draw(
                            &mut swap_chain,
                            &device,
                            &render_pipeline,
                            &queue,
                            vertex_data.clone(),
                            &vertex_buffer,
                            &uniform_bind_group,
                            ratio,
                            &uniform_buffer,
                            (cam_pos_x, cam_pos_y)
                        );
                    }
                }

                WindowEvent::CursorMoved {
                    position,
                    ..
                } => {
                    let (new_cam_pos_x, new_cam_pos_y) = 
                        move_camera(cam_pos_x, cam_pos_y, position.x, position.y);

                    cam_pos_x = new_cam_pos_x;
                    cam_pos_y = new_cam_pos_y;

                    let vertex_data_inner = vertex_data.borrow();

                    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: bytemuck::cast_slice(&vertex_data_inner),
                        usage: wgpu::BufferUsage::VERTEX,
                    });

                    draw(
                        &mut swap_chain,
                        &device,
                        &render_pipeline,
                        &queue,
                        vertex_data.clone(),
                        &vertex_buffer,
                        &uniform_bind_group,
                        ratio,
                        &uniform_buffer,
                        (cam_pos_x, cam_pos_y)
                    );
                }

                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },

            Event::RedrawRequested(_) => {

                let vertex_data_inner = vertex_data.borrow();

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertex_data_inner),
                    usage: wgpu::BufferUsage::VERTEX,
                });            

                draw(
                    &mut swap_chain,
                    &device,
                    &render_pipeline,
                    &queue,
                    vertex_data.clone(),
                    &vertex_buffer,
                    &uniform_bind_group,
                    ratio,
                    &uniform_buffer,
                    (cam_pos_x, cam_pos_y)
                );
            }

            _ => {}
        }
    });
}

fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    wgpu_subscriber::initialize_default_subscriber(None);
    // Temporarily avoid srgb formats for the swapchain on the web
    futures::executor::block_on(run(event_loop, window, wgpu::TextureFormat::Bgra8UnormSrgb));
}
