//! ghost 結線層の起動・終了統括（`GhostRuntime`／`boot`／`shutdown`）。
//!
//! 起動は `boot`／`boot_with_kanade_stop`／`boot_with_origin`／`boot_with_balloon`、動作中の差し替えは
//! `set_shell_dir`（シェルのフォルダ）・`set_balloon_dir`（バルーンの層）と `set_names`（名前）、終了は `shutdown`／
//! `shutdown_within`／`into_parts`。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};

use areka_actor::ActorHandle;
use areka_kanade::{
    BootOrigin, KanadeConfig, KanadeMsg, KanadeNotice, ShioriBackend, ShioriCut, ShioriProbe,
    TranslateSeams, WaitBudget, spawn_kanade_translating, spawn_shiori_actor,
};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{GhostNames, MountError, MountModel, resolve_with_shell};
use areka_sakura::contract::SystemVarSnapshot;
use areka_sakura::strict::StrictJudges;
use areka_sylphya::{
    AskerContext, AskerId, DottedResolution, PersistKey, PersistScope, SylphyaPublisher,
    SylphyaReader,
};
use areka_talk::EpilogueCommand;

use crate::config::resolve_kanade_config;
use crate::dispatcher::{DispatcherMsg, spawn_dispatcher};
use crate::prop_get_sink::PropGetCueSink;
use crate::prop_sink::{PROP_SET_CUE_NAME, PropSetCueSink};
use crate::relay::spawn_relay;
use crate::sink::BootCueSink;
use crate::ticker::{TickerConfig, TickerMsg, spawn_ticker};

/// 起動失敗。
///
/// マウント解決の失敗（起点不在／読取不能／shell 不在・`MountError` の各 variant）を包む。
/// 呼び出し側（areka main）は非致命として扱い、開いた起動窓を保ったままゴーストの実行環境なしで
/// 起動を続ける。e2e は明示の失敗として扱う。
///
/// 起動失敗の種別が増えうるので `#[non_exhaustive]`。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GhostBootError {
    /// descript.txt 起点のマウント解決が失敗した。
    ///
    /// `MountError` は `Display`／`std::error::Error` を実装しないので、`Debug` 表現を
    /// メッセージへ埋め込む。
    #[error("ghost mount resolution failed: {0:?}")]
    Mount(MountError),
}

/// SHIORI 結線方式。
pub enum ShioriWiring {
    /// 実 helper 結線（本番）。
    Helper {
        /// 32bit SHIORI helper 実行ファイルのパス。
        helper_exe: PathBuf,
    },
    /// 任意 backend の注入（scripted fake）。connect closure は shiori アクタースレッド上で
    /// 一度だけ実行される。
    Custom(Box<dyn FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send>),
    /// 同じプロセスに x64 SHIORI4 DLL を読む結線。
    ///
    /// `mount.shiori.dir.join(file)` で解決した DLL をロードし、SHIORI4 生成入口
    /// （`shiori_factory`）→ `IShiori` → `ShioriBackend` へ
    /// [`crate::shiori_inproc::inproc_connect`] が写す。DLL パスはマウント解決結果から導く。
    InProc,
}

/// per-talk に呼ばれ、システム変数の凍結スナップショットを返す供給シーム。
///
/// dispatcher が talk 起動ごとに一度だけ呼び、返ったスナップショットを per-talk の
/// `spawn_talk` へ渡す（talk ごとに凍結する）。本番既定は sylphya 読み口由来
/// （[`SystemVarWiring::FromSylphya`]）。テスト・特殊用途は [`SystemVarWiring::Custom`] で
/// このクロージャを直接注入する。
pub type SystemVarSource = Box<dyn Fn() -> SystemVarSnapshot + Send>;

/// システム変数 provider の結線方式。
///
/// [`SystemVarSource`] の源を選ぶ。
/// - [`FromSylphya`](SystemVarWiring::FromSylphya): 本番既定。boot が据えた sylphya reader と
///   自 `AskerId` から、`talk_snapshot` を [`SystemVarSnapshot`] へ写す provider を構築する
///   （[`crate::sylphya_wiring::from_sylphya_provider`]）。
/// - [`Custom`](SystemVarWiring::Custom): テスト・特殊用途の直接注入。
///
/// どちらでも boot は sylphya アクターを起動して静的構成／username を publish する。
pub enum SystemVarWiring {
    /// 本番既定: boot が据えた sylphya reader からスナップショットを生成する。
    FromSylphya,
    /// テスト・特殊用途の注入。
    Custom(SystemVarSource),
}

/// ticker 起動方式。
pub enum TickerMode {
    /// 実クロック駆動（本番）。
    Real(TickerConfig),
    /// ticker を起動しない（Tick は外部から注入する）。
    Disabled,
}

