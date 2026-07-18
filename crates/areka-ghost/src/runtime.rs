//! ghost 結線層の起動・終了統括（`GhostRuntime`／`boot`／`shutdown`）。
//!
//! task 3.1 で `boot` 手順と `GhostRuntime`（`kanade()`／`dispatcher()` の
//! 投函端アクセサのみ）を実装した。task 3.2 は同じ `GhostRuntime` へ
//! `shutdown`／`into_parts`（`GhostParts`・`GhostHandles`・`GhostShutdownError`
//! を含む終了統括一式）を追加実装した（design.md「終了（shutdown）シーケンス」
//! 「アクター別の停止経路（正本）」）。

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

use areka_actor::ActorHandle;
use areka_kanade::{KanadeMsg, ShioriBackend, spawn_kanade, spawn_shiori_actor};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{MountError, MountModel, resolve};
use areka_sakura::contract::SystemVarSnapshot;
use areka_sakura::sysvar::DEFAULT_USERNAME;

use crate::config::resolve_kanade_config;
use crate::dispatcher::{DispatcherMsg, spawn_dispatcher};
use crate::relay::spawn_relay;
use crate::sink::BootCueSink;
use crate::ticker::{TickerConfig, TickerMsg, spawn_ticker};

/// 起動失敗（design.md「Error Categories and Responses」）。
///
/// マウント解決の失敗（起点不在／読取不能／shell 不在・`MountError` の各
/// variant）を包む。呼び出し側（areka main）はこれを非致命として扱い、
/// ダミー窓・smoke ゲート等の骨格起動を継続する（要件 2.5・8.2）。e2e は
/// 明示 fail として扱う（design.md 該当節）。
///
/// 後続タスクで新たな起動失敗種別が増える可能性に備え `#[non_exhaustive]`
/// を付す。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GhostBootError {
    /// descript.txt 起点のマウント解決が失敗した（要件 2.1・2.5）。
    ///
    /// `MountError` 自体は `areka-parsers` 側の純粋なデータ型（`Display`／
    /// `std::error::Error` 未実装）であるため、`Debug` 表現をメッセージへ
    /// 埋め込む。
    #[error("ghost mount resolution failed: {0:?}")]
    Mount(MountError),
}

/// SHIORI 結線方式（design.md「ghost::runtime」Service Interface）。
pub enum ShioriWiring {
    /// 実 helper 結線（本番・env ゲート追験）。
    Helper {
        /// 32bit SHIORI helper 実行ファイルのパス。
        helper_exe: PathBuf,
    },
    /// 任意 backend 注入（spine e2e＝scripted fake）。connect closure は
    /// shiori アクタースレッド上で一度だけ実行される（要件 3.1）。
    Custom(Box<dyn FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send>),
}

/// per-talk に呼ばれ、システム変数の凍結スナップショットを返す供給シーム
/// （design.md「GhostBootOptions S-3＋provider」・R7.3）。
///
/// dispatcher が talk 起動ごとに一度だけ呼び出し、返ったスナップショットを per-talk の
/// `spawn_talk` へ手渡す（**凍結像の刻印点**＝talk ごと凍結の意味論・sylphya の per-talk
/// 凍結と同形）。W1 の暫定 provider（[`default_system_vars`]）を将来 `areka-P0-sylphya` の
/// 読み口へ差し替える差替点で、sakura 側の契約（[`SystemVarSnapshot`]）は無改変のまま。
pub type SystemVarSource = Box<dyn Fn() -> SystemVarSnapshot + Send>;

/// ticker 起動方式（design.md「ghost::runtime」Service Interface）。
pub enum TickerMode {
    /// 実クロック駆動（本番）。
    Real(TickerConfig),
    /// ticker を起動しない（決定論テスト＝Tick は外部注入・要件 5.4）。
    Disabled,
}

