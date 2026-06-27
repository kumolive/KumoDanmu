//! UI Views

pub mod audience_view;
pub mod main_view;
pub mod setting_view;
pub mod window_wrapper;

pub use audience_view::AudienceView;
pub use main_view::{MainView, render_content_with_links};
pub use setting_view::{ConfigValues, SettingView};
pub use window_wrapper::WindowBoundsTracker;
