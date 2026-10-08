//! OS のセッションの終了で SHIORI を待つ上限（[`SESSION_END_SHIORI_LIMIT`]）のテスト
//! （areka-P0-session-mark-residue task 5.3・要件 1.1・1.3〜1.5・2.1〜2.4・2.6・3.3・7.1・7.2）。
//!
//! 偽の SHIORI の土台（`SwitchRig`）で A を起こし、台本の「解かれるまで固まる」呼び出し
//! （`hold_at`）で段ごとに待ちを作ってから、上限を引数で受ける入口 [`end_session_within`] を呼ぶ。
//! 補助のスレッドは偽の SHIORI が固まったこと（`holding()`）を見てから見張りの手動の口
//! （`cut_now`）を呼ぶ。固まっている間は後始末が戻れず、固まりを解くのは見張りだけなので、
//! 発火の時点の段は時刻に依らず決まる（`cut_now` は見張りを張る前に呼んでも予約として残る）。
//! 上限は負荷で期限の口が先に発火しないよう大きく取る（期限の口そのものは短い上限の 1 本で見る）。
//!
//! # `warn!(shiori_wait_cut)` の件数と段の語をどう判定するか
//! 打ち切りの `warn!` は見張りのスレッドで出るので、呼び手のスレッドだけを捕える
//! `log_capture_kit::capture` には映らない。全スレッドの捕捉（`install_global_capture_all`）は
//! このテストバイナリ（areka の全テスト）の全スレッドの記録を以後ずっと溜め、`tracing::enabled!` を
//! 全スレッドで真にするので、ここでは使わない。代わりに次の 2 つで判定する:
//! - 発火の回数: 見張りの決め手は発火 1 回につき解く手を 1 回呼んでから `warn!` を 1 件出す
//!   （`areka_kanade` の `try_fire`）。偽の SHIORI の解く手は呼ばれた回数を数えるので、
//!   「解く手 1 回」が「`warn!` 1 件」、「解く手 0 回」が「`warn!` 0 件」に当たる。
//! - 段の語: 同じ固まり方の A を別に起こし、受け手が委譲する [`GhostSession::shutdown_within`] を
//!   直に呼んで、戻りの `ShioriCut`（`warn!` の `stage` と同じ値）を見る。
//!
//! 印の理由（`session_mark_kept`）と所要の記録（`os_session_end_done` の `shiori_cut`）は
//! 呼び手のスレッドで出るので、捕捉窓で判定する。
//!
//! # 環境変数（`AREKA_SHIORI_REQUEST_TIMEOUT_MS`）の 3 通り
//! 見張りは環境変数を読まず外から解くので、SHIORI 側の期限として言い換えて起こす:
//! - T より長い・無限: 台本が自分では切れずに固まる形＝段ごとのテスト（T で打ち切られる）。
//! - T より短い: 台本が自分で期限切れを返す形＝[`self_timeout_shorter_than_limit_is_not_cut`]
//!   （打ち切り 0 件・印の判定は今日どおり）。
//!
//! 終了の観測の段は UNLOAD で固まる形で代表する（`request_clean_shutdown` の中で分けられない＝
//! 設計 D4。補助プロセスが終わった後の観測は host32 の既存の短絡のテストが固定する）。
//! 呼び出しの外（`idle`）の段は見張り部品のテスト（`areka-kanade` の `probe_tests.rs`）が見る。

use std::collections::BTreeSet;
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use areka_kanade::{CloseReason, ShioriBusy, ShioriCut, ShioriProbe, WaitBudget};
use log_capture_kit::{CapturedEvent, capture};
use shiori_host32_host::{ExitKind, RequestError};

use super::{SESSION_END_SHIORI_LIMIT, end_session_within};
use crate::boot_resolve::{read_session_mark, write_session_mark};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::hold_support::HoldAt;
use crate::emo2_boot::spine::{
    RecordedCall, ScriptedShioriBackend, ScriptedShioriBackendBuilder, ScriptedShioriHandle,
    spin_wait_until,
};
use crate::ghost_session::{GhostSession, GhostSlot};

/// 負荷で期限の口が先に発火しない上限（手動の口と「上限の中で応答」の回）。
const WIDE: Duration = Duration::from_secs(60);
/// 期限の口を踏む短い上限（偽の SHIORI は固まったまま自分では応答しないので結果は時刻に依らない）。
const NARROW: Duration = Duration::from_millis(50);

