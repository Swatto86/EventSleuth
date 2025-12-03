# EventSleuth Test Documentation

Complete test harness for the EventSleuth Windows Event Log analyzer application.

## Quick Start

### Setup
```bash
# Install dependencies
npm install

# Or use setup script
.\setup-tests.ps1
```

### Run Tests
```bash
# All tests
.\run-all-tests.ps1

# Frontend only
npm test

# Backend only
npm run test:backend

# With coverage
npm run test:coverage
```

## Test Structure

```
EventSleuth/
├── src/
│   ├── __tests__/
│   │   ├── App.test.tsx              # Main app (596 assertions)
│   │   ├── SearchForm.test.tsx       # Search form (536 assertions)
│   │   ├── EventList.test.tsx        # Event list (520 assertions)
│   │   └── Integration.test.tsx      # E2E workflows (508 assertions)
│   └── test-utils/
│       └── setup.ts                   # Test mocks and utilities
├── src-tauri/src/
│   ├── lib.rs                         # Rust tests (inline)
│   └── tests.rs                       # Additional Rust tests
├── vitest.config.ts                   # Vitest configuration
└── TEST_README.md                     # This file
```

## Test Coverage

### Frontend (TypeScript/React)
- **Files**: 4 test files
- **Test Cases**: 100+
- **Assertions**: 1,652+
- **Coverage Target**: 85-95%

Tests cover:
- Component rendering and interactions
- Search functionality (keywords, filters, dates)
- Event display and highlighting
- CSV export
- Theme switching
- Admin privilege detection
- Error handling
- Accessibility

### Backend (Rust)
- **Files**: 2 test modules
- **Test Functions**: 40+
- **Coverage Target**: 90-95%

Tests cover:
- Event log enumeration
- Event searching and filtering
- Keyword matching (including exclude)
- Date range filtering
- Event type/ID/source filtering
- Admin rights detection
- Performance with large datasets
- Concurrent operations
- Edge cases (special characters, Unicode, etc.)

## Available Commands

### Frontend Tests
```bash
npm test                    # Run all (watch mode)
npm run test:run           # Run once
npm run test:watch         # Watch mode
npm run test:coverage      # Generate coverage report
npm run test:ui            # Interactive UI
npm run test:app           # Test App.tsx only
npm run test:search        # Test SearchForm.tsx only
npm run test:events        # Test EventList.tsx only
```

### Backend Tests
```bash
cd src-tauri
cargo test                          # All tests
cargo test -- --nocapture           # With output
cargo test test_name                # Specific test
cargo test -- --test-threads=1      # Sequential execution
```

## What Gets Tested

### Search Features
✅ Keyword search (single/multiple)
✅ Exclude keywords
✅ Date range filtering
✅ Event type filtering (Critical, Error, Warning, Information, Verbose)
✅ Event ID filtering
✅ Source filtering
✅ Max results limiting
✅ Empty/unlimited searches

### Display Features
✅ Event list rendering
✅ Severity color coding
✅ Keyword highlighting
✅ Message formatting
✅ Loading/empty states
✅ Large result sets (100+ events)

### Export & UI
✅ CSV export with special characters
✅ Theme switching (8 themes)
✅ Admin warning banner
✅ Form validation
✅ Advanced options toggle

### Error Handling
✅ Search failures
✅ Invalid inputs
✅ Permission errors
✅ Recovery after errors

### Accessibility
✅ Keyboard navigation
✅ ARIA attributes
✅ Focus management

## Test Utilities

Located in `src/test-utils/setup.ts`:

```typescript
import {
  mockInvoke,           // Mock Tauri API
  mockSave,             // Mock save dialog
  mockWriteFile,        // Mock file write
  setupSuccessfulSearch,
  setupFailedSearch,
  setupAdminCheck,
  setupExport,
  createMockEvent,
  resetAllMocks
} from '../test-utils/setup';
```

### Example Usage
```typescript
import { setupSuccessfulSearch, createMockEvent } from '../test-utils/setup';

it('should display search results', async () => {
  const mockEvent = createMockEvent({ 
    message: 'Test error',
    severity: 'Error'
  });
  
  setupSuccessfulSearch([mockEvent]);
  render(<App />);
  
  await user.click(searchButton);
  
  expect(screen.getByText('Test error')).toBeInTheDocument();
});
```

## Troubleshooting

### Frontend Issues

**Tests timeout**
```typescript
// vitest.config.ts
test: { testTimeout: 10000 }
```

**Mocks not working**
- Ensure `setupFiles: ['./src/test-utils/setup.ts']` in vitest.config.ts
- Check imports from test-utils/setup

**Module not found**
```bash
rm -rf node_modules
npm install
```

### Backend Issues

**Permission denied**
- Run terminal as Administrator
- Some Windows Event Log tests require elevated privileges

**Tests fail on CI**
```yaml
runs-on: windows-latest  # Must use Windows runner
```

## Writing New Tests

### Frontend Template
```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

describe('MyComponent', () => {
  it('should do something', async () => {
    const user = userEvent.setup();
    render(<MyComponent />);
    
    await user.click(screen.getByRole('button'));
    
    expect(screen.getByText('Expected')).toBeInTheDocument();
  });
});
```

### Backend Template
```rust
#[tokio::test]
async fn test_my_function() {
    let params = create_test_params();
    let result = my_function(params).await;
    
    assert!(result.is_ok());
    if let Ok(data) = result {
        assert_eq!(data.field, expected_value);
    }
}
```

## Best Practices

1. ✅ Use descriptive test names
2. ✅ Follow AAA pattern (Arrange, Act, Assert)
3. ✅ Test one thing per test
4. ✅ Use mock utilities from setup.ts
5. ✅ Clean up with resetAllMocks()
6. ✅ Handle async with await/waitFor
7. ✅ Test edge cases and errors
8. ✅ Include accessibility tests

## CI/CD Integration

```yaml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm install
      - run: npm test -- --run
      - run: cd src-tauri && cargo test
```

## Test Statistics

- **Total Test Cases**: 150+
- **Total Assertions**: 2,160+
- **Execution Time**: 15-30 seconds
- **Code Coverage**: 85-95%

## Resources

- [Vitest Documentation](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/react)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tauri Testing](https://tauri.app/v1/guides/testing/)

## Maintenance Checklist

- [ ] All new features have tests
- [ ] Tests pass locally
- [ ] Coverage maintained above 85%
- [ ] Tests are independent
- [ ] Mock data used appropriately
- [ ] Edge cases covered
- [ ] Error scenarios tested
- [ ] Accessibility verified

---

For quick reference, run: `.\run-all-tests.ps1`
