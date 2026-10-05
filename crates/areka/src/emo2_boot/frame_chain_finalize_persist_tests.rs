//! 並べ直しから記憶までを通すテスト（areka-P0-char-position-save-on-exit 要件 1.1・1.3・1.6・
//! 1.7・4.1・5.4・design Testing Strategy「並べ直しから記憶まで」9〜12・11b・16）。
//!
//! 窓の一式・記憶の送り口・最小のゴーストは共有の部品
//! `frame_chain_finalize_persist_test_support.rs` から取る。

use super::chain_finalize_persist_test_support::{
    MinimalGhost, PREP_SIZES, PersistStore, SHOWN_SIZES, persist_world, prep_placements,
    shown_sizes, spawn_shown, work_area_snapshot,
};
use super::test_support::{PerTargetSizes, pos_of, size_of};
use super::*;

use std::collections::HashMap;

use crate::app_exit::close_windows_for_restart;
use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::follow::move_window_to;
use crate::placement::persist::persist_entries;
use crate::placement::resolver::ScopePlacement;
use areka_parsers::charset::DefaultEncoding;
use areka_sylphya::{Axis, PersistKey};
use wintf::ecs::SizeI;
use wintf::ecs::window::drain_window_pos_commands;

/// 部品の確かめ（タスク 3.2）: 起動の準備の関数が返した配置から窓を作り、準備と違う幅の絵を出して
/// 置き直し、送り口を置いた World で並べ直しを回すと、記憶に両方のスコープの位置が届く。
/// 偽の保存先と、`target\` の下の実物のファイルの両方で確かめる。
#[test]
fn support_wired_world_delivers_positions_to_store() {
    for real in [false, true] {
        let ghost = MinimalGhost::plant("support-check");
        let store = if real {
            PersistStore::real(&ghost)
        } else {
            PersistStore::fake()
        };
        let (placements, restored) = crate::restore_merged_placements(
            ghost.root(),
            prep_placements(),
            &work_area_snapshot(),
            DefaultEncoding::Ansi,
        );
        assert!(restored.is_empty(), "前提: 記憶が無い (real={real})");

        let mut world = persist_world();
        store.wire(&mut world);
        let shown = shown_sizes();
        let gw = spawn_shown(&mut world, &placements, &shown);
        for scope in [0, 1] {
            let char = gw.char_window(scope).expect("char 窓がある");
            let (w, h) = SHOWN_SIZES[scope];
            assert_ne!(
                w as i32, PREP_SIZES[scope].w,
                "前提: 準備と出ている幅が違う"
            );
            assert_eq!(
                size_of(&world, char),
                Some(SizeI::new(w as i32, h as i32)),
                "置き直しで出ている絵の大きさになる (real={real}, scope={scope})"
            );
        }

        finalize_chain_once_with(&shown, &mut world);
        let saved = store.load();
        for scope in [0, 1] {
            for axis in [Axis::X, Axis::Y] {
                assert!(
                    saved.contains_key(&PersistKey::WindowPos { scope, axis }),
                    "スコープ {scope} の {axis:?} が記憶に届く (real={real}): {saved:?}"
                );
            }
        }
        store.finish();
        if real {
            // 実物のファイルは、起動の準備の関数が読む置き場と同じ（再起動のテストの前提）。
            let (_, restored) = crate::restore_merged_placements(
                ghost.root(),
                prep_placements(),
                &work_area_snapshot(),
                DefaultEncoding::Ansi,
            );
            assert_eq!(
                restored.into_iter().collect::<Vec<_>>(),
                vec![0, 1],
                "書いた記憶を起動の準備の関数が読み直せる"
            );
        }
        // 窓へ届ける指令は捨てる（実 SetWindowPos を呼ばない）。
        let _residue = drain_window_pos_commands();
    }
}

// ── 並べ直しから記憶まで（design Testing Strategy 9〜12・11b・16） ─────────────────────

