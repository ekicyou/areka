//! 決定論 spine e2e（design.md「spine e2e（決定論・純 x64）」）。
//!
//! 本ファイルは 2 つのテスト専用型を提供する（task 4.1 の成果物）:
//! - [`ScriptedShioriBackend`] — `areka_kanade::ShioriBackend` を実装する台本 fake。
//! - [`RecordingSink`] — `areka_sakura::sink::{SurfaceSink, TextSink}` を実装する、
//!   `Clone` 可能な記録 sink。
//!
//! 後続タスク（4.2〜4.7）はこのファイルへ boot〜close の各シナリオ（S1〜S6）の
//! `#[test]` を追加していく。本タスク（4.1）はその土台となる 2 型自体の構築・検証
//! （台本通りの応答・終了結果・死活遷移を任意に再現できること）のみを担う。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use areka_kanade::ShioriBackend;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_sakura::sink::{SurfaceSink, TextSink};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

// ===================== ScriptedShioriBackend =====================

/// backend が受領した 1 呼出の記録（照合用・要件 7.1「発火内容を蓄積して照合できる」）。
///
/// `Get`/`Notify` は id・references を保持する（task-spec の「id + references 最低限」）。
/// `Unload`/`Status` は引数を持たないため variant のみで足りる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedCall {
    /// GET 呼出（応答を要するイベント）。
    Get { id: String, references: Vec<String> },
    /// NOTIFY 呼出（片道イベント）。
    Notify { id: String, references: Vec<String> },
    /// unload（正規 clean shutdown）呼出。
    Unload,
    /// status（非ブロッキング死活問い合わせ）呼出。
    Status,
}

/// [`ScriptedShioriBackend`] を組み立てるビルダー（台本の事前登録）。
///
/// GET/NOTIFY は id ごとに応答列（`VecDeque`）を積み上げ、呼出のたびに先頭から 1 件
/// 消費する（`RequestError`/`ShutdownError` は `Clone` を実装しないため、値そのものを
/// 使い切り消費する設計にする——スクリプトの再利用は想定しない）。
pub struct ScriptedShioriBackendBuilder {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    initial_status: HelperStatus,
}

impl ScriptedShioriBackendBuilder {
    /// 空の台本（既定 status=`Running`）から開始する。
    pub fn new() -> Self {
        Self {
            get_scripts: HashMap::new(),
            notify_scripts: HashMap::new(),
            unload_script: None,
            initial_status: HelperStatus::Running,
        }
    }

    /// `id` に対する GET 応答を 1 件、応答列の末尾へ積む（複数回呼べば FIFO に消費される）。
    pub fn get(mut self, id: impl Into<String>, response: Result<Option<String>, RequestError>) -> Self {
        self.get_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `id` に対する NOTIFY 応答を 1 件、応答列の末尾へ積む。
    pub fn notify(mut self, id: impl Into<String>, response: Result<(), RequestError>) -> Self {
        self.notify_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `unload()` の結果を台本化する（一度きり消費・`Option::take` で払い出す）。
    pub fn unload(mut self, response: Result<ExitKind, ShutdownError>) -> Self {
        self.unload_script = Some(response);
        self
    }

    /// 初期 `status()` を台本化する（既定は `Running`）。
    pub fn status(mut self, status: HelperStatus) -> Self {
        self.initial_status = status;
        self
    }

    /// backend 本体（アクタースレッドへ move する側）と、テストが状態変更・照合に使う
    /// [`ScriptedShioriHandle`] のペアを構築する。
    pub fn build(self) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        let status = Arc::new(Mutex::new(self.initial_status));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let backend = ScriptedShioriBackend {
            get_scripts: self.get_scripts,
            notify_scripts: self.notify_scripts,
            unload_script: self.unload_script,
            status: Arc::clone(&status),
            calls: Arc::clone(&calls),
        };
        let handle = ScriptedShioriHandle { status, calls };
        (backend, handle)
    }
}

impl Default for ScriptedShioriBackendBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 台本化したテスト専用 SHIORI backend（`areka_kanade::ShioriBackend` 実装・要件 7.1/7.6）。
///
/// プロセス spawn・実窓・i686 成果物を一切要さない（純 x64・要件 7.6）。応答・終了結果は
/// [`ScriptedShioriBackendBuilder`] で事前登録し、`status()` は [`ScriptedShioriHandle`]
/// 経由でテスト自身のスレッドから途中差し替え可能（helper がシナリオ途中で死ぬ様子を
/// 再現するための capability・後続タスクの S3 が利用する）。
pub struct ScriptedShioriBackend {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    status: Arc<Mutex<HelperStatus>>,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriBackend {
    /// ビルダー起点（[`ScriptedShioriBackendBuilder::new`] の別名）。
    pub fn builder() -> ScriptedShioriBackendBuilder {
        ScriptedShioriBackendBuilder::new()
    }
}

impl ShioriBackend for ScriptedShioriBackend {
    fn get(&mut self, id: &str, references: &[String]) -> Result<Option<String>, RequestError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Get {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.get_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::get(\"{id}\"): no scripted response left (never configured or script exhausted)")
            })
    }

