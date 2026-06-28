//! Danmu list item view
//!
//! This module contains the unified view for rendering different types of
//! messages in the danmu list (danmu, interact, entry effect, gift, guard, superchat).

use super::content_rendering::{
    display_name, guard_icon_url, guard_level_name, render_content_with_links, DisplayMessage,
    RenderRow,
};
use super::user_info_card::{SelectedUser, SelectedUserState};
use crate::theme::Colors;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::h_flex;
use jlivertool_core::messages::{
    DanmuMessage, EntryEffectMessage, GiftMessage, GuardMessage, InteractMessage,
};
use jlivertool_core::types::Sender;
use std::collections::HashMap;
use std::rc::Rc;

/// Unified view for rendering any display message type
pub struct DanmuListItemView {
    row: RenderRow,
    index: usize,
    font_size: f32,
    lite_mode: bool,
    opacity: f32,
    selected_user: SelectedUserState,
    nicknames: Rc<HashMap<u64, String>>,
}

impl DanmuListItemView {
    pub fn new(
        row: RenderRow,
        index: usize,
        font_size: f32,
        lite_mode: bool,
        _medal_display: bool,
        opacity: f32,
        selected_user: SelectedUserState,
        nicknames: Rc<HashMap<u64, String>>,
    ) -> Self {
        Self {
            row,
            index,
            font_size,
            lite_mode,
            opacity,
            selected_user,
            nicknames,
        }
    }

    /// Resolve the display label for a sender, taking a nickname (if any) into account.
    fn label(&self, sender: &Sender) -> String {
        display_name(&sender.uname, sender.uid, &self.nicknames)
    }

    fn row_height(&self) -> f32 {
        if self.lite_mode {
            self.font_size + 6.0
        } else {
            self.font_size + 10.0
        }
    }

    fn fold_badge(fold_count: u32, font_size: f32, color: Hsla) -> Div {
        div()
            .text_size(px(font_size * 0.85))
            .font_weight(FontWeight::BOLD)
            .text_color(color)
            .child(format!("×{}", fold_count))
    }

