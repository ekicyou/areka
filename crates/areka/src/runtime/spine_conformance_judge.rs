//! 適合一周走行の判定（areka-P0-emo2-conformance-e2e・task 3.2・design D3「記録の台帳」）。
//!
//! 同一の走行から採った 3 つの列——**交信の列**・**〈段名・表示指令〉の列**・**進行状態の列**——を、
//! 段の駆動をすべて終えた**後にまとめて**、期待と**等値**で突き合わせる（R2.3・R2.4・R2.5）。
//! 部分一致・包含判定・「少なくとも N 件」の類は 1 つも使わない。
//!
//! # 置き場を分けた理由（R2.11）
//!
//! 判定の親である `spine_conformance_lap_tests.rs` は task 3.1 の完了時点で 993 行あり、1 ファイル
//! 1,000 行の見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs`）まで余白が 7 行しか
//! なかった。R2.11 の「超える場合は主題単位に分けて接続する」に従い、判定だけを本ファイルへ分けて
//! **親の末尾から**接続する（`spine.rs` の接続宣言は 3 本で確定済みゆえ、経路をそちらへ増やさない）。
//! 同じ手順を task 2.3 が支援層で踏んでいる。親に残っていた終了握手・列の同一走行性の照合も、
//! 判定である以上ここへ集めた。
//!
//! # 「送らないこと」がなぜ等値だけで成立するか（R3.6・R3.7）
//!
//! 会話の発火（`OnTalk`）・時報（`OnHour`）・更新系 4 種・バルーン変更は、期待列
//! （[`expected_calls`]）に**書かれていない**。等値照合は長さも並びも固定するので、これらが 1 件でも
//! 現れれば列は必ず不一致になる——「0 件であること」を別に数える必要はない。ただしこれが成り立つのは
//! **採取が途中で濾されていない**ときに限る。3 つの取り出し口はいずれも濾さない:
//!
//! - `ScriptedShioriHandle::non_status_calls()`（`spine.rs:302-311`）は死活の問い合わせ
//!   （`RecordedCall::Status`）だけを除く。除かれるのは受け口が自発的に打つ死活監視であって、
//!   一周が送る照会・通知は 1 件も落ちない。
//! - `snapshot_status_calls()`（`spine_conformance_support.rs:60-62`）は台帳をそのまま複製する。
//! - 表示指令は `Emo2Wiring::drain_received()` を素通しで積む（同 `:449-451`）。投影
//!   [`project_display`] も、知らない指令を捨てずに [`DisplayProjection::Unknown`] として 1 件残す。
//!
//! # 判定の本体が 2 行しかないこと（R12.5 の記録・完成判定へ申し送る）
//!
//! design D3 は〈段名・表示指令〉の列の完全一致を「判定の本体」と書く。ところが一周で表示指令を
//! 生む段は**装着と撫での 2 つだけ**であり、この列は **2 行**にしかならない（理由は
//! [`expected_display`] の doc）。**R2.4 の判定は設計が想定するより実質的に弱い**。本走行が機械で
//! 証明していると言えるのは、3 列を合わせた 33 行の等値であって、表示指令の列だけではない。
//! 架空の表示指令で列を水増しすることも、台本の応答を実物から離して指令を作り出すこともしない。
//!
//! # 自己検査と製品の判定を混ぜない（design D3 の裁定）
//!
//! 「採取した時点の注入時刻が当該段の宣言区間に入る」ことは**駆動器が契約どおり動いたことの
//! 自己検査**であって製品の判定ではない。駆動器は採取のたびにこれを見て
//! `StageFailure::CollectedOutsideInterval` を返す（`spine_conformance_support.rs:529-536`）。
//! 本ファイルは採り終えた列の側からも同じ性質を確かめるが、失敗の文面を製品の判定と**別の語**に
//! して、テスト自身の駆動が壊れたことを製品の退行と読み違えないようにする
//! （[`assert_display_collected_within_declared_stage`]）。

use super::super::RecordedCall;
use super::super::conformance_script::{
    LAP_STAGES, expected_calls, expected_display, expected_statuses,
};
use super::super::conformance_support::{CollectedCommand, DisplayProjection, project_display};
use super::{LapLedgers, get_calls};

/// 差分に並べる食い違いの最大行数（これを超えたら以降は省略する）。
///
/// 先頭に 1 件挿入されただけで以降の全位置がずれるため、上限が無いと失敗の出力が読めない量になる。
/// **最初に食い違う位置**は必ず本文に出るので、省略しても原因の頭は失われない。
const MAX_DIFF_LINES: usize = 12;

