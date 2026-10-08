//! 再起動して前回の並びに立つことを決定論のテストで固定する（areka-P0-char-position-save-on-exit
//! タスク 4.1・要件 1.2・2.1・2.2・2.3・5.1・5.2・design Testing Strategy「並べ直しから記憶まで」
//! 13・14）。
//!
//! 窓の一式・記憶の送り口・最小のゴーストは共有の部品
//! `frame_chain_finalize_persist_test_support.rs` から取る。
//!
//! 1 回目も 2 回目も、窓は準備のときの幅（`PREP_SIZES`）で作り、それと違う幅の絵
//! （`SHOWN_SIZES`）を出して置き直し（再スナップ）を回してから並べ直しを回す。同じ幅のまま
//! 組むと、書くとき・戻すとき・中央を保って付け替えるときの計算を通らずに緑になる。

use super::chain_finalize_persist_test_support::{
    MinimalGhost, PersistStore, persist_world, prep_placements, shown_sizes, spawn_shown,
    work_area_snapshot,
};
use super::test_support::{PerTargetSizes, pos_of};
use super::*;

use std::time::Instant;

use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::follow::on_char_drag_end;
use areka_parsers::charset::DefaultEncoding;
use wintf::ecs::Point;
use wintf::ecs::drag::{DragEndEvent, DraggingState};
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::drain_window_pos_commands;

/// 本体をドラッグで動かす横の量（右へ・作業領域の中に収まる）。
const DRAG_DX: i32 = 300;

/// 1 回目の World: 起動の準備の本番の関数を通した配置（記憶なし）で窓を作り、置き直してから
/// 並べ終える。`drag` なら、続けて本体（スコープ 0）だけをドラッグの確定の本番の関数で動かす。
/// 返すのは 1 回目を終えた時点の本体・相方の窓の位置。
///
/// 終わりに閉じる処理（`close_windows_for_restart`・終了の手順）は何も呼ばずに World を捨てる
/// （きれいに終わらなかった回・要件 2.2）。記憶の書き手（sylphya のアクター）は UI と別の
/// スレッドで、送った書き込みは待たずに流れる（fire-and-forget）。強制終了の時点より前に
/// 送った書き込みは、その間に書き手が確定している。`barrier` はその「書き手が追いつくだけの
/// 時間が経った」ことを決定論で置き換えるもので、UI の側の閉じる処理の代わりではない。
fn first_run(ghost: &MinimalGhost, drag: bool) -> [Point; 2] {
    let store = PersistStore::real(ghost);
    let (placements, restored) = crate::restore_merged_placements(
        ghost.root(),
        prep_placements(),
        &work_area_snapshot(),
        DefaultEncoding::Ansi,
    );
    assert!(restored.is_empty(), "前提: 1 回目は記憶が無い");

    let mut world = persist_world();
    store.wire(&mut world);
    let shown = shown_sizes();
    let gw = spawn_shown(&mut world, &placements, &shown);
    let partner_before = pos_of(&world, gw.char_window(1).unwrap()).expect("相方の位置");
    finalize_chain_once_with(&shown, &mut world);
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提: 1 回目を並べ終える"
    );
    let body = gw.char_window(0).unwrap();
    let partner = pos_of(&world, gw.char_window(1).unwrap()).expect("相方の位置");
    assert_ne!(
        partner_before.x, partner.x,
        "前提: 1 回目の並べ直しで相方が動く（相方の位置は並べ終えた時点の書き込みにしか無い）"
    );

    if drag {
        let start = pos_of(&world, body).expect("本体の位置");
        let cursor = (start.x + 50, start.y + 50);
        world.entity_mut(body).insert(DraggingState {
            drag_start_pos: Point::new(cursor.0, cursor.1),
            initial_inset: (start.x as f32, start.y as f32),
        });
        let ev = Phase::Bubble(DragEndEvent {
            target: body,
            position: Point::new(cursor.0 + DRAG_DX, cursor.1),
            cancelled: false,
            is_primary: true,
            timestamp: Instant::now(),
        });
        assert!(!on_char_drag_end(&mut world, body, body, &ev));
        assert_eq!(
            pos_of(&world, body),
            Some(Point::new(start.x + DRAG_DX, start.y)),
            "前提: ドラッグの確定で本体が動く"
        );
    }
    let ends = [
        pos_of(&world, body).expect("本体の位置"),
        pos_of(&world, gw.char_window(1).unwrap()).expect("相方の位置"),
    ];
    // 書き手が送られた分を確定し終えるのを待つ（上の説明）。World は閉じずに捨てる。
    let _ = store.load();
    drop(world);
    store.finish();
    let _residue = drain_window_pos_commands();
    ends
}

