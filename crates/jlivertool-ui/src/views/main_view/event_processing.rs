//! Event processing for MainView

use super::{DisplayMessage, MainView, MAX_DANMU_COUNT};
use gpui::Context;
use jlivertool_core::events::Event;

impl MainView {
    pub(super) fn process_events(&mut self, cx: &mut Context<Self>) {
        let mut list_modified = false;
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::UpdateRoom {
                    room_id,
                    title,
                    live_status,
                    area_id,
                } => {
                    let real_id = room_id.real_id();
                    let owner_uid = room_id.owner_uid();
                    let title_clone = title.clone();
                    self.setting_view.update(cx, |view, cx| {
                        view.set_room_id(Some(real_id), Some(owner_uid), cx);
                        view.set_room_title(title_clone, cx);
                        view.set_area_id(area_id, cx);
                    });

                    let is_new_room = self.room.as_ref().map(|r| r.real_id()) != Some(real_id);

                    if let Some(db) = &self.database {
                        if is_new_room {
                            if let Ok(recent_danmus) = db.get_danmus_since(real_id, 30) {
                                let ts = chrono::Utc::now().timestamp();
                                for danmu in recent_danmus {
                                    self.danmu_list.push_back(DisplayMessage::Danmu(danmu, ts));
                                }
                                while self.danmu_list.len() > MAX_DANMU_COUNT {
                                    self.danmu_list.pop_front();
                                }
                                list_modified = true;
                                self.scroll_to_bottom();
                            }
                        }
                    }

                    self.room = Some(room_id);
                    self.room_title = title;
                    self.area_id = area_id;

                    if is_new_room {
                        self.live_status = live_status;
                    }

                    // Update tray state
                    self.update_tray_state();
                }
                Event::UpdateOnline { count } => {
                    self.online_count = count;
                }
                Event::NewDanmu(danmu) => {
                    if !danmu.is_generated {
                        let should_auto_scroll = self.is_at_bottom();
                        let ts = chrono::Utc::now().timestamp();
                        self.danmu_list.push_back(DisplayMessage::Danmu(danmu, ts));
                        if should_auto_scroll {
                            while self.danmu_list.len() > MAX_DANMU_COUNT {
                                self.danmu_list.pop_front();
                            }
                            self.scroll_to_bottom();
                        }
                        list_modified = true;
                    }
                }
                Event::NewInteract(interact) => {
                    if self.interact_display {
                        let should_auto_scroll = self.is_at_bottom();
                        let ts = chrono::Utc::now().timestamp();
                        self.danmu_list
                            .push_back(DisplayMessage::Interact(interact, ts));
                        if should_auto_scroll {
                            while self.danmu_list.len() > MAX_DANMU_COUNT {
                                self.danmu_list.pop_front();
                            }
                            self.scroll_to_bottom();
                        }
                        list_modified = true;
                    }
                }
                Event::NewEntryEffect(entry) => {
                    let is_guard_entry = entry.privilege_type >= 1 && entry.privilege_type <= 3;
                    let should_display = if is_guard_entry {
                        self.guard_effect
                    } else {
                        self.level_effect
                    };

                    if should_display {
                        let should_auto_scroll = self.is_at_bottom();
                        let ts = chrono::Utc::now().timestamp();
                        self.danmu_list
                            .push_back(DisplayMessage::EntryEffect(entry, ts));
                        if should_auto_scroll {
                            while self.danmu_list.len() > MAX_DANMU_COUNT {
                                self.danmu_list.pop_front();
                            }
                            self.scroll_to_bottom();
                        }
                        list_modified = true;
                    }
                }
                Event::NewGift(gift) => {
                    let should_auto_scroll = self.is_at_bottom();
                    let ts = chrono::Utc::now().timestamp();
                    self.danmu_list.push_back(DisplayMessage::Gift(gift, ts));
                    if should_auto_scroll {
                        while self.danmu_list.len() > MAX_DANMU_COUNT {
                            self.danmu_list.pop_front();
                        }
                        self.scroll_to_bottom();
                    }
                    list_modified = true;
                }
                Event::NewGuard(guard) => {
                    let should_auto_scroll = self.is_at_bottom();
                    let ts = chrono::Utc::now().timestamp();
                    self.danmu_list.push_back(DisplayMessage::Guard(guard, ts));
                    if should_auto_scroll {
                        while self.danmu_list.len() > MAX_DANMU_COUNT {
                            self.danmu_list.pop_front();
                        }
                        self.scroll_to_bottom();
                    }
                    list_modified = true;
                }
                Event::NewSuperChat(sc) => {
                    self.floating_sc.push(sc);
                }
                Event::ConnectionStatus { connected } => {
                    self.connected = connected;
                    self.update_tray_state();
                }
                Event::LiveStart => {
                    self.live_status = 1;
                    self.update_tray_state();
                }
                Event::LiveEnd => {
                    self.live_status = 0;
                    self.update_tray_state();
                }
                Event::LoginStatusChanged {
                    logged_in,
                    user_info,
                } => {
                    self.logged_in = logged_in;
                    // Store the logged-in user's UID
                    self.logged_in_uid = if logged_in {
                        user_info.as_ref().map(|u| u.mid)
                    } else {
                        None
                    };
                    self.setting_view.update(cx, |view, cx| {
                        view.set_login_status(logged_in, user_info, cx);
                    });
                    self.update_tray_state();
                }
                Event::QrCodeGenerated { url, qrcode_key: _ } => {
                    self.setting_view.update(cx, |view, cx| {
                        view.set_qr_code(url, cx);
                    });
                }
                Event::QrLoginStatus { status } => {
                    self.setting_view.update(cx, |view, cx| {
                        view.set_qr_status(status, cx);
                    });
                }
                Event::ConfigLoaded {
                    always_on_top,
                    guard_effect,
                    level_effect,
                    opacity,
                    lite_mode,
                    medal_display,
                    interact_display,
                    theme,
                    font_size,
                    tts_enabled,
                    tts_gift_enabled,
                    tts_sc_enabled,
                    tts_volume,
                    max_danmu_count,
                    log_level,
                    auto_update_check,
                    fold_timeout,
                    fold_lookback,
                } => {
                    crate::theme::set_theme(&theme);

                    self.setting_view.update(cx, |view, cx| {
                        view.load_config(
                            crate::views::ConfigValues {
                                always_on_top,
                                guard_effect,
                                level_effect,
                                opacity,
                                lite_mode,
                                medal_display,
                                interact_display,
                                theme,
                                font_size,
                                tts_enabled,
                                tts_gift_enabled,
                                tts_sc_enabled,
                                tts_volume,
                            },
                            cx,
                        );
                        // Set advanced settings
                        view.set_advanced_settings(max_danmu_count, log_level, cx);
                        // Set auto update check setting
                        view.set_auto_update_check(auto_update_check, cx);
                    });
                    self.opacity = opacity;
                    self.font_size = font_size;
                    self.lite_mode = lite_mode;
                    self.medal_display = medal_display;
                    self.interact_display = interact_display;
                    self.guard_effect = guard_effect;
                    self.level_effect = level_effect;
                    self.always_on_top = always_on_top;
                    self.fold_timeout = fold_timeout;
                    self.fold_lookback = fold_lookback;
                    // Force rebuild of render rows
                    self.last_render_width = 0.0;
                    self.render_rows_source_count = 0;
                    self.audience_view
                        .update(cx, |v, cx| v.set_opacity(opacity, cx));
                    if always_on_top {
                        self.pending_always_on_top = Some(always_on_top);
                    }
                }
                Event::FaceAuthRequired { qr_url } => {
                    self.face_auth_qr_view.update(cx, |view, cx| {
                        view.set_data(qr_url, cx);
                    });
                    self.show_face_auth_dialog = true;
                }
                Event::ClearDanmuList => {
                    self.danmu_list.clear();
                    self.floating_sc.clear();
                    list_modified = true;
                }
                Event::UserInfoFetched { uid, user_info } => {
                    let mut selected = self.selected_user.borrow_mut();
                    if let Some(ref mut user) = *selected {
                        if user.sender.uid == uid {
                            user.fetched_info = Some(user_info);
                        }
                    }
                }
                Event::AudienceListFetched { list } => {
                    self.audience_view.update(cx, |view, cx| {
                        view.set_audience_list(list, cx);
                    });
                }
                Event::GuardListFetched { list, total, page } => {
                    self.audience_view.update(cx, |view, cx| {
                        view.set_guard_list(list, total, page, cx);
                    });
                }
                Event::DataCleared => {
                    self.danmu_list.clear();
                    self.floating_sc.clear();
                    list_modified = true;
                    tracing::info!("Data cleared, UI lists reset");
                }
                Event::RoomChange(room_change) => {
                    if !room_change.title.is_empty() {
                        self.room_title = room_change.title;
                    }
                }
                Event::UpdateCheckResult {
                    has_update,
                    latest_version,
                    release_url,
                    error,
                    ..
                } => {
                    use crate::views::setting_view::UpdateStatus;
                    let status = if let Some(err) = error {
                        UpdateStatus::Error(err)
                    } else if has_update {
                        // Show update dialog popup
                        self.show_update_dialog = true;
                        self.update_info = Some(super::UpdateDialogInfo {
                            latest_version: latest_version.clone(),
                            release_url: release_url.clone(),
                        });
                        UpdateStatus::UpdateAvailable {
                            version: latest_version,
                            url: release_url,
                        }
                    } else {
                        UpdateStatus::UpToDate
                    };
                    self.setting_view.update(cx, |view, cx| {
                        view.set_update_status(status, cx);
                    });
                }
                Event::NicknamesLoaded { map } => {
                    let map_for_setting = map.clone();
                    self.nicknames = std::rc::Rc::new(map);
                    self.setting_view.update(cx, |v, cx| v.set_nicknames(map_for_setting, cx));
                    // Names embed into the per-row prefix width — force rebuild.
                    self.last_render_width = 0.0;
                    self.render_rows_source_count = 0;
                }
                Event::NicknameUpdated { uid, nickname } => {
                    let mut new = (*self.nicknames).clone();
                    match &nickname {
                        Some(n) if !n.is_empty() => {
                            new.insert(uid, n.clone());
                        }
                        _ => {
                            new.remove(&uid);
                        }
                    }
                    self.nicknames = std::rc::Rc::new(new);
                    self.setting_view.update(cx, |v, cx| {
                        v.apply_nickname_change(uid, nickname, cx);
                    });
                    self.last_render_width = 0.0;
                    self.render_rows_source_count = 0;
                }
                _ => {}
            }
        }
        if list_modified {
            // Render rows will be updated in render() via update_render_rows()
            self.render_rows_source_count = 0;
            self.render_rows = std::rc::Rc::new(Vec::new());
        }

        // Prune expired floating SuperChats
        if !self.floating_sc.is_empty() {
            let now = chrono::Utc::now().timestamp();
            self.floating_sc.retain(|sc| now < sc.end_time);
        }
    }
}
