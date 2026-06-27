//! Settings view

use crate::components::{draggable_area, render_window_controls, QrCodeView};
use crate::theme::Colors;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::{
    h_flex,
    slider::{Slider, SliderEvent, SliderState},
    switch::Switch,
    v_flex,
};
use jlivertool_core::bilibili::api::{QrCodeStatus, UserInfoData};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for simple callbacks (no parameters)
type SimpleCallback = Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>;
/// Type alias for room-related callbacks (room_id parameter)
type RoomCallback = Arc<dyn Fn(u64, &mut Window, &mut App) + Send + Sync>;
/// Type alias for opacity/font size callbacks (f32 parameter)
type FloatCallback = Arc<dyn Fn(f32, &mut Window, &mut App) + Send + Sync>;
/// Type alias for window settings callback (5 bool parameters)
type WindowSettingsCallback = Arc<dyn Fn(bool, bool, bool, bool, bool, &mut Window, &mut App) + Send + Sync>;
/// Type alias for theme callback (String parameter)
type ThemeCallback = Arc<dyn Fn(String, &mut Window, &mut App) + Send + Sync>;
/// Type alias for advanced settings callback (max_danmu, log_level)
type AdvancedSettingsCallback = Arc<dyn Fn(usize, String, &mut Window, &mut App) + Send + Sync>;
/// Type alias for clear data callback
type ClearDataCallback = Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>;
/// Type alias for open data folder callback
type OpenDataFolderCallback = Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>;
/// Type alias for update check callback
type UpdateCheckCallback = Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>;
/// Type alias for auto update setting change callback (enabled)
type AutoUpdateCallback = Arc<dyn Fn(bool, &mut Window, &mut App) + Send + Sync>;
/// Type alias for nickname change callback (uid, Some(name) = set, None = remove)
type SetNicknameCallback =
    Arc<dyn Fn(u64, Option<String>, &mut Window, &mut App) + Send + Sync>;

/// Settings view state
pub struct SettingView {
    // Settings data
    settings: Arc<RwLock<SettingsData>>,
    // Room ID display
    room_id: Option<u64>,
    // Room owner UID (to check if user owns the room)
    room_owner_uid: Option<u64>,
    // Room title (display-only)
    room_title: String,
    // Room area ID (display-only)
    area_id: u64,
    // Room editing state
    room_input: Arc<RwLock<RoomInputState>>,
    // Account info
    account: Arc<RwLock<AccountState>>,
    // QR code view
    qr_code_view: Entity<QrCodeView>,
    // Callbacks
    on_qr_login: Option<SimpleCallback>,
    on_logout: Option<SimpleCallback>,
    on_change_room: Option<RoomCallback>,
    on_opacity_change: Option<FloatCallback>,
    on_window_settings_change: Option<WindowSettingsCallback>,
    on_theme_change: Option<ThemeCallback>,
    on_font_size_change: Option<FloatCallback>,
    // Merge settings
    merge_settings: Arc<RwLock<MergeSettings>>,
    // Active tab
    active_tab: usize,
    // Advanced settings
    max_danmu_count: Arc<RwLock<usize>>,
    log_level: Arc<RwLock<String>>,
    on_advanced_settings_change: Option<AdvancedSettingsCallback>,
    on_clear_data: Option<ClearDataCallback>,
    on_open_data_folder: Option<OpenDataFolderCallback>,
    show_clear_data_confirm: bool,
    // Update check
    auto_update_check: Arc<RwLock<bool>>,
    update_status: Arc<RwLock<UpdateStatus>>,
    on_check_update: Option<UpdateCheckCallback>,
    on_auto_update_change: Option<AutoUpdateCallback>,
    // Nicknames
    nicknames: Arc<RwLock<HashMap<u64, String>>>,
    on_set_nickname: Option<SetNicknameCallback>,
    /// Lazily-created per-uid InputState entities for the row editor in the
    /// nicknames tab. Keyed by uid.
    nickname_edit_inputs: HashMap<u64, Entity<InputState>>,
    /// Top-of-tab "add nickname" form state (None until first render).
    add_nickname_uid_input: Option<Entity<InputState>>,
    add_nickname_name_input: Option<Entity<InputState>>,
}

/// Update check status
#[derive(Clone, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable {
        version: String,
        url: String,
    },
    Error(String),
}

/// Tab definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Basic = 0,
    Window = 1,
    Appearance = 2,
    Nicknames = 3,
    Advanced = 4,
    About = 5,
}

impl SettingsTab {
    fn name(&self) -> &'static str {
        match self {
            Self::Basic => "基础设置",
            Self::Window => "窗口设置",
            Self::Appearance => "外观设置",
            Self::Nicknames => "昵称",
            Self::Advanced => "高级设置",
            Self::About => "关于",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            Self::Basic,
            Self::Window,
            Self::Appearance,
            Self::Nicknames,
            Self::Advanced,
            Self::About,
        ]
    }
}

/// Merge settings for danmu aggregation
#[derive(Clone, Default)]
pub struct MergeSettings {
    pub enabled: bool,
    pub rooms: Vec<u64>,
}

/// Room input state
#[derive(Clone, Default)]
pub struct RoomInputState {
    pub editing: bool,
    pub input_text: String,
    pub error: bool,
}

/// Account state
#[derive(Clone, Default)]
pub struct AccountState {
    pub logged_in: bool,
    pub user_info: Option<UserInfoData>,
    pub qr_dialog_open: bool,
    pub qr_url: Option<String>,
    pub qr_status: Option<QrCodeStatus>,
}

/// Configuration values for loading/setting config
#[derive(Debug, Clone)]
pub struct ConfigValues {
    pub always_on_top: bool,
    pub guard_effect: bool,
    pub level_effect: bool,
    pub opacity: f32,
    pub lite_mode: bool,
    pub medal_display: bool,
    pub interact_display: bool,
    pub theme: String,
    pub font_size: f32,
    pub tts_enabled: bool,
    pub tts_gift_enabled: bool,
    pub tts_sc_enabled: bool,
    pub tts_volume: f32,
}

/// Settings data that can be shared
#[derive(Clone)]
pub struct SettingsData {
    // Window display settings
    pub lite_mode: bool,
    pub medal_display: bool,
    pub interact_display: bool,
    pub guard_effect: bool,
    pub level_effect: bool,
    // General settings
    pub always_on_top: bool,
    pub opacity: f32,
    // Appearance settings
    pub theme: String,
    pub font_size: f32,
    // TTS settings
    pub tts_enabled: bool,
    pub gift_tts: bool,
    pub sc_tts: bool,
    pub tts_volume: f32,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            lite_mode: false,
            medal_display: false,
            interact_display: false,
            guard_effect: true,
            level_effect: false,
            always_on_top: false,
            opacity: 1.0,
            theme: "dark".to_string(),
            font_size: 14.0,
            tts_enabled: false,
            gift_tts: false,
            sc_tts: false,
            tts_volume: 1.0,
        }
    }
}

