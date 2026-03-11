use crate::app::{
    PopupState, TranslateLanguage, WritingToolKind, DEFAULT_TOOLS, TONES, TRANSLATE_LANGS,
};
use crate::selection::SelectionContext;
use crate::ui::theme;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_editor, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector};

use super::popup::Message;

/// Render the popup content based on current state.
pub fn popup_content<'a>(
    state: &'a PopupState,
    copy_feedback: bool,
    spinner_tick: usize,
    translate_from: &'a TranslateLanguage,
    translate_to: &'a TranslateLanguage,
    translate_original_content: &'a text_editor::Content,
    translate_output_text: &'a str,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = match state {
        PopupState::Hidden => Space::new(0, 0).into(),
        PopupState::Options(ctx) => options_view(ctx),
        PopupState::ToneSelection(_) => tone_selection_view(),
        PopupState::TranslateSelection(_) => translate_selection_view(translate_from, translate_to),
        PopupState::TranslateResult { .. } => translate_result_view(
            translate_original_content,
            translate_output_text,
            translate_from,
            translate_to,
            copy_feedback,
        ),
        PopupState::Processing { tool, .. } => processing_view(tool, spinner_tick),
        PopupState::Result { output, .. } => result_view(output, copy_feedback),
        PopupState::Error { message, .. } => error_view(message),
    };

    container(content)
        .width(Length::Fill)
        .padding(12)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE)),
            border: Border {
                color: theme::SURFACE_BORDER,
                width: 1.0,
                radius: theme::RADIUS_LG.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            ..Default::default()
        })
        .into()
}

// -- Options view --

fn options_view(selection: &SelectionContext) -> Element<'_, Message> {
    let preview = preview_text(&selection.text);

    let mut col = column![].spacing(8);

    // Header with preview + close
    let header = row![
        text(preview).size(12).style(|_theme: &Theme| text::Style {
            color: Some(theme::TEXT_SECONDARY),
        }),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .align_y(Alignment::Center);

    col = col.push(header);
    col = col.push(separator());

    // Tool buttons
    for tool_def in DEFAULT_TOOLS {
        let kind = match tool_def.id {
            "enhance-writing" => WritingToolKind::EnhanceWriting,
            "fix-grammar" => WritingToolKind::FixGrammar,
            "change-tone" => WritingToolKind::ChangeTone(TONES[0].clone()),
            "translate" => WritingToolKind::Translate {
                from: TRANSLATE_LANGS[0].clone(),
                to: TRANSLATE_LANGS[1].clone(),
            },
            _ => continue,
        };

        col = col.push(tool_button(tool_def.title, kind));
    }

    col.into()
}

