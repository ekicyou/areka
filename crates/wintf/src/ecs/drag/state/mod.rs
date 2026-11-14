//! ドラッグ状態管理
//!
//! thread_local! + RefCellパターンでwndproc層のドラッグ状態を管理する。
//!
//! 各状態には「どの出来事で入り、どの出来事で出るか」を書く（[`DragState`]）。
//! 解放・中断のあとは `JustEnded` で次の左押下まで休み、`Idle` へは戻らない。

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
/// `Copy`。マウスの捕捉の守りは状態の外の専用の枠（`CAPTURE_GUARD`）が持つ。
/// 写しは [`snapshot_drag_state`] で取る。
///
/// 左ボタンを押している間か（`Preparing`・`JustStarted`・`Dragging`）は
/// variant を並べずに [`DragState::is_button_held`] で聞く。
#[derive(Debug, Clone, Copy)]
pub enum DragState {
    /// 押下の無い状態。
    ///
    /// - 入る: 起動時の初期値。製品コードがここへ戻す遷移は無い。
    /// - 出る: 左押下（[`start_preparing`]）で `Preparing` へ。
    Idle,

    /// 左ボタンを押したが、まだ閾値に達していない状態。
    ///
    /// - 入る: 左押下（[`start_preparing`]・`crates/wintf/src/ecs/window_proc/mouse_click.rs`
    ///   の `handle_button_message`）。このときマウスキャプチャを取得する。
    /// - 出る: 閾値到達（[`start_dragging`]・`mouse_move.rs`）で `JustStarted` へ／
    ///   解放（[`end_dragging`]）・中断（[`cancel_dragging`]）で `JustEnded` へ。
    Preparing {
        /// ドラッグ対象エンティティ
        entity: Entity,
        /// マウス押下位置（物理ピクセル、スクリーン座標）
        start_pos: PhysicalPoint,
        /// 押下時刻
        start_time: Instant,
    },

    /// 閾値に達し、ドラッグ開始をまだ配っていない状態。
    ///
    /// - 入る: 閾値到達（[`start_dragging`]）。
    /// - 出る: 次の tick の `dispatch_drag_events`（`crates/wintf/src/ecs/drag/dispatch.rs`・
    ///   `DragTransition::Started` の腕）が [`update_dragging`] を呼んで `Dragging` へ／
    ///   解放（[`end_dragging`]）・中断（[`cancel_dragging`]）で `JustEnded` へ。
    JustStarted {
        /// ドラッグ対象エンティティ
        entity: Entity,
        /// ドラッグ開始位置
        start_pos: PhysicalPoint,
        /// 現在位置
        current_pos: PhysicalPoint,
        /// 開始時刻
        start_time: Instant,
    },

    /// ドラッグ中（閾値到達済み・ドラッグ開始を配り終えた）の状態。
    ///
    /// - 入る: `JustStarted` からの [`update_dragging`]。
    /// - 出る: `WM_MOUSEMOVE` の [`update_dragging`] では位置を更新して `Dragging` のまま／
    ///   解放（[`end_dragging`]）・中断（[`cancel_dragging`]）で `JustEnded` へ。
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
        /// ドラッグ開始時のウィンドウ位置（ウィンドウ枠を含むスクリーン座標）
        initial_window_pos: Point,
        /// DragConfig.move_window のキャッシュ
        move_window: bool,
        /// DragConstraint のキャッシュ
        constraint: Option<DragConstraint>,
    },

    /// 解放または中断のあと、次の左押下までここで休む状態。
    ///
    /// - 入る: 解放（[`end_dragging`]）または中断（[`cancel_dragging`]）。
    ///   マウスキャプチャは解放済み。
    /// - 出る: 次の左押下（[`start_preparing`]）で `Preparing` へ。それまでここで休む。
    ///   製品コードがこの状態を `Idle` へ戻すことは無い。
    JustEnded {
        /// ドラッグ対象エンティティ
        entity: Entity,
        /// 終了位置
        position: PhysicalPoint,
        /// キャンセルされたか
        cancelled: bool,
    },
}

