//! 並べ直しから記憶までを通すテスト（areka-P0-char-position-save-on-exit 要件 1.1・1.3・1.6・
//! 1.7・4.1・5.4・design Testing Strategy「並べ直しから記憶まで」9〜12・11b・16）。
//!
//! 窓の一式・記憶の送り口・最小のゴーストは共有の部品
//! `frame_chain_finalize_persist_test_support.rs` から取る。

use super::chain_finalize_persist_test_support::{
    MinimalGhost, PREP_SIZES, PersistStore, SHOWN_SIZES, persist_world, prep_placements,
    shown_sizes, spawn_shown, work_area_snapshot,
};
use super::test_support::size_of;
use super::*;

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
