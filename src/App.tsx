import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import "./styles.css";
import { format, parseISO } from "date-fns";

// Types
interface SearchParams {
  keywords: string[];
  exclude_keywords: string[];
  start_date: string | null;
  end_date: string | null;
  log_names: string[];
  event_types: number[];
  event_ids: number[];
  sources: string[];
  categories: number[];
  max_results: number | null;
  exact_match: boolean;
}

interface EventLogEntry {
  source: string;
  time_generated: string;
  event_id: number;
  event_type: string;
  severity: string;
  category: number;
  message: string;
  computer_name: string;
  matches: string[];
}

const EVENT_TYPES = [
  {
    id: 1,
    label: "Critical",
    bgColor: "bg-red-500",
    hoverColor: "hover:bg-red-600",
  },
  {
    id: 2,
    label: "Error",
    bgColor: "bg-orange-500",
    hoverColor: "hover:bg-orange-600",
  },
  {
    id: 3,
    label: "Warning",
    bgColor: "bg-yellow-500",
    hoverColor: "hover:bg-yellow-600",
  },
  {
    id: 4,
    label: "Information",
    bgColor: "bg-blue-500",
    hoverColor: "hover:bg-blue-600",
  },
  {
    id: 5,
    label: "Verbose",
    bgColor: "bg-gray-500",
    hoverColor: "hover:bg-gray-600",
  },
];

// Add available themes
const THEMES = [
  "light",
  "dark",
  "cupcake",
  "synthwave",
  "cyberpunk",
  "retro",
  "night",
  "dracula",
];