/// 呼び手のスレッドで見える結果（段の語は [`held_stage`] で別に見る）。
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    /// 補助のスレッドが固まりを見て手動の口を呼んだか（補助のスレッドが無ければ `None`）。
    held_seen: Option<bool>,
    /// 解く手が呼ばれた回数（＝見張りの発火の回数＝`warn!(shiori_wait_cut)` の件数）。
    unblock_calls: usize,
    /// `OnClose` の NOTIFY の Reference0（届いた分だけ）。
    on_close_ref0: Vec<String>,
    /// 後始末の後の起動中の印。
    mark: Option<String>,
    /// `session_mark_kept` の `reason`（印を消した回は `None`）。
    kept_reason: Option<String>,
    /// `os_session_end_done` の `shiori_cut`。
    shiori_cut: Option<String>,
}

/// 呼出列のうち `OnClose` の NOTIFY の Reference0（届いた分だけ）。
fn on_close_ref0(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .filter_map(|c| match c {
            RecordedCall::Notify { id, references } if id == "OnClose" => {
                Some(references.first().cloned().unwrap_or_default())
            }
            _ => None,
        })
        .collect()
}

/// `event` の記録 1 件目の欄 `name`（文字列の欄は生値、それ以外は Debug 表現）。
fn event_field(events: &[CapturedEvent], event: &str, name: &str) -> Option<String> {
    events
        .iter()
        .find(|e| e.field_str("event") == Some(event))
        .and_then(|e| e.field_str(name).or_else(|| e.field(name)))
        .map(str::to_owned)
}

/// 台本 `script` の A を起こし、印＝A を書く。`hold` が GET なら起動の往復が固まるまで、
/// それ以外なら定常まで待つ。
fn boot_a(
    script: impl Fn() -> ScriptedShioriBackendBuilder + 'static,
    hold: Option<HoldAt>,
) -> SwitchRig {
    let mut rig = SwitchRig::new(vec![("A", FakeShiori::Scripted(Box::new(script)))]);
    rig.plant_boot_record("A");
    rig.boot("A");
    write_session_mark(&rig.app_dir(), "A");
    let ready = match hold {
        Some(HoldAt::Get(_)) => {
            let handle = rig.handle("A");
            spin_wait_until(|| handle.holding())
        }
        _ => rig.wait_steady(),
    };
    assert!(ready, "A が {hold:?} の待ちの形に着かない");
    rig
}

/// `hold` で固まる標準の台本の A。
fn boot_held(hold: HoldAt) -> SwitchRig {
    boot_a(move || standard_script("\\0A\\e").hold_at(hold), Some(hold))
}

/// 置き場の A の見張り部品（`cut_now` の口）。
fn probe_of(rig: &SwitchRig) -> ShioriProbe {
    rig.world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(GhostSession::runtime)
        .expect("置き場の A に実行系が在る")
        .shiori_probe()
        .clone()
}

/// 固まりを見てから手動の口を呼ぶ補助のスレッド（固まりを見たかを返す）。
fn cut_when_held(handle: ScriptedShioriHandle, probe: ShioriProbe) -> JoinHandle<bool> {
    thread::spawn(move || {
        let held = spin_wait_until(|| handle.holding());
        if held {
            probe.cut_now();
        }
        held
    })
}

/// 置き場の A に上限 `limit` で OS のセッションの終了を処理させる（`cut` なら補助のスレッドが
/// 固まりを見て手動の口を呼ぶ）。
fn end_session(rig: &mut SwitchRig, limit: Duration, cut: bool) -> Observed {
    let handle = rig.handle("A");
    let cutter = cut.then(|| cut_when_held(handle.clone(), probe_of(rig)));
    let ((), events) = capture(|| end_session_within(&mut rig.world, limit));
    let held_seen = cutter.map(|c| c.join().expect("補助のスレッドは panic しない"));
    let calls = rig.calls("A").last().cloned().unwrap_or_default();
    Observed {
        held_seen,
        unblock_calls: handle.unblock_calls(),
        on_close_ref0: on_close_ref0(&calls),
        mark: read_session_mark(&rig.app_dir()),
        kept_reason: event_field(&events, "session_mark_kept", "reason"),
        shiori_cut: event_field(&events, "os_session_end_done", "shiori_cut"),
    }
}

/// `hold` で固まった A を、受け手が委譲する `GhostSession::shutdown_within` で直に降ろし、
/// 補助のスレッドが固まりを見たか・打ち切りの結果（`warn!` の `stage` と同じ値）・解く手の回数を返す。
fn held_stage(hold: HoldAt) -> (bool, Option<ShioriCut>, usize) {
    let mut rig = boot_held(hold);
    let handle = rig.handle("A");
    let cutter = cut_when_held(handle.clone(), probe_of(&rig));
    let session = rig
        .world
        .get_non_send_mut::<GhostSlot>()
        .and_then(|mut slot| slot.0.take())
        .expect("置き場に A が在る");
    let budget = WaitBudget {
        started: Instant::now(),
        limit: WIDE,
    };
    let (_result, cut) = session.shutdown_within(CloseReason::System, budget);
    let held_seen = cutter.join().expect("補助のスレッドは panic しない");
    (held_seen, cut, handle.unblock_calls())
}

