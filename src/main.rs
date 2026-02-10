// SPDX-License-Identifier: Apache-2.0

#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod app;
mod config;
mod types;
mod vulkan;

use anyhow::Result;
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

use app::App;
use types::{Line, Vec2};

const FRAME_TIME: Duration = Duration::from_micros(16_667);

#[rustfmt::skip]
fn main() -> Result<()> {
    pretty_env_logger::init();

    // Window

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Scribble")
        .with_inner_size(LogicalSize::new(1024, 768))
        .build(&event_loop)?;

    // App

    let mut app = unsafe { App::create(&window)? };
    let mut minimized = false;
    let mut left_mouse_down = false;
    let mut last_frame = Instant::now();
    let mut needs_redraw = true;

    event_loop.run(move |event, elwt| {
        match event {
            // Request a redraw when needed and enough time has passed.
            Event::AboutToWait => {
                if needs_redraw {
                    let now = Instant::now();
                    let next_frame_time = last_frame + FRAME_TIME;

                    if now >= next_frame_time {
                        window.request_redraw();
                        needs_redraw = false;
                    } else {
                        elwt.set_control_flow(ControlFlow::WaitUntil(next_frame_time));
                    }
                } else {
                    elwt.set_control_flow(ControlFlow::Wait);
                }
            }
            Event::WindowEvent { event, .. } => match event {
                // Render a frame if our Vulkan app is not being destroyed.
                WindowEvent::RedrawRequested if !elwt.exiting() && !minimized => {
                    unsafe { app.render(&window) }.unwrap();
                    last_frame = Instant::now();
                    needs_redraw = false;
                },
                // Mark the window as having been resized.
                WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        minimized = true;
                    } else {
                        minimized = false;
                        app.resized = true;
                        needs_redraw = true;
                    }
                }
                // Destroy our Vulkan app.
                WindowEvent::CloseRequested => {
                    elwt.exit();
                    unsafe { app.destroy(); }
                }
                // Handle keyboard events.
//                WindowEvent::KeyboardInput { event, .. } => {
//                    if event.state == ElementState::Pressed {
//                        match event.physical_key {
//                            PhysicalKey::Code(KeyCode::ArrowLeft) if app.models > 1 => app.models -= 1,
//                            PhysicalKey::Code(KeyCode::ArrowRight) if app.models < 4 => app.models += 1,
//                            _ => { }
//                        }
//                    }
//                }
                // Track mouse button state
                WindowEvent::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left {
                        left_mouse_down = state == ElementState::Pressed;
                        if !left_mouse_down {
                            unsafe { app.commit_new_line().unwrap() };
                        }
                        needs_redraw = true;
                    }
                }
                // Record position only when left button is down
                WindowEvent::CursorMoved { position, .. } if left_mouse_down => {
                    let window_size = window.inner_size();

                    // Convert pixel coordinates to NDC (-1 to 1)
                    let ndc_x = (position.x as f32 / window_size.width as f32) * 2.0 - 1.0;
                    let ndc_y = (position.y as f32 / window_size.height as f32) * 2.0 - 1.0;

                    // Create a vertex at the mouse position
                    let vertex = Vec2::new(ndc_x, ndc_y);

                    // Append it to your vertex list
                    unsafe { app.append_vertex(vertex) }.unwrap();
                    needs_redraw = true;
                }
                _ => {}
            }
            _ => {}
        }
    })?;

    Ok(())
}
