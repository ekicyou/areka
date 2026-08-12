//! `zorder_pair` の状態定義と是正判断に対する決定論的テスト（実 HWND・実機不要）。
//!
//! 本ファイルが固定するのは 2 つである。
//!
//! - **状態の契約**——宣言・再断行要求・owner 記録・実行時ストラテジが、素の `World` 上で
//!   付与・読取・除去でき、既定値が設計の明記どおりであること
//!   （design.md「State（コンポーネント契約）」）。
//! - **是正判断の全分岐**——`decide_pair_fix` がトリガ種別・両窓の生存・実測隣接から
//!   何を決めるか（design.md「Testing Strategy > Unit Tests」の 1・2）。
//!
//! 適用系（`SetWindowPos` 発行・検証・ログ）は後続タスクの担当ゆえ本ファイルには無い。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;

use super::{
    ExpectedOrder, InsertSpec, KeepDirectlyAbove, OwnerLink, PairFixDecision, PairObservation,
    PairTrigger, ReassertZOrder, SkipReason, ZOrderPairStrategy, decide_pair_fix,
};

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

// -------------------------------------------------------------------------
// decide_pair_fix - 是正判断の全分岐
// -------------------------------------------------------------------------

/// バルーン窓（手前に居るべき側）。
fn balloon() -> HWND {
    fake_hwnd(0x00B0)
}
/// キャラ窓（背後に居るべき側）。
fn character() -> HWND {
    fake_hwnd(0x00C0)
}
/// キャラ窓の 1 つ手前に割り込んでいる他アプリ窓（是正時の挿入位置になる）。
fn intruder_front() -> HWND {
    fake_hwnd(0x00F1)
}
/// バルーン窓の 1 つ背後に割り込んでいる他アプリ窓（＝ペアが隣接していない証拠）。
fn intruder_back() -> HWND {
    fake_hwnd(0x00F2)
}

const ALL_TRIGGERS: [PairTrigger; 4] = [
    PairTrigger::Establish,
    PairTrigger::Reassert,
    PairTrigger::RaisedAbove,
    PairTrigger::RaisedBelow,
];

const ALL_STRATEGIES: [ZOrderPairStrategy; 3] = [
    ZOrderPairStrategy::OwnerLink {
        raise_assist: false,
    },
    ZOrderPairStrategy::OwnerLink { raise_assist: true },
    ZOrderPairStrategy::ExplicitMaintenance,
];

/// 是正が要る素の観測——両窓が生きていて、間にも手前にも他アプリ窓が割り込んでいる。
///
/// 各テストはここから必要なフィールドだけを差し替える（差分が主張そのものになる）。
fn intruded(trigger: PairTrigger, strategy: ZOrderPairStrategy) -> PairObservation {
    PairObservation {
        trigger,
        strategy,
        above_alive: true,
        below_alive: true,
        above_hwnd: Some(balloon()),
        below_hwnd: Some(character()),
        // バルーンの 1 つ背後はキャラではない＝隣接していない
        measured_below_of_above: Some(intruder_back()),
        // キャラの 1 つ手前も他アプリ窓＝ここがバルーンの挿入先になる
        measured_above_of_below: Some(intruder_front()),
    }
}

/// 判断の入力として与えうる観測の形（ラベルつき）を網羅列挙する。
///
/// 「全分岐」の主張はこの列挙の網羅性に乗る——列挙側に穴があれば
/// [`decide_pair_fix_covers_every_arm_without_ever_yielding_always_on_top`] の
/// 出現確認（各腕が最低 1 回出ること）が落ちる。
fn observation_matrix() -> Vec<(&'static str, PairObservation)> {
    let mut matrix = Vec::new();
    for trigger in ALL_TRIGGERS {
        for strategy in ALL_STRATEGIES {
            let base = intruded(trigger, strategy);

            matrix.push(("割り込みあり", base.clone()));
            matrix.push((
                "キャラが最前面（手前に窓が無い）",
                PairObservation {
                    measured_above_of_below: None,
                    ..base.clone()
                },
            ));
            matrix.push((
                "既に隣接",
                PairObservation {
                    measured_below_of_above: Some(character()),
                    measured_above_of_below: Some(balloon()),
                    ..base.clone()
                },
            ));
            matrix.push((
                "実測 2 値の食い違い（キャラの直前だけがバルーン）",
                PairObservation {
                    measured_above_of_below: Some(balloon()),
                    ..base.clone()
                },
            ));
            matrix.push((
                "バルーン不在",
                PairObservation {
                    above_alive: false,
                    ..base.clone()
                },
            ));
            matrix.push((
                "キャラ不在",
                PairObservation {
                    below_alive: false,
                    ..base.clone()
                },
            ));
            matrix.push((
                "バルーンのハンドル未付与",
                PairObservation {
                    above_hwnd: None,
                    ..base.clone()
                },
            ));
            matrix.push((
                "キャラのハンドル未付与",
                PairObservation {
                    below_hwnd: None,
                    ..base
                },
            ));
        }
    }
    matrix
}

