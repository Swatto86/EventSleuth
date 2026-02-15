# EventSleuth

A fast, modern Windows Event Log viewer built with Rust and egui — a powerful alternative to the traditional Event Viewer.

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![License](https://img.shields.io/badge/license-MIT-green)

## Overview

EventSleuth provides a streamlined interface for viewing and analyzing Windows Event Logs. Built with performance in mind, it handles 100,000+ events smoothly using virtual scrolling and background-threaded queries.

## Features

- **Fast Event Queries** - Background-threaded log queries that don't block the UI
- **Virtual Scrolling** - Handle 100k+ events without performance degradation
- **Composable Filters** - Build complex filters with multiple criteria
  - Filter by event level (Information, Warning, Error, Critical)
  - Filter by source
  - Filter by event ID
  - Filter by time range
  - Search event messages
- **Export Options** - Export filtered results to:
  - CSV format
  - JSON format
- **Modern UI** - Clean, intuitive interface built with egui
- **Real-time Updates** - Monitor logs as events occur
- **Multiple Log Sources** - Access all Windows event log channels
  - Application
  - Security
  - System
  - Setup
  - Custom application logs

## Requirements

- Windows 10 or later
- Administrator privileges (for accessing certain event logs)

## Installation

### Download

Download the latest release from the [Releases](https://github.com/Swatto86/EventSleuth/releases) page.

### Build from Source

```bash
git clone https://github.com/Swatto86/EventSleuth.git
cd EventSleuth

# Build release version
cargo build --release

# Run
cargo run --release
```

## Usage

### Viewing Event Logs

1. Launch EventSleuth (run as Administrator for full access)
2. Select an event log channel from the dropdown
3. Events will load in the background
4. Use filters to narrow down results

### Filtering Events

- **By Level**: Select one or more severity levels (Info, Warning, Error, Critical)
- **By Source**: Filter by event source/provider
- **By Event ID**: Enter specific event IDs to display
- **By Time**: Set a date/time range for events
- **By Message**: Search within event descriptions

### Exporting Data

1. Apply your desired filters
2. Click the "Export" button
3. Choose CSV or JSON format
4. Select save location
5. Filtered events will be exported with all details

## Performance

EventSleuth is designed for performance:

- **Background Threading**: Event queries run in separate threads to keep the UI responsive
- **Virtual Scrolling**: Only visible events are rendered, allowing smooth scrolling through massive logs
- **Efficient Filtering**: Filters are applied efficiently without re-querying the entire log
- **Memory Efficient**: Uses Rust's ownership system to minimize memory usage

## Use Cases

- **System Troubleshooting** - Quickly find error events and their causes
- **Security Auditing** - Review security events and access patterns
- **Application Debugging** - Monitor application-specific event logs
- **Compliance Reporting** - Export filtered logs for compliance documentation
- **Performance Analysis** - Track system events over time
- **Incident Response** - Rapidly search and filter during investigations

## Technical Details

### Built With

- **Rust** - Systems programming language for performance and reliability
- **egui** - Immediate-mode GUI framework for native desktop apps
- **Windows Event Log API** - Direct Windows API integration for log access

### Architecture

EventSleuth uses a multi-threaded architecture:

```
┌─────────────┐
│   UI Thread │ ◄──── User interactions, rendering
└──────┬──────┘
       │
       ▼
┌──────────────────┐
│  Query Threads   │ ◄──── Background event log queries
└──────┬───────────┘
       │
       ▼
┌──────────────────┐
│ Windows Event    │
│ Log API          │
└──────────────────┘
```

Events are fetched in the background and streamed to the UI, allowing for responsive interaction even with large log files.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+F | Focus search filter |
| Ctrl+E | Toggle export dialog |
| Ctrl+R | Refresh current log |
| F5 | Refresh current log |
| Escape | Clear filters |

## Comparison to Event Viewer

| Feature | EventSleuth | Event Viewer |
|---------|-------------|--------------|
| Performance | Fast virtual scrolling | Slow with large logs |
| Filtering | Composable, multi-criteria | Limited filtering |
| Export | CSV, JSON | XML only |
| UI | Modern, responsive | Traditional, dated |
| Search | Full-text search | Basic find |
| Threading | Background queries | Blocks on large queries |

## Troubleshooting

### Access Denied Errors

Some event logs require administrator privileges. Run EventSleuth as Administrator:
- Right-click `EventSleuth.exe`
- Select "Run as administrator"

### Empty Event Lists

If no events appear:
- Verify the selected log channel exists
- Check that the Windows Event Log service is running
- Ensure you have permissions to access the selected log

### Performance Issues

For optimal performance:
- Use filters to reduce the number of displayed events
- Close other resource-intensive applications
- Ensure your system meets minimum requirements

## Development

### Prerequisites

- Rust 1.70 or later
- Windows 10 SDK
- Visual Studio Build Tools (for Windows API bindings)

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Project Structure

```
EventSleuth/
├── src/
│   ├── main.rs           # Application entry point
│   ├── ui/               # UI components
│   ├── eventlog/         # Windows Event Log API wrappers
│   ├── filters/          # Filtering logic
│   └── export/           # Export functionality
├── Cargo.toml            # Dependencies
└── README.md
```

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

**Swatto** - [GitHub](https://github.com/Swatto86) | [Website](https://swatto.co.uk)

## Acknowledgments

- Built with [egui](https://github.com/emilk/egui) - Excellent Rust GUI framework
- Inspired by Event Viewer and other system monitoring tools
- Thanks to the Rust community for excellent libraries and support

---

*Part of the [Swatto Tools](https://swatto.co.uk) collection*
