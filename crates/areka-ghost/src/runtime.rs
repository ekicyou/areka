//! ghost 結線層の起動・終了統括（`GhostRuntime`／`boot`／`shutdown`）。
//!
//! task 3.1 で `boot` 手順と `GhostRuntime`（`kanade()`／`dispatcher()` の
//! 投函端アクセサのみ）を実装した。`shutdown`／`into_parts`（`GhostParts`
//! を含む終了統括一式）は task 3.2 が同じ `GhostRuntime` へ追加実装する
//! （`GhostShutdownError` もそちらで定義する）。

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

use areka_actor::ActorHandle;
use areka_kanade::{KanadeMsg, ShioriBackend, spawn_kanade, spawn_shiori_actor};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{MountError, MountModel, resolve};
use areka_sakura::sink::{SurfaceSink, TextSink};

use crate::config::resolve_kanade_config;
use crate::dispatcher::{DispatcherMsg, spawn_dispatcher};
use crate::relay::spawn_relay;
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

/// ticker 起動方式（design.md「ghost::runtime」Service Interface）。
pub enum TickerMode {
    /// 実クロック駆動（本番）。
    Real(TickerConfig),
    /// ticker を起動しない（決定論テスト＝Tick は外部注入・要件 5.4）。
    Disabled,
}

/// `boot` の入力一式（design.md「ghost::runtime」Service Interface）。
pub struct GhostBootOptions<S, T> {
    /// descript.txt 起点のマウント解決対象ディレクトリ。
    pub ghost_root: PathBuf,
    /// charset 未宣言時の既定エンコーディング（既定 Ansi・SSP 準拠・記憶
    /// areka-descript-encoding）。
    pub default_encoding: DefaultEncoding,
    /// SHIORI 結線方式。
    pub shiori: ShioriWiring,
    /// サーフェス側 sink（構築時注入・要件 4.6）。
    pub surface_sink: S,
    /// テキスト側 sink（構築時注入・要件 4.6）。
    pub text_sink: T,
    /// ticker 起動方式。
    pub ticker: TickerMode,
}

/// ghost 結線層が起動した全コンポーネントの所有者。
///
/// 本タスク（3.1）では [`GhostRuntime::kanade`]／[`GhostRuntime::dispatcher`]
/// のみを公開する。`shutdown`／`into_parts`（`GhostParts` を含む）は task 3.2
/// が同じ構造体へ追加実装する。保持しているフィールドのうち `kanade_tx`／
/// `dispatcher_tx` 以外は現時点では未使用（task 3.2 の終了統括が消費する）
/// ため `dead_code` を明示的に許容する（design.md「保持物」節・机上で
/// 削らない——task 3.2 が必要とする形をここで先に確定させる）。
#[allow(dead_code)]
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
    mount: MountModel,
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
pub fn boot<S, T>(options: GhostBootOptions<S, T>) -> Result<GhostRuntime, GhostBootError>
where
    S: SurfaceSink + Clone + Send + 'static,
    T: TextSink + Clone + Send + 'static,
{
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

    // 7. sakura dispatcher（sink は構築時注入・要件 4.6）。
    let (dispatcher_tx, dispatcher_handle) =
        spawn_dispatcher(kanade_tx.clone(), options.surface_sink, options.text_sink);

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
    use areka_sakura::contract::TalkCue;
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

    impl SurfaceSink for NoopSink {
        fn emit(&mut self, _cue: TalkCue) {}
    }

    impl TextSink for NoopSink {
        fn emit(&mut self, _cue: TalkCue) {}
    }

    /// シナリオ1（happy path）: 解決可能な `ghost_root`・`ShioriWiring::Custom`（fake
    /// backend）・`TickerMode::Disabled` で `boot` すると `Ok(GhostRuntime)` が返り、
    /// `kanade()`／`dispatcher()` の両方の投函端が生きている（＝実際にアクタースレッドが
    /// 起動し受信ループへ入っている）ことを send の成功で確認する（要件 2.1/2.2/2.4）。
    ///
    /// `shutdown()` はまだ存在しない（task 3.2）ため、明示的な join は行わない——
    /// `ActorHandle` は非 RAII（detached）であり、テストプロセス終了時にスレッドが
    /// ブロックしたまま回収されるのは想定どおり（design.md「保持物」節・task の
    /// 制約どおり）。
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
            surface_sink: NoopSink,
            text_sink: NoopSink,
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
            surface_sink: NoopSink,
            text_sink: NoopSink,
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
}
