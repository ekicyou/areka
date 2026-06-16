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
    static DRAG_STATE: RefCell<DragState> = const { RefCell::new(DragState::Idle) };
}

/// ドラッグ状態を更新する（wndprocハンドラから呼ばれる）
pub fn update_drag_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut DragState) -> R,
{
    DRAG_STATE.with(|state| {
        let mut state = state.borrow_mut();
        f(&mut state)
    })
}

/// ドラッグ状態を取得する（読み取り専用）
pub fn read_drag_state<F, R>(f: F) -> R
where
    F: FnOnce(&DragState) -> R,
{
    DRAG_STATE.with(|state| {
        let state = state.borrow();
        f(&state)
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
            // 不変条件（キャプチャ解放保証）: 外側 match で state は
            // Preparing/JustStarted/Dragging のいずれかに確定しており、`old` は同一バリアント。
            // よってこの抽出は必ず Some を返し、`_ => None` は構造的に到達不能。
            // 取り出した CaptureGuard は呼び出し元（_guard）で borrow 解放後にドロップされ
            // ReleaseCapture が必ず実行される（解放漏れなし）。debug_assert はリリースで
            // compile-out（挙動不変）、well-formed 状態では発火しない。
            debug_assert!(
                capture_guard.is_some(),
                "end_dragging: アクティブなドラッグ状態から CaptureGuard が抽出されるべき（キャプチャ解放保証）"
            );
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
            // 不変条件（キャプチャ解放保証）: end_dragging と同様、外側 match で
            // アクティブなドラッグ状態に確定済みのため抽出は必ず Some。取り出した
            // CaptureGuard は _guard で borrow 解放後にドロップされ ReleaseCapture が
            // 実行される（キャンセル経路でも解放漏れなし）。リリースで compile-out。
            debug_assert!(
                capture_guard.is_some(),
                "cancel_dragging: アクティブなドラッグ状態から CaptureGuard が抽出されるべき（キャプチャ解放保証）"
            );
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
///
/// # 整数境界の注意（挙動非破壊で記録）
/// `dx`/`dy` は i32 座標差、`distance_sq = dx*dx + dy*dy` と `threshold_sq = threshold*threshold`
/// はいずれも i32 乗算であり、極値座標差では debug ビルドで桁あふれ panic（`|dx| > 46340` で
/// `dx*dx` が i32::MAX 超過）・release ビルドでラップする理論的経路がある。ただし本番座標は
/// WM lparam 由来の i16 クライアント座標＋ウィンドウ位置オフセット（実モニタ幾何で有界）であり、
/// 実用座標では桁あふれしない。なお本関数は **本番呼び出しがゼロ**（ワークスペース全 grep で
/// 呼び出し元はこの in-source テストのみ）で、本番の閾値判定は `window_proc/mouse_move.rs`
/// 内に同一算術がインライン複製されている（W7a 境界）。飽和/checked 化や複製統合は計算結果や
/// 構造を変える挙動変更のため本ループでは適用せず P62 に記録（R2.4/R5.2）。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::drag::{WindowDragContext, WindowDragContextResource};

    // NOTE: `DRAG_STATE` is a thread_local。テストワーカースレッドは再利用され得るため、
    // 各テストは Idle へ強制リセットしてから開始する（前のテストが残した状態への依存を排除）。
    // SetCapture/ReleaseCapture は HWND::default()（null）に対し UI スレッド外では実質 no-op の
    // ため、状態機械そのものはデバイス非依存に検証できる（capture_guard_panic_safety_test と同じ前提）。

    /// thread_local を Idle へ強制的に戻す（CaptureGuard が残っていれば borrow 解放後にドロップ）。
    fn force_idle() {
        let _guard = update_drag_state(|state| {
            let old = std::mem::replace(state, DragState::Idle);
            // 旧状態に CaptureGuard があれば取り出して borrow 解放後にドロップさせる
            match old {
                DragState::Preparing { capture_guard, .. }
                | DragState::JustStarted { capture_guard, .. }
                | DragState::Dragging { capture_guard, .. } => Some(capture_guard),
                _ => None,
            }
        });
    }

    fn null_hwnd() -> HWND {
        HWND::default()
    }

    fn entity(idx: u32) -> Entity {
        // bevy の World 無しでテスト用 Entity を生成する（index→Entity の決定論的生成）
        Entity::from_raw_u32(idx).expect("valid entity index")
    }

    /// snapshot のバリアント判別子のみを比較するためのヘルパ。
    fn variant_name(s: &DragStateSnapshot) -> &'static str {
        match s {
            DragStateSnapshot::Idle => "Idle",
            DragStateSnapshot::Preparing { .. } => "Preparing",
            DragStateSnapshot::JustStarted { .. } => "JustStarted",
            DragStateSnapshot::Dragging { .. } => "Dragging",
            DragStateSnapshot::JustEnded { .. } => "JustEnded",
        }
    }

    // --- start_preparing -----------------------------------------------------

    /// Idle から start_preparing で Preparing へ遷移し、entity/start_pos を保持する。
    #[test]
    fn test_start_preparing_from_idle_enters_preparing() {
        force_idle();
        let e = entity(1);
        let pos = PhysicalPoint::new(10, 20);
        start_preparing(e, pos, null_hwnd());

        let snap = snapshot_drag_state();
        match snap {
            DragStateSnapshot::Preparing {
                entity, start_pos, ..
            } => {
                assert_eq!(entity, e);
                assert_eq!(start_pos.x, 10);
                assert_eq!(start_pos.y, 20);
            }
            other => panic!("expected Preparing, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// 既に Preparing 中の start_preparing は無視される（複数ボタン同時ドラッグ禁止）。
    /// 既存の entity/start_pos が維持されることを確認する。
    #[test]
    fn test_start_preparing_ignored_when_already_active() {
        force_idle();
        let first = entity(1);
        let second = entity(2);
        start_preparing(first, PhysicalPoint::new(1, 1), null_hwnd());
        // 2 回目（別エンティティ・別座標）は無視されるはず
        start_preparing(second, PhysicalPoint::new(99, 99), null_hwnd());

        let snap = snapshot_drag_state();
        match snap {
            DragStateSnapshot::Preparing {
                entity, start_pos, ..
            } => {
                assert_eq!(entity, first, "最初の press が維持されるべき");
                assert_eq!((start_pos.x, start_pos.y), (1, 1));
            }
            other => panic!("expected Preparing, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// JustEnded からの start_preparing は許可される（前ドラッグ終了後の新規ドラッグ）。
    #[test]
    fn test_start_preparing_allowed_from_just_ended() {
        force_idle();
        let e1 = entity(1);
        start_preparing(e1, PhysicalPoint::new(5, 5), null_hwnd());
        end_dragging(PhysicalPoint::new(5, 5), false); // → JustEnded
        assert!(matches!(snapshot_drag_state(), DragStateSnapshot::JustEnded { .. }));

        let e2 = entity(2);
        start_preparing(e2, PhysicalPoint::new(7, 8), null_hwnd());
        match snapshot_drag_state() {
            DragStateSnapshot::Preparing { entity, .. } => assert_eq!(entity, e2),
            other => panic!("expected Preparing, got {}", variant_name(&other)),
        }
        force_idle();
    }

    // --- start_dragging ------------------------------------------------------

    /// Preparing → JustStarted 遷移。entity/start_pos は維持、current_pos が反映される。
    #[test]
    fn test_start_dragging_preparing_to_just_started() {
        force_idle();
        let e = entity(3);
        start_preparing(e, PhysicalPoint::new(10, 10), null_hwnd());
        start_dragging(PhysicalPoint::new(18, 14));

        match snapshot_drag_state() {
            DragStateSnapshot::JustStarted {
                entity,
                start_pos,
                current_pos,
                ..
            } => {
                assert_eq!(entity, e);
                assert_eq!((start_pos.x, start_pos.y), (10, 10));
                assert_eq!((current_pos.x, current_pos.y), (18, 14));
            }
            other => panic!("expected JustStarted, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// start_dragging は Preparing 以外では no-op（Idle のまま）。
    #[test]
    fn test_start_dragging_noop_when_not_preparing() {
        force_idle();
        start_dragging(PhysicalPoint::new(1, 1));
        assert!(
            matches!(snapshot_drag_state(), DragStateSnapshot::Idle),
            "Idle からの start_dragging は何もしないべき"
        );
        force_idle();
    }

    // --- update_dragging -----------------------------------------------------

    /// JustStarted → Dragging 遷移。drag_context=None のとき HWND/位置/move_window/constraint は
    /// デフォルト値（null HWND, (0,0), false, None）になる。
    #[test]
    fn test_update_dragging_just_started_to_dragging_without_context() {
        force_idle();
        let e = entity(4);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        start_dragging(PhysicalPoint::new(6, 6));
        update_dragging(PhysicalPoint::new(6, 6), None);

        match snapshot_drag_state() {
            DragStateSnapshot::Dragging {
                entity,
                current_pos,
                prev_pos,
                move_window,
                constraint,
                initial_window_pos,
                ..
            } => {
                assert_eq!(entity, e);
                assert_eq!((current_pos.x, current_pos.y), (6, 6));
                // 初回 Dragging では prev_pos == current_pos
                assert_eq!((prev_pos.x, prev_pos.y), (6, 6));
                assert!(!move_window, "context 無しでは move_window=false");
                assert!(constraint.is_none());
                assert_eq!((initial_window_pos.x, initial_window_pos.y), (0, 0));
            }
            other => panic!("expected Dragging, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// JustStarted → Dragging 遷移で WindowDragContextResource の hwnd/initial_window_pos/
    /// move_window/constraint が DragState::Dragging に取り込まれる。
    #[test]
    fn test_update_dragging_reads_window_drag_context() {
        force_idle();
        let e = entity(5);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        start_dragging(PhysicalPoint::new(3, 3));

        let ctx_res = WindowDragContextResource::new();
        let constraint = DragConstraint {
            min_x: Some(-10),
            max_x: Some(500),
            min_y: None,
            max_y: None,
        };
        ctx_res.set(WindowDragContext {
            hwnd: Some(null_hwnd()),
            initial_window_pos: Some(Point { x: 100, y: 200 }),
            move_window: true,
            constraint: Some(constraint),
        });

        update_dragging(PhysicalPoint::new(3, 3), Some(&ctx_res));

        match snapshot_drag_state() {
            DragStateSnapshot::Dragging {
                move_window,
                initial_window_pos,
                constraint,
                ..
            } => {
                assert!(move_window, "context の move_window=true が反映されるべき");
                assert_eq!((initial_window_pos.x, initial_window_pos.y), (100, 200));
                let c = constraint.expect("constraint が反映されるべき");
                assert_eq!(c.min_x, Some(-10));
                assert_eq!(c.max_x, Some(500));
            }
            other => panic!("expected Dragging, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// Dragging → Dragging 更新で current_pos が新値、prev_pos が直前の current_pos になる。
    #[test]
    fn test_update_dragging_dragging_updates_prev_pos() {
        force_idle();
        let e = entity(6);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        start_dragging(PhysicalPoint::new(10, 10));
        update_dragging(PhysicalPoint::new(10, 10), None); // → Dragging (current=prev=10,10)
        update_dragging(PhysicalPoint::new(25, 30), None); // → current=25,30 / prev=10,10

        match snapshot_drag_state() {
            DragStateSnapshot::Dragging {
                current_pos,
                prev_pos,
                ..
            } => {
                assert_eq!((current_pos.x, current_pos.y), (25, 30));
                assert_eq!(
                    (prev_pos.x, prev_pos.y),
                    (10, 10),
                    "prev_pos は直前の current_pos を保持するべき"
                );
            }
            other => panic!("expected Dragging, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// update_dragging は Idle/JustEnded など対象外状態では no-op。
    #[test]
    fn test_update_dragging_noop_when_idle() {
        force_idle();
        update_dragging(PhysicalPoint::new(5, 5), None);
        assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
        force_idle();
    }

    // --- end_dragging --------------------------------------------------------

    /// Preparing からの end_dragging は JustEnded(cancelled=false) へ。entity を保持。
    #[test]
    fn test_end_dragging_from_preparing() {
        force_idle();
        let e = entity(7);
        start_preparing(e, PhysicalPoint::new(40, 50), null_hwnd());
        end_dragging(PhysicalPoint::new(41, 52), false);

        match snapshot_drag_state() {
            DragStateSnapshot::JustEnded {
                entity,
                position,
                cancelled,
            } => {
                assert_eq!(entity, e);
                assert_eq!((position.x, position.y), (41, 52));
                assert!(!cancelled);
            }
            other => panic!("expected JustEnded, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// Dragging からの end_dragging（cancelled=true 指定）は JustEnded(cancelled=true) へ。
    #[test]
    fn test_end_dragging_from_dragging_preserves_cancelled_flag() {
        force_idle();
        let e = entity(8);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        start_dragging(PhysicalPoint::new(9, 9));
        update_dragging(PhysicalPoint::new(9, 9), None);
        end_dragging(PhysicalPoint::new(60, 70), true);

        match snapshot_drag_state() {
            DragStateSnapshot::JustEnded {
                entity,
                position,
                cancelled,
            } => {
                assert_eq!(entity, e);
                assert_eq!((position.x, position.y), (60, 70));
                assert!(cancelled, "end_dragging に渡した cancelled=true が反映されるべき");
            }
            other => panic!("expected JustEnded, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// Idle からの end_dragging は no-op（JustEnded を作らない）。
    #[test]
    fn test_end_dragging_noop_when_idle() {
        force_idle();
        end_dragging(PhysicalPoint::new(1, 1), false);
        assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
        force_idle();
    }

    // --- cancel_dragging -----------------------------------------------------

    /// cancel_dragging は常に JustEnded(cancelled=true, position=start_pos) へ。
    #[test]
    fn test_cancel_dragging_uses_start_pos_and_sets_cancelled() {
        force_idle();
        let e = entity(9);
        start_preparing(e, PhysicalPoint::new(33, 44), null_hwnd());
        start_dragging(PhysicalPoint::new(80, 90)); // start_pos は 33,44 のまま
        cancel_dragging();

        match snapshot_drag_state() {
            DragStateSnapshot::JustEnded {
                entity,
                position,
                cancelled,
            } => {
                assert_eq!(entity, e);
                assert!(cancelled);
                assert_eq!(
                    (position.x, position.y),
                    (33, 44),
                    "cancel は終了位置に start_pos を使うべき"
                );
            }
            other => panic!("expected JustEnded, got {}", variant_name(&other)),
        }
        force_idle();
    }

    /// Idle からの cancel_dragging は no-op。
    #[test]
    fn test_cancel_dragging_noop_when_idle() {
        force_idle();
        cancel_dragging();
        assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
        force_idle();
    }

    // --- reset_to_idle -------------------------------------------------------

    /// reset_to_idle は JustEnded のときのみ Idle に戻す。
    #[test]
    fn test_reset_to_idle_only_from_just_ended() {
        force_idle();
        let e = entity(10);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        end_dragging(PhysicalPoint::new(0, 0), false); // → JustEnded
        reset_to_idle();
        assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
        force_idle();
    }

    /// reset_to_idle は Preparing 等 JustEnded 以外では何もしない。
    #[test]
    fn test_reset_to_idle_noop_when_preparing() {
        force_idle();
        let e = entity(11);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        reset_to_idle();
        assert!(
            matches!(snapshot_drag_state(), DragStateSnapshot::Preparing { .. }),
            "Preparing は reset_to_idle で変化しないべき"
        );
        force_idle();
    }

    // --- check_threshold -----------------------------------------------------

    /// Preparing 中、ユークリッド距離の二乗が閾値の二乗以上なら true。
    #[test]
    fn test_check_threshold_true_at_or_beyond_distance() {
        force_idle();
        let e = entity(12);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        // (3,4) は距離 5 = 閾値ちょうど → true（>=）
        assert!(check_threshold(PhysicalPoint::new(3, 4), 5));
        // (10,0) は距離 10 > 5 → true
        assert!(check_threshold(PhysicalPoint::new(10, 0), 5));
        force_idle();
    }

    /// Preparing 中、距離が閾値未満なら false。
    #[test]
    fn test_check_threshold_false_below_distance() {
        force_idle();
        let e = entity(13);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        // (3,3) は距離 sqrt(18)≈4.24 < 5 → false
        assert!(!check_threshold(PhysicalPoint::new(3, 3), 5));
        force_idle();
    }

    /// Preparing 以外の状態では check_threshold は常に false（warn ログのみ）。
    #[test]
    fn test_check_threshold_false_when_not_preparing() {
        force_idle();
        // Idle 状態
        assert!(!check_threshold(PhysicalPoint::new(100, 100), 1));
        force_idle();
    }

    /// 負方向の座標差（start_pos > current_pos）でも dx*dx が正に評価され、
    /// 距離判定が対称に働く（i32 乗算は符号で結果が変わらないことの特性化）。
    #[test]
    fn test_check_threshold_symmetric_for_negative_delta() {
        force_idle();
        let e = entity(20);
        // start_pos を正値にし、current_pos を左上へ動かして dx/dy を負にする
        start_preparing(e, PhysicalPoint::new(100, 100), null_hwnd());
        // (97,96) → dx=-3, dy=-4, 距離 5 = 閾値ちょうど → true（>= かつ符号非依存）
        assert!(check_threshold(PhysicalPoint::new(97, 96), 5));
        // (98,98) → dx=-2, dy=-2, 距離 sqrt(8)≈2.83 < 5 → false
        assert!(!check_threshold(PhysicalPoint::new(98, 98), 5));
        force_idle();
    }

    /// i16 実用座標の極値差（本番入力範囲の上限相当）でも i32 算術が桁あふれせず
    /// 正確に評価される安全鎖の特性化。dx=32767, dy=0 → distance_sq=1_073_676_289
    /// （< i32::MAX=2_147_483_647）で、閾値 5 を上回り true。
    /// （ドキュメントの桁あふれ境界 |dx|>46340 には達しない実用上限を固定）。
    #[test]
    fn test_check_threshold_i16_extent_delta_no_overflow() {
        force_idle();
        let e = entity(21);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        // i16::MAX 相当の水平デルタ。dx*dx は i32 範囲内で桁あふれせず true。
        assert!(check_threshold(PhysicalPoint::new(32767, 0), 5));
        force_idle();
    }

    // --- snapshot ------------------------------------------------------------

    /// DragState::snapshot が各バリアントを対応する DragStateSnapshot バリアントに写像する。
    #[test]
    fn test_snapshot_maps_each_variant() {
        force_idle();
        // Idle
        assert_eq!(variant_name(&snapshot_drag_state()), "Idle");

        let e = entity(14);
        // Preparing
        start_preparing(e, PhysicalPoint::new(1, 2), null_hwnd());
        assert_eq!(variant_name(&snapshot_drag_state()), "Preparing");

        // JustStarted
        start_dragging(PhysicalPoint::new(2, 3));
        assert_eq!(variant_name(&snapshot_drag_state()), "JustStarted");

        // Dragging
        update_dragging(PhysicalPoint::new(2, 3), None);
        assert_eq!(variant_name(&snapshot_drag_state()), "Dragging");

        // JustEnded
        end_dragging(PhysicalPoint::new(5, 5), false);
        assert_eq!(variant_name(&snapshot_drag_state()), "JustEnded");

        force_idle();
    }

    /// read_drag_state クロージャに現在の DragState 参照が渡される。
    #[test]
    fn test_read_drag_state_observes_current_state() {
        force_idle();
        let observed_idle = read_drag_state(|s| matches!(s, DragState::Idle));
        assert!(observed_idle);

        let e = entity(15);
        start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
        let observed_preparing = read_drag_state(|s| matches!(s, DragState::Preparing { .. }));
        assert!(observed_preparing);
        force_idle();
    }
}