/// `boot` の入力一式。
pub struct GhostBootOptions {
    /// descript.txt 起点のマウント解決対象ディレクトリ。
    pub ghost_root: PathBuf,
    /// charset 未宣言時の既定エンコーディング（既定 Ansi・SSP 準拠）。
    pub default_encoding: DefaultEncoding,
    /// SHIORI 結線方式。
    pub shiori: ShioriWiring,
    /// 構築時に注入する sink 列。登録順が broadcast 順。dispatcher が talk 起動ごとに各要素を
    /// `clone_box` して per-talk の `spawn_talk` へ渡す。診断の既定は `vec![LogSink, DiscardSink]` 相当。
    pub sinks: Vec<Box<dyn BootCueSink>>,
    /// システム変数 provider の結線方式。本番は [`SystemVarWiring::FromSylphya`]、
    /// テストは [`SystemVarWiring::Custom`]。
    pub system_vars: SystemVarWiring,
    /// App スコープの永続 root（sylphya の App 層 profile フォルダ・bin が供給）。
    ///
    /// `None` は App スコープ利用不可。ghost／shell スコープは mount 解決結果から導く
    /// （`<shiori.dir>/profile/areka/`・`<shell.dir>/profile/areka/`）が、App スコープは
    /// マウントに現れないので呼び出し側（bin）が供給する（既定は実行ファイル隣接の `profile/areka/`・
    /// env `AREKA_PROFILE_DIR` で上書き可）。
    pub app_profile_dir: Option<PathBuf>,
    /// ticker 起動方式。
    pub ticker: TickerMode,
    /// strict の記録の判定の差し込み。`boot` がゴースト名と束ねて配り口へ渡す。
    /// `None` は差し込みの無い構築で、strict の台本も記録せずに再生する。
    pub strict_judges: Option<Arc<dyn StrictJudges>>,
}

/// ghost 結線層が起動した全コンポーネントの所有者。
///
/// テスト駆動・Tick 注入点の [`GhostRuntime::kanade`]／[`GhostRuntime::dispatcher`]、終了統括の
/// [`GhostRuntime::shutdown`]、分解用の [`GhostRuntime::into_parts`] を提供する。
/// `mount` は [`GhostRuntime::mount`] で読める。
pub struct GhostRuntime {
    kanade_tx: Sender<KanadeMsg>,
    dispatcher_tx: Sender<DispatcherMsg>,
    ticker_tx: Option<Sender<TickerMsg>>,
    kanade_handle: ActorHandle,
    dispatcher_handle: ActorHandle,
    shiori_handle: ActorHandle,
    start_relay_handle: ActorHandle,
    down_relay_handle: ActorHandle,
    /// 選択待ちの起こしの中継（kanade → 刻み役・`TickerMode::Real` 時のみ `Some`）。
    choice_wake_relay_handle: Option<ActorHandle>,
    ticker_handle: Option<ActorHandle>,
    /// sylphya（統一プロパティシステム）供給端。shutdown の `Close` 送出に使う。
    sylphya_publisher: SylphyaPublisher,
    /// sylphya 読み口（provider と同じ番人へ聞く）。
    sylphya_reader: SylphyaReader,
    /// sylphya アクターの join ハンドル。shutdown の最終段で join して panic を観測する。
    sylphya_handle: ActorHandle,
    mount: MountModel,
    /// SHIORI の待ちの見張り部品（[`GhostRuntime::shutdown_within`] だけが張る）。
    shiori_probe: ShioriProbe,
}

/// `into_parts` が返す全 `ActorHandle`。`shutdown` を経ずに手動で join したいテスト向け。
pub struct GhostHandles {
    pub kanade: ActorHandle,
    pub dispatcher: ActorHandle,
    pub shiori: ActorHandle,
    pub start_relay: ActorHandle,
    pub down_relay: ActorHandle,
    /// 選択待ちの起こしの中継（kanade の起こしの頼みを刻み役へ写す）。`TickerMode::Real` 時のみ `Some`。
    /// kanade の停止で送り端が落ちて自然に終わる。畳むのは `down_relay` の後・刻み役の Close の前。
    pub choice_wake_relay: Option<ActorHandle>,
    /// `TickerMode::Real` 時のみ `Some`。
    pub ticker: Option<ActorHandle>,
    /// sylphya アクターの join ハンドル（shutdown の最終段で join する）。
    pub sylphya: ActorHandle,
}