    fn notify(&mut self, id: &str, references: &[String]) -> Result<(), RequestError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Notify {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.notify_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::notify(\"{id}\"): no scripted response left (never configured or script exhausted)")
            })
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Unload);
        self.unload_script
            .take()
            .unwrap_or_else(|| panic!("ScriptedShioriBackend::unload(): no scripted response configured"))
    }

    fn status(&mut self) -> HelperStatus {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Status);
        *self.status.lock().expect("status mutex poisoned")
    }
}

/// [`ScriptedShioriBackend`] をテスト側から観測・操作するためのハンドル。
///
/// backend 本体（`Box<dyn ShioriBackend>` としてアクタースレッドへ move される）とは
/// 独立に、`Arc` 共有を通じて status の途中差し替え・呼出記録の照合を行える。
#[derive(Clone)]
pub struct ScriptedShioriHandle {
    status: Arc<Mutex<HelperStatus>>,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriHandle {
    /// `status()` が以後返す値を差し替える（helper がシナリオ途中で死ぬ様子の再現）。
    /// テスト自身のスレッドから、backend が別スレッド（shiori actor）で生きている間に
    /// 呼べる。
    pub fn set_status(&self, status: HelperStatus) {
        *self.status.lock().expect("status mutex poisoned") = status;
    }

    /// 受領記録（`Arc` クローン）を返す。backend を別スレッドへ move した後も、
    /// このハンドルから発火列を照合できる。
    pub fn calls(&self) -> Arc<Mutex<Vec<RecordedCall>>> {
        Arc::clone(&self.calls)
    }
}

// ===================== RecordingSink =====================

/// テスト専用の `Clone` 可能な記録 sink（`SurfaceSink`/`TextSink` 両方を実装する）。
///
/// sakura の `MockSink`（`tests/ghost/` から見て他クレートの凍結面）とは同型だが
/// `Clone` を実装しない。dispatcher の per-talk 注入（`S: SurfaceSink + Clone`/
/// `T: TextSink + Clone`）を満たすため、`tests/ghost/` 側で定義し直す（sakura の
/// `sink.rs` には手を入れない・design.md 「spine e2e」参照）。`dispatcher.rs` の
/// テストローカル `RecordingSink` と同じ形——ここでは `tests/ghost/` 配下の複数
/// テストファイル（4.2〜4.7）が 1 つの定義を共有できるようにする。
#[derive(Clone)]
pub struct RecordingSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl RecordingSink {
    /// 空の共有蓄積を持つ sink を生成する。
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 共有蓄積の `Arc` クローンを返す。`RecordingSink` を clone してアクタースレッドへ
    /// 渡した後も、このハンドルから発火列を照合できる。
    pub fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }
}

impl Default for RecordingSink {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records.lock().expect("records mutex poisoned").push(cue);
    }
}

impl TextSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records.lock().expect("records mutex poisoned").push(cue);
    }
}

// ===================== 本タスク（4.1）の証明テスト =====================
//
// 「台本通りの応答・終了結果・死活遷移を任意に再現できることを確認できる」
// （tasks.md 4.1 の観測可能な完了条件）を、6 シナリオで直接固定する。

#[cfg(test)]
mod tests {
    use super::*;

    /// シナリオ1: GET 応答（`Ok(Some)`）が台本どおり返り、呼出が記録されること。
    #[test]
    fn scripted_get_ok_some_returns_exact_value_and_is_recorded() {
        let (mut backend, handle) = ScriptedShioriBackend::builder()
            .get("OnBoot", Ok(Some("\\h\\s0hello\\e".to_string())))
            .build();

        let result = backend.get("OnBoot", &[]);

        // `RequestError` は `PartialEq` を実装しないため（凍結面の消費のみ・機械的写像の
        // 都合）、`Result` 全体の `assert_eq!` はできない——`Ok` の中身を直接照合する。
        match result {
            Ok(Some(script)) => assert_eq!(script, "\\h\\s0hello\\e"),
            other => panic!("expected Ok(Some(..)), got {other:?}"),
        }

        let calls = handle.calls();
        let calls = calls.lock().expect("calls mutex poisoned");
        assert_eq!(
            &*calls,
            &vec![RecordedCall::Get {
                id: "OnBoot".to_string(),
                references: vec![],
            }]
        );
    }

