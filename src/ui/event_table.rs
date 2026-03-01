//! Central event table with virtual scrolling and sortable columns.
//!
//! Uses `egui_extras::TableBuilder` for column layout, which provides
//! built-in virtual scrolling via its `body.rows()` method -- only
//! visible rows are laid out, keeping performance smooth with 100k+ events.

use crate::app::{EventSleuthApp, SortColumn};
use crate::ui::theme;
use crate::util::time::format_table_timestamp;
use egui_extras::{Column, TableBuilder};

impl EventSleuthApp {
    /// Render the virtual-scrolled event table in the central panel.
    ///
    /// Columns: Timestamp, Level, ID, Provider, Message.
    /// Clicking a header sorts by that column (toggle asc/desc).
    /// Clicking a row selects it and shows details.
    ///
    /// When there are no events to display an empty-state message is
    /// shown instead of a blank area, helping first-time users understand
    /// what to do next.
    pub fn render_event_table(&mut self, ui: &mut egui::Ui) {
        let row_count = self.filtered_indices.len();

        // ── Empty state ─────────────────────────────────────────────
        if row_count == 0 {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() / 3.0);
                if self.is_loading {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Loading events...")
                            .color(theme::text_dim(self.dark_mode))
                            .size(15.0),
                    );
                } else if self.all_events.is_empty() {
                    ui.label(
                        egui::RichText::new("\u{1F4CB}")
                            .size(32.0)
                            .color(theme::text_dim(self.dark_mode)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("No events loaded")
                            .color(theme::text_secondary(self.dark_mode))
                            .size(15.0),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Select sources and click Refresh, or open an .evtx file.",
                        )
                        .color(theme::text_dim(self.dark_mode)),
                    );
                } else {
                    // Events loaded but all filtered out
                    ui.label(
                        egui::RichText::new("\u{1F50D}")
                            .size(32.0)
                            .color(theme::text_dim(self.dark_mode)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("No events match current filters")
                            .color(theme::text_secondary(self.dark_mode))
                            .size(15.0),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Try broadening your filters or clearing them.")
                            .color(theme::text_dim(self.dark_mode)),
                    );
                }
            });
            return;
        }

        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(24.0).clip(false)); // Bookmark pin

        // Conditionally add columns based on visibility settings
        let cv = &self.column_visibility;
        let table = if cv.timestamp {
            table.column(Column::auto().at_least(145.0).clip(true))
        } else {
            table
        };
        let table = if cv.level {
            table.column(Column::auto().at_least(85.0).clip(true))
        } else {
            table
        };
        let table = if cv.event_id {
            table.column(Column::auto().at_least(55.0))
        } else {
            table
        };
        let table = if cv.provider {
            table.column(Column::auto().at_least(140.0).clip(true))
        } else {
            table
        };
        let table = if cv.channel {
            table.column(Column::auto().at_least(120.0).clip(true))
        } else {
            table
        };
        let table = if cv.computer {
            table.column(Column::auto().at_least(100.0).clip(true))
        } else {
            table
        };
        let table = if cv.message {
            table.column(Column::remainder().clip(true))
        } else {
            table
        };
        let table = table.sense(egui::Sense::click());

        // Collect bookmark toggles to apply after table rendering
        // (the row closure borrows self immutably for `all_events` access).
        let bookmark_toggle = std::cell::Cell::new(None::<usize>);

        table
            .header(22.0, |mut header| {
                // Copy visibility flags to avoid borrowing self alongside the mutable closure
                let show_timestamp = self.column_visibility.timestamp;
                let show_level = self.column_visibility.level;
                let show_event_id = self.column_visibility.event_id;
                let show_provider = self.column_visibility.provider;
                let show_channel = self.column_visibility.channel;
                let show_computer = self.column_visibility.computer;
                let show_message = self.column_visibility.message;

                // Bookmark column header (pin icon)
                header.col(|ui| {
                    ui.label(
                        egui::RichText::new("\u{2B50}")
                            .small()
                            .color(theme::text_dim(self.dark_mode)),
                    )
                    .on_hover_text("Bookmarked events");
                });
                if show_timestamp {
                    header.col(|ui| {
                        self.render_sort_header(ui, SortColumn::Timestamp, "Timestamp");
                    });
                }
                if show_level {
                    header.col(|ui| {
                        self.render_sort_header(ui, SortColumn::Level, "Level");
                    });
                }
                if show_event_id {
                    header.col(|ui| {
                        self.render_sort_header(ui, SortColumn::EventId, "ID");
                    });
                }
                if show_provider {
                    header.col(|ui| {
                        self.render_sort_header(ui, SortColumn::Provider, "Provider");
                    });
                }
                if show_channel {
                    header.col(|ui| {
                        ui.label(
                            egui::RichText::new("Channel")
                                .color(theme::text_primary(self.dark_mode)),
                        );
                    });
                }
                if show_computer {
                    header.col(|ui| {
                        ui.label(
                            egui::RichText::new("Computer")
                                .color(theme::text_primary(self.dark_mode)),
                        );
                    });
                }
                if show_message {
                    header.col(|ui| {
                        self.render_sort_header(ui, SortColumn::Message, "Message");
                    });
                }
            })
            .body(|body| {
                body.rows(theme::TABLE_ROW_HEIGHT, row_count, |mut row| {
                    let visible_idx = row.index();
                    if visible_idx >= self.filtered_indices.len() {
                        return;
                    }
                    let event_idx = self.filtered_indices[visible_idx];
                    let event = &self.all_events[event_idx];
                    let is_selected = self.selected_event_idx == Some(visible_idx);
                    let dark = self.dark_mode;
                    let level_color = theme::level_color(event.level, dark);
                    let is_bookmarked = self.bookmarked_indices.contains(&event_idx);

                    row.set_selected(is_selected);

                    // Bookmark pin
                    row.col(|ui| {
                        let icon = if is_bookmarked {
                            "\u{2B50}"
                        } else {
                            "\u{2606}"
                        };
                        let btn = ui.add(
                            egui::Button::new(egui::RichText::new(icon).size(12.0).color(
                                if is_bookmarked {
                                    theme::accent(dark)
                                } else {
                                    theme::text_dim(dark)
                                },
                            ))
                            .frame(false),
                        );
                        if btn.clicked() {
                            bookmark_toggle.set(Some(event_idx));
                        }
                        btn.on_hover_text(if is_bookmarked {
                            "Remove bookmark"
                        } else {
                            "Bookmark this event"
                        });
                    });

                    let cv = &self.column_visibility;

                    // Timestamp
                    if cv.timestamp {
                        row.col(|ui| {
                            ui.label(
                                egui::RichText::new(format_table_timestamp(&event.timestamp))
                                    .color(theme::text_secondary(dark))
                                    .small(),
                            );
                        });
                    }

                    // Level (colour-coded)
                    if cv.level {
                        row.col(|ui| {
                            ui.label(egui::RichText::new(&event.level_name).color(level_color));
                        });
                    }

                    // Event ID
                    if cv.event_id {
                        row.col(|ui| {
                            ui.label(event.event_id.to_string());
                        });
                    }

                    // Provider
                    if cv.provider {
                        row.col(|ui| {
                            ui.label(
                                egui::RichText::new(&event.provider_name)
                                    .color(theme::text_secondary(dark)),
                            );
                        });
                    }

                    // Channel
                    if cv.channel {
                        row.col(|ui| {
                            ui.label(
                                egui::RichText::new(&event.channel)
                                    .color(theme::text_secondary(dark)),
                            );
                        });
                    }

                    // Computer
                    if cv.computer {
                        row.col(|ui| {
                            ui.label(
                                egui::RichText::new(&event.computer)
                                    .color(theme::text_secondary(dark)),
                            );
                        });
                    }

                    // Message (truncated to one line)
                    if cv.message {
                        row.col(|ui| {
                            let msg = event.display_message();
                            if msg.len() <= 200 {
                                ui.label(msg);
                            } else {
                                let end = msg
                                    .char_indices()
                                    .nth(200)
                                    .map(|(i, _)| i)
                                    .unwrap_or(msg.len());
                                if end < msg.len() {
                                    ui.label(format!("{}...", &msg[..end]));
                                } else {
                                    ui.label(msg);
                                }
                            }
                        });
                    }

                    if row.response().clicked() {
                        self.selected_event_idx = Some(visible_idx);
                    }
                });
            });

        // Apply deferred bookmark toggle
        if let Some(idx) = bookmark_toggle.get() {
            if self.bookmarked_indices.contains(&idx) {
                self.bookmarked_indices.remove(&idx);
            } else {
                self.bookmarked_indices.insert(idx);
            }
            // If in bookmarks-only mode, refilter to update the view
            if self.show_bookmarks_only {
                self.needs_refilter = true;
            }
        }
    }

    /// Render a sortable column header button.
    ///
    /// Shows an arrow indicator for the current sort column and toggles
    /// direction on click. Tooltip explains the interaction.
    fn render_sort_header(&mut self, ui: &mut egui::Ui, column: SortColumn, label: &str) {
        let is_current = self.sort_column == column;
        let arrow = if is_current {
            if self.sort_ascending {
                " \u{25B2}"
            } else {
                " \u{25BC}"
            }
        } else {
            ""
        };

        let text = format!("{label}{arrow}");
        let dark = self.dark_mode;
        let rich = if is_current {
            egui::RichText::new(text)
                .color(theme::accent(dark))
                .strong()
        } else {
            egui::RichText::new(text).color(theme::text_primary(dark))
        };

        if ui
            .button(rich)
            .on_hover_text(if is_current {
                "Click to reverse sort order"
            } else {
                "Click to sort by this column"
            })
            .clicked()
        {
            if is_current {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = column;
                self.sort_ascending = column != SortColumn::Timestamp;
            }
            self.sort_events();
        }
    }
}