/// `into_parts` の分解結果。
///
/// shiori への投函端は無い（`GhostRuntime` は `shiori_tx` を保持しない）。shiori は kanade の
/// 終了系列の `ShioriMsg::Close` で止まり、kanade が panic したときは `shiori_tx` の drop による
/// 切断で止まる。
pub struct GhostParts {
    /// Close 送出・Tick 注入。
    pub kanade: Sender<KanadeMsg>,
    /// Close 送出・Tick 注入。
    pub dispatcher: Sender<DispatcherMsg>,
    /// `TickerMode::Real` 時のみ `Some`。
    pub ticker: Option<Sender<TickerMsg>>,
    /// sylphya 供給端（`Close` 送出）。
    pub sylphya: SylphyaPublisher,
    /// sylphya 読み口（provider と同じ番人へ聞く）。
    pub sylphya_reader: SylphyaReader,
    /// 全 `ActorHandle`。
    pub handles: GhostHandles,
}

/// 終了統括の失敗。
///
/// 各段の join で観測された panic（[`areka_actor::ActorError`]）を段名つきで収集する。
/// 最後まで走らせた結果、失敗が 1 つでもあったときだけ構築される。どの段がなぜ失敗したかを
/// 個別に保持する。
#[derive(Debug, thiserror::Error)]
#[error("ghost shutdown failed in stage(s): {failures:?}")]
pub struct GhostShutdownError {
    /// `(段名, 観測された ActorError)` の一覧（発生順）。
    pub failures: Vec<(&'static str, areka_actor::ActorError)>,
}

impl GhostRuntime {
    /// kanade inbox への投函端。
    pub fn kanade(&self) -> &Sender<KanadeMsg> {
        &self.kanade_tx
    }

    /// dispatcher inbox への投函端（テストの Tick 注入点）。
    pub fn dispatcher(&self) -> &Sender<DispatcherMsg> {
        &self.dispatcher_tx
    }

    /// sylphya（統一プロパティシステム）供給端への参照。main がこの clone を捕捉して
    /// `PersistWiring`（位置永続の書込端）を組む。
    pub fn sylphya_publisher(&self) -> &SylphyaPublisher {
        &self.sylphya_publisher
    }

    /// sylphya（統一プロパティシステム）の読み口への参照。
    pub fn sylphya_reader(&self) -> &SylphyaReader {
        &self.sylphya_reader
    }

    /// 解決済みマウントモデルへの参照（`readme` キー転記の読み出し口）。
    pub fn mount(&self) -> &MountModel {
        &self.mount
    }

    /// SHIORI の待ちの見張り部品（テストが手動の口 `cut_now` で見張りを起こすための読み口）。
    pub fn shiori_probe(&self) -> &ShioriProbe {
        &self.shiori_probe
    }

    /// 今のシェルのフォルダ（`mount.shell.dir`）を書き換え、sylphya のシェルの層を付け替える。
    ///
    /// シェルの差し替えの後に呼び、`mount.shell.dir` の読み手を差し替え後のシェルへ向ける。
    /// シェルの層の置き場は `<dir>/profile/areka/` になり、前のシェルの層の揮発の値は捨てる
    /// （同じフォルダを渡したときは付け替えず、揮発の値も保つ）。
    /// bindgroup の転記は起動時のシェルのまま（差し替えの資産づくりが新しいシェルから読み直す）。
    pub fn set_shell_dir(&mut self, dir: PathBuf) {
        tracing::info!(
            target: "ghost-runtime",
            from = %self.mount.shell.dir.display(),
            to = %dir.display(),
            "current shell dir replaced"
        );
        // `\![reload,shell]` は今のシェルを同じフォルダで渡し直す。そのときは層の揮発の値を保つ。
        if dir != self.mount.shell.dir {
            self.sylphya_publisher.set_layer_root(
                PersistScope::Shell,
                Some(crate::sylphya_wiring::profile_areka_root(&dir)),
            );
        }
        self.mount.shell.dir = dir;
    }

    /// sylphya のバルーンの層を `<dir>/profile/areka/` へ付け替える（前のバルーンの層の揮発の値は捨てる）。
    pub fn set_balloon_dir(&self, dir: &Path) {
        tracing::info!(
            target: "ghost-runtime",
            to = %dir.display(),
            "current balloon dir replaced"
        );
        self.sylphya_publisher.set_layer_root(
            PersistScope::Balloon,
            Some(crate::sylphya_wiring::profile_areka_root(dir)),
        );
    }