/// 記憶にないはずの値（書かれたら必ず変わる目印）。
const SENTINEL: &str = "-7777";

/// 起動の準備の関数を通した配置（記憶なし）で窓を作り、出ている絵の大きさで置き直した World。
/// 記憶の送り口は `store` のもの。
fn booted(store: &PersistStore, shown: &PerTargetSizes) -> (World, GhostWindows, MinimalGhost) {
    let ghost = MinimalGhost::plant("persist");
    let mut world = persist_world();
    store.wire(&mut world);
    let gw = spawn_shown(&mut world, &unremembered_placements(&ghost), shown);
    (world, gw, ghost)
}

/// 起動の準備の関数（`crate::restore_merged_placements`）が返した配置。記憶が無いことを確かめる。
fn unremembered_placements(ghost: &MinimalGhost) -> Vec<ScopePlacement> {
    let (placements, restored) = crate::restore_merged_placements(
        ghost.root(),
        prep_placements(),
        &work_area_snapshot(),
        DefaultEncoding::Ansi,
    );
    assert!(restored.is_empty(), "前提: 記憶が無い");
    placements
}

/// スコープの位置の記憶（x・y）。
fn saved_pos(saved: &HashMap<PersistKey, String>, scope: u32) -> (Option<&str>, Option<&str>) {
    let get = |axis| {
        saved
            .get(&PersistKey::WindowPos { scope, axis })
            .map(String::as_str)
    };
    (get(Axis::X), get(Axis::Y))
}

/// 窓の今の位置と出ている絵の幅から、記憶に書かれるべき値（下端の中央の x・上端の y）を出す。
fn expected_saved(world: &World, gw: &GhostWindows, scope: usize) -> (String, String) {
    let pos = pos_of(world, gw.char_window(scope).unwrap()).expect("位置がある");
    let w = SHOWN_SIZES[scope].0 as i32;
    ((pos.x + w / 2).to_string(), pos.y.to_string())
}

/// 両方のスコープが、窓の今の位置で書かれていること。
fn assert_both_written(saved: &HashMap<PersistKey, String>, world: &World, gw: &GhostWindows) {
    for scope in [0, 1] {
        let (x, y) = expected_saved(world, gw, scope);
        assert_eq!(
            saved_pos(saved, scope as u32),
            (Some(x.as_str()), Some(y.as_str())),
            "スコープ {scope} は窓の今の位置で書かれる: {saved:?}"
        );
    }
}

/// 位置の記憶を目印で上書きする（本番の送り口 `persist_entries` を通す）。
fn seed_sentinels(world: &World, scopes: &[u32]) {
    let entries = scopes
        .iter()
        .flat_map(|&scope| {
            [Axis::X, Axis::Y]
                .map(|axis| (PersistKey::WindowPos { scope, axis }, SENTINEL.to_string()))
        })
        .collect();
    persist_entries(world, entries);
}

/// テスト 9（要件 1.1）: 記憶なしの 2 スコープで並べ直しを回すと、両方の位置が書かれ、相方の値は
/// 並べ直した後の位置（下端の中央の x）。
#[test]
fn finalize_writes_both_scopes_with_partner_after_rearrangement() {
    let store = PersistStore::fake();
    let shown = shown_sizes();
    let (mut world, gw, _ghost) = booted(&store, &shown);
    let char1 = gw.char_window(1).unwrap();
    let before = pos_of(&world, char1).expect("相方の位置");

    finalize_chain_once_with(&shown, &mut world);

    let body = pos_of(&world, gw.char_window(0).unwrap()).expect("本体の位置");
    let partner = pos_of(&world, char1).expect("相方の位置");
    assert_ne!(before.x, partner.x, "前提: 相方は並べ直しで動く");
    assert_eq!(
        partner.x + SHOWN_SIZES[1].0 as i32,
        body.x,
        "前提: 相方は本体の左に隙間なく並ぶ"
    );
    assert_both_written(&store.load(), &world, &gw);
    let _residue = drain_window_pos_commands();
}