    fn render_danmu(&self, danmu: &DanmuMessage, fold_count: u32) -> Div {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let opacity = self.opacity;
        let row_height = self.row_height();
        let sender = &danmu.sender;

        // Use fixed height for uniform_list
        let mut el = h_flex()
            .w_full()
            .h(px(row_height))
            .gap_1()
            .items_center()
            .rounded_sm()
            .hover(|s| s.bg(Colors::bg_hover_with_opacity(opacity)))
            .overflow_hidden();

        if lite_mode {
            el = el.px_1();
        } else {
            el = el.px_2();
        }

        // Username
        let username_color = if danmu.is_special {
            Colors::warning()
        } else {
            Colors::accent()
        };

        let item_index = self.index;
        let selected_user = self.selected_user.clone();
        let sender_clone = sender.clone();

        let label = self.label(sender);
        if lite_mode {
            let selected_user_lite = selected_user.clone();
            let sender_lite = sender_clone.clone();
            let label_lite = label.clone();
            el = el
                .child(
                    div()
                        .id(SharedString::from(format!("user-{}", item_index)))
                        .text_size(px(font_size))
                        .text_color(username_color)
                        .cursor_pointer()
                        .child(label_lite)
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            *selected_user_lite.borrow_mut() = Some(SelectedUser {
                                sender: sender_lite.clone(),
                                fetched_info: None,
                                fetch_requested: false,
                            });
                            cx.refresh_windows();
                        }),
                )
                .child(
                    div()
                        .text_size(px(font_size))
                        .text_color(Colors::text_muted())
                        .child(":"),
                );
        } else {
            el = el.child(
                div()
                    .id(SharedString::from(format!("user-{}", item_index)))
                    .text_size(px(font_size))
                    .text_color(username_color)
                    .cursor_pointer()
                    .child(format!("{}:", label))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        *selected_user.borrow_mut() = Some(SelectedUser {
                            sender: sender_clone.clone(),
                            fetched_info: None,
                            fetch_requested: false,
                        });
                        cx.refresh_windows();
                    }),
            );
        }

        // Reply indicator
        if let Some(reply) = &danmu.reply_uname {
            if !lite_mode {
                el = el.child(
                    div()
                        .text_size(px(font_size))
                        .text_color(Colors::text_muted())
                        .child(format!("@{}", reply)),
                );
            }
        }

        // Content or Emoji
        if let Some(emoji) = &danmu.emoji_content {
            // For emoji danmu, display emoji image constrained to row height
            // Only render if URL is not empty to avoid GPUI crash on empty images
            if !emoji.url.is_empty() {
                let emoji_size = row_height - 4.0;
                // Use a hash of the URL in the element ID to ensure fresh state when
                // the image changes. This prevents GPUI's animated image frame_index
                // from becoming stale when a different image is displayed at the same index.
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                emoji.url.hash(&mut hasher);
                let url_hash = hasher.finish();
                if danmu.is_mirror {
                    el = el.child(
                        div()
                            .text_size(px(font_size * 0.8))
                            .text_color(Colors::text_muted())
                            .child("[跨房]"),
                    );
                }
                el = el.child(
                    img(emoji.url.clone())
                        .id(SharedString::from(format!("emoji-{}-{:x}", item_index, url_hash)))
                        .max_w(px(emoji_size))
                        .max_h(px(emoji_size))
                        .object_fit(ObjectFit::Contain),
                );
            } else {
                // Fallback to text content if emoji URL is empty
                // Add mirror indicator if it's a mirror danmu
                if danmu.is_mirror {
                    el = el.child(
                        div()
                            .text_size(px(font_size * 0.8))
                            .text_color(Colors::text_muted())
                            .child("[跨房]"),
                    );
                }
                el = el.child(
                    render_content_with_links(&danmu.content, font_size, Colors::text_primary(), item_index)
                        .flex_1(),
                );
            }
        } else {
            // Regular text content with BV link support and tooltip for long content
            // Add mirror indicator if it's a mirror danmu
            if danmu.is_mirror {
                el = el.child(
                    div()
                        .text_size(px(font_size * 0.8))
                        .text_color(Colors::text_muted())
                        .child("[跨房]"),
                );
            }
            el = el.child(
                render_content_with_links(&danmu.content, font_size, Colors::text_primary(), item_index)
                    .flex_1(),
            );
        }

        el = el.when(fold_count > 1, |el| {
            el.child(Self::fold_badge(fold_count, font_size, Colors::text_muted()))
        });

        el
    }

    fn render_interact(&self, interact: &InteractMessage, fold_count: u32) -> Div {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let opacity = self.opacity;
        let row_height = self.row_height();

        let action_text = if interact.action == 2 {
            "关注了直播间"
        } else {
            "进入了直播间"
        };

        let mut el = h_flex()
            .w_full()
            .h(px(row_height))
            .gap_1()
            .items_center()
            .rounded_sm()
            .hover(|s| s.bg(Colors::bg_hover_with_opacity(opacity)))
            .overflow_hidden();

        if lite_mode {
            el = el.px_1();
        } else {
            el = el.px_2();
        }

        el = el
            .child(
                div()
                    .text_size(px(font_size))
                    .text_color(Colors::text_muted())
                    .child(self.label(&interact.sender)),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .text_color(Colors::text_muted())
                    .child(action_text),
            );

        el = el.when(fold_count > 1, |el| {
            el.child(Self::fold_badge(fold_count, font_size, Colors::text_muted()))
        });

        el
    }

    fn render_entry_effect(&self, entry: &EntryEffectMessage, fold_count: u32) -> Div {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let opacity = self.opacity;
        let row_height = self.row_height();

        let privilege_type = entry.privilege_type;
        let level_name = guard_level_name(privilege_type);
        let icon_size = (font_size * 1.2).clamp(14.0, 20.0);

        // Golden color #EDB83F
        let entry_color = hsla(42.0 / 360.0, 0.85, 0.59, 1.0);

        let mut el = h_flex()
            .w_full()
            .h(px(row_height))
            .gap_1()
            .items_center()
            .rounded_sm()
            .border_l_2()
            .border_color(entry_color)
            .hover(|s| s.bg(Colors::bg_hover_with_opacity(opacity)))
            .overflow_hidden();

        if lite_mode {
            el = el.px_1();
        } else {
            el = el.px_2();
        }

        if let Some(icon_url) = guard_icon_url(privilege_type) {
            el = el.child(
                img(icon_url)
                    .size(px(icon_size))
                    .object_fit(ObjectFit::Contain),
            );
        }

        el = el.child(
            div()
                .text_size(px(font_size))
                .text_color(entry_color)
                .child(format!("{} {} 进入直播间", level_name, self.label(&entry.sender))),
        );

        el = el.when(fold_count > 1, |el| {
            el.child(Self::fold_badge(fold_count, font_size, entry_color))
        });

        el
    }

    fn render_gift(&self, gift: &GiftMessage, fold_count: u32) -> Div {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let opacity = self.opacity;
        let row_height = self.row_height();

        let is_paid = gift.gift_info.coin_type != "silver";
        let price_text = if is_paid {
            format!("¥{:.2}", gift.gift_info.price as f64 / 1000.0)
        } else {
            "免费".to_string()
        };

        // Golden color #EDB83F for paid gifts, secondary for free
        let gift_color = if is_paid {
            hsla(42.0 / 360.0, 0.85, 0.59, 1.0)
        } else {
            Colors::text_secondary()
        };

        let mut el = h_flex()
            .w_full()
            .h(px(row_height))
            .gap_1()
            .items_center()
            .rounded_sm()
            .border_l_2()
            .border_t_1()
            .border_b_1()
            .border_color(gift_color)
            .hover(|s| s.bg(Colors::bg_hover_with_opacity(opacity)))
            .overflow_hidden();

        if lite_mode {
            el = el.px_1();
        } else {
            el = el.px_2();
        }

        el = el
            .child(
                div()
                    .text_size(px(font_size))
                    .text_color(Colors::text_secondary())
                    .child(self.label(&gift.sender)),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .text_color(Colors::text_secondary())
                    .child(gift.action.clone()),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(gift_color)
                    .child(gift.gift_info.name.clone()),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .text_color(Colors::text_secondary())
                    .child(format!("x{}", gift.num)),
            );

        if !lite_mode && is_paid {
            el = el.child(
                div()
                    .text_size(px(font_size * 0.9))
                    .text_color(gift_color)
                    .child(price_text),
            );
        }

        el = el.when(fold_count > 1, |el| {
            el.child(Self::fold_badge(fold_count, font_size, gift_color))
        });

        el
    }

    fn render_guard(&self, guard: &GuardMessage, fold_count: u32) -> Div {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let row_height = self.row_height();

        let guard_name = guard_level_name(guard.guard_level);
        let price_text = format!("¥{:.2}", guard.price as f64 / 1000.0);
        let icon_size = (font_size * 1.2).clamp(14.0, 20.0);

        // Golden color #EDB83F for guard messages
        let guard_color = hsla(42.0 / 360.0, 0.85, 0.59, 1.0);

        let mut el = h_flex()
            .w_full()
            .h(px(row_height))
            .gap_1()
            .items_center()
            .rounded_sm()
            .border_l_2()
            .border_color(guard_color)
            .bg(guard_color.opacity(0.1))
            .hover(|s| s.bg(guard_color.opacity(0.15)))
            .overflow_hidden();

        if lite_mode {
            el = el.px_1();
        } else {
            el = el.px_2();
        }

        if let Some(icon_url) = guard_icon_url(guard.guard_level) {
            el = el.child(
                img(icon_url)
                    .size(px(icon_size))
                    .object_fit(ObjectFit::Contain),
            );
        }

        el = el
            .child(
                div()
                    .text_size(px(font_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(guard_color)
                    .child(self.label(&guard.sender)),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .text_color(Colors::text_secondary())
                    .child("开通了"),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(guard_color)
                    .child(guard_name),
            )
            .child(
                div()
                    .text_size(px(font_size))
                    .text_color(Colors::text_secondary())
                    .child(format!("{}{}", guard.num, guard.unit)),
            );

        if !lite_mode {
            el = el.child(
                div()
                    .text_size(px(font_size * 0.9))
                    .font_weight(FontWeight::BOLD)
                    .text_color(guard_color)
                    .child(price_text),
            );
        }

        el = el.when(fold_count > 1, |el| {
            el.child(Self::fold_badge(fold_count, font_size, guard_color))
        });

        el
    }
}

impl Render for DanmuListItemView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.render_element()
    }
}