/// `boot` の入力一式（design.md「ghost::runtime」Service Interface・「GhostBootOptions S-3」）。
pub struct GhostBootOptions {
    /// descript.txt 起点のマウント解決対象ディレクトリ。
    pub ghost_root: PathBuf,
    /// charset 未宣言時の既定エンコーディング（既定 Ansi・SSP 準拠・記憶
    /// areka-descript-encoding）。
    pub default_encoding: DefaultEncoding,
    /// SHIORI 結線方式。
    pub shiori: ShioriWiring,
    /// 構築時注入の可変長 sink 列（S-3・要件 4.6/8.5）。登録順＝broadcast 順（決定論）。
    ///
    /// 旧「2 固定スロット（`surface_sink`/`text_sink`）」の意図的更新——演者数に依らない
    /// 可変長 [`BootCueSink`] 列とし、dispatcher が talk 起動ごとに各要素を `clone_box` して
    /// per-talk の `spawn_talk` へ手渡す。診断既定は `vec![LogSink, DiscardSink]` 相当
    /// （cue ごと 1 回ログの既存性質を維持・design.md「GhostBootOptions S-3」）。
    pub sinks: Vec<Box<dyn BootCueSink>>,
    /// システム変数の供給シーム（S-3・R7.3/7.4）。⓪ghost が埋める責務の実装点で、
    /// dispatcher が talk 起動ごとに一度呼び出し、返った凍結スナップショットを per-talk へ
    /// 手渡す（凍結像の刻印点）。本番の暫定既定は [`default_system_vars`]（`%username` のみ）。
    pub system_vars: SystemVarSource,
    /// ticker 起動方式。
    pub ticker: TickerMode,
}

/// W1 暫定 provider（design.md「GhostBootOptions S-3＋provider」・R7.4）。
///
/// `{"username": DEFAULT_USERNAME}` だけを充填した凍結スナップショットを毎回新規構築して
/// 返すクロージャを返す。既定値は sakura 側 [`areka_sakura::sysvar::DEFAULT_USERNAME`] の
/// **唯一の定義点**を re-use し、`%username` の既定を⓪ghost 側へ書き写して二重定義しない
/// （偽ストアを作らない・R7.4）。将来 `areka-P0-sylphya` の読み口へ差し替える差替シームで、
/// 差替時も本関数の型（[`SystemVarSource`]）と dispatcher の刻印点は無改変のまま。
pub fn default_system_vars() -> SystemVarSource {
    Box::new(|| {
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", DEFAULT_USERNAME);
        snapshot
    })
}

/// ghost 結線層が起動した全コンポーネントの所有者。
///
/// [`GhostRuntime::kanade`]／[`GhostRuntime::dispatcher`]（テスト駆動・Tick 注入点）に
/// 加え、終了統括の [`GhostRuntime::shutdown`] と分解結線用の [`GhostRuntime::into_parts`]
/// を提供する（task 3.2・design.md「ghost::runtime」）。`mount` はログ／後続用の保持物
/// （design.md「保持物」節）で、現時点では読み出さないため `#[allow(dead_code)]` を
/// フィールド単位で残す。
pub struct GhostRuntime {
    kanade_tx: Sender<KanadeMsg>,
    dispatcher_tx: Sender<DispatcherMsg>,
    ticker_tx: Option<Sender<TickerMsg>>,
    kanade_handle: ActorHandle,
    dispatcher_handle: ActorHandle,
    shiori_handle: ActorHandle,
    start_relay_handle: ActorHandle,
    down_relay_handle: ActorHandle,
    ticker_handle: Option<ActorHandle>,
    #[allow(dead_code)]
    mount: MountModel,
}

/// `into_parts` が返す全 `ActorHandle`（design.md「アクター別の停止経路（正本）」・
/// 「ghost::runtime」Service Interface）。S6 全断線シナリオ等、`shutdown` を経ずに
/// 手動でハンドルを join したい上級用途・テスト向け。
pub struct GhostHandles {
    pub kanade: ActorHandle,
    pub dispatcher: ActorHandle,
    pub shiori: ActorHandle,
    pub start_relay: ActorHandle,
    pub down_relay: ActorHandle,
    /// `TickerMode::Real` 時のみ `Some`。
    pub ticker: Option<ActorHandle>,
}

