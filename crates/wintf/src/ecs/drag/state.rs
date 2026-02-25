//! ドラッグ状態管理
//!
//! thread_local! + RefCellパターンでwndproc層のドラッグ状態を管理する。

use crate::ecs::Point;
use crate::ecs::drag::DragConstraint;
use crate::ecs::drag::capture_guard::CaptureGuard;
use crate::ecs::pointer::PhysicalPoint;
use bevy_ecs::entity::Entity;
use std::cell::RefCell;
use std::time::Instant;
use windows::Win32::Foundation::HWND;

/// ドラッグ状態（thread_local!で管理）
///
/// `CaptureGuard` を内包するため Clone 不可。
/// 読み取り専用スナップショットが必要な場合は [`DragStateSnapshot`] / [`snapshot_drag_state`] を使用する。
#[derive(Debug)]
pub enum DragState {
    /// アイドル状態、ドラッグなし
    Idle,

    /// マウス押下済み、閾値未到達
    Preparing {
        /// ドラッグ対象エンティティ
        entity: Entity,
        /// マウス押下位置（物理ピクセル、スクリーン座標）
        start_pos: PhysicalPoint,
        /// 押下時刻
        start_time: Instant,
        /// マウスキャプチャ RAII ガード
        capture_guard: CaptureGuard,
    },

    /// ドラッグ開始直後（1フレームのみ）
    JustStarted {
        /// ドラッグ対象エンティティ
        entity: Entity,
        /// ドラッグ開始位置
        start_pos: PhysicalPoint,
        /// 現在位置
        current_pos: PhysicalPoint,
        /// 開始時刻
        start_time: Instant,
        /// マウスキャプチャ RAII ガード
        capture_guard: CaptureGuard,
    },

    /// ドラッグ中、閾値到達済み
    Dragging {
        /// ドラッグ対象エンティティ
        entity: Entity,
        /// ドラッグ開始位置
        start_pos: PhysicalPoint,
        /// 現在位置
        current_pos: PhysicalPoint,
        /// 前回位置
        prev_pos: PhysicalPoint,
        /// 開始時刻
        start_time: Instant,
        // --- WndProc レベルドラッグ用の新規フィールド ---
        /// Window の Win32 ハンドル
        hwnd: HWND,
        /// ドラッグ開始時のウィンドウ位置（クライアント領域スクリーン座標）
        initial_window_pos: Point,
        /// DragConfig.move_window のキャッシュ
        move_window: bool,
        /// DragConstraint のキャッシュ
        constraint: Option<DragConstraint>,
        /// マウスキャプチャ RAII ガード
        capture_guard: CaptureGuard,
    },

    /// ドラッグ終了直後（1フレームのみ）
    JustEnded {
        /// ドラッグ対象エンティティ
        entity: Entity,
        /// 終了位置
        position: PhysicalPoint,
        /// キャンセルされたか
        cancelled: bool,
    },
}

/// ドラッグ状態の読み取り専用スナップショット。
///
/// `CaptureGuard` を含まないため Clone 可能。
/// WndProc ハンドラが状態を判定するために使用する。
#[derive(Debug, Clone)]
pub enum DragStateSnapshot {
    Idle,
    Preparing {
        entity: Entity,
        start_pos: PhysicalPoint,
        start_time: Instant,
    },
    JustStarted {
        entity: Entity,
        start_pos: PhysicalPoint,
        current_pos: PhysicalPoint,
        start_time: Instant,
    },
    Dragging {
        entity: Entity,
        start_pos: PhysicalPoint,
        current_pos: PhysicalPoint,
        prev_pos: PhysicalPoint,
        start_time: Instant,
        hwnd: HWND,
        initial_window_pos: Point,
        move_window: bool,
        constraint: Option<DragConstraint>,
    },
    JustEnded {
        entity: Entity,
        position: PhysicalPoint,
        cancelled: bool,
    },
}

impl DragState {
    /// 読み取り専用スナップショットを生成する。
    pub fn snapshot(&self) -> DragStateSnapshot {
        match self {
            DragState::Idle => DragStateSnapshot::Idle,
            DragState::Preparing {
                entity,
                start_pos,
                start_time,
                ..
            } => DragStateSnapshot::Preparing {
                entity: *entity,
                start_pos: *start_pos,
                start_time: *start_time,
            },
            DragState::JustStarted {
                entity,
                start_pos,
                current_pos,
                start_time,
                ..
            } => DragStateSnapshot::JustStarted {
                entity: *entity,
                start_pos: *start_pos,
                current_pos: *current_pos,
                start_time: *start_time,
            },
            DragState::Dragging {
                entity,
                start_pos,
                current_pos,
                prev_pos,
                start_time,
                hwnd,
                initial_window_pos,
                move_window,
                constraint,
                ..
            } => DragStateSnapshot::Dragging {
                entity: *entity,
                start_pos: *start_pos,
                current_pos: *current_pos,
                prev_pos: *prev_pos,
                start_time: *start_time,
                hwnd: *hwnd,
                initial_window_pos: *initial_window_pos,
                move_window: *move_window,
                constraint: *constraint,
            },
            DragState::JustEnded {
                entity,
                position,
                cancelled,
            } => DragStateSnapshot::JustEnded {
                entity: *entity,
                position: *position,
                cancelled: *cancelled,
            },
        }
    }
}

