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
use areka_kanade::{KanadeConfig, KanadeMsg, ShioriBackend, spawn_kanade, spawn_shiori_actor};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{MountError, MountModel, resolve};
use areka_sakura::contract::SystemVarSnapshot;
use areka_sylphya::{
    AskerContext, AskerId, DottedResolution, PersistKey, SylphyaPublisher, SylphyaReader,
};
use areka_talk::EpilogueCommand;

use crate::config::resolve_kanade_config;
use crate::dispatcher::{DispatcherMsg, spawn_dispatcher};
use crate::prop_sink::{PROP_SET_CUE_NAME, PropSetCueSink};
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
    /// 正規 in-proc x64 SHIORI4 ロード結線（第 3 の正規結線・要件 1.1/3.1/7.1）。
    ///
    /// `Helper`（別プロセス 32bit helper）／`Custom`（closure 注入 fake）と**同列**に選べる
    /// 第 3 の SHIORI 結線方式。`mount.shiori.dir.join(file)` で解決した x64 DLL を in-proc に
    /// ロードし、SHIORI4 生成入口（`shiori_factory`）→ `IShiori` → `ShioriBackend` へ
    /// [`crate::shiori_inproc::inproc_connect`] が写像する（要件 3.1）。ユニット variant であり
    /// テスト専用パラメータを持たない——DLL パスはマウント解決結果から本番同型に導出される
    /// （design.md D-1）。M2 の native x64 SHIORI4 がそのまま本番消費者として再利用する正規
    /// シームである（要件 7.1・第一級の布石）。
    InProc,
}

/// per-talk に呼ばれ、システム変数の凍結スナップショットを返す供給シーム
/// （design.md「GhostBootOptions S-3＋provider」・R7.3）。
///
/// dispatcher が talk 起動ごとに一度だけ呼び出し、返ったスナップショットを per-talk の
/// `spawn_talk` へ手渡す（**凍結像の刻印点**＝talk ごと凍結の意味論・sylphya の per-talk
/// 凍結と同形）。task 8.2 で本番既定は sylphya 読み口由来（[`SystemVarWiring::FromSylphya`]）へ
/// 移行したが、sakura 側の契約（[`SystemVarSnapshot`]）は無改変のまま——変わるのはスナップショットの
/// **源**（sylphya 鏡像）だけ（R7.1/R2.2）。テスト・特殊用途は [`SystemVarWiring::Custom`] で
/// 従来どおりこのクロージャを直接注入する。
pub type SystemVarSource = Box<dyn Fn() -> SystemVarSnapshot + Send>;