/// `into_parts` の分解結果（S6 段階的解体の駆動口・design.md「ghost::runtime」）。
///
/// shiori への投函端は**存在しない**（`GhostRuntime` は `shiori_tx` を保持しない——
/// 「アクター別の停止経路（正本）」表の前提。shiori の停止は kanade 終了系列の
/// `ShioriMsg::Close` が正経路、kanade panic 時は shiori_tx drop による切断が
/// フォールバックとして機能する）。
pub struct GhostParts {
    /// S6②の Close 送出・S3/S5 の Tick 注入。
    pub kanade: Sender<KanadeMsg>,
    /// S6①の Close 送出・Tick 注入。
    pub dispatcher: Sender<DispatcherMsg>,
    /// `TickerMode::Real` 時のみ `Some`。
    pub ticker: Option<Sender<TickerMsg>>,
    /// kanade／dispatcher／shiori／start-relay／down-relay／ticker(Option) の全 `ActorHandle`。
    pub handles: GhostHandles,
}

/// 終了統括の失敗（design.md「Error Categories and Responses」・要件 6.5）。
///
/// 各段の join で観測された panic（[`areka_actor::ActorError`]）を段名つきで収集する。
/// best-effort 完走の結果、失敗集合が非空だった場合にのみ構築される（全段成功なら
/// `shutdown` は `Ok(())` を返す）。単一の不透明文字列へ潰さず、どの段が・なぜ失敗
/// したかを個別に保持する（silent failure なしの精神）。
#[derive(Debug, thiserror::Error)]
#[error("ghost shutdown failed in stage(s): {failures:?}")]
pub struct GhostShutdownError {
    /// `(段名, 観測された ActorError)` の一覧（発生順）。
    pub failures: Vec<(&'static str, areka_actor::ActorError)>,
}

impl GhostRuntime {
    /// kanade inbox への投函端（テスト駆動・後続 input-events の結線点）。
    pub fn kanade(&self) -> &Sender<KanadeMsg> {
        &self.kanade_tx
    }

    /// dispatcher inbox への投函端（テストの Tick 注入点）。
    pub fn dispatcher(&self) -> &Sender<DispatcherMsg> {
        &self.dispatcher_tx
    }

    /// 終了統括（design.md「終了（shutdown）シーケンス」・要件 6.1/6.4/6.5）。
    ///
    /// 手順: `KanadeMsg::ForceQuit(reason)` 送出 → kanade join → `DispatcherMsg::Close`
    /// 送出 → dispatcher join →（ticker 起動時のみ）`TickerMsg::Close` 送出 → ticker
    /// join → shiori join（明示送出なし。kanade 終了系列が既に `ShioriMsg::Close` を
    /// 送出済み） → start-relay join → down-relay join（両 relay は明示 Close を持たず
    /// 上流停止に連動する自然終了・「アクター別の停止経路」表参照）。
    ///
    /// 冪等: 各段の送出失敗（対象が既に自発停止済み——例えば quit talk 経由の kanade
    /// 自己終了）は `debug!` の上で次段へ進む（正常系）。join の `Err`（panic 観測）は
    /// `error!` の上で失敗集合へ収集し、**処理を継続する**（abort-on-first-failure では
    /// ない・best-effort 完走）。全段完了後、失敗集合が空なら `Ok(())`、非空なら
    /// [`GhostShutdownError`] を返す。
    pub fn shutdown(self, reason: areka_kanade::CloseReason) -> Result<(), GhostShutdownError> {
        let GhostRuntime {
            kanade_tx,
            dispatcher_tx,
            ticker_tx,
            kanade_handle,
            dispatcher_handle,
            shiori_handle,
            start_relay_handle,
            down_relay_handle,
            ticker_handle,
            mount: _,
        } = self;

        let mut failures: Vec<(&'static str, areka_actor::ActorError)> = Vec::new();

        // 1. kanade へ ForceQuit（既に自発停止済みなら送出失敗＝冪等・debug!）。
        if kanade_tx.send(KanadeMsg::ForceQuit { reason }).is_err() {
            tracing::debug!(
                target: "ghost-shutdown",
                "kanade already stopped before ForceQuit send; treating as idempotent \
                 (its own termination sequence already ran Unload etc.)"
            );
        }

        // 2. kanade join（上流から・design.md「join の順序は『上流から』」）。
        if let Err(err) = kanade_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "kanade", error = %err, "kanade actor join failed");
            failures.push(("kanade", err));
        }

