use crate::app::{PopupState, WritingToolKind, DEFAULT_TOOLS, TONES};
use crate::selection::SelectionContext;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length, Theme};

use super::popup::Message;

/// Render the popup content based on current state.
pub fn popup_content<'a>(state: &'a PopupState, copy_feedback: bool) -> Element<'a, Message> {
    let content: Element<'a, Message> = match state {
        PopupState::Hidden => Space::new(0, 0).into(),
        PopupState::Options(ctx) => options_view(ctx),
        PopupState::ToneSelection(_) => tone_selection_view(),
        PopupState::Processing { tool, .. } => processing_view(tool),
        PopupState::Result { output, .. } => result_view(output, copy_feedback),
        PopupState::Error { message, .. } => error_view(message),
    };

    container(content).width(Length::Fill).padding(12).into()
}

// -- Options view --

fn options_view(selection: &SelectionContext) -> Element<'_, Message> {
    let preview = preview_text(&selection.text);

    let mut col = column![].spacing(8);

    // Header with preview + close
    let header = row![
        text(preview).size(12).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.6, 0.6, 0.6)),
        }),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .align_y(Alignment::Center);

    col = col.push(header);

    // Tool buttons
    for tool_def in DEFAULT_TOOLS {
        let kind = match tool_def.id {
            "enhance-writing" => WritingToolKind::EnhanceWriting,
            "fix-grammar" => WritingToolKind::FixGrammar,
            "change-tone" => WritingToolKind::ChangeTone(TONES[0].clone()),
            _ => continue,
        };

        col = col.push(tool_button(tool_def.title, kind));
    }

    col.into()
}

fn tool_button(label: &str, kind: WritingToolKind) -> Element<'_, Message> {
    let content = row![
        text(label).size(14),
        Space::with_width(Length::Fill),
        text("›").size(12).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
        }),
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    button(content)
        .on_press(Message::ToolSelected(kind))
        .width(Length::Fill)
        .style(|_theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(0.5, 0.5, 0.5, 0.12),
                _ => Color::from_rgba(0.5, 0.5, 0.5, 0.06),
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn tone_button(label: &str, tone: crate::app::ToneOption) -> Element<'_, Message> {
    let content = row![text(label).size(14),]
        .align_y(Alignment::Center)
        .padding([8, 10]);

    button(content)
        .on_press(Message::ToneSelected(tone))
        .width(Length::Fill)
        .style(|_theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(0.5, 0.5, 0.5, 0.12),
                _ => Color::from_rgba(0.5, 0.5, 0.5, 0.06),
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

// -- Tone selection view --

fn tone_selection_view() -> Element<'static, Message> {
    let mut col = column![].spacing(8);

    // Header with back + title + close
    let header = row![
        button(text("‹ Back").size(12))
            .on_press(Message::BackToOptions)
            .style(|_theme: &Theme, _status| button::Style {
                background: None,
                text_color: Color::from_rgb(0.6, 0.6, 0.6),
                ..Default::default()
            }),
        Space::with_width(Length::Fill),
        text("Select Tone").size(14),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .align_y(Alignment::Center);

    col = col.push(header);

    for tone in TONES {
        col = col.push(tone_button(tone.name, tone.clone()));
    }

    col.into()
}

// -- Processing view --

fn processing_view(tool: &WritingToolKind) -> Element<'_, Message> {
    let label = format!("{}…", tool);

    row![
        text("⟳").size(14),
        text(label).size(14).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.6, 0.6, 0.6)),
        }),
        Space::with_width(Length::Fill),
        button(text("Cancel").size(12))
            .on_press(Message::Cancel)
            .style(|_theme: &Theme, _status| button::Style {
                background: None,
                text_color: Color::from_rgb(0.8, 0.3, 0.3),
                ..Default::default()
            }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([4, 0])
    .into()
}

// -- Result view --

fn result_view<'a>(output: &'a str, copy_feedback: bool) -> Element<'a, Message> {
    let header = row![
        text("✓").size(14).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.2, 0.8, 0.2)),
        }),
        text("Done").size(14),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let result_text = scrollable(text(output).size(13).width(Length::Fill)).height(Length::Shrink);

    let copy_label = if copy_feedback { "Copied ✓" } else { "Copy" };
    let actions = row![
        action_button(copy_label, Message::CopyResult, ActionRole::Primary),
        action_button("Replace", Message::ReplaceResult, ActionRole::Secondary),
    ]
    .spacing(8);

    column![header, result_text, actions].spacing(10).into()
}

// -- Error view --

fn error_view(message: &str) -> Element<'_, Message> {
    let header = row![
        text("⚠").size(14).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.9, 0.6, 0.1)),
        }),
        text("Error").size(14),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let msg = text(message).size(12).style(|_theme: &Theme| text::Style {
        color: Some(Color::from_rgb(0.6, 0.6, 0.6)),
    });

    let retry_btn = button(text("Retry").size(12))
        .on_press(Message::Retry)
        .style(|_theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(0.3, 0.5, 0.9, 0.2),
                _ => Color::from_rgba(0.3, 0.5, 0.9, 0.12),
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: Color::from_rgb(0.4, 0.6, 1.0),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    column![header, msg, retry_btn].spacing(8).into()
}

// -- Shared components --

fn close_button() -> Element<'static, Message> {
    button(text("✕").size(12))
        .on_press(Message::Dismiss)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            text_color: Color::from_rgb(0.5, 0.5, 0.5),
            ..Default::default()
        })
        .into()
}

#[derive(Clone, Copy)]
enum ActionRole {
    Primary,
    Secondary,
}

fn action_button(label: &str, msg: Message, role: ActionRole) -> Element<'_, Message> {
    let content = text(label).size(12).center();

    button(content)
        .on_press(msg)
        .width(Length::Fill)
        .style(move |_theme: &Theme, status| {
            let (bg_base, fg) = match role {
                ActionRole::Primary => (
                    Color::from_rgb(0.3, 0.5, 0.9),
                    Color::from_rgb(0.4, 0.6, 1.0),
                ),
                ActionRole::Secondary => (
                    Color::from_rgb(0.5, 0.5, 0.5),
                    Color::from_rgb(0.8, 0.8, 0.8),
                ),
            };
            let alpha = match status {
                button::Status::Hovered => 0.2,
                _ => 0.12,
            };
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    bg_base.r, bg_base.g, bg_base.b, alpha,
                ))),
                text_color: fg,
                border: iced::Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn preview_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let one_line = trimmed.replace('\n', " ");
    if one_line.chars().count() > 60 {
        let end = one_line
            .char_indices()
            .nth(57)
            .map(|(i, _)| i)
            .unwrap_or(one_line.len());
        format!("{}…", &one_line[..end])
    } else {
        one_line
    }
}