    /// シナリオ2: GET 応答として台本化した失敗（`Err(RequestError::Timeout)`）が
    /// そのまま variant 一致で返ること。
    #[test]
    fn scripted_get_err_returns_exact_error_variant() {
        let (mut backend, _handle) = ScriptedShioriBackend::builder()
            .get("OnSecondChange", Err(RequestError::Timeout))
            .build();

        let result = backend.get("OnSecondChange", &[]);

        match result {
            Err(RequestError::Timeout) => {}
            other => panic!("expected Err(RequestError::Timeout), got {other:?}"),
        }
    }

    /// シナリオ3: NOTIFY 応答が台本どおり返り、呼出が記録されること。
    #[test]
    fn scripted_notify_returns_exact_value_and_is_recorded() {
        let (mut backend, handle) = ScriptedShioriBackend::builder()
            .notify("OnCloseAll", Ok(()))
            .build();

        let references = vec!["reason".to_string()];
        let result = backend.notify("OnCloseAll", &references);

        assert!(result.is_ok(), "expected Ok(()), got {result:?}");

        let calls = handle.calls();
        let calls = calls.lock().expect("calls mutex poisoned");
        assert_eq!(
            &*calls,
            &vec![RecordedCall::Notify {
                id: "OnCloseAll".to_string(),
                references,
            }]
        );
    }

    /// シナリオ4: `unload()` の結果（`Ok(ExitKind::Clean)`）が台本どおり返ること。
    #[test]
    fn scripted_unload_returns_exact_exit_kind() {
        let (mut backend, _handle) = ScriptedShioriBackend::builder()
            .unload(Ok(ExitKind::Clean))
            .build();

        let result = backend.unload();

        assert_eq!(result.expect("scripted unload should be Ok"), ExitKind::Clean);
    }

    /// シナリオ5: 死活状態の遷移。初期 `status()` は台本どおり `Running` を返し、その後
    /// テストのスレッドから `handle.set_status` で `Exited(Abnormal(1))` へ差し替えると、
    /// 以降の `status()` 呼出はその新しい値を返す（helper がシナリオ途中で死ぬ様子を
    /// 「backend の外側・テスト自身」から駆動できることの直接証跡・要件 7.1）。
    #[test]
    fn status_transitions_from_running_to_exited_when_mutated_externally_mid_scenario() {
        let (mut backend, handle) = ScriptedShioriBackend::builder()
            .status(HelperStatus::Running)
            .build();

        assert_eq!(backend.status(), HelperStatus::Running);

        // シミュレート: helper がシナリオ途中で異常終了する（テスト自身の駆動）。
        handle.set_status(HelperStatus::Exited(ExitKind::Abnormal(1)));

        assert_eq!(
            backend.status(),
            HelperStatus::Exited(ExitKind::Abnormal(1)),
            "status() 呼出は途中差し替え後の値を反映しなければならない"
        );
    }

    /// シナリオ6: `RecordingSink` の clone 共有蓄積。2 つの clone それぞれから
    /// `SurfaceSink`/`TextSink` 経由で 1 件ずつ emit すると、同一の共有蓄積へ FIFO で
    /// 積まれること（dispatcher の per-talk 注入で clone を渡す使い方を裏付ける）。
    #[test]
    fn recording_sink_clones_share_storage_across_both_sink_traits_in_fifo_order() {
        let sink = RecordingSink::new();
        let records = sink.records();

        let mut surface_clone = sink.clone();
        let mut text_clone = sink.clone();

        let surface_cue = TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("via surface".to_string()),
        };
        let text_cue = TalkCue {
            at: 1.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("via text".to_string()),
        };

        SurfaceSink::emit(&mut surface_clone, surface_cue.clone());
        TextSink::emit(&mut text_clone, text_cue.clone());

        let recorded = records.lock().expect("records mutex poisoned");
        assert_eq!(&*recorded, &vec![surface_cue, text_cue]);
    }
}