        // 3. dispatcher へ Close。
        if dispatcher_tx.send(DispatcherMsg::Close).is_err() {
            tracing::debug!(
                target: "ghost-shutdown",
                "dispatcher already stopped before Close send; treating as idempotent"
            );
        }

        // 4. dispatcher join。
        if let Err(err) = dispatcher_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "dispatcher", error = %err, "dispatcher actor join failed");
            failures.push(("dispatcher", err));
        }

        // 5・6. ticker（`TickerMode::Real` 時のみ起動されている・両方 no-op で完全スキップ）。
        if let Some(ticker_tx) = ticker_tx {
            if ticker_tx.send(TickerMsg::Close).is_err() {
                tracing::debug!(
                    target: "ghost-shutdown",
                    "ticker already stopped before Close send; treating as idempotent"
                );
            }
        }
        if let Some(ticker_handle) = ticker_handle {
            if let Err(err) = ticker_handle.join() {
                tracing::error!(target: "ghost-shutdown", stage = "ticker", error = %err, "ticker actor join failed");
                failures.push(("ticker", err));
            }
        }

        // 7. shiori join（送出なし。kanade 終了系列が ShioriMsg::Close を既に送出済み・
        //    「アクター別の停止経路」表参照。GhostRuntime は shiori_tx を保持しない）。
        if let Err(err) = shiori_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "shiori", error = %err, "shiori actor join failed");
            failures.push(("shiori", err));
        }

        // 8. start-relay join（自然終了: kanade 停止による start_tx drop で上流切断）。
        if let Err(err) = start_relay_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "start-relay", error = %err, "start-relay actor join failed");
            failures.push(("start-relay", err));
        }

        // 9. down-relay join（自然終了: shiori 停止による down_tx drop で上流切断）。
        if let Err(err) = down_relay_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "down-relay", error = %err, "down-relay actor join failed");
            failures.push(("down-relay", err));
        }

        if failures.is_empty() {
            tracing::info!(target: "ghost-shutdown", "ghost shutdown sequence completed");
            Ok(())
        } else {
            Err(GhostShutdownError { failures })
        }
    }

    /// 全断線シナリオ等の分解結線用（design.md「ghost::runtime」・S6 段階的解体の
    /// 駆動口）。通常は [`GhostRuntime::shutdown`] を使う。メッセージ送出・join は
    /// 一切行わず、保持している全 `Sender`／`ActorHandle` を呼び出し側へ構造的に
    /// 移譲するのみ（`mount` は後続用途がないため破棄する）。
    pub fn into_parts(self) -> GhostParts {
        let GhostRuntime {
            kanade_tx,
            dispatcher_tx,
            ticker_tx,
            kanade_handle,
            dispatcher_handle,
            shiori_handle,
            start_relay_handle,
            down_relay_handle,
            ticker_handle,
            mount: _,
        } = self;

        GhostParts {
            kanade: kanade_tx,
            dispatcher: dispatcher_tx,
            ticker: ticker_tx,
            handles: GhostHandles {
                kanade: kanade_handle,
                dispatcher: dispatcher_handle,
                shiori: shiori_handle,
                start_relay: start_relay_handle,
                down_relay: down_relay_handle,
                ticker: ticker_handle,
            },
        }
    }
}

