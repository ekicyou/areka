//! snapshot_capture_test.rs — 実機からの代表応答を交信記録機構経由で採取するハーネス
//! （task 6.1・design.md「snapshot 採取ハーネス（env-gate）」・要件 2.6/6.1）。
//!
//! 本ハーネスは**出力形式生成ロジックを純関数**として切り出し（`reconstruct_envelope`／
//! `collect_first_get_snapshots`）、実機なしで単体テスト可能にする（観測可能な完了＝
//! これらが単体テストで green）。実 pasta 越しの実採取は `HOST32_PASTA_DLL`＋
//! `AREKA_SNAPSHOT_OUT` の**両方**が設定された場合のみ動作し、いずれか欠落（または空文字）時は
//! 静かにスキップする（既存 env-gate 流儀・`real_pasta_test.rs` を踏襲）。
//!
//! # 採取フロー（design.md 「Batch / Job Contract」）
//! 検体 `emo2` の使い捨ての複製に実 pasta DLL を持ち込み（`real_pasta_test.rs` と同じ形）→
//! `ShioriWiring::Custom(Recorder(real_connect(helper_exe, mount.shiori, DefaultEncoding::Ansi)))`
//! で boot→talk→close
//! を一周（実時間可・本ハーネスは決定論を要求されない）→ 記録から **GET 交信の ID ごと初出**
//! を取り、`Value(s)`→`SHIORI/3.0 200 OK`＋`Charset: UTF-8`＋`Value: <s>` の正準 envelope、
//! `NoContent`→204 envelope へ再構成し `AREKA_SNAPSHOT_OUT/<ID>.txt` へ書き出す。
//!
//! # narrowing（design.md「narrowing（要件 2.2 との対応）」）
//! スナップショット対象は **GET 交信のみ**。NOTIFY／Unload は採取対象外（応答がワイヤ契約上
//! 破棄される・または unload 結果ゆえ）。GET の `Value`／`NoContent` のみ再構成し、その他の
//! outcome（`Failed`／`NotifyOk`／`Unloaded`）は `None`（採取不能）として除外する。
//!
//! # 出力の正準形式（`crates/shiori4-testdll/snapshots/OnFirstBoot.txt` と一致）
//! 200: `SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: <s>\r\n\r\n`
//! 204: `SHIORI/3.0 204 No Content\r\n\r\n`
//! **`X-Areka-Snapshot: PROVISIONAL` マーカは付けない**（実採取は非 provisional・凍結コミット時に
//! そのまま `snapshots/` へ drop-in できる正準形。task 6.2 の凍結コミットが差し替え点）。

use std::collections::HashSet;

use crate::recorder::{ExchangeKind, ExchangeOutcome, ExchangeRecord};

/// 1 交信の結果（`ExchangeOutcome`）を SHIORI/3.0 応答の正準 envelope へ再構成する純関数
/// （出力形式生成ロジックの単一権威・実機なしで単体検証可能）。
///
/// GET の `Value`／`NoContent` のみ採取可能（`Some`）。NOTIFY/Unload/失敗（`Failed`／
/// `NotifyOk`／`Unloaded`）は採取不能ゆえ `None`（narrowing: GET only）。
///
/// - `Value(s)` → `SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: {s}\r\n\r\n`
/// - `NoContent` → `SHIORI/3.0 204 No Content\r\n\r\n`
///
/// CRLF 行末・`Charset: UTF-8`・`Value:` 行・空行終端で `snapshots/OnFirstBoot.txt`
/// の正準形式と一致する（PROVISIONAL マーカは含めない）。
fn reconstruct_envelope(outcome: &ExchangeOutcome) -> Option<String> {
    match outcome {
        ExchangeOutcome::Value(s) => Some(format!(
            "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: {s}\r\n\r\n"
        )),
        ExchangeOutcome::NoContent => Some("SHIORI/3.0 204 No Content\r\n\r\n".to_string()),
        // narrowing（GET only）: NOTIFY 成功・失敗・unload 結果は採取対象外。
        ExchangeOutcome::Failed(_) | ExchangeOutcome::NotifyOk | ExchangeOutcome::Unloaded(_) => {
            None
        }
    }
}