/// 確立・再断行では、バルーンをキャラのすぐ手前へ——挿入位置は「キャラの 1 つ手前の窓」。
///
/// `SetWindowPos` の `hWndInsertAfter` は指定窓の**背後**へ対象を置くため、
/// 「キャラのすぐ手前」を作る挿入位置はキャラ自身ではなくキャラの 1 つ手前の窓である。
/// ここを取り違えると割り込み窓とキャラの間ではなく、キャラの直後へ落ちてしまう。
#[test]
fn decide_pair_fix_places_balloon_directly_above_character_on_establish_and_reassert() {
    for trigger in [PairTrigger::Establish, PairTrigger::Reassert] {
        for strategy in ALL_STRATEGIES {
            assert_eq!(
                decide_pair_fix(&intruded(trigger, strategy)),
                PairFixDecision::PlaceAboveOverBelow {
                    insert_after: InsertSpec::After(intruder_front()),
                },
                "{trigger:?}／{strategy:?}: バルーンはキャラの 1 つ手前の窓の背後へ入る"
            );
            // 対照: キャラ自身を挿入位置にすると「すぐ背後」になってしまう（上下反転）
            assert_ne!(
                decide_pair_fix(&intruded(trigger, strategy)),
                PairFixDecision::PlaceAboveOverBelow {
                    insert_after: InsertSpec::After(character()),
                },
                "{trigger:?}／{strategy:?}: キャラ自身を挿入位置にはしない"
            );
        }
    }
}

/// キャラが最上位で 1 つ手前の窓が無い縁では、挿入位置は最上位（`TopEdge`）になる。
#[test]
fn decide_pair_fix_yields_top_edge_when_character_is_frontmost() {
    let obs = PairObservation {
        measured_above_of_below: None,
        ..intruded(PairTrigger::Reassert, ZOrderPairStrategy::default())
    };

    assert_eq!(
        decide_pair_fix(&obs),
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::TopEdge,
        },
        "キャラより手前に窓が無いなら、バルーンは最上位へ置くほか無い"
    );
    // 対照: 1 つ手前の窓が在れば縁ではなく After になる（TopEdge が既定へ潰れていない）
    assert_eq!(
        decide_pair_fix(&intruded(
            PairTrigger::Reassert,
            ZOrderPairStrategy::default()
        )),
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::After(intruder_front()),
        },
    );
}

/// バルーン側の重なりが動いたら、キャラをバルーンのすぐ背後へ引き上げる（要件 1.3）。
///
/// 処理するのは raise assist 有効時（ゲート G7 FAIL）と案 B のみ。
#[test]
fn decide_pair_fix_places_character_directly_under_balloon_on_raised_above() {
    for strategy in [
        ZOrderPairStrategy::OwnerLink { raise_assist: true },
        ZOrderPairStrategy::ExplicitMaintenance,
    ] {
        assert_eq!(
            decide_pair_fix(&intruded(PairTrigger::RaisedAbove, strategy)),
            PairFixDecision::PlaceBelowUnderAbove {
                insert_after: balloon(),
            },
            "{strategy:?}: キャラはバルーンの背後へ入る＝挿入位置はバルーン自身"
        );
    }
}

/// キャラ側の重なりが動いた場合の扱いはストラテジで分かれる。
///
/// - 案 B: 明示維持ゆえバルーンをキャラのすぐ手前へ（要件 1.2）
/// - 案 A（raise assist 有効）: キャラの浮上には OS が被 owner のバルーンを連れて上がる
///   ため是正は要らない——エコーないし無関係として見送る
#[test]
fn decide_pair_fix_handles_raised_below_per_strategy() {
    assert_eq!(
        decide_pair_fix(&intruded(
            PairTrigger::RaisedBelow,
            ZOrderPairStrategy::ExplicitMaintenance
        )),
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::After(intruder_front()),
        },
        "案 B ではキャラの浮上に追随してバルーンを手前へ戻す"
    );
    assert_eq!(
        decide_pair_fix(&intruded(
            PairTrigger::RaisedBelow,
            ZOrderPairStrategy::OwnerLink { raise_assist: true }
        )),
        PairFixDecision::Skip(SkipReason::EchoOrIrrelevant),
        "案 A ではキャラの浮上は OS がペアごと持ち上げる＝是正は無関係"
    );
}