thread_local! {
    /// グローバルドラッグ状態（単一ドラッグのみ）
    static DRAG_STATE: RefCell<DragState> = RefCell::new(DragState::Idle);
}

/// ドラッグ状態を更新する（wndprocハンドラから呼ばれる）
pub fn update_drag_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut DragState) -> R,
{
    DRAG_STATE.with(|state| {
        let mut state = state.borrow_mut();
        f(&mut *state)
    })
}

/// ドラッグ状態を取得する（読み取り専用）
pub fn read_drag_state<F, R>(f: F) -> R
where
    F: FnOnce(&DragState) -> R,
{
    DRAG_STATE.with(|state| {
        let state = state.borrow();
        f(&*state)
    })
}

/// ドラッグ状態のスナップショットを取得する（clone 相当）。
///
/// `CaptureGuard` を含まない `DragStateSnapshot` を返す。
/// WndProc ハンドラ等で状態のパターンマッチに使用する。
#[inline]
pub fn snapshot_drag_state() -> DragStateSnapshot {
    read_drag_state(|state| state.snapshot())
}

/// ドラッグ開始準備（WM_LBUTTONDOWN時）
///
/// `hwnd` を使って `SetCapture` を呼び出し、`CaptureGuard` を生成する。
#[inline]
pub fn start_preparing(entity: Entity, pos: PhysicalPoint, hwnd: HWND) {
    update_drag_state(|state| {
        // 既にドラッグ中の場合は無視（複数ボタン同時ドラッグ禁止）
        // JustEndedは許可（前回のドラッグが終了した後の新しいドラッグ）
        if matches!(
            state,
            DragState::Preparing { .. }
                | DragState::JustStarted { .. }
                | DragState::Dragging { .. }
        ) {
            tracing::debug!("[drag] Already dragging, ignoring new button press");
            return;
        }

        let capture_guard = CaptureGuard::acquire(hwnd);

        *state = DragState::Preparing {
            entity,
            start_pos: pos,
            start_time: Instant::now(),
            capture_guard,
        };

        tracing::debug!(
            entity = ?entity,
            x = pos.x,
            y = pos.y,
            hwnd = format!("0x{:X}", hwnd.0 as usize),
            "[start_preparing] DragState -> Preparing (with capture)"
        );
    });
}

/// ドラッグ開始（閾値到達時）
///
/// Preparing → JustStarted 遷移。CaptureGuard を引き継ぐ。
#[inline]
pub fn start_dragging(current_pos: PhysicalPoint) {
    update_drag_state(|state| {
        if !matches!(state, DragState::Preparing { .. }) {
            return;
        }

        let old = std::mem::replace(state, DragState::Idle);
        if let DragState::Preparing {
            entity,
            start_pos,
            start_time,
            capture_guard,
        } = old
        {
            tracing::debug!(
                entity = ?entity,
                start_x = start_pos.x,
                start_y = start_pos.y,
                current_x = current_pos.x,
                current_y = current_pos.y,
                "[drag] Dragging started"
            );

            *state = DragState::JustStarted {
                entity,
                start_pos,
                current_pos,
                start_time,
                capture_guard,
            };
        }
    });
}

/// ドラッグ移動（WM_MOUSEMOVE時）
///
/// JustStarted → Dragging 遷移時に WindowDragContextResource から
/// HWND・初期位置・制約情報を読み取り、DragState::Dragging にセットする。
/// CaptureGuard は `std::mem::replace` で所有権を移動する。
#[inline]
pub fn update_dragging(
    current_pos: PhysicalPoint,
    drag_context: Option<&crate::ecs::drag::WindowDragContextResource>,
) {
    update_drag_state(|state| {
        match state {
            DragState::JustStarted { .. } => {
                let old = std::mem::replace(state, DragState::Idle);
                if let DragState::JustStarted {
                    entity,
                    start_pos,
                    start_time,
                    capture_guard,
                    ..
                } = old
                {
                    // WindowDragContextResource から Window 情報を読み取り
                    let (hwnd, initial_window_pos, move_window, constraint) =
                        if let Some(ctx_res) = drag_context {
                            if let Some(ctx) = ctx_res.get() {
                                (
                                    ctx.hwnd.unwrap_or(HWND::default()),
                                    ctx.initial_window_pos.unwrap_or(Point { x: 0, y: 0 }),
                                    ctx.move_window,
                                    ctx.constraint,
                                )
                            } else {
                                (HWND::default(), Point { x: 0, y: 0 }, false, None)
                            }
                        } else {
                            (HWND::default(), Point { x: 0, y: 0 }, false, None)
                        };

                    tracing::debug!(
                        entity = ?entity,
                        hwnd = format!("0x{:X}", hwnd.0 as usize),
                        initial_x = initial_window_pos.x,
                        initial_y = initial_window_pos.y,
                        move_window = move_window,
                        "[update_dragging] JustStarted -> Dragging with WindowDragContext"
                    );

                    *state = DragState::Dragging {
                        entity,
                        start_pos,
                        current_pos,
                        prev_pos: current_pos,
                        start_time,
                        hwnd,
                        initial_window_pos,
                        move_window,
                        constraint,
                        capture_guard,
                    };
                }
            }
            DragState::Dragging { .. } => {
                let old = std::mem::replace(state, DragState::Idle);
                if let DragState::Dragging {
                    entity,
                    start_pos,
                    current_pos: old_pos,
                    start_time,
                    hwnd,
                    initial_window_pos,
                    move_window,
                    constraint,
                    capture_guard,
                    ..
                } = old
                {
                    *state = DragState::Dragging {
                        entity,
                        start_pos,
                        current_pos,
                        prev_pos: old_pos,
                        start_time,
                        hwnd,
                        initial_window_pos,
                        move_window,
                        constraint,
                        capture_guard,
                    };
                }
            }
            _ => {}
        }
    });
}

