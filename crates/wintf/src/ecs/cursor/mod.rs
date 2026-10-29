//! マウスカーソルの形。
//!
//! 窓の手続きが形を問い合わせる材料と答えの型、自分で作ったカーソルの持ち主、OS の標準の形を
//! 読む口、`.cur` の目録の読みを置く。どの形を出すかは使う側（[`CursorQuery`] の関数）が決め、
//! wintf は使う側の型を知らない。`unsafe` と Win32 の呼び出しは `image.rs` と `answer.rs` だけに置く。

pub mod answer;
pub mod cur_file;
mod image;

use bevy_ecs::prelude::*;
use std::path::PathBuf;
use windows::Win32::UI::WindowsAndMessaging::HCURSOR;

pub use cur_file::{CurEntry, CurFileError, pick_cur_entry, read_cur_directory};
pub use image::{OwnedCursor, cursor_size_for_dpi, stock_cursor};

/// 形を問い合わせた引き金（WM_SETCURSOR の上位語・出し直しの出どころ）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorTrigger {
    Move,
    Press { right: bool },
    Release,
    Hover,
    Wheel,
    Refresh,
}

/// 問い合わせの材料（窓の手続きが Win32 から読んだ値）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorAsk {
    pub trigger: CursorTrigger,
    /// 窓の client 物理 px（GetCursorPos → ScreenToClient）。
    pub client: (i32, i32),
    /// その位置の持ち主が窓そのものか無い（絵の透けた所）。
    pub see_through: bool,
    /// 左・右・中・拡張 2 つのどれかが今押されている（GetAsyncKeyState）。
    pub buttons_down: bool,
    /// 窓の DPI（GetDpiForWindow）。
    pub dpi: u32,
}

/// 答え。`Show` はその形を出す。`Keep` は形に触らない（今出ている形＝下に見える窓の形のまま）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorReply {
    Show(HCURSOR),
    Keep,
}

/// 窓の形を答える関数（Resource。無ければ今の振る舞い）。`None` は「この窓は受け持たない」
/// （バルーンなど＝今の振る舞いの DefWindowProcW）。
#[derive(Resource, Clone, Copy)]
pub struct CursorQuery(pub fn(&World, Entity, &CursorAsk) -> Option<CursorReply>);

/// カーソルを作れない・読めない理由。
#[derive(Debug, thiserror::Error)]
pub enum CursorError {
    #[error("カーソルのファイルを読めない: {path}")]
    Load {
        path: PathBuf,
        #[source]
        source: windows::core::Error,
    },
    #[error("カーソルを作れない")]
    Create(#[source] windows::core::Error),
    #[error(".cur の形が壊れている: {0}")]
    CurFile(CurFileError),
    #[error("標準のカーソルを読めない")]
    Stock(#[source] windows::core::Error),
}