    /// 動作中のゴーストの名前（`mount.names`）を替え、静的プロパティ（`selfname` ほか・
    /// `baseware.*`）を新しい名前で出し直す（descript の読み直し）。
    /// 新しい名前が定義しない静的な名前は先に sylphya から消す（新しい descript で起動したのと同じ状態）。
    pub fn set_names(&mut self, names: GhostNames) {
        tracing::info!(
            target: "ghost-runtime",
            from = ?self.mount.names,
            to = ?names,
            "ghost names replaced"
        );
        self.mount.names = names;
        let asker = crate::sylphya_wiring::ghost_asker_id(&self.mount.shiori.dir);
        // 新しい名前に無い静的名は消す（起動時に積まないのと同じ）。
        self.sylphya_publisher.remove_flat(
            asker.clone(),
            crate::sylphya_wiring::absent_flat_statics(&self.mount.names),
        );
        crate::sylphya_wiring::publish_ghost_statics(
            &self.sylphya_publisher,
            asker,
            &self.mount.names,
            env!("CARGO_PKG_VERSION"),
        );
    }

    /// 期限つきの終了統括。見張りを `budget` で張ってから [`GhostRuntime::shutdown`] をそのまま
    /// 走らせ、降ろした結果と打ち切りの結果（上限で SHIORI の待ちを解いたときだけ `Some`）を返す。
    /// join の順・冪等・失敗の収集は `shutdown` のまま。
    pub fn shutdown_within(
        self,
        reason: areka_kanade::CloseReason,
        budget: WaitBudget,
    ) -> (Result<(), GhostShutdownError>, Option<ShioriCut>) {
        let guard = self.shiori_probe.arm(budget);
        let result = self.shutdown(reason);
        (result, guard.finish())
    }

    /// 終了統括。
    ///
    /// 手順: `KanadeMsg::ForceQuit(reason)` 送出 → kanade join → `DispatcherMsg::Close`
    /// 送出 → dispatcher join → shiori join（kanade の終了系列が `ShioriMsg::Close` を送出済みなので
    /// 明示送出なし） → start-relay join → down-relay join →（ticker 起動時のみ）
    /// choice-wake-relay join → `TickerMsg::Close` 送出 → ticker join。relay は明示 Close を持たず
    /// 上流の停止で自然に終わる。起こしの中継は刻み役の送り端を持つが、kanade の停止で上流が先に
    /// 落ちるので、刻み役の Close の前に join しても詰まらない。
    ///
    /// 冪等: 各段の送出失敗（対象が既に自発停止済み。例えば quit talk 経由の kanade 自己終了）は
    /// `debug!` の上で次段へ進む。join の `Err`（panic）は `error!` の上で失敗集合へ収集し、
    /// 最後まで続ける。失敗集合が空なら `Ok()`、非空なら [`GhostShutdownError`] を返す。
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
            choice_wake_relay_handle,
            ticker_handle,
            sylphya_publisher,
            sylphya_reader: _,
            sylphya_handle,
            mount: _,
            shiori_probe: _,
        } = self;

