// Uncomment these following global attributes to silence most warnings of "low" interest:
/*
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
*/
extern crate nalgebra_glm as glm;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::{mem, os::raw::c_void, ptr};

mod shader;
mod util;

mod scene_graph;
use scene_graph::*;

mod mesh;
use mesh::*;

use glutin::event::{
    DeviceEvent,
    ElementState::{Pressed, Released},
    Event, KeyboardInput,
    VirtualKeyCode::{self, *},
    WindowEvent,
};
use glutin::event_loop::ControlFlow;
use glutin::window::CursorGrabMode;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  byte_size_of_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()

// == // Generate your VAO here
unsafe fn create_vao(
    vertices: &Vec<f32>,
    normals: &Vec<f32>,
    colors: &Vec<f32>,
    indices: &Vec<u32>,
) -> u32 {
    let mut vao: u32 = 0;
    gl::GenVertexArrays(1, &mut vao);
    gl::BindVertexArray(vao);

    let mut vbo: u32 = 0;
    gl::GenBuffers(1, &mut vbo);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices) + byte_size_of_array(normals) + byte_size_of_array(colors),
        ptr::null(),
        gl::STATIC_DRAW,
    );

    gl::BufferSubData(
        gl::ARRAY_BUFFER,
        0,
        byte_size_of_array(vertices),
        pointer_to_array(vertices),
    );
    gl::BufferSubData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices),
        byte_size_of_array(normals),
        pointer_to_array(normals),
    );
    gl::BufferSubData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices) + byte_size_of_array(normals),
        byte_size_of_array(colors),
        pointer_to_array(colors),
    );

    gl::VertexAttribPointer(
        0,
        3,
        gl::FLOAT,
        gl::FALSE,
        size_of::<f32>() * 3,
        ptr::null(),
    );
    gl::EnableVertexAttribArray(0);

    gl::VertexAttribPointer(
        1,
        3,
        gl::FLOAT,
        gl::FALSE,
        size_of::<f32>() * 3,
        byte_size_of_array(vertices) as *const c_void,
    );
    gl::EnableVertexAttribArray(1);

    gl::VertexAttribPointer(
        2,
        4,
        gl::FLOAT,
        gl::FALSE,
        size_of::<f32>() * 4,
        (byte_size_of_array(vertices) + byte_size_of_array(normals)) as *const c_void,
    );
    gl::EnableVertexAttribArray(2);

    let mut ibo: u32 = 0;
    gl::GenBuffers(1, &mut ibo);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo);
    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER,
        byte_size_of_array(indices),
        pointer_to_array(indices),
        gl::STATIC_DRAW,
    );

    return vao;
}
unsafe fn draw_scene(
    node: &scene_graph::SceneNode,
    view_projection_matrix: &glm::Mat4,
    transformation_so_far: &glm::Mat4,
) {
    // Perform any logic needed before drawing the node
    // Check if node is drawable, if so: set uniforms, bind VAO and draw VAO
    if node.index_count > 0 {
        println!("vao={}, index_count={}", node.vao_id, node.index_count);
        gl::BindVertexArray(node.vao_id);

        gl::DrawElements(
            gl::TRIANGLES,
            node.index_count,
            gl::UNSIGNED_INT,
            ptr::null(),
        );
    }

    // Recurse
    for &child in &node.children {
        draw_scene(&*child, view_projection_matrix, transformation_so_far);
    }
}

fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(
            INITIAL_SCREEN_W,
            INITIAL_SCREEN_H,
        ));
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    windowed_context
        .window()
        .set_cursor_grab(CursorGrabMode::Confined)
        .expect("failed to grab cursor");
    windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            // gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!(
                "{}: {}",
                util::get_gl_string(gl::VENDOR),
                util::get_gl_string(gl::RENDERER)
            );
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!(
                "GLSL\t: {}",
                util::get_gl_string(gl::SHADING_LANGUAGE_VERSION)
            );
        }

        let terrain: Mesh = Terrain::load("resources/lunarsurface.obj");
        let helicopter: Helicopter = Helicopter::load("resources/helicopter.obj");

        // == // Set up your VAO around here
        let terrain_vao = unsafe {
            create_vao(
                &terrain.vertices,
                &terrain.normals,
                &terrain.colors,
                &terrain.indices,
            )
        };

        let helicopter_vao: &[u32; 4] = unsafe {
            &[
                create_vao(
                    &helicopter.body.vertices,
                    &helicopter.body.normals,
                    &helicopter.body.colors,
                    &helicopter.body.indices,
                ),
                create_vao(
                    &helicopter.door.vertices,
                    &helicopter.door.normals,
                    &helicopter.door.colors,
                    &helicopter.door.indices,
                ),
                create_vao(
                    &helicopter.main_rotor.vertices,
                    &helicopter.main_rotor.normals,
                    &helicopter.main_rotor.colors,
                    &helicopter.main_rotor.indices,
                ),
                create_vao(
                    &helicopter.tail_rotor.vertices,
                    &helicopter.tail_rotor.normals,
                    &helicopter.tail_rotor.colors,
                    &helicopter.tail_rotor.indices,
                ),
            ]
        };

        let mut scene_root: Node = SceneNode::new();

        let mut terrain_node: Node = SceneNode::from_vao(terrain_vao, terrain.index_count);
        let mut helicopter_root: Node =
            SceneNode::from_vao(helicopter_vao[0], helicopter.body.index_count);

        let mut helicopter_door: Node =
            SceneNode::from_vao(helicopter_vao[1], helicopter.door.index_count);
        let mut helicopter_main_rotor: Node =
            SceneNode::from_vao(helicopter_vao[2], helicopter.main_rotor.index_count);
        let mut helicopter_tail_rotor: Node =
            SceneNode::from_vao(helicopter_vao[3], helicopter.tail_rotor.index_count);

        helicopter_root.add_child(&helicopter_door);
        helicopter_root.add_child(&helicopter_main_rotor);
        helicopter_root.add_child(&helicopter_tail_rotor);

        terrain_node.add_child(&helicopter_root);
        scene_root.add_child(&terrain_node);

        let mut camera_position: glm::Vec3 = glm::vec3(0.0, 0.0, 3.0);

        let mut camera_pitch = 0.0_f32;
        let mut camera_yaw = 0.0_f32;

        // == // Set up your shaders here

        // Basic usage of shader helper:
        // The example code below creates a 'shader' object.
        // It which contains the field `.program_id` and the method `.activate()`.
        // The `.` in the path is relative to `Cargo.toml`.
        // This snippet is not enough to do the exercise, and will need to be modified (outside
        // of just using the correct path), but it only needs to be called once

        let (width, height, _) = *window_size.lock().unwrap();
        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./shaders/simple.vert")
                .attach_file("./shaders/simple.frag")
                .link()
        };
        unsafe {
            simple_shader.activate();
            gl::Uniform2f(
                simple_shader.get_uniform_location("u_resolution"),
                width as f32,
                height as f32,
            );
        }

        // Used to demonstrate keyboard handling for exercise 2.

        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut previous_frame_time = first_frame_time;
        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let _elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(previous_frame_time).as_secs_f32();
            previous_frame_time = now;

            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Window was resized to {}x{}", new_size.0, new_size.1);
                    unsafe {
                        gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32);
                    }
                }
            }

            // Handle keyboard input
            let move_speed = 32.0_f32;
            let rotation_speed = 100_f32;
            let mut input = glm::vec3(0.0, 0.0, 0.0);

            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html
                        VirtualKeyCode::A => {
                            input.x -= delta_time * move_speed;
                        }
                        VirtualKeyCode::D => {
                            input.x += delta_time * move_speed;
                        }

                        VirtualKeyCode::W => {
                            input.z += delta_time * move_speed;
                        }
                        VirtualKeyCode::S => {
                            input.z -= delta_time * move_speed;
                        }
                        VirtualKeyCode::Space => {
                            input.y += delta_time * move_speed;
                        }
                        VirtualKeyCode::LControl => {
                            input.y -= delta_time * move_speed;
                        }

                        VirtualKeyCode::Left => {
                            camera_yaw -= rotation_speed * delta_time;
                        }
                        VirtualKeyCode::Right => {
                            camera_yaw += rotation_speed * delta_time;
                        }

                        VirtualKeyCode::Up => {
                            camera_pitch += rotation_speed * delta_time;
                        }
                        VirtualKeyCode::Down => {
                            camera_pitch -= rotation_speed * delta_time;
                        }

                        // default handler:
                        _ => {}
                    }
                }
            }
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {
                // == // Optionally access the accumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (delta.0 / width as f32, delta.1 / height as f32);
                let sensitivity = 8.0_f32;
                camera_yaw += delta.0 * sensitivity;
                camera_pitch -= delta.1 * sensitivity;

                *delta = (0.0, 0.0); // reset when done
            }

            // == // Please compute camera transforms here (exercise 2 & 3)

            unsafe {
                let yaw = camera_yaw.to_radians();
                let pitch = camera_pitch.to_radians();

                let camera_forward: glm::Vec3 = glm::vec3(
                    pitch.cos() * yaw.cos(),
                    pitch.sin(),
                    pitch.cos() * yaw.sin(),
                );
                let camera_right: glm::Vec3 =
                    glm::normalize(&camera_forward.cross(&glm::vec3(0.0, 1.0, 0.0)));
                let camera_up: glm::Vec3 = camera_right.cross(&camera_forward);

                camera_position = camera_position + glm::vec3(
                    camera_right.dot(&input),
                    camera_up.dot(&input),
                    camera_forward.dot(&input)
                );
                let translation: glm::Vec3 = glm::vec3(
                    -camera_right.dot(&camera_position),
                    -camera_up.dot(&camera_position),
                    camera_forward.dot(&camera_position),
                );

                let view = glm::mat4(
                    camera_right.x,
                    camera_right.y,
                    camera_right.z,
                    translation.x,
                    camera_up.x,
                    camera_up.y,
                    camera_up.z,
                    translation.y,
                    -camera_forward.x,
                    -camera_forward.y,
                    -camera_forward.z,
                    translation.z,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                );

                let perspective: glm::Mat4 = glm::perspective(
                    window_aspect_ratio,
                    45.0_f32.to_radians(),
                    1.0_f32,
                    1000_f32,
                );
                let transform: glm::Mat4 = perspective * view;

                gl::UniformMatrix4fv(
                    simple_shader.get_uniform_location("u_transform"),
                    1,
                    0,
                    transform.as_ptr(),
                );

                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                // == // Issue the necessary gl:: commands to draw your scene here
                draw_scene(&scene_root, &transform, &transform);
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });

    // == //
    // == // From here on down there are only internals.
    // == //

    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                println!(
                    "New window size received: {}x{}",
                    physical_size.width, physical_size.height
                );
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: key_state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        }
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Q => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => {}
        }
    });
}
