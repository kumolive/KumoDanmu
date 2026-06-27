//! Render methods for MainView

use super::content_rendering::display_name;
use super::{DanmuListItemView, MainView, UserInfoCard};
use crate::app::UiCommand;
use crate::components::draggable_area;
use crate::theme::Colors;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::h_flex;
use gpui_component::scroll::Scrollbar;
use gpui_component::v_flex;
use std::rc::Rc;

/// Parse a "#RRGGBB" hex color into an `Hsla`, applying the given opacity.
/// Falls back to a neutral semi-transparent dark color when parsing fails.
fn parse_sc_hex(hex: &str, opacity: f32) -> Hsla {
    let s = hex.trim_start_matches('#');
    let (r, g, b) = if s.len() == 6 {
        (
            u8::from_str_radix(&s[0..2], 16).unwrap_or(40),
            u8::from_str_radix(&s[2..4], 16).unwrap_or(96),
            u8::from_str_radix(&s[4..6], 16).unwrap_or(178),
        )
    } else {
        (40u8, 96u8, 178u8)
    };
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: opacity.clamp(0.0, 1.0),
    }
    .into()
}

impl MainView {
    pub(super) fn render_header(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_live = self.live_status == 1;
        let opacity = self.opacity;

        #[cfg(target_os = "macos")]
        let left_padding = px(78.0);
        #[cfg(not(target_os = "macos"))]
        let left_padding = px(12.0);

        let header_bg = if is_live {
            Colors::live_with_opacity(opacity)
        } else {
            Colors::bg_secondary_with_opacity(opacity)
        };

        let header_text_color = if is_live {
            hsla(0.0, 0.0, 1.0, 1.0)
        } else {
            Colors::text_primary()
        };

        h_flex()
            .w_full()
            .h(px(32.0))
            .items_center()
            .bg(header_bg)
            .child(
                draggable_area()
                    .flex_1()
                    .h_full()
                    .pl(left_padding)
                    .flex()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .when(is_live, |this| {
                                this.child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(header_text_color)
                                        .child("LIVE"),
                                )
                            })
                            .when(is_live, |this| {
                                this.child(
                                    h_flex().gap_1().items_center().child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(header_text_color)
                                            .child(format!("{}", self.online_count)),
                                    ),
                                )
                            }),
                    ),
            )
            .child(
                h_flex()
                    .pr_2()
                    .gap_2()
                    .items_center()
                    .child(self.render_pin_button(is_live, cx))
                    .child(self.render_audience_button(is_live, cx))
                    .child(self.render_settings_button(is_live, cx)),
            )
    }

    fn render_pin_button(&self, is_live: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let is_pinned = self.always_on_top;
        let icon_color = if is_pinned {
            Colors::accent()
        } else if is_live {
            hsla(0.0, 0.0, 1.0, 0.7)
        } else {
            Colors::text_muted()
        };

        div()
            .id("pin-btn")
            .size(px(24.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .when(is_pinned, |this| this.bg(hsla(0.0, 0.0, 1.0, 0.1)))
            .hover(|s| s.bg(hsla(0.0, 0.0, 1.0, 0.2)))
            .on_click(cx.listener(|this, _event, window, cx| {
                this.always_on_top = !this.always_on_top;
                let always_on_top = this.always_on_top;

                crate::platform::set_window_always_on_top(window, always_on_top);

                if let Some(handle) = &this.settings_window {
                    let _ = cx.update_window(*handle, |_, win, _| {
                        crate::platform::set_window_always_on_top(win, always_on_top);
                    });
                }
                if let Some(handle) = &this.audience_window {
                    let _ = cx.update_window(*handle, |_, win, _| {
                        crate::platform::set_window_always_on_top(win, always_on_top);
                    });
                }

                let _ = this
                    .command_tx
                    .send(UiCommand::UpdateAlwaysOnTop(always_on_top));
                cx.notify();
            }))
            .child(
                div()
                    .size(px(14.0))
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(3.0))
                            .size(px(8.0))
                            .rounded_full()
                            .border_1()
                            .border_color(icon_color)
                            .when(is_pinned, |this| this.bg(icon_color)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(8.0))
                            .left(px(6.0))
                            .w(px(2.0))
                            .h(px(6.0))
                            .bg(icon_color),
                    ),
            )
    }

    fn render_audience_button(&self, is_live: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_color = if is_live {
            hsla(0.0, 0.0, 1.0, 0.7)
        } else {
            Colors::text_muted()
        };

        div()
            .id("audience-window-btn")
            .size(px(24.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .hover(|s| s.bg(hsla(0.0, 0.0, 1.0, 0.2)))
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_audience_window(window, cx);
            }))
            .child(
                h_flex()
                    .gap(px(1.0))
                    .items_end()
                    .h(px(12.0))
                    .child(
                        v_flex()
                            .items_center()
                            .child(div().size(px(4.0)).rounded_full().bg(icon_color))
                            .child(
                                div()
                                    .w(px(6.0))
                                    .h(px(4.0))
                                    .rounded_t(px(3.0))
                                    .bg(icon_color),
                            ),
                    )
                    .child(
                        v_flex()
                            .items_center()
                            .child(div().size(px(4.0)).rounded_full().bg(icon_color))
                            .child(
                                div()
                                    .w(px(6.0))
                                    .h(px(4.0))
                                    .rounded_t(px(3.0))
                                    .bg(icon_color),
                            ),
                    ),
            )
    }

    fn render_settings_button(&self, is_live: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_color = if is_live {
            hsla(0.0, 0.0, 1.0, 0.7)
        } else {
            Colors::text_muted()
        };

        div()
            .id("settings-btn")
            .size(px(24.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .hover(|s| s.bg(hsla(0.0, 0.0, 1.0, 0.2)))
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_settings_window(window, cx);
            }))
            .child(
                v_flex()
                    .gap(px(2.0))
                    .child(div().w(px(12.0)).h(px(2.0)).rounded_sm().bg(icon_color))
                    .child(div().w(px(12.0)).h(px(2.0)).rounded_sm().bg(icon_color))
                    .child(div().w(px(12.0)).h(px(2.0)).rounded_sm().bg(icon_color)),
            )
    }

    pub(super) fn render_footer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let opacity = self.opacity;
        let command_tx = self.command_tx.clone();
        let room_id = self.room.as_ref().map(|r| r.real_id());

        struct CommandInputWrapper {
            input: Entity<gpui_component::input::InputState>,
        }

        let state = window.use_keyed_state(
            SharedString::from("command-input-state"),
            cx,
            |window, cx| {
                let input = cx.new(|cx| {
                    gpui_component::input::InputState::new(window, cx).placeholder("发送弹幕…")
                });
                CommandInputWrapper { input }
            },
        );

        let input_state = state.read(cx).input.clone();

        if self.input_state.as_ref() != Some(&input_state) {
            let command_tx_enter = self.command_tx.clone();
            let pending_clear = self.pending_input_clear.clone();
            let subscription = cx.subscribe(
                &input_state,
                move |this, input, event: &gpui_component::input::InputEvent, cx| {
                    if let gpui_component::input::InputEvent::PressEnter { .. } = event {
                        let text = input.read(cx).text().to_string().trim().to_string();
                        if text.is_empty() {
                            return;
                        }
                        if let Some(room_id) = this.room.as_ref().map(|r| r.real_id()) {
                            let _ = command_tx_enter.send(UiCommand::SendDanmu {
                                room_id,
                                message: text,
                            });
                        }
                        pending_clear.set(true);
                        cx.notify();
                    }
                },
            );

            self.input_state = Some(input_state.clone());
            self._input_subscription = Some(subscription);
        }

        if self.pending_input_clear.get() {
            input_state.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            self.pending_input_clear.set(false);
        }

        let input_state_for_click = input_state.clone();

        v_flex()
            .w_full()
            .bg(Colors::bg_secondary_with_opacity(opacity))
            .border_t_1()
            .border_color(Colors::bg_hover_with_opacity(opacity))
            .child(
                h_flex()
                    .w_full()
                    .h(px(32.0))
                    .px_3()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(Colors::text_secondary())
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(if self.room_title.is_empty() {
                                "未连接房间".to_string()
                            } else {
                                self.room_title.clone()
                            }),
                    )
                    .child(h_flex().gap_1().items_center().child(
                        div().size(px(6.0)).rounded_full().bg(if self.connected {
                            Colors::success()
                        } else {
                            Colors::error()
                        }),
                    )),
            )
            .when(!self.lite_mode, |el| {
                el.child(
                    h_flex()
                        .w_full()
                        .h(px(40.0))
                        .px_3()
                        .py_2()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .child(gpui_component::input::Input::new(&input_state).cleanable(true)),
                        )
                        .child(
                            div()
                                .id("send-btn")
                                .px_3()
                                .py(px(6.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .bg(Colors::accent())
                                .hover(|s| s.opacity(0.8))
                                .text_size(px(12.0))
                                .text_color(Colors::button_text())
                                .child("发送")
                                .on_click(move |_event, window, cx| {
                                    let text = input_state_for_click
                                        .read(cx)
                                        .text()
                                        .to_string()
                                        .trim()
                                        .to_string();
                                    if text.is_empty() {
                                        return;
                                    }
                                    if let Some(room_id) = room_id {
                                        let _ = command_tx.send(UiCommand::SendDanmu {
                                            room_id,
                                            message: text,
                                        });
                                    }
                                    input_state_for_click.update(cx, |state, cx| {
                                        state.set_value("", window, cx);
                                    });
                                }),
                        ),
                )
            })
    }

    /// Render the floating SuperChat overlay above the danmu list area.
    /// Each card is clickable: tap to dismiss immediately.
    pub(super) fn render_floating_sc(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if self.floating_sc.is_empty() {
            return None;
        }
        let opacity = self.opacity;
        let font_size = self.font_size;

        let mut stack = v_flex()
            .id("floating-sc-stack")
            .absolute()
            .top(px(36.0))
            .left(px(8.0))
            .right(px(22.0))
            .gap_1()
            .max_h(px(220.0))
            .overflow_hidden();

        // Newest SC on top; show up to 5
        for sc in self.floating_sc.iter().rev().take(5) {
            let bg = parse_sc_hex(&sc.background_bottom_color, opacity);
            let header_bg = parse_sc_hex(&sc.background_color, opacity);
            let sc_id = sc.id.clone();
            stack = stack.child(
                v_flex()
                    .id(SharedString::from(format!("sc-card-{}", sc.id)))
                    .rounded(px(6.0))
                    .overflow_hidden()
                    .border_1()
                    .border_color(Colors::border())
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        let id = sc_id.clone();
                        this.floating_sc.retain(|x| x.id != id);
                        cx.notify();
                    }))
                    .child(
                        h_flex()
                            .px_2()
                            .py(px(4.0))
                            .gap_2()
                            .items_center()
                            .bg(header_bg)
                            .text_color(Colors::text_primary())
                            .child(
                                div()
                                    .text_size(px(font_size))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("¥{}", sc.price)),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size))
                                    .overflow_hidden()
                                    .child(display_name(
                                        &sc.sender.uname,
                                        sc.sender.uid,
                                        &self.nicknames,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .px_2()
                            .py(px(6.0))
                            .text_size(px(font_size))
                            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                            .bg(bg)
                            .child(sc.message.clone()),
                    ),
            );
        }

        Some(stack)
    }

    pub(super) fn render_danmu_list(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let medal_display = self.medal_display;
        let opacity = self.opacity;
        let scroll_handle = self.scroll_handle.clone();
        let selected_user = self.selected_user.clone();
        let nicknames = Rc::clone(&self.nicknames);

        let render_rows = Rc::clone(&self.render_rows);
        let item_count = render_rows.len();

        h_flex()
            .id("danmu-container")
            .flex_1()
            .w_full()
            .overflow_hidden()
            .child(
                uniform_list("danmu-list", item_count, {
                    move |range, _window, _cx| {
                        range
                            .map(|ix| {
                                let row = render_rows[ix].clone();
                                let selected_user = selected_user.clone();
                                DanmuListItemView::new(
                                    row,
                                    ix,
                                    font_size,
                                    lite_mode,
                                    medal_display,
                                    opacity,
                                    selected_user,
                                    Rc::clone(&nicknames),
                                ).render_element()
                            })
                            .collect()
                    }
                })
                .flex_1()
                .h_full()
                .track_scroll(scroll_handle.clone()),
            )
            .child(Scrollbar::vertical(&scroll_handle))
    }
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.process_events(cx);

        if let Some(always_on_top) = self.pending_always_on_top.take() {
            crate::platform::set_window_always_on_top(window, always_on_top);
        }

        // Check if tray toggled click-through mode
        if self.pending_click_through.swap(false, std::sync::atomic::Ordering::Relaxed) {
            // Read the actual state from tray manager
            let click_through = self
                .tray_manager
                .as_ref()
                .map(|t| t.lock().click_through_enabled())
                .unwrap_or(false);
            self.click_through = click_through;
        }

        // Check if tray requested to open settings
        // Use cx.spawn to defer window opening to after render completes
        // Opening a window during render causes segfault due to GPUI global state modification
        if self.pending_open_settings.swap(false, std::sync::atomic::Ordering::Relaxed) {
            cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                // Small delay to ensure render is complete
                Timer::after(std::time::Duration::from_millis(1)).await;
                let _ = cx.update(|cx| {
                    let _ = this.update(cx, |view, cx| {
                        view.open_settings_window_deferred(cx);
                    });
                });
            })
            .detach();
        }

        let bounds = window.bounds();
        let current_bounds = (
            f32::from(bounds.origin.x) as i32,
            f32::from(bounds.origin.y) as i32,
            f32::from(bounds.size.width) as u32,
            f32::from(bounds.size.height) as u32,
        );
        if self.last_saved_bounds != Some(current_bounds) {
            self.last_saved_bounds = Some(current_bounds);
            let _ = self.command_tx.send(UiCommand::SaveWindowBounds {
                window_type: jlivertool_core::types::WindowType::Main,
                x: current_bounds.0,
                y: current_bounds.1,
                width: current_bounds.2,
                height: current_bounds.3,
            });
        }

        // Update render rows (incremental if width unchanged, full rebuild if changed)
        let window_width = f32::from(bounds.size.width);
        self.update_render_rows(window_width);
        self.apply_pending_scroll();

        {
            let mut selected = self.selected_user.borrow_mut();
            if let Some(ref mut user) = *selected {
                if user.fetched_info.is_none() && !user.fetch_requested {
                    user.fetch_requested = true;
                    let uid = user.sender.uid;
                    let _ = self.command_tx.send(UiCommand::FetchUserInfo { uid });
                }
            }
        }

        let opacity = self.opacity;
        let selected_user = self.selected_user.borrow().clone();
        let selected_user_state = self.selected_user.clone();

        // Ensure the per-uid nickname input state is in sync with the currently
        // selected user before we render the popup. Creating an InputState
        // requires `window`/`cx`, neither of which is available inside the
        // `.when_some` closure further down, so do it here.
        if let Some(sel) = selected_user.as_ref() {
            let uid = sel.sender.uid;
            let needs_new = self
                .nickname_input
                .as_ref()
                .map(|(u, _)| *u != uid)
                .unwrap_or(true);
            if needs_new {
                let current = self.nicknames.get(&uid).cloned().unwrap_or_default();
                let entity = cx.new(|cx| {
                    gpui_component::input::InputState::new(window, cx)
                        .placeholder("输入昵称…")
                        .default_value(current)
                });
                self.nickname_input = Some((uid, entity));
            }
        } else if self.nickname_input.is_some() {
            self.nickname_input = None;
        }

        let danmu_history: Vec<(String, i64)> = if let Some(ref selected) = selected_user {
            if let (Some(db), Some(room)) = (&self.database, &self.room) {
                let uid = selected.sender.uid;
                let room_id = room.real_id();
                db.get_danmus_by_user(room_id, uid, 50).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let show_update_dialog = self.show_update_dialog;
        let update_info = self.update_info.clone();
        let show_face_auth_dialog = self.show_face_auth_dialog;
        let face_auth_qr_view = self.face_auth_qr_view.clone();

        let floating_sc = self.render_floating_sc(cx);

        v_flex()
            .size_full()
            .relative()
            .bg(Colors::bg_primary_with_opacity(opacity))
            .text_color(Colors::text_primary())
            .child(self.render_header(window, cx))
            .child(self.render_danmu_list(window, cx))
            .child(self.render_footer(window, cx))
            .when_some(floating_sc, |this, sc| this.child(sc))
            .when_some(selected_user, |this, selected| {
                let state_for_close = selected_user_state.clone();
                let history = danmu_history.clone();
                let uid = selected.sender.uid;
                let nickname_input = self
                    .nickname_input
                    .as_ref()
                    .map(|(_, e)| e.clone())
                    .expect("nickname_input was synced above when selected_user is Some");
                let current_nickname = self.nicknames.get(&uid).cloned();
                let cmd_tx = self.command_tx.clone();
                let on_save: super::user_info_card::NicknameSaveCallback =
                    std::sync::Arc::new(move |payload, _window, cx| {
                        let _ = cmd_tx.send(UiCommand::SetNickname {
                            uid,
                            nickname: payload,
                        });
                        cx.refresh_windows();
                    });
                this.child(
                    div()
                        .id("user-info-overlay")
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .p_4()
                        .bg(hsla(0.0, 0.0, 0.0, 0.5 * opacity))
                        .child(
                            div()
                                .relative()
                                .w_full()
                                .max_w(px(300.0))
                                .child(UserInfoCard::render_element(
                                    &selected,
                                    history,
                                    current_nickname.as_deref(),
                                    nickname_input,
                                    on_save,
                                ))
                                .child(
                                    div()
                                        .id("close-card-btn")
                                        .absolute()
                                        .top(px(-8.0))
                                        .right(px(-8.0))
                                        .size(px(24.0))
                                        .rounded_full()
                                        .cursor_pointer()
                                        .bg(Colors::bg_secondary_with_opacity(opacity))
                                        .border_2()
                                        .border_color(Colors::border())
                                        .hover(|s| s.bg(Colors::error()))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(14.0))
                                        .text_color(Colors::text_secondary())
                                        .child("×")
                                        .on_click(move |_, _, cx| {
                                            *state_for_close.borrow_mut() = None;
                                            cx.refresh_windows();
                                        }),
                                ),
                        ),
                )
            })
            // Update available dialog
            .when(show_update_dialog, |this| {
                let info = update_info.clone();
                this.child(
                    div()
                        .id("update-dialog-overlay")
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(hsla(0.0, 0.0, 0.0, 0.5 * opacity))
                        .child(
                            v_flex()
                                .w(px(320.0))
                                .p_4()
                                .rounded(px(8.0))
                                .bg(Colors::bg_secondary())
                                .border_1()
                                .border_color(Colors::border())
                                .gap_3()
                                .child(
                                    div()
                                        .text_size(px(16.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("发现新版本"),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(Colors::text_secondary())
                                        .child(format!(
                                            "新版本 v{} 已发布，建议更新以获得最新功能和修复。",
                                            info.as_ref().map(|i| i.latest_version.as_str()).unwrap_or("")
                                        )),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .justify_end()
                                        .child(
                                            div()
                                                .id("update-later-btn")
                                                .px_3()
                                                .py(px(6.0))
                                                .rounded(px(4.0))
                                                .cursor_pointer()
                                                .bg(Colors::bg_hover_with_opacity(opacity))
                                                .text_size(px(12.0))
                                                .hover(|s| s.opacity(0.8))
                                                .on_click(cx.listener(|this, _event, _window, cx| {
                                                    this.show_update_dialog = false;
                                                    cx.notify();
                                                }))
                                                .child("稍后"),
                                        )
                                        .child({
                                            let url = info.map(|i| i.release_url).unwrap_or_default();
                                            div()
                                                .id("update-now-btn")
                                                .px_3()
                                                .py(px(6.0))
                                                .rounded(px(4.0))
                                                .cursor_pointer()
                                                .bg(Colors::accent())
                                                .text_size(px(12.0))
                                                .text_color(gpui::white())
                                                .hover(|s| s.opacity(0.8))
                                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                                    // Open release URL in browser
                                                    let _ = open::that(&url);
                                                    this.show_update_dialog = false;
                                                    cx.notify();
                                                }))
                                                .child("前往下载")
                                        }),
                                ),
                        ),
                )
            })
            // Face auth dialog (Bilibili face authentication QR code overlay)
            .when(show_face_auth_dialog, |this| {
                this.child(
                    div()
                        .id("face-auth-dialog-overlay")
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(hsla(0.0, 0.0, 0.0, 0.5 * opacity))
                        .child(
                            v_flex()
                                .w(px(320.0))
                                .p_4()
                                .rounded(px(8.0))
                                .bg(Colors::bg_secondary())
                                .border_1()
                                .border_color(Colors::border())
                                .gap_3()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_size(px(16.0))
                                                .font_weight(FontWeight::BOLD)
                                                .child("人脸认证"),
                                        )
                                        .child(
                                            div()
                                                .id("close-face-auth-btn")
                                                .px_2()
                                                .py(px(2.0))
                                                .rounded(px(4.0))
                                                .cursor_pointer()
                                                .text_size(px(14.0))
                                                .text_color(Colors::text_secondary())
                                                .hover(|s| s.bg(Colors::bg_hover_with_opacity(opacity)))
                                                .child("×")
                                                .on_click(cx.listener(|this, _event, _window, cx| {
                                                    this.show_face_auth_dialog = false;
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(Colors::text_secondary())
                                        .child("请使用哔哩哔哩 App 扫描二维码完成人脸认证"),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .justify_center()
                                        .child(face_auth_qr_view),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(Colors::text_muted())
                                        .text_align(gpui::TextAlign::Center)
                                        .child("认证完成后请重新点击开播"),
                                ),
                        ),
                )
            })
    }
}
