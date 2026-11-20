//! 起動の並べ直しが寄せを通してから x と y を書く檻（window-work-area-clamp タスク 2.1・
//! 要件 1.1〜1.5・5.1・5.2・7.5・design Testing Strategy ⑻）。
//!
//! 本体が主の画面の左端に立ち、相方を左へ並べると左隣の画面へ渡る並び（x=−1108 の形）。
//! 並べ直しは相方を本体の右の隣へ並べ替え、本体は動かさない。並べ終えた時点の保存には
//! 寄せた後の位置（下端中央の x）が書かれ、相方のバルーンはずらしどおりに立つ。

use super::finalize_chain_once_with;
use super::test_support::pos_of;
use areka_runtime::frame::persist_test_support::left_edge::{SIZES, placements, shown, world};
use areka_runtime::frame::persist_test_support::{PersistStore, spawn_shown};

use areka_sylphya::{Axis, PersistKey};
use wintf::ecs::window::drain_window_pos_commands;

/// ⑻ 並べ終えた時点の保存に、相方の寄せた後の位置（下端中央の x・上端の y）が書かれる（要件 7.5）。
#[test]
fn save_at_finalize_writes_clamped_partner_position() {
    let mut world = world();
    let store = PersistStore::fake();
    store.wire(&mut world);
    let gw = spawn_shown(&mut world, &placements(), &shown());

    finalize_chain_once_with(&shown(), &mut world);

    let partner = pos_of(&world, gw.char_window(1).unwrap()).unwrap();
    assert_eq!(
        partner.x, SIZES[0].0 as i32,
        "前提: 相方は右の隣へ寄せられた"
    );
    let saved = store.load();
    let get = |axis| {
        saved
            .get(&PersistKey::WindowPos { scope: 1, axis })
            .cloned()
    };
    assert_eq!(
        (get(Axis::X), get(Axis::Y)),
        (
            Some((partner.x + SIZES[1].0 as i32 / 2).to_string()),
            Some(partner.y.to_string())
        ),
        "寄せた後の位置が書かれる: {saved:?}"
    );
    let _residue = drain_window_pos_commands();
}