/// テスト 10: 同じ World でもう一度回しても書かない（1 度だけ）。1 回目の後に記憶を目印で
/// 上書きしておき、2 回目が書けば目印が消えることで見分ける。
#[test]
fn finalize_writes_only_once_per_window_set() {
    let store = PersistStore::fake();
    let shown = shown_sizes();
    let (mut world, _gw, _ghost) = booted(&store, &shown);
    finalize_chain_once_with(&shown, &mut world);
    seed_sentinels(&world, &[0, 1]);
    let before = store.load();
    assert_eq!(
        saved_pos(&before, 1),
        (Some(SENTINEL), Some(SENTINEL)),
        "前提: 記憶に目印が入っている"
    );

    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(store.load(), before, "2 度目は記憶を変えない");
    let _residue = drain_window_pos_commands();
}

/// テスト 11（要件 1.6）: 相方の絵が出ていないまま何フレーム回しても、並べ終えないので記憶は
/// 空のまま。最後に相方の絵を出すと書かれる（送り口がつながっているので、空には意味がある）。
#[test]
fn nothing_is_written_until_partner_is_shown() {
    let store = PersistStore::fake();
    let partner_hidden = PerTargetSizes::new([(0, Some(SHOWN_SIZES[0])), (1, None)]);
    let (mut world, gw, _ghost) = booted(&store, &partner_hidden);

    // 毎フレームの処理と同じく、置き直し → 並べ直しの順で回す。
    for _ in 0..30 {
        resnap_with(&partner_hidden, &mut world);
        finalize_chain_once_with(&partner_hidden, &mut world);
    }
    assert!(
        !world.contains_resource::<ChainFinalized>(),
        "前提: 並べ終えていない"
    );
    assert!(store.load().is_empty(), "並べ終える前は何も書かない");

    let shown = shown_sizes();
    resnap_with(&shown, &mut world);
    finalize_chain_once_with(&shown, &mut world);
    assert_both_written(&store.load(), &world, &gw);
    let _residue = drain_window_pos_commands();
}

/// テスト 11b（要件 1.7）: 並べ終える前に、記憶に位置が無い相方を SHIORI の移動の指示と同じ口
/// （`move_window_to`）で動かすと、相方は書かれず本体だけ書かれる。相方の記憶には目印を
/// 入れておき、書けば目印が消えることで見分ける。
#[test]
fn partner_moved_by_shiori_before_finalize_is_not_written() {
    let store = PersistStore::fake();
    let shown = shown_sizes();
    let (mut world, gw, _ghost) = booted(&store, &shown);
    seed_sentinels(&world, &[1]);
    let before = store.load();
    assert_eq!(
        saved_pos(&before, 1),
        (Some(SENTINEL), Some(SENTINEL)),
        "前提: 相方の記憶に目印が入っている"
    );
    let char1 = gw.char_window(1).unwrap();
    let start = pos_of(&world, char1).expect("相方の位置");
    assert!(
        move_window_to(&mut world, char1, start.x - 77, start.y - 33),
        "前提: 移動の指示が窓へ届く"
    );
    let moved = pos_of(&world, char1);

    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(
        pos_of(&world, char1),
        moved,
        "前提: 並べ直しは動かされた相方を動かさない"
    );
    let saved = store.load();
    assert_eq!(
        saved_pos(&saved, 1),
        (Some(SENTINEL), Some(SENTINEL)),
        "相方は書かない"
    );
    let (x, y) = expected_saved(&world, &gw, 0);
    assert_eq!(
        saved_pos(&saved, 0),
        (Some(x.as_str()), Some(y.as_str())),
        "本体は書かれる: {saved:?}"
    );
    let _residue = drain_window_pos_commands();
}

