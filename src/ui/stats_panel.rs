//! Event statistics summary panel.
//!
//! Displays a collapsible overview of the currently loaded events:
//! counts by severity level, top providers, and an events-per-hour
//! histogram. Provides immediate situational awareness for incident
//! response and triage workflows.

use crate::app::EventSleuthApp;
use crate::ui::theme;
use std::collections::HashMap;

/// Maximum number of top providers to display in the summary.
const MAX_TOP_PROVIDERS: usize = 10;

/// Maximum number of hourly histogram buckets to display.
const MAX_HISTOGRAM_BUCKETS: usize = 24;

/// Pre-computed statistics snapshot for the current event set.
///
/// Computed lazily and cached in app state; invalidated whenever
/// `needs_refilter` triggers or the event list changes.
#[derive(Debug, Clone, Default)]
pub struct EventStats {
    /// Total events in the filtered set.
    pub total: usize,
    /// Counts per severity level (index 0..=5).
    pub level_counts: [usize; 6],
    /// Top N providers by frequency: `(provider_name, count)`.
    pub top_providers: Vec<(String, usize)>,
    /// Hourly event counts for the histogram, ordered oldest-first.
    /// Each entry is `(hour_label, count)`.
    pub hourly_histogram: Vec<(String, usize)>,
}

impl EventSleuthApp {
    /// Compute statistics from the currently filtered events.
    ///
    /// Called when the stats panel is visible and the event data has changed.
    /// Results are cached in `self.stats_cache` until the next refilter.
    pub fn compute_stats(&self) -> EventStats {
        let events = &self.all_events;
        let indices = &self.filtered_indices;

        if indices.is_empty() {
            return EventStats::default();
        }

        // Level counts
        let mut level_counts = [0usize; 6];
        let mut provider_counts: HashMap<&str, usize> = HashMap::new();

        // Collect timestamps for histogram
        let mut timestamps: Vec<chrono::DateTime<chrono::Utc>> = Vec::with_capacity(indices.len());

        for &idx in indices {
            let event = &events[idx];
            let level_idx = (event.level as usize).min(5);
            level_counts[level_idx] += 1;
            *provider_counts
                .entry(event.provider_name.as_str())
                .or_insert(0) += 1;
            timestamps.push(event.timestamp);
        }

        // Top providers
        let mut provider_vec: Vec<(String, usize)> = provider_counts
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        provider_vec.sort_by(|a, b| b.1.cmp(&a.1));
        provider_vec.truncate(MAX_TOP_PROVIDERS);

        // Hourly histogram
        let hourly_histogram = if timestamps.is_empty() {
            Vec::new()
        } else {
            Self::build_hourly_histogram(&timestamps)
        };

        EventStats {
            total: indices.len(),
            level_counts,
            top_providers: provider_vec,
            hourly_histogram,
        }
    }

