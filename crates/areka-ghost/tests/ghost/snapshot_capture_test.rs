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
//! 実 pasta fixture を組立（`real_pasta_test.rs::write_real_pasta_ghost_fixture` 流儀）→
//! `ShioriWiring::Custom(Recorder(real_connect(helper_exe, mount.shiori)))` で boot→talk→close
//! を一周（実時間可・本ハーネスは決定論を要求されない）→ 記録から **GET 交信の ID ごと初出**
//! を取り、`Value(s)`→`SHIORI/3.0 200 OK`＋`Charset: UTF-8`＋`Value: <s>` の正準 envelope、
//! `NoContent`→204 envelope へ再構成し `AREKA_SNAPSHOT_OUT/<ID>.txt` へ書き出す。
//!
//! # narrowing（design.md「narrowing（要件 2.2 との対応）」）
//! スナップショット対象は **GET 交信のみ**。NOTIFY／Unload は採取対象外（応答がワイヤ契約上
//! 破棄される・または unload 結果ゆえ）。GET の `Value`／`NoContent` のみ再構成し、その他の
//! outcome（`Failed`／`NotifyOk`／`Unloaded`）は `None`（採取不能）として除外する。
//!
//! # 出力の正準形式（`crates/shiori4-testdll/snapshots/OnBoot.txt`／`OnFirstBoot.txt` と一致）
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
/// CRLF 行末・`Charset: UTF-8`・`Value:` 行・空行終端で `snapshots/OnBoot.txt`／
/// `OnFirstBoot.txt` の正準形式と一致する（PROVISIONAL マーカは含めない）。
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
// 以下の項目（capture_env / resolve_helper_exe / unique_temp_dir /
// write_real_pasta_ghost_fixture / BoxedBackend / run_bounded_shutdown）は
// `capture_real_pasta_snapshots` の実採取パスから参照される。env 未設定でも
// コードパスはコンパイルインされる（実行時のみ silent skip）。
// ===========================================================================

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_ghost::shiori_wiring::real_connect;
use areka_ghost::ticker::TickerConfig;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::{CloseReason, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package;

use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use temp_path_kit::TempPath;

use crate::recorder::{Recorder, RecorderHandle};
use crate::spine_e2e_test::RecordingSink;

/// OnBoot 一周で sink に発火が観測されるまでの有界待機上限（実プロセス・実ブレイン往復ゆえ
/// 秒単位の余裕・`real_pasta_test.rs::OBSERVE_TIMEOUT` と同旨）。本ハーネスは決定論を要求
/// されないため実時計待機で足りる。
const OBSERVE_TIMEOUT: Duration = Duration::from_secs(60);

/// 有界待機の再試行間隔（`real_pasta_test.rs::POLL_INTERVAL` と同旨・`TickerMode::Real` が
/// 背後で実 wall-clock 駆動するため本ハーネス限定で実 sleep ポーリングを許容する）。
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// `shutdown()` 呼出を有界に観測する上限（`real_pasta_test.rs::SHUTDOWN_BOUND` と同旨）。
const SHUTDOWN_BOUND: Duration = Duration::from_secs(30);

/// `inproc_e2e_test.rs::BoxedBackend` と同型の薄い委譲アダプタ（`Box<dyn ShioriBackend>` は
/// `ShioriBackend` を実装せず孤児則で blanket impl も足せないため newtype で橋渡しする）。
/// 全呼出を内側の実 backend へ素通しし、記録は外側の `Recorder` が担う。
struct BoxedBackend(Box<dyn ShioriBackend>);

impl ShioriBackend for BoxedBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.0.get(id, references, status)
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.0.notify(id, references, status)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.0.unload()
    }

    fn status(&mut self) -> HelperStatus {
        self.0.status()
    }
}

/// i686 helper 実行ファイルのパスを解決する（`real_pasta_test.rs::resolve_helper_exe` を
/// 逐語的に踏襲——env `HOST32_HELPER_EXE` 最優先 → ワークスペース
/// `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`）。
fn resolve_helper_exe() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("HOST32_HELPER_EXE") {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "HOST32_HELPER_EXE={explicit:?} が指すファイルが存在しません（i686 helper を先にビルド）"
        ));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.clone());
    let target_base = workspace_root.join("target").join("i686-pc-windows-msvc");

    for profile in ["debug", "release"] {
        let candidate = target_base.join(profile).join("shiori-host32-helper.exe");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "i686 helper exe が見つかりません（探索先: {}\\{{debug,release}}\\shiori-host32-helper.exe）。\
         PowerShell で先に i686 helper をビルドしてください: \
         cargo build -p shiori-host32-helper --target i686-pc-windows-msvc",
        target_base.display()
    ))
}

/// このテスト専用の一時ディレクトリ。共通窓口 `temp-path-kit` 経由で組むので、
/// 名前にプロセス識別子と連番が入り**プロセス間でも一意**（同じテストを同時に
/// 複数プロセスで走らせても互いの一時ファイルを消し合わない）。
///
/// 返り値が生きている間だけ実体が存在し、破棄で中身ごと消える。
fn unique_temp_dir(tag: &str) -> TempPath {
    TempPath::new(&format!("ghost-snapshot-capture-{tag}"))
}