impl DragState {
    /// 左ボタン（製品で [`start_preparing`] を呼ぶのは左押下のみ）を押している間か。
    /// `Preparing`・`JustStarted`・`Dragging` で真、`Idle`・`JustEnded` で偽。
    /// 「ドラッグ中か」ではない（`Preparing` は閾値未到達だが押している）。
    pub fn is_button_held(&self) -> bool {
        matches!(
            self,
            Self::Preparing { .. } | Self::JustStarted { .. } | Self::Dragging { .. }
        )
    }
}

thread_local! {
    /// グローバルドラッグ状態（単一ドラッグのみ）
    static DRAG_STATE: RefCell<DragState> = const { RefCell::new(DragState::Idle) };
    /// 押している間（Preparing・JustStarted・Dragging）のマウスの捕捉の守り。押していない間は空。
    static CAPTURE_GUARD: RefCell<Option<CaptureGuard>> = const { RefCell::new(None) };
    /// 最後に休みへ移したとき開始を配ったか取り消しだったか（次に休みへ移すときだけ書き換える）
    static LAST_RELEASE_WAS_DRAG: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl DragState {
    /// 最後に休み（`JustEnded`）へ移したとき、その押下が閾値を越えていた（開始を配った）か取り消しだったとき真。
    /// 今の状態によらず返す（解放と次の押下が同じ tick に入っても、直前の解放の答えが読める）。
    /// `JustEnded` の欄は変えない（形を変えると wintf の他のテストのファイルに手が入るため）。
    pub fn last_release_was_drag() -> bool {
        LAST_RELEASE_WAS_DRAG.with(|v| v.get())
    }
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

/// 今のドラッグの状態の写し（`Copy`）。WndProc ハンドラ等で状態のパターンマッチに使用する。
#[inline]
pub fn snapshot_drag_state() -> DragState {
    read_drag_state(|state| *state)
}

/// ドラッグ開始準備（WM_LBUTTONDOWN時）
///
/// `hwnd` を使って `SetCapture` を呼び出し、`CaptureGuard` を生成して `CAPTURE_GUARD` へ入れる。
///
/// 枠に残った守り（テストが状態を直に戻したときだけ残る）を先に落としてから捕捉を取る
/// （後に落とすと新しい捕捉を外してしまう）。落とすのも取るのも、どちらの借用も返してから行う。
/// `ReleaseCapture`・`SetCapture` は `WM_CAPTURECHANGED` を同期で送り、その手続きが
/// ドラッグの状態と守りの枠を借りに来るため。
#[inline]
pub fn start_preparing(entity: Entity, pos: PhysicalPoint, hwnd: HWND) {
    // 既に押している間は無視（複数ボタン同時ドラッグ禁止）
    // JustEndedは許可（前回のドラッグが終了した後の新しいドラッグ）
    if read_drag_state(DragState::is_button_held) {
        tracing::debug!("[drag] Already dragging, ignoring new button press");
        return;
    }

    drop(CAPTURE_GUARD.with_borrow_mut(Option::take));
    let capture_guard = CaptureGuard::acquire(hwnd);
    CAPTURE_GUARD.set(Some(capture_guard));

    update_drag_state(|state| {
        *state = DragState::Preparing {
            entity,
            start_pos: pos,
            start_time: Instant::now(),
        }
    });

    tracing::debug!(
        entity = ?entity,
        x = pos.x,
        y = pos.y,
        hwnd = format!("0x{:X}", hwnd.0 as usize),
        "[start_preparing] DragState -> Preparing (with capture)"
    );
}

/// ドラッグ開始（閾値到達時）
///
/// Preparing → JustStarted 遷移。
#[inline]
pub fn start_dragging(current_pos: PhysicalPoint) {
    update_drag_state(|state| {
        if let DragState::Preparing {
            entity,
            start_pos,
            start_time,
        } = *state
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
            };
        }
    });
}

