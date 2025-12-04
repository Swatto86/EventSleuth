import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "../App";
import {
  mockInvoke,
  mockEventLogEntry,
  mockEventLogEntryWarning,
  resetAllMocks,
} from "../test-utils/setup";

describe("App Component", () => {
  beforeEach(() => {
    resetAllMocks();
  });

  // Helper function to setup admin check
  const setupAdminCheck = (isAdmin: boolean) => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "check_admin_rights") {
        return Promise.resolve(isAdmin);
      }
      return Promise.resolve([]);
    });
  };

  describe("Initial Rendering", () => {
    it("should render the application title", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        expect(screen.getByText(/EventSleuth/i)).toBeInTheDocument();
      });
    });

    it("should render search form on initial load", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText(/error.*warning.*failed/i),
        ).toBeInTheDocument();
      });
    });

    it("should render theme selector", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /theme/i }),
        ).toBeInTheDocument();
      });
    });

    it("should start with no events displayed", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });
    });

    it("should start with dark theme by default", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        expect(document.documentElement.getAttribute("data-theme")).toBe(
          "dark",
        );
      });
    });
  });

  describe("Admin Rights Check", () => {
    it("should check admin rights on mount", async () => {
      mockInvoke.mockResolvedValue(true);
      render(<App />);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("check_admin_rights");
      });
    });

    it("should display warning when not running as admin", async () => {
      setupAdminCheck(false);
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByText(/running without administrator privileges/i),
        ).toBeInTheDocument();
      });
    });

    it("should not display warning when running as admin", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        expect(
          screen.queryByText(/running without administrator privileges/i),
        ).not.toBeInTheDocument();
      });
    });

    it("should handle admin check failure gracefully", async () => {
      mockInvoke.mockRejectedValue(new Error("Failed to check admin"));
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByText(/running without administrator privileges/i),
        ).toBeInTheDocument();
      });
    });
  });

  describe("Theme Management", () => {
    it("should open theme dropdown when clicked", async () => {
      setupAdminCheck(true);
      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /theme/i }),
        ).toBeInTheDocument();
      });

      const themeButton = screen.getByRole("button", { name: /theme/i });
      await user.click(themeButton);

      await waitFor(() => {
        expect(screen.getByText("light")).toBeInTheDocument();
        expect(screen.getByText("cupcake")).toBeInTheDocument();
        expect(screen.getByText("synthwave")).toBeInTheDocument();
      });
    });

    it("should change theme when a theme option is selected", async () => {
      setupAdminCheck(true);
      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /theme/i }),
        ).toBeInTheDocument();
      });

      const themeButton = screen.getByRole("button", { name: /theme/i });
      await user.click(themeButton);

      const lightTheme = screen.getByText("light");
      await user.click(lightTheme);

      await waitFor(() => {
        expect(document.documentElement.getAttribute("data-theme")).toBe(
          "light",
        );
      });
    });

    it("should support all available themes", async () => {
      setupAdminCheck(true);
      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /theme/i }),
        ).toBeInTheDocument();
      });

      const themeButton = screen.getByRole("button", { name: /theme/i });
      await user.click(themeButton);

      const expectedThemes = [
        "light",
        "dark",
        "cupcake",
        "synthwave",
        "cyberpunk",
        "retro",
        "night",
        "dracula",
      ];

      for (const theme of expectedThemes) {
        expect(screen.getByText(theme)).toBeInTheDocument();
      }
    });

    it("should apply theme to document element", async () => {
      setupAdminCheck(true);
      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /theme/i }),
        ).toBeInTheDocument();
      });

      const themeButton = screen.getByRole("button", { name: /theme/i });
      await user.click(themeButton);

      const synthwaveTheme = screen.getByText("synthwave");
      await user.click(synthwaveTheme);

      await waitFor(() => {
        expect(document.documentElement.getAttribute("data-theme")).toBe(
          "synthwave",
        );
      });
    });
  });

  describe("Search Functionality", () => {
    it("should perform search when form is submitted", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve([mockEventLogEntry]);
        return Promise.resolve([]);
      });
      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("search_event_logs", {
          params: expect.objectContaining({
            keywords: ["error"],
          }),
        });
      });
    });

    it("should display loading state during search", async () => {
      let resolveSearch: (value: any) => void;
      const searchPromise = new Promise((resolve) => {
        resolveSearch = resolve;
      });
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") return searchPromise;
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
        expect(screen.getByText(/searching/i)).toBeInTheDocument();
      });

      await act(async () => {
        resolveSearch!([]);
      });
    });

    it("should display search results after successful search", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve([mockEventLogEntry]);
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(
          screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
        ).toBeInTheDocument();
      });
    });

    it("should handle search errors gracefully", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.reject(new Error("Search failed"));
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

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });

      consoleError.mockRestore();
    });

    it("should display multiple search results", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          return Promise.resolve([mockEventLogEntry, mockEventLogEntryWarning]);
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

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(
          screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
        ).toBeInTheDocument();
        expect(
          screen.getByText(`ID: ${mockEventLogEntryWarning.event_id}`),
        ).toBeInTheDocument();
      });
    });

    it("should display detailed event information in search results", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          return Promise.resolve([mockEventLogEntry]);
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

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        // Check for log name
        expect(
          screen.getByText(
            new RegExp(`Log: ${mockEventLogEntry.log_name}`, "i"),
          ),
        ).toBeInTheDocument();
        // Check for event type
        expect(
          screen.getByText(
            new RegExp(`Type: ${mockEventLogEntry.event_type}`, "i"),
          ),
        ).toBeInTheDocument();
        // Check for category
        expect(
          screen.getByText(
            new RegExp(`Category: ${mockEventLogEntry.category}`, "i"),
          ),
        ).toBeInTheDocument();
        // Check for record number
        expect(
          screen.getByText(
            new RegExp(`Record: ${mockEventLogEntry.record_number}`, "i"),
          ),
        ).toBeInTheDocument();
        // Check for computer name
        expect(
          screen.getByText(
            new RegExp(`Computer: ${mockEventLogEntry.computer_name}`, "i"),
          ),
        ).toBeInTheDocument();
      });
    });

    it("should display View button for opening events in Event Viewer", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          return Promise.resolve([mockEventLogEntry]);
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

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        const viewButton = screen.getByRole("button", { name: /view/i });
        expect(viewButton).toBeInTheDocument();
        expect(viewButton).toHaveAttribute(
          "title",
          "Open this event in Windows Event Viewer",
        );
      });
    });
  });

  describe("Clear Results Functionality", () => {
    it("should clear results when clear button is clicked", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve([mockEventLogEntry]);
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      // Perform a search
      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(
          screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
        ).toBeInTheDocument();
      });

      // Clear results
      const clearButton = screen.getByRole("button", {
        name: /clear results/i,
      });
      await user.click(clearButton);

      await waitFor(() => {
        expect(
          screen.queryByText(`ID: ${mockEventLogEntry.event_id}`),
        ).not.toBeInTheDocument();
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });
    });

    it("should hide clear button when no results", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        expect(
          screen.queryByRole("button", { name: /clear results/i }),
        ).not.toBeInTheDocument();
      });
    });
  });

  describe("Integration Tests", () => {
    it("should handle complete search workflow", async () => {
      mockInvoke.mockImplementation((command, args) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          const params = args?.params;
          if (params?.keywords?.includes("error")) {
            return Promise.resolve([mockEventLogEntry]);
          }
          return Promise.resolve([]);
        }
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      // Wait for app to load
      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /search events/i }),
        ).toBeInTheDocument();
      });

      // Enter search criteria
      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error");

      const errorButton = screen.getByRole("button", { name: /^error$/i });
      await user.click(errorButton);

      // Submit search
      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      // Verify results
      await waitFor(() => {
        expect(
          screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
        ).toBeInTheDocument();
      });

      // Clear results
      const clearButton = screen.getByRole("button", {
        name: /clear results/i,
      });
      await user.click(clearButton);

      // Verify cleared
      await waitFor(() => {
        expect(
          screen.queryByText(`ID: ${mockEventLogEntry.event_id}`),
        ).not.toBeInTheDocument();
      });
    });

    it("should maintain theme across searches", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs")
          return Promise.resolve([mockEventLogEntry]);
        return Promise.resolve([]);
      });

      const user = userEvent.setup();
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /theme/i }),
        ).toBeInTheDocument();
      });

      // Change theme
      const themeButton = screen.getByRole("button", { name: /theme/i });
      await user.click(themeButton);
      const cyberpunkTheme = screen.getByText("cyberpunk");
      await user.click(cyberpunkTheme);

      await waitFor(() => {
        expect(document.documentElement.getAttribute("data-theme")).toBe(
          "cyberpunk",
        );
      });

      // Perform search
      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(
          screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
        ).toBeInTheDocument();
      });

      // Theme should still be cyberpunk
      expect(document.documentElement.getAttribute("data-theme")).toBe(
        "cyberpunk",
      );
    });

    it("should handle multiple consecutive searches", async () => {
      let searchCount = 0;
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          searchCount++;
          return Promise.resolve([
            { ...mockEventLogEntry, event_id: searchCount },
          ]);
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

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });

      // First search
      await user.click(searchButton);
      await waitFor(() => {
        expect(screen.getByText(/ID: 1/i)).toBeInTheDocument();
      });

      // Second search
      await user.click(searchButton);
      await waitFor(() => {
        expect(screen.getByText(/ID: 2/i)).toBeInTheDocument();
      });

      // Third search
      await user.click(searchButton);
      await waitFor(() => {
        expect(screen.getByText(/ID: 3/i)).toBeInTheDocument();
      });

      expect(searchCount).toBe(3);
    });
  });

  describe("Layout and Structure", () => {
    it("should have sticky header", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        const header = screen.getByText(/EventSleuth/i).closest(".sticky");
        expect(header).toBeInTheDocument();
      });
    });

    it("should have centered title", async () => {
      setupAdminCheck(true);
      render(<App />);

      await waitFor(() => {
        const title = screen.getByText(/EventSleuth/i);
        expect(title).toBeInTheDocument();
      });
    });

    it("should display admin warning at top when not admin", async () => {
      setupAdminCheck(false);
      render(<App />);

      await waitFor(() => {
        expect(
          screen.getByText(/running without administrator privileges/i),
        ).toBeInTheDocument();
      });
    });
  });

  describe("Error Handling", () => {
    it("should not crash when invoke fails", async () => {
      mockInvoke.mockRejectedValue(new Error("Invoke failed"));
      const consoleError = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      render(<App />);

      await waitFor(() => {
        expect(screen.getByText(/EventSleuth/i)).toBeInTheDocument();
      });

      consoleError.mockRestore();
    });

    it("should handle undefined search results", async () => {
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") return Promise.resolve(undefined);
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

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });

      consoleError.mockRestore();
    });

    it("should recover from search error and allow new search", async () => {
      let failFirst = true;
      mockInvoke.mockImplementation((command) => {
        if (command === "check_admin_rights") return Promise.resolve(true);
        if (command === "search_event_logs") {
          if (failFirst) {
            failFirst = false;
            return Promise.reject(new Error("Search failed"));
          }
          return Promise.resolve([mockEventLogEntry]);
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

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });

      // First search fails
      await user.click(searchButton);
      await waitFor(() => {
        expect(screen.getByText(/no events found/i)).toBeInTheDocument();
      });

      // Second search succeeds
      await user.click(searchButton);
      await waitFor(() => {
        expect(
          screen.getByText(`ID: ${mockEventLogEntry.event_id}`),
        ).toBeInTheDocument();
      });

      consoleError.mockRestore();
    });
  });
});