/// 交信記録列から、GET 交信の **ID ごと初出**の応答のみを正準 envelope へ再構成して
/// `(id, envelope)` 列で返す純関数（first-seen 順・決定論）。
///
/// - `kind == Get` かつ `id` を持つ記録のみ対象。NOTIFY/Unload は除外。
/// - 各 ID は**初出のみ**採用（後続の重複 GET は無視・dedup は id 単位で初出時に確定）。
/// - 初出 GET を `reconstruct_envelope` にかけ、`Some(env)` のときだけ `(id, env)` を emit
///   （GET でも `Value`／`NoContent` 以外の outcome は出力しない）。
fn collect_first_get_snapshots(records: &[ExchangeRecord]) -> Vec<(String, String)> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<(String, String)> = Vec::new();
    for record in records {
        if record.kind != ExchangeKind::Get {
            continue;
        }
        let Some(id) = record.id.as_ref() else {
            continue;
        };
        // 初出のみ採用（id 単位の dedup は初出時点で確定・後続重複は無視）。
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(env) = reconstruct_envelope(&record.outcome) {
            out.push((id.clone(), env));
        }
    }
    out
}

// ===========================================================================
// env-gate 実採取（`HOST32_PASTA_DLL`＋`AREKA_SNAPSHOT_OUT` 両設定時のみ動作）。
// 実採取の道（`capture_real_pasta_snapshots`）は env 未設定でもコンパイルインされる（実行時のみ
// silent skip）。
// ===========================================================================

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use areka_ghost::shiori_wiring::real_connect;
use areka_ghost::ticker::TickerConfig;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::ShioriBackend;
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package;
use sample_ghost_kit::SampleRoot;

use crate::common::{resolve_helper_exe, run_bounded_shutdown};
use crate::real_pasta_test::{OBSERVE_TIMEOUT, SHUTDOWN_BOUND, wait_for_a_cue};
use crate::recorder::{BoxedBackend, Recorder, RecorderHandle};
use crate::spine_e2e_test::RecordingSink;

