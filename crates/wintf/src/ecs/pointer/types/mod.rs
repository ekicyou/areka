//! ポインター型定義
//!
//! ポインター入力の基本型（座標、ボタン、ホイール、状態コンポーネント）と
//! バッファ型を定義する。

use bevy_ecs::prelude::*;

// 共通幾何型
use crate::ecs::types::Point;

// ============================================================================
// 基本型定義
// ============================================================================

/// 後方互換性エイリアス（PhysicalPoint → Point）
pub type PhysicalPoint = Point;

/// ダブルクリック種別（1フレームのみ有効）
///
/// dispatch_pointer_events の末尾で None にリセットされる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DoubleClick {
    #[default]
    None,
    Left,
    Right,
    Middle,
    XButton1,
    XButton2,
}

/// ホイール回転情報（1フレームのみ有効）
///
/// WM_MOUSEWHEEL / WM_MOUSEHWHEEL から透過転送。
/// FrameFinalize でリセットされる。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WheelDelta {
    /// 垂直ホイール回転量（WHEEL_DELTA単位、正=上、負=下）
    pub vertical: i16,
    /// 水平ホイール回転量（WHEEL_DELTA単位、正=右、負=左）
    pub horizontal: i16,
}

/// ボタン解放の旗（1フレームのみ有効）
///
/// tick 中に離されたボタンごとに `true` が立つ。押下状態（`PointerState::*_down`）と
/// 独立しているため、同じ tick に押下と解放が入っても解放を落とさない。
/// `DoubleClick` と同じく dispatch の末尾でリセットされる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ButtonReleased {
    /// 左ボタンが離された
    pub left: bool,
    /// 右ボタンが離された
    pub right: bool,
    /// 中ボタンが離された
    pub middle: bool,
    /// XButton1 が離された
    pub xbutton1: bool,
    /// XButton2 が離された
    pub xbutton2: bool,
}

impl ButtonReleased {
    /// いずれかのボタンが離されたか
    #[inline]
    pub fn any(&self) -> bool {
        self.left || self.right || self.middle || self.xbutton1 || self.xbutton2
    }
}

/// ポインターボタン種別（マウスボタン）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
    XButton1,
    XButton2,
}

// ============================================================================
// PointerState コンポーネント
// ============================================================================

/// ポインター状態コンポーネント（WinUI3 スタイル）
///
/// hit_test がヒットしたエンティティに付与される。
/// コンポーネントの存在 = ホバー中。
/// Added<PointerState> で Enter を検出。
///
/// Win32マウスメッセージの情報を透過的にECSに転送する。
/// 情報の解釈（Click判定等）はアプリ側の責務。
///
/// メモリ戦略: SparseSet - 頻繁な挿入/削除
#[derive(Component, Debug, Clone, Default)]
#[component(storage = "SparseSet")]
pub struct PointerState {
    /// クライアント座標（物理ピクセル）
    pub client_point: PhysicalPoint,
    /// エンティティローカル座標（物理ピクセル）
    pub local_point: PhysicalPoint,

    // === ボタン押下（この tick に押下が届いたボタン。配りの後に `dispatch_pointer_events` が下ろす）===
    /// この tick に左ボタンが押された
    pub left_down: bool,
    /// この tick に右ボタンが押された
    pub right_down: bool,
    /// この tick に中ボタンが押された
    pub middle_down: bool,
    /// この tick に XButton1（4thボタン）が押された
    pub xbutton1_down: bool,
    /// この tick に XButton2（5thボタン）が押された
    pub xbutton2_down: bool,

    // === 修飾キー状態（wParam から透過転送）===
    /// Shift押下中 (MK_SHIFT)
    pub shift_down: bool,
    /// Ctrl押下中 (MK_CONTROL)
    pub ctrl_down: bool,

    // === ダブルクリック（1フレームのみ有効）===
    /// ダブルクリック検出（dispatch_pointer_events の末尾で None にリセット）
    pub double_click: DoubleClick,

    // === ボタン解放（1フレームのみ有効）===
    /// 解放されたボタンの旗（dispatch_pointer_events の末尾でリセット）
    pub released: ButtonReleased,

    // === ホイール（1フレームのみ有効）===
    /// ホイール回転情報（FrameFinalizeでリセット）
    pub wheel: WheelDelta,
}

// ============================================================================
// PointerLeave マーカー
// ============================================================================

/// ポインター離脱マーカー（1フレーム限り）
///
/// PointerState が削除されたフレームに付与される。
/// FrameFinalize で削除されるため、1フレームのみ存在。
///
/// メモリ戦略: SparseSet - 一時的マーカー
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct PointerLeave;

// ============================================================================
// ButtonBuffer
// ============================================================================

/// ボタンバッファ
///
/// 1 tick 内に複数のボタンイベントが発生する可能性があるため、
/// down/up の発生を記録する。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ButtonBuffer {
    /// tick中にDownが発生したか
    pub down_received: bool,
    /// tick中にUpが発生したか
    pub up_received: bool,
}

impl ButtonBuffer {
    /// ボタン押下を記録
    pub fn record_down(&mut self) {
        self.down_received = true;
    }

    /// ボタン解放を記録
    pub fn record_up(&mut self) {
        self.up_received = true;
    }
}

// ============================================================================
// ホイールバッファ（tick 内累積用）
// ============================================================================

/// ホイールバッファ
///
/// 1 tick 内に複数の WM_MOUSEWHEEL/WM_MOUSEHWHEEL が発生する可能性があるため、
/// デルタを累積する。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WheelBuffer {
    /// 垂直ホイール累積
    pub vertical: i16,
    /// 水平ホイール累積
    pub horizontal: i16,
}

impl WheelBuffer {
    /// 垂直ホイール回転を累積
    pub fn add_vertical(&mut self, delta: i16) {
        self.vertical = self.vertical.saturating_add(delta);
    }

    /// 水平ホイール回転を累積
    pub fn add_horizontal(&mut self, delta: i16) {
        self.horizontal = self.horizontal.saturating_add(delta);
    }
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests;
