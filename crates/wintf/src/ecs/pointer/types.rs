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

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use PointerButton instead")]
pub type MouseButton = PointerButton;

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

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use PointerState instead")]
pub type MouseState = PointerState;

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

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use PointerLeave instead")]
pub type MouseLeave = PointerLeave;

// ============================================================================
// WindowPointerTracking コンポーネント
// ============================================================================

/// TrackMouseEvent 状態追跡
///
/// ウィンドウエンティティに自動付与される。
/// `true` = TrackMouseEvent(TME_LEAVE) が有効
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WindowPointerTracking(pub bool);

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use WindowPointerTracking instead")]
pub type WindowMouseTracking = WindowPointerTracking;

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

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use PointerBuffer instead")]
pub type MouseBuffer = PointerBuffer;

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
mod tests {
    use super::*;

    #[test]
    fn test_pointer_buffer_push() {
        let mut buffer = PointerBuffer::new();
        assert!(buffer.is_empty());

        buffer.push(PositionSample {
            x: 10.0,
            y: 20.0,
            timestamp: Instant::now(),
        });
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_pointer_buffer_max_samples() {
        let mut buffer = PointerBuffer::new();
        for i in 0..10 {
            buffer.push(PositionSample {
                x: i as f32,
                y: i as f32,
                timestamp: Instant::now(),
            });
        }
        // MAX_SAMPLES (5) を超えないことを確認
        assert_eq!(buffer.len(), PointerBuffer::MAX_SAMPLES);

        // 最新の値が最後に追加されたものであることを確認
        let latest = buffer.latest().unwrap();
        assert_eq!(latest.x, 9.0);
    }

    #[test]
    fn test_velocity_calculation() {
        let mut buffer = PointerBuffer::new();
        let t1 = Instant::now();
        buffer.push(PositionSample {
            x: 0.0,
            y: 0.0,
            timestamp: t1,
        });

        // 1サンプルでは速度は0
        let (vx, vy) = buffer.calculate_velocity();
        assert_eq!(vx, 0.0);
        assert_eq!(vy, 0.0);
    }

    #[test]
    fn test_button_buffer_state() {
        let mut buffer = ButtonBuffer::default();
        assert!(!buffer.down_received);
        assert!(!buffer.up_received);

        buffer.record_down();
        assert!(buffer.down_received);
        assert!(!buffer.up_received);

        buffer.record_up();
        assert!(buffer.down_received);
        assert!(buffer.up_received);

        buffer.reset();
        assert!(!buffer.down_received);
        assert!(!buffer.up_received);
    }

    #[test]
    fn test_wheel_buffer() {
        let mut buffer = WheelBuffer::default();
        assert_eq!(buffer.vertical, 0);
        assert_eq!(buffer.horizontal, 0);

        buffer.add_vertical(120);
        buffer.add_vertical(120);
        assert_eq!(buffer.vertical, 240);

        buffer.add_horizontal(-60);
        assert_eq!(buffer.horizontal, -60);

        buffer.reset();
        assert_eq!(buffer.vertical, 0);
        assert_eq!(buffer.horizontal, 0);
    }

    #[test]
    fn test_cursor_velocity_new() {
        let v = CursorVelocity::new(3.0, 4.0);
        assert_eq!(v.x, 3.0);
        assert_eq!(v.y, 4.0);
        assert_eq!(v.magnitude, 5.0); // 3-4-5 直角三角形
    }

    #[test]
    fn test_pointer_state_default() {
        let state = PointerState::default();
        assert_eq!(state.client_point, PhysicalPoint::default());
        assert_eq!(state.local_point, PhysicalPoint::default());
        assert!(!state.left_down);
        assert!(!state.right_down);
        assert!(!state.middle_down);
        assert!(!state.xbutton1_down);
        assert!(!state.xbutton2_down);
        assert!(!state.shift_down);
        assert!(!state.ctrl_down);
        assert_eq!(state.double_click, DoubleClick::None);
        assert_eq!(state.wheel, WheelDelta::default());
    }

    #[test]
    fn test_pointer_leave_marker() {
        // PointerLeaveはunitスタイルのマーカーコンポーネント
        let leave1 = PointerLeave;
        let leave2 = PointerLeave;
        assert_eq!(leave1, leave2);
    }

    #[test]
    fn test_window_pointer_tracking_default() {
        let tracking = WindowPointerTracking::default();
        assert!(!tracking.0);

        let tracking_enabled = WindowPointerTracking(true);
        assert!(tracking_enabled.0);
    }

    #[test]
    fn test_physical_point_new() {
        let pt = PhysicalPoint::new(100, 200);
        assert_eq!(pt.x, 100);
        assert_eq!(pt.y, 200);
    }

    #[test]
    fn test_double_click_variants() {
        assert_eq!(DoubleClick::default(), DoubleClick::None);

        // 各バリアントが異なることを確認
        assert_ne!(DoubleClick::Left, DoubleClick::Right);
        assert_ne!(DoubleClick::Middle, DoubleClick::XButton1);
        assert_ne!(DoubleClick::XButton1, DoubleClick::XButton2);
    }

    #[test]
    fn test_wheel_delta_default() {
        let delta = WheelDelta::default();
        assert_eq!(delta.vertical, 0);
        assert_eq!(delta.horizontal, 0);
    }

    #[test]
    fn test_pointer_button_enum() {
        // PointerButtonのHashトレイト実装を確認
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PointerButton::Left);
        set.insert(PointerButton::Right);
        set.insert(PointerButton::Middle);
        set.insert(PointerButton::XButton1);
        set.insert(PointerButton::XButton2);
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_velocity_calculation_two_samples_nonzero() {
        // 2 サンプル間で非ゼロ速度を計算（既存テストは 1 サンプルの 0 ケースのみ）
        let mut buffer = PointerBuffer::new();
        let t0 = Instant::now();
        buffer.push(PositionSample {
            x: 0.0,
            y: 0.0,
            timestamp: t0,
        });
        buffer.push(PositionSample {
            x: 100.0,
            y: 50.0,
            timestamp: t0 + std::time::Duration::from_millis(100),
        });

        // dx=100/0.1s=1000px/s, dy=50/0.1s=500px/s（最新2サンプル間）
        let (vx, vy) = buffer.calculate_velocity();
        assert!((vx - 1000.0).abs() < 1.0, "vx ≈ 1000px/s, got {vx}");
        assert!((vy - 500.0).abs() < 1.0, "vy ≈ 500px/s, got {vy}");
    }

    #[test]
    fn test_velocity_calculation_tiny_dt_guards_to_zero() {
        // dt < 0.0001s（同一タイムスタンプ）では 0 を返す（ゼロ除算/発散ガード）
        let mut buffer = PointerBuffer::new();
        let t = Instant::now();
        buffer.push(PositionSample {
            x: 0.0,
            y: 0.0,
            timestamp: t,
        });
        buffer.push(PositionSample {
            x: 100.0,
            y: 100.0,
            timestamp: t, // 同一タイムスタンプ → dt ≈ 0
        });

        let (vx, vy) = buffer.calculate_velocity();
        assert_eq!(vx, 0.0, "微小 dt では vx=0");
        assert_eq!(vy, 0.0, "微小 dt では vy=0");
    }

    #[test]
    fn test_velocity_uses_latest_two_samples_only() {
        // 3 サンプル以上でも「最新 2 サンプル間」のみで計算する
        let mut buffer = PointerBuffer::new();
        let t0 = Instant::now();
        // 古いサンプル（無関係な大ジャンプ）
        buffer.push(PositionSample {
            x: 0.0,
            y: 0.0,
            timestamp: t0,
        });
        buffer.push(PositionSample {
            x: 1000.0,
            y: 0.0,
            timestamp: t0 + std::time::Duration::from_millis(10),
        });
        // 最新 2 サンプル: 1000→1010 を 0.1s で
        buffer.push(PositionSample {
            x: 1010.0,
            y: 0.0,
            timestamp: t0 + std::time::Duration::from_millis(110),
        });

        let (vx, _vy) = buffer.calculate_velocity();
        // 10px/0.1s = 100px/s（最初の大ジャンプは無視される）
        assert!((vx - 100.0).abs() < 1.0, "最新2サンプルのみ: vx ≈ 100, got {vx}");
    }

    #[test]
    fn test_pointer_buffer_clear_resets_to_empty() {
        let mut buffer = PointerBuffer::new();
        buffer.push(PositionSample {
            x: 1.0,
            y: 2.0,
            timestamp: Instant::now(),
        });
        assert!(!buffer.is_empty());

        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        // クリア後は速度計算も 0（サンプル < 2）
        assert_eq!(buffer.calculate_velocity(), (0.0, 0.0));
    }

    #[test]
    fn test_pointer_buffer_eviction_keeps_newest() {
        // MAX_SAMPLES 超過で先頭（最古）が捨てられ、最新が保持される
        let mut buffer = PointerBuffer::new();
        let t0 = Instant::now();
        for i in 0..(PointerBuffer::MAX_SAMPLES + 2) {
            buffer.push(PositionSample {
                x: i as f32,
                y: 0.0,
                timestamp: t0,
            });
        }
        assert_eq!(buffer.len(), PointerBuffer::MAX_SAMPLES);
        // 最新は最後に push した値
        assert_eq!(
            buffer.latest().unwrap().x,
            (PointerBuffer::MAX_SAMPLES + 1) as f32
        );
    }

    #[test]
    fn test_wheel_buffer_saturates_at_i16_bounds() {
        // saturating_add により i16 範囲を超えても飽和（オーバーフロー panic/ラップなし）
        let mut buffer = WheelBuffer::default();
        buffer.add_vertical(i16::MAX);
        buffer.add_vertical(i16::MAX);
        assert_eq!(buffer.vertical, i16::MAX, "正方向に飽和");

        buffer.add_horizontal(i16::MIN);
        buffer.add_horizontal(i16::MIN);
        assert_eq!(buffer.horizontal, i16::MIN, "負方向に飽和");
    }

    #[test]
    fn test_cursor_velocity_zero_is_zero_magnitude() {
        // (0,0) の magnitude は 0（sqrt(0)）
        let v = CursorVelocity::new(0.0, 0.0);
        assert_eq!(v.magnitude, 0.0);
        // Default も全成分 0
        let d = CursorVelocity::default();
        assert_eq!((d.x, d.y, d.magnitude), (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_hit_test_placeholder_returns_window_entity() {
        // Phase 1 プレースホルダー: 常にウィンドウエンティティを返す
        let mut world = bevy_ecs::world::World::new();
        let window = world.spawn_empty().id();
        let other = world.spawn_empty().id();

        let hit = hit_test_placeholder(&world, window, (123.0, 456.0));
        assert_eq!(hit, Some(window), "常に window_entity を返す");
        assert_ne!(hit, Some(other), "他エンティティは返さない");
    }

    #[test]
    fn test_hit_test_with_local_coords_passes_through_screen_coords() {
        // Phase 1: スクリーン座標をそのままローカル座標として返す
        let mut world = bevy_ecs::world::World::new();
        let window = world.spawn_empty().id();

        let result = hit_test_with_local_coords(&world, window, 300, 400);
        let (entity, local) = result.expect("Phase 1 は常に Some");
        assert_eq!(entity, window);
        assert_eq!(local, PhysicalPoint::new(300, 400), "ローカル = スクリーン座標");
    }
}