/// 3 つの列を期待と等値で突き合わせる（design D3・R2.3／2.4／2.5・R3.6／3.7）。
///
/// 呼び手は段の駆動を**すべて終えてから**1 度だけ呼ぶ（段の途中で部分照合しない・design D1）。
pub(super) fn judge_lap(ledgers: &LapLedgers) {
    // ── 自己検査（製品の判定ではない・design D3 の裁定） ──
    assert_display_collected_within_declared_stage(&ledgers.display);

    // ── 解放の件数（design「失敗の形」の独立した 1 行・R3.9） ──
    //     等値照合でも捕まるが、件数の食い違いは列の差分より先に、件数として読めた方が速い。
    assert_eq!(
        unload_calls(&ledgers.calls),
        1,
        "解放がちょうど 1 件でない（R3.9）: 実測 {} 件",
        unload_calls(&ledgers.calls)
    );
    assert_eq!(
        get_calls(&ledgers.calls, "OnClose"),
        1,
        "終了の照会がちょうど 1 件でない: 実測 {} 件",
        get_calls(&ledgers.calls, "OnClose")
    );

    // ── 3 つの列が同一の走行から採れていること（design D1「一貫性」） ──
    //     進行状態は照会・片道の 2 種にだけ載り、解放には載らない。
    assert_eq!(
        ledgers.statuses.len(),
        ledgers.calls.len() - unload_calls(&ledgers.calls),
        "進行状態の列が交信の列と同じ走行から採れていない（進行状態 {} 件・交信 {} 件のうち解放 {} 件）",
        ledgers.statuses.len(),
        ledgers.calls.len(),
        unload_calls(&ledgers.calls)
    );

    // ── 判定 1: 交信の列（呼出の別・id・参照列）──
    assert_sequence_eq("交信の列", &ledgers.calls, &expected_calls());

    // ── 判定 2: 〈段名・表示指令〉の列（design D3 の「判定の本体」）──
    let display: Vec<(&'static str, DisplayProjection)> = ledgers
        .display
        .iter()
        .map(|(stage, collected)| (*stage, project_display(&collected.command)))
        .collect();
    assert_sequence_eq("〈段名・表示指令〉の列", &display, &expected_display());

    // ── 判定 3: 進行状態の列（選択待ちを会話中と区別できる唯一の列・R3.8）──
    assert_sequence_eq("進行状態の列", &ledgers.statuses, &expected_statuses());
}

/// 記録の中の解放の件数。
fn unload_calls(calls: &[RecordedCall]) -> usize {
    calls
        .iter()
        .filter(|call| matches!(call, RecordedCall::Unload))
        .count()
}

/// 2 つの列を**等値**で突き合わせ、違えば食い違う位置を名指しして落とす。
///
/// # なぜ素の `assert_eq!` を使わないのか（R12.5）
///
/// 列は 15〜16 要素あり、素の `assert_eq!` は 2 つの列を丸ごと 1 行に印字するだけで、**どこが**
/// 違うのかを読み手に探させる。1 件の挿入と 1 件の値の違いも同じ見た目になるため、退行の形が
/// 判別できない。ここでは⑴長さ⑵最初に食い違う位置⑶食い違う各位置の期待と実測、を出す。
fn assert_sequence_eq<T: PartialEq + std::fmt::Debug>(label: &str, actual: &[T], expected: &[T]) {
    if actual == expected {
        return;
    }
    let span = actual.len().max(expected.len());
    let first = (0..span)
        .find(|&i| actual.get(i) != expected.get(i))
        .expect("列が等しくないなら食い違う位置が必ず 1 つは在る");

    let mut shown = 0usize;
    let mut lines = String::new();
    for i in 0..span {
        if actual.get(i) == expected.get(i) {
            continue;
        }
        if shown == MAX_DIFF_LINES {
            lines.push_str("\n  …（食い違いが多いため以降は省略）");
            break;
        }
        shown += 1;
        lines.push_str(&format!(
            "\n  [{i}] 期待: {:?}\n       実測: {:?}",
            expected.get(i),
            actual.get(i)
        ));
    }

    panic!(
        "{label}が期待と一致しない（等値照合・部分一致は用いない＝R2.5）\n  件数: 期待 {} 件・実測 {} 件\n  最初に食い違う位置: [{first}]{lines}",
        expected.len(),
        actual.len()
    );
}

/// 採取した表示指令の注入時刻が、その段の宣言区間に入っていることを確かめる（**自己検査**）。
///
/// 製品の判定ではない。駆動器の不変条件（注入時刻は段の上限を超えない）が守られる限り必ず真に
/// なるため、製品が退行しても赤にならない——残す理由は、テスト自身の駆動が壊れたときに
/// 「段名の食い違い」として製品の退行に見えることを防ぐためである（design D3 の裁定）。
///
/// 段名 `サブメニューと戻り` は区間を 2 つ持つ（選択肢 ID が直前の台本の `\q` 帳簿に縛られるため
/// 1 段では 2 つ選べない）。ゆえに「**同名のいずれかの区間**に入っていること」を見る。
fn assert_display_collected_within_declared_stage(display: &[(&'static str, CollectedCommand)]) {
    for (index, (stage, collected)) in display.iter().enumerate() {
        let intervals: Vec<&'static str> = LAP_STAGES
            .iter()
            .filter(|declared| declared.name == *stage)
            .map(|declared| declared.name)
            .collect();
        assert!(
            !intervals.is_empty(),
            "駆動器の自己検査が失敗: 表示指令 [{index}] の段名「{stage}」が段の宣言に無い（製品の退行ではなく駆動が壊れている）"
        );
        let inside = LAP_STAGES.iter().any(|declared| {
            declared.name == *stage
                && (declared.begin_ms..=declared.limit_ms).contains(&collected.collected_at_ms)
        });
        assert!(
            inside,
            "駆動器の自己検査が失敗: 表示指令 [{index}]（段「{stage}」）の採取時の注入時刻 {}ms が宣言区間の外（製品の退行ではなく駆動が壊れている）",
            collected.collected_at_ms
        );
    }
}
