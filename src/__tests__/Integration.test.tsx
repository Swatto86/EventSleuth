import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "../App";
import {
  mockInvoke,
  mockSave,
  mockWriteFile,
  createMockEvent,
  resetAllMocks,
} from "../test-utils/setup";

describe("Integration Tests - Complete User Workflows", () => {
  beforeEach(() => {
    resetAllMocks();
  });

  describe("Complete Search and Export Workflow", () => {
    it("should complete full search-to-export workflow", async () => {
      // Setup mocks
      const mockEvents = [
        createMockEvent({
          event_id: 1000,
          severity: "Error",
          message: "Application error occurred",
          matches: ["error"],
        }),
        createMockEvent({
          event_id: 1001,
          severity: "Error",
          message: "Another error event",
          matches: ["error"],
        }),
      ];

      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") return Promise.resolve(mockEvents);
        return Promise.resolve([]);
      });

      mockSave.mockResolvedValue("C:/test/export.csv");
      mockWriteFile.mockResolvedValue(undefined);

      const user = userEvent.setup();
      render(<App />);

      // Wait for app to load
      await waitFor(() => {
        expect(
          screen.getByPlaceholderText(/error.*warning.*failed/i),
        ).toBeInTheDocument();
      });

      // Step 1: Enter search criteria
      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error");

      // Step 2: Select event type
      const errorButton = screen.getByRole("button", { name: /^error$/i });
      await user.click(errorButton);

      // Step 3: Submit search
      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      // Step 4: Verify results displayed
      await waitFor(() => {
        expect(screen.getByText(/ID: 1000/i)).toBeInTheDocument();
        expect(screen.getByText(/ID: 1001/i)).toBeInTheDocument();
      });

      // Step 5: Export results
      const exportButton = screen.getByRole("button", {
        name: /export to csv/i,
      });
      await user.click(exportButton);

      // Step 6: Verify export called
      await waitFor(() => {
        expect(mockSave).toHaveBeenCalled();
        expect(mockWriteFile).toHaveBeenCalled();
      });

      // Step 7: Clear results
      const clearButton = screen.getByRole("button", {
        name: /clear results/i,
      });
      await user.click(clearButton);

      // Step 8: Verify cleared
      // Verify cleared
      await waitFor(() => {
        expect(screen.queryByText(/ID: 1000/i)).not.toBeInTheDocument();
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });
    });
  });

  describe("Advanced Search with Multiple Filters", () => {
    it("should handle complex search with all filters", async () => {
      const mockEvent = createMockEvent({
        event_id: 2000,
        severity: "Warning",
        message: "Service started successfully",
        source: "Application",
        matches: ["service"],
      });

      mockInvoke.mockImplementation((command, args) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          // Verify all parameters were passed
          const params = args?.params;
          expect(params).toHaveProperty("keywords");
          expect(params).toHaveProperty("exclude_keywords");
          expect(params).toHaveProperty("event_ids");
          expect(params).toHaveProperty("sources");
          expect(params).toHaveProperty("max_results");
          return Promise.resolve([mockEvent]);
        }
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      // Enter keywords
      await user.type(
        screen.getByPlaceholderText(/error.*warning.*failed/i),
        "service, started",
      );

      // Set date range
      const startDateInput = document.getElementById(
        "start-date",
      ) as HTMLInputElement;
      const endDateInput = document.getElementById(
        "end-date",
      ) as HTMLInputElement;
      await user.type(startDateInput, "2024-01-01T00:00");
      await user.type(endDateInput, "2024-12-31T23:59");

      // Select event types
      await user.click(screen.getByRole("button", { name: /warning/i }));
      await user.click(screen.getByRole("button", { name: /information/i }));

      // Show advanced options
      await user.click(screen.getByRole("button", { name: /advanced/i }));

      // Set advanced filters
      await user.type(
        screen.getByLabelText(/exclude keywords/i),
        "error, failed",
      );
      await user.type(screen.getByLabelText(/event ids/i), "2000, 3000, 4000");
      await user.type(screen.getByLabelText(/sources/i), "Application, System");
      await user.clear(screen.getByLabelText(/maximum results/i));
      await user.type(screen.getByLabelText(/maximum results/i), "50");

      // Submit search
      await user.click(screen.getByRole("button", { name: /search events/i }));

      // Verify results
      await waitFor(() => {
        expect(screen.getByText(/ID: 2000/i)).toBeInTheDocument();
      });

      // Verify search was called with correct parameters
      expect(mockInvoke).toHaveBeenCalledWith(
        "search_event_logs",
        expect.objectContaining({
          params: expect.objectContaining({
            keywords: expect.arrayContaining(["service", "started"]),
            exclude_keywords: expect.arrayContaining(["error", "failed"]),
            event_ids: expect.arrayContaining([2000, 3000, 4000]),
            sources: expect.arrayContaining(["Application", "System"]),
            max_results: 50,
          }),
        }),
      );
    });
  });

  describe("Theme Persistence During Operations", () => {
    it("should maintain theme selection throughout workflow", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          return Promise.resolve([
            createMockEvent({ message: "Test event", matches: ["test"] }),
          ]);
        }
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /theme/i }),
        ).toBeInTheDocument();
      });

      // Default theme should be dark
      expect(document.documentElement.getAttribute("data-theme")).toBe("dark");

      // Change to light theme
      await user.click(screen.getByRole("button", { name: /theme/i }));
      await user.click(screen.getByText("light"));

      await waitFor(() => {
        expect(document.documentElement.getAttribute("data-theme")).toBe(
          "light",
        );
      });

      // Perform search
      await user.type(
        screen.getByPlaceholderText(/error.*warning.*failed/i),
        "test",
      );
      await user.click(screen.getByRole("button", { name: /search events/i }));

      await waitFor(() => {
        expect(screen.getByText(/ID: 1000/i)).toBeInTheDocument();
      });

      // Theme should still be light
      expect(document.documentElement.getAttribute("data-theme")).toBe("light");

      // Change to another theme
      await user.click(screen.getByRole("button", { name: /theme/i }));
      await user.click(screen.getByText("cyberpunk"));

      await waitFor(() => {
        expect(document.documentElement.getAttribute("data-theme")).toBe(
          "cyberpunk",
        );
      });

      // Clear results
      await user.click(screen.getByRole("button", { name: /clear results/i }));

      // Theme should still be cyberpunk
      expect(document.documentElement.getAttribute("data-theme")).toBe(
        "cyberpunk",
      );
    });
  });

  describe("Error Recovery and User Feedback", () => {
    it("should recover gracefully from search errors", async () => {
      let callCount = 0;
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          callCount++;
          if (callCount === 1) {
            return Promise.reject(new Error("Network error"));
          }
          return Promise.resolve([
            createMockEvent({ message: "Success", matches: [] }),
          ]);
        }
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      const consoleError = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      // First search fails
      await user.click(screen.getByRole("button", { name: /search events/i }));

      await waitFor(() => {
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });

      // Second search succeeds
      await user.click(screen.getByRole("button", { name: /search events/i }));

      await waitFor(() => {
        expect(screen.getByText(/ID: 1000/i)).toBeInTheDocument();
      });

      consoleError.mockRestore();
    });
  });

  describe("Admin vs Non-Admin User Experience", () => {
    it("should show warning banner for non-admin users", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(false);
        return Promise.resolve([]);
      });

      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByText(/running without administrator privileges/i),
        ).toBeInTheDocument();
      });
    });

    it("should not show warning banner for admin users", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        return Promise.resolve([]);
      });

      render(<App />);

      await waitFor(() => {
        expect(screen.getByText(/EventSleuth/i)).toBeInTheDocument();
      });

      expect(
        screen.queryByText(/running without administrator privileges/i),
      ).not.toBeInTheDocument();
    });
  });

  describe("Multiple Search Refinements", () => {
    it("should allow refining search multiple times", async () => {
      let searchCount = 0;
      mockInvoke.mockImplementation((command, args) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          searchCount++;
          const keywords = args?.params?.keywords || [];

          if (keywords.includes("error")) {
            return Promise.resolve([
              createMockEvent({
                event_id: 1,
                message: "Error message",
                matches: ["error"],
              }),
            ]);
          }

          if (keywords.includes("warning")) {
            return Promise.resolve([
              createMockEvent({
                event_id: 2,
                severity: "Warning",
                message: "Warning message",
                matches: ["warning"],
              }),
            ]);
          }

          return Promise.resolve([]);
        }
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText(/error.*warning.*failed/i),
        ).toBeInTheDocument();
      });

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });

      // First search - error events
      await user.clear(keywordInput);
      await user.type(keywordInput, "error");
      await user.click(searchButton);

      await waitFor(() => {
        expect(screen.getByText(/ID: 1/i)).toBeInTheDocument();
      });

      // Second search - warning events
      await user.clear(keywordInput);
      await user.type(keywordInput, "warning");
      await user.click(searchButton);

      await waitFor(() => {
        expect(screen.getByText(/ID: 2/i)).toBeInTheDocument();
        expect(screen.queryByText(/ID: 1/i)).not.toBeInTheDocument();
      });

      // Third search - no keywords (all events)
      await user.clear(keywordInput);
      await user.click(searchButton);

      await waitFor(() => {
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });

      expect(searchCount).toBe(3);
    });
  });

  describe("CSV Export with Various Data", () => {
    it("should export events with special characters correctly", async () => {
      const mockEvents = [
        createMockEvent({
          message: 'Message with "quotes"',
          matches: [],
        }),
        createMockEvent({
          message: "Message with\nnewlines\nand\ttabs",
          matches: [],
        }),
        createMockEvent({
          message: "Message with, commas, everywhere",
          matches: [],
        }),
      ];

      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") return Promise.resolve(mockEvents);
        return Promise.resolve([]);
      });

      mockSave.mockResolvedValue("C:/test/special_chars.csv");
      mockWriteFile.mockResolvedValue(undefined);

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      // Perform search
      await user.click(screen.getByRole("button", { name: /search events/i }));

      await waitFor(() => {
        expect(screen.getByText(/Message with "quotes"/i)).toBeInTheDocument();
      });

      // Export
      await user.click(screen.getByRole("button", { name: /export to csv/i }));

      await waitFor(() => {
        expect(mockWriteFile).toHaveBeenCalled();
        const writtenData = mockWriteFile.mock.calls[0][1];
        const csvContent = new TextDecoder().decode(writtenData);

        // Verify CSV formatting with new fields
        expect(csvContent).toContain(
          "Time,Log Name,Source,Event ID,Event Type,Severity,Category,Record Number,Computer,Message",
        );
        expect(csvContent).toContain('""'); // Escaped quotes
        expect(csvContent).toMatch(/"[^"]*"/); // Quoted fields
      });
    });
  });

  describe("Accessibility and Keyboard Navigation", () => {
    it("should be navigable with keyboard", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      // Tab through form elements - main content should come before theme button
      await user.tab();
      expect(
        screen.getByPlaceholderText(/error.*warning.*failed/i),
      ).toHaveFocus();

      await user.tab();
      await user.tab(); // Move through date inputs

      // Event type buttons should be reachable
      await user.tab();

      // Should be able to activate with keyboard
      await user.keyboard("{Enter}");
    });
  });

  describe("Performance with Large Result Sets", () => {
    it("should handle rendering 100+ events efficiently", async () => {
      const largeEventSet = Array.from({ length: 100 }, (_, i) =>
        createMockEvent({
          event_id: i,
          message: `Event number ${i}`,
          matches: [],
        }),
      );

      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve(largeEventSet);
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      const startTime = performance.now();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /search events/i }));

      await waitFor(
        () => {
          expect(screen.getByText(/Event number 0/i)).toBeInTheDocument();
        },
        { timeout: 5000 },
      );

      const endTime = performance.now();
      const renderTime = endTime - startTime;

      // Should render reasonably fast (less than 5 seconds)
      expect(renderTime).toBeLessThan(5000);

      // Verify all events are accessible
      const events = screen.getAllByText(/Event number \d+/);
      expect(events.length).toBe(100);
    });
  });

  describe("Detailed Event Information Workflow", () => {
    it("should display all detailed event information after search", async () => {
      const mockEvent = createMockEvent({
        log_name: "Application",
        source: "TestApp",
        event_id: 5678,
        event_type: "Warning",
        severity: "Warning",
        category: 10,
        record_number: 123456,
        computer_name: "WORKSTATION-01",
        message: "Test warning message",
        matches: ["warning"],
      });

      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve([mockEvent]);
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      // Verify all detailed fields are displayed
      await waitFor(() => {
        expect(screen.getByText(/Log: Application/i)).toBeInTheDocument();
        expect(screen.getByText(/Source: TestApp/i)).toBeInTheDocument();
        expect(screen.getByText(/ID: 5678/i)).toBeInTheDocument();
        expect(screen.getByText(/Type: Warning/i)).toBeInTheDocument();
        expect(screen.getByText(/Category: 10/i)).toBeInTheDocument();
        expect(screen.getByText(/Record: 123456/i)).toBeInTheDocument();
        expect(
          screen.getByText(/Computer: WORKSTATION-01/i),
        ).toBeInTheDocument();
      });
    });

    it("should handle Event Viewer button interaction", async () => {
      const mockEvent = createMockEvent({
        log_name: "System",
        event_id: 9999,
      });

      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve([mockEvent]);
        if (command === "open_event_in_viewer") return Promise.resolve();
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /view/i }),
        ).toBeInTheDocument();
      });

      const viewButton = screen.getByRole("button", { name: /view/i });
      await user.click(viewButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("open_event_in_viewer", {
          logName: "System",
          eventId: 9999,
        });
      });
    });

    it("should export detailed event information to CSV", async () => {
      const mockEvent = createMockEvent({
        log_name: "Security",
        source: "SecurityApp",
        event_id: 4624,
        event_type: "Information",
        severity: "Information",
        category: 12544,
        record_number: 987654,
        computer_name: "SERVER-01",
        message: "An account was successfully logged on",
      });

      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve([mockEvent]);
        return Promise.resolve([]);
      });

      mockSave.mockResolvedValue("C:/test/detailed_export.csv");
      mockWriteFile.mockResolvedValue(undefined);

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /export to csv/i }),
        ).toBeInTheDocument();
      });

      const exportButton = screen.getByRole("button", {
        name: /export to csv/i,
      });
      await user.click(exportButton);

      await waitFor(() => {
        expect(mockWriteFile).toHaveBeenCalled();
      });

      // Verify CSV contains all new fields
      const csvData = new TextDecoder().decode(
        mockWriteFile.mock.calls[0][1] as Uint8Array,
      );

      expect(csvData).toContain("Log Name");
      expect(csvData).toContain("Event Type");
      expect(csvData).toContain("Category");
      expect(csvData).toContain("Record Number");
      expect(csvData).toContain("Security");
      expect(csvData).toContain("SecurityApp");
      expect(csvData).toContain("Information");
      expect(csvData).toContain("12544");
      expect(csvData).toContain("987654");
      expect(csvData).toContain("SERVER-01");
    });

    it("should display detailed information for multiple events", async () => {
      const mockEvents = [
        createMockEvent({
          log_name: "Application",
          source: "App1",
          event_id: 1000,
          event_type: "Error",
          category: 1,
          record_number: 100001,
        }),
        createMockEvent({
          log_name: "System",
          source: "Sys1",
          event_id: 2000,
          event_type: "Warning",
          category: 2,
          record_number: 100002,
        }),
        createMockEvent({
          log_name: "Security",
          source: "Sec1",
          event_id: 3000,
          event_type: "Information",
          category: 3,
          record_number: 100003,
        }),
      ];

      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") return Promise.resolve(mockEvents);
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        // Verify each event shows its detailed information
        expect(screen.getByText(/Log: Application/i)).toBeInTheDocument();
        expect(screen.getByText(/Source: App1/i)).toBeInTheDocument();
        expect(screen.getByText(/Record: 100001/i)).toBeInTheDocument();

        expect(screen.getByText(/Log: System/i)).toBeInTheDocument();
        expect(screen.getByText(/Source: Sys1/i)).toBeInTheDocument();
        expect(screen.getByText(/Record: 100002/i)).toBeInTheDocument();

        expect(screen.getByText(/Log: Security/i)).toBeInTheDocument();
        expect(screen.getByText(/Source: Sec1/i)).toBeInTheDocument();
        expect(screen.getByText(/Record: 100003/i)).toBeInTheDocument();

        // Verify each event has a View button
        const viewButtons = screen.getAllByRole("button", { name: /view/i });
        expect(viewButtons).toHaveLength(3);
      });
    });
  });
});
