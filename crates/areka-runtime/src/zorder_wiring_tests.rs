// =============================================================================
// タグ入口の結線（areka-P0-scope-zorder-pinning 要件 11.2）と、shell 設定由来の基底の起動時結線
// （同 spec 要件 5.1／5.2）の決定論テストのうち、`areka-runtime` の側にあるもの
//
// この task が足すのは判断ではなく**配線**である。受け口も台帳も相の本体も既に在って
// 兄弟のテストが判断を全部押さえているので、ここで測るのは次の 3 点だけになる。
//
// 1. タグを含む台本が受け口へ届くと、指令が**台帳適用の相まで**通ること（到達性・zwi01／02）。
// 2. 相順と相の呼出の字面（zwi06／07）。呼出を丸ごと削っても判断のテストは 1 本も赤くならず
//    （task 4.2／5.1／5.2 で三度実証済み）、相順に至っては「是正が 1 心拍ぶん遅れる」だけなので、
//    実窓を持たない檻には**原理的に**映らない。
// 3. shell 設定由来の基底を据える側（`lib.rs`）の字面（zwi09）。据える行を落としても
//    「設定を書いていないゴースト」と全く同じ挙動になるので、挙動を見る檻には映らない。
//
// 字面の走査は必ず**説明文を落とした本文**へ当てる。素の全文には本ファイルの説明の語も
// 相手ファイルの doc の語も入るので、当てる先を間違えると検査が恒真になる
// （task 3.1 の教訓。各檻の末尾に両方向の対照を置いてある）。
//
// 起動の結線の字面（zwi05）と、設定の値を `main` から結線へ運ぶ行の字面（zwi08）は、
// `areka` の `runtime_tests/` の同じ名前のファイルにある。
// =============================================================================

use bevy_ecs::prelude::*;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use std::sync::mpsc::channel;
use wintf::ecs::window::ZOrderChainPlan;

use super::frame::run_zorder_drain_phase;
use super::source_text_test_support::{code_only, index_of, squeeze};
use super::zorder_cue::{ZOrderCueSink, ZOrderDirective};
use crate::consumer_ledger::test_support::carrier_cue;
use areka_placement::follow::OffsetBase;
use areka_placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use areka_placement::source::GhostTitles;
use areka_placement::spawn::spawn_ghost_windows;
use areka_placement::zorder_group_ledger::ZOrderGroupLedger;

// ---------------------------------------------------------------- 道具立て

/// 文字の cue（台本の大半はこれで、受け口には全部届く）。
fn text_cue(text: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text(text.into()),
        duration: 0.0,
    }
}

