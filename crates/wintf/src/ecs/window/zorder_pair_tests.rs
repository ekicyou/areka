//! `zorder_pair` の状態定義に対する決定論的テスト（実 HWND・実機不要）。
//!
//! 本ファイルが固定するのは**状態の契約**のみ——宣言・再断行要求・owner 記録・
//! 実行時ストラテジが、素の `World` 上で付与・読取・除去できること、および
//! 既定値が設計の明記どおりであること（design.md「State（コンポーネント契約）」）。
//! 判断ロジック（`decide_pair_fix`）と適用系は後続タスクの担当ゆえ本檻には無い。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;

use super::{ExpectedOrder, KeepDirectlyAbove, OwnerLink, ReassertZOrder, ZOrderPairStrategy};

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// `T: Send + Sync` を型検査で要求するだけのヘルパ。
///
/// bevy の `Component`／`Resource` は `Send + Sync + 'static` を要求するため、
/// HWND を内包する型で手動 `unsafe impl` を落とすとここがコンパイルエラーになる。
fn assert_send_sync<T: Send + Sync + 'static>() {}

// -------------------------------------------------------------------------
// 既定値（design.md「State（コンポーネント契約）」）
// -------------------------------------------------------------------------

/// 実行時ストラテジの既定は案 A（owner 保証）・raise assist 無効。
///
/// 対照: 既定が案 B や raise assist 有効へ書き換わったら落ちるよう、
/// 取り違えうる 2 値との不一致も併せて固定する。
#[test]
fn zorder_pair_strategy_default_is_owner_link_without_raise_assist() {
    let default = ZOrderPairStrategy::default();

    assert_eq!(
        default,
        ZOrderPairStrategy::OwnerLink {
            raise_assist: false
        },
        "既定は案 A（owner 保証）かつ raise assist 無効であること"
    );
    // 対照（取り違えうる隣接値・これらが既定になったら上の assert では気付けない形を潰す）
    assert_ne!(
        default,
        ZOrderPairStrategy::OwnerLink { raise_assist: true },
        "raise assist はゲート G7 FAIL 時のみ有効化される"
    );
    assert_ne!(
        default,
        ZOrderPairStrategy::ExplicitMaintenance,
        "案 B は実機ゲート FAIL 時のフォールバックであり既定ではない"
    );
}

/// ストラテジは Resource として World へ入り、両バリアントが読み書きできる。
#[test]
fn zorder_pair_strategy_round_trips_as_resource() {
    assert_send_sync::<ZOrderPairStrategy>();

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    assert_eq!(
        *world.resource::<ZOrderPairStrategy>(),
        ZOrderPairStrategy::OwnerLink {
            raise_assist: false
        }
    );

    // 案 B へ切り替えても同じ Resource として読める（実行時切替の契約）
    world.insert_resource(ZOrderPairStrategy::ExplicitMaintenance);
    assert_eq!(
        *world.resource::<ZOrderPairStrategy>(),
        ZOrderPairStrategy::ExplicitMaintenance
    );

    // raise assist 有効化（ゲート G7 FAIL 時の形）
    world.insert_resource(ZOrderPairStrategy::OwnerLink { raise_assist: true });
    assert_eq!(
        *world.resource::<ZOrderPairStrategy>(),
        ZOrderPairStrategy::OwnerLink { raise_assist: true }
    );
}

/// 挿入直後の再断行要求は「未適用」段階（`pending_verify: None`）である。
///
/// 対照: 「適用済み・検証待ち」段階を実際に作り、同じフィールドが `Some` を
/// 保持できることを併せて示す（`None` 固定の空虚な主張にしない）。
#[test]
fn reassert_zorder_default_is_not_yet_applied_and_applied_stage_round_trips() {
    let fresh = ReassertZOrder::default();
    assert_eq!(
        fresh.pending_verify, None,
        "挿入直後は未適用（検証待ちではない）"
    );

    let expected = ExpectedOrder {
        above: fake_hwnd(0x1234),
        below: fake_hwnd(0x5678),
    };
    let applied = ReassertZOrder {
        pending_verify: Some(expected),
    };
    assert_eq!(
        applied.pending_verify,
        Some(expected),
        "適用済み段階は期待隣接をそのまま保持する"
    );
    assert_ne!(
        applied, fresh,
        "未適用と適用済み・検証待ちは区別できる（段階が潰れていない）"
    );

    // 期待隣接は「above が below のすぐ手前」の 2 窓であり、順序は入れ替わらない
    assert_eq!(expected.above, fake_hwnd(0x1234));
    assert_eq!(expected.below, fake_hwnd(0x5678));
    assert_ne!(
        expected,
        ExpectedOrder {
            above: fake_hwnd(0x5678),
            below: fake_hwnd(0x1234),
        },
        "上下を入れ替えた期待値は別物として区別される"
    );
}

