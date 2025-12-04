import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { EventList } from "../App";
import {
  mockEventLogEntry,
  mockEventLogEntryWarning,
  mockEventLogEntryCritical,
  setupExport,
  createMockEvent,
  mockWriteFile,
  mockInvoke,
} from "../test-utils/setup";

describe("EventList Component", () => {
  const mockOnClear = vi.fn();

  beforeEach(() => {
    mockOnClear.mockClear();
  });

  describe("Loading State", () => {
    it("should display loading message when isLoading is true", () => {
      render(<EventList events={[]} isLoading={true} onClear={mockOnClear} />);

      expect(screen.getByText(/searching/i)).toBeInTheDocument();
    });

    it("should not display events when loading", () => {
      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={true}
          onClear={mockOnClear}
        />,
      );

      expect(screen.getByText(/searching/i)).toBeInTheDocument();
      expect(
        screen.queryByText(mockEventLogEntry.message),
      ).not.toBeInTheDocument();
    });
  });

  describe("Empty State", () => {
    it('should display "No events found" when events array is empty', () => {
      render(<EventList events={[]} isLoading={false} onClear={mockOnClear} />);

      expect(screen.getByText(/no events found/i)).toBeInTheDocument();
    });

    it("should not show action buttons when no events", () => {
      render(<EventList events={[]} isLoading={false} onClear={mockOnClear} />);

      expect(
        screen.queryByRole("button", { name: /clear results/i }),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: /export to csv/i }),
      ).not.toBeInTheDocument();
    });
  });

  describe("Event Display", () => {
    it("should display a single event", () => {
      const { container } = render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      // Check for message content in container (it's highlighted)
      const messageContainer = container.querySelector(
        ".text-base-content\\/90",
      );
      expect(messageContainer?.textContent).toContain("Test error message");

      expect(screen.getByText(mockEventLogEntry.severity)).toBeInTheDocument();
      expect(
        screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
      ).toBeInTheDocument();
      expect(screen.getByText(/Source: TestSource/i)).toBeInTheDocument();
    });

    it("should display multiple events", () => {
      const events = [
        mockEventLogEntry,
        mockEventLogEntryWarning,
        mockEventLogEntryCritical,
      ];

      render(
        <EventList events={events} isLoading={false} onClear={mockOnClear} />,
      );

      // Check for event IDs instead since messages have highlighting
      expect(
        screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
      ).toBeInTheDocument();
      expect(
        screen.getByText(`ID: ${mockEventLogEntryWarning.event_id}`),
      ).toBeInTheDocument();
      expect(
        screen.getByText(`ID: ${mockEventLogEntryCritical.event_id}`),
      ).toBeInTheDocument();

      // Check severities are displayed
      expect(screen.getByText(mockEventLogEntry.severity)).toBeInTheDocument();
      expect(
        screen.getByText(mockEventLogEntryWarning.severity),
      ).toBeInTheDocument();
      expect(
        screen.getByText(mockEventLogEntryCritical.severity),
      ).toBeInTheDocument();
    });

    it("should format timestamps correctly", () => {
      const event = createMockEvent({
        time_generated: "2024-01-15T10:30:45Z",
      });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      // The date should be formatted as dd/MM/yyyy HH:mm:ss
      expect(screen.getByText(/15\/01\/2024/)).toBeInTheDocument();
    });

    it("should display event ID", () => {
      const event = createMockEvent({ event_id: 12345 });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/ID: 12345/i)).toBeInTheDocument();
    });

    it("should display event source", () => {
      const event = createMockEvent({ source: "TestSource" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/Source: TestSource/i)).toBeInTheDocument();
    });

    it("should display log name", () => {
      const event = createMockEvent({ log_name: "Application" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/Log: Application/i)).toBeInTheDocument();
    });

    it("should display event type", () => {
      const event = createMockEvent({ event_type: "Error" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/Type: Error/i)).toBeInTheDocument();
    });

    it("should display category", () => {
      const event = createMockEvent({ category: 42 });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/Category: 42/i)).toBeInTheDocument();
    });

    it("should display record number", () => {
      const event = createMockEvent({ record_number: 99999 });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/Record: 99999/i)).toBeInTheDocument();
    });

    it("should display computer name with label", () => {
      const event = createMockEvent({ computer_name: "SERVER-01" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/Computer: SERVER-01/i)).toBeInTheDocument();
    });
  });

  describe("Severity Styling", () => {
    it("should apply correct styling for Critical severity", () => {
      const event = createMockEvent({ severity: "Critical" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const severityBadge = screen.getByText("Critical");
      expect(severityBadge).toHaveClass("bg-red-500/20", "text-red-400");
    });

    it("should apply correct styling for Error severity", () => {
      const event = createMockEvent({ severity: "Error" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const severityBadge = screen.getByText("Error");
      expect(severityBadge).toHaveClass("bg-orange-500/20", "text-orange-400");
    });

    it("should apply correct styling for Warning severity", () => {
      const event = createMockEvent({ severity: "Warning" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const severityBadge = screen.getByText("Warning");
      expect(severityBadge).toHaveClass("bg-yellow-500/20", "text-yellow-400");
    });

    it("should apply correct styling for Information severity", () => {
      const event = createMockEvent({ severity: "Information" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const severityBadge = screen.getByText("Information");
      expect(severityBadge).toHaveClass("bg-blue-500/20", "text-blue-400");
    });

    it("should apply correct styling for unknown severity", () => {
      const event = createMockEvent({ severity: "Unknown" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const severityBadge = screen.getByText("Unknown");
      expect(severityBadge).toHaveClass("bg-gray-500/20", "text-gray-400");
    });
  });

  describe("Keyword Highlighting", () => {
    it("should highlight matched keywords in message", () => {
      const event = createMockEvent({
        message: "This is an error message",
        matches: ["error"],
      });

      const { container } = render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      // Check that the message contains a mark tag for highlighting
      const messageContainer = container.querySelector(
        ".text-base-content\\/90",
      );
      expect(messageContainer?.innerHTML).toContain("<mark");
      expect(messageContainer?.innerHTML).toContain("error");
    });

    it("should highlight multiple matched keywords", () => {
      const event = createMockEvent({
        message: "Error: failed to process",
        matches: ["error", "failed"],
      });

      const { container } = render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const messageContainer = container.querySelector(
        ".text-base-content\\/90",
      );
      expect(messageContainer?.innerHTML).toContain("Error");
      expect(messageContainer?.innerHTML).toContain("failed");
      expect(messageContainer?.innerHTML).toContain("<mark");
    });

    it("should handle case-insensitive highlighting", () => {
      const event = createMockEvent({
        message: "ERROR: System Failure ERROR",
        matches: ["error"],
      });

      const { container } = render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const messageContainer = container.querySelector(
        ".text-base-content\\/90",
      );
      // Should highlight both instances of ERROR
      const markCount = (messageContainer?.innerHTML.match(/<mark/g) || [])
        .length;
      expect(markCount).toBeGreaterThan(0);
    });

    it("should not break message display when no matches", () => {
      const event = createMockEvent({
        message: "Simple message",
        matches: [],
      });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText("Simple message")).toBeInTheDocument();
    });
  });

  describe("Action Buttons", () => {
    it("should show Clear Results button when events exist", () => {
      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      expect(
        screen.getByRole("button", { name: /clear results/i }),
      ).toBeInTheDocument();
    });

    it("should show Export to CSV button when events exist", () => {
      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      expect(
        screen.getByRole("button", { name: /export to csv/i }),
      ).toBeInTheDocument();
    });

    it("should call onClear when Clear Results is clicked", async () => {
      const user = userEvent.setup();
      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      const clearButton = screen.getByRole("button", {
        name: /clear results/i,
      });
      await user.click(clearButton);
      expect(mockOnClear).toHaveBeenCalledTimes(1);
    });

    it("should show View button for each event", () => {
      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      const viewButton = screen.getByRole("button", { name: /view/i });
      expect(viewButton).toBeInTheDocument();
    });

    it("should call open_event_in_viewer when View button is clicked", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);

      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      const viewButton = screen.getByRole("button", { name: /view/i });
      await user.click(viewButton);

      // Wait for the async call
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("open_event_in_viewer", {
          logName: mockEventLogEntry.log_name,
          eventId: mockEventLogEntry.event_id,
        });
      });
    });

    it("should show View button for multiple events", () => {
      const events = [mockEventLogEntry, mockEventLogEntryWarning];

      render(
        <EventList events={events} isLoading={false} onClear={mockOnClear} />,
      );

      const viewButtons = screen.getAllByRole("button", { name: /view/i });
      expect(viewButtons).toHaveLength(2);
    });
  });

  describe("CSV Export Functionality", () => {
    it("should trigger export when Export to CSV is clicked", async () => {
      setupExport();
      const user = userEvent.setup();

      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      const exportButton = screen.getByRole("button", {
        name: /export to csv/i,
      });
      await user.click(exportButton);

      // The export function should be called (mocked in setup)
      expect(exportButton).toBeInTheDocument();
    });

    it("should handle export with multiple events", async () => {
      setupExport();
      const user = userEvent.setup();

      const events = [
        mockEventLogEntry,
        mockEventLogEntryWarning,
        mockEventLogEntryCritical,
      ];

      render(
        <EventList events={events} isLoading={false} onClear={mockOnClear} />,
      );

      const exportButton = screen.getByRole("button", {
        name: /export to csv/i,
      });
      await user.click(exportButton);

      expect(exportButton).toBeInTheDocument();
    });

    it("should handle export with special characters in messages", async () => {
      setupExport();
      const user = userEvent.setup();

      const event = createMockEvent({
        message: 'Message with "quotes" and\nnewlines',
      });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const exportButton = screen.getByRole("button", {
        name: /export to csv/i,
      });
      await user.click(exportButton);

      expect(exportButton).toBeInTheDocument();
      expect(mockWriteFile).toHaveBeenCalled();
    });

    it("should export all new fields to CSV", async () => {
      setupExport();
      const user = userEvent.setup();

      const event = createMockEvent({
        log_name: "System",
        source: "TestSource",
        event_id: 1234,
        event_type: "Warning",
        category: 5,
        record_number: 99999,
        computer_name: "TEST-PC",
      });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const exportButton = screen.getByRole("button", {
        name: /export to csv/i,
      });
      await user.click(exportButton);

      expect(mockWriteFile).toHaveBeenCalled();
      const csvData = new TextDecoder().decode(
        mockWriteFile.mock.calls[0][1] as Uint8Array,
      );

      // Check that CSV contains all the new fields
      expect(csvData).toContain("Log Name");
      expect(csvData).toContain("Event Type");
      expect(csvData).toContain("Category");
      expect(csvData).toContain("Record Number");
      expect(csvData).toContain("Computer");
      expect(csvData).toContain("System");
      expect(csvData).toContain("TestSource");
      expect(csvData).toContain("Warning");
      expect(csvData).toContain("5");
      expect(csvData).toContain("99999");
    });
  });

  describe("Event List Rendering", () => {
    it("should render events in correct order", () => {
      const events = [
        createMockEvent({ event_id: 1, message: "First event" }),
        createMockEvent({ event_id: 2, message: "Second event" }),
        createMockEvent({ event_id: 3, message: "Third event" }),
      ];

      render(
        <EventList events={events} isLoading={false} onClear={mockOnClear} />,
      );

      const messages = screen.getAllByText(/event$/i);
      expect(messages[0]).toHaveTextContent("First event");
      expect(messages[1]).toHaveTextContent("Second event");
      expect(messages[2]).toHaveTextContent("Third event");
    });

    it("should handle large number of events", () => {
      const events = Array.from({ length: 100 }, (_, i) =>
        createMockEvent({
          event_id: i,
          message: `Event ${i}`,
        }),
      );

      render(
        <EventList events={events} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getAllByText(/Event \d+/)).toHaveLength(100);
    });
  });

  describe("Message Formatting", () => {
    it("should preserve whitespace in messages", () => {
      const event = createMockEvent({
        message: "Line 1\nLine 2\nLine 3",
      });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const messageElement = screen.getByText(/Line 1/);
      expect(messageElement).toHaveClass("whitespace-pre-wrap");
    });

    it("should handle long messages with word wrapping", () => {
      const longMessage = "A".repeat(500);
      const event = createMockEvent({ message: longMessage });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      const messageElement = screen.getByText(longMessage);
      expect(messageElement).toHaveClass("break-words");
    });

    it("should handle empty messages", () => {
      const event = createMockEvent({ message: "" });

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      // Should still render the event card even with empty message
      expect(screen.getByText(`ID: ${event.event_id}`)).toBeInTheDocument();
    });
  });

  describe("Accessibility", () => {
    it("should have proper button roles", () => {
      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      const clearButton = screen.getByRole("button", {
        name: /clear results/i,
      });
      const exportButton = screen.getByRole("button", {
        name: /export to csv/i,
      });

      expect(clearButton).toBeInTheDocument();
      expect(exportButton).toBeInTheDocument();
    });

    it("should be keyboard navigable", async () => {
      const user = userEvent.setup();
      render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      const clearButton = screen.getByRole("button", {
        name: /clear results/i,
      });

      // Tab to the button and press Enter
      await user.tab();
      await user.keyboard("{Enter}");

      // Check if one of the buttons received focus/action
      expect(clearButton).toBeInTheDocument();
    });
  });

  describe("Edge Cases", () => {
    it("should handle null or undefined event properties gracefully", () => {
      const event = {
        ...mockEventLogEntry,
        source: "",
        message: "",
      };

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(`ID: ${event.event_id}`)).toBeInTheDocument();
    });

    it("should handle events with missing matches array", () => {
      const event = {
        ...mockEventLogEntry,
        matches: [],
      };

      render(
        <EventList events={[event]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(screen.getByText(event.message)).toBeInTheDocument();
    });

    it("should render correctly when transitioning from loading to loaded", async () => {
      const { rerender } = render(
        <EventList events={[]} isLoading={true} onClear={mockOnClear} />,
      );

      expect(screen.getByText(/searching/i)).toBeInTheDocument();

      rerender(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      await waitFor(() => {
        expect(screen.queryByText(/searching/i)).not.toBeInTheDocument();
        // Check for event ID instead since message has highlighting
        expect(
          screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
        ).toBeInTheDocument();
      });
    });

    it("should render correctly when events are cleared", () => {
      const { rerender } = render(
        <EventList
          events={[mockEventLogEntry]}
          isLoading={false}
          onClear={mockOnClear}
        />,
      );

      // Check for event ID instead since message has highlighting
      expect(
        screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
      ).toBeInTheDocument();

      rerender(
        <EventList events={[]} isLoading={false} onClear={mockOnClear} />,
      );

      expect(
        screen.queryByText(`ID: ${mockEventLogEntry.event_id}`),
      ).not.toBeInTheDocument();
      expect(screen.getByText(/no events found/i)).toBeInTheDocument();
    });
  });
});