        let mut failures: Vec<(&'static str, areka_actor::ActorError)> = Vec::new();

        if kanade_tx.send(KanadeMsg::ForceQuit { reason }).is_err() {
            tracing::debug!(
                target: "ghost-shutdown",
                "kanade already stopped before ForceQuit send; treating as idempotent \
                 (its own termination sequence already ran Unload etc.)"
            );
        }

        if let Err(err) = kanade_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "kanade", error = %err, "kanade actor join failed");
            failures.push(("kanade", err));
        }

        if dispatcher_tx.send(DispatcherMsg::Close).is_err() {
            tracing::debug!(
                target: "ghost-shutdown",
                "dispatcher already stopped before Close send; treating as idempotent"
            );
        }

        if let Err(err) = dispatcher_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "dispatcher", error = %err, "dispatcher actor join failed");
            failures.push(("dispatcher", err));
        }

        // shiori は join だけ（`GhostRuntime` は `shiori_tx` を保持しない）。
        if let Err(err) = shiori_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "shiori", error = %err, "shiori actor join failed");
            failures.push(("shiori", err));
        }

        // relay は自然終了する。start-relay は kanade 停止による start_tx の drop で、down-relay は
        // shiori 停止による down_tx の drop で、上流が切れる。
        if let Err(err) = start_relay_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "start-relay", error = %err, "start-relay actor join failed");
            failures.push(("start-relay", err));
        }

        if let Err(err) = down_relay_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "down-relay", error = %err, "down-relay actor join failed");
            failures.push(("down-relay", err));
        }

        // 起こしの中継は `TickerMode::Real` 時のみ。kanade 停止で起こしの線の送り端が落ちて終わる。
        if let Some(handle) = choice_wake_relay_handle
            && let Err(err) = handle.join()
        {
            tracing::error!(target: "ghost-shutdown", stage = "choice-wake-relay", error = %err, "choice-wake-relay actor join failed");
            failures.push(("choice-wake-relay", err));
        }

        // ticker は `TickerMode::Real` 時のみ起動されている。
        if let Some(ticker_tx) = ticker_tx
            && ticker_tx.send(TickerMsg::Close).is_err()
        {
            tracing::debug!(
                target: "ghost-shutdown",
                "ticker already stopped before Close send; treating as idempotent"
            );
        }
        if let Some(ticker_handle) = ticker_handle
            && let Err(err) = ticker_handle.join()
        {
            tracing::error!(target: "ghost-shutdown", stage = "ticker", error = %err, "ticker actor join failed");
            failures.push(("ticker", err));
        }

        // 供給者が全て止まった後に sylphya を畳む。停止済みへの再送は SylphyaPublisher が warn して
        // 縮退する。
        //
        // DragEnd の永続書込は FIFO で投函済みで、FIFO の close がそれを保証する。それでも close の
        // 前に barrier を置き、終了直前までの永続 put が反映されてからストアを畳む。失敗しても
        // warn して続行する。
        match sylphya_publisher.barrier() {
            Ok(()) => {
                tracing::info!(target: "ghost-shutdown", "persist flush confirmed");
            }
            Err(err) => {
                tracing::warn!(
                    target: "ghost-shutdown",
                    error = %err,
                    "persist flush barrier failed before sylphya close; continuing \
                     (write-through via FIFO close is the primary guarantee)"
                );
            }
        }
        sylphya_publisher.close();
        if let Err(err) = sylphya_handle.join() {
            tracing::error!(target: "ghost-shutdown", stage = "sylphya", error = %err, "sylphya actor join failed");
            failures.push(("sylphya", err));
        }

        if failures.is_empty() {
            tracing::info!(target: "ghost-shutdown", "ghost shutdown sequence completed");
            Ok(())
        } else {
            Err(GhostShutdownError { failures })
        }
    }

    /// 分解結線用（テストが手動で解体する）。通常は [`GhostRuntime::shutdown`] を使う。
    /// メッセージ送出・join は行わず、保持している全 `Sender`／`ActorHandle` を呼び出し側へ渡す
    /// （`mount` は破棄する）。
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
            choice_wake_relay_handle,
            ticker_handle,
            sylphya_publisher,
            sylphya_reader,
            sylphya_handle,
            mount: _,
            shiori_probe: _,
        } = self;

        GhostParts {
            kanade: kanade_tx,
            dispatcher: dispatcher_tx,
            ticker: ticker_tx,
            sylphya: sylphya_publisher,
            sylphya_reader,
            handles: GhostHandles {
                kanade: kanade_handle,
                dispatcher: dispatcher_handle,
                shiori: shiori_handle,
                start_relay: start_relay_handle,
                down_relay: down_relay_handle,
                choice_wake_relay: choice_wake_relay_handle,
                ticker: ticker_handle,
                sylphya: sylphya_handle,
            },
        }
    }
}

/// 起動記録ゲート。
///
/// sylphya 読み口（`reader`）と ghost 自身の `ghost_asker` から記憶を引き、[`KanadeConfig`] の
/// 初回起動系フィールドを解決して返す。読むだけで、panic しない。
///
/// 1. `areka.boot.count`（正準文字列は [`PersistKey::BootCount`]）が記憶に在れば
///    `first_boot=false`、なければ `true`。値は数値として読まず、存在だけを見る。
/// 2. `areka.vanish.count` を u32 として読んで `config.vanish_count` へ。不在は 0、非数値は
///    warn して 0（起動は止めない）。
/// 3. `first_boot` のとき、初回挨拶トークの完走時に起動記録を書く汎用プロパティ SET キュー 1 件
///    （`[PROP_SET_CUE_NAME, [BootCount 正準 key, "1"]]`）を `config.first_boot_epilogue` へ据える。
///    kanade は正準 key をそのまま運ぶだけで、sylphya に依存しない。
fn apply_boot_record_gate(
    mut config: KanadeConfig,
    reader: &SylphyaReader,
    ghost_asker: &AskerId,
) -> KanadeConfig {
    let ctx = AskerContext {
        asker: ghost_asker.clone(),
    };

    let boot_count_key = PersistKey::BootCount.to_canonical_key();
    config.first_boot = match reader.resolve_dotted_str(&ctx, &boot_count_key) {
        DottedResolution::Value(_) => false,
        DottedResolution::NotFound => true,
    };

    let vanish_count_key = PersistKey::VanishCount.to_canonical_key();
    config.vanish_count = match reader.resolve_dotted_str(&ctx, &vanish_count_key) {
        DottedResolution::Value(raw) => raw.parse::<u32>().unwrap_or_else(|_| {
            tracing::warn!(
                target: "ghost-boot",
                key = %vanish_count_key,
                raw = %raw,
                "areka.vanish.count が非数値——0 へ寛容縮退する（起動は止めない・要件 6.3）"
            );
            0
        }),
        DottedResolution::NotFound => 0,
    };

    if config.first_boot {
        config.first_boot_epilogue = vec![EpilogueCommand {
            name: PROP_SET_CUE_NAME.to_string(),
            tokens: vec![boot_count_key, "1".to_string()],
        }];
    }

    config
}

