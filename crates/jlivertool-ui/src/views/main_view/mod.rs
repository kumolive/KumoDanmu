//! Main window view
//!
//! This module contains the main window view and its components:
//! - `content_rendering` - BV link rendering, guard icons, DisplayMessage
//! - `user_info_card` - User info card popup component
//! - `danmu_list_item` - Unified danmu list item rendering
//! - `event_processing` - Event handling logic
//! - `render` - Render methods and Render trait implementation

mod content_rendering;
mod danmu_list_item;
mod event_processing;
mod render;
mod user_info_card;

pub use content_rendering::{render_content_with_links, DisplayMessage, RenderRow};
use content_rendering::{
    are_messages_equivalent, display_name, estimate_danmu_prefix_width, estimate_text_width,
    message_timestamp, split_content_to_lines,
};
use danmu_list_item::DanmuListItemView;
use user_info_card::{SelectedUserState, UserInfoCard};

use crate::app::UiCommand;
use crate::components::QrCodeView;
use crate::tray::{TrayManager, TrayState};
use crate::views::AudienceView;
use crate::views::SettingView;
use gpui::*;
use jlivertool_core::database::Database;
use jlivertool_core::events::Event;
use jlivertool_core::messages::{HistoryEntry, SuperChatMessage};
use jlivertool_core::types::RoomId;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

const MAX_DANMU_COUNT: usize = 200;
const HISTORY_PAGE_SIZE: usize = 100;
const SCROLL_NEAR_TOP_THRESHOLD: usize = 5;

/// Main window view state
pub struct MainView {
    event_rx: mpsc::Receiver<Event>,
    #[allow(dead_code)] // Used indirectly through closures
    command_tx: mpsc::Sender<UiCommand>,
    /// Flag indicating there are pending events to process (used by timer)
    #[allow(dead_code)]
    has_events: Arc<AtomicBool>,
    room: Option<RoomId>,
    room_title: String,
    live_status: u8,
    area_id: u64,
    online_count: u64,
    connected: bool,
    danmu_list: VecDeque<DisplayMessage>,
    /// Historical messages loaded from DB (prepended before live messages on scroll-up)
    history_msgs: VecDeque<DisplayMessage>,
    /// Earliest timestamp loaded from history (cursor for next page)
    history_cursor: Option<i64>,
    /// True when the DB has returned an empty page (no more history)
    history_exhausted: bool,
    /// Guard against concurrent history loads
    loading_history: bool,
    /// History entries loaded from DB, waiting to be applied during render
    pending_history_entries: Option<(u64, Vec<HistoryEntry>)>,
    /// User nicknames (uid → custom name) — wrapped in Rc so it can be cheaply
    /// cloned into each `DanmuListItemView` per frame.
    pub(super) nicknames: Rc<HashMap<u64, String>>,
    /// SuperChats floating on top of the danmu list (pruned when end_time elapses)
    pub(super) floating_sc: Vec<SuperChatMessage>,
    /// Flattened render rows for the uniform_list (1 source message → 1-2 rows)
    render_rows: Rc<Vec<RenderRow>>,
    /// Last window width used to compute render_rows (for change detection)
    last_render_width: f32,
    /// Number of source danmu_list items processed into render_rows (for incremental append)
    render_rows_source_count: usize,
    /// Deferred scroll-to-bottom (applied after render_rows are rebuilt)
    pending_scroll_to_bottom: bool,
    // Settings view entity
    setting_view: Entity<SettingView>,
    // Audience view entity
    audience_view: Entity<AudienceView>,
    // Database reference for statistics
    database: Option<Arc<Database>>,
    // Config store reference for window bounds
    config: Option<Arc<parking_lot::RwLock<jlivertool_core::config::ConfigStore>>>,
    // Window opacity
    opacity: f32,
    // Danmu font size
    font_size: f32,
    // Window display settings
    lite_mode: bool,
    medal_display: bool,
    interact_display: bool,
    guard_effect: bool,
    level_effect: bool,
    // Always on top
    always_on_top: bool,
    // Flag to apply always_on_top on next render
    pending_always_on_top: Option<bool>,
    // Danmu folding timeout in seconds (consecutive identical messages within this window are folded)
    fold_timeout: u32,
    // Max Full rows to scan backwards when looking for a fold match
    fold_lookback: u8,
    // Click-through mode
    click_through: bool,
    // Pending click-through change from tray (Arc for thread-safe sharing)
    pending_click_through: Arc<AtomicBool>,
    // Scroll handle for danmu list
    scroll_handle: UniformListScrollHandle,
    // Window handles for single-instance windows
    settings_window: Option<AnyWindowHandle>,
    audience_window: Option<AnyWindowHandle>,
    // Input state for danmu input (lazily initialized)
    input_state: Option<Entity<gpui_component::input::InputState>>,
    // Subscription for input events (must be kept alive)
    _input_subscription: Option<Subscription>,
    // Flag to clear input on next render (using Rc<Cell> for sharing with callback)
    pending_input_clear: Rc<Cell<bool>>,
    // Selected user for info card popup
    selected_user: SelectedUserState,
    /// Per-uid nickname input state for the user info card popup. Recreated
    /// lazily whenever the displayed uid changes.
    nickname_input: Option<(u64, Entity<gpui_component::input::InputState>)>,
    // Last saved window bounds (to avoid saving on every frame)
    last_saved_bounds: Option<(i32, i32, u32, u32)>,
    // Update dialog state
    show_update_dialog: bool,
    update_info: Option<UpdateDialogInfo>,
    // Face auth dialog state (Bilibili sometimes requires face auth before starting live)
    show_face_auth_dialog: bool,
    face_auth_qr_view: Entity<QrCodeView>,
    // Tray manager for system tray integration
    tray_manager: Option<Arc<parking_lot::Mutex<TrayManager>>>,
    // Login status for tray
    logged_in: bool,
    // Logged-in user's UID (to check if user owns the room)
    logged_in_uid: Option<u64>,
    // Pending tray command to open settings (Arc for thread-safe sharing)
    pending_open_settings: Arc<AtomicBool>,
}