/// env-gate 実採取（design.md「snapshot 採取ハーネス（env-gate）」・要件 2.6/6.1）。
///
/// `HOST32_PASTA_DLL` と `AREKA_SNAPSHOT_OUT` の**両方**が設定（かつ非空）された場合のみ動作し、
/// いずれか欠落・空文字なら静かにスキップして PASS する（常設 `cargo test --workspace` を
/// fail/hang させない）。両設定時は実 32bit helper＋実 pasta 越しに `Custom(Recorder(real_connect))`
/// で boot→talk→close を一周し、記録から GET 初出応答を正準 envelope へ再構成して
/// `AREKA_SNAPSHOT_OUT/<ID>.txt` へ書き出す（上書き・冪等）。
#[test]
fn capture_real_pasta_snapshots() {
    // --- env-gate: 両方揃い、かつ非空のときだけ動作（いずれか欠落・空は silent skip・PASS）---
    let (Ok(pasta_dll), Ok(out_dir)) = (
        std::env::var("HOST32_PASTA_DLL"),
        std::env::var("AREKA_SNAPSHOT_OUT"),
    ) else {
        eprintln!(
            "snapshot capture skipped (env not set): HOST32_PASTA_DLL / AREKA_SNAPSHOT_OUT の\
             いずれかが未設定のため実採取を skip（env-gate・要件 2.6/6.1）。"
        );
        return;
    };
    if pasta_dll.trim().is_empty() || out_dir.trim().is_empty() {
        eprintln!(
            "snapshot capture skipped (env not set): HOST32_PASTA_DLL / AREKA_SNAPSHOT_OUT の\
             いずれかが空文字のため実採取を skip（env-gate・要件 2.6/6.1）。"
        );
        return;
    }

    // --- 前提の明示 fail（silent skip は env 欠落のみ・以降の不備は genuine failure）---
    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（指定 DLL 不在は明示 fail）"
    );
    let helper_exe = resolve_helper_exe().expect("i686 helper exe の解決に失敗");
    let out_path = PathBuf::from(&out_dir);
    std::fs::create_dir_all(&out_path)
        .expect("AREKA_SNAPSHOT_OUT ディレクトリを作成できませんでした");

    // --- 起動記録の無い新品の emo2（辞書込み・`profile` を持たない）に指定の DLL を持ち込む ---
    let sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let root = sample.folder().to_path_buf();
    std::fs::copy(
        &dll_path,
        root.join("ghost").join("master").join("pasta.dll"),
    )
    .expect("HOST32_PASTA_DLL を emo2 の ghost/master へ写せませんでした");

    // boot が内部で通すのと同一契約で mount を自前解決し、`mount.shiori` を real_connect へ渡す。
    let mount = package::resolve_with_shell(&root, DefaultEncoding::Ansi, None)
        .expect("採取フィクスチャは実マウント解決を成功裏に通過すべき（boot 内部と同一契約）");

    // --- 実 backend へ Recorder を合成した Custom wiring（design.md「Batch/Job Contract」）---
    let handle_slot: Arc<Mutex<Option<RecorderHandle>>> = Arc::new(Mutex::new(None));
    let slot2 = Arc::clone(&handle_slot);
    // SHIORI 通信の既定も**本番と同じ** `Ansi`（＝Shift_JIS）を渡す。ここを UTF-8 にすると
    // 本番と違う最初の要求を採ってしまう（design §Boundary Commitments）。
    let connect = real_connect(helper_exe, mount.shiori, DefaultEncoding::Ansi);
    let wiring = ShioriWiring::Custom(Box::new(move || {
        // 実 pasta backend を確立（shiori アクタースレッド上で helper spawn＋LOAD）。
        let inner = connect()?;
        let (recorder, handle) = Recorder::new(BoxedBackend(inner));
        *slot2.lock().expect("recorder handle slot poisoned") = Some(handle);
        Ok(Box::new(recorder) as Box<dyn ShioriBackend>)
    }));

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

    let options = GhostBootOptions {
        strict_judges: None,
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Ansi,
        shiori: wiring,
        sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
        system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
        app_profile_dir: None,
        // 実時計駆動（本ハーネスは決定論を要求されない・OnBoot 一周を実運行で起こす）。
        ticker: TickerMode::Real(TickerConfig::default()),
    };

    let runtime =
        boot(options).expect("real pasta 越しの boot は DLL/helper 実在確認済みなら成功するはず");

    // --- OnBoot 一周で sink に何らかの発火が観測されるまで有界待機（実時計ポーリング可）---
    // 何も見えないまま 204 の写しを書いて緑にしない。
    assert!(
        wait_for_a_cue(&surface_records, &text_records),
        "実 pasta 越しの OnBoot 一周が {OBSERVE_TIMEOUT:?} 以内に RecordingSink へ何も記録しなかった"
    );

    // --- 正規終了（close 系列で OnClose NOTIFY→Unload まで記録を確定させる）---
    run_bounded_shutdown("snapshot capture shutdown", SHUTDOWN_BOUND, runtime);

    // --- 記録から GET 初出を正準 envelope へ再構成しファイル出力（上書き・冪等）---
    let records = handle_slot
        .lock()
        .expect("recorder handle slot poisoned")
        .as_ref()
        .expect("connect closure が RecorderHandle をスロットへ格納しているはず（shiori actor spawn 済み）")
        .records();

    let snapshots = collect_first_get_snapshots(&records);
    for (id, envelope) in &snapshots {
        let file = out_path.join(format!("{id}.txt"));
        std::fs::write(&file, envelope).unwrap_or_else(|e| {
            panic!("スナップショット {} の書き出しに失敗: {e}", file.display())
        });
    }
    eprintln!(
        "snapshot capture: {} 件の GET 初出応答を {} へ書き出した（正準 envelope・上書き）。",
        snapshots.len(),
        out_path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ExchangeRecord` を素直に組み立てる小ヘルパ（フィールドは全て pub・synthetic 記録用）。
    fn get_record(id: &str, outcome: ExchangeOutcome) -> ExchangeRecord {
        ExchangeRecord {
            kind: ExchangeKind::Get,
            id: Some(id.to_string()),
            references: Vec::new(),
            status: None,
            outcome,
        }
    }

    fn notify_record(id: &str) -> ExchangeRecord {
        ExchangeRecord {
            kind: ExchangeKind::Notify,
            id: Some(id.to_string()),
            references: Vec::new(),
            status: None,
            outcome: ExchangeOutcome::NotifyOk,
        }
    }

    fn unload_record() -> ExchangeRecord {
        ExchangeRecord {
            kind: ExchangeKind::Unload,
            id: None,
            references: Vec::new(),
            status: None,
            outcome: ExchangeOutcome::Unloaded("Ok(Clean)".to_string()),
        }
    }

    #[test]
    fn reconstruct_value_produces_canonical_200_envelope() {
        let env = reconstruct_envelope(&ExchangeOutcome::Value(r"\0\s[0]おはよう\e".to_string()))
            .expect("Value は 200 envelope へ再構成される");
        assert_eq!(
            env, "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: \\0\\s[0]おはよう\\e\r\n\r\n",
            "200 envelope は status line＋Charset UTF-8＋Value 行＋空行終端（CRLF・PROVISIONAL マーカ無し）"
        );
    }

    #[test]
    fn reconstruct_no_content_produces_canonical_204_envelope() {
        let env = reconstruct_envelope(&ExchangeOutcome::NoContent)
            .expect("NoContent は 204 envelope へ再構成される");
        assert_eq!(
            env, "SHIORI/3.0 204 No Content\r\n\r\n",
            "204 envelope は status line＋空行終端（CRLF・PROVISIONAL マーカ無し）"
        );
    }

    #[test]
    fn reconstruct_non_snapshottable_outcomes_are_none() {
        assert_eq!(
            reconstruct_envelope(&ExchangeOutcome::Failed("boom".to_string())),
            None,
            "Failed は採取不能（None）"
        );
        assert_eq!(
            reconstruct_envelope(&ExchangeOutcome::NotifyOk),
            None,
            "NotifyOk は GET 応答でない（None）"
        );
        assert_eq!(
            reconstruct_envelope(&ExchangeOutcome::Unloaded("Ok(Clean)".to_string())),
            None,
            "Unloaded は GET 応答でない（None）"
        );
    }

    #[test]
    fn collect_takes_first_get_per_id_and_excludes_notify_and_unload() {
        // boot 一周を模した synthetic 記録列（NOTIFY／Unload 混在・OnBoot は重複）。
        let records = vec![
            notify_record("OnInitialize"),
            get_record("OnFirstBoot", ExchangeOutcome::NoContent),
            get_record("OnBoot", ExchangeOutcome::Value("first".to_string())),
            notify_record("basewareversion"),
            // 重複 GET（初出優先ゆえ無視される）。
            get_record("OnBoot", ExchangeOutcome::Value("second".to_string())),
            unload_record(),
        ];

        let snapshots = collect_first_get_snapshots(&records);

        let expected = vec![
            (
                "OnFirstBoot".to_string(),
                "SHIORI/3.0 204 No Content\r\n\r\n".to_string(),
            ),
            (
                "OnBoot".to_string(),
                "SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: first\r\n\r\n".to_string(),
            ),
        ];
        assert_eq!(
            snapshots, expected,
            "GET 初出のみ・GET only（NOTIFY/Unload 除外）・重複 OnBoot は初出（first）採用・first-seen 順"
        );
    }

    #[test]
    fn collect_skips_get_with_non_snapshottable_outcome() {
        // GET だが outcome が Failed（採取不能）→ 出力から除外される。
        let records = vec![
            get_record("OnBoot", ExchangeOutcome::Failed("timeout".to_string())),
            get_record("OnFirstBoot", ExchangeOutcome::NoContent),
        ];
        let snapshots = collect_first_get_snapshots(&records);
        assert_eq!(
            snapshots,
            vec![(
                "OnFirstBoot".to_string(),
                "SHIORI/3.0 204 No Content\r\n\r\n".to_string(),
            )],
            "GET でも Value/NoContent 以外の outcome は採取対象外（初出 dedup は成立するが出力しない）"
        );
    }
}