export function SearchForm({
  onSearch,
}: {
  onSearch: (params: SearchParams) => void;
}) {
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [searchParams, setSearchParams] = useState<SearchParams>({
    keywords: [],
    exclude_keywords: [],
    log_names: [],
    event_types: [],
    event_ids: [],
    sources: [],
    categories: [],
    start_date: null,
    end_date: null,
    max_results: null,
    exact_match: false,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch(searchParams);
  };

  const handleMaxResultsChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseInt(e.target.value);
    setSearchParams({
      ...searchParams,
      max_results: !e.target.value || value === 0 ? null : value,
    });
  };

  return (
    <div className="bg-base-200 p-4 rounded-xl shadow-2xl border border-base-300">
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Search Header */}
        <div className="flex items-center gap-3 pb-2 border-b border-base-300">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-6 w-6 text-primary"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <h2 className="text-xl font-semibold text-base-content">
            Search Event Logs
          </h2>
        </div>

        {/* Basic Search */}
        <div className="space-y-3">
          {/* Keywords Input */}
          <div>
            <label className="block text-sm font-medium mb-2 text-base-content/90">
              🔍 Keywords
            </label>
            <input
              type="text"
              placeholder="e.g., error, warning, failed (separate with commas)"
              className="w-full p-3 input input-bordered focus:input-primary transition-all"
              onChange={(e) =>
                setSearchParams({
                  ...searchParams,
                  keywords: e.target.value
                    .split(",")
                    .map((k) => k.trim())
                    .filter((k) => k),
                })
              }
            />
            <p className="text-xs text-base-content/60 mt-1">
              Search for specific terms in event messages
            </p>
            {/* Exact Match Checkbox */}
            <div className="flex items-center gap-2 mt-2">
              <input
                type="checkbox"
                id="exact-match"
                className="checkbox checkbox-primary checkbox-sm"
                checked={searchParams.exact_match}
                onChange={(e) =>
                  setSearchParams({
                    ...searchParams,
                    exact_match: e.target.checked,
                  })
                }
              />
              <label
                htmlFor="exact-match"
                className="text-sm text-base-content/80 cursor-pointer"
              >
                Exact word match only
              </label>
            </div>
          </div>

          {/* Date Range */}
          <div>
            <label className="block text-sm font-medium mb-2 text-base-content/90">
              📅 Date Range
            </label>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label
                  htmlFor="start-date"
                  className="block text-xs mb-1 text-base-content/70"
                >
                  From
                </label>
                <input
                  id="start-date"
                  type="datetime-local"
                  className="p-3 input input-bordered w-full focus:input-primary transition-all"
                  onChange={(e) =>
                    setSearchParams({
                      ...searchParams,
                      start_date: e.target.value
                        ? new Date(e.target.value).toISOString()
                        : null,
                    })
                  }
                />
              </div>
              <div>
                <label
                  htmlFor="end-date"
                  className="block text-xs mb-1 text-base-content/70"
                >
                  To
                </label>
                <input
                  id="end-date"
                  type="datetime-local"
                  className="p-3 input input-bordered w-full focus:input-primary transition-all"
                  onChange={(e) =>
                    setSearchParams({
                      ...searchParams,
                      end_date: e.target.value
                        ? new Date(e.target.value).toISOString()
                        : null,
                    })
                  }
                />
              </div>
            </div>
          </div>

          {/* Event Severity Filters */}
          <div>
            <label className="block text-sm font-medium mb-3 text-base-content/90">
              ⚠️ Event Severity Levels
            </label>
            <div className="flex flex-wrap gap-2">
              {EVENT_TYPES.map((type) => (
                <button
                  key={type.id}
                  type="button"
                  aria-pressed={searchParams.event_types.includes(type.id)}
                  onClick={() => {
                    setSearchParams((prev) => ({
                      ...prev,
                      event_types: prev.event_types.includes(type.id)
                        ? prev.event_types.filter((id) => id !== type.id)
                        : [...prev.event_types, type.id],
                    }));
                  }}
                  className={`px-5 py-2.5 rounded-lg font-medium transition-all transform hover:scale-105 ${
                    searchParams.event_types.includes(type.id)
                      ? `${type.bgColor} text-white shadow-lg`
                      : "bg-base-300 text-base-content hover:bg-base-300/70"
                  }`}
                >
                  {type.label}
                </button>
              ))}
            </div>
            <p className="text-xs text-base-content/60 mt-2">
              {searchParams.event_types.length === 0
                ? "No filters selected - all severity levels will be included"
                : `Filtering by: ${searchParams.event_types.map((id) => EVENT_TYPES.find((t) => t.id === id)?.label).join(", ")}`}
            </p>
          </div>
        </div>

        {/* Advanced Search Toggle */}
        <button
          type="button"
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="flex items-center gap-2 text-primary hover:text-primary-focus transition-colors font-medium"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className={`h-5 w-5 transition-transform ${showAdvanced ? "rotate-180" : ""}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M19 9l-7 7-7-7"
            />
          </svg>
          {showAdvanced ? "Hide Advanced Filters" : "Show Advanced Filters"}
        </button>

        {/* Advanced Search Options */}
        {showAdvanced && (
          <div className="space-y-3 p-4 bg-base-300/50 rounded-xl border border-base-300 backdrop-blur-sm">
            <div className="flex items-center gap-2 pb-3 border-b border-base-300">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-5 w-5 text-primary"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"
                />
              </svg>
              <h3 className="font-semibold text-base-content">
                Advanced Filters
              </h3>
            </div>

            <div>
              <label
                htmlFor="exclude-keywords"
                className="block text-sm font-medium mb-2 text-base-content/90"
              >
                🚫 Exclude Keywords
              </label>
              <input
                id="exclude-keywords"
                type="text"
                placeholder="e.g., debug, trace (separate with commas)"
                className="w-full p-3 input input-bordered focus:input-primary transition-all"
                onChange={(e) =>
                  setSearchParams({
                    ...searchParams,
                    exclude_keywords: e.target.value
                      .split(",")
                      .map((k) => k.trim())
                      .filter((k) => k),
                  })
                }
              />
              <p className="text-xs text-base-content/60 mt-1">
                Exclude events containing these terms
              </p>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label
                  htmlFor="event-ids"
                  className="block text-sm font-medium mb-2 text-base-content/90"
                >
                  🔢 Event IDs
                </label>
                <input
                  id="event-ids"
                  type="text"
                  placeholder="e.g., 1000, 1001, 2000"
                  className="w-full p-3 input input-bordered focus:input-primary transition-all"
                  onChange={(e) => {
                    const ids = e.target.value
                      .split(",")
                      .map((id) => parseInt(id.trim()))
                      .filter((id) => !isNaN(id));
                    setSearchParams({ ...searchParams, event_ids: ids });
                  }}
                />
                <p className="text-xs text-base-content/60 mt-1">
                  Filter by specific event IDs
                </p>
              </div>

              <div>
                <label
                  htmlFor="sources"
                  className="block text-sm font-medium mb-2 text-base-content/90"
                >
                  📦 Sources
                </label>
                <input
                  id="sources"
                  type="text"
                  placeholder="e.g., Application, System"
                  className="w-full p-3 input input-bordered focus:input-primary transition-all"
                  onChange={(e) => {
                    const sources = e.target.value
                      .split(",")
                      .map((s) => s.trim())
                      .filter((s) => s);
                    setSearchParams({ ...searchParams, sources });
                  }}
                />
                <p className="text-xs text-base-content/60 mt-1">
                  Filter by event sources
                </p>
              </div>
            </div>

            <div>
              <label
                htmlFor="max-results"
                className="block text-sm font-medium mb-2 text-base-content/90"
              >
                📊 Maximum Results
              </label>
              <input
                id="max-results"
                type="number"
                className="w-full p-3 input input-bordered focus:input-primary transition-all"
                placeholder="Leave empty for unlimited"
                min={0}
                onChange={handleMaxResultsChange}
              />
              <p className="text-xs text-base-content/60 mt-1">
                {searchParams.max_results
                  ? `Limit results to ${searchParams.max_results} events`
                  : "No limit - return all matching events"}
              </p>
            </div>
          </div>
        )}

        <button
          type="submit"
          className="w-full btn btn-primary btn-lg gap-2 shadow-lg hover:shadow-xl transition-all"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-5 w-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          Search Events
        </button>
      </form>
    </div>
  );
}

export function EventList({
  events,
  isLoading,
  onClear,
}: {
  events: EventLogEntry[];
  isLoading: boolean;
  onClear: () => void;
}) {
  // Safety check for events array
  const safeEvents = events || [];
  const eventCount = safeEvents.length;

  const handleExport = async () => {
    try {
      // Create CSV content
      const headers = ["Time", "Severity", "Event ID", "Source", "Message"];
      const rows = safeEvents.map((event) => [
        event.time_generated,
        event.severity,
        event.event_id,
        event.source,
        // Replace newlines and quotes in message to maintain CSV format
        event.message.replace(/[\n\r]+/g, " ").replace(/"/g, '""'),
      ]);

      const csvContent = [
        headers.join(","),
        ...rows.map((row) => row.map((cell) => `"${cell}"`).join(",")),
      ].join("\n");

      // Show save dialog
      const filePath = await save({
        filters: [
          {
            name: "CSV",
            extensions: ["csv"],
          },
        ],
        defaultPath: "event_logs.csv",
      });

      if (filePath) {
        // Convert string to Uint8Array before writing
        const encoder = new TextEncoder();
        const data = encoder.encode(csvContent);
        await writeFile(filePath, data);
      }
    } catch (error) {
      console.error("Error exporting CSV:", error);
    }
  };

  if (isLoading) {
    return (
      <div className="flex flex-col items-center justify-center p-12 bg-base-200 rounded-xl">
        <div className="loading loading-spinner loading-lg text-primary"></div>
        <p className="mt-4 text-base-content/70">Searching event logs...</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {safeEvents.length > 0 && (
        <div className="bg-base-200 p-4 rounded-xl border border-base-300 flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="badge badge-primary badge-lg">
              {eventCount} {eventCount === 1 ? "Event" : "Events"} Found
            </div>
            <span className="text-sm text-base-content/70">
              {new Date().toLocaleString()}
            </span>
          </div>
          <div className="flex gap-2">
            <button onClick={handleExport} className="btn btn-success gap-2">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-5 w-5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
              Export to CSV
            </button>
            <button onClick={onClear} className="btn btn-error gap-2">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-5 w-5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
              Clear Results
            </button>
          </div>
        </div>
      )}

      {safeEvents.length === 0 ? (
        <div className="flex flex-col items-center justify-center p-12 bg-base-200 rounded-xl border-2 border-dashed border-base-300">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-16 w-16 text-base-content/30 mb-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <p className="text-lg font-medium text-base-content/70">
            No events found
          </p>
          <p className="text-sm text-base-content/50 mt-2">
            Try adjusting your search filters
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {safeEvents.map((event, index) => (
            <div
              key={index}
              className="bg-base-200 p-6 rounded-xl shadow-lg border border-base-300 hover:shadow-xl transition-all"
            >
              <div className="flex flex-col sm:flex-row sm:justify-between sm:items-start gap-4 mb-4">
                <div className="space-y-3 flex-1">
                  <div className="flex flex-wrap items-center gap-3">
                    <span
                      className={`px-4 py-1.5 rounded-full text-sm font-bold shadow-lg ${
                        event.severity === "Critical"
                          ? "bg-red-500/20 text-red-400 border border-red-500/30"
                          : event.severity === "Error"
                            ? "bg-orange-500/20 text-orange-400 border border-orange-500/30"
                            : event.severity === "Warning"
                              ? "bg-yellow-500/20 text-yellow-400 border border-yellow-500/30"
                              : event.severity === "Information"
                                ? "bg-blue-500/20 text-blue-400 border border-blue-500/30"
                                : "bg-gray-500/20 text-gray-400 border border-gray-500/30"
                      }`}
                    >
                      {event.severity}
                    </span>
                    <span className="badge badge-outline gap-1">
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        className="h-4 w-4"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M7 20l4-16m2 16l4-16M6 9h14M4 15h14"
                        />
                      </svg>
                      ID: {event.event_id}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-4 text-sm text-base-content/70">
                    <span className="flex items-center gap-1">
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        className="h-4 w-4"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"
                        />
                      </svg>
                      {event.source}
                    </span>
                    <span className="flex items-center gap-1">
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        className="h-4 w-4"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
                        />
                      </svg>
                      {event.computer_name}
                    </span>
                  </div>
                </div>
                <div className="text-sm font-medium text-base-content/80 bg-base-300/50 px-3 py-2 rounded-lg whitespace-nowrap flex items-center gap-2">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    className="h-4 w-4"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  {format(
                    parseISO(event.time_generated),
                    "dd/MM/yyyy HH:mm:ss",
                  )}
                </div>
              </div>
              <div className="divider my-3"></div>
              <div
                className="text-base-content/90 leading-relaxed break-words whitespace-pre-wrap overflow-x-auto max-w-full bg-base-300/30 p-4 rounded-lg"
                dangerouslySetInnerHTML={{
                  __html:
                    event.matches && event.matches.length > 0
                      ? event.message.replace(
                          new RegExp(`(${event.matches.join("|")})`, "gi"),
                          '<mark class="bg-yellow-500/40 text-yellow-200 px-1.5 py-0.5 rounded font-semibold">$1</mark>',
                        )
                      : event.message,
                }}
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function App() {
  const [events, setEvents] = useState<EventLogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [theme, setTheme] = useState("dark");
  const [isAdmin, setIsAdmin] = useState<boolean>(false); // Changed default to false

  // Theme management
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  // Check admin status on app launch
  useEffect(() => {
    const checkAdminStatus = async () => {
      try {
        const isAdmin = await invoke<boolean>("check_admin_rights");
        setIsAdmin(isAdmin);
      } catch (error) {
        console.error("Failed to check admin status:", error);
        setIsAdmin(false);
      }
    };
    checkAdminStatus();
  }, []);

  const handleSearch = async (params: SearchParams) => {
    setIsLoading(true);
    try {
      const results = await invoke<EventLogEntry[]>("search_event_logs", {
        params,
      });
      setEvents(results);
    } catch (error) {
      console.error("Error searching logs:", error);
      setEvents([]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClear = () => {
    setEvents([]);
  };

  return (
    <div className="h-screen overflow-auto bg-gradient-to-br from-base-100 to-base-200 text-base-content">
      <div className="sticky top-0 z-50 bg-base-100/95 backdrop-blur-md shadow-lg border-b border-base-300">
        {!isAdmin && (
          <div className="bg-error/10 text-error p-3 text-center text-sm border-b border-error/20 flex items-center justify-center gap-2">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-5 w-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            <span className="font-medium">
              Running without administrator privileges
            </span>
            <span className="text-xs opacity-80">
              • Some event logs may not be accessible
            </span>
          </div>
        )}
        <div className="container mx-auto px-4 py-3 flex items-center justify-center relative">
          <div className="flex items-center gap-3">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-8 w-8 text-primary"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            <h1 className="text-3xl font-bold bg-gradient-to-r from-primary to-secondary bg-clip-text text-transparent">
              EventSleuth
            </h1>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="container mx-auto px-4 py-4 max-w-7xl">
        <SearchForm onSearch={handleSearch} />
        <div className="mt-4">
          <EventList
            events={events}
            isLoading={isLoading}
            onClear={handleClear}
          />
        </div>
      </div>

      {/* Theme selector - positioned in header with CSS but comes after main content in DOM for proper tab order */}
      <div className="fixed bottom-6 right-6 z-50 dropdown dropdown-top dropdown-end">
        <button
          className="btn btn-circle btn-primary shadow-2xl hover:shadow-primary/50 transition-all"
          aria-label="Theme selector"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-6 w-6"
            fill="currentColor"
            viewBox="0 0 16 16"
          >
            <path d="M8 5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3m4 3a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3M5.5 7a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m.5 6a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3" />
            <path d="M16 8c0 3.15-1.866 2.585-3.567 2.07C11.42 9.763 10.465 9.473 10 10c-.603.683-.475 1.819-.351 2.92C9.826 14.495 9.996 16 8 16a8 8 0 1 1 8-8m-8 7c1.573 0 1.445-1.063 1.322-2.27-.107-1.042-.23-2.23.352-2.91.583-.68 1.538-.391 2.543-.085 1.701.515 3.783 1.08 3.783-1.735a7 7 0 1 0-7 7" />
          </svg>
        </button>
        <ul className="dropdown-content z-[1] menu p-2 shadow-2xl bg-base-200 rounded-box w-52 mb-2 border border-base-300">
          <li className="menu-title">
            <span className="flex items-center gap-2">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01"
                />
              </svg>
              Choose Theme
            </span>
          </li>
          {THEMES.map((t) => (
            <li key={t}>
              <button
                onClick={() => setTheme(t)}
                className={`capitalize ${theme === t ? "active" : ""}`}
              >
                {theme === t && (
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    className="h-4 w-4"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                )}
                {t}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export default App;
