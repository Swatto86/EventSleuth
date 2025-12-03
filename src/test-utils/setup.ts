import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock Tauri API
export const mockInvoke = vi.fn();
export const mockSave = vi.fn();
export const mockWriteFile = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: mockSave,
}));

vi.mock('@tauri-apps/plugin-fs', () => ({
  writeFile: mockWriteFile,
}));

// Mock event log data
export const mockEventLogEntry = {
  source: 'Application',
  time_generated: '2024-01-15T10:30:00Z',
  event_id: 1000,
  event_type: 'Error',
  severity: 'Error',
  category: 0,
  message: 'Test error message',
  computer_name: 'TEST-PC',
  matches: ['error'],
};

export const mockEventLogEntryWarning = {
  source: 'System',
  time_generated: '2024-01-15T11:30:00Z',
  event_id: 2000,
  event_type: 'Warning',
  severity: 'Warning',
  category: 1,
  message: 'Test warning message',
  computer_name: 'TEST-PC',
  matches: ['warning'],
};

export const mockEventLogEntryCritical = {
  source: 'Security',
  time_generated: '2024-01-15T12:30:00Z',
  event_id: 3000,
  event_type: 'Critical',
  severity: 'Critical',
  category: 2,
  message: 'Test critical message',
  computer_name: 'TEST-PC',
  matches: ['critical'],
};

export const mockSearchParams = {
  keywords: ['error'],
  exclude_keywords: [],
  start_date: null,
  end_date: null,
  log_names: ['Application'],
  event_types: [2],
  event_ids: [],
  sources: [],
  categories: [],
  max_results: null,
};

// Helper to reset all mocks
export const resetAllMocks = () => {
  mockInvoke.mockReset();
  mockSave.mockReset();
  mockWriteFile.mockReset();
};

// Helper to setup successful search
export const setupSuccessfulSearch = (results = [mockEventLogEntry]) => {
  mockInvoke.mockResolvedValue(results);
};

// Helper to setup failed search
export const setupFailedSearch = (error = 'Search failed') => {
  mockInvoke.mockRejectedValue(new Error(error));
};

// Helper to setup admin check
export const setupAdminCheck = (isAdmin = true) => {
  mockInvoke.mockImplementation((command: string) => {
    if (command === 'check_admin_rights') {
      return Promise.resolve(isAdmin);
    }
    return Promise.resolve([]);
  });
};

// Helper to setup export functionality
export const setupExport = (filePath = 'C:/test/export.csv') => {
  mockSave.mockResolvedValue(filePath);
  mockWriteFile.mockResolvedValue(undefined);
};

// Helper to create test event with custom properties
export const createMockEvent = (overrides = {}) => ({
  ...mockEventLogEntry,
  ...overrides,
});

// Wait for async updates
export const waitForAsync = () => new Promise(resolve => setTimeout(resolve, 0));

// Global beforeEach to reset mocks
beforeEach(() => {
  resetAllMocks();
});
