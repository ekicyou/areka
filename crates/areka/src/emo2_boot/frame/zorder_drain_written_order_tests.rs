// =============================================================================
// zorder drain 相：書かれた順の受理の行・起動の段・解除と読み直しの決定論テスト
// （areka-P0-zorder-scope-inversion task 1.2・要件 1.3／1.4／1.5／2.3／4.5／6.5／7.4）
//
// この相が逆転・非隣接について負うのは 4 つである——「受理の行が並びの形を語ること」
// 「起動の段（descript）がタグと同じ解釈と同じ行を通ること」「受理と受け口への公開が
// 同じ相の 1 回で済むこと」「解除と読み直しで計画が組の並びへ戻ること」。
//
// 行の照合は**逐語**で行う（要件 7.4）。既存の読み手（完了 spec の実機の手順・既存の檻）は
// `[zorder-group] applied`・`false`・`+bN` の字面を読んでいるので、部分一致で丸めると
// 綴りが静かに割れた日に気づけない。
// =============================================================================

use super::test_support::*;
use super::*;

use crate::emo2_boot::frame::zorder_descript::{apply_descript_base, apply_descript_rebase};

/// 1 回の相で `tokens` を受理させ、受理の行（ちょうど 1 本）と World を返す。
fn accept_once(tokens: &[&str], scopes: &[usize]) -> (String, World, ZOrderGroupLedger) {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(scopes);
    tx.send(set_directive(tokens)).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(
        applied.len(),
        1,
        "受理の行がちょうど 1 本ではない: {logs:?}"
    );
    (applied[0].to_string(), world, ledger)
}

// ---------------------------------------------------------------------------
// 受理の行の語（要件 1.4・2.3・7.4）
// ---------------------------------------------------------------------------

/// `s0,b0,s1` の受理の行は `normalized=0:inverted,1:false+b1` を逐語で持ち、
/// 既存の記録の名前・出力先・欄の並びは今のまま（要件 1.4・7.4）。
///
/// 「採らなかった」の旧語 `true` が `normalized=` 欄に 1 つも出ないことも併せて見る
/// （要件 1.4 後段）。
#[test]
fn t_zdw01_inverted_with_implied_partner_line_is_verbatim() {
    let (line, _, _) = accept_once(&["s0", "b0", "s1"], &[0, 1]);

    assert!(
        line.contains(GROUP_TARGET),
        "受理の行の出力先が変わった: {line}"
    );
    assert!(line.contains("level=DEBUG"), "受理の水準が変わった: {line}");
    assert!(
        line.contains(
            "action=set group_id=0 source=Tag members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1"
        ),
        "受理の行の本文が逐語で一致しない: {line}"
    );
    let normalized = line
        .split("normalized=")
        .nth(1)
        .expect("normalized= 欄が無い");
    assert!(
        !normalized.contains("true"),
        "「書かれた順を採らなかった」の語が出ている: {line}"
    );
}

/// 非隣接 `b1,s0,s1,b0` の受理の行は `1:split,0:inverted-split`（スコープの初出順）
/// で、補いの綴りは付かない（要件 2.3）。
#[test]
fn t_zdw02_split_line_names_both_split_shapes() {
    let (line, _, _) = accept_once(&["b1", "s0", "s1", "b0"], &[0, 1]);

    assert!(
        line.contains("members=b1,s0,s1,b0 normalized=1:split,0:inverted-split"),
        "非隣接の語が逐語で載っていない: {line}"
    );
    assert!(!line.contains('+'), "書いていない補いが載った: {line}");
}

/// 補いの綴り `+bN`／`+sN` と隣り合いの語 `false` は据え置き（要件 7.4）。
///
/// `b1` だけ → `+s1`（キャラクターを補う）、`s0` だけ → `+b0`（バルーンを補う）の
/// 両方向を 1 行で踏む。
#[test]
fn t_zdw03_existing_spellings_false_and_partner_are_kept() {
    let (line, _, _) = accept_once(&["b1", "s0"], &[0, 1]);

    assert!(
        line.contains("members=b1,s1,b0,s0 normalized=1:false+s1,0:false+b0"),
        "既存の綴り（false・+sN・+bN）が変わった: {line}"
    );
}

// ---------------------------------------------------------------------------
// 受理と公開が同じ相で済む（要件 1.5・6.5）
// ---------------------------------------------------------------------------

/// 逆転のタグの受理と同じ 1 回の相で、書かれた順の計画が受け口へ公開され印が立つ。
///
/// 後の巡を待たない（要件 1.5）。窓が揃っていれば「次の相」は要らない。
#[test]
fn t_zdw04_acceptance_and_publication_happen_in_the_same_phase() {
    let (_, world, _) = accept_once(&["s0", "b0", "s1"], &[0, 1]);

    assert_eq!(
        chain_members(&world),
        Some(vec![
            char_of(&world, 0),
            balloon_of(&world, 0),
            balloon_of(&world, 1),
            char_of(&world, 1),
        ]),
        "受理と同じ相で書かれた順（補い済み）の計画が公開されていない"
    );
    assert_eq!(dirty(&world), Some(true), "受理と同じ相で印が立っていない");
}

// ---------------------------------------------------------------------------
// 起動の段の descript がタグと同じ解釈を通る（要件 1.3）
// ---------------------------------------------------------------------------