/// descript.txt 起点で全エンジンを起動順に結線する（design.md「ghost::runtime」
/// Responsibilities & Constraints・「起動（boot）シーケンス」）。
///
/// 手順（要件 2.2 の順序）: マウント解決 → 運行設定解決 → shiori actor →
/// kanade → sakura dispatcher（＋start/down relay）→ ticker（`TickerMode::Real`
/// 時のみ）→ `KanadeMsg::Boot` 送出。
///
/// マウント解決が失敗した場合、他のいかなるコンポーネントも spawn される前に
/// `error!` の上で `Err(GhostBootError::Mount(_))` を返す（要件 2.5・後片付け
/// 不要——何も起動していない）。
pub fn boot(options: GhostBootOptions) -> Result<GhostRuntime, GhostBootError> {
    // 1. マウント解決（失敗は即座に打ち切り・要件 2.1/2.5）。
    let mount = match resolve(&options.ghost_root, options.default_encoding) {
        Ok(mount) => mount,
        Err(err) => {
            tracing::error!(
                target: "ghost-boot",
                ghost_root = %options.ghost_root.display(),
                error = ?err,
                "mount resolution failed; boot aborted before spawning any component"
            );
            return Err(GhostBootError::Mount(err));
        }
    };

    // 2. 運行設定の値源解決（task 2.3）。
    let config = resolve_kanade_config(&mount, options.default_encoding);

    // 3. 循環解消用の素の中継チャンネル（design.md「結線トポロジの要点」）。
    let (start_tx, start_rx) = mpsc::channel::<areka_kanade::StartTalk>();
    let (down_tx, down_rx) = mpsc::channel::<KanadeMsg>();

    // 4. connect closure の構成（Helper＝本番・Custom＝spine e2e 等の注入）。
    let connect: Box<dyn FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send> =
        match options.shiori {
            ShioriWiring::Helper { helper_exe } => Box::new(crate::shiori_wiring::real_connect(
                helper_exe,
                mount.shiori.clone(),
            )),
            ShioriWiring::Custom(connect) => connect,
        };

    // 5. shiori actor。
    let (shiori_tx, shiori_handle) = spawn_shiori_actor(connect, down_tx);

    // 6. kanade（シグネチャ不変・start_tx を「自身の」sakura Sender として渡す）。
    let (kanade_tx, kanade_handle) = spawn_kanade(config, shiori_tx, start_tx);

    // 7. sakura dispatcher（可変長 sink 列＋system_vars provider を構築時注入・S-3・
    //    要件 4.6/8.5/7.3）。provider は dispatcher が talk 起動ごとに呼び出す（刻印点）。
    let (dispatcher_tx, dispatcher_handle) =
        spawn_dispatcher(kanade_tx.clone(), options.sinks, options.system_vars);

    // 8. relay 2 本（循環解消・design.md 参照）。
    let start_relay_handle = spawn_relay::<areka_kanade::StartTalk, DispatcherMsg>(
        "start-relay",
        start_rx,
        dispatcher_tx.clone(),
    );
    let down_relay_handle =
        spawn_relay::<KanadeMsg, KanadeMsg>("down-relay", down_rx, kanade_tx.clone());

    // 9. ticker（`TickerMode::Real` 時のみ起動・要件 5.3）。
    let (ticker_tx, ticker_handle) = match options.ticker {
        TickerMode::Real(cfg) => {
            let (tx, handle) = spawn_ticker(cfg, kanade_tx.clone(), dispatcher_tx.clone());
            (Some(tx), Some(handle))
        }
        TickerMode::Disabled => (None, None),
    };

    // 10. boot 起点の起動指示（要件 2.4）。kanade は直前に spawn したばかりで
    //     常に生存しているはずだが、万一の送出失敗は panic せず観測のみに留める。
    if kanade_tx.send(KanadeMsg::Boot).is_err() {
        tracing::error!(
            target: "ghost-boot",
            "failed to send initial KanadeMsg::Boot; kanade actor appears to be gone already"
        );
    }

    tracing::info!(target: "ghost-boot", "ghost boot sequence completed");

    // 11. 全ハンドル・Sender・MountModel を保持して返す（design.md「保持物」）。
    Ok(GhostRuntime {
        kanade_tx,
        dispatcher_tx,
        ticker_tx,
        kanade_handle,
        dispatcher_handle,
        shiori_handle,
        start_relay_handle,
        down_relay_handle,
        ticker_handle,
        mount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mount_variant_constructs_and_displays() {
        let err = GhostBootError::Mount(MountError::StartPointMissing {
            expected: PathBuf::from("ghost/master/descript.txt"),
        });

        let rendered = err.to_string();
        assert!(
            rendered.contains("ghost mount resolution failed"),
            "unexpected Display output: {rendered}"
        );
        assert!(
            rendered.contains("StartPointMissing"),
            "Display should surface the underlying MountError variant: {rendered}"
        );
    }

    #[test]
    fn mount_variant_is_a_std_error() {
        let err = GhostBootError::Mount(MountError::ShellDirMissing {
            expected: PathBuf::from("ghost/master/shell/master"),
        });

        // 呼び出し側が `Box<dyn std::error::Error>` 等で一律に扱えることの確認。
        let as_std_error: &dyn std::error::Error = &err;
        assert!(as_std_error.source().is_none());
    }

    // ---- boot 統合テスト（task 3.1） ----

    use areka_kanade::MonotonicMs;
    use areka_sakura::contract::{CueSink, TalkCue};
    use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

    /// このテスト専用の一意な一時ディレクトリを返す（関数名でユニーク化・衝突回避・
    /// `config.rs` テストの流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_runtime_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
    /// `shell/master/descript.txt`）を構築する（`boot` が内部で `resolve` を通す
    /// ための happy-path fixture）。
    fn write_minimal_resolvable_ghost_fixture(root: &std::path::Path) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            b"charset,UTF-8\nname,TestShell\n",
        )
        .expect("write shell descript.txt");
    }

    /// テスト専用の最小 `ShioriBackend` fake（`get`/`notify` は無害な既定応答・`unload`
    /// は Clean・`status` は Running を返すのみ・task 4.1 の `ScriptedShioriBackend` の
    /// ような台本化はしない——boot 組み上げの結線成立だけを確認すれば足りる）。
    struct FakeShioriBackend;

    impl ShioriBackend for FakeShioriBackend {
        fn get(
            &mut self,
            _id: &str,
            _references: &[String],
        ) -> Result<Option<String>, RequestError> {
            Ok(None)
        }

        fn notify(&mut self, _id: &str, _references: &[String]) -> Result<(), RequestError> {
            Ok(())
        }

        fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
            Ok(ExitKind::Clean)
        }

        fn status(&mut self) -> HelperStatus {
            HelperStatus::Running
        }
    }

    /// テスト専用の `Clone` 可能な no-op sink（dispatcher の per-talk 注入
    /// （`S: Clone`/`T: Clone`）を満たすためだけの最小実装・`dispatcher.rs` の
    /// `RecordingSink` 流儀に倣うが、本テストは発火内容を検査しないため蓄積しない）。
    #[derive(Clone)]
    struct NoopSink;

    impl CueSink for NoopSink {
        fn emit(&mut self, _cue: TalkCue) {}
    }

    /// シナリオ1（happy path）: 解決可能な `ghost_root`・`ShioriWiring::Custom`（fake
    /// backend）・`TickerMode::Disabled` で `boot` すると `Ok(GhostRuntime)` が返り、
    /// `kanade()`／`dispatcher()` の両方の投函端が生きている（＝実際にアクタースレッドが
    /// 起動し受信ループへ入っている）ことを send の成功で確認する（要件 2.1/2.2/2.4）。
    ///
    /// 本テストは boot 単体の結線成立のみを見るため、意図的に `shutdown()` を呼ばず
    /// `runtime` を drop する——`ActorHandle` は非 RAII（detached）であり、テストプロセス
    /// 終了時にスレッドがブロックしたまま回収されるのは想定どおり（design.md「保持物」
    /// 節）。boot→shutdown の一連の流れは下記の
    /// `boot_then_shutdown_joins_everything_and_returns_ok`（task 3.2）で確認する。
    #[test]
    fn boot_happy_path_wires_all_components_and_kicks_off_boot_sequence() {
        let root =
            unique_temp_dir("boot_happy_path_wires_all_components_and_kicks_off_boot_sequence");
        let _ = std::fs::remove_dir_all(&root);
        write_minimal_resolvable_ghost_fixture(&root);

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(|| {
                Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
            system_vars: default_system_vars(),
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        // kanade actor が生存し受信ループに入っていることの直接証跡（send 成功）。
        runtime
            .kanade()
            .send(KanadeMsg::Tick {
                now: MonotonicMs(1),
            })
            .expect("kanade actor thread should be alive and receiving after boot");

        // dispatcher actor が生存し受信ループに入っていることの直接証跡（send 成功）。
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(1),
            })
            .expect("dispatcher actor thread should be alive and receiving after boot");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// シナリオ2（mount 失敗の短絡）: `ghost_root` に `ghost/master/descript.txt` が
    /// 存在しない場合、`boot` は `Err(GhostBootError::Mount(_))` を返す。`shiori`
    /// フィールドの connect closure は「呼ばれたら panic する」ものを故意に仕込み、
    /// もし実装がマウント失敗より後にも connect を評価してしまうバグがあればこの
    /// テスト自体が panic で失敗する——マウント解決失敗時に他のいかなるコンポーネント
    /// も spawn されない（短絡する）ことの直接証跡になる（要件 2.5）。
    #[test]
    fn boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring() {
        let root = unique_temp_dir(
            "boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring",
        );
        // 起点不在を保証する（ディレクトリごと未作成・ghost/master/descript.txt 無し）。
        let _ = std::fs::remove_dir_all(&root);

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Ansi,
            shiori: ShioriWiring::Custom(Box::new(|| -> Result<Box<dyn ShioriBackend>, String> {
                panic!(
                    "connect must never be invoked when mount resolution fails \
                     (boot must short-circuit before spawning anything)"
                );
            })),
            sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
            system_vars: default_system_vars(),
            ticker: TickerMode::Disabled,
        };

        match boot(options) {
            Err(GhostBootError::Mount(_)) => {}
            Ok(_) => panic!(
                "boot must fail with GhostBootError::Mount when ghost_root has no \
                 ghost/master/descript.txt"
            ),
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- boot→shutdown 統合テスト（task 3.2） ----

    /// テスト用の有界待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければ
    /// テストを失敗させる（`dispatcher.rs`／`ticker.rs` テストモジュールと同じ流儀の
    /// ローカルコピー・仮に `shutdown` の join が宙吊りするバグがあってもテスト
    /// スイート全体をハングさせない）。
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

    /// シナリオ3（happy path・task 3.2）: 解決可能な `ghost_root` で `boot` した
    /// `GhostRuntime` に対し `shutdown(CloseReason::System)` を呼ぶと、`ForceQuit` →
    /// kanade join → `DispatcherMsg::Close` → dispatcher join → shiori join → relay
    /// 2 本の join という全段が完走し `Ok(())` を返す（要件 6.1/6.4）。`TickerMode::Disabled`
    /// で組むため ticker 段は完全にスキップされる。`shutdown` 呼出自体を別スレッドへ
    /// 逃がし有界 `recv_timeout` で観測する（本テストの完了条件そのものが「正常な
    /// 起動〜終了の一連の流れ」の直接証跡になる）。
    #[test]
    fn boot_then_shutdown_joins_everything_and_returns_ok() {
        let root = unique_temp_dir("boot_then_shutdown_joins_everything_and_returns_ok");
        let _ = std::fs::remove_dir_all(&root);
        write_minimal_resolvable_ghost_fixture(&root);

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(|| {
                Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
            system_vars: default_system_vars(),
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        run_bounded(
            "shutdown after boot",
            std::time::Duration::from_secs(10),
            move || {
                let result = runtime.shutdown(areka_kanade::CloseReason::System);
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) when every stage joins cleanly, got {result:?}"
                );
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// シナリオ4（`into_parts` 構造分解・task 3.2）: `boot` した `GhostRuntime` から
    /// `into_parts()` で `GhostParts` を取り出すと、`kanade`／`dispatcher` の投函端が
    /// 生きており（send 成功で確認）、`TickerMode::Disabled` に対応して `ticker` は
    /// `None`、`handles` に全 `ActorHandle` が揃っている。取り出した部品だけを使って
    /// `shutdown()` と同等の手順（ForceQuit→kanade join→Close→dispatcher join→
    /// shiori join→relay 2 本 join）を手作業で駆動できることを示す——`into_parts` が
    /// S6 全断線シナリオ等の分解結線に必要な全てを過不足なく提供している直接証跡。
    #[test]
    fn into_parts_exposes_live_senders_and_all_handles_for_manual_teardown() {
        let root =
            unique_temp_dir("into_parts_exposes_live_senders_and_all_handles_for_manual_teardown");
        let _ = std::fs::remove_dir_all(&root);
        write_minimal_resolvable_ghost_fixture(&root);

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(|| {
                Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
            system_vars: default_system_vars(),
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
        let parts = runtime.into_parts();

        // ticker は TickerMode::Disabled に対応して None（送出端・handle 両方）。
        assert!(
            parts.ticker.is_none(),
            "ticker sender must be None when TickerMode::Disabled was used"
        );
        assert!(
            parts.handles.ticker.is_none(),
            "ticker handle must be None when TickerMode::Disabled was used"
        );

        // kanade/dispatcher の投函端が生きていることの直接証跡（send 成功）。
        parts
            .kanade
            .send(KanadeMsg::Tick {
                now: MonotonicMs(1),
            })
            .expect("kanade sender from into_parts should still be alive");
        parts
            .dispatcher
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(1),
            })
            .expect("dispatcher sender from into_parts should still be alive");

        let GhostParts {
            kanade,
            dispatcher,
            ticker: _,
            handles,
        } = parts;
        let GhostHandles {
            kanade: kanade_handle,
            dispatcher: dispatcher_handle,
            shiori: shiori_handle,
            start_relay: start_relay_handle,
            down_relay: down_relay_handle,
            ticker: _,
        } = handles;

        // shutdown() と同等の手順を手作業で駆動する（ForceQuit→join→Close→join→
        // shiori/relay join・design.md「終了（shutdown）シーケンス」）。
        run_bounded(
            "manual teardown driven from into_parts",
            std::time::Duration::from_secs(10),
            move || {
                kanade
                    .send(KanadeMsg::ForceQuit {
                        reason: areka_kanade::CloseReason::System,
                    })
                    .expect("kanade should still accept ForceQuit");
                kanade_handle
                    .join()
                    .expect("kanade should terminate normally after ForceQuit");

                dispatcher
                    .send(DispatcherMsg::Close)
                    .expect("dispatcher should still accept Close");
                dispatcher_handle
                    .join()
                    .expect("dispatcher should terminate normally after Close");

                shiori_handle
                    .join()
                    .expect("shiori should terminate normally (shiori_tx dropped with kanade)");
                start_relay_handle
                    .join()
                    .expect("start-relay should terminate normally (natural disconnect)");
                down_relay_handle
                    .join()
                    .expect("down-relay should terminate normally (natural disconnect)");
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