/// システム変数 provider の結線方式（design「ghost（結線・provider 差替）」Service Interface・R7.1）。
///
/// dispatcher の刻印点（[`SystemVarSource`]）は無改変のまま、その **源** を選ぶ:
/// - [`FromSylphya`](SystemVarWiring::FromSylphya): 本番既定。boot が内部で据えた sylphya
///   reader ＋自 `AskerId` を捕捉し、`talk_snapshot` を [`SystemVarSnapshot`] へ写像する provider を
///   構築する（[`crate::sylphya_wiring::from_sylphya_provider`]）。
/// - [`Custom`](SystemVarWiring::Custom): テスト・特殊用途の直接注入（従来の [`SystemVarSource`]）。
///
/// いずれの場合も boot は sylphya アクターを起動して静的構成／username を publish する——`Custom`
/// でも sylphya は生きており、provider の源だけが差し替わる（8.3/8.4 のテストは `Custom` を使う）。
pub enum SystemVarWiring {
    /// 本番既定: boot が内部で据えた sylphya reader からスナップショットを生成する（R7.1）。
    FromSylphya,
    /// テスト・特殊用途の注入（型は従来の [`SystemVarSource`]）。
    Custom(SystemVarSource),
}

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
    /// システム変数 provider の結線方式（S-3・R7.1/7.3）。本番は
    /// [`SystemVarWiring::FromSylphya`]（boot が据えた sylphya reader 由来）、テストは
    /// [`SystemVarWiring::Custom`]（従来の [`SystemVarSource`] 直接注入）。dispatcher の刻印点は
    /// 無改変で、選ぶのはスナップショットの源だけ（R7.1/R2.2）。
    pub system_vars: SystemVarWiring,
    /// App スコープの永続 root（sylphya の App 層 profile フォルダ・bin が供給・R6.5）。
    ///
    /// `None` は App スコープ利用不可（不在縮退）。ghost／shell スコープは mount 解決結果から
    /// 導く（`<shiori.dir>/profile/areka/`・`<shell.dir>/profile/areka/`）が、App スコープは
    /// マウントに現れないため呼び出し側（bin）が供給する（既定＝実行ファイル隣接 `profile/areka/`・
    /// env `AREKA_PROFILE_DIR` で上書き可・R8.2）。
    pub app_profile_dir: Option<PathBuf>,
    /// ticker 起動方式。
    pub ticker: TickerMode,
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
    /// sylphya（統一プロパティシステム）供給端。shutdown の `Close` 送出に使う。
    sylphya_publisher: SylphyaPublisher,
    /// sylphya 読み口（provider が捕捉するのと同一鏡像を共有・test 検証／後続用途で保持）。
    sylphya_reader: SylphyaReader,
    /// sylphya アクターの join ハンドル。shutdown の最終段で join して panic を観測する。
    sylphya_handle: ActorHandle,
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
    /// sylphya アクターの join ハンドル（掲示板供給者・shutdown 最終段で join）。
    pub sylphya: ActorHandle,
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
    /// sylphya 供給端（`Close` 送出・手動解体で掲示板を畳む）。
    pub sylphya: SylphyaPublisher,
    /// sylphya 読み口（手動解体時の値検証・provider と同一鏡像を共有）。
    pub sylphya_reader: SylphyaReader,
    /// kanade／dispatcher／shiori／start-relay／down-relay／ticker(Option)／sylphya の全 `ActorHandle`。
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

    /// sylphya（統一プロパティシステム）供給端への参照（design.md「C5 GhostRuntime 増分」・
    /// requirements.md 6.2）。`kanade()`／`dispatcher()` と同型の additive アクセサで、main が
    /// この clone を捕捉して `PersistWiring`（位置永続の write-through 端）を組む。
    pub fn sylphya_publisher(&self) -> &SylphyaPublisher {
        &self.sylphya_publisher
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
            sylphya_publisher,
            sylphya_reader: _,
            sylphya_handle,
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

        // 10. sylphya Close＋join（既存段の後・供給者停止後に掲示板を畳む・design「shutdown」）。
        //     供給者（ghost 静的 publish は boot 済み・kanade は上流で既に停止）が全て止まった後に
        //     掲示板を畳む。既に停止済みへの再送は SylphyaPublisher が warn＋縮退（冪等・非 panic）。
        //
        // close() の直前に barrier() で終了時フラッシュを明示確認する（requirements.md 1.2・
        // design.md「C5 GhostRuntime 増分」step 10 直前・E2-lite の安全網）。DragEnd write-through は
        // 既に FIFO で投函済み（E1＝FIFO close が保証の正本）だが、close() の前に反映フェンスを 1 枚置く
        // ことで「終了直前までに投函された永続 put が確実に反映されてからストアが畳まれる」ことを保証する。
        // Ok なら確認 info・Err（アクター既死等）なら warn の上で**続行**する——早期 return も panic も
        // しない（write-through 済みが正本・design「Error Handling」shutdown barrier() Err 行）。
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
            sylphya_publisher,
            sylphya_reader,
            sylphya_handle,
            mount: _,
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
                ticker: ticker_handle,
                sylphya: sylphya_handle,
            },
        }
    }
}