impl DanmuListItemView {
    /// Render the element directly without going through the Render trait.
    /// This is used by uniform_list to avoid creating entities which would
    /// reset tooltip state on each render.
    pub fn render_element(&self) -> Div {
        match &self.row {
            RenderRow::Full(msg, fold_count, _) => match msg {
                DisplayMessage::Danmu(danmu, _) => self.render_danmu(danmu, *fold_count),
                DisplayMessage::Interact(interact, _) => self.render_interact(interact, *fold_count),
                DisplayMessage::EntryEffect(entry, _) => self.render_entry_effect(entry, *fold_count),
                DisplayMessage::Gift(gift, _) => self.render_gift(gift, *fold_count),
                DisplayMessage::Guard(guard, _) => self.render_guard(guard, *fold_count),
            },
            RenderRow::DanmuFirstLine { danmu, content_slice } => {
                self.render_danmu_first_line(danmu, content_slice)
            }
            RenderRow::DanmuContinuation { danmu, content_slice, continuation_index } => {
                self.render_danmu_continuation(danmu, content_slice, *continuation_index)
            }
        }
    }

    /// Render the first line of a wrapped danmu (username + partial content)
    fn render_danmu_first_line(&self, danmu: &DanmuMessage, content_slice: &str) -> Div {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let opacity = self.opacity;
        let row_height = self.row_height();
        let sender = &danmu.sender;

        let mut el = h_flex()
            .w_full()
            .h(px(row_height))
            .gap_1()
            .items_center()
            .rounded_sm()
            .hover(|s| s.bg(Colors::bg_hover_with_opacity(opacity)))
            .overflow_hidden();

        if lite_mode {
            el = el.px_1();
        } else {
            el = el.px_2();
        }

        // Username
        let username_color = if danmu.is_special {
            Colors::warning()
        } else {
            Colors::accent()
        };

        let item_index = self.index;
        let selected_user = self.selected_user.clone();
        let sender_clone = sender.clone();

        let label = self.label(sender);
        if lite_mode {
            let selected_user_lite = selected_user.clone();
            let sender_lite = sender_clone.clone();
            let label_lite = label.clone();
            el = el
                .child(
                    div()
                        .id(SharedString::from(format!("user-{}", item_index)))
                        .text_size(px(font_size))
                        .text_color(username_color)
                        .cursor_pointer()
                        .child(label_lite)
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            *selected_user_lite.borrow_mut() = Some(SelectedUser {
                                sender: sender_lite.clone(),
                                fetched_info: None,
                                fetch_requested: false,
                            });
                            cx.refresh_windows();
                        }),
                )
                .child(
                    div()
                        .text_size(px(font_size))
                        .text_color(Colors::text_muted())
                        .child(":"),
                );
        } else {
            el = el.child(
                div()
                    .id(SharedString::from(format!("user-{}", item_index)))
                    .text_size(px(font_size))
                    .text_color(username_color)
                    .cursor_pointer()
                    .child(format!("{}:", label))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        *selected_user.borrow_mut() = Some(SelectedUser {
                            sender: sender_clone.clone(),
                            fetched_info: None,
                            fetch_requested: false,
                        });
                        cx.refresh_windows();
                    }),
            );
        }

        // Reply indicator
        if let Some(reply) = &danmu.reply_uname {
            if !lite_mode {
                el = el.child(
                    div()
                        .text_size(px(font_size))
                        .text_color(Colors::text_muted())
                        .child(format!("@{}", reply)),
                );
            }
        }

        // First line content with BV link support (no ellipsis, no tooltip — content is pre-split)
        el = el.child(
            render_content_with_links(content_slice, font_size, Colors::text_primary(), item_index)
                .overflow_hidden(),
        );

        el
    }

    /// Render a continuation line of a wrapped danmu (indented remaining content)
    fn render_danmu_continuation(
        &self,
        _danmu: &DanmuMessage,
        content_slice: &str,
        _continuation_index: usize,
    ) -> Div {
        let font_size = self.font_size;
        let lite_mode = self.lite_mode;
        let opacity = self.opacity;
        let row_height = self.row_height();
        let item_index = self.index;

        let mut el = h_flex()
            .w_full()
            .h(px(row_height))
            .items_center()
            .rounded_sm()
            .hover(|s| s.bg(Colors::bg_hover_with_opacity(opacity)))
            .overflow_hidden();

        if lite_mode {
            el = el.px_1();
        } else {
            el = el.px_2();
        }

        // Continuation content with BV link support
        el = el.child(
            render_content_with_links(content_slice, font_size, Colors::text_primary(), item_index)
                .id(SharedString::from(format!("cont-{}", item_index)))
                .flex_1()
                .overflow_hidden(),
        );

        el
    }
}
