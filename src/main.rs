mod app;
mod camera;
mod gpu;
mod gui;
mod input;
mod loader;
mod renderer;
mod scene;

use app::App;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();
    log::info!("Starting Render Engine...");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