/// ドラッグ移動（WM_MOUSEMOVE時）
///
/// JustStarted → Dragging 遷移時に WindowDragContextResource から
/// HWND・初期位置・制約情報を読み取り、DragState::Dragging にセットする。
#[inline]
pub fn update_dragging(
    current_pos: PhysicalPoint,
    drag_context: Option<&crate::ecs::drag::WindowDragContextResource>,
) {
    update_drag_state(|state| {
        match *state {
            DragState::JustStarted {
                entity,
                start_pos,
                start_time,
                ..
            } => {
                // WindowDragContextResource から Window 情報を読み取り
                // 窓を動かすのは、控えが動かすと言い、かつ始めの位置があるときだけ
                let (hwnd, initial_window_pos, move_window, constraint) = match drag_context
                    .and_then(|ctx_res| ctx_res.get())
                {
                    Some(ctx) => (
                        ctx.hwnd.unwrap_or_default(),
                        ctx.initial_window_pos.unwrap_or(Point { x: 0, y: 0 }),
                        ctx.move_window && ctx.initial_window_pos.is_some(),
                        ctx.constraint,
                    ),
                    None => {
                        tracing::warn!(?entity, "[update_dragging] 窓の控えが無い: 窓は動かさない");
                        (HWND::default(), Point { x: 0, y: 0 }, false, None)
                    }
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
                };
            }
            DragState::Dragging {
                entity,
                start_pos,
                current_pos: old_pos,
                start_time,
                hwnd,
                initial_window_pos,
                move_window,
                constraint,
                ..
            } => {
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
                };
            }
            _ => {}
        }
    });
}

/// ドラッグを休ませた結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragClose {
    /// 休ませた。`notified` は終了の種を積んだか（前の状態が `JustStarted`／`Dragging` で、
    /// wndproc 側の控えへ渡せたとき真）。
    Closed { entity: Entity, notified: bool },
    /// 離した窓が、捕捉を取った窓と違う（何もしていない）。
    OtherWindow,
    /// 押している状態ではなかった（何もしていない）。
    NotActive,
}

/// 押している状態（`Preparing`・`JustStarted`・`Dragging`）なら、
/// 押した位置・最後の画面の座標を返す。
fn held_parts(state: &DragState) -> Option<(PhysicalPoint, PhysicalPoint)> {
    match *state {
        DragState::Preparing { start_pos, .. } => Some((start_pos, start_pos)),
        DragState::JustStarted {
            start_pos,
            current_pos,
            ..
        }
        | DragState::Dragging {
            start_pos,
            current_pos,
            ..
        } => Some((start_pos, current_pos)),
        DragState::Idle | DragState::JustEnded { .. } => None,
    }
}

/// ドラッグを休ませる 1 か所。
///
/// `pick` が押している状態を見て (終了位置, 取り消しか) を返したときだけ休ませる。
/// 順は「状態を `JustEnded` へ移す → 前の状態が開始済みなら終了の種を積む →
/// `DRAG_STATE` の借用を返す → 枠から捕捉の守りを取り出す（枠の借用も返す）→ 守りを落とす」。
/// 守りの `ReleaseCapture` は `WM_CAPTURECHANGED` を同期で送るので、借用の中で落とすと入れ子の借用になる。
fn close_drag(
    pick: impl FnOnce(&DragState) -> Result<(PhysicalPoint, bool), DragClose>,
) -> DragClose {
    let result = update_drag_state(|state| {
        let (position, cancelled) = match pick(state) {
            Ok(v) => v,
            Err(result) => return result,
        };
        let (entity, started) = match *state {
            DragState::Preparing { entity, .. } => (entity, false),
            DragState::JustStarted { entity, .. } | DragState::Dragging { entity, .. } => {
                (entity, true)
            }
            // pick は押している状態でしか Ok を返さない。届いたら何もしない。
            other => {
                tracing::error!(event = "drag_close_unexpected_state", state = ?other);
                debug_assert!(false, "close_drag: pick が押していない状態で Ok を返した");
                return DragClose::NotActive;
            }
        };
        *state = DragState::JustEnded {
            entity,
            position,
            cancelled,
        };
        LAST_RELEASE_WAS_DRAG.with(|v| v.set(started || cancelled));
        // 開始を配ったドラッグだけが終了の種を積む（閾値前は積まない）
        let notified =
            started && crate::ecs::drag::accumulator::push_ended_seed(entity, position, cancelled);

        tracing::debug!(
            entity = ?entity,
            x = position.x,
            y = position.y,
            cancelled,
            notified,
            "[drag] Dragging ended"
        );
        DragClose::Closed { entity, notified }
    });
    if matches!(result, DragClose::Closed { .. }) {
        // 取り出した守りはこの文の終わりでドロップ → ReleaseCapture がどちらの借用の外でも実行される
        drop(CAPTURE_GUARD.with_borrow_mut(Option::take));
    }
    result
}

