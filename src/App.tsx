import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import './styles.css';
import { format, parseISO } from 'date-fns';

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
  { id: 1, label: 'Critical', bgColor: 'bg-red-500', hoverColor: 'hover:bg-red-600' },
  { id: 2, label: 'Error', bgColor: 'bg-orange-500', hoverColor: 'hover:bg-orange-600' },
  { id: 3, label: 'Warning', bgColor: 'bg-yellow-500', hoverColor: 'hover:bg-yellow-600' },
  { id: 4, label: 'Information', bgColor: 'bg-blue-500', hoverColor: 'hover:bg-blue-600' },
  { id: 5, label: 'Verbose', bgColor: 'bg-gray-500', hoverColor: 'hover:bg-gray-600' },
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

function SearchForm({ onSearch }: { onSearch: (params: SearchParams) => void }) {
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
    max_results: null
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch(searchParams);
  };

  const handleMaxResultsChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseInt(e.target.value);
    setSearchParams({
      ...searchParams,
      max_results: (!e.target.value || value === 0) ? null : value
    });
  };

  return (
    <div className="bg-base-200 p-6 rounded-lg shadow-xl">
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Basic Search */}
        <div className="space-y-4">
          <input
            type="text"
            placeholder="Enter keywords separated by commas..."
            className="w-full p-3 input input-bordered"
            onChange={(e) => setSearchParams({
              ...searchParams,
              keywords: e.target.value.split(',').map(k => k.trim()).filter(k => k)
            })}
          />

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm mb-2">Start Date</label>
              <input
                type="datetime-local"
                className="p-3 input input-bordered w-full"
                onChange={(e) => setSearchParams({
                  ...searchParams,
                  start_date: e.target.value ? new Date(e.target.value).toISOString() : null
                })}
              />
            </div>
            <div>
              <label className="block text-sm mb-2">End Date</label>
              <input
                type="datetime-local"
                className="p-3 input input-bordered w-full"
                onChange={(e) => setSearchParams({
                  ...searchParams,
                  end_date: e.target.value ? new Date(e.target.value).toISOString() : null
                })}
              />
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            {EVENT_TYPES.map((type) => (
              <button
                key={type.id}
                type="button"
                aria-pressed={searchParams.event_types.includes(type.id)}
                onClick={() => {
                  setSearchParams(prev => ({
                    ...prev,
                    event_types: prev.event_types.includes(type.id)
                      ? prev.event_types.filter(id => id !== type.id)
                      : [...prev.event_types, type.id]
                  }));
                }}
                className={`px-4 py-2 rounded-lg transition-colors ${
                  searchParams.event_types.includes(type.id)
                    ? `${type.bgColor} text-white`
                    : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                }`}
              >
                {type.label}
              </button>
            ))}
          </div>
        </div>

        {/* Advanced Search Toggle */}
        <button
          type="button"
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="text-blue-400 hover:text-blue-300 text-sm"
        >
          {showAdvanced ? '− Hide Advanced Options' : '+ Show Advanced Options'}
        </button>

        {/* Advanced Search Options */}
        {showAdvanced && (
          <div className="space-y-4 p-4 bg-base-300 rounded-lg">
            <div>
              <label className="block text-sm mb-2">
                Exclude Keywords (comma-separated)
              </label>
              <input
                type="text"
                className="w-full p-3 input input-bordered"
                onChange={(e) => setSearchParams({
                  ...searchParams,
                  exclude_keywords: e.target.value.split(',').map(k => k.trim()).filter(k => k)
                })}
              />
            </div>

            <div>
              <label className="block text-sm mb-2">
                Event IDs (comma-separated)
              </label>
              <input
                type="text"
                className="w-full p-3 input input-bordered"
                onChange={(e) => {
                  const ids = e.target.value
                    .split(',')
                    .map(id => parseInt(id.trim()))
                    .filter(id => !isNaN(id));
                  setSearchParams({...searchParams, event_ids: ids});
                }}
              />
            </div>

            <div>
              <label className="block text-sm mb-2">
                Sources (comma-separated)
              </label>
              <input
                type="text"
                className="w-full p-3 input input-bordered"
                onChange={(e) => {
                  const sources = e.target.value
                    .split(',')
                    .map(s => s.trim())
                    .filter(s => s);
                  setSearchParams({...searchParams, sources});
                }}
              />
            </div>

            <div>
              <label className="block text-sm mb-2">
                Maximum Results (0 or empty for unlimited)
              </label>
              <input
                type="number"
                className="w-full p-3 input input-bordered"
                placeholder="Unlimited"
                min={0}
                onChange={handleMaxResultsChange}
              />
            </div>
          </div>
        )}

        <button
          type="submit"
          className="w-full btn btn-primary"
        >
          Search Events
        </button>
      </form>
    </div>
  );
}

