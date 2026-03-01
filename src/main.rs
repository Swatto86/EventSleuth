//! EventSleuth — a fast, filterable Windows Event Log viewer.
//!
//! Entry point: initialises structured logging and launches the eframe
//! application window.

// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Declare crate modules
mod app;
mod app_actions;
mod app_update;
mod core;
mod export;
mod ui;

use tracing_subscriber::Layer as _;
mod util;

use app::EventSleuthApp;
use util::constants;

/// Guard that holds the single-instance mutex for the lifetime of the process.
/// When dropped the OS automatically releases the mutex.
struct SingleInstanceGuard {
    _handle: windows::Win32::Foundation::HANDLE,
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self._handle);
        }
    }
}

/// Attempt to acquire a named mutex. Returns `Some(guard)` if this is the
/// first instance, or `None` if another instance already holds the mutex.
fn acquire_single_instance() -> Option<SingleInstanceGuard> {
    use windows::core::w;
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::Win32::System::Threading::CreateMutexW;

    let handle = unsafe { CreateMutexW(None, true, w!("Global\\EventSleuth_SingleInstance")) };

    match handle {
        Ok(h) => {
            // Check if the mutex already existed (another instance owns it)
            let last_err = unsafe { windows::Win32::Foundation::GetLastError() };
            if last_err == ERROR_ALREADY_EXISTS {
                unsafe {
                    let _ = windows::Win32::Foundation::CloseHandle(h);
                }
                None
            } else {
                Some(SingleInstanceGuard { _handle: h })
            }
        }
        Err(_) => None,
    }
}

fn main() -> eframe::Result<()> {
    // Enforce single instance
    let _instance_guard = match acquire_single_instance() {
        Some(guard) => guard,
        None => {
            // Another instance is already running — show a message box and exit
            use windows::core::w;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
            unsafe {
                MessageBoxW(
                    None,
                    w!("EventSleuth is already running."),
                    w!("EventSleuth"),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            return Ok(());
        }
    };

    // ── Persistent file logging (Rule 10) ──────────────────────────────
    // Set up dual-layer logging: stderr (env-controlled) + file (always debug).
    // The file log lives at %LOCALAPPDATA%\EventSleuth\logs\eventsleuth.log.
    let log_dir = init_log_dir();
    init_logging(&log_dir);

    tracing::info!(
        "{} v{} starting",
        constants::APP_NAME,
        constants::APP_VERSION,
    );
    if let Some(dir) = &log_dir {
        tracing::info!("Log file: {}", dir.join(constants::LOG_FILE_NAME).display());
    }

    // ── Pre-init (Rule 16: avoid eframe white flash) ────────────────
    // Perform all expensive initialisation BEFORE calling run_native()
    // so the creator closure is trivial.
    let icon = load_app_icon();
    let pre_init = app::PreInitState::build();

    // Configure the native window
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!(
            "{} v{}",
            constants::APP_NAME,
            constants::APP_VERSION
        ))
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 500.0]);

    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        persist_window: true,
        ..Default::default()
    };

    // Launch the application -- creator closure is trivial per Rule 16
    eframe::run_native(
        constants::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(EventSleuthApp::from_pre_init(cc, pre_init)))),
    )
}

/// Create the persistent log directory under `%LOCALAPPDATA%`.
///
/// Returns `Some(path)` to the log directory on success, `None` if the
/// directory cannot be created (logging falls back to stderr only).
fn init_log_dir() -> Option<std::path::PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let log_dir = std::path::PathBuf::from(local_app_data)
        .join(constants::APP_DATA_DIR)
        .join(constants::LOG_DIR);
    std::fs::create_dir_all(&log_dir).ok()?;

    // Rotate the log file if it exceeds the size limit.
    let log_file = log_dir.join(constants::LOG_FILE_NAME);
    if log_file.exists() {
        if let Ok(meta) = std::fs::metadata(&log_file) {
            if meta.len() > constants::MAX_LOG_FILE_SIZE {
                let backup = log_dir.join("eventsleuth.log.old");
                let _ = std::fs::rename(&log_file, &backup);
            }
        }
    }

    Some(log_dir)
}

/// Initialise the dual-layer tracing subscriber.
///
/// - **stderr layer**: filtered by `RUST_LOG` env var (default: `info`).
/// - **file layer** (if `log_dir` is `Some`): always writes at `debug` level
///   to a persistent log file for post-mortem diagnostics.
fn init_logging(log_dir: &Option<std::path::PathBuf>) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr);

    if let Some(dir) = log_dir {
        let log_path = dir.join(constants::LOG_FILE_NAME);
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let file_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(file))
                .with_filter(tracing_subscriber::EnvFilter::new("debug"));

            tracing_subscriber::registry()
                .with(stderr_layer.with_filter(env_filter))
                .with(file_layer)
                .init();
            return;
        }
    }

    // Fallback: stderr only
    tracing_subscriber::registry()
        .with(stderr_layer.with_filter(env_filter))
        .init();
}

/// Load the application icon from the compile-time-embedded ICO data.
///
/// The ICO file is generated by `build.rs` and embedded via `include_bytes!`.
/// Extracts the largest image entry and decodes it to RGBA for use as
/// the window titlebar and taskbar icon.
/// Returns `None` if the icon cannot be decoded.
fn load_app_icon() -> Option<std::sync::Arc<egui::IconData>> {
    static ICO_BYTES: &[u8] = include_bytes!("../assets/icon.ico");

    // Parse the ICO header to find the largest image entry.
    // ICO format: 6-byte header, then 16-byte directory entries.
    if ICO_BYTES.len() < 6 {
        return None;
    }
    let count = u16::from_le_bytes([ICO_BYTES[4], ICO_BYTES[5]]) as usize;
    if count == 0 {
        return None;
    }

    // Find the entry with the largest dimensions
    let mut best_idx = 0usize;
    let mut best_size = 0u32;
    for i in 0..count {
        let offset = 6 + i * 16;
        if offset + 16 > ICO_BYTES.len() {
            break;
        }
        // Width/height: 0 means 256
        let w = if ICO_BYTES[offset] == 0 {
            256u32
        } else {
            ICO_BYTES[offset] as u32
        };
        let h = if ICO_BYTES[offset + 1] == 0 {
            256u32
        } else {
            ICO_BYTES[offset + 1] as u32
        };
        if w * h > best_size {
            best_size = w * h;
            best_idx = i;
        }
    }

    // Read the data offset and size for the best entry
    let dir_offset = 6 + best_idx * 16;
    let data_size = u32::from_le_bytes([
        ICO_BYTES[dir_offset + 8],
        ICO_BYTES[dir_offset + 9],
        ICO_BYTES[dir_offset + 10],
        ICO_BYTES[dir_offset + 11],
    ]) as usize;
    let data_offset = u32::from_le_bytes([
        ICO_BYTES[dir_offset + 12],
        ICO_BYTES[dir_offset + 13],
        ICO_BYTES[dir_offset + 14],
        ICO_BYTES[dir_offset + 15],
    ]) as usize;

    // Use checked_add to guard against integer overflow on malformed ICO data
    // where the sum of offset + size would wrap past usize::MAX (Bug fix: overflow).
    let data_end = data_offset.checked_add(data_size)?;
    if data_end > ICO_BYTES.len() {
        return None;
    }

    let png_data = &ICO_BYTES[data_offset..data_end];

    // Decode the PNG into RGBA pixels
    let img = image::load_from_memory(png_data).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width() as u32, rgba.height() as u32);

    Some(std::sync::Arc::new(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }))
}
