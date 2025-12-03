# EventSleuth

Advanced Windows Event Log Analyzer with powerful search and filtering capabilities.

## Features

- **Comprehensive Search**: Search across all Windows Event Logs with keyword filtering
- **Advanced Filtering**:
  - Filter by severity levels (Critical, Error, Warning, Information, Verbose)
  - Date range filtering with precise start/end times
  - Event ID filtering
  - Source filtering
  - Exclude keywords
  - Category filtering
  - Maximum results limit
- **Keyword Highlighting**: Matched keywords are highlighted in search results
- **CSV Export**: Export search results to CSV format
- **Theme Support**: 8 built-in themes (Light, Dark, Cupcake, Synthwave, Cyberpunk, Retro, Night, Dracula)
- **Admin Detection**: Displays warning when not running with administrator privileges
- **System Tray Integration**: Minimize to system tray with quick show/hide functionality
- **Responsive UI**: Modern, clean interface built with React and Tailwind CSS

## Technology Stack

- **Frontend**: React 18, TypeScript, Vite, Tailwind CSS, DaisyUI
- **Backend**: Rust, Tauri 2.0, Windows Event Log API
- **Testing**: Vitest, @testing-library/react (102 tests)

## System Requirements

- Windows 10 or later
- Administrator privileges (recommended for full event log access)

## Installation

### Option 1: Using the Installer

1. Download `EventSleuth_1.0.0_x64-setup.exe` from the releases
2. Run the installer
3. Follow the installation wizard
4. Launch EventSleuth from the Start Menu

### Option 2: Build from Source

**Prerequisites:**
- Node.js 18+
- Rust 1.70+
- npm or yarn

**Steps:**

1. Clone the repository:
```bash
git clone https://github.com/yourusername/EventSleuth.git
cd EventSleuth
```

2. Install dependencies:
```bash
npm install
```

3. Run in development mode:
```bash
npm run tauri dev
```

4. Build for production:
```bash
npm run tauri build
```

The installer will be created in `src-tauri/target/release/bundle/nsis/`

## Usage

### Basic Search

1. Enter keywords in the search field (comma-separated for multiple keywords)
2. Optionally select a date range
3. Click event severity buttons to filter by severity level
4. Click "Search Events" to execute the search

### Advanced Filters

Click "Show Advanced Filters" to access:
- **Exclude Keywords**: Exclude events containing specific terms
- **Event IDs**: Filter by specific event IDs (comma-separated)
- **Sources**: Filter by event sources (comma-separated)
- **Maximum Results**: Limit the number of results returned (leave empty for unlimited)

### Exporting Results

1. After performing a search, click "Export to CSV"
2. Choose a location to save the file
3. The CSV will include: Time, Severity, Event ID, Source, and Message

### Changing Themes

Click the theme button (palette icon) in the bottom-right corner and select your preferred theme.

## Running Tests

### Frontend Tests
```bash
npm run test           # Run tests in watch mode
npm run test:run       # Run tests once
npm run test:coverage  # Run with coverage report
```

### Backend Tests
```bash
npm run test:backend          # Run Rust tests
npm run test:backend:verbose  # Run with output
```

### All Tests
```bash
npm run test:all  # Run both frontend and backend tests
```

## Development

### Project Structure
```
EventSleuth/
├── src/                    # React frontend source
│   ├── __tests__/         # Frontend test files
│   ├── test-utils/        # Test utilities and mocks
│   ├── App.tsx            # Main application component
│   ├── main.tsx           # Entry point
│   └── styles.css         # Global styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Main Rust library
│   │   ├── main.rs        # Entry point
│   │   └── tests.rs       # Rust tests
│   ├── icons/             # Application icons
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── package.json           # Node dependencies and scripts
└── vite.config.ts         # Vite configuration
```

### Key Commands

```bash
npm run dev              # Start Vite dev server
npm run build            # Build frontend
npm run tauri dev        # Run Tauri in development mode
npm run tauri build      # Build production installer
npm test                 # Run tests in watch mode
```

## Architecture

### Frontend Components

- **App**: Main application container with state management
- **SearchForm**: Search input and filter controls
- **EventList**: Display and export event search results

### Backend Functions

- `search_event_logs`: Main search function with comprehensive filtering
- `get_available_logs`: Enumerate all accessible Windows Event Logs
- `check_admin_rights`: Detect administrator privileges

### Event Filtering Logic

The application filters events in the following order for optimal performance:
1. Date range (before message extraction)
2. Event type/severity
3. Event IDs
4. Categories
5. Sources
6. Message extraction
7. Keyword matching
8. Exclude keywords

## Performance

- Optimized binary size through aggressive compiler optimizations
- Efficient filtering (date/type checks before expensive message extraction)
- LZMA compression for smaller installer size
- Link-Time Optimization (LTO) enabled

## Troubleshooting

### "Running without administrator privileges" warning

EventSleuth can run without admin rights, but some event logs (particularly Security logs) require administrator access. To access all logs:
1. Right-click EventSleuth
2. Select "Run as administrator"

### No events found

- Verify your search criteria aren't too restrictive
- Check that the date range encompasses the events you're looking for
- Try searching without keywords to see all events
- Ensure you have permissions to access the selected log sources

### Build errors

If you encounter build errors:
```bash
# Update dependencies
npm update
cargo update

# Clear build cache
npm run build --clean
cd src-tauri && cargo clean
```

## License

Copyright (c) 2025 Swatto. All rights reserved.

## Contributing

This is a private project. For bug reports or feature requests, please contact the author.

## Version History

### v1.0.0 (2025)
- Initial release
- Comprehensive Windows Event Log search and filtering
- CSV export functionality
- Multiple theme support
- System tray integration
- NSIS installer