/// descript.txt 起点で全エンジンを起動順に結線する。
///
/// 手順: マウント解決 → 運行設定解決 → shiori actor → kanade → sakura dispatcher
/// （＋start/down relay）→ ticker（`TickerMode::Real` 時のみ）→ `KanadeMsg::Boot` 送出。
///
/// マウント解決が失敗したら、何も spawn しないうちに `error!` の上で
/// `Err(GhostBootError::Mount(_))` を返す。
pub fn boot(options: GhostBootOptions) -> Result<GhostRuntime, GhostBootError> {
    boot_with_kanade_stop(options, None)
}

/// kanade の停止通知の投函端つきで ghost を起動する。
///
/// [`boot`] の末尾に `kanade_stop` が 1 つ増えるだけの派生（[`boot`] は `None` を渡す）。
/// [`GhostBootOptions`] は struct リテラルで組まれ、フィールドを増やすと全構築点の書き換えに
/// なるので、[`areka_kanade::spawn_kanade_with_stop_sink`] と同じく派生関数にしている。
///
/// `Some` を渡すと、kanade の終了系列が完了した時点で [`KanadeNotice::Stopped`] が 1 件届く。
/// 受け手（UI の毎フレーム結線）はそれを合図に全ゴースト窓を閉じる。`None` なら通知は出ない。
pub fn boot_with_kanade_stop(
    options: GhostBootOptions,
    kanade_stop: Option<Sender<KanadeNotice>>,
) -> Result<GhostRuntime, GhostBootError> {
    boot_with_origin(options, kanade_stop, BootOrigin::Plain, None, None)
}

/// 起動の由来つきで ghost を起動する。
///
/// [`boot_with_kanade_stop`] の末尾に `origin` が 1 つ増えるだけの派生（[`GhostBootOptions`] に
/// 欄を足さない）。起動記録の判定の後で、kanade の設定へ由来と
/// シェルのフォルダ名（マウントしたシェルのフォルダの末尾・取れなければシェル名のまま）を
/// 詰めてから kanade を起こす。切替で来たなら `OnGhostChanged`（Ref7＝シェルのフォルダ名）、
/// 前回落ちたなら `OnBoot` の Ref6/Ref7 に現れる。
///
/// `shell` は起動するシェルのフォルダ名。`Some(名)` なら `shell/<名>/` をマウントし、
/// `OnBoot` の Ref0 もそのシェルの名前になる。`None` は `seriko.defaultsurfacedirectoryname`、
/// 無ければ `master`。
///
/// `persist_failures` はゴーストの記憶の書き手の失敗の受け口。`Some` なら、書き手が
/// 保存に失敗するたびに書けなかった中身（[`areka_sylphya::PersistFailure`]）が 1 件届く。
/// 書き手を起こすときに渡すので、起動のトークの最中の書き込みも取りこぼさない。`None` なら送らない。
pub fn boot_with_origin(
    options: GhostBootOptions,
    kanade_stop: Option<Sender<KanadeNotice>>,
    origin: BootOrigin,
    shell: Option<&str>,
    persist_failures: Option<Sender<areka_sylphya::PersistFailure>>,
) -> Result<GhostRuntime, GhostBootError> {
    boot_with_balloon(options, kanade_stop, origin, shell, None, persist_failures)
}