function EventList({ events, isLoading, onClear }: { events: EventLogEntry[]; isLoading: boolean; onClear: () => void }) {
  const handleExport = async () => {
    try {
      // Create CSV content
      const headers = ['Time', 'Severity', 'Event ID', 'Source', 'Message'];
      const rows = events.map(event => [
        event.time_generated,
        event.severity,
        event.event_id,
        event.source,
        // Replace newlines and quotes in message to maintain CSV format
        event.message.replace(/[\n\r]+/g, ' ').replace(/"/g, '""')
      ]);
      
      const csvContent = [
        headers.join(','),
        ...rows.map(row => row.map(cell => `"${cell}"`).join(','))
      ].join('\n');

      // Show save dialog
      const filePath = await save({
        filters: [{
          name: 'CSV',
          extensions: ['csv']
        }],
        defaultPath: 'event_logs.csv'
      });

      if (filePath) {
        // Convert string to Uint8Array before writing
        const encoder = new TextEncoder();
        const data = encoder.encode(csvContent);
        await writeFile(filePath, data);
      }
    } catch (error) {
      console.error('Error exporting CSV:', error);
    }
  };

  if (isLoading) {
    return <div className="p-6 text-center">Searching events...</div>;
  }

  return (
    <div className="space-y-4">
      {events.length > 0 && (
        <div className="flex justify-end gap-2">
          <button
            onClick={onClear}
            className="btn btn-error"
          >
            Clear Results
          </button>
          <button
            onClick={handleExport}
            className="btn btn-success"
          >
            Export to CSV
          </button>
        </div>
      )}
      
      {events.length === 0 ? (
        <div className="p-6 text-center">No events found</div>
      ) : (
        <div className="space-y-4">
          {events.map((event, index) => (
            <div key={index} className="bg-base-200 p-6 rounded-lg shadow-lg">
              <div className="flex justify-between items-start mb-3">
                <div className="space-y-2">
                  <div className="flex items-center gap-3">
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      event.severity === 'Critical' ? 'bg-red-500/20 text-red-400' :
                      event.severity === 'Error' ? 'bg-orange-500/20 text-orange-400' :
                      event.severity === 'Warning' ? 'bg-yellow-500/20 text-yellow-400' :
                      event.severity === 'Information' ? 'bg-blue-500/20 text-blue-400' :
                      'bg-gray-500/20 text-gray-400'
                    }`}>
                      {event.severity}
                    </span>
                    <span className="text-base-content/70">ID: {event.event_id}</span>
                  </div>
                  <div className="flex items-center gap-2 text-sm text-base-content/70">
                    <span>{event.source}</span>
                  </div>
                </div>
                <span className="text-sm text-base-content/70">
                  {format(parseISO(event.time_generated), 'dd/MM/yyyy HH:mm:ss')}
                </span>
              </div>
              <div 
                className="text-base-content/90 leading-relaxed break-words whitespace-pre-wrap overflow-x-auto max-w-full" 
                dangerouslySetInnerHTML={{ 
                  __html: event.message.replace(
                    new RegExp(`(${event.matches.join('|')})`, 'gi'), 
                    '<mark class="bg-yellow-500/30 text-yellow-200 px-1 rounded">$1</mark>'
                  )
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

  // Theme management
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  const handleSearch = async (params: SearchParams) => {
    setIsLoading(true);
    try {
      const results = await invoke<EventLogEntry[]>('search_event_logs', { params });
      setEvents(results);
    } catch (error) {
      console.error('Error searching logs:', error);
      setEvents([]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClear = () => {
    setEvents([]);
  };

  return (
    <div className="min-h-screen bg-base-100 text-base-content">
      <div className="sticky top-0 z-50 bg-base-100">
        <div className="container mx-auto px-4 py-4 flex justify-center items-center relative">
          <h1 className="text-3xl font-bold absolute left-1/2 transform -translate-x-1/2">EventSleuth</h1>
          <div className="dropdown dropdown-end ml-auto">
            <button tabIndex={0} className="btn btn-ghost">
              Theme
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" className="bi bi-palette ml-2" viewBox="0 0 16 16">
                <path d="M8 5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3m4 3a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3M5.5 7a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m.5 6a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3"/>
                <path d="M16 8c0 3.15-1.866 2.585-3.567 2.07C11.42 9.763 10.465 9.473 10 10c-.603.683-.475 1.819-.351 2.92C9.826 14.495 9.996 16 8 16a8 8 0 1 1 8-8m-8 7c1.573 0 1.445-1.063 1.322-2.27-.107-1.042-.23-2.23.352-2.91.583-.68 1.538-.391 2.543-.085 1.701.515 3.783 1.08 3.783-1.735a7 7 0 1 0-7 7"/>
              </svg>
            </button>
            <ul tabIndex={0} className="dropdown-content z-[1] menu p-2 shadow bg-base-200 rounded-box w-52">
              {THEMES.map((t) => (
                <li key={t}>
                  <button 
                    onClick={() => setTheme(t)}
                    className={`${theme === t ? 'active' : ''}`}
                  >
                    {t}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="container mx-auto px-4 py-6">
        <SearchForm onSearch={handleSearch} />
        <div className="mt-6">
          <EventList 
            events={events} 
            isLoading={isLoading} 
            onClear={handleClear}
          />
        </div>
      </div>
    </div>
  );
}

export default App;