impl SettingView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let qr_code_view = cx.new(QrCodeView::new);

        Self {
            settings: Arc::new(RwLock::new(SettingsData::default())),
            room_id: None,
            room_owner_uid: None,
            room_title: String::new(),
            area_id: 0,
            room_input: Arc::new(RwLock::new(RoomInputState::default())),
            account: Arc::new(RwLock::new(AccountState::default())),
            qr_code_view,
            on_qr_login: None,
            on_logout: None,
            on_change_room: None,
            on_opacity_change: None,
            on_window_settings_change: None,
            on_theme_change: None,
            on_font_size_change: None,
            merge_settings: Arc::new(RwLock::new(MergeSettings::default())),
            active_tab: 0,
            max_danmu_count: Arc::new(RwLock::new(200)),
            log_level: Arc::new(RwLock::new("info".to_string())),
            on_advanced_settings_change: None,
            on_clear_data: None,
            on_open_data_folder: None,
            show_clear_data_confirm: false,
            auto_update_check: Arc::new(RwLock::new(true)),
            update_status: Arc::new(RwLock::new(UpdateStatus::default())),
            on_check_update: None,
            on_auto_update_change: None,
            nicknames: Arc::new(RwLock::new(HashMap::new())),
            on_set_nickname: None,
            nickname_edit_inputs: HashMap::new(),
            add_nickname_uid_input: None,
            add_nickname_name_input: None,
        }
    }

    /// Replace the entire nickname cache (used when NicknamesLoaded arrives).
    pub fn set_nicknames(
        &mut self,
        map: HashMap<u64, String>,
        cx: &mut Context<Self>,
    ) {
        // Drop edit inputs for uids that no longer exist; the rest will be
        // re-bound lazily during the next render.
        self.nickname_edit_inputs
            .retain(|uid, _| map.contains_key(uid));
        *self.nicknames.write() = map;
        cx.notify();
    }

    /// Apply a single (uid, nickname) change to the cache.
    /// `None` means the nickname was deleted.
    pub fn apply_nickname_change(
        &mut self,
        uid: u64,
        nickname: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let mut map = self.nicknames.write();
        match nickname {
            Some(n) if !n.is_empty() => {
                map.insert(uid, n);
            }
            _ => {
                map.remove(&uid);
                self.nickname_edit_inputs.remove(&uid);
            }
        }
        drop(map);
        cx.notify();
    }

    /// Register the callback fired when the user saves or deletes a nickname
    /// from either the row editor or the top-of-tab "add" form.
    pub fn on_set_nickname<F>(&mut self, callback: F)
    where
        F: Fn(u64, Option<String>, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_set_nickname = Some(Arc::new(callback));
    }

    /// Set room ID and owner UID to display
    pub fn set_room_id(&mut self, room_id: Option<u64>, owner_uid: Option<u64>, cx: &mut Context<Self>) {
        self.room_id = room_id;
        self.room_owner_uid = owner_uid;
        // Update input text if not editing
        if !self.room_input.read().editing {
            let mut input = self.room_input.write();
            input.input_text = room_id.map(|id| id.to_string()).unwrap_or_default();
            input.error = false;
        }
        cx.notify();
    }

    /// Set room title
    pub fn set_room_title(&mut self, title: String, cx: &mut Context<Self>) {
        self.room_title = title;
        cx.notify();
    }

    /// Set area ID for starting live
    pub fn set_area_id(&mut self, area_id: u64, cx: &mut Context<Self>) {
        self.area_id = area_id;
        cx.notify();
    }

    /// Get area ID for starting live
    pub fn get_area_id(&self) -> u64 {
        self.area_id
    }


    /// Set change room callback
    pub fn on_change_room<F>(&mut self, callback: F)
    where
        F: Fn(u64, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_change_room = Some(Arc::new(callback));
    }

    /// Set login callback
    pub fn on_qr_login<F>(&mut self, callback: F)
    where
        F: Fn(&mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_qr_login = Some(Arc::new(callback));
    }

    /// Set logout callback
    pub fn on_logout<F>(&mut self, callback: F)
    where
        F: Fn(&mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_logout = Some(Arc::new(callback));
    }

    /// Set opacity change callback
    pub fn on_opacity_change<F>(&mut self, callback: F)
    where
        F: Fn(f32, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_opacity_change = Some(Arc::new(callback));
    }

    /// Set callback for window settings changes
    /// Parameters: lite_mode, medal_display, interact_display, guard_effect, level_effect
    pub fn on_window_settings_change<F>(&mut self, callback: F)
    where
        F: Fn(bool, bool, bool, bool, bool, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_window_settings_change = Some(Arc::new(callback));
    }

    /// Set callback for theme changes
    pub fn on_theme_change<F>(&mut self, callback: F)
    where
        F: Fn(String, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_theme_change = Some(Arc::new(callback));
    }

    /// Set callback for font size changes
    pub fn on_font_size_change<F>(&mut self, callback: F)
    where
        F: Fn(f32, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_font_size_change = Some(Arc::new(callback));
    }

    /// Set advanced settings change callback
    pub fn on_advanced_settings_change<F>(&mut self, callback: F)
    where
        F: Fn(usize, String, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_advanced_settings_change = Some(Arc::new(callback));
    }

    /// Set clear data callback
    pub fn on_clear_data<F>(&mut self, callback: F)
    where
        F: Fn(&mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_clear_data = Some(Arc::new(callback));
    }

    /// Set open data folder callback
    pub fn on_open_data_folder<F>(&mut self, callback: F)
    where
        F: Fn(&mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_open_data_folder = Some(Arc::new(callback));
    }

    /// Set check update callback
    pub fn on_check_update<F>(&mut self, callback: F)
    where
        F: Fn(&mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_check_update = Some(Arc::new(callback));
    }

    /// Set auto update setting change callback
    pub fn on_auto_update_change<F>(&mut self, callback: F)
    where
        F: Fn(bool, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_auto_update_change = Some(Arc::new(callback));
    }

    /// Set auto update check setting
    pub fn set_auto_update_check(&mut self, enabled: bool, cx: &mut Context<Self>) {
        *self.auto_update_check.write() = enabled;
        cx.notify();
    }

    /// Set update status
    pub fn set_update_status(&mut self, status: UpdateStatus, cx: &mut Context<Self>) {
        *self.update_status.write() = status;
        cx.notify();
    }

    /// Set advanced settings values
    pub fn set_advanced_settings(&mut self, max_danmu: usize, log_level: String, cx: &mut Context<Self>) {
        *self.max_danmu_count.write() = max_danmu;
        *self.log_level.write() = log_level;
        cx.notify();
    }

    /// Notify window settings change
    fn notify_window_settings_change(&self, window: &mut Window, cx: &mut App) {
        if let Some(ref callback) = self.on_window_settings_change {
            let settings = self.settings.read();
            callback(
                settings.lite_mode,
                settings.medal_display,
                settings.interact_display,
                settings.guard_effect,
                settings.level_effect,
                window,
                cx,
            );
        }
    }

    /// Update login status
    pub fn set_login_status(
        &mut self,
        logged_in: bool,
        user_info: Option<UserInfoData>,
        cx: &mut Context<Self>,
    ) {
        let mut account = self.account.write();
        account.logged_in = logged_in;
        account.user_info = user_info;
        if logged_in {
            account.qr_dialog_open = false;
            account.qr_url = None;
            account.qr_status = None;
        }
        drop(account);
        cx.notify();
    }

    /// Update QR code info
    pub fn set_qr_code(&mut self, url: String, cx: &mut Context<Self>) {
        // Update QR code view
        self.qr_code_view.update(cx, |view, cx| {
            view.set_data(url.clone(), cx);
        });

        let mut account = self.account.write();
        account.qr_url = Some(url);
        account.qr_dialog_open = true;
        account.qr_status = Some(QrCodeStatus::NeedScan);
        drop(account);
        cx.notify();
    }

    /// Update QR code status
    pub fn set_qr_status(&mut self, status: QrCodeStatus, cx: &mut Context<Self>) {
        let mut account = self.account.write();
        account.qr_status = Some(status);
        if status == QrCodeStatus::Expired {
            account.qr_dialog_open = false;
        }
        drop(account);
        cx.notify();
    }

    /// Load config values
    pub fn load_config(&mut self, config: ConfigValues, cx: &mut Context<Self>) {
        let mut settings = self.settings.write();
        settings.always_on_top = config.always_on_top;
        settings.guard_effect = config.guard_effect;
        settings.level_effect = config.level_effect;
        settings.opacity = config.opacity;
        settings.lite_mode = config.lite_mode;
        settings.medal_display = config.medal_display;
        settings.interact_display = config.interact_display;
        settings.theme = config.theme;
        settings.font_size = config.font_size;
        settings.tts_enabled = config.tts_enabled;
        settings.gift_tts = config.tts_gift_enabled;
        settings.sc_tts = config.tts_sc_enabled;
        settings.tts_volume = config.tts_volume;
        drop(settings);
        cx.notify();
    }

    /// Set settings from config
    pub fn set_config(&mut self, config: ConfigValues, _cx: &mut Context<Self>) {
        let mut settings = self.settings.write();
        settings.always_on_top = config.always_on_top;
        settings.guard_effect = config.guard_effect;
        settings.level_effect = config.level_effect;
        settings.opacity = config.opacity;
        settings.lite_mode = config.lite_mode;
        settings.medal_display = config.medal_display;
        settings.interact_display = config.interact_display;
        settings.theme = config.theme;
        settings.font_size = config.font_size;
        settings.tts_enabled = config.tts_enabled;
        settings.gift_tts = config.tts_gift_enabled;
        settings.sc_tts = config.tts_sc_enabled;
        settings.tts_volume = config.tts_volume;
    }

    /// Set opacity
    pub fn set_opacity(&mut self, opacity: f32, cx: &mut Context<Self>) {
        self.settings.write().opacity = opacity;
        cx.notify();
    }

    /// Get opacity
    pub fn get_opacity(&self) -> f32 {
        self.settings.read().opacity
    }

    /// Get current settings
    pub fn get_settings(&self) -> SettingsData {
        self.settings.read().clone()
    }

    fn render_section_title(&self, title: &str) -> impl IntoElement {
        div()
            .w_full()
            .pb_3()
            .mb_4()
            .border_b_1()
            .border_color(Colors::bg_hover())
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(Colors::text_primary())
                    .child(title.to_string()),
            )
    }

    fn render_section_card(&self, content: impl IntoElement) -> impl IntoElement {
        div()
            .w_full()
            .p_5()
            .rounded(px(8.0))
            .bg(Colors::bg_secondary())
            .border_1()
            .border_color(Colors::bg_hover())
            .child(content)
    }

    fn render_setting_row(
        &self,
        label: &str,
        description: &str,
        control: impl IntoElement,
    ) -> impl IntoElement {
        h_flex()
            .w_full()
            .py_2()
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(Colors::text_primary())
                            .child(label.to_string()),
                    )
                    .when(!description.is_empty(), |this| {
                        this.child(
                            div()
                                .text_size(px(11.0))
                                .text_color(Colors::text_muted())
                                .child(description.to_string()),
                        )
                    }),
            )
            .child(control)
    }

    fn render_account_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let account = self.account.read().clone();
        let entity = cx.entity().clone();

        self.render_section_card(
            v_flex()
                .w_full()
                .child(self.render_section_title("账号设置"))
                .child(if account.logged_in {
                // Logged in state
                let user_info = account.user_info.clone();
                let on_logout = self.on_logout.clone();

                h_flex()
                    .w_full()
                    .py_3()
                    .gap_4()
                    .items_center()
                    // User avatar
                    .child({
                        let avatar_url = user_info.as_ref().and_then(|u| {
                            if u.face.is_empty() {
                                None
                            } else {
                                Some(u.face.clone())
                            }
                        });

                        if let Some(url) = avatar_url {
                            img(url)
                                .size(px(56.0))
                                .rounded_full()
                                .object_fit(gpui::ObjectFit::Cover)
                                .into_any_element()
                        } else {
                            // Fallback to initial letter
                            div()
                                .size(px(56.0))
                                .rounded_full()
                                .bg(Colors::accent())
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(24.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(Colors::text_primary())
                                .child(
                                    user_info
                                        .as_ref()
                                        .map(|u| u.name.chars().next().unwrap_or('?').to_string())
                                        .unwrap_or_else(|| "?".to_string()),
                                )
                                .into_any_element()
                        }
                    })
                    // User info
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(16.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(Colors::text_primary())
                                    .child(
                                        user_info
                                            .as_ref()
                                            .map(|u| u.name.clone())
                                            .unwrap_or_else(|| "用户".to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(Colors::text_muted())
                                    .child(format!(
                                        "UID: {}",
                                        user_info.as_ref().map(|u| u.mid).unwrap_or(0)
                                    )),
                            )
                            .child(
                                div()
                                    .id("logout-link")
                                    .text_size(px(12.0))
                                    .text_color(Colors::accent())
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(Colors::error()))
                                    .child("注销登录")
                                    .on_click(move |_event, window, cx| {
                                        if let Some(ref callback) = on_logout {
                                            callback(window, cx);
                                        }
                                    }),
                            ),
                    )
                    .into_any_element()
            } else if account.qr_dialog_open {
                // QR code scanning state
                let status_text = match account.qr_status {
                    Some(QrCodeStatus::NeedScan) => "请使用 bilibili App 扫描二维码",
                    Some(QrCodeStatus::NeedConfirm) => "已扫码，请在手机上确认登录",
                    Some(QrCodeStatus::Success) => "登录成功！",
                    Some(QrCodeStatus::Expired) => "二维码已过期，请重新获取",
                    Some(QrCodeStatus::Error) => "登录出错，请重试",
                    None => "正在生成二维码...",
                };

                let status_color = match account.qr_status {
                    Some(QrCodeStatus::NeedConfirm) => Colors::warning(),
                    Some(QrCodeStatus::Success) => Colors::success(),
                    Some(QrCodeStatus::Expired) | Some(QrCodeStatus::Error) => Colors::error(),
                    _ => Colors::text_secondary(),
                };

                let on_qr_login = self.on_qr_login.clone();

                v_flex()
                    .w_full()
                    .gap_4()
                    .items_center()
                    // QR code display
                    .child(self.qr_code_view.clone())
                    // Status text
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(status_color)
                            .child(status_text),
                    )
                    // Action buttons
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .id("cancel-qr-btn")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .bg(Colors::bg_hover())
                                    .hover(|s| s.bg(Colors::bg_secondary()))
                                    .text_size(px(12.0))
                                    .text_color(Colors::text_secondary())
                                    .child("取消")
                                    .on_click({
                                        let account = self.account.clone();
                                        move |_event, _window, cx| {
                                            account.write().qr_dialog_open = false;
                                            entity.update(cx, |_, cx| cx.notify());
                                        }
                                    }),
                            )
                            .when(
                                matches!(
                                    account.qr_status,
                                    Some(QrCodeStatus::Expired) | Some(QrCodeStatus::Error)
                                ),
                                |this| {
                                    this.child(
                                        div()
                                            .id("refresh-qr-btn")
                                            .px_4()
                                            .py_2()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .bg(Colors::accent())
                                            .hover(|s| s.opacity(0.8))
                                            .text_size(px(12.0))
                                            .text_color(Colors::button_text())
                                            .child("重新获取")
                                            .on_click(move |_event, window, cx| {
                                                if let Some(ref callback) = on_qr_login {
                                                    callback(window, cx);
                                                }
                                            }),
                                    )
                                },
                            ),
                    )
                    .into_any_element()
            } else {
                // Not logged in state - show login prompt
                let on_qr_login = self.on_qr_login.clone();

                v_flex()
                    .w_full()
                    .py_4()
                    .gap_3()
                    .items_center()
                    // Icon placeholder
                    .child(
                        div()
                            .size(px(64.0))
                            .rounded_full()
                            .bg(Colors::bg_hover())
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(28.0))
                            .text_color(Colors::text_muted())
                            .child("👤"),
                    )
                    // Description
                    .child(
                        v_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(Colors::text_primary())
                                    .child("登录 bilibili 账号"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(Colors::text_muted())
                                    .child("登录后可发送弹幕、查看更多信息"),
                            ),
                    )
                    // Login button
                    .child(
                        div()
                            .id("qr-login-btn")
                            .mt_2()
                            .px_6()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(Colors::accent())
                            .hover(|s| s.opacity(0.8))
                            .text_size(px(13.0))
                            .text_color(Colors::button_text())
                            .child("扫码登录")
                            .on_click(move |_event, window, cx| {
                                if let Some(ref callback) = on_qr_login {
                                    callback(window, cx);
                                }
                            }),
                    )
                    .into_any_element()
            }),
        )
    }

    fn render_room_section(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let room_input_state = self.room_input.clone();
        let room_input = room_input_state.read().clone();
        let on_change_room = self.on_change_room.clone();
        let current_room_id = self.room_id;

        // Create InputState using keyed state - returns a wrapper struct
        struct RoomInputWrapper {
            input: Entity<gpui_component::input::InputState>,
        }

        let state =
            window.use_keyed_state(SharedString::from("room-input-state"), cx, |window, cx| {
                let initial_value = current_room_id.map(|id| id.to_string()).unwrap_or_default();

                let input = cx.new(|cx| {
                    gpui_component::input::InputState::new(window, cx)
                        .placeholder("输入房间号...")
                        .default_value(initial_value)
                });

                RoomInputWrapper { input }
            });

        let input_state = state.read(cx).input.clone();

        self.render_section_card(
            v_flex()
                .w_full()
                .child(self.render_section_title("直播间设置"))
                .child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(Colors::text_primary())
                                .child("房间号"),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(Colors::text_muted())
                                .child("输入直播间的数字 ID"),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .items_center()
                                .child(div().flex_1().child(
                                    gpui_component::input::Input::new(&input_state).cleanable(true),
                                ))
                                .child(
                                    div()
                                        .id("confirm-room-btn")
                                        .px_4()
                                        .py(px(7.0))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .bg(Colors::accent())
                                        .hover(|s| s.opacity(0.8))
                                        .text_size(px(13.0))
                                        .text_color(Colors::button_text())
                                        .child("确认")
                                        .on_click({
                                            let input_state = input_state.clone();
                                            move |_event, window, cx| {
                                                let text = input_state.read(cx).text().to_string();
                                                if let Ok(room_id) = text.parse::<u64>() {
                                                    if let Some(ref callback) = on_change_room {
                                                        callback(room_id, window, cx);
                                                    }
                                                }
                                            }
                                        }),
                                ),
                        )
                        .when(room_input.error, |this| {
                            this.child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(Colors::error())
                                    .child("房间号无效，请检查输入"),
                            )
                        }),
                ),
        )
    }

    fn render_tab_content(
        &mut self,
        tab_index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tab = match tab_index {
            0 => SettingsTab::Basic,
            1 => SettingsTab::Window,
            2 => SettingsTab::Appearance,
            3 => SettingsTab::Nicknames,
            4 => SettingsTab::Advanced,
            5 => SettingsTab::About,
            _ => SettingsTab::Basic,
        };

        match tab {
            SettingsTab::Basic => self.render_basic_tab(window, cx).into_any_element(),
            SettingsTab::Window => self.render_window_tab(cx).into_any_element(),
            SettingsTab::Appearance => self.render_appearance_tab(window, cx).into_any_element(),
            SettingsTab::Nicknames => self.render_nicknames_tab(window, cx).into_any_element(),
            SettingsTab::Advanced => self.render_advanced_tab(cx).into_any_element(),
            SettingsTab::About => self.render_about_tab(cx).into_any_element(),
        }
    }

    fn render_basic_tab(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .p_6()
            .gap_4()
            .child(self.render_account_section(cx))
            .child(self.render_room_section(window, cx))
            .child(self.render_merge_section(window, cx))
    }

    fn render_merge_section(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let merge_settings = self.merge_settings.clone();
        let merge_enabled = merge_settings.read().enabled;
        let merge_rooms = merge_settings.read().rooms.clone();
        let entity = cx.entity().clone();

        // Room colors for visual distinction
        let room_colors = [
            hsla(0.6, 0.7, 0.5, 1.0),   // Blue
            hsla(0.3, 0.7, 0.5, 1.0),   // Green
            hsla(0.08, 0.7, 0.5, 1.0),  // Orange
            hsla(0.85, 0.7, 0.5, 1.0),  // Pink
            hsla(0.5, 0.7, 0.5, 1.0),   // Cyan
        ];

        // Create input state for adding rooms
        struct MergeRoomInputWrapper {
            input: Entity<gpui_component::input::InputState>,
        }

        let state = window.use_keyed_state(
            SharedString::from("merge-room-input-state"),
            cx,
            |window, cx| {
                let input = cx.new(|cx| {
                    gpui_component::input::InputState::new(window, cx)
                        .placeholder("输入房间号...")
                });
                MergeRoomInputWrapper { input }
            },
        );

        let input_state = state.read(cx).input.clone();

        self.render_section_card(
            v_flex()
                .w_full()
                .child(self.render_section_title("弹幕聚合"))
                .child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .child(self.render_setting_row(
                            "启用弹幕聚合",
                            "聚合多个直播间的弹幕到主窗口",
                            Switch::new("merge_enabled").checked(merge_enabled).on_click({
                                let merge_settings = merge_settings.clone();
                                let entity = entity.clone();
                                move |checked: &bool, _window, cx| {
                                    merge_settings.write().enabled = *checked;
                                    entity.update(cx, |_, cx| cx.notify());
                                }
                            }),
                        ))
                        .when(merge_enabled, |this| {
                            this.child(
                                v_flex()
                                    .w_full()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(Colors::text_muted())
                                            .child("副房间列表（最多5个）"),
                                    )
                                    // Room list
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .gap_1()
                                            .children(merge_rooms.iter().enumerate().map(|(idx, room_id)| {
                                                let color = room_colors[idx % room_colors.len()];
                                                let room_id_str = room_id.to_string();
                                                let merge_settings = merge_settings.clone();
                                                let entity = entity.clone();
                                                let room_to_remove = *room_id;

                                                h_flex()
                                                    .w_full()
                                                    .px_3()
                                                    .py_2()
                                                    .rounded(px(6.0))
                                                    .bg(Colors::bg_secondary())
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .items_center()
                                                            .child(
                                                                div()
                                                                    .size(px(8.0))
                                                                    .rounded_full()
                                                                    .bg(color),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(13.0))
                                                                    .text_color(Colors::text_primary())
                                                                    .child(room_id_str),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .id(SharedString::from(format!("remove-merge-room-{}", idx)))
                                                            .px_2()
                                                            .py_1()
                                                            .rounded(px(4.0))
                                                            .cursor_pointer()
                                                            .text_size(px(11.0))
                                                            .text_color(Colors::error())
                                                            .hover(|s| s.bg(Colors::error().opacity(0.1)))
                                                            .child("移除")
                                                            .on_click(move |_event, _window, cx| {
                                                                let mut settings = merge_settings.write();
                                                                settings.rooms.retain(|&r| r != room_to_remove);
                                                                entity.update(cx, |_, cx| cx.notify());
                                                            }),
                                                    )
                                            })),
                                    )
                                    // Add room input
                                    .when(merge_rooms.len() < 5, |this| {
                                        let merge_settings = merge_settings.clone();
                                        let entity = entity.clone();
                                        let input_state_for_click = input_state.clone();

                                        this.child(
                                            h_flex()
                                                .w_full()
                                                .gap_2()
                                                .items_center()
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .child(gpui_component::input::Input::new(&input_state).cleanable(true)),
                                                )
                                                .child(
                                                    div()
                                                        .id("add-merge-room-btn")
                                                        .px_3()
                                                        .py(px(7.0))
                                                        .rounded(px(6.0))
                                                        .cursor_pointer()
                                                        .bg(Colors::accent())
                                                        .hover(|s| s.opacity(0.8))
                                                        .text_size(px(12.0))
                                                        .text_color(Colors::button_text())
                                                        .child("添加")
                                                        .on_click(move |_event, _window, cx| {
                                                            let text = input_state_for_click.read(cx).text().to_string();
                                                            if let Ok(room_id) = text.parse::<u64>() {
                                                                let mut settings = merge_settings.write();
                                                                if settings.rooms.len() < 5 && !settings.rooms.contains(&room_id) {
                                                                    settings.rooms.push(room_id);
                                                                }
                                                            }
                                                            entity.update(cx, |_, cx| cx.notify());
                                                        }),
                                                ),
                                        )
                                    }),
                            )
                        }),
                ),
        )
    }

    fn render_window_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.settings.clone();
        let lite_mode = settings.read().lite_mode;
        let medal_display = settings.read().medal_display;
        let interact_display = settings.read().interact_display;
        let guard_effect = settings.read().guard_effect;
        let level_effect = settings.read().level_effect;

        let entity = cx.entity().clone();
        let entity2 = cx.entity().clone();
        let entity3 = cx.entity().clone();
        let entity4 = cx.entity().clone();
        let entity5 = cx.entity().clone();

        v_flex()
            .w_full()
            .p_6()
            .gap_4()
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("弹幕窗口设置"))
                        .child(self.render_setting_row(
                            "精简模式",
                            "简化弹幕窗口显示",
                            Switch::new("lite_mode").checked(lite_mode).on_click({
                                let settings = settings.clone();
                                move |checked: &bool, window, cx| {
                                    settings.write().lite_mode = *checked;
                                    entity.update(cx, |view, cx| {
                                        view.notify_window_settings_change(window, cx);
                                        cx.notify();
                                    });
                                }
                            }),
                        ))
                        .child(self.render_setting_row(
                            "勋章显示",
                            "显示用户的粉丝勋章",
                            Switch::new("medal_display").checked(medal_display).on_click({
                                let settings = settings.clone();
                                move |checked: &bool, window, cx| {
                                    settings.write().medal_display = *checked;
                                    entity2.update(cx, |view, cx| {
                                        view.notify_window_settings_change(window, cx);
                                        cx.notify();
                                    });
                                }
                            }),
                        ))
                        .child(self.render_setting_row(
                            "交互信息",
                            "显示普通用户进入和关注通知",
                            Switch::new("interact_display").checked(interact_display).on_click({
                                let settings = settings.clone();
                                move |checked: &bool, window, cx| {
                                    settings.write().interact_display = *checked;
                                    entity3.update(cx, |view, cx| {
                                        view.notify_window_settings_change(window, cx);
                                        cx.notify();
                                    });
                                }
                            }),
                        )),
                ),
            )
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("入场特效"))
                        .child(self.render_setting_row(
                            "舰长特效",
                            "显示大航海入场特效",
                            Switch::new("guard_effect").checked(guard_effect).on_click({
                                let settings = settings.clone();
                                move |checked: &bool, window, cx| {
                                    settings.write().guard_effect = *checked;
                                    entity4.update(cx, |view, cx| {
                                        view.notify_window_settings_change(window, cx);
                                        cx.notify();
                                    });
                                }
                            }),
                        ))
                        .child(self.render_setting_row(
                            "荣耀等级特效",
                            "显示荣耀等级入场特效",
                            Switch::new("level_effect").checked(level_effect).on_click({
                                let settings = settings.clone();
                                move |checked: &bool, window, cx| {
                                    settings.write().level_effect = *checked;
                                    entity5.update(cx, |view, cx| {
                                        view.notify_window_settings_change(window, cx);
                                        cx.notify();
                                    });
                                }
                            }),
                        )),
                ),
            )
    }

    fn render_appearance_tab(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .w_full()
            .p_6()
            .gap_4()
            .child(self.render_danmu_style_section(window, cx))
            .child(self.render_window_appearance_section(window, cx))
    }

    fn render_danmu_style_section(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let settings = self.settings.clone();
        let current_font_size = settings.read().font_size;

        // Create font size slider state
        struct FontSizeSliderWrapper {
            slider: Entity<SliderState>,
            subscribed: bool,
        }

        let initial_font_size = current_font_size;
        let font_size_slider_state = window.use_keyed_state(
            SharedString::from("font-size-slider-state"),
            cx,
            move |_window, cx| {
                let slider = cx.new(|_| {
                    SliderState::new()
                        .min(8.0)
                        .max(64.0)
                        .step(1.0)
                        .default_value(initial_font_size)
                });
                FontSizeSliderWrapper {
                    slider,
                    subscribed: false,
                }
            },
        );

        let slider_state = font_size_slider_state.read(cx).slider.clone();
        let is_subscribed = font_size_slider_state.read(cx).subscribed;

        // Sync slider value with current font size
        slider_state.update(cx, |state, cx| {
            let slider_value = state.value().start();
            if (slider_value - current_font_size).abs() > 0.1 {
                state.set_value(current_font_size, window, cx);
            }
        });

        // Subscribe to slider change events only once
        if !is_subscribed {
            let settings_for_slider = self.settings.clone();
            let on_font_size_change = self.on_font_size_change.clone();

            cx.subscribe_in(
                &slider_state,
                window,
                move |_view, _, event: &SliderEvent, window, cx| {
                    let SliderEvent::Change(value) = event;
                    let font_size = value.start();
                    settings_for_slider.write().font_size = font_size;
                    if let Some(ref callback) = on_font_size_change {
                        callback(font_size, window, cx);
                    }
                    cx.notify();
                },
            )
            .detach();

            font_size_slider_state.write(
                cx,
                FontSizeSliderWrapper {
                    slider: slider_state.clone(),
                    subscribed: true,
                },
            );
        }

        self.render_section_card(
            v_flex()
                .w_full()
                .child(self.render_section_title("弹幕设置"))
                .child(
                    v_flex()
                        .w_full()
                        .py_2()
                        .gap_4()
                        // Font size setting
                        .child(
                            v_flex()
                                .w_full()
                                .gap_2()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .justify_between()
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child("字体大小"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_muted())
                                                        .child("调整弹幕字体大小 (8px - 64px)"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .text_color(Colors::text_secondary())
                                                .child(format!("{}px", current_font_size as i32)),
                                        ),
                                )
                                .child(div().w_full().child(Slider::new(&slider_state))),
                        ),
                ),
        )
    }

    fn render_window_appearance_section(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let settings = self.settings.clone();
        let current_theme = settings.read().theme.clone();
        let current_opacity = settings.read().opacity;
        let entity = cx.entity().clone();
        let on_theme_change = self.on_theme_change.clone();

        let themes = vec![("light", "浅色"), ("dark", "深色")];

        // Create opacity slider state
        struct OpacitySliderWrapper {
            slider: Entity<SliderState>,
            subscribed: bool,
        }

        let initial_opacity = current_opacity;
        let opacity_slider_state = window.use_keyed_state(
            SharedString::from("appearance-opacity-slider-state"),
            cx,
            move |_window, cx| {
                let slider = cx.new(|_| {
                    SliderState::new()
                        .min(0.0)
                        .max(1.0)
                        .step(0.01)
                        .default_value(initial_opacity)
                });
                OpacitySliderWrapper {
                    slider,
                    subscribed: false,
                }
            },
        );

        let slider_state = opacity_slider_state.read(cx).slider.clone();
        let is_subscribed = opacity_slider_state.read(cx).subscribed;

        // Sync slider value with current opacity
        slider_state.update(cx, |state, cx| {
            let slider_value = state.value().start();
            if (slider_value - current_opacity).abs() > 0.001 {
                state.set_value(current_opacity, window, cx);
            }
        });

        // Subscribe to slider change events only once
        if !is_subscribed {
            let settings_for_slider = self.settings.clone();
            let on_opacity_change = self.on_opacity_change.clone();

            cx.subscribe_in(
                &slider_state,
                window,
                move |_view, _, event: &SliderEvent, window, cx| {
                    let SliderEvent::Change(value) = event;
                    let opacity = value.start();
                    settings_for_slider.write().opacity = opacity;
                    if let Some(ref callback) = on_opacity_change {
                        callback(opacity, window, cx);
                    }
                    cx.notify();
                },
            )
            .detach();

            opacity_slider_state.write(
                cx,
                OpacitySliderWrapper {
                    slider: slider_state.clone(),
                    subscribed: true,
                },
            );
        }

        self.render_section_card(
            v_flex()
                .w_full()
                .child(self.render_section_title("窗口设置"))
                // Theme selection
                .child(
                    v_flex()
                        .w_full()
                        .py_2()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(Colors::text_primary())
                                .child("主题"),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(Colors::text_muted())
                                .child("选择窗口主题颜色"),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .flex_wrap()
                                .children(themes.into_iter().map(|(id, name)| {
                                    let is_selected = id == current_theme;
                                    let settings = settings.clone();
                                    let entity = entity.clone();
                                    let theme_id = id.to_string();
                                    let on_theme_change = on_theme_change.clone();

                                    div()
                                        .id(SharedString::from(format!("theme-{}", id)))
                                        .px_3()
                                        .py_2()
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .border_1()
                                        .when(is_selected, |this| {
                                            this.border_color(Colors::accent())
                                                .bg(Colors::accent().opacity(0.1))
                                        })
                                        .when(!is_selected, |this| {
                                            this.border_color(Colors::bg_hover())
                                                .hover(|s| s.bg(Colors::bg_hover()))
                                        })
                                        .text_size(px(12.0))
                                        .text_color(Colors::text_primary())
                                        .child(name)
                                        .on_click(move |_event, _window, cx| {
                                            settings.write().theme = theme_id.clone();
                                            // Apply theme immediately
                                            crate::theme::set_theme(&theme_id);
                                            crate::theme::update_gpui_component_theme(cx);
                                            if let Some(ref callback) = on_theme_change {
                                                callback(theme_id.clone(), _window, cx);
                                            }
                                            entity.update(cx, |_, cx| cx.notify());
                                        })
                                })),
                        ),
                )
                // Opacity slider
                .child(
                    v_flex()
                        .w_full()
                        .py_2()
                        .gap_2()
                        .child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(Colors::text_primary())
                                                .child("透明度"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(Colors::text_muted())
                                                .child("除设置窗口外，其他窗口的透明度"),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(Colors::text_secondary())
                                        .child(format!("{:.2}", current_opacity)),
                                ),
                        )
                        .child(div().w_full().child(Slider::new(&slider_state))),
                ),
        )
    }

    fn render_advanced_tab(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let log_levels = vec![("info", "Info"), ("debug", "Debug"), ("warn", "Warn"), ("error", "Error")];
        let current_log_level = self.log_level.read().clone();
        let current_max_danmu = *self.max_danmu_count.read();
        let on_advanced_settings_change = self.on_advanced_settings_change.clone();
        let on_clear_data = self.on_clear_data.clone();
        let on_open_data_folder = self.on_open_data_folder.clone();
        let show_clear_confirm = self.show_clear_data_confirm;
        let max_danmu_count = self.max_danmu_count.clone();
        let log_level = self.log_level.clone();

        v_flex()
            .w_full()
            .p_6()
            .gap_4()
            // Danmu limit section
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("弹幕数量限制"))
                        .child(
                            v_flex()
                                .w_full()
                                .py_2()
                                .gap_3()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .justify_between()
                                        .items_center()
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child("主窗口最大弹幕数"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_muted())
                                                        .child("超过此数量后，旧弹幕将被移除"),
                                                ),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_center()
                                                .children([50usize, 100, 200, 500, 1000].into_iter().map(|count| {
                                                    let is_selected = count == current_max_danmu;
                                                    let max_danmu_count = max_danmu_count.clone();
                                                    let log_level = log_level.clone();
                                                    let callback = on_advanced_settings_change.clone();
                                                    div()
                                                        .id(SharedString::from(format!("max-danmu-{}", count)))
                                                        .px_2()
                                                        .py_1()
                                                        .rounded(px(4.0))
                                                        .cursor_pointer()
                                                        .border_1()
                                                        .when(is_selected, |this| {
                                                            this.border_color(Colors::accent())
                                                                .bg(Colors::accent().opacity(0.1))
                                                        })
                                                        .when(!is_selected, |this| {
                                                            this.border_color(Colors::bg_hover())
                                                                .hover(|s| s.bg(Colors::bg_hover()))
                                                        })
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_primary())
                                                        .child(count.to_string())
                                                        .on_click(move |_event, window, cx| {
                                                            *max_danmu_count.write() = count;
                                                            if let Some(ref cb) = callback {
                                                                cb(count, log_level.read().clone(), window, cx);
                                                            }
                                                            cx.refresh_windows();
                                                        })
                                                })),
                                        ),
                                ),
                        ),
                ),
            )
            // Log level section
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("日志设置"))
                        .child(
                            v_flex()
                                .w_full()
                                .py_2()
                                .gap_2()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .justify_between()
                                        .items_center()
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(Colors::text_primary())
                                                .child("日志等级"),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .children(log_levels.into_iter().map(|(id, name)| {
                                                    let is_selected = id == current_log_level;
                                                    let max_danmu_count = self.max_danmu_count.clone();
                                                    let log_level = self.log_level.clone();
                                                    let callback = self.on_advanced_settings_change.clone();
                                                    div()
                                                        .id(SharedString::from(format!("log-level-{}", id)))
                                                        .px_3()
                                                        .py_1()
                                                        .rounded(px(4.0))
                                                        .cursor_pointer()
                                                        .border_1()
                                                        .when(is_selected, |this| {
                                                            this.border_color(Colors::accent())
                                                                .bg(Colors::accent().opacity(0.1))
                                                        })
                                                        .when(!is_selected, |this| {
                                                            this.border_color(Colors::bg_hover())
                                                                .hover(|s| s.bg(Colors::bg_hover()))
                                                        })
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_primary())
                                                        .child(name)
                                                        .on_click(move |_event, window, cx| {
                                                            *log_level.write() = id.to_string();
                                                            if let Some(ref cb) = callback {
                                                                cb(*max_danmu_count.read(), id.to_string(), window, cx);
                                                            }
                                                            cx.refresh_windows();
                                                        })
                                                })),
                                        ),
                                ),
                        ),
                ),
            )
            // Data management section
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("数据管理"))
                        .child(
                            v_flex()
                                .w_full()
                                .py_2()
                                .gap_3()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .justify_between()
                                        .items_center()
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child("打开数据目录"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_muted())
                                                        .child("查看配置文件和数据库"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("open-data-folder-btn")
                                                .px_3()
                                                .py_2()
                                                .rounded(px(6.0))
                                                .cursor_pointer()
                                                .bg(Colors::bg_hover())
                                                .text_size(px(12.0))
                                                .text_color(Colors::text_primary())
                                                .hover(|s| s.bg(Colors::accent().opacity(0.2)))
                                                .child("打开目录")
                                                .on_click({
                                                    let callback = on_open_data_folder.clone();
                                                    move |_event, window, cx| {
                                                        if let Some(ref cb) = callback {
                                                            cb(window, cx);
                                                        }
                                                    }
                                                }),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .w_full()
                                        .justify_between()
                                        .items_center()
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child("清除所有数据"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_muted())
                                                        .child("删除所有弹幕、礼物、SC记录"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("clear-data-btn")
                                                .px_3()
                                                .py_2()
                                                .rounded(px(6.0))
                                                .cursor_pointer()
                                                .bg(hsla(0.0, 0.7, 0.5, 0.2))
                                                .text_size(px(12.0))
                                                .text_color(hsla(0.0, 0.7, 0.5, 1.0))
                                                .hover(|s| s.bg(hsla(0.0, 0.7, 0.5, 0.3)))
                                                .child("清除数据")
                                                .on_click(cx.listener(|this, _event, _window, cx| {
                                                    this.show_clear_data_confirm = true;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        ),
                ),
            )
            // Clear data confirmation dialog
            .when(show_clear_confirm, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(hsla(0.0, 0.0, 0.0, 0.5))
                        .flex()
                        .items_center()
                        .justify_center()
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
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("确认清除数据"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(Colors::text_secondary())
                                        .child("此操作将删除所有弹幕、礼物、SC记录。此操作不可撤销！"),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .justify_end()
                                        .child(
                                            div()
                                                .id("cancel-clear-data-btn")
                                                .px_3()
                                                .py(px(6.0))
                                                .rounded(px(4.0))
                                                .cursor_pointer()
                                                .bg(Colors::bg_hover())
                                                .text_size(px(12.0))
                                                .hover(|s| s.opacity(0.8))
                                                .on_click(cx.listener(|this, _event, _window, cx| {
                                                    this.show_clear_data_confirm = false;
                                                    cx.notify();
                                                }))
                                                .child("取消"),
                                        )
                                        .child({
                                            let callback = on_clear_data.clone();
                                            div()
                                                .id("confirm-clear-data-btn")
                                                .px_3()
                                                .py(px(6.0))
                                                .rounded(px(4.0))
                                                .cursor_pointer()
                                                .bg(hsla(0.0, 0.7, 0.5, 1.0))
                                                .text_size(px(12.0))
                                                .text_color(gpui::white())
                                                .hover(|s| s.opacity(0.8))
                                                .on_click(cx.listener(move |this, _event, window, cx| {
                                                    this.show_clear_data_confirm = false;
                                                    if let Some(ref cb) = callback {
                                                        cb(window, cx);
                                                    }
                                                    cx.notify();
                                                }))
                                                .child("确认清除")
                                        }),
                                ),
                        ),
                )
            })
    }

    fn render_nicknames_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Lazily create the "add nickname" form input states the first time the
        // tab is entered.
        if self.add_nickname_uid_input.is_none() {
            let uid_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("UID(数字)")
            });
            self.add_nickname_uid_input = Some(uid_input);
        }
        if self.add_nickname_name_input.is_none() {
            let name_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("昵称")
            });
            self.add_nickname_name_input = Some(name_input);
        }

        // Build a sorted snapshot of nicknames (by uid ascending) and make sure
        // an InputState entity exists for each row.
        let mut entries: Vec<(u64, String)> = self
            .nicknames
            .read()
            .iter()
            .map(|(uid, nick)| (*uid, nick.clone()))
            .collect();
        entries.sort_by_key(|(uid, _)| *uid);

        for (uid, nick) in entries.iter() {
            if !self.nickname_edit_inputs.contains_key(uid) {
                let default = nick.clone();
                let input = cx.new(|cx| {
                    InputState::new(window, cx).default_value(default)
                });
                self.nickname_edit_inputs.insert(*uid, input);
            }
        }

        let add_uid = self.add_nickname_uid_input.as_ref().unwrap().clone();
        let add_name = self.add_nickname_name_input.as_ref().unwrap().clone();
        let on_set = self.on_set_nickname.clone();

        let add_card = self.render_section_card(
            v_flex()
                .w_full()
                .child(self.render_section_title("添加昵称"))
                .child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .items_center()
                                .child(div().flex_1().child(Input::new(&add_uid)))
                                .child(div().flex_1().child(Input::new(&add_name)))
                                .child({
                                    let add_uid = add_uid.clone();
                                    let add_name = add_name.clone();
                                    let on_set = on_set.clone();
                                    div()
                                        .id("nickname-add-btn")
                                        .px_3()
                                        .py(px(6.0))
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .bg(Colors::accent())
                                        .hover(|s| s.opacity(0.8))
                                        .text_size(px(12.0))
                                        .text_color(Colors::button_text())
                                        .child("添加")
                                        .on_click(move |_event, window, cx| {
                                            let uid_text = add_uid.read(cx).text().to_string();
                                            let nick_text =
                                                add_name.read(cx).text().to_string();
                                            let uid = match uid_text.trim().parse::<u64>() {
                                                Ok(v) => v,
                                                Err(_) => return,
                                            };
                                            let nick = nick_text.trim().to_string();
                                            if nick.is_empty() {
                                                return;
                                            }
                                            if let Some(ref cb) = on_set {
                                                cb(uid, Some(nick), window, cx);
                                            }
                                            // Reset the form
                                            add_uid.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                            add_name.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        })
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(Colors::text_muted())
                                .child("用户 UID 是 B 站用户的数字 ID"),
                        ),
                ),
        );

        let mut list_body = v_flex().w_full().gap_2();
        if entries.is_empty() {
            list_body = list_body.child(
                div()
                    .text_size(px(12.0))
                    .text_color(Colors::text_muted())
                    .py_4()
                    .child("尚未设置任何昵称"),
            );
        } else {
            for (uid, _nick) in entries.iter() {
                let uid = *uid;
                let input = self.nickname_edit_inputs.get(&uid).unwrap().clone();
                let on_set_save = on_set.clone();
                let on_set_delete = on_set.clone();
                let input_for_save = input.clone();
                list_body = list_body.child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .min_w(px(96.0))
                                .text_size(px(12.0))
                                .text_color(Colors::text_secondary())
                                .child(format!("{}", uid)),
                        )
                        .child(div().flex_1().child(Input::new(&input)))
                        .child(
                            div()
                                .id(SharedString::from(format!("nick-save-{}", uid)))
                                .px_2()
                                .py(px(5.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .bg(Colors::accent())
                                .hover(|s| s.opacity(0.8))
                                .text_size(px(11.0))
                                .text_color(Colors::button_text())
                                .child("保存")
                                .on_click(move |_event, window, cx| {
                                    let text = input_for_save.read(cx).text().to_string();
                                    let trimmed = text.trim().to_string();
                                    let payload =
                                        if trimmed.is_empty() { None } else { Some(trimmed) };
                                    if let Some(ref cb) = on_set_save {
                                        cb(uid, payload, window, cx);
                                    }
                                }),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("nick-del-{}", uid)))
                                .px_2()
                                .py(px(5.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .bg(Colors::error())
                                .hover(|s| s.opacity(0.8))
                                .text_size(px(11.0))
                                .text_color(gpui::white())
                                .child("删除")
                                .on_click(move |_event, window, cx| {
                                    if let Some(ref cb) = on_set_delete {
                                        cb(uid, None, window, cx);
                                    }
                                }),
                        ),
                );
            }
        }
        let list_card = self.render_section_card(
            v_flex()
                .w_full()
                .child(self.render_section_title("已有昵称"))
                .child(list_body),
        );

        v_flex()
            .w_full()
            .p_6()
            .gap_4()
            .child(add_card)
            .child(list_card)
    }

    fn render_about_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .p_6()
            .gap_4()
            // App info section
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("关于"))
                        .child(
                            v_flex()
                                .w_full()
                                .py_2()
                                .gap_4()
                                // App logo and name
                                .child(
                                    h_flex()
                                        .w_full()
                                        .gap_4()
                                        .items_center()
                                        .child(
                                            div()
                                                .size(px(64.0))
                                                .rounded(px(12.0))
                                                .bg(Colors::accent())
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_size(px(28.0))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(Colors::text_primary())
                                                .child("K"),
                                        )
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_size(px(18.0))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(Colors::text_primary())
                                                        .child("KumoTool"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.0))
                                                        .text_color(Colors::text_muted())
                                                        .child("Bilibili 直播弹幕工具"),
                                                ),
                                        ),
                                )
                                // Version info
                                .child(
                                    v_flex()
                                        .w_full()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_secondary())
                                                        .child("版本"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child(env!("CARGO_PKG_VERSION")),
                                                ),
                                        )
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_secondary())
                                                        .child("框架"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child("GPUI (Rust)"),
                                                ),
                                        ),
                                ),
                        ),
                ),
            )
            // Links section
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("链接"))
                        .child(
                            v_flex()
                                .w_full()
                                .py_2()
                                .gap_2()
                                .child(
                                    div()
                                        .id("github-link")
                                        .w_full()
                                        .px_3()
                                        .py_2()
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .bg(Colors::bg_secondary())
                                        .hover(|s| s.bg(Colors::bg_hover()))
                                        .on_click(cx.listener(|_this, _event, _window, cx| {
                                            cx.open_url("https://github.com/kumolive/KumoDanmu");
                                        }))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child("GitHub 仓库"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_muted())
                                                        .child("查看源代码"),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("issues-link")
                                        .w_full()
                                        .px_3()
                                        .py_2()
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .bg(Colors::bg_secondary())
                                        .hover(|s| s.bg(Colors::bg_hover()))
                                        .on_click(cx.listener(|_this, _event, _window, cx| {
                                            cx.open_url("https://github.com/kumolive/KumoDanmu/issues");
                                        }))
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_size(px(13.0))
                                                        .text_color(Colors::text_primary())
                                                        .child("问题反馈"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(Colors::text_muted())
                                                        .child("报告问题或建议"),
                                                ),
                                        ),
                                ),
                        ),
                ),
            )
            // Update section
            .child(
                self.render_section_card(
                    v_flex()
                        .w_full()
                        .child(self.render_section_title("更新"))
                        .child(
                            v_flex()
                                .w_full()
                                .py_2()
                                .gap_3()
                                .child(self.render_setting_row(
                                    "自动检查更新",
                                    "启动时检查新版本",
                                    Switch::new("auto_update")
                                        .checked(*self.auto_update_check.read())
                                        .on_click({
                                            let entity = cx.entity().clone();
                                            let auto_update_check = self.auto_update_check.clone();
                                            let on_auto_update_change = self.on_auto_update_change.clone();
                                            move |checked: &bool, window, cx| {
                                                let checked = *checked;
                                                *auto_update_check.write() = checked;
                                                if let Some(ref callback) = on_auto_update_change {
                                                    callback(checked, window, cx);
                                                }
                                                entity.update(cx, |_, cx| cx.notify());
                                            }
                                        }),
                                ))
                                .child(self.render_update_button(cx)),
                        ),
                ),
            )
    }

    fn render_update_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.update_status.read().clone();
        let (button_text, button_enabled, status_text) = match &status {
            UpdateStatus::Idle => ("检查更新", true, None),
            UpdateStatus::Checking => ("检查中...", false, None),
            UpdateStatus::UpToDate => ("检查更新", true, Some("已是最新版本".to_string())),
            UpdateStatus::UpdateAvailable { version, .. } => {
                ("前往下载", true, Some(format!("发现新版本: {}", version)))
            }
            UpdateStatus::Error(msg) => ("重试", true, Some(format!("检查失败: {}", msg))),
        };

        let is_update_available = matches!(status, UpdateStatus::UpdateAvailable { .. });
        let update_url = if let UpdateStatus::UpdateAvailable { url, .. } = &status {
            Some(url.clone())
        } else {
            None
        };

        v_flex()
            .w_full()
            .gap_2()
            .when_some(status_text, |this, text| {
                this.child(
                    div()
                        .w_full()
                        .text_size(px(12.0))
                        .text_color(if is_update_available {
                            Colors::accent()
                        } else {
                            Colors::text_muted()
                        })
                        .child(text),
                )
            })
            .child(
                div()
                    .id("check-update-btn")
                    .w_full()
                    .px_3()
                    .py_2()
                    .rounded(px(6.0))
                    .when(button_enabled, |this| this.cursor_pointer())
                    .when(!button_enabled, |this| this.opacity(0.6))
                    .bg(if is_update_available {
                        Colors::accent()
                    } else {
                        Colors::bg_secondary()
                    })
                    .when(button_enabled, |this| this.hover(|s| s.opacity(0.8)))
                    .text_size(px(13.0))
                    .text_color(if is_update_available {
                        Colors::button_text()
                    } else {
                        Colors::text_primary()
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(button_text)
                    .when(button_enabled, |this| {
                        this.on_click(cx.listener(move |this, _event, window, cx| {
                            if let Some(ref url) = update_url {
                                cx.open_url(url);
                            } else if let Some(ref callback) = this.on_check_update {
                                *this.update_status.write() = UpdateStatus::Checking;
                                callback(window, cx);
                            }
                            cx.notify();
                        }))
                    }),
            )
    }
}