fn tool_button<'a>(label: &'a str, kind: WritingToolKind) -> Element<'a, Message> {
    let has_submenu = matches!(
        kind,
        WritingToolKind::ChangeTone(_) | WritingToolKind::Translate { .. }
    );
    let arrow = if has_submenu { ">" } else { "" };

    let content = row![
        text(label).size(14),
        Space::with_width(Length::Fill),
        text(arrow).size(12).style(|_theme: &Theme| text::Style {
            color: Some(theme::TEXT_MUTED),
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([10, 14]);

    button(content)
        .on_press(Message::ToolSelected(kind))
        .width(Length::Fill)
        .style(|_theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(0.231, 0.510, 0.965, 0.12),
                _ => Color::from_rgba(0.5, 0.5, 0.5, 0.06),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn tone_button<'a>(label: &'a str, tone: crate::app::ToneOption) -> Element<'a, Message> {
    let content = row![text(label).size(14)]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([10, 14]);

    button(content)
        .on_press(Message::ToneSelected(tone))
        .width(Length::Fill)
        .style(|_theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(0.231, 0.510, 0.965, 0.12),
                _ => Color::from_rgba(0.5, 0.5, 0.5, 0.06),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: Border {
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
    // Header with back + title + close
    let header = row![
        back_button(),
        Space::with_width(Length::Fill),
        text("Select Tone").size(14),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .align_y(Alignment::Center);

    let mut tone_col = column![].spacing(6);
    for tone in TONES {
        tone_col = tone_col.push(tone_button(tone.name, tone.clone()));
    }

    column![header, scrollable(tone_col).height(Length::Fill)]
        .spacing(8)
        .into()
}

// -- Translate selection view --

fn translate_selection_view<'a>(
    from: &'a TranslateLanguage,
    to: &'a TranslateLanguage,
) -> Element<'a, Message> {
    let header = row![
        back_button(),
        Space::with_width(Length::Fill),
        text("Translate").size(14),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .align_y(Alignment::Center);

    let lang_row = translate_lang_row(from, to);

    let valid = to.id != "auto" && from != to;

    let translate_btn = button(text("Translate").size(13).center().width(Length::Fill))
        .on_press_maybe(valid.then_some(Message::TranslateConfirm))
        .width(Length::Fill)
        .style(|_theme: &Theme, status| {
            let alpha = match status {
                button::Status::Hovered => 0.2,
                _ => 0.12,
            };
            button::Style {
                background: Some(Background::Color(Color::from_rgba(
                    theme::PRIMARY.r,
                    theme::PRIMARY.g,
                    theme::PRIMARY.b,
                    alpha,
                ))),
                text_color: theme::PRIMARY_LIGHT,
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    column![header, separator(), lang_row, translate_btn]
        .spacing(10)
        .into()
}

// -- Translate result view --

fn translate_result_view<'a>(
    original_content: &'a text_editor::Content,
    translated: &'a str,
    from: &'a TranslateLanguage,
    to: &'a TranslateLanguage,
    copy_feedback: bool,
) -> Element<'a, Message> {
    let header = row![
        back_button(),
        Space::with_width(Length::Fill),
        text("Translate").size(14),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .align_y(Alignment::Center);

    let lang_row = translate_lang_row(from, to);

    let original_editor = column![
        text("Original").size(11).style(|_t: &Theme| text::Style {
            color: Some(theme::TEXT_MUTED),
        }),
        container(
            text_editor(original_content)
                .on_action(Message::TranslateOriginalAction)
                .height(60)
                .style(|_theme: &Theme, _status| text_editor::Style {
                    background: Background::Color(theme::SURFACE_ELEVATED),
                    border: Border {
                        radius: 6.0.into(),
                        width: 1.0,
                        color: theme::SURFACE_BORDER,
                    },
                    icon: theme::TEXT_MUTED,
                    placeholder: theme::TEXT_MUTED,
                    value: theme::TEXT_PRIMARY,
                    selection: Color::from_rgba(0.231, 0.510, 0.965, 0.3),
                })
        ),
    ]
    .spacing(3);

    let translated_box = column![
        text("Translated").size(11).style(|_t: &Theme| text::Style {
            color: Some(theme::TEXT_MUTED),
        }),
        container(scrollable(text(translated).size(13).width(Length::Fill)).height(60))
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::SURFACE_ELEVATED)),
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: theme::SURFACE_BORDER,
                },
                ..Default::default()
            })
            .padding(8)
            .width(Length::Fill),
    ]
    .spacing(3);

    let valid = to.id != "auto" && from != to;

    let retranslate_btn = button(text("Translate").size(12).center().width(Length::Fill))
        .on_press_maybe(valid.then_some(Message::TranslateConfirm))
        .width(Length::Fill)
        .style(|_theme: &Theme, status| {
            let alpha = match status {
                button::Status::Hovered => 0.2,
                _ => 0.12,
            };
            button::Style {
                background: Some(Background::Color(Color::from_rgba(
                    theme::PRIMARY.r,
                    theme::PRIMARY.g,
                    theme::PRIMARY.b,
                    alpha,
                ))),
                text_color: theme::PRIMARY_LIGHT,
                border: Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let copy_label = if copy_feedback { "Copied!" } else { "Copy" };
    let actions = row![
        action_button(
            copy_label,
            Message::TranslateCopyResult,
            ActionRole::Primary
        ),
        action_button(
            "Replace",
            Message::TranslateReplaceResult,
            ActionRole::Secondary
        ),
    ]
    .spacing(8);

    column![
        header,
        separator(),
        lang_row,
        original_editor,
        translated_box,
        retranslate_btn,
        actions,
    ]
    .spacing(8)
    .into()
}

/// Shared language picker row with swap button for translate views.
fn translate_lang_row<'a>(
    from: &'a TranslateLanguage,
    to: &'a TranslateLanguage,
) -> Element<'a, Message> {
    let langs: Vec<TranslateLanguage> = TRANSLATE_LANGS.to_vec();

    let from_picker = pick_list(
        langs.clone(),
        Some(from.clone()),
        Message::TranslateFromChanged,
    )
    .text_size(13)
    .width(Length::Fill);

    let to_picker = pick_list(langs, Some(to.clone()), Message::TranslateToChanged)
        .text_size(13)
        .width(Length::Fill);

    let swap_btn = button(text("<>").size(12).center())
        .on_press(Message::TranslateSwapLangs)
        .width(32)
        .style(|_theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(0.231, 0.510, 0.965, 0.18),
                _ => Color::from_rgba(0.5, 0.5, 0.5, 0.1),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    row![
        column![
            text("From").size(11).style(|_t: &Theme| text::Style {
                color: Some(theme::TEXT_MUTED),
            }),
            from_picker,
        ]
        .spacing(4)
        .width(Length::Fill),
        swap_btn,
        column![
            text("To").size(11).style(|_t: &Theme| text::Style {
                color: Some(theme::TEXT_MUTED),
            }),
            to_picker,
        ]
        .spacing(4)
        .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

// -- Processing view --

fn processing_view(tool: &WritingToolKind, tick: usize) -> Element<'_, Message> {
    let label = format!("{}...", tool);
    let frame = theme::SPINNER_FRAMES[tick % theme::SPINNER_FRAMES.len()];

    row![
        text(frame).size(14).style(|_theme: &Theme| text::Style {
            color: Some(theme::PRIMARY),
        }),
        text(label).size(14).style(|_theme: &Theme| text::Style {
            color: Some(theme::TEXT_SECONDARY),
        }),
        Space::with_width(Length::Fill),
        button(text("Cancel").size(12))
            .on_press(Message::Cancel)
            .style(|_theme: &Theme, _status| button::Style {
                background: None,
                text_color: theme::ERROR,
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
        back_button(),
        Space::with_width(Length::Fill),
        text("Done").size(14).style(|_theme: &Theme| text::Style {
            color: Some(theme::SUCCESS),
        }),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let result_box = container(scrollable(text(output).size(13).width(Length::Fill)))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_ELEVATED)),
            border: Border {
                radius: theme::RADIUS_MD.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .padding(12)
        .width(Length::Fill);

    let copy_label = if copy_feedback { "Copied!" } else { "Copy" };
    let actions = row![
        action_button(copy_label, Message::CopyResult, ActionRole::Primary),
        action_button("Replace", Message::ReplaceResult, ActionRole::Secondary),
    ]
    .spacing(8);

    column![header, result_box, actions].spacing(10).into()
}

// -- Error view --

fn error_view(message: &str) -> Element<'_, Message> {
    let header = row![
        back_button(),
        Space::with_width(Length::Fill),
        text("Error").size(14).style(|_theme: &Theme| text::Style {
            color: Some(Color::from_rgb(0.9, 0.6, 0.1)),
        }),
        Space::with_width(Length::Fill),
        close_button(),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let msg = text(message).size(12).style(|_theme: &Theme| text::Style {
        color: Some(theme::TEXT_SECONDARY),
    });

    let retry_btn = button(text("Retry").size(12))
        .on_press(Message::Retry)
        .style(|_theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => {
                    Color::from_rgba(theme::PRIMARY.r, theme::PRIMARY.g, theme::PRIMARY.b, 0.2)
                }
                _ => Color::from_rgba(theme::PRIMARY.r, theme::PRIMARY.g, theme::PRIMARY.b, 0.12),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme::PRIMARY_LIGHT,
                border: Border {
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
    button(text("X").size(12))
        .on_press(Message::Dismiss)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            text_color: theme::TEXT_MUTED,
            ..Default::default()
        })
        .into()
}

fn back_button() -> Element<'static, Message> {
    button(text("< Back").size(12))
        .on_press(Message::BackToOptions)
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            text_color: theme::TEXT_SECONDARY,
            ..Default::default()
        })
        .into()
}

fn separator<'a>() -> Element<'a, Message> {
    container(Space::new(Length::Fill, 1))
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(theme::SURFACE_BORDER)),
            ..Default::default()
        })
        .width(Length::Fill)
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
                ActionRole::Primary => (theme::PRIMARY, theme::PRIMARY_LIGHT),
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
                background: Some(Background::Color(Color::from_rgba(
                    bg_base.r, bg_base.g, bg_base.b, alpha,
                ))),
                text_color: fg,
                border: Border {
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
        format!("{}...", &one_line[..end])
    } else {
        one_line
    }
}