/// 1 スコープぶんの合成配置（値は散らしただけで意味を持たない。この相は窓を動かさない）。
fn placement(scope: usize) -> ScopePlacement {
    let base = 100 * (scope as i32 + 1);
    ScopePlacement {
        scope,
        char_pos: PointPx { x: base, y: base },
        char_size: SizePx { w: 200, h: 300 },
        balloon_pos: PointPx {
            x: base + 220,
            y: base,
        },
        balloon_size: SizePx { w: 180, h: 120 },
        balloon_offset: PointPx { x: 220, y: 0 },
        balloon_offset_base: OffsetBase::unpinned(PointPx { x: 220, y: 0 }),
        balloon_limit: false,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 指定したスコープの窓だけを持つ World を組む（`GhostWindows` は Resource として載る）。
fn world_with_scopes(scopes: &[usize]) -> World {
    let mut world = World::new();
    let placements: Vec<ScopePlacement> = scopes.iter().map(|s| placement(*s)).collect();
    let titles = GhostTitles::from_scope_titles(
        scopes
            .iter()
            .map(|s| (*s, format!("scope-{s}")))
            .collect::<Vec<_>>(),
    );
    spawn_ghost_windows(&mut world, &placements, &titles);
    world
}

// ---------------------------------------------------------------------------
// 到達性——タグを含む台本が台帳適用の相まで届く（完了状態）
// ---------------------------------------------------------------------------

/// 台本に混ざった `\![set,zorder,1,0]` が、受け口→受け渡し口→取り出しの相を通って
/// **台帳に載る**（task 6.2 の完了状態）。
///
/// 台本には他人宛の演出も文字も流れるので、同じ受け口へまとめて浴びせる。届いてよいのは
/// 1 本だけであり、他の cue は台帳にも受け口にも痕跡を残さない（要件 11.2）。
#[test]
fn t_zwi01_a_script_tag_reaches_the_ledger_through_the_wired_channel() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);

    // 台本の並び——文字・他人宛のタグ・担当外の選別子・そして自分宛のタグ 1 本。
    sink.emit(text_cue("あひる"));
    sink.emit(carrier_cue("move", &["10", "20"]));
    sink.emit(carrier_cue("set", &["windowstate", "minimize"]));
    sink.emit(carrier_cue("set", &["zorder", "1", "0"]));

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        ledger.groups().len(),
        1,
        "台帳へ載ったグループがちょうど 1 本ではない（届いていないか、担当外まで拾っている）"
    );
    let members: Vec<u32> = ledger.groups()[0]
        .members
        .iter()
        .map(|element| element.scope)
        .collect();
    assert_eq!(
        members,
        vec![1, 1, 0, 0],
        "台帳の要素列が書かれたとおり（手前が scope 1）になっていない"
    );

    // 射影まで通って受け口が出来ていること——ここまでが「相まで届く」の終端である。
    let receiver = world
        .get_resource::<ZOrderChainPlan>()
        .expect("受け口（ZOrderChainPlan）が出来ていない＝射影まで届いていない");
    let chain = receiver
        .chain
        .as_ref()
        .expect("受け口は在るのに鎖が空（台帳から鎖の合成まで届いていない）");
    assert_eq!(
        chain.members.len(),
        4,
        "実在する窓 4 枚が鎖に載っていない: {:?}",
        chain.members
    );
    assert!(
        receiver.dirty,
        "内容が動いた巡に印が立っていない（適用系が鎖を書きに行かない）"
    );
}

/// 逆側——重なりのタグを 1 本も含まない台本では、受け口の Resource すら作られない
/// （要件 11.2「担当が存在しないコマンドは従来どおり良性に読み飛ばされる」）。
///
/// 上の檻だけでは「何を浴びせても 1 本載る」形と区別が付かない。
#[test]
fn t_zwi02_a_script_without_the_tag_leaves_the_mechanism_absent() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);

    sink.emit(text_cue("あひる"));
    sink.emit(carrier_cue("move", &["10", "20"]));
    sink.emit(carrier_cue("set", &["windowstate", "minimize"]));
    sink.emit(carrier_cue("reset", &["windowstate"]));

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert!(
        ledger.groups().is_empty(),
        "担当外のコマンドで台帳が動いた（自己選別が漏れている）"
    );
    assert!(
        world.get_resource::<ZOrderChainPlan>().is_none(),
        "グループが 1 本も無い走行で受け口が作られた（既定＝非強制が構造で成り立っていない）"
    );
}