/// Update dialog information
#[derive(Clone)]
pub struct UpdateDialogInfo {
    pub latest_version: String,
    pub release_url: String,
}

impl MainView {
    pub fn new(
        event_rx: mpsc::Receiver<Event>,
        command_tx: mpsc::Sender<UiCommand>,
        has_events: Arc<AtomicBool>,
        cx: &mut Context<Self>,
    ) -> Self {
        let setting_view = cx.new(SettingView::new);
        let audience_view = cx.new(AudienceView::new);
        let face_auth_qr_view = cx.new(QrCodeView::new);

        // Setup callbacks for setting view
        let tx_login = command_tx.clone();
        let tx_logout = command_tx.clone();
        let tx_room = command_tx.clone();
        let tx_opacity = command_tx.clone();

        // Get entity for opacity callback
        let entity = cx.entity().downgrade();

        setting_view.update(cx, |view, _cx| {
            view.on_qr_login(move |_window, _cx| {
                let _ = tx_login.send(UiCommand::RequestQrLogin);
            });

            view.on_logout(move |_window, _cx| {
                let _ = tx_logout.send(UiCommand::RequestLogout);
            });

            view.on_change_room(move |room_id, _window, _cx| {
                let _ = tx_room.send(UiCommand::ChangeRoom(room_id));
            });

            view.on_opacity_change({
                let entity = entity.clone();
                move |opacity, _window, cx| {
                    // Send command to persist opacity
                    let _ = tx_opacity.send(UiCommand::UpdateOpacity(opacity));
                    let _ = entity.update(cx, |view, cx| {
                        view.opacity = opacity;
                        view.audience_view
                            .update(cx, |v, cx| v.set_opacity(opacity, cx));
                        cx.notify();
                    });
                }
            });

            view.on_window_settings_change({
                let tx = command_tx.clone();
                let entity = entity.clone();
                move |lite_mode,
                      medal_display,
                      interact_display,
                      guard_effect,
                      level_effect,
                      _window,
                      cx| {
                    let _ = tx.send(UiCommand::UpdateWindowSettings {
                        lite_mode,
                        medal_display,
                        interact_display,
                        guard_effect,
                        level_effect,
                    });
                    // Update local settings and clean up messages if settings are disabled
                    let _ = entity.update(cx, |view, cx| {
                        let old_interact_display = view.interact_display;
                        let old_guard_effect = view.guard_effect;
                        let old_level_effect = view.level_effect;

                        view.lite_mode = lite_mode;
                        view.medal_display = medal_display;
                        view.interact_display = interact_display;
                        view.guard_effect = guard_effect;
                        view.level_effect = level_effect;

                        // Force rebuild of render rows on layout-affecting changes
                        view.last_render_width = 0.0;
                        view.render_rows_source_count = 0;

                        // Remove interact messages if interact_display was disabled
                        if old_interact_display && !interact_display {
                            view.danmu_list
                                .retain(|msg| !matches!(msg, DisplayMessage::Interact(_, _)));
                        }

                        // Remove entry effect messages based on settings changes
                        if (old_guard_effect && !guard_effect)
                            || (old_level_effect && !level_effect)
                        {
                            view.danmu_list.retain(|msg| {
                                if let DisplayMessage::EntryEffect(entry, _) = msg {
                                    let is_guard =
                                        entry.privilege_type >= 1 && entry.privilege_type <= 3;
                                    if is_guard {
                                        guard_effect // Keep if guard_effect is still enabled
                                    } else {
                                        level_effect // Keep if level_effect is still enabled
                                    }
                                } else {
                                    true // Keep non-entry-effect messages
                                }
                            });
                        }

                        cx.notify();
                    });
                }
            });

            view.on_theme_change({
                let tx = command_tx.clone();
                move |theme, _window, _cx| {
                    let _ = tx.send(UiCommand::UpdateTheme(theme));
                }
            });

            view.on_font_size_change({
                let tx = command_tx.clone();
                let entity = entity.clone();
                move |font_size, _window, cx| {
                    let _ = tx.send(UiCommand::UpdateFontSize(font_size));
                    // Update local font size
                    let _ = entity.update(cx, |view, cx| {
                        view.font_size = font_size;
                        view.last_render_width = 0.0; // Force rebuild of render rows
                        cx.notify();
                    });
                }
            });

            view.on_advanced_settings_change({
                let tx = command_tx.clone();
                move |max_danmu, log_level, _window, _cx| {
                    let _ = tx.send(UiCommand::UpdateAdvancedSettings { max_danmu, log_level });
                }
            });

            view.on_clear_data({
                let tx = command_tx.clone();
                move |_window, _cx| {
                    let _ = tx.send(UiCommand::ClearAllData);
                }
            });

            view.on_open_data_folder({
                let tx = command_tx.clone();
                move |_window, _cx| {
                    let _ = tx.send(UiCommand::OpenDataFolder);
                }
            });

            view.on_check_update({
                let tx = command_tx.clone();
                move |_window, _cx| {
                    let _ = tx.send(UiCommand::CheckForUpdate);
                }
            });

            view.on_auto_update_change({
                let tx = command_tx.clone();
                move |enabled, _window, _cx| {
                    let _ = tx.send(UiCommand::UpdateAutoUpdateCheck(enabled));
                }
            });

            view.on_set_nickname({
                let tx = command_tx.clone();
                move |uid, nickname, _window, _cx| {
                    let _ = tx.send(UiCommand::SetNickname { uid, nickname });
                }
            });
        });

        let this = Self {
            event_rx,
            command_tx,
            has_events: has_events.clone(),
            room: None,
            room_title: String::new(),
            live_status: 0,
            area_id: 0,
            online_count: 0,
            connected: false,
            danmu_list: VecDeque::with_capacity(MAX_DANMU_COUNT),
            history_msgs: VecDeque::new(),
            history_cursor: None,
            history_exhausted: false,
            loading_history: false,
            pending_history_entries: None,
            nicknames: Rc::new(HashMap::new()),
            floating_sc: Vec::new(),
            render_rows: Rc::new(Vec::new()),
            last_render_width: 0.0,
            render_rows_source_count: 0,
            pending_scroll_to_bottom: true,
            setting_view,
            audience_view,
            database: None,
            config: None,
            opacity: 1.0,
            font_size: 14.0,
            lite_mode: false,
            medal_display: false,
            interact_display: false,
            guard_effect: true,
            level_effect: false,
            always_on_top: false,
            pending_always_on_top: None,
            fold_timeout: 10,
            fold_lookback: 10,
            click_through: false,
            pending_click_through: Arc::new(AtomicBool::new(false)),
            scroll_handle: UniformListScrollHandle::new(),
            settings_window: None,
            audience_window: None,
            input_state: None,
            _input_subscription: None,
            pending_input_clear: Rc::new(Cell::new(false)),
            selected_user: Rc::new(RefCell::new(None)),
            nickname_input: None,
            last_saved_bounds: None,
            show_update_dialog: false,
            update_info: None,
            show_face_auth_dialog: false,
            face_auth_qr_view,
            tray_manager: None,
            logged_in: false,
            logged_in_uid: None,
            pending_open_settings: Arc::new(AtomicBool::new(false)),
        };

        // Start a timer to periodically check for new events from the backend
        // Only triggers re-render when has_events flag is set
        cx.spawn(async move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                Timer::after(Duration::from_millis(200)).await;
                // Only notify if there are pending events
                if has_events.swap(false, Ordering::Relaxed) {
                    let result = cx.update(|cx| {
                        view.update(cx, |_, cx| {
                            cx.notify();
                        })
                    });
                    if result.is_err() {
                        break; // View was dropped
                    }
                }
            }
        })
        .detach();

        this
    }

    /// Get the setting view entity
    pub fn setting_view(&self) -> &Entity<SettingView> {
        &self.setting_view
    }

    /// Set the database reference
    pub fn set_database(&mut self, db: Arc<Database>) {
        self.database = Some(db.clone());
    }

    /// Set the config store reference
    pub fn set_config(
        &mut self,
        config: Arc<parking_lot::RwLock<jlivertool_core::config::ConfigStore>>,
    ) {
        self.config = Some(config);
    }

    /// Set the tray manager reference
    pub fn set_tray_manager(&mut self, tray: Arc<parking_lot::Mutex<TrayManager>>) {
        self.tray_manager = Some(tray);
    }

    /// Set the pending_open_settings flag (shared with tray command handler)
    pub fn set_pending_open_settings_flag(&mut self, flag: Arc<AtomicBool>) {
        self.pending_open_settings = flag;
    }

    /// Set the pending_click_through flag (shared with tray command handler)
    pub fn set_pending_click_through_flag(&mut self, flag: Arc<AtomicBool>) {
        self.pending_click_through = flag;
    }

    /// Get the pending_open_settings flag for tray command handling
    pub fn pending_open_settings_flag(&self) -> Arc<AtomicBool> {
        self.pending_open_settings.clone()
    }

    /// Update the tray state based on current view state
    fn update_tray_state(&self) {
        if let Some(ref tray) = self.tray_manager {
            // Check if logged-in user owns the current room
            let is_room_owner = match (self.logged_in_uid, self.room.as_ref()) {
                (Some(uid), Some(room)) => uid == room.owner_uid(),
                _ => false,
            };

            let state = TrayState {
                room_id: self.room.as_ref().map(|r| r.real_id()),
                room_title: self.room_title.clone(),
                area_id: self.area_id,
                live_status: self.live_status,
                logged_in: self.logged_in,
                is_room_owner,
                connected: self.connected,
                window_visible: true, // We don't track this in MainView currently
                click_through: false, // Managed by TrayManager directly
            };
            tray.lock().update_state(state);

            // Update icon based on live status
            tray.lock().update_icon(self.live_status == 1);
        }
    }

    /// Check if scroll is at or near the bottom
    fn is_at_bottom(&self) -> bool {
        if self.render_rows.len() <= 1 {
            return true;
        }

        let scroll_state = self.scroll_handle.0.borrow();
        let base_handle = &scroll_state.base_handle;
        let offset = base_handle.offset();
        let max_offset = base_handle.max_offset();
        let threshold = px(50.0);
        offset.y <= -max_offset.height + threshold
    }

    /// Scroll to the bottom of the danmu list (deferred until render_rows are rebuilt)
    fn scroll_to_bottom(&mut self) {
        self.pending_scroll_to_bottom = true;
    }

    /// Apply pending scroll-to-bottom after render_rows have been rebuilt
    fn apply_pending_scroll(&mut self) {
        if self.pending_scroll_to_bottom && !self.render_rows.is_empty() {
            self.pending_scroll_to_bottom = false;
            let last_index = self.render_rows.len().saturating_sub(1);
            self.scroll_handle
                .scroll_to_item(last_index, ScrollStrategy::Bottom);
        }
    }

    /// Update render rows: full rebuild if width changed, incremental append otherwise.
    pub(super) fn update_render_rows(&mut self, window_width: f32) {
        if (window_width - self.last_render_width).abs() > 1.0 || self.last_render_width == 0.0 {
            self.rebuild_render_rows(window_width);
        } else {
            self.append_new_render_rows(window_width);
        }
    }

    /// Try to fold `msg` into a recent equivalent Full row within the timeout window.
    fn try_fold_last(&self, rows: &mut Vec<RenderRow>, msg: &DisplayMessage) -> bool {
        let cur_ts = message_timestamp(msg);
        let mut checked = 0u16;
        // anchor_ts is not monotonic when scanning backwards — a group folded into
        // later in time can have a newer anchor_ts than rows added after it.
        for i in (0..rows.len()).rev() {
            if let RenderRow::Full(ref mut existing, ref mut count, ref mut anchor_ts) = rows[i] {
                checked += 1;
                if checked > self.max_fold_scan() {
                    return false;
                }
                let age = cur_ts - *anchor_ts;
                if age > self.fold_timeout as i64 {
                    continue;
                }
                if are_messages_equivalent(existing, msg) {
                    *count += 1;
                    *anchor_ts = cur_ts;
                    return true;
                }
            }
        }
        false
    }

    /// Ceiling on Full-row scan depth to prevent O(n²) when fold_timeout is huge.
    fn max_fold_scan(&self) -> u16 {
        (self.fold_lookback as u16).max(5) * 5
    }

    /// Row height for a single danmu list item (must match DanmuListItemView::row_height).
    fn item_height(&self) -> f32 {
        if self.lite_mode {
            self.font_size + 6.0
        } else {
            self.font_size + 10.0
        }
    }

    /// Index of the first visible row in render_rows.
    fn visible_top_row_index(&self) -> usize {
        let scroll_state = self.scroll_handle.0.borrow();
        let offset = scroll_state.base_handle.offset();
        let item_height = self.item_height();
        let offset_y: f32 = (-offset.y).into();
        if offset_y > 0.0 && item_height > 0.0 {
            (offset_y / item_height) as usize
        } else {
            0
        }
    }

    /// Check if the user has scrolled near the top and trigger a history load if needed.
    fn try_load_history(&mut self, cx: &mut Context<Self>) {
        if self.loading_history || self.history_exhausted {
            return;
        }
        if self.room.is_none() || self.database.is_none() {
            return;
        }
        if self.render_rows.is_empty() {
            return;
        }

        // Only trigger when user has actually scrolled up (not just at top of a short list)
        if self.is_at_bottom() {
            return;
        }

        let top_visible = self.visible_top_row_index();
        if top_visible > SCROLL_NEAR_TOP_THRESHOLD {
            return;
        }

        let before_ts = self.history_cursor.unwrap_or_else(|| {
            // If no cursor yet, use the oldest message timestamp (check history first, then live)
            self.history_msgs
                .front()
                .or(self.danmu_list.front())
                .map(|m| message_timestamp(m))
                .unwrap_or_else(|| chrono::Utc::now().timestamp())
        });

        let db = self.database.clone();
        let rid = self.room.as_ref().unwrap().real_id();
        self.loading_history = true;

        cx.spawn(
            async move |this: WeakEntity<MainView>, cx: &mut AsyncApp| {
                let result = db
                    .as_ref()
                    .unwrap()
                    .get_messages_before(rid, before_ts, HISTORY_PAGE_SIZE as u32);
                let _ = cx.update(|cx| {
                    this.update(cx, |view, _cx| {
                        view.on_history_loaded(rid, result);
                    })
                });
            },
        )
        .detach();
    }

    /// Called from the async task when history data arrives from the DB.
    fn on_history_loaded(&mut self, request_room: u64, result: anyhow::Result<Vec<HistoryEntry>>) {
        self.loading_history = false;

        // Discard if user switched rooms while the query was in-flight
        if self.room.as_ref().map(|r| r.real_id()) != Some(request_room) {
            return;
        }

        match result {
            Ok(entries) if !entries.is_empty() => {
                self.pending_history_entries = Some((request_room, entries));
            }
            Ok(_) => {
                self.history_exhausted = true;
            }
            Err(e) => {
                tracing::warn!("Failed to load history: {}", e);
            }
        }
    }

    /// Apply pending history entries: prepend to history_msgs, rebuild render_rows,
    /// and preserve scroll position so the visible content doesn't jump.
    pub(super) fn apply_pending_history(&mut self, window_width: f32) {
        let (load_room, entries) = match self.pending_history_entries.take() {
            Some(e) => e,
            None => return,
        };

        // Discard if room changed since the load was requested
        if self.room.as_ref().map(|r| r.real_id()) != Some(load_room) {
            return;
        }

        // Record pixel-precise scroll offset before modification
        let old_offset = self.scroll_handle.0.borrow().base_handle.offset();
        let item_height = self.item_height();
        let old_render_len = self.render_rows.len();

        // Update cursor to the earliest timestamp in this batch
        if let Some(earliest) = entries.last() {
            self.history_cursor = Some(earliest.timestamp());
        }

        // Convert to DisplayMessage (entries are DESC by timestamp; reverse to chronological)
        for entry in entries.into_iter().rev() {
            let msg = match entry {
                HistoryEntry::Danmu(d, ts) => DisplayMessage::Danmu(d, ts),
                HistoryEntry::Gift(g) => {
                    let ts = g.timestamp;
                    DisplayMessage::Gift(g, ts)
                }
                HistoryEntry::Guard(g) => {
                    let ts = g.timestamp;
                    DisplayMessage::Guard(g, ts)
                }
            };
            self.history_msgs.push_front(msg);
        }

        // Force full rebuild (history was prepended)
        self.last_render_width = 0.0;
        self.render_rows_source_count = 0;
        self.rebuild_render_rows(window_width);

        // Pixel-precise scroll preservation: shift offset by exact height of prepended rows
        let new_render_len = self.render_rows.len();
        let prepended_rows = new_render_len.saturating_sub(old_render_len);
        let prepend_height = px(item_height * prepended_rows as f32);
        let new_offset = point(old_offset.x, old_offset.y - prepend_height);
        self.scroll_handle
            .0
            .borrow()
            .base_handle
            .set_offset(new_offset);

        // Cancel any pending scroll-to-bottom — user is scrolling through history
        self.pending_scroll_to_bottom = false;
    }

    /// Fully rebuild render_rows from danmu_list (and history_msgs) for the given window width.
    fn rebuild_render_rows(&mut self, window_width: f32) {
        let mut rows = Vec::new();
        let available_width = window_width - 14.0; // scrollbar buffer

        // Process history messages first (oldest at front)
        for msg in self.history_msgs.iter() {
            if !self.try_fold_last(&mut rows, msg) {
                Self::append_message_rows(
                    &mut rows,
                    msg,
                    available_width,
                    self.font_size,
                    self.lite_mode,
                    self.medal_display,
                    &self.nicknames,
                );
            }
        }
        // Then live messages
        for msg in self.danmu_list.iter() {
            if !self.try_fold_last(&mut rows, msg) {
                Self::append_message_rows(
                    &mut rows,
                    msg,
                    available_width,
                    self.font_size,
                    self.lite_mode,
                    self.medal_display,
                    &self.nicknames,
                );
            }
        }
        self.render_rows = Rc::new(rows);
        self.last_render_width = window_width;
        self.render_rows_source_count = self.history_msgs.len() + self.danmu_list.len();
    }

    /// Incrementally append new messages to render_rows.
    fn append_new_render_rows(&mut self, window_width: f32) {
        let history_len = self.history_msgs.len();
        let total_source = history_len + self.danmu_list.len();

        // If items were removed (shouldn't happen normally), full rebuild
        if total_source < self.render_rows_source_count {
            self.rebuild_render_rows(window_width);
            return;
        }

        // Nothing new to add
        if total_source == self.render_rows_source_count {
            return;
        }

        // If history was modified, full rebuild (simpler than incremental merge)
        if history_len > 0 && self.render_rows_source_count < history_len {
            self.rebuild_render_rows(window_width);
            return;
        }

        // New items are at the end of danmu_list
        let available_width = window_width - 14.0;
        let mut rows = Rc::try_unwrap(std::mem::replace(&mut self.render_rows, Rc::new(Vec::new())))
            .unwrap_or_else(|rc| (*rc).clone());

        let danmu_new_start = self.render_rows_source_count.saturating_sub(history_len);
        for i in danmu_new_start..self.danmu_list.len() {
            let msg = &self.danmu_list[i];
            if !self.try_fold_last(&mut rows, msg) {
                Self::append_message_rows(
                    &mut rows,
                    msg,
                    available_width,
                    self.font_size,
                    self.lite_mode,
                    self.medal_display,
                    &self.nicknames,
                );
            }
        }

        self.render_rows = Rc::new(rows);
        self.render_rows_source_count = total_source;
    }

    /// Convert a single DisplayMessage into one or more RenderRows and append them.
    fn append_message_rows(
        rows: &mut Vec<RenderRow>,
        msg: &DisplayMessage,
        available_width: f32,
        font_size: f32,
        lite_mode: bool,
        medal_display: bool,
        nicknames: &HashMap<u64, String>,
    ) {
        match msg {
            DisplayMessage::Danmu(danmu, _) => {
                // Skip wrapping for emoji danmu
                if danmu.emoji_content.is_some() {
                    rows.push(RenderRow::Full(msg.clone(), 1, message_timestamp(msg)));
                    return;
                }

                let label = display_name(&danmu.sender.uname, danmu.sender.uid, nicknames);
                let prefix_width =
                    estimate_danmu_prefix_width(danmu, &label, font_size, lite_mode, medal_display);
                let first_line_content_width = available_width - prefix_width;
                // Continuation line has only padding, no prefix
                let padding = if lite_mode { 4.0 * 2.0 } else { 8.0 * 2.0 };
                let continuation_content_width = available_width - padding;

                let content_width = estimate_text_width(&danmu.content, font_size);
                if content_width <= first_line_content_width || first_line_content_width <= 0.0 {
                    rows.push(RenderRow::Full(msg.clone(), 1, message_timestamp(msg)));
                } else {
                    let lines = split_content_to_lines(
                        &danmu.content,
                        font_size,
                        first_line_content_width,
                        continuation_content_width,
                    );

                    if lines.len() <= 1 {
                        rows.push(RenderRow::Full(msg.clone(), 1, message_timestamp(msg)));
                    } else {
                        rows.push(RenderRow::DanmuFirstLine {
                            danmu: danmu.clone(),
                            content_slice: lines[0].clone(),
                        });
                        for (i, line) in lines[1..].iter().enumerate() {
                            rows.push(RenderRow::DanmuContinuation {
                                danmu: danmu.clone(),
                                content_slice: line.clone(),
                                continuation_index: i,
                            });
                        }
                    }
                }
            }
            _ => {
                rows.push(RenderRow::Full(msg.clone(), 1, message_timestamp(msg)));
            }
        }
    }

    /// Open settings window
    fn open_settings_window(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        use crate::views::WindowBoundsTracker;
        use gpui_component::Root;
        use jlivertool_core::types::WindowType;

        // Check if window is already open and focus it
        if let Some(handle) = &self.settings_window {
            if cx
                .update_window(*handle, |_, window, _cx| {
                    window.activate_window();
                })
                .is_ok()
            {
                return;
            }
            self.settings_window = None;
        }

        // Load saved bounds or use default
        let bounds = if let Some(ref config) = self.config {
            let saved = config.read().get_window_config(WindowType::Setting);
            if saved.width > 0 && saved.height > 0 {
                Bounds::new(
                    point(px(saved.x as f32), px(saved.y as f32)),
                    size(px(saved.width as f32), px(saved.height as f32)),
                )
            } else {
                Bounds::centered(None, size(px(800.0), px(700.0)), cx)
            }
        } else {
            Bounds::centered(None, size(px(800.0), px(700.0)), cx)
        };

        let setting_view = self.setting_view.clone();
        let always_on_top = self.always_on_top;
        let command_tx = self.command_tx.clone();

        if let Ok(handle) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("设置".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_background: WindowBackgroundAppearance::Transparent,
                window_min_size: Some(size(px(700.0), px(500.0))),
                ..Default::default()
            },
            |new_window, cx| {
                if always_on_top {
                    crate::platform::set_window_always_on_top(new_window, true);
                }
                let tracker = cx.new(|_| {
                    WindowBoundsTracker::new(setting_view, WindowType::Setting, command_tx)
                });
                cx.new(|cx| Root::new(tracker, new_window, cx))
            },
        ) {
            self.settings_window = Some(handle.into());
        }
    }

    /// Open settings window (deferred version for use outside render context)
    /// This is called from cx.spawn() to avoid opening windows during render
    fn open_settings_window_deferred(&mut self, cx: &mut Context<Self>) {
        use crate::views::WindowBoundsTracker;
        use gpui_component::Root;
        use jlivertool_core::types::WindowType;

        // Check if window is already open and focus it
        if let Some(handle) = &self.settings_window {
            if cx
                .update_window(*handle, |_, window, _cx| {
                    window.activate_window();
                })
                .is_ok()
            {
                return;
            }
            self.settings_window = None;
        }

        // Load saved bounds or use default
        let bounds = if let Some(ref config) = self.config {
            let saved = config.read().get_window_config(WindowType::Setting);
            if saved.width > 0 && saved.height > 0 {
                Bounds::new(
                    point(px(saved.x as f32), px(saved.y as f32)),
                    size(px(saved.width as f32), px(saved.height as f32)),
                )
            } else {
                Bounds::centered(None, size(px(800.0), px(700.0)), cx)
            }
        } else {
            Bounds::centered(None, size(px(800.0), px(700.0)), cx)
        };

        let setting_view = self.setting_view.clone();
        let always_on_top = self.always_on_top;
        let command_tx = self.command_tx.clone();

        if let Ok(handle) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("设置".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_background: WindowBackgroundAppearance::Transparent,
                window_min_size: Some(size(px(700.0), px(500.0))),
                ..Default::default()
            },
            |new_window, cx| {
                if always_on_top {
                    crate::platform::set_window_always_on_top(new_window, true);
                }
                let tracker = cx.new(|_| {
                    WindowBoundsTracker::new(setting_view, WindowType::Setting, command_tx)
                });
                cx.new(|cx| Root::new(tracker, new_window, cx))
            },
        ) {
            self.settings_window = Some(handle.into());
        }
    }

    /// Open audience window
    fn open_audience_window(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        use crate::views::WindowBoundsTracker;
        use gpui_component::Root;
        use jlivertool_core::types::WindowType;

        if let Some(handle) = &self.audience_window {
            if cx
                .update_window(*handle, |_, window, _cx| {
                    window.activate_window();
                })
                .is_ok()
            {
                return;
            }
            self.audience_window = None;
        }

        let bounds = if let Some(ref config) = self.config {
            let saved = config.read().get_window_config(WindowType::Rank);
            if saved.width > 0 && saved.height > 0 {
                Bounds::new(
                    point(px(saved.x as f32), px(saved.y as f32)),
                    size(px(saved.width as f32), px(saved.height as f32)),
                )
            } else {
                Bounds::centered(None, size(px(400.0), px(500.0)), cx)
            }
        } else {
            Bounds::centered(None, size(px(400.0), px(500.0)), cx)
        };

        let audience_view = self.audience_view.clone();
        let always_on_top = self.always_on_top;
        let command_tx = self.command_tx.clone();

        if let Some(room) = &self.room {
            let room_id = room.real_id();
            let ruid = room.owner_uid();
            let tx_audience = command_tx.clone();
            let tx_guards = command_tx.clone();

            self.audience_view.update(cx, |view, _cx| {
                view.on_fetch_audience(move |_window, _cx| {
                    let _ = tx_audience.send(UiCommand::FetchAudienceList { room_id, ruid });
                });
                view.on_fetch_guards(move |page, _window, _cx| {
                    let _ = tx_guards.send(UiCommand::FetchGuardList {
                        room_id,
                        ruid,
                        page,
                    });
                });
            });
        }

        let command_tx_for_tracker = self.command_tx.clone();

        if let Ok(handle) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("观众列表".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_background: WindowBackgroundAppearance::Transparent,
                window_min_size: Some(size(px(350.0), px(300.0))),
                ..Default::default()
            },
            |new_window, cx| {
                if always_on_top {
                    crate::platform::set_window_always_on_top(new_window, true);
                }
                let tracker = cx.new(|_| {
                    WindowBoundsTracker::new(
                        audience_view,
                        WindowType::Rank,
                        command_tx_for_tracker,
                    )
                });
                cx.new(|cx| Root::new(tracker, new_window, cx))
            },
        ) {
            self.audience_window = Some(handle.into());

            if self.room.is_some() {
                let room = self.room.as_ref().unwrap();
                let room_id = room.real_id();
                let ruid = room.owner_uid();
                let _ = self
                    .command_tx
                    .send(UiCommand::FetchAudienceList { room_id, ruid });
                let _ = self.command_tx.send(UiCommand::FetchGuardList {
                    room_id,
                    ruid,
                    page: 1,
                });
            }
        }
    }
}