/// 段 `hold` で固まったとき: 後始末が戻り（このテストが終わること）、解く手 1 回（＝`warn!` 1 件）・
/// 印が残り理由が上限の語・所要の記録の打ち切りが真・`OnClose` の Ref0＝`system` は多くて 1 件。
/// 段の語は委譲先を直に呼んだ回の `ShioriCut` で見る。
fn assert_cut_at(hold: HoldAt, stage: &'static str, on_close_ref0: Vec<String>) {
    let mut rig = boot_held(hold);
    let observed = end_session(&mut rig, WIDE, true);
    // 足場は 1 つのスレッドに 1 つ（`RigPermit`）。`held_stage` が次の足場を作る前に捨てる。
    drop(rig);
    assert_eq!(
        observed,
        Observed {
            held_seen: Some(true),
            unblock_calls: 1,
            on_close_ref0,
            mark: Some("A".to_owned()),
            kept_reason: Some("session_end_deadline".to_owned()),
            shiori_cut: Some("true".to_owned()),
        },
        "{hold:?} で固まったセッションの終了が上限で打ち切られない"
    );
    assert_eq!(
        held_stage(hold),
        (
            true,
            Some(ShioriCut {
                stage,
                unblocked: true
            }),
            1
        ),
        "{hold:?} で固まった降ろしの打ち切りの段（固まりを見た・結果・解く手の回数）"
    );
}

/// 後始末に入る前から待っていた在来の往復（起動の `OnBoot` の GET）で固まる → `in_flight_request`。
/// 定常を待たずに受け手を呼ぶ。kanade は起動の往復を同期に待っているので、強制終了を読む前に
/// 起動の往復の期限切れで降ろしへ進んで止まり、`OnClose` は送らない（順序は揺れない）。
#[test]
fn cut_while_in_flight_get_is_held() {
    assert_cut_at(HoldAt::Get("OnBoot"), "in_flight_request", vec![]);
}

/// `OnClose` の通知（Ref0＝`system`）で固まる → `on_close_notify`。固まった `OnClose` は 1 件だけ。
#[test]
fn cut_while_on_close_notify_is_held() {
    assert_cut_at(
        HoldAt::Notify("OnClose"),
        "on_close_notify",
        vec!["system".to_owned()],
    );
}

/// UNLOAD で固まる → `unload`（UNLOAD の応答と終了の観測の 2 段をこの 1 つで代表する＝設計 D4）。
#[test]
fn cut_while_unload_is_held() {
    assert_cut_at(HoldAt::Unload, "unload", vec!["system".to_owned()]);
}

/// 手動の口を使わず、短い上限の期限の口からも発火する（`OnClose` で固まったまま自分では応答
/// しないので、発火は時刻に依らず起きる。発火の時点の段は `OnClose` の前か最中かで揺れうるので
/// 判定しない）。
#[test]
fn deadline_fires_without_manual_cut() {
    let mut rig = boot_held(HoldAt::Notify("OnClose"));
    assert_eq!(
        end_session(&mut rig, NARROW, false),
        Observed {
            held_seen: None,
            unblock_calls: 1,
            on_close_ref0: vec!["system".to_owned()],
            mark: Some("A".to_owned()),
            kept_reason: Some("session_end_deadline".to_owned()),
            shiori_cut: Some("true".to_owned()),
        },
        "短い上限の期限の口で打ち切られない"
    );
}

/// 上限の中で応答すれば今日どおり: 印が消え・解く手 0 回（＝`warn!` 0 件。対照は段ごとのテストの
/// 1 回）・所要の記録の打ち切りは偽。
#[test]
fn answered_within_limit_is_not_cut() {
    let mut rig = boot_a(|| standard_script("\\0A\\e"), None);
    assert_eq!(
        end_session(&mut rig, WIDE, false),
        Observed {
            held_seen: None,
            unblock_calls: 0,
            on_close_ref0: vec!["system".to_owned()],
            mark: None,
            kept_reason: None,
            shiori_cut: Some("false".to_owned()),
        },
        "上限の中で応答したのに打ち切られた・印が消えない"
    );
}

