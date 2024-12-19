// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde::{Deserialize, Serialize};
use windows::Win32::System::EventLog::{
    OpenEventLogW, ReadEventLogW, CloseEventLog,
    EVENTLOG_SEQUENTIAL_READ, READ_EVENT_LOG_READ_FLAGS,
    EVENTLOGRECORD,
};
use windows::Win32::System::Registry::*;
use windows::core::{PCWSTR, PWSTR};
use regex::Regex;
use windows::Win32::Foundation::*;
use tauri_plugin_dialog;
use tauri_plugin_fs;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconEvent, MouseButton, MouseButtonState},
    WindowEvent,
    Manager,
};
use std::sync::Once;
use tauri::tray::TrayIconBuilder;
use is_elevated;

static TRAY_INIT: Once = Once::new();

#[derive(Debug, Serialize)]
struct EventLogEntry {
    source: String,
    time_generated: String,
    event_id: u32,
    event_type: String,
    severity: String,
    category: u16,
    message: String,
    computer_name: String,
    raw_data: Option<Vec<u8>>,
    user_sid: Option<String>,
    matches: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SearchParams {
    keywords: Vec<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    log_names: Vec<String>,
    event_types: Vec<u32>,
    event_ids: Vec<u32>,
    sources: Vec<String>,
    categories: Vec<u16>,
    exclude_keywords: Vec<String>,
    max_results: Option<usize>,
}

fn convert_event_type(event_type: u32) -> String {
    match event_type {
        1 => "Error".to_string(),        // EVENTLOG_ERROR_TYPE
        2 => "Warning".to_string(),      // EVENTLOG_WARNING_TYPE
        4 => "Information".to_string(),  // EVENTLOG_INFORMATION_TYPE
        8 => "Audit Success".to_string(), // EVENTLOG_AUDIT_SUCCESS
        16 => "Audit Failure".to_string(), // EVENTLOG_AUDIT_FAILURE
        _ => "Unknown".to_string(),
    }
}

// Helper function to convert severity level to string
fn convert_severity(event_type: u32) -> String {
    match event_type {
        1 => "Critical".to_string(),
        2 => "Error".to_string(),
        3 => "Warning".to_string(),
        4 => "Information".to_string(),
        5 => "Verbose".to_string(),
        _ => format!("Unknown ({})", event_type),
    }
}

#[tauri::command]
fn check_admin_rights() -> bool {
    is_elevated::is_elevated()
}

#[tauri::command]
async fn search_event_logs(params: SearchParams) -> Result<Vec<EventLogEntry>, String> {
    println!("Starting search with params: {:?}", params);
    let mut results = Vec::new();
    
    // Compile regex patterns for both include and exclude keywords
    let keywords: Vec<Regex> = params
        .keywords
        .iter()
        .map(|k| Regex::new(&regex::escape(k)).unwrap())
        .collect();
    
    let exclude_patterns: Vec<Regex> = params
        .exclude_keywords
        .iter()
        .map(|k| Regex::new(&regex::escape(k)).unwrap())
        .collect();

    // Convert date strings to timestamps if provided
    let start_timestamp = params.start_date
        .as_ref()
        .and_then(|date_str| chrono::DateTime::parse_from_rfc3339(date_str).ok())
        .map(|dt| dt.timestamp());
    
    let end_timestamp = params.end_date
        .as_ref()
        .and_then(|date_str| chrono::DateTime::parse_from_rfc3339(date_str).ok())
        .map(|dt| dt.timestamp());

    // Get all available logs if no specific logs are requested
    let logs_to_search = if params.log_names.is_empty() {
        match get_available_logs().await {
            Ok(logs) => logs,
            Err(_) => vec!["Application".to_string(), "System".to_string(), "Security".to_string()] // Fallback to default logs
        }
    } else {
        params.log_names.clone()
    };

    println!("Searching logs: {:?}", logs_to_search);

    const EVENTLOG_FORWARDS_READ: u32 = 0x0004;

    // Only apply limit if max_results is Some and greater than 0
    let should_limit = params.max_results.map_or(false, |limit| limit > 0);

    for log_name in &logs_to_search {
        // Change the limit check to respect unlimited results
        if should_limit && results.len() >= params.max_results.unwrap() {
            println!("Reached maximum results limit of {}", params.max_results.unwrap());
            break;
        }

        println!("Attempting to open log: {}", log_name);
        unsafe {
            let handle = OpenEventLogW(
                PCWSTR::null(),
                PCWSTR::from_raw(
                    log_name.encode_utf16().chain(Some(0)).collect::<Vec<u16>>().as_ptr(),
                ),
            );

            if let Ok(handle) = handle {
                println!("Successfully opened log: {}", log_name);
                let mut buffer_size = 0x20000; // Start with 128KB
                let mut record_buffer: Vec<u8> = vec![0; buffer_size];
                let mut read_bytes: u32 = 0;
                let mut needed_bytes: u32 = 0;

                loop {
                    // Update the limit check here too
                    if should_limit && results.len() >= params.max_results.unwrap() {
                        break;
                    }

                    let flags = READ_EVENT_LOG_READ_FLAGS(
                        EVENTLOG_SEQUENTIAL_READ.0 | EVENTLOG_FORWARDS_READ
                    );

                    let result = ReadEventLogW(
                        handle,
                        flags,
                        0,
                        record_buffer.as_mut_ptr() as _,
                        buffer_size as u32,
                        &mut read_bytes,
                        &mut needed_bytes,
                    );

                    match result {
                        Ok(_) => {
                            if read_bytes == 0 {
                                break;
                            }

                            let mut offset = 0;
                            while offset < read_bytes as usize {
                                let record = (record_buffer.as_ptr().add(offset)) as *const EVENTLOGRECORD;
                                if offset + (*record).Length as usize > read_bytes as usize {
                                    break;
                                }

                                let time_generated = (*record).TimeGenerated as i64;
                                let event_type = (*record).EventType.0 as u32;
                                let event_id = (*record).EventID;
                                let category = (*record).EventCategory;

                                // Apply filters
                                let mut should_include = true;

                                // Date range filter
                                if let Some(start) = start_timestamp {
                                    if time_generated < start {
                                        should_include = false;
                                    }
                                }
                                if let Some(end) = end_timestamp {
                                    if time_generated > end {
                                        should_include = false;
                                    }
                                }

                                // Event type filter
                                if !params.event_types.is_empty() && !params.event_types.contains(&event_type) {
                                    should_include = false;
                                }

                                // Event ID filter
                                if !params.event_ids.is_empty() && !params.event_ids.contains(&event_id) {
                                    should_include = false;
                                }

                                // Category filter
                                if !params.categories.is_empty() && !params.categories.contains(&category) {
                                    should_include = false;
                                }

                                // Source filter
                                if !params.sources.is_empty() {
                                    let source_name = {
                                        let string_offset = offset + (*record).StringOffset as usize;
                                        let string_ptr = record_buffer[string_offset..].as_ptr() as *const u16;
                                        let mut string_len = 0;
                                        while *string_ptr.add(string_len) != 0 {
                                            string_len += 1;
                                        }
                                        String::from_utf16(std::slice::from_raw_parts(string_ptr, string_len))
                                            .unwrap_or_default()
                                    };
                                    if !params.sources.iter().any(|s| s.eq_ignore_ascii_case(&source_name)) {
                                        should_include = false;
                                    }
                                }

                                // Extract message only if other filters pass
                                if should_include {
                                    let message = {
                                        let strings_offset = offset + (*record).StringOffset as usize;
                                        let strings_count = (*record).NumStrings;
                                        let mut string_parts = Vec::new();
                                        let mut current_offset = strings_offset;

                                        for _ in 0..strings_count {
                                            if current_offset >= record_buffer.len() {
                                                break;
                                            }

                                            let string_ptr = record_buffer[current_offset..].as_ptr() as *const u16;
                                            let mut string_len = 0;
                                            
                                            while string_len < ((record_buffer.len() - current_offset) / 2) {
                                                if *string_ptr.add(string_len) == 0 {
                                                    break;
                                                }
                                                string_len += 1;
                                            }

                                            if string_len > 0 {
                                                if let Ok(part) = String::from_utf16(
                                                    std::slice::from_raw_parts(string_ptr, string_len)
                                                ) {
                                                    string_parts.push(part);
                                                }
                                            }

                                            current_offset += (string_len + 1) * 2;
                                            if current_offset >= read_bytes as usize {
                                                break;
                                            }
                                        }

                                        if !string_parts.is_empty() {
                                            let mut msg = string_parts[0].clone();
                                            for (i, part) in string_parts[1..].iter().enumerate() {
                                                msg = msg.replace(&format!("%{}", i + 1), part);
                                            }
                                            msg
                                        } else {
                                            String::new()
                                        }
                                    };

                                    // Keyword and exclude filters
                                    if exclude_patterns.iter().any(|pattern| pattern.is_match(&message.to_lowercase())) {
                                        should_include = false;
                                    }

                                    let mut matches = Vec::new();
                                    if !keywords.is_empty() {
                                        let message_lower = message.to_lowercase();
                                        for keyword in &keywords {
                                            if keyword.is_match(&message_lower) {
                                                matches.push(params.keywords[matches.len()].clone());
                                            }
                                        }
                                        if matches.is_empty() {
                                            should_include = false;
                                        }
                                    } else {
                                        matches.push("*".to_string());
                                    }

                                    if should_include {
                                        results.push(EventLogEntry {
                                            source: log_name.clone(),
                                            time_generated: chrono::DateTime::from_timestamp(
                                                time_generated,
                                                0,
                                            )
                                            .unwrap_or_default()
                                            .with_timezone(&chrono::Local)
                                            .to_rfc3339(),
                                            event_id,
                                            event_type: convert_event_type(event_type),
                                            severity: convert_severity(event_type),
                                            category,
                                            message,
                                            computer_name: String::new(),
                                            raw_data: None,
                                            user_sid: None,
                                            matches,
                                        });
                                    }
                                }

                                offset += (*record).Length as usize;
                            }
                        },
                        Err(error) => {
                            if error.code().0 as u32 == ERROR_INSUFFICIENT_BUFFER.0 {
                                buffer_size = needed_bytes as usize;
                                record_buffer.resize(buffer_size, 0);
                                continue;
                            }
                            // End of file is expected, not an error
                            if error.code().0 as u32 != 0x80070026 {
                                println!("Error reading log {}: {:?}", log_name, error);
                            }
                            break;
                        }
                    }
                }

                CloseEventLog(handle).ok();
                println!("Closed log: {}", log_name);
            }
        }
    }

    println!("Search complete. Found {} results", results.len());
    Ok(results)
}

#[tauri::command]
async fn get_available_logs() -> Result<Vec<String>, String> {
    let mut logs = Vec::new();
    unsafe {
        let mut key_handle = HKEY::default();
        let path = "SYSTEM\\CurrentControlSet\\Services\\EventLog\0";
        let path_wide: Vec<u16> = path.encode_utf16().collect();
        
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR::from_raw(path_wide.as_ptr()),
            0,
            KEY_READ,
            &mut key_handle,
        ).is_ok() {
            let mut index = 0;
            let mut name_buffer = vec![0u16; 256];
            let mut name_size = name_buffer.len() as u32;

            while RegEnumKeyExW(
                key_handle,
                index,
                PWSTR::from_raw(name_buffer.as_mut_ptr()),
                &mut name_size,
                Some(std::ptr::null_mut()),
                PWSTR::null(),
                Some(std::ptr::null_mut()),
                Some(std::ptr::null_mut()),
            ).is_ok() {
                if let Ok(log_name) = String::from_utf16(&name_buffer[..name_size as usize]) {
                    // Try to open the log to verify it's accessible
                    let log_name_wide: Vec<u16> = log_name.encode_utf16().chain(Some(0)).collect();
                    let handle = OpenEventLogW(
                        PCWSTR::null(),
                        PCWSTR::from_raw(log_name_wide.as_ptr()),
                    );
                    
                    if let Ok(handle) = handle {
                        logs.push(log_name.clone());
                        let _ = CloseEventLog(handle);
                    }

                    // Also check for subkeys (Application and Services logs)
                    let mut subkey_handle = HKEY::default();
                    let subkey_path = format!("{}\\{}", path.trim_end_matches('\0'), log_name);
                    let subkey_path_wide: Vec<u16> = subkey_path.encode_utf16().collect();
                    
                    if RegOpenKeyExW(
                        HKEY_LOCAL_MACHINE,
                        PCWSTR::from_raw(subkey_path_wide.as_ptr()),
                        0,
                        KEY_READ,
                        &mut subkey_handle,
                    ).is_ok() {
                        let mut sub_index = 0;
                        let mut sub_name_size = name_buffer.len() as u32;

                        while RegEnumKeyExW(
                            subkey_handle,
                            sub_index,
                            PWSTR::from_raw(name_buffer.as_mut_ptr()),
                            &mut sub_name_size,
                            Some(std::ptr::null_mut()),
                            PWSTR::null(),
                            Some(std::ptr::null_mut()),
                            Some(std::ptr::null_mut()),
                        ).is_ok() {
                            if let Ok(sub_log_name) = String::from_utf16(&name_buffer[..sub_name_size as usize]) {
                                let full_log_name = format!("{}/{}", log_name, sub_log_name);
                                let full_name_wide: Vec<u16> = full_log_name.encode_utf16().chain(Some(0)).collect();
                                let sub_handle = OpenEventLogW(
                                    PCWSTR::null(),
                                    PCWSTR::from_raw(full_name_wide.as_ptr()),
                                );
                                
                                if let Ok(sub_handle) = sub_handle {
                                    logs.push(full_log_name);
                                    let _ = CloseEventLog(sub_handle);
                                }
                            }
                            sub_index += 1;
                            sub_name_size = name_buffer.len() as u32;
                        }
                        let _ = RegCloseKey(subkey_handle).map_err(|e| {
                            println!("Warning: Failed to close subkey handle: {:?}", e);
                        });
                    }
                }
                index += 1;
                name_size = name_buffer.len() as u32;
            }
            let _ = RegCloseKey(key_handle).map_err(|e| {
                println!("Warning: Failed to close registry key handle: {:?}", e);
            });
        }
    }
    Ok(logs)
}