/// ドラッグ終了（位置と取り消しの印を指定して休ませる）。
///
/// 戻り値は [`DragClose`]（以前は `()`）。前の状態が `JustStarted`／`Dragging` なら、
/// 呼ぶだけで終了の種（`DragTransition::Ended`）が累積器へ積まれる。呼び出し側でも
/// 積むと二重になる（2 件目は累積器の入口で捨てられる）。
#[inline]
pub fn end_dragging(position: PhysicalPoint, cancelled: bool) -> DragClose {
    close_drag(|state| {
        held_parts(state)
            .map(|_| (position, cancelled))
            .ok_or(DragClose::NotActive)
    })
}

/// ドラッグキャンセル（ESCキー、WM_CANCELMODE、非活性化時）。押した位置・取り消しの印つきで休ませる。
///
/// 戻り値は [`DragClose`]（以前は `()`）。種の規則は [`end_dragging`] と同じで、
/// 呼ぶだけで終了の種が積まれる（呼び出し側でも積むと二重になる）。
#[inline]
pub fn cancel_dragging() -> DragClose {
    close_drag(|state| {
        held_parts(state)
            .map(|(start_pos, _)| (start_pos, true))
            .ok_or(DragClose::NotActive)
    })
}

/// マウスの捕捉を失ったとき（`WM_CAPTURECHANGED`）。
///
/// 捕捉の守りに解放済みの印を付けてから [`cancel_dragging`] と同じことをする
/// （OS が既に解放しているので `ReleaseCapture` は呼ばない）。呼ぶだけで終了の種が積まれる。
#[inline]
pub fn cancel_dragging_on_capture_lost() -> DragClose {
    close_drag(|state| {
        held_parts(state)
            .map(|(start_pos, _)| {
                CAPTURE_GUARD.with_borrow_mut(|guard| {
                    if let Some(guard) = guard {
                        guard.mark_released();
                    }
                });
                (start_pos, true)
            })
            .ok_or(DragClose::NotActive)
    })
}

/// 左ボタンを離したとき（`WM_LBUTTONUP`）。
///
/// `hwnd` が捕捉を取った窓と同じときだけ、取り消しの印なしで休ませる（違えば
/// [`DragClose::OtherWindow`]）。`position` が `None` のときは、状態が持つ最後の画面の座標
/// （`JustStarted`／`Dragging` は current_pos、`Preparing` は start_pos）を使う。
/// 呼ぶだけで終了の種が積まれる（呼び出し側でも積むと二重になる）。
#[inline]
pub fn end_dragging_on_release(hwnd: HWND, position: Option<PhysicalPoint>) -> DragClose {
    close_drag(|state| {
        let (_, last_pos) = held_parts(state).ok_or(DragClose::NotActive)?;
        if CAPTURE_GUARD.with_borrow(|guard| guard.as_ref().map(CaptureGuard::hwnd)) != Some(hwnd) {
            return Err(DragClose::OtherWindow);
        }
        Ok((position.unwrap_or(last_pos), false))
    })
}

/// ドラッグ準備中をDraggingに遷移させるか判定する
///
/// # 整数境界の注意
/// `dx`/`dy` は i32 座標差、`distance_sq = dx*dx + dy*dy` と `threshold_sq = threshold*threshold`
/// はいずれも i32 乗算であり、極値座標差では debug ビルドで桁あふれ panic（`|dx| > 46340` で
/// `dx*dx` が i32::MAX 超過）・release ビルドでラップする理論的経路がある。ただし本番座標は
/// WM lparam 由来の i16 クライアント座標＋ウィンドウ位置オフセット（実モニタ幾何で有界）であり、
/// 実用座標では桁あふれしない。
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
mod tests;
