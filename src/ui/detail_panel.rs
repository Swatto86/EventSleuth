//! Detail panel: displays all fields of the currently selected event.
//!
//! Provides two tabs — **Details** (formatted view with event data table)
//! and **XML** (raw XML string in a monospaced scrollable area).
//!
//! When a text search is active, matching substrings are highlighted with
//! a contrasting background colour via `egui::text::LayoutJob`.

use crate::app::{DetailTab, EventSleuthApp};
use crate::ui::theme;
use crate::util::time::format_detail_timestamp;

impl EventSleuthApp {
    /// Render the bottom detail panel for the currently selected event.
    pub fn render_detail_panel(&mut self, ui: &mut egui::Ui) {
        let event = match self.selected_event() {
            Some(e) => e.clone(),
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.label(
                        egui::RichText::new("\u{1F446} Select an event above to view its details")
                            .color(theme::text_dim(self.dark_mode))
                            .size(13.0),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Tip: use \u{2191}/\u{2193} arrow keys to navigate")
                            .color(theme::text_dim(self.dark_mode))
                            .small(),
                    );
                });
                return;
            }
        };

        // \u{2500}\u{2500} Tab bar \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.detail_tab,
                DetailTab::Details,
                egui::RichText::new("\u{1F4DD} Details").strong(),
            );
            ui.selectable_value(
                &mut self.detail_tab,
                DetailTab::Xml,
                egui::RichText::new("\u{1F4C4} XML").strong(),
            );

            // Copy actions and bookmark toggle grouped on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("\u{1F4CB} XML")
                    .on_hover_text("Copy the raw XML to the clipboard")
                    .clicked()
                {
                    ui.ctx().copy_text(event.raw_xml.clone());
                }
                if ui
                    .small_button("\u{1F4CB} Message")
                    .on_hover_text("Copy the formatted message to the clipboard")
                    .clicked()
                {
                    ui.ctx().copy_text(event.message.clone());
                }
                if ui
                    .small_button("\u{1F4CB} ID")
                    .on_hover_text("Copy the Event ID to the clipboard")
                    .clicked()
                {
                    ui.ctx().copy_text(event.event_id.to_string());
                }
                ui.label(
                    egui::RichText::new("Copy:")
                        .color(theme::text_dim(self.dark_mode))
                        .small(),
                );
                ui.separator();
                // Bookmark toggle for the selected event
                if let Some(vis_idx) = self.selected_event_idx {
                    if let Some(&ev_idx) = self.filtered_indices.get(vis_idx) {
                        let is_bookmarked = self.bookmarked_indices.contains(&ev_idx);
                        let pin_icon = if is_bookmarked {
                            "\u{2B50} Unpin"
                        } else {
                            "\u{2606} Pin"
                        };
                        if ui
                            .small_button(pin_icon)
                            .on_hover_text(if is_bookmarked {
                                "Remove bookmark"
                            } else {
                                "Bookmark this event for later reference"
                            })
                            .clicked()
                        {
                            if is_bookmarked {
                                self.bookmarked_indices.remove(&ev_idx);
                            } else {
                                self.bookmarked_indices.insert(ev_idx);
                            }
                        }
                    }
                }
            });
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| match self.detail_tab {
            DetailTab::Details => self.render_detail_formatted(ui, &event),
            DetailTab::Xml => self.render_detail_xml(ui, &event),
        });
    }

    /// Render the formatted details view: header fields, message, event data.
    fn render_detail_formatted(
        &self,
        ui: &mut egui::Ui,
        event: &crate::core::event_record::EventRecord,
    ) {
        let dark = self.dark_mode;
        let level_color = theme::level_color(event.level, dark);

        // ── Header grid ─────────────────────────────────────────────
        egui::Grid::new("detail_header_grid")
            .num_columns(4)
            .striped(false)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                // Row 1
                ui.label(egui::RichText::new("Event ID").color(theme::text_dim(dark)));
                ui.label(event.event_id.to_string());
                ui.label(egui::RichText::new("Level").color(theme::text_dim(dark)));
                ui.label(egui::RichText::new(&event.level_name).color(level_color));
                ui.end_row();

                // Row 2
                ui.label(egui::RichText::new("Provider").color(theme::text_dim(dark)));
                ui.label(&event.provider_name);
                ui.label(egui::RichText::new("Channel").color(theme::text_dim(dark)));
                ui.label(&event.channel);
                ui.end_row();

                // Row 3
                ui.label(egui::RichText::new("Timestamp").color(theme::text_dim(dark)));
                ui.label(format_detail_timestamp(&event.timestamp));
                ui.label(egui::RichText::new("Computer").color(theme::text_dim(dark)));
                ui.label(&event.computer);
                ui.end_row();

                // Row 4
                ui.label(egui::RichText::new("Process ID").color(theme::text_dim(dark)));
                ui.label(event.process_id.to_string());
                ui.label(egui::RichText::new("Thread ID").color(theme::text_dim(dark)));
                ui.label(event.thread_id.to_string());
                ui.end_row();

                // Row 5 (optional fields)
                if let Some(ref sid) = event.user_sid {
                    ui.label(egui::RichText::new("User SID").color(theme::text_dim(dark)));
                    ui.label(sid);
                } else {
                    ui.label("");
                    ui.label("");
                }
                if let Some(ref aid) = event.activity_id {
                    ui.label(egui::RichText::new("Activity ID").color(theme::text_dim(dark)));
                    ui.label(aid);
                } else {
                    ui.label("");
                    ui.label("");
                }
                ui.end_row();
            });

        ui.add_space(8.0);

        // ── Message ─────────────────────────────────────────────────
        ui.label(
            egui::RichText::new("💬 Message")
                .color(theme::accent(dark))
                .strong(),
        );
        ui.separator();

        let msg = event.display_message();
        if msg.is_empty() || msg == "(no message)" {
            ui.label(
                egui::RichText::new("(no formatted message available)")
                    .color(theme::text_dim(dark))
                    .italics(),
            );
        } else {
            // Render with search-match highlighting when a search is active
            let search = &self.filter.text_search;
            if search.is_empty() {
                ui.label(
                    egui::RichText::new(msg)
                        .color(theme::text_primary(dark))
                        .size(13.0),
                );
            } else {
                let job = Self::build_highlighted_job(
                    msg,
                    search,
                    self.filter.case_sensitive,
                    13.0,
                    false,
                    dark,
                );
                ui.label(job);
            }
        }

        // ── Event Data table ────────────────────────────────────────
        if !event.event_data.is_empty() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("📊 Event Data")
                    .color(theme::accent(dark))
                    .strong(),
            );
            ui.separator();

            egui::Grid::new("event_data_grid")
                .num_columns(2)
                .striped(true)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    // Header row
                    ui.label(
                        egui::RichText::new("Name")
                            .color(theme::text_dim(dark))
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Value")
                            .color(theme::text_dim(dark))
                            .strong(),
                    );
                    ui.end_row();

                    for (key, value) in &event.event_data {
                        ui.label(egui::RichText::new(key).color(theme::text_secondary(dark)));
                        // Wrap long values (char-safe truncation)
                        let display = if value.chars().count() > 500 {
                            let end = value
                                .char_indices()
                                .nth(500)
                                .map(|(i, _)| i)
                                .unwrap_or(value.len());
                            format!("{}... ({} chars)", &value[..end], value.chars().count())
                        } else {
                            value.clone()
                        };
                        // Highlight search matches in event data values
                        let search = &self.filter.text_search;
                        if search.is_empty() {
                            ui.label(&display);
                        } else {
                            let job = Self::build_highlighted_job(
                                &display,
                                search,
                                self.filter.case_sensitive,
                                13.0,
                                false,
                                dark,
                            );
                            ui.label(job);
                        }
                        ui.end_row();
                    }
                });
        }
    }

    /// Render the raw XML view with monospace font in a scrollable area.
    /// Search matches are highlighted when a text search is active.
    fn render_detail_xml(&self, ui: &mut egui::Ui, event: &crate::core::event_record::EventRecord) {
        let dark = self.dark_mode;
        let search = &self.filter.text_search;
        if search.is_empty() {
            ui.label(
                egui::RichText::new(&event.raw_xml)
                    .monospace()
                    .size(12.0)
                    .color(theme::text_secondary(dark)),
            );
        } else {
            let job = Self::build_highlighted_job(
                &event.raw_xml,
                search,
                self.filter.case_sensitive,
                12.0,
                true,
                dark,
            );
            ui.label(job);
        }
    }

    /// Build a [`egui::text::LayoutJob`] that renders `text` with
    /// highlighted search-match segments.
    ///
    /// Non-matching text uses [`theme::text_primary`] (or [`theme::text_secondary`]
    /// for monospace). Matched substrings get a [`theme::highlight_bg`]
    /// background and [`theme::highlight_text`] foreground.
    fn build_highlighted_job(
        text: &str,
        search: &str,
        case_sensitive: bool,
        font_size: f32,
        monospace: bool,
        dark: bool,
    ) -> egui::text::LayoutJob {
        use egui::text::{LayoutJob, LayoutSection};
        use egui::{FontFamily, FontId, TextFormat};

        let family = if monospace {
            FontFamily::Monospace
        } else {
            FontFamily::Proportional
        };
        let font_id = FontId::new(font_size, family);

        let normal_fmt = TextFormat {
            font_id: font_id.clone(),
            color: if monospace {
                theme::text_secondary(dark)
            } else {
                theme::text_primary(dark)
            },
            ..Default::default()
        };

        let highlight_fmt = TextFormat {
            font_id,
            color: theme::highlight_text(dark),
            background: theme::highlight_bg(dark),
            ..Default::default()
        };

        let mut job = LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        job.text = text.to_owned();

        if search.is_empty() {
            job.sections.push(LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format: normal_fmt,
            });
            return job;
        }

        // Find all match positions
        if case_sensitive {
            // Case-sensitive: byte positions in the original text are
            // used directly -- no mapping needed.
            let needle_len = search.len();
            let mut pos = 0usize;
            loop {
                match text[pos..].find(search) {
                    Some(rel_start) => {
                        let abs_start = pos + rel_start;
                        if abs_start > pos {
                            job.sections.push(LayoutSection {
                                leading_space: 0.0,
                                byte_range: pos..abs_start,
                                format: normal_fmt.clone(),
                            });
                        }
                        job.sections.push(LayoutSection {
                            leading_space: 0.0,
                            byte_range: abs_start..abs_start + needle_len,
                            format: highlight_fmt.clone(),
                        });
                        pos = abs_start + needle_len;
                    }
                    None => {
                        if pos < text.len() {
                            job.sections.push(LayoutSection {
                                leading_space: 0.0,
                                byte_range: pos..text.len(),
                                format: normal_fmt.clone(),
                            });
                        }
                        break;
                    }
                }
            }
        } else {
            // Case-insensitive: build a byte-position mapping from the
            // lowered text back to the original text.  `to_lowercase()`
            // can change byte lengths for certain Unicode code-points
            // (e.g. U+0130 LATIN CAPITAL LETTER I WITH DOT ABOVE), so
            // raw lowered-text byte offsets are NOT valid for `job.text`
            // which contains the original (un-lowered) text.
            let search_lower = search.to_lowercase();
            let mut lowered = String::with_capacity(text.len());
            let mut low_to_orig: Vec<usize> = Vec::with_capacity(text.len() + 1);
            let mut orig_pos = 0usize;
            for ch in text.chars() {
                let orig_len = ch.len_utf8();
                for lc in ch.to_lowercase() {
                    for _ in 0..lc.len_utf8() {
                        low_to_orig.push(orig_pos);
                    }
                    lowered.push(lc);
                }
                orig_pos += orig_len;
            }
            low_to_orig.push(orig_pos); // sentinel for end-of-string

            let needle_len = search_lower.len();
            let mut pos = 0usize;
            loop {
                match lowered[pos..].find(search_lower.as_str()) {
                    Some(rel_start) => {
                        let abs_start = pos + rel_start;
                        if abs_start > pos {
                            job.sections.push(LayoutSection {
                                leading_space: 0.0,
                                byte_range: low_to_orig[pos]..low_to_orig[abs_start],
                                format: normal_fmt.clone(),
                            });
                        }
                        job.sections.push(LayoutSection {
                            leading_space: 0.0,
                            byte_range: low_to_orig[abs_start]..low_to_orig[abs_start + needle_len],
                            format: highlight_fmt.clone(),
                        });
                        pos = abs_start + needle_len;
                    }
                    None => {
                        if pos < lowered.len() {
                            job.sections.push(LayoutSection {
                                leading_space: 0.0,
                                byte_range: low_to_orig[pos]..text.len(),
                                format: normal_fmt.clone(),
                            });
                        }
                        break;
                    }
                }
            }
        }

        job
    }
}
