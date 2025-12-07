mod shaders;
mod gl_utils;
mod world;
mod player;

use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextAttributesBuilder, NotCurrentGlContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasRawWindowHandle;
use std::ffi::{CStr, CString};
use std::num::NonZeroU32;
use winit::{
    event::{Event, WindowEvent, KeyEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::player::Player;
use crate::world::{GRID_SPACING, BUILDING_WIDTH, GROUND_LEVEL, MAX_BUILDING_HEIGHT, get_building_info, check_collision};
use crate::gl_utils::{compile_shader, link_program};
use crate::shaders::{SKY_VERTEX_SHADER, SKY_FRAGMENT_SHADER, SCENE_VERTEX_SHADER, SCENE_FRAGMENT_SHADER};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window_builder = WindowBuilder::new().with_title("Arcade Flyer - Sunrise/Sunset Edition ");

    let template = ConfigTemplateBuilder::new();
    let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

    let (window, gl_config) = display_builder
        .build(&event_loop, template, |configs| {
            configs.reduce(|accum, config| {
                if config.num_samples() > accum.num_samples() { config } else { accum }
            }).unwrap()
        }).unwrap();

    let raw_window_handle = window.as_ref().map(|w| w.raw_window_handle());
    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(glutin::context::ContextApi::Gles(None))
        .build(raw_window_handle);

    let mut not_current_gl_context = Some(unsafe {
        gl_display.create_context(&gl_config, &context_attributes).unwrap_or_else(|_| {
            gl_display.create_context(&gl_config, &fallback_context_attributes).expect("failed to create context ")
        })
    });

    let window = window.unwrap();
    let attrs = window.inner_size();
    let width = std::num::NonZeroU32::new(attrs.width).unwrap();
    let height = std::num::NonZeroU32::new(attrs.height).unwrap();
    let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(window.raw_window_handle(), width, height);
    let surface = unsafe { gl_display.create_window_surface(&gl_config, &surface_attributes).unwrap() };
    let gl_context = not_current_gl_context.take().unwrap().make_current(&surface).unwrap();

    gl::load_with(|symbol| {
        let symbol = CString::new(symbol).unwrap();
        gl_display.get_proc_address(symbol.as_c_str()).cast()
    });

    // --- OpenGL Setup ---
    let sky_program = unsafe {
        let vs = compile_shader(SKY_VERTEX_SHADER, gl::VERTEX_SHADER);
        let fs = compile_shader(SKY_FRAGMENT_SHADER, gl::FRAGMENT_SHADER);
        link_program(vs, fs)
    };

    let scene_program = unsafe {
        let vs = compile_shader(SCENE_VERTEX_SHADER, gl::VERTEX_SHADER);
        let fs = compile_shader(SCENE_FRAGMENT_SHADER, gl::FRAGMENT_SHADER);
        link_program(vs, fs)
    };

    let vertices: [f32; 108] = [
        -0.5, -0.5, -0.5,  0.5, -0.5, -0.5,  0.5,  0.5, -0.5, 
         0.5,  0.5, -0.5, -0.5,  0.5, -0.5, -0.5, -0.5, -0.5, 
        -0.5, -0.5,  0.5,  0.5, -0.5,  0.5,  0.5,  0.5,  0.5, 
         0.5,  0.5,  0.5, -0.5,  0.5,  0.5, -0.5, -0.5,  0.5, 
        -0.5,  0.5,  0.5, -0.5,  0.5, -0.5, -0.5, -0.5, -0.5, 
        -0.5, -0.5, -0.5, -0.5, -0.5,  0.5, -0.5,  0.5,  0.5, 
         0.5,  0.5,  0.5,  0.5,  0.5, -0.5,  0.5, -0.5, -0.5, 
         0.5, -0.5, -0.5,  0.5, -0.5,  0.5,  0.5,  0.5,  0.5, 
        -0.5, -0.5, -0.5,  0.5, -0.5, -0.5,  0.5, -0.5,  0.5, 
         0.5, -0.5,  0.5, -0.5, -0.5,  0.5, -0.5, -0.5, -0.5, 
        -0.5,  0.5, -0.5,  0.5,  0.5, -0.5,  0.5,  0.5,  0.5, 
         0.5,  0.5,  0.5, -0.5,  0.5,  0.5, -0.5,  0.5, -0.5, 
    ];

    let (mut vao, mut vbo) = (0, 0);
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(gl::ARRAY_BUFFER, (vertices.len() * 4) as isize, vertices.as_ptr() as *const _, gl::STATIC_DRAW);
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * 4, std::ptr::null());
        gl::EnableVertexAttribArray(0);
        gl::Enable(gl::DEPTH_TEST);
    }

    let mut player = Player {
        pos: glam::Vec3::new(0.0, 30.0, 0.0),
        yaw: 0.0_f32.to_radians(), // Facing +X (Sunrise)
        pitch: 0.0,
        roll: 0.0,
        speed: 25.0,
    };
    
    let mut keys_pressed = std::collections::HashSet::new();
    let mut last_frame = std::time::Instant::now(); // Initialized here
    let mut game_over = false;
    let mut paused = false;
    let mut p_key_was_pressed = false;
    let mut total_time_elapsed = 0.0f32;

    let _ = event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(size) => {
                    if size.width != 0 && size.height != 0 {
                        surface.resize(&gl_context, NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap());
                        unsafe { gl::Viewport(0, 0, size.width as i32, size.height as i32) };
                    }
                }
                WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(keycode), state, .. }, .. } => {
                     match state {
                        winit::event::ElementState::Pressed => { 
                            keys_pressed.insert(keycode); 
                            if keycode == KeyCode::KeyP && !p_key_was_pressed {
                                paused = !paused;
                                p_key_was_pressed = true;
                                println!("Game Paused: {}", paused);
                            }
                            if game_over && keycode == KeyCode::KeyR {
                                game_over = false;
                                player.pos = glam::Vec3::new(0.0, 40.0, 0.0);
                                player.yaw = 0.0_f32.to_radians();
                                player.pitch = 0.0;
                                player.roll = 0.0;
                                player.speed = 25.0;
                                paused = false; 
                            }
                        }
                        winit::event::ElementState::Released => { 
                            keys_pressed.remove(&keycode); 
                            if keycode == KeyCode::KeyP {
                                p_key_was_pressed = false;
                            }
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    let now = std::time::Instant::now();
                    let dt_duration = now.duration_since(last_frame);
                    let dt = if paused { 0.0 } else { dt_duration.as_secs_f32() };
                    
                    // Only update last_frame if we processed time, OR keep it updating?
                    // If we pause, 'now' keeps advancing. 
                    // Correct logic: 'last_frame' tracks real time. 
                    // 'total_time_elapsed' tracks game time.
                    last_frame = now;

                    if !paused {
                        total_time_elapsed += dt;
                    }
                    let total_time = total_time_elapsed;

                    // --- Day/Night Cycle ---
                    let cycle_duration = 60.0;
                    let day_progress = (total_time % cycle_duration) / cycle_duration;
                    let sun_angle = day_progress * std::f32::consts::TAU;
                    let sun_dir = glam::Vec3::new(
                         sun_angle.cos(), // X
                         sun_angle.sin(), // Y - Height
                         0.2              // Z - Slight tilt
                    ).normalize();

                    if !game_over && !paused {
                        let turn_speed = 2.0 * dt;
                        let mut target_roll = 0.0;
                        
                        if keys_pressed.contains(&KeyCode::ArrowLeft) {
                            player.yaw -= turn_speed;
                            target_roll = -45.0_f32.to_radians(); 
                        } else if keys_pressed.contains(&KeyCode::ArrowRight) {
                            player.yaw += turn_speed;
                            target_roll = 45.0_f32.to_radians(); 
                        }

                        let pitch_speed = 1.5 * dt;
                        if keys_pressed.contains(&KeyCode::ArrowUp) { player.pitch += pitch_speed; } 
                        else if keys_pressed.contains(&KeyCode::ArrowDown) { player.pitch -= pitch_speed; }
                        player.pitch = player.pitch.clamp(-80.0_f32.to_radians(), 80.0_f32.to_radians());

                        if keys_pressed.contains(&KeyCode::KeyW) { player.speed += 20.0 * dt; } 
                        else if keys_pressed.contains(&KeyCode::KeyS) { player.speed -= 20.0 * dt; }
                        player.speed = player.speed.clamp(10.0, 100.0);

                        let roll_lerp_speed = 3.0 * dt;
                        player.roll = player.roll + (target_roll - player.roll) * roll_lerp_speed;

                        let direction = glam::Vec3::new(
                            player.yaw.cos() * player.pitch.cos(),
                            player.pitch.sin(),
                            player.yaw.sin() * player.pitch.cos()
                        ).normalize();

                        player.pos += direction * player.speed * dt;
                        if check_collision(player.pos) { game_over = true; println!("CRASH!"); }
                    }

                    // --- Render ---
                    unsafe {
                        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                        // Calculate Camera Matrices
                        let front = glam::Vec3::new(
                            player.yaw.cos() * player.pitch.cos(),
                            player.pitch.sin(),
                            player.yaw.sin() * player.pitch.cos()
                        ).normalize();
                        let right = front.cross(glam::Vec3::Y).normalize();
                        let roll_quat = glam::Quat::from_axis_angle(front, player.roll);
                        let camera_up = roll_quat * right.cross(front).normalize();
                        
                        let view = glam::Mat4::look_at_rh(player.pos, player.pos + front, camera_up);
                        let projection = glam::Mat4::perspective_rh_gl(60.0_f32.to_radians(), window.inner_size().width as f32 / window.inner_size().height as f32, 0.1, 1000.0);

                        // 1. Draw Skybox
                        gl::Disable(gl::DEPTH_TEST);
                        gl::DepthMask(gl::FALSE);
                        gl::UseProgram(sky_program);
                        
                        let s_view_loc = gl::GetUniformLocation(sky_program, CStr::from_bytes_with_nul(b"view\0").unwrap().as_ptr());
                        let s_proj_loc = gl::GetUniformLocation(sky_program, CStr::from_bytes_with_nul(b"projection\0").unwrap().as_ptr());
                        let s_sun_loc = gl::GetUniformLocation(sky_program, CStr::from_bytes_with_nul(b"uSunDir\0").unwrap().as_ptr());
                        let s_time_loc = gl::GetUniformLocation(sky_program, CStr::from_bytes_with_nul(b"uTime\0").unwrap().as_ptr());
                        let s_cam_loc = gl::GetUniformLocation(sky_program, CStr::from_bytes_with_nul(b"uCameraPos\0").unwrap().as_ptr());

                        gl::UniformMatrix4fv(s_view_loc, 1, gl::FALSE, &view.to_cols_array()[0]);
                        gl::UniformMatrix4fv(s_proj_loc, 1, gl::FALSE, &projection.to_cols_array()[0]);
                        gl::Uniform3f(s_sun_loc, sun_dir.x, sun_dir.y, sun_dir.z);
                        gl::Uniform1f(s_time_loc, total_time);
                        gl::Uniform3f(s_cam_loc, player.pos.x, player.pos.y, player.pos.z);
                        
                        gl::BindVertexArray(vao);
                        gl::DrawArrays(gl::TRIANGLES, 0, 36);

                        // 2. Draw Scene
                        gl::Enable(gl::DEPTH_TEST); // Re-enable depth test
                        gl::DepthMask(gl::TRUE);
                        gl::UseProgram(scene_program);

                        let m_model_loc = gl::GetUniformLocation(scene_program, CStr::from_bytes_with_nul(b"model\0").unwrap().as_ptr());
                        let m_view_loc = gl::GetUniformLocation(scene_program, CStr::from_bytes_with_nul(b"view\0").unwrap().as_ptr());
                        let m_proj_loc = gl::GetUniformLocation(scene_program, CStr::from_bytes_with_nul(b"projection\0").unwrap().as_ptr());
                        let m_color_loc = gl::GetUniformLocation(scene_program, CStr::from_bytes_with_nul(b"uBaseColor\0").unwrap().as_ptr());
                        let m_h_loc = gl::GetUniformLocation(scene_program, CStr::from_bytes_with_nul(b"uMaxHeight\0").unwrap().as_ptr());
                        let m_sun_loc = gl::GetUniformLocation(scene_program, CStr::from_bytes_with_nul(b"uSunDir\0").unwrap().as_ptr());
                        let m_cam_loc = gl::GetUniformLocation(scene_program, CStr::from_bytes_with_nul(b"uCameraPos\0").unwrap().as_ptr());

                        gl::UniformMatrix4fv(m_view_loc, 1, gl::FALSE, &view.to_cols_array()[0]);
                        gl::UniformMatrix4fv(m_proj_loc, 1, gl::FALSE, &projection.to_cols_array()[0]);
                        gl::Uniform1f(m_h_loc, MAX_BUILDING_HEIGHT);
                        gl::Uniform3f(m_sun_loc, sun_dir.x, sun_dir.y, sun_dir.z);
                        gl::Uniform3f(m_cam_loc, player.pos.x, player.pos.y, player.pos.z);

                        // Render Buildings
                        let grid_pos_x = (player.pos.x / GRID_SPACING).floor() as i32;
                        let grid_pos_z = (player.pos.z / GRID_SPACING).floor() as i32;
                        let view_dist = 25; 

                        for x in (grid_pos_x - view_dist)..=(grid_pos_x + view_dist) {
                             for z in (grid_pos_z - view_dist)..=(grid_pos_z + view_dist) {
                                 if let Some((height, color)) = get_building_info(x, z) {
                                     let world_x = x as f32 * GRID_SPACING;
                                     let world_z = z as f32 * GRID_SPACING;
                                     
                                     let model = glam::Mat4::from_translation(glam::Vec3::new(world_x, (height / 2.0) + GROUND_LEVEL, world_z)) 
                                               * glam::Mat4::from_scale(glam::Vec3::new(BUILDING_WIDTH, height, BUILDING_WIDTH));
                                     
                                     gl::UniformMatrix4fv(m_model_loc, 1, gl::FALSE, &model.to_cols_array()[0]);
                                     gl::Uniform3f(m_color_loc, color.x, color.y, color.z);
                                     gl::DrawArrays(gl::TRIANGLES, 0, 36);
                                 }
                             }
                        }

                        // Ground
                        let ground_color = glam::Vec3::new(0.9, 0.8, 0.85); 
                        gl::Uniform3f(m_color_loc, ground_color.x, ground_color.y, ground_color.z);
                        let model = glam::Mat4::from_translation(glam::Vec3::new(player.pos.x, GROUND_LEVEL - 1.0, player.pos.z)) * glam::Mat4::from_scale(glam::Vec3::new(800.0, 1.0, 800.0));
                        gl::UniformMatrix4fv(m_model_loc, 1, gl::FALSE, &model.to_cols_array()[0]);
                        gl::DrawArrays(gl::TRIANGLES, 0, 36);
                    }
                    surface.swap_buffers(&gl_context).unwrap();
                }
                _ => (),
            },
            Event::AboutToWait => { window.request_redraw(); }
            _ => (),
        }
    });
}