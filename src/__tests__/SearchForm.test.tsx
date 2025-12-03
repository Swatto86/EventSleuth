import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SearchForm } from "../App";

describe("SearchForm Component", () => {
  const mockOnSearch = vi.fn();

  beforeEach(() => {
    mockOnSearch.mockClear();
  });

  describe("Basic Rendering", () => {
    it("should render all basic input fields", () => {
      render(<SearchForm onSearch={mockOnSearch} />);

      expect(
        screen.getByPlaceholderText(/error.*warning.*failed/i),
      ).toBeInTheDocument();
      expect(screen.getByText(/date range/i)).toBeInTheDocument();
      expect(screen.getByText(/from/i)).toBeInTheDocument();
      expect(screen.getByText(/to/i)).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /search events/i }),
      ).toBeInTheDocument();
    });

    it("should render all event type filter buttons", () => {
      render(<SearchForm onSearch={mockOnSearch} />);

      expect(
        screen.getByRole("button", { name: /critical/i }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /^error$/i }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /warning/i }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /information/i }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /verbose/i }),
      ).toBeInTheDocument();
    });

    it("should show advanced options toggle button", () => {
      render(<SearchForm onSearch={mockOnSearch} />);

      expect(
        screen.getByRole("button", { name: /advanced/i }),
      ).toBeInTheDocument();
    });

    it("should not show advanced fields initially", () => {
      render(<SearchForm onSearch={mockOnSearch} />);

      expect(screen.queryByText(/exclude keywords/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/event ids/i)).not.toBeInTheDocument();
    });
  });

  describe("Keyword Input", () => {
    it("should handle single keyword input", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          keywords: ["error"],
        }),
      );
    });

    it("should handle multiple keywords separated by commas", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error, warning, critical");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          keywords: ["error", "warning", "critical"],
        }),
      );
    });

    it("should trim whitespace from keywords", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "  error  ,  warning  ,  critical  ");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          keywords: ["error", "warning", "critical"],
        }),
      );
    });

    it("should filter out empty keywords", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error,,warning,,,critical");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          keywords: ["error", "warning", "critical"],
        }),
      );
    });

    it("should handle exact match checkbox", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const exactMatchCheckbox = screen.getByLabelText(
        /exact word match only/i,
      ) as HTMLInputElement;

      // Initially unchecked
      expect(exactMatchCheckbox.checked).toBe(false);

      // Check the checkbox
      await user.click(exactMatchCheckbox);
      expect(exactMatchCheckbox.checked).toBe(true);

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          exact_match: true,
        }),
      );
    });
  });

  describe("Date Range Selection", () => {
    it("should handle start date selection", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const startDateInput = document.getElementById(
        "start-date",
      ) as HTMLInputElement;
      await user.type(startDateInput, "2024-01-15T10:00");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(mockOnSearch).toHaveBeenCalledWith(
          expect.objectContaining({
            start_date: expect.any(String),
          }),
        );
      });
    });

    it("should handle end date selection", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const endDateInput = document.getElementById(
        "end-date",
      ) as HTMLInputElement;
      await user.type(endDateInput, "2024-01-20T10:00");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(mockOnSearch).toHaveBeenCalledWith(
          expect.objectContaining({
            end_date: expect.any(String),
          }),
        );
      });
    });

    it("should handle both start and end dates", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const startDateInput = document.getElementById(
        "start-date",
      ) as HTMLInputElement;
      const endDateInput = document.getElementById(
        "end-date",
      ) as HTMLInputElement;

      await user.type(startDateInput, "2024-01-15T10:00");
      await user.type(endDateInput, "2024-01-20T10:00");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      await waitFor(() => {
        expect(mockOnSearch).toHaveBeenCalledWith(
          expect.objectContaining({
            start_date: expect.any(String),
            end_date: expect.any(String),
          }),
        );
      });
    });
  });

  describe("Event Type Filters", () => {
    it("should toggle Critical event type", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const criticalButton = screen.getByRole("button", { name: /critical/i });
      await user.click(criticalButton);

      expect(criticalButton).toHaveAttribute("aria-pressed", "true");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          event_types: [1],
        }),
      );
    });

    it("should toggle multiple event types", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const errorButton = screen.getByRole("button", { name: /^error$/i });
      const warningButton = screen.getByRole("button", { name: /warning/i });

      await user.click(errorButton);
      await user.click(warningButton);

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          event_types: expect.arrayContaining([2, 3]),
        }),
      );
    });

    it("should deselect event type when clicked again", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const errorButton = screen.getByRole("button", { name: /^error$/i });

      await user.click(errorButton);
      expect(errorButton).toHaveAttribute("aria-pressed", "true");

      await user.click(errorButton);
      expect(errorButton).toHaveAttribute("aria-pressed", "false");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          event_types: [],
        }),
      );
    });

    it("should handle all event types selected", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /critical/i }));
      await user.click(screen.getByRole("button", { name: /^error$/i }));
      await user.click(screen.getByRole("button", { name: /warning/i }));
      await user.click(screen.getByRole("button", { name: /information/i }));
      await user.click(screen.getByRole("button", { name: /verbose/i }));

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          event_types: [1, 2, 3, 4, 5],
        }),
      );
    });
  });

  describe("Advanced Options", () => {
    it("should show advanced options when toggle is clicked", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const toggleButton = screen.getByRole("button", {
        name: /advanced/i,
      });
      await user.click(toggleButton);

      expect(screen.getByLabelText(/exclude keywords/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/event ids/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/sources/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/maximum results/i)).toBeInTheDocument();
    });

    it("should hide advanced options when toggle is clicked again", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const toggleButton = screen.getByRole("button", {
        name: /advanced/i,
      });
      await user.click(toggleButton);
      expect(screen.getByText(/exclude keywords/i)).toBeInTheDocument();

      await user.click(toggleButton);
      await waitFor(() => {
        expect(screen.queryByText(/exclude keywords/i)).not.toBeInTheDocument();
      });
    });

    it("should handle exclude keywords input", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const excludeInput = screen.getByLabelText(/exclude keywords/i);
      await user.type(excludeInput, "test, debug");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          exclude_keywords: ["test", "debug"],
        }),
      );
    });

    it("should handle event IDs input", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const eventIdsInput = screen.getByLabelText(/event ids/i);
      await user.type(eventIdsInput, "1000, 2000, 3000");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          event_ids: [1000, 2000, 3000],
        }),
      );
    });

    it("should filter out invalid event IDs", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const eventIdsInput = screen.getByLabelText(/event ids/i);
      await user.type(eventIdsInput, "1000, abc, 2000, xyz");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          event_ids: [1000, 2000],
        }),
      );
    });

    it("should handle sources input", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const sourcesInput = screen.getByLabelText(/sources/i);
      await user.type(sourcesInput, "Application, System");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          sources: ["Application", "System"],
        }),
      );
    });

    it("should handle max results input", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const maxResultsInput = screen.getByLabelText(/maximum results/i);
      await user.clear(maxResultsInput);
      await user.type(maxResultsInput, "100");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          max_results: 100,
        }),
      );
    });

    it("should treat 0 max results as unlimited", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const maxResultsInput = screen.getByLabelText(/maximum results/i);
      await user.clear(maxResultsInput);
      await user.type(maxResultsInput, "0");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          max_results: null,
        }),
      );
    });

    it("should treat empty max results as unlimited", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          max_results: null,
        }),
      );
    });
  });

  describe("Form Submission", () => {
    it("should call onSearch when form is submitted", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error");

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledTimes(1);
    });

    it("should prevent default form submission behavior", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const form = screen
        .getByRole("button", { name: /search events/i })
        .closest("form");
      const submitHandler = vi.fn((e) => e.preventDefault());

      if (form) {
        form.addEventListener("submit", submitHandler);
        await user.click(
          screen.getByRole("button", { name: /search events/i }),
        );
        expect(submitHandler).toHaveBeenCalled();
      }
    });

    it("should submit with empty keywords", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          keywords: [],
        }),
      );
    });
  });

  describe("Complex Search Scenarios", () => {
    it("should handle complete search with all parameters", async () => {
      const user = userEvent.setup();
      render(<SearchForm onSearch={mockOnSearch} />);

      // Set keywords
      const keywordInput = screen.getByPlaceholderText(
        /error.*warning.*failed/i,
      );
      await user.type(keywordInput, "error, failure");

      // Set dates
      const startDateInput = document.getElementById(
        "start-date",
      ) as HTMLInputElement;
      const endDateInput = document.getElementById(
        "end-date",
      ) as HTMLInputElement;

      await user.type(startDateInput, "2024-01-15T10:00");
      await user.type(endDateInput, "2024-01-20T10:00");

      // Set event types
      await user.click(screen.getByRole("button", { name: /^error$/i }));
      await user.click(screen.getByRole("button", { name: /warning/i }));

      // Show advanced and set advanced options
      await user.click(screen.getByRole("button", { name: /advanced/i }));

      const excludeInput = screen.getByLabelText(/exclude keywords/i);
      await user.type(excludeInput, "test");

      const eventIdsInput = screen.getByLabelText(/event ids/i);
      await user.type(eventIdsInput, "1000, 2000");

      const sourcesInput = screen.getByLabelText(/sources/i);
      await user.type(sourcesInput, "Application");

      const maxResultsInput = screen.getByLabelText(/maximum results/i);
      await user.clear(maxResultsInput);
      await user.type(maxResultsInput, "50");

      // Submit
      const searchButton = screen.getByRole("button", {
        name: /search events/i,
      });
      await user.click(searchButton);

      expect(mockOnSearch).toHaveBeenCalledWith({
        keywords: ["error", "failure"],
        exclude_keywords: ["test"],
        start_date: expect.any(String),
        end_date: expect.any(String),
        log_names: [],
        event_types: expect.arrayContaining([2, 3]),
        event_ids: [1000, 2000],
        sources: ["Application"],
        categories: [],
        max_results: 50,
        exact_match: false,
      });
    });
  });
});
