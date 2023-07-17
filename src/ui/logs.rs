use bevy::prelude::{Query, Res, ResMut};
use bevy_egui::egui::{Hyperlink, ScrollArea, Slider, TextEdit, Widget, Window};
use bevy_egui::EguiContexts;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use syntect::parsing::Regex;

use macros::measured;

use crate::logs::{LogEntry, Logs, Pinned};
use crate::ui::debug::Performance;
use crate::ui::main_layout::UiState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoggingFilterType {
    Regex,
    Fuzzy,
}

#[measured]
pub fn render_logs(
    mut egui_context: EguiContexts,
    logs: Res<Logs>,
    pins: Query<&Pinned>,
    mut ui_state: ResMut<UiState>,
) {
    Window::new("Logs").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(
                TextEdit::singleline(&mut ui_state.logging_filter)
                    .desired_width(200.0)
                    .hint_text("type in something to search for:"),
            );
            ui.radio_value(&mut ui_state.logging_filter_type, LoggingFilterType::Regex, "regex");
            ui.radio_value(&mut ui_state.logging_filter_type, LoggingFilterType::Fuzzy, "fuzzy");
        });
        if ui_state.regex_error.is_some() {
            ui.label(format!("Regex error: {}", ui_state.regex_error.as_ref().unwrap()));
        }
        ui.collapsing("Instructions & settings", |ui| {
            ui.label("Click on a 'P' button in other windows to pin entites. Only pinned entities will be shown here. Click 'U' button to unpin entities. Clicking on 'Pin' column header will list only pinned entities without changing sorting selected");
            ui.label("Click on 'fuzzy' button to enable fuzzy case insensitive filtering.");
            ui.horizontal(|ui| {
                ui.label("Click on 'regex' button to enable regex filtering. To learn more about regex visit: ");
                let link = Hyperlink::from_label_and_url(
                    "here",
                    "https://regexr.com/",
                );
                link.ui(ui);
                let link = Hyperlink::from_label_and_url(
                    " or here",
                    "https://regexone.com/",
                );
                link.ui(ui);
            });
            ui.add(Slider::new(&mut ui_state.fuzzy_match_threshold, 0..=100).text("Fuzzy match threshold"));
        });
        ScrollArea::vertical().show(ui, |ui| {
            let shown_logs = filter_logs(&logs.entries, &mut ui_state, pins);
            let mut log_text = shown_logs
                .iter()
                .map(|log| format!("Day: {} | {}", log.day, log.entry.text.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            TextEdit::multiline(&mut log_text)
                .desired_rows(10)
                .lock_focus(true)
                .interactive(false)
                .desired_width(f32::INFINITY)
                .show(ui);
        });
    });
}

fn filter_logs<'a>(
    logs: &'a [LogEntry],
    ui_state: &'a mut UiState,
    pins: Query<'a, 'a, &Pinned>,
) -> Vec<&'a LogEntry> {
    match ui_state.logging_filter_type {
        LoggingFilterType::Regex => match Regex::try_compile(&ui_state.logging_filter) {
            Some(e) => {
                ui_state.regex_error = Some(format!("Invalid regex: {}", e));
                vec![]
            }
            None => {
                let regex = Regex::new(ui_state.logging_filter.clone());
                ui_state.regex_error = None;
                logs.iter()
                    .filter(|log| {
                        pins.get(log.entry.entity).is_ok() && regex.is_match(&log.entry.text)
                    })
                    .collect::<Vec<_>>()
            }
        },
        LoggingFilterType::Fuzzy => {
            ui_state.regex_error = None;
            logs.iter()
                .filter(|log| {
                    pins.get(log.entry.entity).is_ok()
                        && (is_fuzzy_match(
                            &log.entry.text.to_ascii_lowercase(),
                            &ui_state.logging_filter.to_ascii_lowercase(),
                            ui_state,
                        ) || ui_state.logging_filter.is_empty())
                })
                .collect::<Vec<&LogEntry>>()
        }
    }
}

pub fn is_fuzzy_match(haystack: &str, needle: &str, ui_state: &UiState) -> bool {
    let matcher = SkimMatcherV2::default();
    if let Some(score) = matcher.fuzzy_match(haystack, needle) {
        // you might want to adjust the score threshold according to your needs
        return score > ui_state.fuzzy_match_threshold;
    }
    false
}