/// 案 A・raise assist 無効では、重なりの変化を契機とする両トリガが見送られる。
///
/// この構成では z 変化検知そのものを結線しない（design.md「WM_WINDOWPOSCHANGED z 変化検知」）
/// ——万一トリガが届いても判断側で止まることを固定する。
#[test]
fn decide_pair_fix_skips_z_change_triggers_when_raise_assist_is_disabled() {
    let disabled = ZOrderPairStrategy::OwnerLink {
        raise_assist: false,
    };
    for trigger in [PairTrigger::RaisedAbove, PairTrigger::RaisedBelow] {
        assert_eq!(
            decide_pair_fix(&intruded(trigger, disabled)),
            PairFixDecision::Skip(SkipReason::StrategyDisabled),
            "{trigger:?}: raise assist 無効の案 A では見送る"
        );
        // 対照: raise assist を有効にすれば同じ観測が見送りではなくなる
        // （StrategyDisabled が全構成で恒真になっていない）
        assert_ne!(
            decide_pair_fix(&intruded(
                trigger,
                ZOrderPairStrategy::OwnerLink { raise_assist: true }
            )),
            PairFixDecision::Skip(SkipReason::StrategyDisabled),
            "{trigger:?}: raise assist 有効時は見送り理由が変わる"
        );
    }
    // 対照: 確立・再断行は raise assist の有無に関わらず処理される
    assert_eq!(
        decide_pair_fix(&intruded(PairTrigger::Reassert, disabled)),
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::After(intruder_front()),
        },
        "再断行の要求は案 A でも処理する（2.6 シームは両案共通）"
    );
}

/// 片方の窓が存在しないときは理由つきで見送る（要件 1.5）。
///
/// 純関数ゆえ残る窓へ書き込む手段そのものが無いが、返り値が是正でないことを固定する。
#[test]
fn decide_pair_fix_skips_with_peer_missing_when_either_window_is_gone() {
    for trigger in ALL_TRIGGERS {
        for strategy in ALL_STRATEGIES {
            let base = intruded(trigger, strategy);
            for (label, obs) in [
                (
                    "バルーン不在",
                    PairObservation {
                        above_alive: false,
                        ..base.clone()
                    },
                ),
                (
                    "キャラ不在",
                    PairObservation {
                        below_alive: false,
                        ..base.clone()
                    },
                ),
                (
                    "両方不在",
                    PairObservation {
                        above_alive: false,
                        below_alive: false,
                        ..base.clone()
                    },
                ),
            ] {
                assert_eq!(
                    decide_pair_fix(&obs),
                    PairFixDecision::Skip(SkipReason::PeerMissing),
                    "{label}／{trigger:?}／{strategy:?}: 相手が居なければ何もしない"
                );
            }
        }
    }
}

/// 生存していてもハンドルが未付与なら、不在とは別の理由で見送る。
#[test]
fn decide_pair_fix_skips_with_handle_missing_before_any_measurement() {
    let base = intruded(PairTrigger::Establish, ZOrderPairStrategy::default());
    for (label, obs) in [
        (
            "バルーンのハンドル未付与",
            PairObservation {
                above_hwnd: None,
                ..base.clone()
            },
        ),
        (
            "キャラのハンドル未付与",
            PairObservation {
                below_hwnd: None,
                ..base.clone()
            },
        ),
    ] {
        assert_eq!(
            decide_pair_fix(&obs),
            PairFixDecision::Skip(SkipReason::HandleMissing),
            "{label}: 窓は生きているがハンドルがまだ無い（不在とは別の理由）"
        );
    }
    // 対照: 生存フラグが落ちていれば理由は PeerMissing 側になる（2 つの見送りが潰れていない）
    assert_eq!(
        decide_pair_fix(&PairObservation {
            above_alive: false,
            above_hwnd: None,
            ..base
        }),
        PairFixDecision::Skip(SkipReason::PeerMissing),
    );
}

/// 既に隣接しているなら、いかなるトリガ・ストラテジでも必ず何もしない（収束の同値ガード）。
#[test]
fn decide_pair_fix_always_skips_when_already_adjacent() {
    for trigger in ALL_TRIGGERS {
        for strategy in ALL_STRATEGIES {
            let adjacent = PairObservation {
                measured_below_of_above: Some(character()),
                measured_above_of_below: Some(balloon()),
                ..intruded(trigger, strategy)
            };
            assert_eq!(
                decide_pair_fix(&adjacent),
                PairFixDecision::Skip(SkipReason::AlreadyAdjacent),
                "{trigger:?}／{strategy:?}: バルーンの 1 つ背後がキャラなら是正は不要"
            );

            // 実測 2 値が食い違う場合（キャラの直前だけがバルーン）も隣接扱いで止める。
            // ここを是正へ流すと挿入位置がバルーン自身になり、自分を自分の背後へ置く
            // 意味を成さない指令が出る。
            let inconsistent = PairObservation {
                measured_above_of_below: Some(balloon()),
                ..intruded(trigger, strategy)
            };
            assert_eq!(
                decide_pair_fix(&inconsistent),
                PairFixDecision::Skip(SkipReason::AlreadyAdjacent),
                "{trigger:?}／{strategy:?}: 実測が食い違っても自己参照の指令は出さない"
            );
        }
    }
}