fn create_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    TRAY_INIT.call_once(|| {
        // Create menu items
        let quit_item = MenuItem::with_id(app.handle(), "quit", "Quit", true, None::<&str>)
            .expect("Failed to create quit menu item");

        // Create the menu
        let menu = Menu::with_items(app, &[&quit_item])
            .expect("Failed to create menu");

        let _tray = TrayIconBuilder::new()
            .icon(app.default_window_icon().unwrap().clone())
            .menu(&menu)
            .menu_on_left_click(false)
            .on_tray_icon_event(|tray_handle, event| match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    .. 
                } => {
                    let app_handle = tray_handle.app_handle().clone();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        tauri::async_runtime::spawn(async move {
                            if let Ok(is_visible) = window.is_visible() {
                                if is_visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.unminimize();
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        });
                    }
                }
                _ => ()
            })
            .on_menu_event(|app, event| match event.id() {
                id if id == "quit" => {
                    app.exit(0);
                }
                _ => {}
            })
            .build(app)
            .expect("Failed to build tray icon");
    });
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            // Get main window handle
            let main_window = app.get_webview_window("main").unwrap();
            
            // Create separate clones for different uses
            let window_for_centering = main_window.clone();
            let window_for_events = main_window.clone();
            
            // Add programmatic window positioning and showing
            tauri::async_runtime::spawn(async move {
                std::thread::sleep(std::time::Duration::from_millis(100));
                window_for_centering.center().unwrap();
                window_for_centering.show().unwrap();
                window_for_centering.set_focus().unwrap();
            });
            
            main_window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window_for_events.hide();
                    api.prevent_close();
                }
            });

            create_tray(app)?;
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            search_event_logs, 
            get_available_logs,
            check_admin_rights  // Add this to the invoke handler
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