impl Render for SettingView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // macOS: left padding for traffic light buttons (buttons are ~70px wide)
        #[cfg(target_os = "macos")]
        let left_padding = px(78.0);
        #[cfg(not(target_os = "macos"))]
        let left_padding = px(24.0);

        let active_tab = self.active_tab;
        let is_maximized = window.is_maximized();

        v_flex()
            .size_full()
            .bg(Colors::bg_primary())
            .text_color(Colors::text_primary())
            .child(
                // Header - matches other window headers style
                h_flex()
                    .w_full()
                    .h(px(32.0))
                    .items_center()
                    .bg(Colors::bg_secondary())
                    .child(
                        draggable_area()
                            .flex_1()
                            .h_full()
                            .pl(left_padding)
                            .pr_2()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(Colors::text_primary())
                                    .child("设置"),
                            ),
                    )
                    .child(render_window_controls(is_maximized)),
            )
            .child(
                // Content area with tabs
                h_flex()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(
                        // Tab sidebar
                        v_flex()
                            .w(px(160.0))
                            .h_full()
                            .bg(Colors::bg_secondary())
                            .border_r_1()
                            .border_color(Colors::bg_hover())
                            .p_3()
                            .gap_1()
                            .children(
                                SettingsTab::all().into_iter().enumerate().map(|(idx, tab)| {
                                    let is_active = idx == active_tab;
                                    div()
                                        .id(SharedString::from(format!("tab-{}", idx)))
                                        .w_full()
                                        .px_3()
                                        .py_2()
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .when(is_active, |this| {
                                            this.bg(Colors::accent())
                                                .text_color(Colors::button_text())
                                        })
                                        .when(!is_active, |this| {
                                            this.text_color(Colors::text_secondary())
                                                .hover(|s| s.bg(Colors::bg_hover()))
                                        })
                                        .text_size(px(13.0))
                                        .child(tab.name())
                                        .on_click(cx.listener(move |this, _event, _window, cx| {
                                            this.active_tab = idx;
                                            cx.notify();
                                        }))
                                }),
                            ),
                    )
                    .child(
                        // Tab content
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_y_hidden()
                            .child(
                                div()
                                    .id("tab-content")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .child(self.render_tab_content(active_tab, window, cx)),
                            ),
                    ),
            )
    }
}