/// 全分岐を機械的に列挙し、常時最前面を返す経路が存在しないことを対照込みで確かめる。
///
/// - 要件 4.3／8.1: 判断が返しうる重なり指定は「ある窓の背後」と「最上位」だけであり、
///   `ZOrder::TopMost` 相当（常時最前面）を表す値は返り値の型に**存在しない**。
///   下の `match` は網羅（`_` の受けを置いていない）ため、常時最前面を運べる腕が
///   足されればこのテストはコンパイルできなくなる。
/// - 要件 1.6: 各腕をフィールド省略（`..`）無しで分解しており、座標・寸法のフィールドが
///   足されれば同じくコンパイルできなくなる。
/// - 対照: 「最上位へ置く」（`TopEdge`）は実際に返り得る。常時最前面が返らないという主張が
///   「そもそも何も返らない」による恒真になっていない。
#[test]
fn decide_pair_fix_covers_every_arm_without_ever_yielding_always_on_top() {
    let mut seen_skips: Vec<SkipReason> = Vec::new();
    let mut seen_after = false;
    let mut seen_top_edge = false;
    let mut seen_place_below = false;

    for (label, obs) in observation_matrix() {
        match decide_pair_fix(&obs) {
            PairFixDecision::Skip(reason) => {
                if !seen_skips.contains(&reason) {
                    seen_skips.push(reason);
                }
            }
            PairFixDecision::PlaceAboveOverBelow { insert_after } => match insert_after {
                InsertSpec::After(hwnd) => {
                    seen_after = true;
                    assert_ne!(
                        hwnd,
                        balloon(),
                        "{label}: 動かす窓自身を挿入位置にはしない（自己参照の指令）"
                    );
                }
                InsertSpec::TopEdge => seen_top_edge = true,
            },
            PairFixDecision::PlaceBelowUnderAbove { insert_after } => {
                seen_place_below = true;
                assert_eq!(
                    insert_after,
                    balloon(),
                    "{label}: キャラの挿入位置はバルーン自身に限る"
                );
            }
        }
    }

    // 見送りは 5 種すべてが実際に出る（理由が 1 つに潰れていない＝要件 6.3 の素地）
    for reason in [
        SkipReason::AlreadyAdjacent,
        SkipReason::PeerMissing,
        SkipReason::HandleMissing,
        SkipReason::EchoOrIrrelevant,
        SkipReason::StrategyDisabled,
    ] {
        assert!(
            seen_skips.contains(&reason),
            "見送り理由 {reason:?} が全列挙で一度も出ていない（分岐が到達不能）"
        );
    }
    assert!(seen_after, "「ある窓の背後へ」が一度も出ていない");
    assert!(seen_top_edge, "「最上位へ」（縁）が一度も出ていない");
    assert!(seen_place_below, "「キャラを背後へ」が一度も出ていない");
}

/// 動かす対象は宣言されたペアの 2 窓に限られる（要件 3.4／3.1〜3.3）。
///
/// 是正の腕は 2 つしか無く、それぞれ動かすのはバルーン窓かキャラ窓のどちらか 1 枚である。
/// 挿入位置には他アプリ窓が現れうるが、それは**基準**であって動かす対象ではない
/// ——結果として他スコープ・他アプリの窓どうしの相対順は変わらない。
#[test]
fn decide_pair_fix_moves_only_the_two_declared_pair_windows() {
    let pair = [balloon(), character()];
    let mut moved_balloon = false;
    let mut moved_character = false;

    for (label, obs) in observation_matrix() {
        // 動かす窓は腕の種別だけで決まる（観測の他のどこにも依存しない）
        let moved = match decide_pair_fix(&obs) {
            PairFixDecision::Skip(_) => None,
            PairFixDecision::PlaceAboveOverBelow { insert_after: _ } => obs.above_hwnd,
            PairFixDecision::PlaceBelowUnderAbove { insert_after: _ } => obs.below_hwnd,
        };
        let Some(moved) = moved else {
            continue;
        };
        assert!(
            pair.contains(&moved),
            "{label}: 宣言ペア以外の窓を動かそうとしている（{moved:?}）"
        );
        moved_balloon |= moved == balloon();
        moved_character |= moved == character();
    }

    // 対照: 2 窓とも実際に動く側になり得る（片側しか動かさない実装で通る主張にしない）
    assert!(moved_balloon, "バルーン窓が動く腕が一度も出ていない");
    assert!(moved_character, "キャラ窓が動く腕が一度も出ていない");
}
