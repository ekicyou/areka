//! 別クレート（areka）から wintf のペア宣言・再断行要求へ到達できることの檻。
//!
//! 要件 1.1／2.6／3.4 の宣言は wintf 側に住むが、**付けるのは areka（scope を知る
//! 唯一の層）**であり、要件 2.6 の再断行要求は `areka-P0-balloon-visibility` が
//! 再表示時に挿入する契約点でもある（requirements.md「Boundary Context」の相互登記）。
//! ゆえに「wintf の公開エクスポート経由で別クレートから付けられる」ことが契約であり、
//! wintf 内部のテストでは再エクスポート漏れを検出できない。本檻はその到達性のみを固定する
//! （実際の spawn 時付与は本フィーチャーの後続タスクが担当）。
//!
//! import は必ず `wintf::ecs::...`（公開エクスポート面）から行う——
//! `wintf::ecs::window::...` を直接指すと再エクスポート漏れを見逃す。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;
use wintf::ecs::{ExpectedOrder, KeepDirectlyAbove, ReassertZOrder, ZOrderPairStrategy};

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// areka 側から宣言と再断行要求を実体へ付け、各状態を読み書きできる。
#[test]
fn areka_can_attach_and_read_back_zorder_pair_state_through_public_exports() {
    let mut world = World::new();
    let char_window = world.spawn_empty().id();
    let balloon_window = world.spawn_empty().id();

    // ① 永続宣言（バルーン窓はキャラ窓のすぐ手前に居るべき・要件 1.1/3.4）
    world
        .entity_mut(balloon_window)
        .insert(KeepDirectlyAbove { peer: char_window });
    assert_eq!(
        world
            .entity(balloon_window)
            .get::<KeepDirectlyAbove>()
            .expect("areka から付けた宣言が読める")
            .peer,
        char_window,
        "peer は同一スコープのキャラ窓 entity"
    );
    // 対照: 付けていない窓には宣言が無い（付与が全窓へ漏れていない）
    assert!(
        world
            .entity(char_window)
            .get::<KeepDirectlyAbove>()
            .is_none(),
        "宣言はバルーン窓側にのみ付く"
    );

    // ② 再断行要求（要件 2.6 のシーム＝再表示時に vis 側が入れる契約点）
    world
        .entity_mut(balloon_window)
        .insert(ReassertZOrder::default());
    assert_eq!(
        world
            .entity(balloon_window)
            .get::<ReassertZOrder>()
            .expect("areka から付けた再断行要求が読める")
            .pending_verify,
        None,
        "挿入直後は未適用"
    );

    // ③ 適用済み・検証待ち段階も別クレートから組み立てられる
    let expected = ExpectedOrder {
        above: fake_hwnd(0x1111),
        below: fake_hwnd(0x2222),
    };
    world
        .entity_mut(balloon_window)
        .get_mut::<ReassertZOrder>()
        .expect("要求は生きている")
        .pending_verify = Some(expected);
    assert_eq!(
        world
            .entity(balloon_window)
            .get::<ReassertZOrder>()
            .expect("段階更新後も読める")
            .pending_verify,
        Some(expected),
        "適用済み・検証待ちの期待隣接が読み書きできる"
    );

    // ④ 消費（一回限りの要求）
    world.entity_mut(balloon_window).remove::<ReassertZOrder>();
    assert!(
        world
            .entity(balloon_window)
            .get::<ReassertZOrder>()
            .is_none(),
        "要求は除去できる"
    );
}

/// 実行時ストラテジも areka（main）から Resource として挿入できる。
#[test]
fn areka_can_insert_zorder_pair_strategy_resource() {
    let mut world = World::new();

    world.insert_resource(ZOrderPairStrategy::default());
    assert_eq!(
        *world.resource::<ZOrderPairStrategy>(),
        ZOrderPairStrategy::OwnerLink {
            raise_assist: false
        },
        "areka main が挿入する既定は案 A・raise assist 無効"
    );

    // 対照: 案 B を明示挿入すれば切り替わる（既定が固定値で焼き付いていない）
    world.insert_resource(ZOrderPairStrategy::ExplicitMaintenance);
    assert_eq!(
        *world.resource::<ZOrderPairStrategy>(),
        ZOrderPairStrategy::ExplicitMaintenance
    );
}