    /// Build an hourly histogram from a set of timestamps.
    ///
    /// Divides the time span into hour-aligned buckets and counts events
    /// in each. Limits to the most recent [`MAX_HISTOGRAM_BUCKETS`] hours.
    fn build_hourly_histogram(
        timestamps: &[chrono::DateTime<chrono::Utc>],
    ) -> Vec<(String, usize)> {
        use chrono::{Duration, Local, Timelike};

        if timestamps.is_empty() {
            return Vec::new();
        }

        let min_ts = timestamps.iter().copied().min().unwrap();
        let max_ts = timestamps.iter().copied().max().unwrap();

        // Round down to the nearest hour
        let start_hour = min_ts
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0))
            .unwrap_or(min_ts);
        // Use checked arithmetic when advancing the end bucket by one hour to
        // guard against a panic if max_ts is near DateTime<Utc>::MAX_UTC.
        // In practice event timestamps are never near the maximum, but defensive
        // code here avoids a panic on malformed or synthetic evtx files.
        let max_ts_rounded = max_ts
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0))
            .unwrap_or(max_ts);
        let end_hour = max_ts_rounded
            .checked_add_signed(Duration::hours(1))
            .unwrap_or(max_ts_rounded);

        let total_hours = ((end_hour - start_hour).num_hours()).max(1) as usize;

        // If span is too wide, take only the most recent hours
        let display_hours = total_hours.min(MAX_HISTOGRAM_BUCKETS);
        let bucket_start = if total_hours > MAX_HISTOGRAM_BUCKETS {
            end_hour - Duration::hours(MAX_HISTOGRAM_BUCKETS as i64)
        } else {
            start_hour
        };

        let mut buckets = vec![0usize; display_hours];

        for &ts in timestamps {
            if ts < bucket_start {
                continue;
            }
            let idx = ((ts - bucket_start).num_hours()).max(0) as usize;
            if idx < buckets.len() {
                buckets[idx] += 1;
            }
        }

        buckets
            .into_iter()
            .enumerate()
            .map(|(i, count)| {
                let hour_ts = bucket_start + Duration::hours(i as i64);
                let local_hour = hour_ts.with_timezone(&Local);
                let label = local_hour.format("%H:%M").to_string();
                (label, count)
            })
            .collect()
    }

    /// Render the statistics summary panel.
    ///
    /// Shown as a collapsible section in a window anchored to the toolbar.
    /// Visibility is controlled by `self.show_stats`.
    pub fn render_stats_panel(&mut self, ctx: &egui::Context) {
        if !self.show_stats {
            return;
        }

        let mut open = true;
        let max_h = ctx.screen_rect().height() * 0.75;
        egui::Window::new("\u{1F4CA} Event Statistics")
            .open(&mut open)
            .collapsible(true)
            .resizable(true)
            .default_width(340.0)
            .default_height(max_h.min(500.0))
            .max_height(max_h)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Recompute stats if needed
                    if self.stats_dirty {
                        self.stats_cache = self.compute_stats();
                        self.stats_dirty = false;
                    }

                    let stats = &self.stats_cache;
                    let dark = self.dark_mode;

                    if stats.total == 0 {
                        ui.label(
                            egui::RichText::new("No events to analyse")
                                .color(theme::text_dim(dark))
                                .italics(),
                        );
                        return;
                    }

                    ui.label(
                        egui::RichText::new(format!("{} filtered events", stats.total))
                            .color(theme::accent(dark))
                            .strong(),
                    );

                    ui.add_space(theme::SECTION_SPACING);

                    // ── Counts by level ─────────────────────────────────
                    egui::CollapsingHeader::new(
                        egui::RichText::new("\u{1F4CA} By Severity").strong(),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        let level_names = [
                            "LogAlways",
                            "Critical",
                            "Error",
                            "Warning",
                            "Information",
                            "Verbose",
                        ];
                        for (i, name) in level_names.iter().enumerate() {
                            let count = stats.level_counts[i];
                            if count == 0 {
                                continue;
                            }
                            let color = theme::level_color(i as u8, dark);
                            let pct = (count as f64 / stats.total as f64 * 100.0).round() as u32;
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(*name).color(color));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{count} ({pct}%)"))
                                                .color(theme::text_secondary(dark)),
                                        );
                                    },
                                );
                            });
                            // Mini bar
                            let bar_frac = count as f32 / stats.total as f32;
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 4.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(rect.width() * bar_frac, rect.height()),
                                ),
                                2.0,
                                color,
                            );
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect.min + egui::vec2(rect.width() * bar_frac, 0.0),
                                    egui::vec2(rect.width() * (1.0 - bar_frac), rect.height()),
                                ),
                                2.0,
                                if dark {
                                    egui::Color32::from_rgb(50, 50, 65)
                                } else {
                                    egui::Color32::from_rgb(220, 220, 228)
                                },
                            );
                            ui.add_space(2.0);
                        }
                    });

                    ui.add_space(theme::SECTION_SPACING);

                    // ── Top providers ───────────────────────────────────
                    egui::CollapsingHeader::new(
                        egui::RichText::new("\u{1F3F7}\u{FE0F} Top Providers").strong(),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        if stats.top_providers.is_empty() {
                            ui.label(
                                egui::RichText::new("No providers")
                                    .color(theme::text_dim(dark))
                                    .italics(),
                            );
                            return;
                        }
                        let max_count = stats
                            .top_providers
                            .first()
                            .map(|(_, c)| *c)
                            .unwrap_or(1)
                            .max(1);

                        for (name, count) in &stats.top_providers {
                            ui.horizontal(|ui| {
                                // Truncate long provider names
                                let display_name = if name.len() > 35 {
                                    let end = name
                                        .char_indices()
                                        .nth(32)
                                        .map(|(i, _)| i)
                                        .unwrap_or(name.len());
                                    format!("{}...", &name[..end])
                                } else {
                                    name.clone()
                                };
                                ui.label(
                                    egui::RichText::new(display_name)
                                        .color(theme::text_primary(dark)),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(count.to_string())
                                                .color(theme::text_secondary(dark)),
                                        );
                                    },
                                );
                            });
                            // Mini bar
                            let bar_frac = *count as f32 / max_count as f32;
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 3.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(rect.width() * bar_frac, rect.height()),
                                ),
                                1.5,
                                theme::accent(dark),
                            );
                            ui.add_space(1.0);
                        }
                    });

                    ui.add_space(theme::SECTION_SPACING);

                    // ── Hourly histogram ────────────────────────────────
                    egui::CollapsingHeader::new(
                        egui::RichText::new("\u{1F552} Events per Hour").strong(),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        if stats.hourly_histogram.is_empty() {
                            ui.label(
                                egui::RichText::new("Insufficient data")
                                    .color(theme::text_dim(dark))
                                    .italics(),
                            );
                            return;
                        }

                        let max_count = stats
                            .hourly_histogram
                            .iter()
                            .map(|(_, c)| *c)
                            .max()
                            .unwrap_or(1)
                            .max(1);

                        let bar_height = 14.0;
                        let total_width = ui.available_width();

                        for (label, count) in &stats.hourly_histogram {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(label)
                                        .color(theme::text_dim(dark))
                                        .monospace()
                                        .small(),
                                );
                                let bar_frac = *count as f32 / max_count as f32;
                                let bar_width = (total_width - 80.0) * bar_frac;
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(bar_width.max(2.0), bar_height),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2.0, theme::accent_dim(dark));
                                ui.label(
                                    egui::RichText::new(count.to_string())
                                        .color(theme::text_secondary(dark))
                                        .small(),
                                );
                            });
                        }
                    });
                }); // ScrollArea
            });

        if !open {
            self.show_stats = false;
        }
    }
}