/// バルーンのフォルダつきで ghost を起動する。
///
/// [`boot_with_origin`] に `balloon_dir` が 1 つ増えるだけの派生（[`boot_with_origin`] は `None` を渡す）。
/// `Some(フォルダ)` なら sylphya のバルーンの層の置き場を `<フォルダ>/profile/areka/` にする。
/// `None` ならバルーンの層の置き場は無い。
pub fn boot_with_balloon(
    mut options: GhostBootOptions,
    kanade_stop: Option<Sender<KanadeNotice>>,
    origin: BootOrigin,
    shell: Option<&str>,
    balloon_dir: Option<&Path>,
    persist_failures: Option<Sender<areka_sylphya::PersistFailure>>,
) -> Result<GhostRuntime, GhostBootError> {
    let mount = match resolve_with_shell(&options.ghost_root, options.default_encoding, shell) {
        Ok(mount) => mount,
        Err(err) => {
            tracing::error!(
                target: "ghost-boot",
                ghost_root = %options.ghost_root.display(),
                shell = ?shell,
                error = ?err,
                "mount resolution failed; boot aborted before spawning any component"
            );
            return Err(GhostBootError::Mount(err));
        }
    };

    let config = resolve_kanade_config(&mount, options.default_encoding);
    // strict の記録の束ね（名は kanade の `script` の記録と同じ作り）。
    let strict = crate::config::strict_hooks(&mount, options.strict_judges.take());

    // app の置き場だけは bin が与え、ほかは `<フォルダ>/profile/areka/`。
    let scope_roots = areka_sylphya::ScopeRoots {
        app: options.app_profile_dir.clone(),
        ghost: Some(crate::sylphya_wiring::profile_areka_root(&mount.shiori.dir)),
        shell: Some(crate::sylphya_wiring::profile_areka_root(&mount.shell.dir)),
        balloon: balloon_dir.map(crate::sylphya_wiring::profile_areka_root),
    };

    // 新規ゴーストでは ghost profile root が無く、初回の Ghost スコープの書きが
    // os error 3（NotFound）で Degraded に倒れ、位置・起動記録が保存されない。先に作る
    // （書きの時にも作り直す）。失敗しても boot は止めない。
    if let Some(ghost_root) = scope_roots.ghost.as_ref()
        && let Err(err) =
            areka_sylphya::persist::ensure_root_dir(ghost_root, &areka_sylphya::store::fs_backend())
    {
        tracing::warn!(
            target: "ghost-boot",
            path = %ghost_root.display(),
            error = %err,
            "ghost profile root の作成に失敗——継続（書きの時に作り直す）"
        );
    }

    let areka_sylphya::SylphyaParts {
        reader: sylphya_reader,
        publisher: sylphya_publisher,
        handle: sylphya_handle,
    } = crate::sylphya_wiring::spawn_ghost_sylphya(scope_roots, persist_failures);

    // ghost 自身の AskerId（`mount.shiori.dir` 由来・provider／prefetch sink が共有）。
    let ghost_asker = crate::sylphya_wiring::ghost_asker_id(&mount.shiori.dir);

    let mut config = apply_boot_record_gate(config, &sylphya_reader, &ghost_asker);

    // 選択待ちの起こしの線。刻み役が本物のときだけ作る（`Disabled` は毎秒の Tick を外から
    // 注入するテストなので `None` のまま）。
    let choice_wake_rx = match options.ticker {
        TickerMode::Real(_) => {
            let (wake_tx, wake_rx) = mpsc::channel::<dola::TimePoint>();
            config.choice_wake = Some(wake_tx);
            Some(wake_rx)
        }
        TickerMode::Disabled => None,
    };

    // フォルダ名が取れなければ `KanadeConfig::new` が写したシェル名のまま。
    config.boot_origin = origin;
    if let Some(folder) = mount.shell.dir.file_name() {
        config.shell_folder = folder.to_string_lossy().into_owned();
    }

    // 静的構成の publish（selfname 系と baseware 2 項。version は areka-ghost の CARGO_PKG_VERSION）。
    // 投函のみで、反映は prefetch sink の barrier が待つ。
    crate::sylphya_wiring::publish_ghost_statics(
        &sylphya_publisher,
        ghost_asker.clone(),
        &mount.names,
        env!("CARGO_PKG_VERSION"),
    );

    // kanade → talk 再生系は [`TalkCommand`] の単一チャンネル。起動・選択解決・選択解除の 3 形が
    // 同じチャンネル・単一 relay・dispatcher の単一 inbox を流れるので、FIFO 順が保たれる。
    let (start_tx, start_rx) = mpsc::channel::<areka_kanade::TalkCommand>();
    let (down_tx, down_rx) = mpsc::channel::<KanadeMsg>();

    let connect: Box<dyn FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send> =
        match options.shiori {
            ShioriWiring::Helper { helper_exe } => Box::new(crate::shiori_wiring::real_connect(
                helper_exe,
                mount.shiori.clone(),
                options.default_encoding,
            )),
            ShioriWiring::Custom(connect) => connect,
            ShioriWiring::InProc => {
                Box::new(crate::shiori_inproc::inproc_connect(mount.shiori.clone()))
            }
        };

    // 見張り部品は実行系に持たせる（`shutdown_within` が張る）。
    let (shiori_tx, shiori_handle, shiori_probe) = spawn_shiori_actor(connect, down_tx);

    // prefetch（username GET）の応答を sylphya へ反映する `ResourceSink`。publish_shiori
    // （Value→Some／204・失敗→None）の投函後に barrier で反映を待って返るので、初回 talk の前に
    // 反映が済む。
    let resource_sink = crate::sylphya_wiring::make_username_resource_sink(
        sylphya_publisher.clone(),
        ghost_asker.clone(),
    );
    // 写しの源は kanade の起動の前に翻訳用と再生用の 2 つに分ける。FromSylphya は同じ reader と
    // 自 asker から 1 つずつ（翻訳用は再生用の固定の記録を出さない）、Custom は注入された 1 つを
    // `split_source` で分ける。再生用は dispatcher が talk 起動ごとに呼ぶ。
    let (translate_source, system_var_source) = match options.system_vars {
        SystemVarWiring::FromSylphya => (
            crate::sylphya_wiring::translate_snapshot_source(
                sylphya_reader.clone(),
                ghost_asker.clone(),
            ),
            crate::sylphya_wiring::from_sylphya_provider(
                sylphya_reader.clone(),
                ghost_asker.clone(),
            ),
        ),
        SystemVarWiring::Custom(src) => crate::translate_wiring::split_source(src),
    };
    // 展開は写しの源から組み、MAKOTO は素通し。
    let seams = TranslateSeams {
        expand: crate::translate_wiring::make_script_expander(translate_source),
        makoto: TranslateSeams::passthrough().makoto,
    };
    let (kanade_tx, kanade_handle) = spawn_kanade_translating(
        config,
        shiori_tx,
        start_tx,
        resource_sink,
        kanade_stop,
        seams,
    );

    // 起動記録 SET sink の登録点。`spawn_dispatcher` の直前の 1 か所なので、wired／fallback の
    // 両ブート経路がここを通る。
    options
        .sinks
        .push(Box::new(PropSetCueSink::new(sylphya_publisher.clone())));
    // `\![get,property]` の受け口も同じ所で足す（MCP と同じ問い手）。
    options.sinks.push(Box::new(PropGetCueSink::new(
        sylphya_reader.clone(),
        ghost_asker.clone(),
        kanade_tx.clone(),
    )));
    let (dispatcher_tx, dispatcher_handle) =
        spawn_dispatcher(kanade_tx.clone(), options.sinks, system_var_source, strict);

    let start_relay_handle = spawn_relay::<areka_kanade::TalkCommand, DispatcherMsg>(
        "start-relay",
        start_rx,
        dispatcher_tx.clone(),
    );
    let down_relay_handle =
        spawn_relay::<KanadeMsg, KanadeMsg>("down-relay", down_rx, kanade_tx.clone());

    let (ticker_tx, ticker_handle) = match options.ticker {
        TickerMode::Real(cfg) => {
            let (tx, handle) = spawn_ticker(cfg, kanade_tx.clone(), dispatcher_tx.clone());
            (Some(tx), Some(handle))
        }
        TickerMode::Disabled => (None, None),
    };
    // 起こしの中継（kanade の頼み → `TickerMsg::WakeKanadeAt`）は刻み役の起動の後につなぐ。
    let choice_wake_relay_handle = choice_wake_rx
        .zip(ticker_tx.clone())
        .map(|(rx, tx)| spawn_relay::<dola::TimePoint, TickerMsg>("choice-wake-relay", rx, tx));

    // kanade は直前に spawn したばかりなので生きているはずだが、送出に失敗しても panic せず記録だけ残す。
    if kanade_tx.send(KanadeMsg::Boot).is_err() {
        tracing::error!(
            target: "ghost-boot",
            "failed to send initial KanadeMsg::Boot; kanade actor appears to be gone already"
        );
    }

    tracing::info!(target: "ghost-boot", "ghost boot sequence completed");

    Ok(GhostRuntime {
        kanade_tx,
        dispatcher_tx,
        ticker_tx,
        kanade_handle,
        dispatcher_handle,
        shiori_handle,
        start_relay_handle,
        down_relay_handle,
        choice_wake_relay_handle,
        ticker_handle,
        sylphya_publisher,
        sylphya_reader,
        sylphya_handle,
        mount,
        shiori_probe,
    })
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "runtime_origin_tests.rs"]
mod origin_tests;

#[cfg(test)]
#[path = "runtime_within_tests.rs"]
mod within_tests;

#[cfg(test)]
#[path = "runtime_shell_tests.rs"]
mod shell_tests;

#[cfg(test)]
#[path = "runtime_layer_root_tests.rs"]
mod layer_root_tests;

#[cfg(test)]
#[path = "runtime_names_tests.rs"]
mod names_tests;

#[cfg(test)]
#[path = "runtime_choice_wake_tests.rs"]
mod choice_wake_tests;