/// 標準の台本の `OnClose` を「SHIORI 側の期限切れ」に替えたもの。
fn on_close_times_out_by_itself() -> ScriptedShioriBackendBuilder {
    ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some("\\0A\\e".to_owned())))
        .notify("basewareversion", Ok(()))
        .notify("OnClose", Err(RequestError::Timeout))
        .unload(Ok(ExitKind::Clean))
}

/// SHIORI 側の期限（環境変数）が T より短い形: 台本が自分で期限切れを返す → 見張りは発火せず
/// （解く手 0 回・打ち切りは偽）、印の判定は今日どおり（`OnClose` の失敗は降ろしの中の応答として
/// 畳まれ、きれいな終わりで印が消える）。
#[test]
fn self_timeout_shorter_than_limit_is_not_cut() {
    let mut rig = boot_a(on_close_times_out_by_itself, None);
    assert_eq!(
        end_session(&mut rig, WIDE, false),
        Observed {
            held_seen: None,
            unblock_calls: 0,
            on_close_ref0: vec!["system".to_owned()],
            mark: None,
            kept_reason: None,
            shiori_cut: Some("false".to_owned()),
        },
        "SHIORI が自分で期限切れを返した回が打ち切り扱いになった・印の判定が変わった"
    );
}

/// 上限なしの降ろし方（`GhostSession::shutdown`）は見張りを張らない: 手動の口の予約を先に置き、
/// `OnClose` で固まったところをテストが外から解いても、解く手はテストの 1 回だけで、予約は
/// 降ろした後まで残っている（テストが張り直すと、その場で予約が発火して解く手が 1 回増える）。
///
/// 張ったかどうかは「予約が残っているか」で見る。上限なしの降ろし方が張っていれば予約はそこで
/// 消費され、見張りが遅れて発火しても段が降り済みなら何もしない（切らない）ので、解く手の回数
/// だけでは張ったことを取りこぼしうる。予約は張った瞬間に消費されるので、残っているかは時刻に依らない。
///
/// # 非空虚性
/// 上限なしの降ろし方が見張りを張ると予約が消費され、テストが張り直しても発火しない（解く手が
/// 増えず、待ちが期限切れになって赤）。
#[test]
fn unbounded_shutdown_never_cuts() {
    let mut rig = boot_held(HoldAt::Notify("OnClose"));
    let handle = rig.handle("A");
    let probe = probe_of(&rig);
    probe.cut_now();
    let releaser = {
        let handle = handle.clone();
        thread::spawn(move || {
            let held = spin_wait_until(|| handle.holding());
            if held {
                handle.release();
            }
            held
        })
    };
    let session = rig
        .world
        .get_non_send_mut::<GhostSlot>()
        .and_then(|mut slot| slot.0.take())
        .expect("置き場に A が在る");
    let _ = session.shutdown(CloseReason::System);
    let held_seen = releaser.join().expect("補助のスレッドは panic しない");
    let calls_after_shutdown = handle.unblock_calls();

    // 降ろした後（shiori のアクターは居ない）に呼び出しの外へ書き戻して張り直す: 予約が残っていれば
    // その場で発火し、段 `idle` で解く手を 1 回呼ぶ。
    probe.set_busy(ShioriBusy::Idle);
    let guard = probe.arm(WaitBudget {
        started: Instant::now(),
        limit: WIDE,
    });
    let reserved_fired = spin_wait_until(|| handle.unblock_calls() > calls_after_shutdown);
    let cut = guard.finish();

    assert_eq!(
        (held_seen, calls_after_shutdown, reserved_fired, cut),
        (
            true,
            1,
            true,
            Some(ShioriCut {
                stage: "idle",
                unblocked: true
            })
        ),
        "上限なしの降ろし方が見張りを張った（固まりを見た・降ろした後の解く手の回数・予約が残って\
         いて張り直しで発火した・その結果）"
    );
}

/// 上限の定数は 3 秒（2026-09-27 開発者の確定・要件 1.2・8.1）。
#[test]
fn session_end_limit_is_three_seconds() {
    assert_eq!(SESSION_END_SHIORI_LIMIT, Duration::from_secs(3));
}

// ============================ 期限つきの降ろしと見張りの張りを呼ぶ所（要件 3.3・7.2）

/// 走査の字面: 期限つきの降ろし・見張りの張り・上限を受ける入口の呼び出し。
const DEADLINE_CALL_TOKENS: [&str; 3] = [".shutdown_within(", ".arm(", "end_session_within("];

