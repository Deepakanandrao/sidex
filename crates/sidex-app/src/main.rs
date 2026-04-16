//! SideX application entry point.

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use winit::event_loop::EventLoop;

mod app;
mod clipboard;
mod commands;
mod document_state;
mod event_loop;
mod file_dialog;
mod layout;

/// SideX — a fast, native code editor.
#[derive(Parser, Debug)]
#[command(name = "sidex", version, about)]
struct Cli {
    /// File or folder to open.
    path: Option<PathBuf>,

    /// Open in a new window (even if SideX is already running).
    #[arg(long)]
    new_window: bool,

    /// Wait for the file to be closed before returning.
    #[arg(long)]
    wait: bool,

    /// Show a diff between two files.
    #[arg(long, num_args = 2, value_names = ["FILE1", "FILE2"])]
    diff: Option<Vec<PathBuf>>,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("SideX starting");

    let cli = Cli::parse();
    log::debug!("CLI args: {cli:?}");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let window_attrs = winit::window::Window::default_attributes()
        .with_title("SideX")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0_f64, 720.0));

    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(window_attrs)
            .expect("failed to create window"),
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    let mut application = rt
        .block_on(app::App::new(window.clone(), cli.path.as_deref()))
        .expect("failed to initialise application");

    // If a file path was passed, open it
    if let Some(path) = &cli.path {
        if path.is_file() {
            application.open_file(path);
        }
    }

    log::info!("entering main event loop");
    event_loop::run(event_loop, &mut application, &window);
}