/// テスト 12（要件 1.3）: ゴーストの切り替えの閉じ方（`close_windows_for_restart`）で閉じ、記憶の
/// 送り口を別の保存先へ差し替えて窓を作り直すと、並べ直しがもう 1 度書き、書かれるのは
/// 差し替えた後の保存先だけ。前の保存先には目印を入れておき、書けば目印が消えることで見分ける。
#[test]
fn restart_writes_once_more_to_the_new_store_only() {
    let first = PersistStore::fake();
    let shown = shown_sizes();
    let (mut world, _gw, ghost) = booted(&first, &shown);
    finalize_chain_once_with(&shown, &mut world);
    seed_sentinels(&world, &[0, 1]);
    let first_before = first.load();
    assert_eq!(
        saved_pos(&first_before, 0),
        (Some(SENTINEL), Some(SENTINEL)),
        "前提: 前の保存先に目印が入っている"
    );

    let _ = close_windows_for_restart(&mut world);
    let second = PersistStore::fake();
    second.wire(&mut world);
    let gw2 = spawn_shown(&mut world, &unremembered_placements(&ghost), &shown);
    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(first.load(), first_before, "前の保存先は変わらない");
    assert_both_written(&second.load(), &world, &gw2);
    let _residue = drain_window_pos_commands();
}

/// テスト 16（要件 4.1・5.4）: 書き込みを失敗させても印 `ChainFinalized` は置かれ、記憶は前の
/// まま、次のフレームの処理（置き直し → 並べ直し）が続く。最後に窓を作り直して並べ終え、
/// 書き込みが届くことで、失敗のあとも書き手が止まっていないことを確かめる。
///
/// 「次のフレーム」は `resnap_with` → `finalize_chain_once_with` の 2 段で回す。毎フレームの処理
/// `emo2_frame_system` は本物の表示側（`Emo2Wiring`）が無いと最初で戻るのでテストから回せず、
/// その中の置き直しと並べ直しはこの 2 つへ委ねている（`resnap_shell_targets`・`finalize_chain_once`）。
#[test]
fn failed_write_keeps_marker_and_next_frame_continues() {
    let store = PersistStore::fake();
    let shown = shown_sizes();
    let (mut world, gw, ghost) = booted(&store, &shown);
    let offset_key = PersistKey::BalloonOffset {
        scope: 0,
        axis: Axis::X,
    };
    persist_entries(&world, vec![(offset_key, "12".to_string())]);
    let before = store.load();
    assert_eq!(before.len(), 1, "前提: 記憶に中身がある: {before:?}");

    store.fail_next_commit();
    finalize_chain_once_with(&shown, &mut world);

    assert!(
        world.contains_resource::<ChainFinalized>(),
        "書き込みが失敗しても印は置かれる"
    );
    assert_eq!(store.load(), before, "失敗した書き込みは記憶を変えない");

    // 次のフレーム: 本体の絵の大きさが変わる → 置き直しが窓へ届き、並べ直しは走らない。
    let grown = (SHOWN_SIZES[0].0 + 14, SHOWN_SIZES[0].1);
    let next = PerTargetSizes::new([(0, Some(grown)), (1, Some(SHOWN_SIZES[1]))]);
    resnap_with(&next, &mut world);
    finalize_chain_once_with(&next, &mut world);
    assert_eq!(
        size_of(&world, gw.char_window(0).unwrap()),
        Some(SizeI::new(grown.0 as i32, grown.1 as i32)),
        "次のフレームの置き直しが窓へ届く"
    );
    assert_eq!(store.load(), before, "印のあとのフレームは書かない");

    // 窓を作り直して並べ終えると、書き込みが届く（書き手は止まっていない）。
    let _ = close_windows_for_restart(&mut world);
    let gw2 = spawn_shown(&mut world, &unremembered_placements(&ghost), &shown);
    finalize_chain_once_with(&shown, &mut world);
    let saved = store.load();
    assert_both_written(&saved, &world, &gw2);
    assert_eq!(
        saved.get(&offset_key).map(String::as_str),
        Some("12"),
        "ほかの記憶は残る"
    );
    let _residue = drain_window_pos_commands();
}