// -------------------------------------------------------------------------
// 素の World 上での付与・読取・除去
// -------------------------------------------------------------------------

/// ペア宣言は entity 参照だけで付与でき、読取・除去まで往復する。
#[test]
fn keep_directly_above_round_trips_on_bare_world() {
    assert_send_sync::<KeepDirectlyAbove>();

    let mut world = World::new();
    let char_window = world.spawn_empty().id();
    let balloon_window = world.spawn_empty().id();

    world
        .entity_mut(balloon_window)
        .insert(KeepDirectlyAbove { peer: char_window });

    let decl = *world
        .entity(balloon_window)
        .get::<KeepDirectlyAbove>()
        .expect("宣言はバルーン窓側に付く");
    assert_eq!(decl.peer, char_window, "peer はキャラ窓 entity を指す");
    // 対照: 宣言は付けた窓だけに付く（キャラ窓側へは付かない）
    assert!(
        world
            .entity(char_window)
            .get::<KeepDirectlyAbove>()
            .is_none(),
        "宣言は片側（バルーン窓）にのみ付く"
    );

    world
        .entity_mut(balloon_window)
        .remove::<KeepDirectlyAbove>();
    assert!(
        world
            .entity(balloon_window)
            .get::<KeepDirectlyAbove>()
            .is_none(),
        "除去できる（宣言の撤回が可能）"
    );
}

/// 再断行要求は付与→段階更新→除去（消費）まで往復する。
#[test]
fn reassert_zorder_round_trips_on_bare_world() {
    assert_send_sync::<ReassertZOrder>();

    let mut world = World::new();
    let balloon_window = world.spawn_empty().id();

    // ① 挿入（未適用）
    world
        .entity_mut(balloon_window)
        .insert(ReassertZOrder::default());
    assert_eq!(
        world
            .entity(balloon_window)
            .get::<ReassertZOrder>()
            .expect("挿入した要求が読める")
            .pending_verify,
        None
    );

    // ② 適用済み・検証待ちへ進める
    let expected = ExpectedOrder {
        above: fake_hwnd(0xAAAA),
        below: fake_hwnd(0xBBBB),
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
        "適用済み段階が World 上で保持される"
    );

    // ③ 消費（維持系が検証完了で remove する）
    world.entity_mut(balloon_window).remove::<ReassertZOrder>();
    assert!(
        world
            .entity(balloon_window)
            .get::<ReassertZOrder>()
            .is_none(),
        "一回限りの要求ゆえ消費できる"
    );
}

/// owner 確立の記録は HWND を保持したまま往復する。
#[test]
fn owner_link_round_trips_on_bare_world() {
    assert_send_sync::<OwnerLink>();

    let mut world = World::new();
    let balloon_window = world.spawn_empty().id();
    let owner_hwnd = fake_hwnd(0xC0DE);

    world
        .entity_mut(balloon_window)
        .insert(OwnerLink { owner_hwnd });
    assert_eq!(
        world
            .entity(balloon_window)
            .get::<OwnerLink>()
            .expect("確立記録が読める")
            .owner_hwnd,
        owner_hwnd,
        "切離しに使う owner の HWND がそのまま残る"
    );

    // 対照: 記録が無い窓では None（「張った事実」と「張っていない事実」が区別できる）
    let unlinked = world.spawn_empty().id();
    assert!(
        world.entity(unlinked).get::<OwnerLink>().is_none(),
        "owner を張っていない窓には記録が無い"
    );

    world.entity_mut(balloon_window).remove::<OwnerLink>();
    assert!(
        world.entity(balloon_window).get::<OwnerLink>().is_none(),
        "切離し時に記録を除去できる"
    );
}