/// ドラッグ終了（WM_LBUTTONUP時）
///
/// `CaptureGuard` は `RefCell` borrow 解放後にドロップされる。
/// `ReleaseCapture` が同期的に `WM_CAPTURECHANGED` をディスパッチするため、
/// borrow 中に Drop すると `RefCell already borrowed` パニックになる。
#[inline]
pub fn end_dragging(position: PhysicalPoint, cancelled: bool) {
    // CaptureGuard を closure の外に取り出し、borrow 解放後にドロップする
    let _guard = update_drag_state(|state| match state {
        DragState::Preparing { entity, .. }
        | DragState::JustStarted { entity, .. }
        | DragState::Dragging { entity, .. } => {
            let entity = *entity;
            // 旧状態から CaptureGuard を抽出
            let old = std::mem::replace(state, DragState::Idle);
            let capture_guard = match old {
                DragState::Preparing { capture_guard, .. }
                | DragState::JustStarted { capture_guard, .. }
                | DragState::Dragging { capture_guard, .. } => Some(capture_guard),
                _ => None,
            };
            *state = DragState::JustEnded {
                entity,
                position,
                cancelled,
            };

            tracing::debug!(
                entity = ?entity,
                x = position.x,
                y = position.y,
                cancelled,
                "[drag] Dragging ended"
            );
            capture_guard
        }
        _ => None,
    });
    // _guard がここでドロップ → ReleaseCapture が RefCell borrow 外で実行される
}

/// ドラッグキャンセル（ESCキー、WM_CANCELMODE時）
///
/// `CaptureGuard` は `RefCell` borrow 解放後にドロップされる。
#[inline]
pub fn cancel_dragging() {
    let _guard = update_drag_state(|state| match state {
        DragState::Preparing {
            entity, start_pos, ..
        }
        | DragState::JustStarted {
            entity, start_pos, ..
        }
        | DragState::Dragging {
            entity, start_pos, ..
        } => {
            let entity = *entity;
            let position = *start_pos;
            // 旧状態から CaptureGuard を抽出
            let old = std::mem::replace(state, DragState::Idle);
            let capture_guard = match old {
                DragState::Preparing { capture_guard, .. }
                | DragState::JustStarted { capture_guard, .. }
                | DragState::Dragging { capture_guard, .. } => Some(capture_guard),
                _ => None,
            };
            *state = DragState::JustEnded {
                entity,
                position,
                cancelled: true,
            };

            tracing::debug!(
                entity = ?entity,
                "[drag] Dragging cancelled"
            );
            capture_guard
        }
        _ => None,
    });
    // _guard がここでドロップ
}

/// ドラッグ状態をIdleにリセット（dispatch_drag_events後）
#[inline]
pub fn reset_to_idle() {
    update_drag_state(|state| {
        if matches!(state, DragState::JustEnded { .. }) {
            *state = DragState::Idle;
        }
    });
}

/// ドラッグ準備中をDraggingに遷移させるか判定する
#[inline]
pub fn check_threshold(current_pos: PhysicalPoint, threshold: i32) -> bool {
    read_drag_state(|state| {
        if let DragState::Preparing { start_pos, .. } = state {
            let dx = current_pos.x - start_pos.x;
            let dy = current_pos.y - start_pos.y;
            let distance_sq = dx * dx + dy * dy;
            let threshold_sq = threshold * threshold;
            let result = distance_sq >= threshold_sq;

            tracing::debug!(
                dx,
                dy,
                distance_sq,
                threshold_sq,
                result,
                "[check_threshold]"
            );

            result
        } else {
            tracing::warn!(state = ?state, "[check_threshold] Not in Preparing state");
            false
        }
    })
}
