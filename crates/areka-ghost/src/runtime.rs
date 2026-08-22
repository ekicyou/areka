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

    // 永続書込先ディレクトリの即時保証（実機初回起動サインオフで判明した恒久修正・要件 6.3）:
    // position-persist は M1 初の永続書込を導入した——sylphya は「M1 本番経路に永続書込呼出は無い」
    // read-only 前提で完了しており、ghost profile root（`<shiori.dir>/profile/areka/`）を作る責任者が
    // 結線とストアの狭間に落ちていた。新規ゴーストでこの dir が無いと初回の全 Ghost スコープ commit が
    // os error 3（NotFound）で Degraded に倒れ、位置・起動記録が保存されない。ここで先に作る
    // （`FsPersistIo::commit` も commit 時に `create_dir_all` する二重の安全網）。失敗しても boot は
    // 止めない（warn＋継続・log-first・6.3——FsPersistIo が commit 時に再試行する）。
    if let Some(ghost_root) = scope_roots.ghost.as_ref() {
        if let Err(err) = std::fs::create_dir_all(ghost_root) {
            tracing::warn!(
                target: "ghost-boot",
                path = %ghost_root.display(),
                error = %err,
                "ghost profile root の作成に失敗——継続（FsPersistIo が commit 時に再試行する・6.3）"
            );
        }
    }

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
    //    kanade → talk 再生系は [`TalkCommand`] の**単一チャンネル**（DD-5）——起動・選択解決・
    //    選択解除の 3 形が同一チャンネル＋単一 relay ＋ dispatcher 単一 inbox を流れることで
    //    FIFO 順序が保存される（`areka-talk` の `TalkCommand` doc に契約として明記）。
    //    結線トポロジ自体は不変（relay は従来どおり 1 本）。
    let (start_tx, start_rx) = mpsc::channel::<areka_kanade::TalkCommand>();
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
        SystemVarWiring::FromSylphya => crate::sylphya_wiring::from_sylphya_provider(
            sylphya_reader.clone(),
            ghost_asker.clone(),
        ),
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
    let start_relay_handle = spawn_relay::<areka_kanade::TalkCommand, DispatcherMsg>(
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
#[path = "runtime_tests.rs"]
mod tests;