// ===================== S1: boot 成功シナリオ（task 4.2） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S1:
// 「Boot→OnBoot GET が Value→StartTalk→sakura 再生→RecordingSink の発火列（at 昇順・
// 内容一致）→TalkDone{Ended} が kanade へ転送される」を、起動から実 ghost スタック
// （kanade→start-relay→dispatcher→sakura の実アクター一式）を通して駆動し、時刻注入
// （Tick）のみで確認する（sleep 不使用・要件 7.2/7.4/7.6・純 x64）。
#[cfg(test)]
mod s1_boot_success {
    use super::*;

    use areka_ghost::dispatcher::DispatcherMsg;
    use areka_ghost::{GhostBootOptions, ShioriWiring, TickerMode, boot};
    use areka_kanade::{KanadeConfig, MonotonicMs, ShioriCall, events};
    use areka_parsers::charset::DefaultEncoding;

    /// このテスト専用の一意な一時ディレクトリ（`runtime.rs`/`config.rs` テストの流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_spine_e2e_s1_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
    /// `shell/master/descript.txt`）を構築する。shell descript の `name` は `shell_name`
    /// （`OnBoot` Ref0・`KanadeConfig::shell_name` の値源と一致させるための既知値・task
    /// 4.2 参照材料 4/5）。
    fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,S1TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            format!("charset,UTF-8\nname,{shell_name}\n").as_bytes(),
        )
        .expect("write shell descript.txt");
    }

    /// events 表由来の [`ShioriCall`] を、このファイル固有の [`RecordedCall`]（task 4.1 の
    /// [`ScriptedShioriBackend`] 記録型）へ変換する（fixture・assert・実装が単一の正本＝
    /// events 表を共有する・Req 7.1）。kanade 自身の統合テストが使う `expected_call`/
    /// `CallMethod` は kanade クレート専用の private 型であり本ファイルからは参照できない
    /// ため、ここで同旨の変換を用意する（task 4.2 参照材料 6 の指示どおり）。
    fn expected_from_shiori_call(call: ShioriCall) -> RecordedCall {
        match call {
            ShioriCall::Get { id, references } => RecordedCall::Get {
                id: id.to_string(),
                references,
            },
            ShioriCall::Notify { id, references } => RecordedCall::Notify {
                id: id.to_string(),
                references,
            },
        }
    }

    /// 有界待機ヘルパ（`runtime.rs`/`dispatcher.rs` テストモジュールと同旨のローカルコピー）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: std::time::Duration, f: F) {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);
        std::thread::spawn(move || {
            f();
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(timeout).is_ok(),
            "'{what}' did not complete within {timeout:?} (possible hang)"
        );
    }

    /// S1: boot 成功——boot→OnBoot(Value)→StartTalk→sakura 再生→RecordingSink の発火列
    /// （at 昇順・内容一致）→TalkDone を、Tick 注入のみで決定論的に確認する
    /// （要件 7.2/7.4/7.6）。
    #[test]
    fn s1_boot_success_plays_greeting_and_records_expected_cue_sequence() {
        const SHELL_NAME: &str = "S1BootShell";

        let root =
            unique_temp_dir("s1_boot_success_plays_greeting_and_records_expected_cue_sequence");
        let _ = std::fs::remove_dir_all(&root);
        write_ghost_fixture(&root, SHELL_NAME);

        // events 表と同一パラメタで期待値導出用 config を構築する（`resolve_kanade_config` が
        // 実際に組み立てる値と shell_name/baseware_version が一致する・task 4.2 参照材料 4）。
        let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

        // boot 系列一式のみを台本化する（OnSecondChange は kanade へ Tick を一切送らないため
        // 不要・OnClose/Unload は本テスト末尾の shutdown() が消費する）。
        let (backend, handle) = ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
            .build();

        let surface_sink = RecordingSink::new();
        let text_sink = RecordingSink::new();
        let surface_records = surface_sink.records();
        let text_records = text_sink.records();

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            surface_sink,
            text_sink,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        // boot() は内部で KanadeMsg::Boot を既に送出済み——boot 系列は kanade アクタースレッド
        // 上で同期往復（oneshot round trip）のみで完走するため、この時点で OnInitialize〜
        // basewareversion の 4 呼出はスケジューリング次第で既に発火し終えている。しかし
        // StartTalk は start_tx→start-relay→dispatcher_tx の 2 hop（別スレッド）を経るため、
        // dispatcher の active slot に talk が実際に載るタイミングはスレッドスケジューリング
        // 依存であり、単一の Tick 送出が必ず間に合う保証はない。sleep は使わず、Tick を送る
        // たびに RecordingSink を確認する再送ループ（実時間待機なし・単調増加する `now` の
        // 注入のみ・`yield_now` で他スレッドに実行機会を譲るだけ）でこの橋渡しをする——
        // script に `\w`（待ち）を含めていないため、dispatcher の active slot に talk が
        // 載った直後の最初の Tick で全発火（Emote＋Text）と自然終端（TalkDone{Ended}）が
        // 単一 Tick 内で完了する。
        let mut now: u64 = 1;
        let mut fired = false;
        for _ in 0..10_000u32 {
            runtime
                .dispatcher()
                .send(DispatcherMsg::Tick {
                    now: MonotonicMs(now),
                })
                .expect("dispatcher actor should still be alive while probing for the boot talk");
            now += 1;
            if !surface_records
                .lock()
                .expect("records mutex poisoned")
                .is_empty()
            {
                fired = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            fired,
            "S1: surface cue never fired after repeated Tick — boot talk did not reach \
             dispatcher's active slot within bound"
        );

        // ---- (a) 起動系列が正典順序で発火（NOTIFY／GET の別・Reference 構成込み） ----
        // real shiori アクター（run_shiori_loop）はメッセージ到達のたびに冒頭で
        // backend.status() を確認する（死活監視・親モジュール rustdoc 参照）ため、
        // calls() には Get/Notify の間に RecordedCall::Status が挟まる。起動系列の
        // 順序判定はこの死活監視ノイズと無関係なので除外して比較する。
        let expected_boot_prefix = vec![
            expected_from_shiori_call(events::on_initialize()),
            expected_from_shiori_call(events::on_first_boot()),
            expected_from_shiori_call(events::on_boot(&config)),
            expected_from_shiori_call(events::baseware_version(&config)),
        ];
        let calls = handle.calls();
        let calls_without_status: Vec<RecordedCall> = calls
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .cloned()
            .collect();
        assert_eq!(
            calls_without_status, expected_boot_prefix,
            "起動系列（OnInitialize→OnFirstBoot→OnBoot→basewareversion）が正典順序で発火していない"
        );

        // ---- (b)(c) RecordingSink の発火列（at 昇順・内容一致）----
        let surface = surface_records
            .lock()
            .expect("records mutex poisoned")
            .clone();
        assert_eq!(
            surface.len(),
            1,
            "surface 発火は \\s[0] 由来の Emote 1 件のみのはず: {surface:?}"
        );
        assert_eq!(surface[0].at, 0.0);
        assert_eq!(surface[0].actor, ActorKey::from("0"));
        assert_eq!(
            surface[0].command,
            CueCommand::Emote {
                key: "0".to_string()
            }
        );

        let text = text_records.lock().expect("records mutex poisoned").clone();
        assert_eq!(
            text.len(),
            1,
            "text 発火は \"hello\" 由来の Text 1 件のみのはず: {text:?}"
        );
        assert_eq!(text[0].at, 0.0);
        assert_eq!(text[0].actor, ActorKey::from("0"));
        assert_eq!(text[0].command, CueCommand::Text("hello".to_string()));

        // at 昇順（両 sink とも単調非減少であること・design「発火列」節）。
        for pair in surface.windows(2) {
            assert!(
                pair[0].at <= pair[1].at,
                "surface 発火列は at 昇順であるべき"
            );
        }
        for pair in text.windows(2) {
            assert!(pair[0].at <= pair[1].at, "text 発火列は at 昇順であるべき");
        }

        // ---- 後片付け兼 (c) の間接証跡 ----
        // TalkDone{Ended} が dispatcher→kanade へ転送済みであること（dispatcher の slot が
        // 解放され kanade が Steady{None} へ戻っていること）は、kanade inbox を直接覗く
        // 経路が公開面に無いため、後続の shutdown（ForceQuit→OnClose NOTIFY→Unload の順）
        // が台本どおり完走し Ok(()) を返すことをもって間接的に確認する——もし TalkDone が
        // 届かず kanade が Steady{Some} に取り残されていても ForceQuit は横断遷移で全 Phase
        // から Unloading{Forced} へ直行するため shutdown 自体は成立してしまうが、これは
        // 「正規終了握手」シナリオ（task 4.5）の担当範囲であり、本タスクの主眼は (a)(b) の
        // 発火列検証に置く（CONCERNS 参照）。
        run_bounded(
            "shutdown after S1 boot talk completion",
            std::time::Duration::from_secs(10),
            move || {
                let result = runtime.shutdown(areka_kanade::CloseReason::System);
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) after S1 boot talk completes, got {result:?}"
                );
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
