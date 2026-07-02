//! クリック透過機構の UI スレッド側判定ロジック（純関数部）。
//!
//! 本ファイルは差分ガード＋ドラッグ抑止を適用して「今回適用すべき変化」を
//! 返す **World 非依存の純関数** [`resolve_transition`] のみを提供する。
//! UI async ループ本体（`ClickThroughController::start`）は後続タスク（3.1）で
//! 追加する。ここでは副作用・World アクセスを一切持たない純ロジックに限定し、
//! in-source ユニットテストで差分ガード・ドラッグ抑止・`JustEnded` 再収束を網羅する。

use crate::ecs::drag::DragStateSnapshot;
use bevy_ecs::entity::Entity;

use super::DesiredState;

/// 差分ガード＋ドラッグ抑止を適用して「今回適用すべき変化」を返す純関数。
///
/// 適用不要（差分なし or ドラッグ抑止）の場合は `None` を返す。
///
/// # 判定順序（ゲーティング）
/// 1. **ドラッグ最優先ゲート（R5.1/R5.3）**: ドラッグ移動中は透過 ON へ遷移させない。
///    移動中は望ましい状態を強制的に `Opaque` とし、`last_applied != Opaque` の
///    時のみ `Some(Opaque)`（透過を外して掴み維持）、同一なら `None`。
///    - 抑止対象は「ドラッグ移動中」の状態。design（§System Flows「ゲーティング順序」）は
///      抑止スコープを *ドラッグ中* と規定する。本実装では `Dragging`（閾値到達・移動中）に
///      加え、その直前 1 フレームの `JustStarted`（移動が始まった直後・掴み確定）も抑止対象と
///      する。両者はボタン押下＋ドラッグ開始済みの「移動中」フェーズであり、ここで透過 ON に
///      なると掴みが崩れる（R5.1 のアンチフリッカ意図）。一方 `Preparing`（押下のみ・閾値未到達）は
///      まだドラッグ開始前なので非ドラッグ写像に委ねる。
/// 2. **`JustEnded` 再収束（R5.2）**: ドラッグ終了直後は抑止を解除し、現在の `hit` に
///    基づく非ドラッグ写像へ委ねる（終了サイクルで正しい状態へ再収束する）。
/// 3. **非ドラッグ写像（R3.3/R2.1/R2.2）**: `Some(entity)` → `Opaque`（不透過・自窓で受領）、
///    `None` → `Transparent`（透過・背面プロセスへ通過）。
/// 4. **差分ガード（R3.2）**: 望ましい状態が `last_applied` と同一なら `None`（再適用しない）。
///    異なる場合のみ `Some(desired)`（ちょうど一度だけ適用・R3.3）。
///
/// # 純粋性
/// `&World`・I/O・グローバル状態・副作用を持たない決定的関数。`last_applied` の
/// 真実源（[`super::ClickThroughRegistry`]）への書き戻しは呼び出し側（タスク 3.1）の
/// 責務であり、本関数は計算のみを担う。
pub(crate) fn resolve_transition(
    hit: Option<Entity>,
    drag: &DragStateSnapshot,
    last_applied: DesiredState,
) -> Option<DesiredState> {
    // 望ましい状態を判定する。
    let desired = match drag {
        // ドラッグ移動中: 透過 ON へは絶対に遷移させない。強制 Opaque。
        DragStateSnapshot::Dragging { .. } | DragStateSnapshot::JustStarted { .. } => {
            DesiredState::Opaque
        }
        // それ以外（Idle / Preparing / JustEnded）は非ドラッグ写像に委ねる。
        // JustEnded は抑止解除サイクルとして現在の hit に従い再収束する（R5.2）。
        DragStateSnapshot::Idle
        | DragStateSnapshot::Preparing { .. }
        | DragStateSnapshot::JustEnded { .. } => match hit {
            Some(_) => DesiredState::Opaque,
            None => DesiredState::Transparent,
        },
    };

    // 差分ガード（R3.2）: 変化がある時だけ適用対象を返す。
    if desired == last_applied {
        None
    } else {
        Some(desired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Point;
    use crate::ecs::pointer::PhysicalPoint;
    use std::time::Instant;
    use windows::Win32::Foundation::HWND;

    fn entity(id: u32) -> Entity {
        // bevy_ecs 0.18: from_raw_u32 は index から Entity を生成する（None は
        // 予約インデックスのみ）。テスト用ダミーなので unwrap で十分。
        Entity::from_raw_u32(id).expect("valid test entity index")
    }

    fn ppoint() -> PhysicalPoint {
        PhysicalPoint { x: 0, y: 0 }
    }

    /// 閾値到達・移動中の `Dragging` スナップショット。
    fn dragging() -> DragStateSnapshot {
        DragStateSnapshot::Dragging {
            entity: entity(1),
            start_pos: ppoint(),
            current_pos: ppoint(),
            prev_pos: ppoint(),
            start_time: Instant::now(),
            hwnd: HWND::default(),
            initial_window_pos: Point { x: 0, y: 0 },
            move_window: true,
            constraint: None,
        }
    }

    /// ドラッグ終了直後の `JustEnded` スナップショット。
    fn just_ended() -> DragStateSnapshot {
        DragStateSnapshot::JustEnded {
            entity: entity(1),
            position: ppoint(),
            cancelled: false,
        }
    }

    // --- 非ドラッグ写像（R3.3） ---

    #[test]
    fn maps_hit_some_to_opaque_when_last_was_transparent() {
        let out = resolve_transition(
            Some(entity(7)),
            &DragStateSnapshot::Idle,
            DesiredState::Transparent,
        );
        assert_eq!(out, Some(DesiredState::Opaque));
    }

    #[test]
    fn maps_hit_none_to_transparent_when_last_was_opaque() {
        let out = resolve_transition(None, &DragStateSnapshot::Idle, DesiredState::Opaque);
        assert_eq!(out, Some(DesiredState::Transparent));
    }

    // --- 差分ガード（R3.2） ---

    #[test]
    fn diff_guard_hit_some_already_opaque_returns_none() {
        let out = resolve_transition(
            Some(entity(7)),
            &DragStateSnapshot::Idle,
            DesiredState::Opaque,
        );
        assert_eq!(out, None);
    }

    #[test]
    fn diff_guard_hit_none_already_transparent_returns_none() {
        let out = resolve_transition(None, &DragStateSnapshot::Idle, DesiredState::Transparent);
        assert_eq!(out, None);
    }

    // --- ドラッグ抑止（R5.1/R5.3） ---

    #[test]
    fn dragging_never_goes_transparent_even_when_hit_none() {
        // コア R5 アンチフリッカ: 移動中にカーソルがキャラから外れても透過に落とさない。
        let out = resolve_transition(None, &dragging(), DesiredState::Opaque);
        assert_eq!(out, None);
    }

    #[test]
    fn dragging_forces_opaque_when_last_was_transparent() {
        // 移動中は強制 Opaque。透過状態から入ったら不透過へ引き戻す（透過にはしない）。
        let out = resolve_transition(None, &dragging(), DesiredState::Transparent);
        assert_eq!(out, Some(DesiredState::Opaque));
    }

    #[test]
    fn just_started_also_suppresses_transparent() {
        // JustStarted（移動開始直後）も抑止対象。
        let just_started = DragStateSnapshot::JustStarted {
            entity: entity(1),
            start_pos: ppoint(),
            current_pos: ppoint(),
            start_time: Instant::now(),
        };
        assert_eq!(
            resolve_transition(None, &just_started, DesiredState::Opaque),
            None
        );
    }

    // --- JustEnded 再収束（R5.2） ---

    #[test]
    fn just_ended_reconverges_to_transparent_when_hit_none() {
        // 抑止解除後、現在 hit=None なので透過へ再収束する。
        let out = resolve_transition(None, &just_ended(), DesiredState::Opaque);
        assert_eq!(out, Some(DesiredState::Transparent));
    }

    #[test]
    fn just_ended_reconverges_to_opaque_when_hit_some() {
        let out = resolve_transition(
            Some(entity(3)),
            &just_ended(),
            DesiredState::Transparent,
        );
        assert_eq!(out, Some(DesiredState::Opaque));
    }

    // --- Preparing は非ドラッグ写像として振る舞う ---

    #[test]
    fn preparing_behaves_as_non_drag_mapping() {
        let preparing = DragStateSnapshot::Preparing {
            entity: entity(1),
            start_pos: ppoint(),
            start_time: Instant::now(),
        };
        // hit=None → Transparent（押下のみ・閾値未到達なのでまだドラッグではない）。
        assert_eq!(
            resolve_transition(None, &preparing, DesiredState::Opaque),
            Some(DesiredState::Transparent)
        );
        // hit=Some → Opaque。
        assert_eq!(
            resolve_transition(Some(entity(2)), &preparing, DesiredState::Transparent),
            Some(DesiredState::Opaque)
        );
    }
}