/// 2 回目の World: 起動の準備の本番の関数で記憶を読み直し、本番（`ghost_session.rs` の窓を作る
/// 閉包）と同じ順で、窓を作る → 返った集合のスコープの既定の位置を消す → 絵が出て置き直す →
/// 並べ直しを回す。窓を作る閉包そのものはテストから呼べないので、同じ `clear_default_char_pos`
/// を同じ集合に対して呼ぶ。返すのは 2 回目を並べ終えた時点の本体・相方の窓の位置。
///
/// 並べ直しの移動が 0 件であること（窓への指令が出ない・位置が変わらない）と、並べ直しの前後で
/// 記憶が同じこと（要件 1.2）もここで確かめる。
fn second_run(ghost: &MinimalGhost) -> [Point; 2] {
    let store = PersistStore::real(ghost);
    let (placements, restored) = crate::restore_merged_placements(
        ghost.root(),
        prep_placements(),
        &work_area_snapshot(),
        DefaultEncoding::Ansi,
    );
    assert_eq!(
        restored.iter().copied().collect::<Vec<_>>(),
        vec![0, 1],
        "1 回目に並べ終えた時点で両方の位置が記憶にある"
    );

    let mut world = persist_world();
    store.wire(&mut world);
    // 窓を作るだけ（絵はまだ出ていない＝置き直さない）。
    let not_shown = PerTargetSizes::new([(0, None), (1, None)]);
    let gw = spawn_shown(&mut world, &placements, &not_shown);
    {
        let mut ghost_windows = world.resource_mut::<GhostWindows>();
        for scope in &restored {
            assert!(ghost_windows.clear_default_char_pos(*scope));
        }
    }
    let shown = shown_sizes();
    resnap_with(&shown, &mut world);

    let windows = [gw.char_window(0).unwrap(), gw.char_window(1).unwrap()];
    let before = windows.map(|e| pos_of(&world, e).expect("窓の位置"));
    let saved_before = store.load();
    let _resnap_commands = drain_window_pos_commands();

    finalize_chain_once_with(&shown, &mut world);

    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提: 2 回目を並べ終える"
    );
    let moves = drain_window_pos_commands();
    assert!(
        moves.is_empty(),
        "2 回目の並べ直しは窓を動かさない: {} 件",
        moves.len()
    );
    let after = windows.map(|e| pos_of(&world, e).expect("窓の位置"));
    assert_eq!(after, before, "2 回目の並べ直しの前後で位置が同じ");
    assert_eq!(
        store.load(),
        saved_before,
        "2 回目の並べ直しの前後で記憶が同じ"
    );
    store.finish();
    after
}

/// テスト 13（要件 2.1・2.2・1.2・5.1）: 1 回目を並べ終える → 本体だけドラッグ → 閉じずに捨てる
/// → 2 回目は本体が動かした位置、相方が 1 回目の位置に立ち、並べ直しの移動は 0 件。
#[test]
fn restart_after_body_drag_keeps_partner_at_previous_position() {
    let ghost = MinimalGhost::plant("restart-drag");
    let [body1, partner1] = first_run(&ghost, true);
    let [body2, partner2] = second_run(&ghost);
    assert_eq!(body2, body1, "本体はドラッグで動かした位置に立つ");
    assert_eq!(
        partner2, partner1,
        "相方は 1 回目の位置に立つ（並べ直されない）"
    );
}

/// テスト 14（要件 2.3・5.2）: ドラッグせずに同じことをすると、両方が 1 回目の位置に立つ。
#[test]
fn restart_without_drag_keeps_both_at_previous_positions() {
    let ghost = MinimalGhost::plant("restart-nodrag");
    let first = first_run(&ghost, false);
    assert_eq!(second_run(&ghost), first, "両方が 1 回目の位置に立つ");
}