/// 起動記録ゲート（design.md「C5 GhostRuntime 増分」boot() step 1-3・要件 3.1/3.4/4.1/4.2/6.3）。
///
/// boot が据えた sylphya 読み口（`reader`）と ghost 自身の `ghost_asker` から永続鏡像を引き、
/// [`KanadeConfig`] の初回起動系フィールドを解決して返す純ロジック（テスト単体で駆動可能なよう
/// `boot()` から切り出す・記憶知見「判断分岐のみ檻に入れる」）。read のみ・**決して panic しない**
/// （reader は panic-free・全不在は既定へ寛容縮退・要件 6.3「永続読取失敗は起動を止めない」）。
///
/// 手順:
/// 1. **存在ゲート**（要件 3.1/3.4）: `areka.boot.count`（正準文字列は
///    [`PersistKey::BootCount`] から取得＝単一権威）が鏡像に**存在**すれば `first_boot=false`
///    （2 回目以降起動）、不在なら `true`（初回起動）。値の**数値解釈はしない**——存在の有無のみを
///    見る（過剰実装回避・design C5-1）。
/// 2. **vanish 寛容 parse**（要件 4.1/4.2）: `areka.vanish.count` を u32 として寛容 parse し
///    `config.vanish_count` へ。不在→0・**非数値 present は warn の上 0**（起動は止めない・6.3）。
/// 3. **初回起動記録 epilogue 注入**（要件 3.4）: `first_boot==true` のとき、初回挨拶トーク
///    再生完走時に起動記録を書く汎用プロパティ SET キュー 1 件を `config.first_boot_epilogue` へ
///    据える（`[PROP_SET_CUE_NAME, [BootCount 正準 key, "1"]]`）。kanade は正準 key を**不透明搬送**
///    するのみで sylphya へは依存しない（依存方向規律の担保・design C5-3）。
fn apply_boot_record_gate(
    mut config: KanadeConfig,
    reader: &SylphyaReader,
    ghost_asker: &AskerId,
) -> KanadeConfig {
    let ctx = AskerContext {
        asker: ghost_asker.clone(),
    };

    // step 1: 起動記録の**存在**ゲート（値は数値解釈しない・design C5-1・要件 3.1/3.4）。
    let boot_count_key = PersistKey::BootCount.to_canonical_key();
    config.first_boot = match reader.resolve_dotted_str(&ctx, &boot_count_key) {
        // 記録あり（値の中身は問わない）→ 2 回目以降起動。
        DottedResolution::Value(_) => false,
        // 記録なし → 初回起動（既定挙動）。
        DottedResolution::NotFound => true,
    };

    // step 2: vanish 回数の寛容 parse（不在→0・非数値→0＋warn・design C5-2・要件 4.1/4.2/6.3）。
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

    // step 3: 初回起動なら起動記録書込 epilogue を据える（正準 key を不透明搬送・design C5-3・要件 3.4）。
    if config.first_boot {
        config.first_boot_epilogue = vec![EpilogueCommand {
            name: PROP_SET_CUE_NAME.to_string(),
            tokens: vec![boot_count_key, "1".to_string()],
        }];
    }

    config
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
pub fn boot(mut options: GhostBootOptions) -> Result<GhostRuntime, GhostBootError> {
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

    // 2b. sylphya（統一プロパティシステム）起動＋静的構成 publish（task 8.2・design「boot 系列」）。
    //     層別 profile root: ghost＝`<shiori.dir>/profile/areka/`・shell＝`<shell.dir>/profile/areka/`・
    //     app＝bin 供給（`options.app_profile_dir`）・balloon＝None。起動時に全スコープを寛容ロード。
    let scope_roots = areka_sylphya::ScopeRoots {
        app: options.app_profile_dir.clone(),
        ghost: Some(crate::sylphya_wiring::profile_areka_root(&mount.shiori.dir)),
        shell: Some(crate::sylphya_wiring::profile_areka_root(&mount.shell.dir)),
        balloon: None,
    };
    let areka_sylphya::SylphyaParts {
        reader: sylphya_reader,
        publisher: sylphya_publisher,
        handle: sylphya_handle,
    } = crate::sylphya_wiring::spawn_ghost_sylphya(scope_roots);

    // ghost 自身の AskerId（MountModel.shiori.dir 由来の正準文字列・provider／prefetch sink が共有）。
    let ghost_asker = crate::sylphya_wiring::ghost_asker_id(&mount.shiori.dir);

    // 2c. 起動記録ゲート（design「C5 GhostRuntime 増分」boot() step 1-3・要件 3.1/3.4/4.1/4.2/6.3）。
    //     sylphya reader＋ghost_asker が揃った直後に永続鏡像を引き、初回起動ゲート・vanish 回数・
    //     初回起動記録 epilogue を解決した config へ差し替える（read のみ・panic なし・不在は既定縮退）。
    let config = apply_boot_record_gate(config, &sylphya_reader, &ghost_asker);

    // 静的構成層 publish: フラット（selfname 系＝derive_flat_statics）＋大域点付き（baseware 2 項・
    // version＝areka-ghost の CARGO_PKG_VERSION・R5.1）。投函のみ（反映は prefetch sink の barrier で担保）。
    crate::sylphya_wiring::publish_ghost_statics(
        &sylphya_publisher,
        ghost_asker.clone(),
        &mount.names,
        env!("CARGO_PKG_VERSION"),
    );

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
            // 第 3 の正規結線（要件 1.1/3.1/7.1）: `Helper` arm と同型に、mount 解決済みの
            // `ShioriMount` を渡して in-proc x64 DLL ロード connect closure を構成する。
            ShioriWiring::InProc => {
                Box::new(crate::shiori_inproc::inproc_connect(mount.shiori.clone()))
            }
        };

    // 5. shiori actor。
    let (shiori_tx, shiori_handle) = spawn_shiori_actor(connect, down_tx);

    // 6. kanade（start_tx を「自身の」sakura Sender として渡す）。
    //    task 8.2: prefetch 段（username GET）の応答を sylphya へ反映する実 `ResourceSink` を注入する。
    //    sink は publish_shiori（Value→Some／204・失敗→None）投函後に barrier で反映完了を待って返る
    //    （初回 talk 前の反映順序を決定論化・R4.1/R4.2）。
    let resource_sink = crate::sylphya_wiring::make_username_resource_sink(
        sylphya_publisher.clone(),
        ghost_asker.clone(),
    );
    let (kanade_tx, kanade_handle) = spawn_kanade(config, shiori_tx, start_tx, resource_sink);

    // 7. sakura dispatcher（可変長 sink 列＋system_vars provider を構築時注入・S-3・
    //    要件 4.6/8.5/7.1）。provider は dispatcher が talk 起動ごとに呼び出す（刻印点＝無改変）。
    //    provider の源を解決: FromSylphya＝reader＋自 asker 捕捉クロージャ（talk_snapshot→SystemVarSnapshot）／
    //    Custom＝注入された SystemVarSource をそのまま（R7.1・design「provider 差替」）。
    let system_var_source: SystemVarSource = match options.system_vars {
        SystemVarWiring::FromSylphya => {
            crate::sylphya_wiring::from_sylphya_provider(sylphya_reader.clone(), ghost_asker.clone())
        }
        SystemVarWiring::Custom(src) => src,
    };
    // 起動記録 SET sink 登録（design「C5 GhostRuntime 増分」boot() step 4・要件 3.4/6.2/7.1）。
    //     `spawn_dispatcher` 直前の単一登録点——wired／fallback 両ブート経路が本 1 点を通るため
    //     自動被覆する（per-path 登録にしない・emo2_boot 不触）。以後 dispatcher が talk ごとに
    //     clone して broadcast し、`areka.prop.set`／カウンタ key を名前自己選別して write-through する。
    options
        .sinks
        .push(Box::new(PropSetCueSink::new(sylphya_publisher.clone())));
    let (dispatcher_tx, dispatcher_handle) =
        spawn_dispatcher(kanade_tx.clone(), options.sinks, system_var_source);

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
        sylphya_publisher,
        sylphya_reader,
        sylphya_handle,
        mount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// task 8.3: 退役した `default_system_vars()` の忠実な代役スタンドイン。
    ///
    /// `{"username": DEFAULT_USERNAME}` のみを充填した凍結スナップショットを毎回新規構築して
    /// 返す（退役前 provider と同一挙動）。既存テストの意図——既定 username 前提の凍結像刻印——を
    /// [`SystemVarWiring::Custom`] 経由で無改変のまま保つための in-crate 共有ヘルパ。sylphya 読み口
    /// （[`SystemVarWiring::FromSylphya`]）差替後も、これら既存テストは従来どおりの直接注入
    /// セマンティクスを `Custom` で維持する（R7.1・R9.1・design「テスト呼出面の一括更新」）。
    fn test_system_vars() -> SystemVarSource {
        Box::new(|| {
            let mut snapshot = SystemVarSnapshot::default();
            snapshot.insert("username", areka_sakura::sysvar::DEFAULT_USERNAME);
            snapshot
        })
    }

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
            _status: Option<&str>,
        ) -> Result<Option<String>, RequestError> {
            Ok(None)
        }

        fn notify(
            &mut self,
            _id: &str,
            _references: &[String],
            _status: Option<&str>,
        ) -> Result<(), RequestError> {
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
            system_vars: SystemVarWiring::Custom(test_system_vars()),
            app_profile_dir: None,
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
            system_vars: SystemVarWiring::Custom(test_system_vars()),
            app_profile_dir: None,
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
            system_vars: SystemVarWiring::Custom(test_system_vars()),
            app_profile_dir: None,
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
            system_vars: SystemVarWiring::Custom(test_system_vars()),
            app_profile_dir: None,
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
            sylphya,
            sylphya_reader: _,
            handles,
        } = parts;
        let GhostHandles {
            kanade: kanade_handle,
            dispatcher: dispatcher_handle,
            shiori: shiori_handle,
            start_relay: start_relay_handle,
            down_relay: down_relay_handle,
            ticker: _,
            sylphya: sylphya_handle,
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

                // sylphya 供給者停止＋join（shutdown() 最終段の手作業ミラー・design「shutdown」step 10）。
                sylphya.close();
                sylphya_handle
                    .join()
                    .expect("sylphya should terminate normally after Close");
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- sylphya_publisher() アクセサ（task 3.1・position-persist） ----

    /// シナリオ（task 3.1・position-persist）: `boot` した `GhostRuntime` から
    /// [`GhostRuntime::sylphya_publisher`] で sylphya 供給端の参照を取得し、その参照経由で
    /// `persist_put`（永続 put の投函）が呼び出せることを確認する（requirements.md 6.2・
    /// design.md「C5 GhostRuntime 増分」——main が `PersistWiring` を組むために公開する
    /// `kanade()`／`dispatcher()` と同型の additive アクセサ）。
    ///
    /// アクセサが返すのは boot が据えた生きた sylphya アクターの供給端であることを、
    /// `persist_put` 投函後に `barrier()` が `Ok(())` を返す（＝アクターが投函を処理して
    /// 反映を完了できた）ことで観測する（fire-and-forget な `persist_put` 自体は戻り値を
    /// 持たないため、直後の反映フェンスで生存を証拠づける）。
    #[test]
    fn sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put() {
        let root = unique_temp_dir(
            "sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put",
        );
        let _ = std::fs::remove_dir_all(&root);
        write_minimal_resolvable_ghost_fixture(&root);

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(|| {
                Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
            system_vars: SystemVarWiring::Custom(test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        // アクセサ経由で取得した供給端で persist_put が呼び出せる（requirements.md 6.2）。
        let publisher: &areka_sylphya::SylphyaPublisher = runtime.sylphya_publisher();
        publisher.persist_put(
            areka_sylphya::PersistScope::Ghost,
            vec![(areka_sylphya::PersistKey::BootCount, "1".into())],
        );

        // 反映フェンスで、アクセサが返したのが生きた供給端であることを観測する
        // （投函が処理され反映が完了 → sylphya アクター生存）。宙吊り防止に有界化。
        run_bounded(
            "barrier after persist_put via sylphya_publisher()",
            std::time::Duration::from_secs(10),
            move || {
                runtime
                    .sylphya_publisher()
                    .barrier()
                    .expect("sylphya actor should process the persist_put and reflect it");
                // アクター後片付け（宙吊りスレッド回避のため clean shutdown）。
                let _ = runtime.shutdown(areka_kanade::CloseReason::System);
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- shutdown() の barrier() 明示フラッシュ確認（task 3.2・position-persist） ----

    /// シナリオ（task 3.2・position-persist）: `boot` した `GhostRuntime` を
    /// `shutdown(CloseReason::System)` すると、sylphya `close()`（step 10）の**直前**に
    /// `barrier()` が呼ばれ、成功時に固定 info ログ `persist flush confirmed`
    /// （target `ghost-shutdown`）を発行する（requirements.md 1.2・design.md「C5 GhostRuntime
    /// 増分」step 10 直前・Monitoring「persist flush confirmed（C5 info）」）。
    ///
    /// write-through（FIFO close＝E1）が保証の正本で、この `barrier()` は best-effort な
    /// 終了時フラッシュ安全網（E2-lite）。ログ発行は「barrier が Ok を返し、その位置が close の
    /// 直前である」ことの直接証跡になる（barrier→info→close のコード配置で順序が担保される）。
    ///
    /// `capture` はスレッドローカルに subscriber を差すため、`shutdown` を走らせる同一スレッド
    /// （`run_bounded` の spawn 先）で発行された info ログを捕捉する。join の宙吊りに備え有界化する。
    #[test]
    fn shutdown_confirms_persist_flush_via_barrier_before_close() {
        use crate::test_log_capture::{assert_logged, capture};

        let root = unique_temp_dir("shutdown_confirms_persist_flush_via_barrier_before_close");
        let _ = std::fs::remove_dir_all(&root);
        write_minimal_resolvable_ghost_fixture(&root);

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(|| {
                Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
            system_vars: SystemVarWiring::Custom(test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        run_bounded(
            "shutdown with barrier flush confirmation",
            std::time::Duration::from_secs(10),
            move || {
                let events = capture(|| {
                    let result = runtime.shutdown(areka_kanade::CloseReason::System);
                    // Err パス（barrier がアクター既死で Err）でも panic せず続行するのが正で、
                    // 生きた sylphya アクターに対する本 happy path は必ず Ok を返す（要件 6.1/6.4）。
                    assert!(
                        result.is_ok(),
                        "shutdown should return Ok(()) with a live sylphya actor, got {result:?}"
                    );
                });
                // barrier() が Ok を返し、その確認ログが close() の直前で発行されたことの直接証跡。
                assert_logged(
                    &events,
                    tracing::Level::INFO,
                    "ghost-shutdown",
                    "persist flush confirmed",
                );
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- InProc 結線の生成〜駆動〜終了 統合テスト（task 2.4） ----

    use std::sync::{Arc, Mutex};

    /// task 2.4 専用のローカル記録 sink（`dispatcher.rs`／spine e2e の `RecordingSink` と同型・
    /// `Clone` 可能で全 cue を `Arc<Mutex<Vec<TalkCue>>>` へ蓄積する）。tests バイナリ側の
    /// `RecordingSink` は runtime.rs の in-crate 檻からは import できないため、ここでローカルに
    /// 定義し直す（`areka-bin-crate-internal-tests-in-crate` の流儀）。
    #[derive(Clone)]
    struct RecordingSink {
        records: Arc<Mutex<Vec<TalkCue>>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                records: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
            Arc::clone(&self.records)
        }
    }

    impl CueSink for RecordingSink {
        fn emit(&mut self, cue: TalkCue) {
            self.records
                .lock()
                .expect("records mutex poisoned")
                .push(cue);
        }
    }

    /// シナリオ5（task 2.4・第 3 の結線 `ShioriWiring::InProc` の生成〜駆動〜終了 一気通貫）:
    /// 実ビルド済み x64 テスト DLL（`shiori4_testdll.dll`）を fixture の `ghost/master/` へ配置し、
    /// `ShioriWiring::InProc`（他 2 方式＝`Helper`／`Custom` と同列に選べる第 3 の正規結線・
    /// 要件 1.1/3.1/7.1）で boot する。boot が内部送出する `KanadeMsg::Boot` を起点に、実 DLL の
    /// OnBoot 応答（挨拶 Value）が kanade→start-relay→dispatcher→sakura→`RecordingSink` へ届く
    /// までを Tick 注入のみ（sleep 不使用・要件 7.3——注入時刻のみで前進）で駆動し、`RecordingSink`
    /// に少なくとも 1 件の `TalkCue` が捕捉されること（＝実 InProc DLL 境界を横断して挨拶が sink まで
    /// 到達した＝生成〜駆動）を確認する。続いて `shutdown(CloseReason::System)` が `Ok(())` を返す
    /// こと（＝正規 clean close・終了）を有界待機で確認する。本番 main 結線・機種自動判別（要件
    /// 7.2/7.3）には一切触れない——`boot()` の match へ `InProc` arm を 1 本足すのみ。
    ///
    /// 本テストは**結線 smoke**（第 3 arm の end-to-end 成立の証明）に留める。OnBoot cue 列の
    /// 全順序照合＋ドリフト検出の厳密 e2e は task 5.1 の担当であり、ここでは「≥1 cue 捕捉＋clean
    /// shutdown」で足りる（重複させない）。
    #[test]
    fn inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll() {
        // 実ビルド済み cdylib を deps ディレクトリから locate する（`shiori_inproc.rs` の
        // happy_path 檻と同一導出・design.md D-1）。不在時は silent skip せず明示 panic。
        let test_exe = std::env::current_exe().expect("test executable path is available");
        let deps_dir = test_exe
            .parent()
            .expect("test executable resides in a deps directory");
        let built_dll = deps_dir.join(shiori4_testdll::DLL_FILE_NAME);
        assert!(
            built_dll.exists(),
            "built test DLL が正準位置に不在: {}\n\
             この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置（deps）へ出力する。\
             単独実行時は先に `cargo test --workspace`（または `cargo build -p shiori4-testdll`）を\
             実行すること（フォールバックは設けない・design.md D-1）。",
            built_dll.display()
        );

        let root =
            unique_temp_dir("inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll");
        let _ = std::fs::remove_dir_all(&root);

        // fixture: ghost/master/descript.txt（`shiori,` 行＝テスト DLL 名）＋ shell/master/descript.txt
        // （shell dir 存在チェックを通すための最小 descript）。
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            format!(
                "charset,UTF-8\nname,Shiori4TestGhost\nshiori,{}\nseriko.defaultsurfacedirectoryname,master\n",
                shiori4_testdll::DLL_FILE_NAME
            )
            .as_bytes(),
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            b"charset,UTF-8\nname,Shiori4TestShell\n",
        )
        .expect("write shell descript.txt");

        // 実 cdylib を fixture の ghost/master/ へコピーする（`inproc_connect` が
        // `mount.shiori.dir.join(file)` でロードする位置・D-1）。
        std::fs::copy(&built_dll, ghost_master.join(shiori4_testdll::DLL_FILE_NAME))
            .expect("copy built test DLL into ghost/master");

        let recording = RecordingSink::new();
        let records = recording.records();

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::InProc,
            sinks: vec![Box::new(recording.clone()), Box::new(recording.clone())],
            system_vars: SystemVarWiring::Custom(test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed through ShioriWiring::InProc");

        // 駆動: dispatcher へ Tick を注入し続け、`RecordingSink` が挨拶 cue を捕捉するまで待つ
        // （sleep 不使用・単調増加 `now` の注入と `yield_now` のみ・S1 spine 技法）。boot が内部で
        // 送った `KanadeMsg::Boot` により OnBoot GET が発火し、StartTalk が
        // start-relay→dispatcher の別スレッド 2 hop を渡り active slot に載った直後の Tick で
        // 挨拶が全 broadcast される。
        //
        // 反復回数ではなく**壁時計デッドライン**で括る点が S1 と異なる（S1 の scripted backend は
        // 即応するが、本 InProc 経路は最初の SHIORI 呼出が shiori アクタースレッド上で実 DLL の
        // `LoadLibraryW`＋`CreateInstance` を初めて走らせ、これに数十 ms を要する）。デッドラインは
        // `run_bounded` の `recv_timeout` と同じく**宙吊り防止の上限**にすぎず、シミュレーション時刻の
        // 前進は依然として注入 Tick のみ（sleep で talk timeline を進めない・要件 7.3）。DLL ロード
        // 完了後は最初の Tick で挨拶が必ず発火するため結果は決定論的。
        let mut now: u64 = 1;
        let mut fired = false;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            runtime
                .dispatcher()
                .send(DispatcherMsg::Tick {
                    now: MonotonicMs(now),
                })
                .expect("dispatcher actor should still be alive while probing for the boot talk");
            now += 1;
            if !records.lock().expect("records mutex poisoned").is_empty() {
                fired = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            fired,
            "InProc: OnBoot 挨拶 cue が Tick 注入後も sink に届かなかった——実 DLL 境界を横断した \
             生成〜駆動が成立していない"
        );

        // 終了: shutdown が clean に完走し `Ok(())` を返すこと（正規 close・有界待機で宙吊り防止）。
        run_bounded(
            "shutdown after InProc boot talk",
            std::time::Duration::from_secs(10),
            move || {
                let result = runtime.shutdown(areka_kanade::CloseReason::System);
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) after InProc boot talk, got {result:?}"
                );
            },
        );

        // ≥1 cue 捕捉の最終 assert（挨拶が実 InProc DLL 境界を越えて sink に届いた・task 2.4 の
        // 観測可能な完了条件）。
        assert!(
            !records.lock().expect("records mutex poisoned").is_empty(),
            "RecordingSink には少なくとも 1 件の TalkCue が捕捉されているべき（挨拶が InProc 境界を越えた）"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- apply_boot_record_gate 単体（task 7.1・position-persist・design「C5 GhostRuntime 増分」step 1-3） ----

    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{save_scope, spawn_sylphya, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts};

    /// 同一 [`FakePersistIo`] を `Arc` 共有する委譲 IO（prop_sink.rs 先例の再実装）。
    ///
    /// `FakePersistIo` は内部 `Mutex` で Clone 不可ゆえ、seed（`save_scope`）と spawn（`build_initial_image`
    /// のロード）が**同一 store** を観測できるよう Arc 共有ハンドルで委譲する。
    #[derive(Clone)]
    struct SharedGateIo(Arc<FakePersistIo>);
    impl PersistIo for SharedGateIo {
        fn read(&self, path: &std::path::Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &std::path::Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

    /// Ghost スコープへ `seed` を事前保存した実 sylphya を起動し、`(parts, ghost_asker)` を返す。
    ///
    /// spawn 時の `build_initial_image` が seed を初期鏡像の大域点付き区画（正準 key）へ投影するため、
    /// `parts.reader` は無待機で `resolve_dotted_str` により seed 値を観測できる（本番 boot と同型の
    /// 「起動時に永続をロード」経路）。seed が空なら不在ケース（記録なし）を表す。
    fn spawn_reader_seeded(seed: Vec<(PersistKey, String)>) -> (SylphyaParts, AskerId) {
        let shared = SharedGateIo(Arc::new(FakePersistIo::new()));
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/gate-ghost")),
            ..ScopeRoots::default()
        };
        if !seed.is_empty() {
            save_scope(PersistScope::Ghost, &roots, &shared, seed);
        }
        let parts = spawn_sylphya(SylphyaInit {
            roots,
            io: Box::new(shared),
            runtime_sink: None,
        });
        (parts, AskerId::new("ghost/gate-asker"))
    }

    /// テスト用の素の config（gate が触る 3 フィールドのみが観測対象・他は既定）。
    fn base_config() -> KanadeConfig {
        KanadeConfig::new("master", "0.0.0-test")
    }

    /// step 1＋3（記録なし）: `areka.boot.count` 不在 → `first_boot=true` かつ
    /// `first_boot_epilogue` に `areka.prop.set`/`areka.boot.count`/"1" が 1 件添付される
    /// （design C5-1/C5-3・要件 3.1/3.4）。
    #[test]
    fn gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue() {
        let (parts, asker) = spawn_reader_seeded(vec![]);

        let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

        assert!(
            gated.first_boot,
            "起動記録（areka.boot.count）不在 → 初回起動（first_boot=true）"
        );
        assert_eq!(
            gated.first_boot_epilogue,
            vec![EpilogueCommand {
                name: PROP_SET_CUE_NAME.to_string(),
                tokens: vec!["areka.boot.count".to_string(), "1".to_string()],
            }],
            "初回は areka.prop.set / areka.boot.count / \"1\" の SET epilogue を 1 件添付する"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// step 1＋3（記録あり）: `areka.boot.count` 存在 → `first_boot=false` かつ epilogue 非添付
    /// （2 回目以降起動・design C5-1/C5-3・要件 3.4）。
    #[test]
    fn gate_present_boot_record_marks_returning_and_no_epilogue() {
        let (parts, asker) = spawn_reader_seeded(vec![(PersistKey::BootCount, "1".into())]);

        let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

        assert!(
            !gated.first_boot,
            "起動記録あり → 2 回目以降起動（first_boot=false）"
        );
        assert!(
            gated.first_boot_epilogue.is_empty(),
            "非初回は起動記録 epilogue を添付しない"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// step 1（存在ゲートは数値解釈しない）: `areka.boot.count="0"` でも「存在」ゆえ
    /// `first_boot=false`（値の中身を問わない存在判定・design C5-1「数値解釈しない」）。
    #[test]
    fn gate_boot_record_is_existence_not_value() {
        let (parts, asker) = spawn_reader_seeded(vec![(PersistKey::BootCount, "0".into())]);

        let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

        assert!(
            !gated.first_boot,
            "存在ゲート: boot.count の値（\"0\"）に関わらず、存在すれば非初回"
        );
        assert!(gated.first_boot_epilogue.is_empty());

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// step 2（vanish 数値 present）: `areka.vanish.count="7"` → `vanish_count==7`（要件 4.1）。
    #[test]
    fn gate_vanish_count_present_numeric_is_parsed() {
        let (parts, asker) = spawn_reader_seeded(vec![
            (PersistKey::BootCount, "1".into()),
            (PersistKey::VanishCount, "7".into()),
        ]);

        let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

        assert_eq!(gated.vanish_count, 7, "areka.vanish.count=\"7\" → 7 が parse される");

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// step 2（vanish 不在）: `areka.vanish.count` 不在 → `vanish_count==0`（既定縮退・要件 4.2）。
    #[test]
    fn gate_vanish_count_absent_defaults_zero() {
        let (parts, asker) = spawn_reader_seeded(vec![]);

        let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

        assert_eq!(gated.vanish_count, 0, "vanish.count 不在 → 0 縮退");

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }

    /// step 2（vanish 非数値）: `areka.vanish.count="abc"` → `vanish_count==0`（寛容縮退・panic せず
    /// 起動を止めない・要件 4.2/6.3）。
    #[test]
    fn gate_vanish_count_non_numeric_degrades_zero() {
        let (parts, asker) = spawn_reader_seeded(vec![
            (PersistKey::BootCount, "1".into()),
            (PersistKey::VanishCount, "abc".into()),
        ]);

        let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

        assert_eq!(
            gated.vanish_count, 0,
            "非数値 vanish.count は 0 へ寛容縮退する（起動は止めない・要件 6.3）"
        );

        parts.publisher.close();
        parts.handle.join().expect("clean close joins");
    }
}
