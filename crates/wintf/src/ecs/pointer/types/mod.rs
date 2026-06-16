//! ポインター型定義
//!
//! ポインター入力の基本型（座標、ボタン、ホイール、状態コンポーネント）と
//! バッファ型を定義する。

use bevy_ecs::prelude::*;
use std::collections::VecDeque;
use std::time::Instant;

// 共通幾何型
use crate::ecs::types::Point;

// ============================================================================
// 基本型定義
// ============================================================================

/// 後方互換性エイリアス（PhysicalPoint → Point）
pub type PhysicalPoint = Point;

/// ダブルクリック種別（1フレームのみ有効）
///
/// FrameFinalize で None にリセットされる。
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

/// カーソル移動速度（ピクセル/秒）
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CursorVelocity {
    pub x: f32,
    pub y: f32,
    pub magnitude: f32,
}

impl CursorVelocity {
    /// 新しいCursorVelocityを作成
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            magnitude: (x * x + y * y).sqrt(),
        }
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
#[derive(Component, Debug, Clone)]
#[component(storage = "SparseSet")]
pub struct PointerState {
    /// クライアント座標（物理ピクセル）
    pub client_point: PhysicalPoint,
    /// エンティティローカル座標（物理ピクセル）
    pub local_point: PhysicalPoint,

    // === ボタン押下状態（wParam のビットマスクを透過転送）===
    /// 左ボタン押下中 (MK_LBUTTON)
    pub left_down: bool,
    /// 右ボタン押下中 (MK_RBUTTON)
    pub right_down: bool,
    /// 中ボタン押下中 (MK_MBUTTON)
    pub middle_down: bool,
    /// XButton1 押下中 (MK_XBUTTON1) - 4thボタン
    pub xbutton1_down: bool,
    /// XButton2 押下中 (MK_XBUTTON2) - 5thボタン
    pub xbutton2_down: bool,

    // === 修飾キー状態（wParam から透過転送）===
    /// Shift押下中 (MK_SHIFT)
    pub shift_down: bool,
    /// Ctrl押下中 (MK_CONTROL)
    pub ctrl_down: bool,

    // === ダブルクリック（1フレームのみ有効）===
    /// ダブルクリック検出（FrameFinalizeでNoneにリセット）
    pub double_click: DoubleClick,

    // === ホイール（1フレームのみ有効）===
    /// ホイール回転情報（FrameFinalizeでリセット）
    pub wheel: WheelDelta,

    // === その他 ===
    /// カーソル移動速度
    pub velocity: CursorVelocity,
    /// タイムスタンプ
    pub timestamp: Instant,
}

impl Default for PointerState {
    fn default() -> Self {
        Self {
            client_point: PhysicalPoint::default(),
            local_point: PhysicalPoint::default(),
            left_down: false,
            right_down: false,
            middle_down: false,
            xbutton1_down: false,
            xbutton2_down: false,
            shift_down: false,
            ctrl_down: false,
            double_click: DoubleClick::None,
            wheel: WheelDelta::default(),
            velocity: CursorVelocity::default(),
            timestamp: Instant::now(),
        }
    }
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
// WindowPointerTracking コンポーネント
// ============================================================================

/// TrackMouseEvent 状態追跡
///
/// ウィンドウエンティティに自動付与される。
/// `true` = TrackMouseEvent(TME_LEAVE) が有効
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WindowPointerTracking(pub bool);

// ============================================================================
// PointerBuffer
// ============================================================================

/// 位置サンプル
#[derive(Debug, Clone, Copy)]
pub struct PositionSample {
    pub x: f32,
    pub y: f32,
    pub timestamp: Instant,
}

/// ポインターバッファ（thread_local! で管理）
///
/// WndProc内で複数のWM_MOUSEMOVEが発生する可能性があるため、
/// バッファに蓄積してInputスケジュールで処理する。
#[derive(Debug, Default)]
pub struct PointerBuffer {
    samples: VecDeque<PositionSample>,
}

impl PointerBuffer {
    /// 最大サンプル数（速度計算用）
    const MAX_SAMPLES: usize = 5;

    /// 新しいPointerBufferを作成
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(Self::MAX_SAMPLES),
        }
    }

    /// サンプルを追加
    pub fn push(&mut self, sample: PositionSample) {
        if self.samples.len() >= Self::MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// 最新のサンプルを取得
    pub fn latest(&self) -> Option<&PositionSample> {
        self.samples.back()
    }

    /// サンプル数を取得
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// バッファが空かどうか
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// サンプルをクリア（速度計算用履歴は保持しない）
    pub fn clear(&mut self) {
        self.samples.clear()
    }

    /// 速度計算（最新2サンプル間）
    pub fn calculate_velocity(&self) -> (f32, f32) {
        if self.samples.len() < 2 {
            return (0.0, 0.0);
        }
        // 不変条件: 直上の早期 return により、ここに到達した時点で
        // `self.samples.len() >= 2` が保証される。したがって
        // (a) `back()` は必ず `Some`（`unwrap()` はパニックしない）、
        // (b) usize 添字 `len() - 2` はアンダーフローせず（len>=2）かつ範囲内（< len）である。
        // 内部不変条件チェック（リリースでは compile-out され挙動不変）。
        debug_assert!(
            self.samples.len() >= 2,
            "calculate_velocity invariant: samples.len() >= 2 (guarded above)"
        );
        let newest = self.samples.back().unwrap();
        let prev = &self.samples[self.samples.len() - 2];
        let dt = newest
            .timestamp
            .duration_since(prev.timestamp)
            .as_secs_f32();
        if dt < 0.0001 {
            return (0.0, 0.0);
        }
        ((newest.x - prev.x) / dt, (newest.y - prev.y) / dt)
    }
}

// ============================================================================
// ButtonBuffer
// ============================================================================

/// ボタンバッファ
///
/// 1 tick 内に複数のボタンイベントが発生する可能性があるため、
/// down/up の発生を記録する。
#[derive(Debug, Clone, Copy, Default)]
pub struct ButtonBuffer {
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

    /// バッファをリセット
    pub fn reset(&mut self) {
        self.down_received = false;
        self.up_received = false;
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
pub struct WheelBuffer {
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

    /// バッファをリセット
    pub fn reset(&mut self) {
        self.vertical = 0;
        self.horizontal = 0;
    }
}

// ============================================================================
// hit_test 仮スタブ
// ============================================================================

/// hit_test プレースホルダー（Phase 1）
///
/// event-hit-test 完了後に実際の実装に差し替え。
/// Phase 1では常にウィンドウエンティティを返す。
///
/// # Returns
/// - `Some(Entity)`: ヒットしたエンティティ
/// - `None`: ヒットなし（透過）
#[inline]
pub fn hit_test_placeholder(
    _world: &bevy_ecs::world::World,
    window_entity: Entity,
    _position: (f32, f32),
) -> Option<Entity> {
    // Phase 1: 常にウィンドウエンティティを返す
    Some(window_entity)
}

/// hit_test プレースホルダー（ローカル座標変換付き）
///
/// Phase 1ではスクリーン座標をそのまま返す。
#[inline]
pub fn hit_test_with_local_coords(
    _world: &bevy_ecs::world::World,
    window_entity: Entity,
    screen_x: i32,
    screen_y: i32,
) -> Option<(Entity, PhysicalPoint)> {
    // Phase 1: 常にウィンドウエンティティを返し、ローカル座標＝スクリーン座標
    Some((window_entity, PhysicalPoint::new(screen_x, screen_y)))
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests;