/// 毎フレームの相は**鎖の適用系より前**に走る（task 3.2 の必須事項）——ただしその順序を
/// 与えるのは `.before` ではなく**段そのものの並び**である（裁定 2026-09-12・要件 2.4）。
///
/// 相の登録先は 1 巡の末尾の段（`FrameFinalize`）から `Update` へ移った（絵の着地を文字と
/// 同じ巡に揃えるため）。段をまたぐ順序指定は書けないので、順序を担うのは 13 段の並びに
/// なる——`Update` は適用系が載る末尾の段より前なので、相が公開した望む鎖は同じ巡のうちに
/// 適用系が読む。
///
/// 後ろに載ると、相が組んだ望む鎖を適用系が読むのが 1 心拍ぶん遅れる。遅れるだけで結果は
/// 同じなので、挙動を見る檻には映らない——ここは 2 点を名指しで押さえる。本番の登録先が
/// `Update` であること（旧い `.before` の形が残っていないこと）・その段が 1 巡で末尾の段
/// より前に回ること。登録先そのものの構造の檻は
/// `frame_schedule_tests.rs`（T-N10）が受け持つ。
#[test]
fn t_zwi06_the_frame_system_is_ordered_before_the_chain_apply() {
    let raw = include_str!("lib.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains("add_systems(Update, frame_system.after(update_typewriters));"),
        "相の登録先が `Update` ではない（末尾の段へ戻すと、相が公開した望む鎖を適用系が読むのは次の巡になる）"
    );
    assert!(
        !squeezed.contains("frame_system.before(apply_zorder_chain)"),
        "旧い段内の順序指定（`.before(apply_zorder_chain)`）が残っている（段をまたぐ順序は指定できない）"
    );

    // その 2 段は 1 巡でこの前後に回る（並びの正本は wintf・`try_tick_world` の実行順）。
    let labels = wintf::ecs::world::tick_diag::SCHEDULE_LABELS;
    let update_at = labels
        .iter()
        .position(|label| *label == "Update")
        .expect("1 巡の並びに `Update` が無い");
    let finalize_at = labels
        .iter()
        .position(|label| *label == "FrameFinalize")
        .expect("1 巡の並びに末尾の段が無い");
    assert!(
        update_at < finalize_at,
        "1 巡の並びで `Update` が末尾の段より後ろに来ている（相順の前提が崩れている・update={update_at}・finalize={finalize_at}）"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("world.insert_non_send(wiring);"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("各サブモジュールが機能の実装を持つ。"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("各サブモジュールが機能の実装を持つ。"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

/// 相の呼出は窓寸 reconcile の**後**・再スナップの**前**に 1 つだけ在る。
///
/// `\![move]` の取り出しは areka-P0-move-canon（design D4）で再スナップの後へ移ったが、
/// 本相の置き場は変えていない（鎖の適用系より前であることは段の並びが担う）。
///
/// 呼出を丸ごと削っても判断の檻は 1 本も赤くならない（task 4.2／5.1／5.2 で三度実証）。
/// ここが唯一その削除を捕まえる。
#[test]
fn t_zwi07_the_frame_calls_the_zorder_drain_after_the_size_reconcile() {
    let raw = include_str!("frame.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains(
            "run_zorder_drain_phase(&wiring.zorder_rx, &mut wiring.zorder_ledger, world);"
        ),
        "取り出しの相の呼出が本文に無い（相順の所有者から呼ばれていない）"
    );
    assert_eq!(
        squeezed.matches("run_zorder_drain_phase(").count(),
        1,
        "取り出しの相の呼出がちょうど 1 つではない（二重に回すと指令が二度適用される）"
    );

    let reconcile_at = index_of(
        &squeezed,
        "reconcile_reported_sizes(&mut wiring.presenter, world);",
    );
    let zorder_at = index_of(&squeezed, "run_zorder_drain_phase(");
    let resnap_at = index_of(&squeezed, "resnap_shell_targets(&wiring.presenter, world);");
    assert!(
        reconcile_at < zorder_at && zorder_at < resnap_at,
        "取り出しの相が窓寸 reconcile の後・再スナップの前に無い（設計の相順と食い違う・reconcile={reconcile_at}・zorder={zorder_at}・resnap={resnap_at}）"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("pub fn frame_system(world: &mut World) {"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("donor パターン: remove→各フェーズ→insert"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("donor パターン: remove→各フェーズ→insert"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

/// 結線は台帳へ基底を据えてから結線状態を World へ載せる（要件 5.1 の適用時点）。
///
/// 順序が逆になると結線状態は既に move 済みで種を蒔けない（コンパイルが通らない）ため、
/// ここが押さえるのは**呼出が 1 つだけ在ること**と**載せるより手前に在ること**である。
/// 呼出を丸ごと削っても、設定を書いていないゴーストと同じ挙動になるだけで誰も気づかない。
#[test]
fn t_zwi09_the_boot_seats_the_descript_base_before_inserting_the_wiring() {
    let raw = include_str!("lib.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert_eq!(
        squeezed
            .matches("wiring.seed_zorder_descript_base(zorder_descript);")
            .count(),
        1,
        "shell 設定由来の基底を据える呼出がちょうど 1 つではない（0 なら設定が効かず、2 なら二度据える）"
    );

    let seed_at = index_of(
        &squeezed,
        "wiring.seed_zorder_descript_base(zorder_descript);",
    );
    let insert_at = index_of(&squeezed, "world.insert_non_send(wiring);");
    assert!(
        seed_at < insert_at,
        "基底を据える段が結線状態を World へ載せるより後ろに在る（seed={seed_at}・insert={insert_at}）"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("zorder_descript: Option<&str>,"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("解釈できない値は理由とともに記録され"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("解釈できない値は理由とともに記録され"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}
