use pixels::{Pixels, SurfaceTexture};
use rayon::prelude::*;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseScrollDelta, WindowEvent},
    event_loop::EventLoop,
    keyboard::KeyCode,
    window::{Window, WindowAttributes},
};

const LOGICAL_WIDTH: u32 = 500; // High performance!
const LOGICAL_HEIGHT: u32 = 400;
const WIDTH: u32 = 2000; // Big window!
const HEIGHT: u32 = 1600;
const FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / FPS);

struct Input {
    last_mouse_pos: (f64, f64),
    is_clicked: bool,
}

struct Camera {
    center_re: f64,
    center_im: f64,
    zoom: f64, // Higher = more zoomed in
}

impl Camera {
    fn map(&self, x: u32, y: u32, width: u32, height: u32) -> (f64, f64) {
        let aspect_ratio = width as f64 / height as f64;
        let range = 4.0 / self.zoom;

        let re = self.center_re + (x as f64 / width as f64 - 0.5) * range * aspect_ratio;
        let im = self.center_im + (y as f64 / height as f64 - 0.5) * range;

        (re, im)
    }
}

struct Mandelbrot {
    window: Option<&'static Window>,
    pixels: Option<Pixels<'static>>,
    last_frame: Instant,
    camera: Camera,
    input: Input,
}

impl Mandelbrot {
    fn is_in_set(c_re: f32, c_im: f32, max_depth: u32) -> u8 {
        let mut z_re = 0.0;
        let mut z_im = 0.0;

        for i in 0..max_depth {
            let re_sq = z_re * z_re;
            let im_sq = z_im * z_im;

            if re_sq + im_sq > 4.0 {
                return ((i as f32 / max_depth as f32) * 255.0) as u8;
            }
            z_im = 2.0 * z_re * z_im + c_im;
            z_re = re_sq - im_sq + c_re;
        }

        255
    }
}

impl Default for Mandelbrot {
    fn default() -> Self {
        Mandelbrot {
            window: None,
            pixels: None,
            last_frame: Instant::now(),
            camera: Camera {
                center_re: 0.0,
                center_im: 0.0,
                zoom: 1.0,
            },
            input: Input {
                last_mouse_pos: (0.0, 0.0),
                is_clicked: false,
            },
        }
    }
}

impl ApplicationHandler for Mandelbrot {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let win_attr = WindowAttributes::default()
            .with_title("Mandelbrot")
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));

        let win: &'static Window = Box::leak(Box::new(event_loop.create_window(win_attr).unwrap()));

        // The first two arguments are the width/height of the PIXEL BUFFER
        let surface_texture = SurfaceTexture::new(LOGICAL_WIDTH, LOGICAL_HEIGHT, win);

        // Pixels will now take our small buffer and upscale it to the window
        let pixels = Pixels::new(LOGICAL_WIDTH, LOGICAL_HEIGHT, surface_texture).unwrap();

        self.window = Some(win);
        self.pixels = Some(pixels);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    match event.physical_key {
                        winit::keyboard::PhysicalKey::Code(KeyCode::Escape) => {
                            event_loop.exit();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, .. } => {
                self.input.is_clicked = state.is_pressed();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };

                if scroll_amount > 0.0 {
                    self.camera.zoom *= 1.1;
                } else {
                    self.camera.zoom /= 1.1;
                }

                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let d_x = position.x - self.input.last_mouse_pos.0;
                let d_y = position.y - self.input.last_mouse_pos.1;

                if self.input.is_clicked {
                    let aspect_ratio = WIDTH as f64 / HEIGHT as f64;
                    let range = 4.0 / self.camera.zoom;

                    self.camera.center_re -= (d_x / WIDTH as f64) * range * aspect_ratio;
                    self.camera.center_im -= (d_y / HEIGHT as f64) * range;

                    if let Some(window) = self.window {
                        window.request_redraw();
                    }
                }
                self.input.last_mouse_pos = (position.x, position.y);
            }

            WindowEvent::RedrawRequested => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    frame.fill(0);
                    frame
                        .par_chunks_exact_mut(4)
                        .enumerate()
                        .for_each(|(i, pixel)| {
                            let x = (i as u32) % LOGICAL_WIDTH;
                            let y = (i as u32) / LOGICAL_WIDTH;

                            let (re, im) = self.camera.map(x, y, LOGICAL_WIDTH, LOGICAL_HEIGHT);
                            let val = Mandelbrot::is_in_set(re as f32, im as f32, 100);

                            pixel[0] = val;
                            pixel[1] = val;
                            pixel[2] = val;
                            pixel[3] = 255;
                        });
                    if let Err(err) = pixels.render() {
                        eprintln!("pixels.render() failed: {err}");
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = Instant::now();
        if now - self.last_frame >= FRAME_DURATION {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

fn main(){
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = Mandelbrot::default();
    let _ = event_loop.run_app(&mut app);
}
