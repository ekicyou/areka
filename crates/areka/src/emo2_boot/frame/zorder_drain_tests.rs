// =============================================================================
// zorder drain 相：指令の台帳適用と記録の決定論テスト（task 3.3・要件 1.4／8.1／8.3／8.4）
//
// この相が指令に対して負うのは 3 つである——「窓の正本が無い間は取り出さないこと」
// 「到着順にそのまま適用すること」「受理も拒否も必ず記録すること」。
//
// 檻は毎回**両側から**挟む。「取り出さない」の隣に「後で取り出せる」を、「拒否は台帳を
// 変えない」の隣に「受理は台帳を変える」を、「記録が出ない」の隣に「記録が出る」を置く。
// 片側だけの主張は、経路そのものが死んでいても緑のままになる（1.2 の差し戻しで実証済み）。
// =============================================================================

use super::test_support::*;
use super::*;

use crate::placement::zorder_group_ledger::GroupSource;

// ---------------------------------------------------------------------------
// 窓の正本が無い間は取り出さない（要件 1.4・move の相と同じ保留の作法）
// ---------------------------------------------------------------------------

/// `GhostWindows` が無い巡は指令を 1 件も取り出さず、チャネルへ残したまま戻る。
///
/// 「残っている」ことを受信端から直に確かめる——台帳が空であることだけでは
/// 「取り出して捨てた」と区別が付かない（起動直後の指令が黙って消える形がまさにそれ）。
#[test]
fn t_zdr01_holds_directives_while_the_window_registry_is_absent() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = World::new();

    tx.send(set_directive(&["1", "0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert!(
        ledger.groups().is_empty(),
        "窓の正本が無い巡に台帳が動いた（保留になっていない）"
    );
    assert_eq!(
        projected(&world),
        None,
        "窓の正本が無い巡に受け口が作られた（射影する対象が無い）"
    );
    assert!(
        logs.is_empty(),
        "保留の巡に記録が出た（取り出していないのだから報せることは無い）: {logs:?}"
    );
    assert_eq!(
        rx.try_recv(),
        Ok(set_directive(&["1", "0"])),
        "保留のはずの指令がチャネルから消えている（取り出して捨てている）"
    );
}

/// 保留された指令は、窓が生えた最初の巡に**到着順のまま**まとめて適用される。
///
/// 上の檻の反対側である。ここが緑にならなければ、保留は「取りこぼさない」ではなく
/// 「永久に効かない」を意味してしまう。
#[test]
fn t_zdr02_applies_held_directives_in_arrival_order_once_windows_exist() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = World::new();

    tx.send(set_directive(&["1", "0"])).unwrap();
    tx.send(set_directive(&["3", "2"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert!(ledger.groups().is_empty(), "保留の巡に台帳が動いた");

    spawn_scopes(&mut world, &[0, 1, 2, 3]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    let ids: Vec<u32> = ledger.groups().iter().map(|g| g.id).collect();
    assert_eq!(ids, vec![0, 1], "到着順に台帳へ載っていない");
    assert_eq!(
        ledger.groups()[0].members[0].scope,
        1,
        "先に届いた指令の先頭要素が入れ替わっている"
    );
    assert_eq!(
        ledger.groups()[1].members[0].scope,
        3,
        "後に届いた指令の先頭要素が入れ替わっている"
    );
}

// ---------------------------------------------------------------------------
// 受理の記録（要件 8.3・2.4）
// ---------------------------------------------------------------------------

/// 受理は `[zorder-group] applied` で、台帳に載った内容とともに残る。
#[test]
fn t_zdr03_accepted_tag_is_recorded_with_the_ledger_content() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);

    tx.send(set_directive(&["1", "0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(
        applied.len(),
        1,
        "受理の記録がちょうど 1 本ではない: {logs:?}"
    );
    let line = applied[0];
    assert!(
        line.contains(GROUP_TARGET),
        "記録の出力先が 1 本に揃っていない: {line}"
    );
    assert!(
        line.contains("level=DEBUG"),
        "受理の水準が debug でない: {line}"
    );
    assert!(line.contains("action=set"), "受理の種別が読めない: {line}");
    assert!(
        line.contains("group_id=0"),
        "グループの識別子が載っていない: {line}"
    );
    assert!(
        line.contains("members=b1,s1,b0,s0"),
        "台帳へ載った要素列が載っていない（数値モードの展開は手前から b→s）: {line}"
    );
    assert!(
        line.contains("source=Tag"),
        "出所（タグ由来）が載っていない: {line}"
    );
}

/// 明示モードで作者の書いた順を採らなかったときは、その事実が受理の記録に載る（要件 2.4）。
///
/// 対照として数値モードを並べる——あちらは調整そのものが起きないので番兵になる。
/// 片側だけだと「常に番兵」でも「常に何か載る」でも通ってしまう。
#[test]
fn t_zdr04_normalization_is_surfaced_only_when_the_author_order_was_adjusted() {
    // 反転（s1 が b1 より前）＝作者の順をそのままでは採らなかった。
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);
    tx.send(set_directive(&["s1", "b1", "b0", "s0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    let line = lines_with(&logs, "[zorder-group] applied")[0];
    assert!(
        line.contains("normalized=1:true,0:false"),
        "同一スコープの調整の有無が両方載っていない: {line}"
    );
    assert!(
        line.contains("members=b1,s1,b0,s0"),
        "隣接ブロックへ寄せた後の要素列が載っていない: {line}"
    );

    // 数値モード＝エンジンが組んだ並びゆえ調整の記録は無い（番兵）。
    let (tx2, rx2) = directive_channel();
    let mut ledger2 = ZOrderGroupLedger::default();
    let mut world2 = world_with_scopes(&[0, 1]);
    tx2.send(set_directive(&["1", "0"])).unwrap();
    let logs2 = capture_logs(|| run_zorder_drain_phase(&rx2, &mut ledger2, &mut world2));
    let line2 = lines_with(&logs2, "[zorder-group] applied")[0];
    assert!(
        line2.contains("normalized=-"),
        "数値モードで調整の記録が出ている（作者の指定順ではない並びを調整と呼んでいる）: {line2}"
    );
}

/// 片方の窓だけを指名したタグでは、**補った相棒窓**が受理の記録に載る（要件 2.6 の
/// 「加えたことを記録する」・design「Requirements Traceability」2.6 行が名指しする
/// `[zorder-group] applied` の `normalized=` 欄）。
///
/// 檻は**実際の字面のまま**当てる。`members=` と `normalized=` は行の中で隣り合うので
/// 続けて 1 本で見るが、間に他の欄が挟まる形を連結して丸めることはしない。
///
/// あわせて、畳み込みの導入で**嘘をつくようになっていないこと**を 2 方向から見る。
/// ⑴ 補ったスコープの欄が `1:false` のままだと「作者の書いた順をそのまま採った」と
///    読めてしまう（窓 2 枚を黙って足しておきながら「調整していない」と言う行）。
/// ⑵ 2 窓そろって書かれたスコープの字面は 1 バイトも変わっていない（要件 9.5）。
#[test]
fn t_zdr11_an_implied_partner_window_is_named_in_the_accepted_record() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);
    tx.send(set_directive(&["b1", "s0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    let line = lines_with(&logs, "[zorder-group] applied")[0];

    assert!(
        line.contains("members=b1,s1,b0,s0 normalized=1:false+s1,0:false+b0"),
        "補った相棒窓（s1 と b0）が記録に名指しされていない: {line}"
    );
    assert!(
        !line.contains("normalized=1:false,0:false"),
        "窓 2 枚を暗黙に加えておきながら「調整していない」と読める行が出ている: {line}"
    );

    // 対照: 2 窓そろって書かれたスコープの字面は据え置き（既存の檻が読む語）。
    let (tx2, rx2) = directive_channel();
    let mut ledger2 = ZOrderGroupLedger::default();
    let mut world2 = world_with_scopes(&[0, 1]);
    tx2.send(set_directive(&["s1", "b1", "b0", "s0"])).unwrap();
    let logs2 = capture_logs(|| run_zorder_drain_phase(&rx2, &mut ledger2, &mut world2));
    let line2 = lines_with(&logs2, "[zorder-group] applied")[0];
    assert!(
        line2.contains("members=b1,s1,b0,s0 normalized=1:true,0:false"),
        "2 窓そろったスコープの字面が畳み込みの導入で変わっている（要件 9.5）: {line2}"
    );
    assert!(
        !line2.contains("+b"),
        "補っていないのに補いの欄が出ている: {line2}"
    );
    assert!(
        !line2.contains("+s"),
        "補っていないのに補いの欄が出ている: {line2}"
    );
}

// ---------------------------------------------------------------------------
// 拒否の記録（要件 8.1／8.3／3.2）
// ---------------------------------------------------------------------------

/// 解釈できない指定は台帳を一切変えず、理由と**受け取ったトークン列**を warn で残す。
#[test]
fn t_zdr05_unparsable_tag_leaves_the_ledger_untouched_and_is_recorded() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);

    tx.send(set_directive(&["Balloon0", "s1"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert!(
        ledger.groups().is_empty(),
        "拒否したのに台帳が動いた（部分適用の禁止・要件 8.1）"
    );
    let rejected = lines_with(&logs, "[zorder-group] rejected");
    assert_eq!(
        rejected.len(),
        1,
        "拒否の記録がちょうど 1 本ではない: {logs:?}"
    );
    let line = rejected[0];
    assert!(
        line.contains(GROUP_TARGET),
        "記録の出力先が 1 本に揃っていない: {line}"
    );
    assert!(
        line.contains("level=WARN"),
        "拒否の水準が warn でない（既定運転で読めなければ黙殺と同じ）: {line}"
    );
    assert!(
        line.contains("reason=UnparsableToken(Balloon0)"),
        "拒否理由が載っていない: {line}"
    );
    assert!(
        line.contains("tokens=Balloon0,s1"),
        "受け取ったトークン列が載っていない（作者が何を書いたか復元できない）: {line}"
    );
    assert!(
        lines_with(&logs, "[zorder-group] applied").is_empty(),
        "拒否したのに受理の記録が出ている: {logs:?}"
    );
}

/// 台帳の段で落ちる拒否（既に他グループが押さえているスコープ）も同じ形で残り、
/// 既存グループは 1 つも変わらない（要件 3.2）。
#[test]
fn t_zdr06_cross_group_redesignation_is_rejected_whole_and_recorded() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1, 2]);

    tx.send(set_directive(&["1", "0"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    let before = ledger.groups().to_vec();

    tx.send(set_directive(&["2", "0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert_eq!(
        ledger.groups(),
        before.as_slice(),
        "再指定を拒否したのに台帳が動いた（要件 3.2）"
    );
    let line = lines_with(&logs, "[zorder-group] rejected")[0];
    assert!(
        line.contains("reason=CrossGroupRedesignation(0)"),
        "衝突したスコープが載っていない: {line}"
    );
    assert!(
        line.contains("tokens=2,0"),
        "受け取ったトークン列が載っていない: {line}"
    );
}

/// 1 件の拒否は、同じ巡でその後ろに並んでいる指令を止めない（log-first・非 panic）。
#[test]
fn t_zdr07_one_rejected_directive_does_not_stop_the_ones_behind_it() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1, 2, 3]);

    tx.send(set_directive(&["0", "b1"])).unwrap(); // モード混在＝拒否
    tx.send(set_directive(&["3", "2"])).unwrap(); // 後続＝受理
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert_eq!(
        ledger.groups().len(),
        1,
        "後続の指令が巻き添えで落ちている（または拒否が載ってしまった）"
    );
    assert_eq!(
        ledger.groups()[0].members[0].scope,
        3,
        "台帳に載ったのが後続の指令ではない"
    );
    assert_eq!(
        lines_with(&logs, "[zorder-group] rejected").len(),
        1,
        "拒否の記録が 1 本ではない: {logs:?}"
    );
    assert_eq!(
        lines_with(&logs, "[zorder-group] applied").len(),
        1,
        "受理の記録が 1 本ではない: {logs:?}"
    );
    let line = lines_with(&logs, "[zorder-group] rejected")[0];
    assert!(
        line.contains("reason=ModeMixed"),
        "混在の拒否理由が載っていない: {line}"
    );
}

// ---------------------------------------------------------------------------
// 解除（要件 4.1／4.2）
// ---------------------------------------------------------------------------

/// 解除はタグ由来を落として基底へ戻り、終状態が記録に残る。
///
/// 基底が在る場合と無い場合を並べる——片方だけだと「常に空へ戻す」実装も
/// 「常に何も落とさない」実装も素通りする。
#[test]
fn t_zdr08_reset_falls_back_to_the_descript_base_and_records_the_result() {
    // 基底あり: タグ由来だけが落ちる。
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1, 2, 3]);
    ledger.set_descript_base(vec![
        GroupElement {
            scope: 3,
            kind: GroupWindowKind::Balloon,
        },
        GroupElement {
            scope: 3,
            kind: GroupWindowKind::Char,
        },
        GroupElement {
            scope: 2,
            kind: GroupWindowKind::Balloon,
        },
        GroupElement {
            scope: 2,
            kind: GroupWindowKind::Char,
        },
    ]);
    tx.send(set_directive(&["1", "0"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        ledger.groups().len(),
        2,
        "下ごしらえ（基底＋タグ）が成立していない"
    );

    tx.send(ZOrderDirective::Reset).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert_eq!(ledger.groups().len(), 1, "解除でタグ由来が落ちていない");
    assert_eq!(
        ledger.groups()[0].source,
        GroupSource::Descript,
        "解除で基底まで落ちている（要件 4.1）"
    );
    let line = lines_with(&logs, "[zorder-group] applied")[0];
    assert!(
        line.contains("action=reset"),
        "解除の種別が読めない: {line}"
    );
    assert!(
        line.contains("groups=1"),
        "解除後の残り本数が載っていない: {line}"
    );
    assert!(
        line.contains("base=b3,s3,b2,s2"),
        "戻り先の基底が載っていない: {line}"
    );

    // 基底なし: 既定状態（0 本）へ戻り、記録の欄は番兵になる。
    let (tx2, rx2) = directive_channel();
    let mut ledger2 = ZOrderGroupLedger::default();
    let mut world2 = world_with_scopes(&[0, 1]);
    tx2.send(set_directive(&["1", "0"])).unwrap();
    run_zorder_drain_phase(&rx2, &mut ledger2, &mut world2);
    tx2.send(ZOrderDirective::Reset).unwrap();
    let logs2 = capture_logs(|| run_zorder_drain_phase(&rx2, &mut ledger2, &mut world2));

    assert!(
        ledger2.groups().is_empty(),
        "基底が無いのに何かが残った（要件 4.2）"
    );
    let line2 = lines_with(&logs2, "[zorder-group] applied")[0];
    assert!(
        line2.contains("groups=0"),
        "解除後の残り本数が 0 でない: {line2}"
    );
    assert!(
        line2.contains("base=-"),
        "基底なしが番兵で表れていない: {line2}"
    );
}

// ---------------------------------------------------------------------------
// 記録の出口は 1 本（要件 9.5・task 2.1 が入口を閉じた理由）
// ---------------------------------------------------------------------------

/// この相のソースには `tracing` のマクロが 1 つも無い（記録は wintf の入口だけを通る）。
///
/// 対照として、同じ走査が実際にマクロを持つファイル（受け口の `zorder_cue.rs`）では
/// 必ず何かを見つけることを併置する——走査そのものが空振りしていれば、不在の主張は
/// 恒真になる。
#[test]
fn t_zdr09_this_phase_emits_no_records_of_its_own() {
    const MACRO_NEEDLES: [&str; 6] = [
        "trace!(",
        "debug!(",
        "info!(",
        "warn!(",
        "error!(",
        "tracing::",
    ];
    let here = code_only(include_str!("zorder_drain.rs"));
    let sibling = code_only(include_str!("../zorder_cue.rs"));

    for needle in MACRO_NEEDLES {
        assert!(
            !here.contains(needle),
            "この相に `{needle}` が現れた（サインオフの grep 対象が 2 本へ割れる）"
        );
    }
    let found = MACRO_NEEDLES
        .iter()
        .filter(|needle| sibling.contains(**needle))
        .count();
    assert!(
        found >= 2,
        "走査がマクロを持つ兄弟でも何も見つけない（走査そのものが壊れている疑い）"
    );
    assert!(
        here.contains("fn run_zorder_drain_phase("),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// 註釈の行（`//` で始まる行）を落とした本文。説明文に書いてあるだけの綴りを
/// 「在る」と数えないため（`tick_gate_config_producers_tests.rs` と同じ流儀）。
///
/// 削り過ぎの検出は**除去後の側**へ当てる（除去前へ当てると、doc に語があるだけで
/// 真になり、コード行を全部落としても緑のまま通る＝3.1 の教訓）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// 何も届いていない巡・受け口が閉じた巡（縮退で台本を殺さない）
// ---------------------------------------------------------------------------

/// 指令が 1 件も無い巡は、記録も台帳の変化も起こさない。
#[test]
fn t_zdr10_an_empty_pass_is_silent() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);
    drop(tx); // 送信端が全て落ちた形（try_iter は尽きるだけで panic しない）

    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert!(
        ledger.groups().is_empty(),
        "何も届いていないのに台帳が動いた"
    );
    assert!(logs.is_empty(), "何も届いていない巡に記録が出た: {logs:?}");
    assert_eq!(
        projected(&world),
        None,
        "グループが 1 つも無いのに受け口が作られた（要件 6.1）"
    );
}