/// descript の `s0,b0,s1` はタグと同じ要素列・同じ `normalized=` の語で据わり、
/// 最初の相でタグと同じ計画が公開される（要件 1.3）。
///
/// 突き合わせの相手はタグの経路の実際の行である——片方だけを期待値と比べると、
/// 両方が同じ向きに崩れた日に気づけない。
#[test]
fn t_zdw05_descript_inversion_goes_through_the_tag_interpretation() {
    let (tag_line, _, _) = accept_once(&["s0", "b0", "s1"], &[0, 1]);

    let mut ledger = ZOrderGroupLedger::default();
    let logs = capture_logs(|| apply_descript_base(&mut ledger, Some("s0,b0,s1")));
    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(
        applied.len(),
        1,
        "起動の段の受理の行が 1 本ではない: {logs:?}"
    );
    let boot_line = applied[0];
    assert!(
        boot_line.contains(
            "action=set group_id=0 source=Descript members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1"
        ),
        "起動の段の行が逐語で一致しない: {boot_line}"
    );
    let tail = |line: &str| line.split("members=").nth(1).map(str::to_string);
    assert_eq!(
        tail(boot_line),
        tail(&tag_line),
        "起動の段とタグで要素列・並びの形の語が割れた"
    );

    let mut world = world_with_scopes(&[0, 1]);
    let (_tx, rx) = directive_channel();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        chain_members(&world),
        Some(vec![
            char_of(&world, 0),
            balloon_of(&world, 0),
            balloon_of(&world, 1),
            char_of(&world, 1),
        ]),
        "descript の逆転が最初の相で書かれた順のまま公開されていない"
    );
}

// ---------------------------------------------------------------------------
// 解除と読み直しで組の並びへ戻る（要件 4.5）
// ---------------------------------------------------------------------------

/// `\![reset,zorder]` で逆転のタグが解け、計画は既定（鎖なし）へ戻る。
/// 印が立つので、適用系は同じ相の後で組を結び直せる（要件 4.5・6.5）。
#[test]
fn t_zdw06_reset_returns_the_plan_to_pairs() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);
    tx.send(set_directive(&["s0", "b0", "s1"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert!(chain(&world).is_some(), "下ごしらえ: 逆転の計画が無い");
    clear_dirty(&mut world);

    tx.send(ZOrderDirective::Reset).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(chain(&world), None, "解除で計画が組の既定へ戻っていない");
    assert_eq!(dirty(&world), Some(true), "解除が同じ相で公開されていない");
}

/// descript の基底が逆転を書いていれば、解除でそのスコープは descript の並びへ戻り、
/// タグで逆転させたスコープは組（バルーンがキャラクターのすぐ手前）へ戻る（要件 4.5 括弧書き）。
#[test]
fn t_zdw07_reset_keeps_the_descript_inversion_and_pairs_the_rest() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    apply_descript_base(&mut ledger, Some("s2,b2"));
    let mut world = world_with_scopes(&[0, 1, 2]);
    tx.send(set_directive(&["s0", "b0", "s1"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    clear_dirty(&mut world);

    tx.send(ZOrderDirective::Reset).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        chain_members(&world),
        Some(vec![
            char_of(&world, 2),
            balloon_of(&world, 2),
            balloon_of(&world, 0),
            char_of(&world, 0),
            balloon_of(&world, 1),
            char_of(&world, 1),
        ]),
        "解除の後が「descript の逆転＋残りは組」になっていない"
    );
    assert_eq!(dirty(&world), Some(true), "解除が同じ相で公開されていない");
}

/// シェルの切り替えの後の `\![reset,zorder]` で逆転のタグが解け、計画は新しい
/// シェルの組の並びへ戻る。設定の無いシェルなら既定（鎖なし）へ戻る（要件 4.5）。
///
/// 切り替えそのものは台本の組を保つ（`stay-on-top-directives` 要件 5.1）ので、
/// 台本の組と重なる新しい基底は reset で据わる（同 要件 5.5）。
#[test]
fn t_zdw08_descript_reload_returns_the_plan_to_pairs() {
    // 新しいシェルが数字だけの記法を書いている → 組の並び。
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);
    tx.send(set_directive(&["s0", "b0", "s1"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    clear_dirty(&mut world);

    apply_descript_rebase(&mut ledger, Some("0,1"));
    tx.send(ZOrderDirective::Reset).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        chain_members(&world),
        Some(vec![
            balloon_of(&world, 0),
            char_of(&world, 0),
            balloon_of(&world, 1),
            char_of(&world, 1),
        ]),
        "読み直しの後が組の並びになっていない"
    );
    assert_eq!(dirty(&world), Some(true), "読み直しが公開されていない");

    // 新しいシェルに設定が無い → 既定（鎖なし）。非隣接のタグから戻す。
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);
    tx.send(set_directive(&["b1", "s0", "s1", "b0"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    clear_dirty(&mut world);
    apply_descript_rebase(&mut ledger, None);
    tx.send(ZOrderDirective::Reset).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        chain(&world),
        None,
        "設定の無いシェルで計画が既定へ戻らない"
    );
    assert_eq!(dirty(&world), Some(true), "既定への戻りが公開されていない");
}