/// `root` 直下に、実 pasta DLL を物理的に持ち込んだ `ghost/master/` を含む解決可能な
/// ゴーストツリーを構築する（`real_pasta_test.rs::write_real_pasta_ghost_fixture` を踏襲——
/// `ShioriMount.dir` は常に `ghost_root/ghost/master` に固定されるため実 DLL をフィクスチャ側へ
/// コピーして持ち込む）。
/// `src` 直下の全エントリ（ファイル・サブディレクトリ）を `dst` へ再帰コピーする
/// （pasta の支援資産一式を fixture の load_dir へ持ち込むため・task 6.2）。
fn copy_dir_contents_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create dst dir");
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let target = dst.join(&name);
        if path.is_dir() {
            copy_dir_contents_recursive(&path, &target);
        } else if path.is_file() {
            let _ = std::fs::copy(&path, &target);
        }
    }
}

fn write_real_pasta_ghost_fixture(root: &Path, dll_path: &Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let dll_file_name = dll_path
        .file_name()
        .expect("HOST32_PASTA_DLL にファイル名がない")
        .to_string_lossy()
        .into_owned();
    // 実 pasta は応答（トーク）に自身の支援資産（`dic/`・`scripts/`・`pasta.toml` 等）を
    // load_dir（=ghost/master）から読む。DLL 単体では OnBoot にスクリプトを返せないため、
    // DLL の親ディレクトリ（＝実 emo2 の `ghost/master/`）の中身を丸ごと再帰コピーして
    // pasta へフル環境を与える（task 6.2・実採取が talk を得るために必須）。
    if let Some(src_dir) = dll_path.parent() {
        copy_dir_contents_recursive(src_dir, &ghost_master);
    }
    // DLL は上のコピーに含まれるが、親不明時の保険として単体コピーも冪等に行う。
    let dll_dst = ghost_master.join(&dll_file_name);
    if !dll_dst.is_file() {
        std::fs::copy(dll_path, &dll_dst)
            .expect("実 pasta DLL を ghost/master へコピーできませんでした");
    }

    std::fs::write(
        ghost_master.join("descript.txt"),
        format!(
            "charset,UTF-8\nname,SnapshotCaptureGhost\nshiori,{dll_file_name}\n\
             seriko.defaultsurfacedirectoryname,master\n"
        ),
    )
    .expect("write ghost descript.txt");

    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,SnapshotCaptureShell\n",
    )
    .expect("write shell descript.txt");
}

/// `shutdown` を別スレッドへ逃がして有界に完走させる（宙吊り防止・`real_pasta_test.rs` の
/// shutdown 観測と同旨）。
fn run_bounded_shutdown(runtime: areka_ghost::GhostRuntime) {
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<Result<(), String>>(0);
    std::thread::spawn(move || {
        let result = runtime
            .shutdown(CloseReason::System)
            .map_err(|err| format!("{err:?}"));
        let _ = done_tx.send(result);
    });
    match done_rx.recv_timeout(SHUTDOWN_BOUND) {
        Ok(result) => assert!(
            result.is_ok(),
            "実 pasta 越しの shutdown は Ok(()) を返すはず（採取一周の正規終了）: {result:?}"
        ),
        Err(_) => panic!("shutdown が {SHUTDOWN_BOUND:?} 以内に完了しなかった（宙吊りの疑い）"),
    }
}

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

    // --- 実 pasta fixture を組立（DLL をフィクスチャ ghost/master へ持ち込む）---
    let temp = unique_temp_dir("capture-real-pasta-snapshots");
    let root = temp.path().to_path_buf();
    write_real_pasta_ghost_fixture(&root, &dll_path);

    // boot が内部で通すのと同一契約で mount を自前解決し、`mount.shiori` を real_connect へ渡す。
    let mount = package::resolve(&root, DefaultEncoding::Utf8)
        .expect("採取フィクスチャは実マウント解決を成功裏に通過すべき（boot 内部と同一契約）");

    // --- 実 backend へ Recorder を合成した Custom wiring（design.md「Batch/Job Contract」）---
    let handle_slot: Arc<Mutex<Option<RecorderHandle>>> = Arc::new(Mutex::new(None));
    let slot2 = Arc::clone(&handle_slot);
    let connect = real_connect(helper_exe, mount.shiori);
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
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
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
    let deadline = Instant::now() + OBSERVE_TIMEOUT;
    while Instant::now() < deadline {
        let surface_non_empty = !surface_records
            .lock()
            .expect("surface records mutex poisoned")
            .is_empty();
        let text_non_empty = !text_records
            .lock()
            .expect("text records mutex poisoned")
            .is_empty();
        if surface_non_empty || text_non_empty {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    // --- 正規終了（close 系列で OnClose NOTIFY→Unload まで記録を確定させる）---
    run_bounded_shutdown(runtime);

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
