//! ポインター入力モジュール
//!
//! Win32マウスメッセージをECSコンポーネントとして正規化し、
//! ウィンドウレベルのポインター状態管理を提供する。
//!
//! WinUI3 スタイルの命名規則を採用し、将来のタッチ/ペン対応に備える。

mod buffers;
mod dispatch;
pub(crate) mod nchittest_cache;
mod systems;
mod types;

// dispatch re-exports
pub use dispatch::{
    EventHandler, OnPointerEntered, OnPointerExited, OnPointerMoved, OnPointerPressed,
    OnPointerReleased, Phase, PointerEventHandler, build_bubble_path, dispatch_event_for_handler,
    dispatch_pointer_events,
};

// types re-exports
pub use types::*;

// systems re-exports
pub use systems::{
    clear_transient_pointer_state, debug_pointer_leave, debug_pointer_state_changes,
};

// 後方互換性エイリアス（systems）
#[allow(deprecated)]
pub use systems::{clear_transient_mouse_state, debug_mouse_leave, debug_mouse_state_changes};

// pub(crate) buffer helpers
pub(crate) use buffers::{
    add_wheel_horizontal, add_wheel_vertical, push_pointer_sample, record_button_down,
    record_button_up, set_modifier_state, transfer_buffers_to_world,
};