/// 本番ソースでそれらを呼んでよい行（`src/` からの相対パス・前後の空白を落とした行）。
/// OS のセッションの終了の受け手が定数を渡す 1 行・受け手の降ろし・`GhostSession` の委譲の 3 行だけ。
const ALLOWED_DEADLINE_CALLS: [(&str, &str); 3] = [
    (
        "ghost_session.rs",
        "let (result, fired) = runtime.shutdown_within(reason, budget);",
    ),
    (
        "session_end.rs",
        "end_session_within(world, SESSION_END_SHIORI_LIMIT)",
    ),
    (
        "session_end.rs",
        "session.shutdown_within(CloseReason::System, WaitBudget { started, limit });",
    ),
];

/// `crates/areka/src/` の本番 `.rs`（テストとテストの土台＝`_tests.rs`・`_support.rs` を除く）を
/// `(src/ からの相対パス, 本文)` で集める。
fn production_sources() -> Vec<(String, String)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = std::fs::read_dir(dir).unwrap_or_else(|e| {
            panic!("木を歩けない（走査が空振りする）: {} — {e}", dir.display())
        });
        for entry in entries {
            let path = entry.expect("ディレクトリ項目が読めない").path();
            if path.is_dir() {
                walk(root, &path, out);
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.ends_with(".rs")
                || name.ends_with("_tests.rs")
                || name.ends_with("_support.rs")
            {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .expect("走査の根の下に無い")
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&path).expect("本番ファイルが読めない");
            out.push((rel, src));
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out
}

/// 走査の当たり: 字面を含む行（註釈の行と関数の定義の行を除く）を `(相対パス, 行)` で集める。
fn deadline_call_sites(files: &[(String, String)]) -> BTreeSet<(String, String)> {
    files
        .iter()
        .flat_map(|(rel, src)| {
            src.lines()
                .map(str::trim)
                .filter(|line| !line.starts_with("//") && !line.contains("fn "))
                .filter(|line| DEADLINE_CALL_TOKENS.iter().any(|t| line.contains(t)))
                .map(move |line| (rel.clone(), line.to_owned()))
        })
        .collect()
}

/// 当たりと許可の差（表に無い当たり・当たりの無い表の行）。
fn deadline_call_mismatch(
    found: &BTreeSet<(String, String)>,
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let allowed: BTreeSet<(String, String)> = ALLOWED_DEADLINE_CALLS
        .iter()
        .map(|(rel, line)| ((*rel).to_owned(), (*line).to_owned()))
        .collect();
    (
        found.difference(&allowed).cloned().collect(),
        allowed.difference(found).cloned().collect(),
    )
}

/// 本番ソースで期限つきの降ろしと見張りの張りを呼ぶのは、OS のセッションの終了の受け手と
/// `GhostSession` の委譲だけ（表に無い当たりも、当たりの無い表の行も赤）。
#[test]
fn only_session_end_calls_the_bounded_shutdown() {
    let found = deadline_call_sites(&production_sources());
    assert_eq!(
        deadline_call_mismatch(&found),
        (vec![], vec![]),
        "期限つきの降ろし・見張りの張りの呼び出しが許可表と合わない（表に無い当たり・当たりの無い表の行）: {found:?}"
    );
}

/// 走査の較正: 本番ソースの写しから許可の行を 1 行消すと「当たりの無い表の行」、見張りの張りを
/// 1 行足すと「表に無い当たり」で赤になる（検査が空振りしていない）。
#[test]
fn deadline_call_scan_turns_red_on_one_line_removed_or_added() {
    let files = production_sources();

    let removed: Vec<(String, String)> = files
        .iter()
        .map(|(rel, src)| {
            let src = if rel == "session_end.rs" {
                src.replace("end_session_within(world, SESSION_END_SHIORI_LIMIT)", "")
            } else {
                src.clone()
            };
            (rel.clone(), src)
        })
        .collect();
    let (extra, missing) = deadline_call_mismatch(&deadline_call_sites(&removed));
    assert_eq!(
        (extra, missing),
        (
            vec![],
            vec![(
                "session_end.rs".to_owned(),
                "end_session_within(world, SESSION_END_SHIORI_LIMIT)".to_owned()
            )]
        ),
        "許可の行を 1 行消しても赤にならない"
    );

    let mut added = files.clone();
    added.push((
        "main.rs".to_owned(),
        "let guard = probe.arm(budget);".to_owned(),
    ));
    let (extra, missing) = deadline_call_mismatch(&deadline_call_sites(&added));
    assert_eq!(
        (extra, missing),
        (
            vec![(
                "main.rs".to_owned(),
                "let guard = probe.arm(budget);".to_owned()
            )],
            vec![]
        ),
        "見張りの張りを 1 行足しても赤にならない"
    );
}
