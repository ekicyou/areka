#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! areka 本番アプリ骨格
//!
//! wintf フレームワークを使用したデスクトップマスコット「ぱすたさん」の本番アプリ骨格。
//! この骨格は「アプリ起動の器」に徹する:
//! - 構造化ロギング初期化（RUST_LOG フォールバック）・パニックハンドラ設定
//! - 構成入力（ゴースト／バルーンのルートパス）の解決とログ出力（マウントはしない）
//! - UI ランタイム起動（`WinApp::new()`）・SHIORI 実走デモの env-gate 呼び口
//! - 起動窓シーム（`open_startup_window`・window-placement task 6.2 で本物のゴースト窓生成へ
//!   差し替え済み。準備失敗時は検証用ダミー窓へフォールバック）
//! - `main` 自身が所有するメッセージループ（`app.run()`）と起動窓 close での正常終了
//!
//! 座標・配置ロジックは `placement` モジュール（areka-P0-window-placement）が所有し、
//! 骨格自身は座標を一切持たない。旧モック UI は `examples/mock-shell.rs` へ退避済み。
//!
//! `main` は `open_startup_window`／ダミー窓／smoke ゲートを不変に保ったまま、`WinApp::new()`
//! ／`open_startup_window` の後で `emo2_boot::wire_emo2_boot` を呼び、その成否で実 sink boot
//! （`wired=true`）／既存 `LogSink`×2 フォールバック boot（`wired=false`）を呼び分ける（task 5.2・
//! design.md「エントリポイント / main.rs＋wire_emo2_boot」・DD-7）。`run()` 復帰後は
//! `GhostRuntime::shutdown(CloseReason::User)`（DD-10）→ seriko `ActorHandle::join` で終了を
//! 総仕上げする。boot 失敗は非致命として扱い骨格起動を止めない（要件 7.3・8.2）。

use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;
use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension};
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::{ChildOf, FrameFinalize, Window, WindowStyle};
use wintf::*;

/// areka 本体側 `IShioriHost` 実装（単一 sink・突合枠・メールボックス投函）。
/// 脳（`IShiori` 実装）が `Load` で受け取る sink を areka 側で実装する（task 4.1）。
mod shiori_host;

/// in-proc アクティベーション経路とリクエスト利用規律（単一 in-flight・遅延完了タイムアウト）。
/// in-proc の `IShiori`（脳）へ到達し `Load` で sink を渡す最小経路と、単一 in-flight・
/// `Unload` 保留取消・設定可能タイムアウトの利用規律を所有する（task 4.2）。
mod shiori_session;

/// 製品コード（非テスト）のリファレンス脳＋ファクトリ＋C 入口。`#[implement(IShiori)]`/
/// `#[implement(IShioriFactory)]` 実装＋純粋C コンストラクタ `shiori_factory` を所有する正解見本。
mod reference_brain;

/// 実走デモドライバ。`shiori_factory`→`ShioriSession` で activate→数往復 get→
/// `poll_completions`→raise/notify 観測→drop teardown を駆動し tracing で観測する。
mod shiori_demo;

/// 窓配置機構（areka-P0-window-placement）。ゴースト定義からキャラ窓・バルーン窓の
/// 初期配置を解決し窓 entity を組み立てる配置パイプライン。`open_startup_window`
/// シーム（task 6.2）が `prepare_ghost_windows`→`spawn_ghost_windows` を結線する。
mod placement;

/// emo2 統合結線（areka-P0-emo2-boot）。完成済み 5 トラックのエンジンを束ね、シェル
/// アニメーション側の表示指令を表示層の指令へ変換するアダプタ＋各エンジン結線＋観測を
/// 所有する（`target_map`／`adapter`／`talk_clock`／`assets`／`frame`＋`BootWiringError`・
/// `wire_emo2_boot`）。
mod emo2_boot;

/// UI→kanade のマウス入力配信配線（areka-P0-input-events）。キャラ窓のポインタイベントを
/// 捉え、当たり判定名を resolver で解決し、送出間引き（`throttle`）を通して kanade へ配信する
/// 薄い配線層。現状は `throttle`（送出間引きの純粋判定・task 2.4）のみ。ポインタハンドラ結線と
/// per-scope 状態保持（`MouseWiring`）は task 2.6／2.7 で増設される。
mod input_events;

/// 遅延応答と push 経路の end-to-end 結合テスト。
/// モック脳が `SHIORI_S_PENDING`＋token を返し、後で保持 host へ safe `complete`/`raise` を発火する
/// 一連の流れを `ShioriSession` 越しに 1 シナリオで通す（sink/session の単体テストと重複させない）。
#[cfg(test)]
mod shiori_e2e_tests;

/// ライフサイクルと単一 in-flight 規律の end-to-end 結合テスト。
/// 新 ABI の生成〜利用〜teardown（factory create→get→drop teardown・「未ロード状態」は存在しない）と、
/// `Deferred` 保留中の drop 取消→再 activate 後の正常動作を通しシナリオで実証する。
#[cfg(test)]
mod shiori_lifecycle_e2e_tests;

/// 製品 `ReferenceFactory`/`ReferenceBrain` × `ShioriSession` の end-to-end 結合テスト。
/// `shiori_factory` で取得した本物の製品 factory/brain を `ShioriSession` 越しに駆動し、
/// load_dir/shiori_name の貫通（D1）・即時→遅延+complete→raise→notify の数往復・単一 in-flight 拒否・
/// 決定的タイムアウト・stale complete 拒否・drop teardown を実時間 sleep に依存せず検証する。
#[cfg(test)]
mod shiori_reference_e2e_tests;

// ---------------------------------------------------------------------------
// Config Inputs (task 2.1)
// ---------------------------------------------------------------------------

/// 構成入力（解決済みルートパス）。
///
/// ゴースト／バルーンのルートパスを保持する。決定のみで実在は保証しない
/// （マウント・descript.txt 読取・`areka-parsers` 呼び出しは一切行わない・R6.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigInputs {
    ghost_root: std::path::PathBuf,
    balloon_root: std::path::PathBuf,
}

/// ゴーストルートの既定パス（`CARGO_MANIFEST_DIR` 相対・DD1）。
///
/// `crates/areka` 配下には現状ゴースト fixture が無いため（emo2 fixture は別クレート
/// `crates/pilot/...` にありクロスクレート `../` 参照は脆いので採らない）、ukadoc 標準の
/// ルート配置 `ghost/master` を **プレースホルダ subpath** として採用する。実在は検証せず、
/// 実マウント対象の確定は下流 ghost-setup の領分（本仕様スコープ外）。
fn default_ghost_root() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/ghost/master"))
}

/// バルーンルートの既定パス（`CARGO_MANIFEST_DIR` 相対・DD1）。
///
/// ゴースト既定と同じく `env!("CARGO_MANIFEST_DIR")` 相対のプレースホルダ subpath
/// `balloon/master` を採用する（実在検証なし・下流 ghost-setup が実体を確定）。
fn default_balloon_root() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/balloon/master"))
}

/// 起動引数（位置引数）と既定パスから構成入力を決定する。純粋・副作用なし。
///
/// - `args[0]` は実行ファイル名。`args[1]` = ghost root、`args[2]` = balloon root。
/// - 位置引数が与えられていれば採用し（R3.3）、欠落時は `CARGO_MANIFEST_DIR` 相対の
///   既定へフォールバックする（R3.4・DD1）。
/// - `args` を入力に取ることで `std::env::args()` を内部で呼ばず、実プロセス引数に触れずに
///   単体テスト可能な純粋関数に保つ。std（`std::path`・`env!`）のみに依存し、マウントも
///   descript.txt 読取も行わない（R6.1）。
fn resolve_config_inputs(args: &[String]) -> ConfigInputs {
    let ghost_root = args
        .get(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_ghost_root);
    let balloon_root = args
        .get(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_balloon_root);
    ConfigInputs {
        ghost_root,
        balloon_root,
    }
}

// ---------------------------------------------------------------------------
// Ghost Wiring (task 3.3)
// ---------------------------------------------------------------------------

/// 実行ファイル隣接の 32bit SHIORI helper 実行ファイルパスを解決する（純粋・DD 準拠）。
///
/// `std::env::current_exe()` の親ディレクトリへ `shiori-host32-helper.exe` を結合する。
/// `current_exe()` が失敗した場合（環境依存の稀な事象）は、この骨格の既存の寛容な
/// （panic しない）流儀に倣い `"."` を親ディレクトリ扱いにフォールバックする——`boot` 呼び出し
/// 自体はどのみち非致命として扱われるため、ここで panic/Err 伝播する必要はない。
fn default_helper_exe_path() -> std::path::PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.join("shiori-host32-helper.exe")
}

/// `ghost_root`／helper パスから `GhostBootOptions` を組み立てる純粋ヘルパ
/// （design.md「main の ghost boot／shutdown 結線」）。
///
/// - `shiori`: `ShioriWiring::Helper { helper_exe }`（実行ファイル隣接の 32bit helper・本番結線）。
/// - `default_encoding`: `DefaultEncoding::Ansi`（charset 未宣言時の SSP 既定・記憶
///   areka-descript-encoding-ishiori-utf8）。
/// - `sinks`: 可変長 sink 列（S-3）を `vec![LogSink, DiscardSink]` で埋める。broadcast（D4）で
///   全 cue は登録された全 sink へ配られるため、両スロットを `LogSink` にすると 1 cue が 2 回ログ
///   される（二重ログ）。記録 sink を **1 本（`LogSink`）だけ**にし、もう一方を破棄専用の
///   `DiscardSink` で埋めることで cue ごと 1 回ログへ正す（設計 D4 Topic 2）。
/// - `system_vars`: W1 暫定 provider（`default_system_vars`＝`%username` の既定スナップショット）。
/// - `ticker`: `TickerMode::Real` を既定 `TickerConfig`（`base_interval=50ms`／
///   `kanade_interval=1000ms`／実クロック `GetTickCount64`）で駆動する。
fn ghost_boot_options(
    ghost_root: std::path::PathBuf,
    helper_exe: std::path::PathBuf,
) -> areka_ghost::GhostBootOptions {
    areka_ghost::GhostBootOptions {
        ghost_root,
        default_encoding: areka_parsers::charset::DefaultEncoding::Ansi,
        shiori: areka_ghost::ShioriWiring::Helper { helper_exe },
        sinks: vec![
            Box::new(areka_ghost::sink::LogSink::new()),
            Box::new(areka_ghost::sink::DiscardSink::new()),
        ],
        system_vars: areka_ghost::default_system_vars(),
        ticker: areka_ghost::TickerMode::Real(Default::default()),
    }
}

/// `GhostBootError` を「起点不在（良性・`warn!` どまり）」と「それ以外（予期しない・`error!`）」
/// へ分類する純粋関数（design.md「main の ghost boot／shutdown 結線」・要件 8.2）。
///
/// `default_ghost_root()` はプレースホルダ subpath であり、この開発サンドボックスでは
/// 実在しないのが常態（＝`MountError::StartPointMissing` は想定内の事象）。読取不能
/// （`StartPointUnreadable`）・shell 不在（`ShellDirMissing`）・将来追加される
/// `#[non_exhaustive]` variant は、真に予期しない I/O 問題として区別する。
///
/// `pub(crate)`: `emo2_boot::wire_emo2_boot`（task 5.1）が boot 失敗（`GhostBootError`）を
/// 同一方針（起点不在＝良性 `warn!`・他＝`error!`・R7.4）で分類するため再利用する。
pub(crate) fn is_benign_boot_error(err: &areka_ghost::GhostBootError) -> bool {
    match err {
        areka_ghost::GhostBootError::Mount(
            areka_parsers::package::MountError::StartPointMissing { .. },
        ) => true,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Marker Components
// ---------------------------------------------------------------------------

/// 検証用ダミー窓を識別するマーカーコンポーネント（task 2.2・replace-me シーム）。
///
/// `open_startup_window` が開く最小の liveness プローブ窓に付与し、despawn（手動
/// ダブルクリック／task 2.3 の env ゲート自動 close）がダミー窓のみを狙えるようにする。
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct DummyWindowMarker;

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    human_panic::setup_panic!();

    // tracing-subscriber 初期化（RUST_LOG環境変数対応、デフォルト info）
    // 外部入力の扱い（A1-V）: RUST_LOG が未設定・非UTF-8・不正な構文の場合は
    // try_from_default_env() が Err を返し "info" へフォールバックする（panic 経路なし）。
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 構成入力（ゴースト／バルーンのルートパス）の解決とログ出力（R3.1/3.3/3.4・マウントしない）。
    let args: Vec<String> = std::env::args().collect();
    let cfg = resolve_config_inputs(&args);
    tracing::info!(
        ghost_root = %cfg.ghost_root.display(),
        balloon_root = %cfg.balloon_root.display(),
        "resolved config inputs"
    );

    // 存在検証は warn どまり・強制しない（R3 は「パスの決定とログ」でありマウント/存在保証ではない）。
    // 回復可能事象は `warn!` で記録して継続する（`areka-log-first-no-silent-failure`）。異常終了経路を作らない。
    for (label, root) in [
        ("ghost_root", &cfg.ghost_root),
        ("balloon_root", &cfg.balloon_root),
    ] {
        if !root.exists() {
            tracing::warn!(
                path = %root.display(),
                "{label} が存在しません（決定のみでマウントはしない・継続します）"
            );
        }
    }

    // 実行ファイル隣接の 32bit SHIORI helper パスを一度だけ解決する（実 sink 結線経路と
    // `LogSink` フォールバック boot 経路の双方が使うため main で保持する・DD-7）。
    let helper_exe = default_helper_exe_path();

    // UI ランタイム起動（COM/DPI 初期化・World 生成・shutdown hook 結線）（R2.4）。
    // DD-7/R7.1: 実 sink 結線（`wire_emo2_boot`）は UI 基盤の後に行うため、`WinApp::new()` を
    // すべての boot より前へ移動した（旧・task 3.3 の boot 先行順序を再編）。
    let app = WinApp::new()?;

    // リファレンス脳の実走デモ（要件 5.3/5.4）。環境変数 `AREKA_SHIORI_DEMO` が有効な
    // ときのみ main スレッドで同期駆動する（既定 OFF）。診断目的のため失敗しても通常
    // 起動を中断せず、`app.run()` の UI 立ち上げを阻害しないよう必ずその前に完走させる。
    if let Err(e) = shiori_demo::run_demo_if_enabled() {
        tracing::error!(error = %e, "[main] shiori reference demo failed");
    }

    // 起動窓シーム（window-placement 1.4）: ゴースト定義から本物のゴースト窓
    // （キャラ窓＋バルーン窓）を配置・生成する。準備失敗時（fixture 不在等）は
    // 検証用ダミー窓へフォールバックし、`main` 所有の `app.run()` ループに
    // heartbeat を与える骨格保証（boot→loop→exit）を維持する（DD14）。
    open_startup_window(&app, &cfg);

    // emo2 統合結線（task 5.2・design「エントリポイント / main.rs＋wire_emo2_boot」・DD-7）:
    // UI 基盤・起動窓の後で完成済み 5 トラック（seriko／sakura／emo-present／emo-text／actor）を
    // 束ねる実 sink 結線を試みる。`wired=true` なら実 sink boot が成立し、ghost／seriko ハンドルを
    // 終了処理へ運ぶ。`wired=false`（asset 組立失敗・boot 失敗等）は現行の `LogSink`×2 フォール
    // バック boot へ倒し、既存 smoke 前提・非致命 boot 意味論を温存する（R7.1/7.3・DD-7）。
    let outcome = emo2_boot::wire_emo2_boot(&app, &cfg.ghost_root, &cfg.balloon_root, &helper_exe);
    let (ghost_runtime, seriko_handle, loop_ticker) = if outcome.wired {
        tracing::info!("実 sink 結線で起動しました（emo2-boot wire 成立・SERIKO ループ ticker 稼働）");
        // マウス配信資源を World へ結線（task 3.1・design「main.rs＋wire_mouse_input」・
        // DD-IE-9）: kanade Sender クローンで MouseWiring（NonSend・Presenter）を挿入する。
        // 挿入は wire_emo2_boot 成功後＝Emo2Wiring 挿入済みゆえ presenter 経由の region 解決が
        // 成立する（Emo2Wiring 挿入と同位置・同型・self-gating）。窓へのハンドラ登録は task 3.2。
        if let Some(runtime) = outcome.ghost.as_ref() {
            let sender = runtime.kanade().clone();
            input_events::wire_mouse_input(app.world().borrow_mut().world_mut(), sender);
        }
        (outcome.ghost, outcome.seriko, outcome.loop_ticker)
    } else {
        // フォールバック（R7.3・DD-7）: 現行の `LogSink`×2 boot を UI 基盤・起動窓の後へ
        // relocate したもの。失敗は非致命——`default_ghost_root()` はこのサンドボックスでは
        // 常態的に不在（`MountError::StartPointMissing`）であり、`warn!` の上で `None` として
        // 骨格起動を継続する（要件 8.2）。それ以外の予期しない失敗（読取不能・shell 不在等）は
        // `error!`（`is_benign_boot_error` の分類は不変・R7.4）。
        let ghost_options = ghost_boot_options(cfg.ghost_root.clone(), helper_exe.clone());
        let ghost = match areka_ghost::boot(ghost_options) {
            Ok(runtime) => {
                tracing::info!("LogSink フォールバックで起動しました（emo2-boot wire 不成立）");
                Some(runtime)
            }
            Err(err) => {
                if is_benign_boot_error(&err) {
                    tracing::warn!(
                        error = %err,
                        "ghost 結線層の起動起点が見つかりません（決定のみで継続・骨格起動は阻害しません）"
                    );
                } else {
                    tracing::error!(
                        error = %err,
                        "ghost 結線層の起動に失敗しました（継続・骨格起動は阻害しません）"
                    );
                }
                None
            }
        };
        // フォールバック経路に seriko アクター・loop ticker はない（実 sink 結線が成立していない）。
        (ghost, None, None)
    };

    // `main` 所有のブロッキングメッセージループ（R2.4/R4.1）。ダミー窓／ゴースト窓が
    // 閉じられると `WindowRegistry` が空へ遷移し `run()` が `Ok` を返して正常終了する（DD7 改定）。
    app.run()?;

    // 終了順序（task 9.5・design「結線・資産・実機経路（main.rs）」）:
    //   ① loop ticker Close → ② ghost.shutdown → ③ seriko join。
    //
    // ① loop ticker Close（本ブロック）: SERIKO ループ ticker の worker スレッドは closure 内へ
    // `SerikoSink` クローン（tick_sink）を握る。これを先に停止させないと seriko inbox が ticker 経由で
    // 生き続け、③ の join が「全 Sender drop」を永遠に待って hang する。停止端 Sender へ
    // `TickerMsg::Close` を送ると worker は `recv_timeout` から `Ok(Close)` で return し、その closure＝
    // tick_sink が drop される（inbox 切断の片翼が外れる。残る片翼＝ghost 側 SerikoSink は ② が外す）。
    // main は ticker の JoinHandle を持たない（`wire_emo2_boot` が保持せず drop 済み）ため直接 join でき
    // ないが、③ の seriko join が全 Sender drop まで block するため実質 ticker worker の終端を待つ形に
    // なり hang しない（Close 未達で worker が既に終端していても drop で disconnected 経路へ倒れる）。
    // 失敗（既に終端済み）は shutdown 期待事象ゆえ `debug!`（silent failure 禁止・非致命・R7.5）。
    if let Some(ticker) = loop_ticker {
        match ticker.send(areka_ghost::ticker::TickerMsg::Close) {
            Ok(()) => tracing::info!(
                "seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止）"
            ),
            Err(_) => tracing::debug!(
                "seriko: loop ticker は既に終端済み（Close 送信先なし・shutdown 期待事象）"
            ),
        }
        // 送信の成否に依らず停止端 Sender をここで drop し、確実に制御チャンネルを disconnected にする。
        drop(ticker);
    }

    // ② 終了握手（task 5.2・design「終了握手（R6）」・DD-10）: `run()` 復帰後、boot 済み
    // （`Some`）のときのみ `shutdown` を呼ぶ。DD-10 により終了理由は `System` から
    // `CloseReason::User` へ改定（全窓 close funnel はユーザ操作起点）。OnClose 応答の再生
    // 完了待ちは kanade の `ForceQuit` 終了系列内で処理される（本仕様は `shutdown` を呼ぶだけ・
    // 不改変・R6.2）。失敗は `error!` の上で main 自身の `Result` へ伝播する（genuine な失敗を
    // 黙って exit 0 にしない・R6.3）。
    if let Some(runtime) = ghost_runtime {
        if let Err(err) = runtime.shutdown(areka_kanade::CloseReason::User) {
            tracing::error!(error = %err, "ghost 結線層の終了統括に失敗しました");
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ));
        }
    }

    // ③ seriko アクターの join（design「終了握手（R6）」・R6.3）。seriko inbox への送信端は 2 本ある:
    // (a) ghost 側の `SerikoSink`（surface_sink・②の `shutdown` が drop）と (b) loop ticker closure の
    // `tick_sink`（①の Close→worker return で drop）。①②で両端が drop されて inbox が切断され、seriko
    // worker は自然終了する。main は自前の `SerikoSink` クローンを保持しない（sink は `wire_emo2_boot`
    // が boot／ticker へ move 済み）ため、この `join` は両端 drop 完了（＝ticker worker 終端）まで block
    // したうえで速やかに戻る（①で ticker を先に Close したことが hang 回避の要）。join 失敗（worker
    // panic）は握り潰さず `error!`＋`Err` 伝播する（genuine な失敗を隠さない）。
    if let Some(seriko) = seriko_handle {
        if let Err(err) = seriko.join() {
            tracing::error!(error = %err, "seriko アクターの join に失敗しました");
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Startup Window (replace-me seam・task 2.2)
// ---------------------------------------------------------------------------

/// 検証用ダミー窓の Entity を構築する（headless・task 2.2）。
///
/// UI ランタイム起動と正常終了の観測目的に限る最小の窓を spawn する。窓が存在するために
/// 必要な最小コンポーネント（`Window`・`WindowStyle`・可視/クリック可能な最小 `BoxStyle`
/// サイズ・ダブルクリック despawn の observer）だけを与え、`DummyWindowMarker` で識別する。
///
/// **配置・座標・DPI を一切主張しない**: `WindowPos` の位置（`position`）を設定せず、座標
/// ロジックも持たない（既定位置で開く）。placement は window-placement の領分であり、ダミー窓は
/// そこへ踏み込まない（2026-07-05 placement リジェクト再発防止・R2.5）。`create_shell_window`
/// と同じく bare `World` だけで構築でき、headless 単体テスト可能。
fn spawn_dummy_window(world: &mut World) -> Entity {
    let dummy = world
        .spawn((
            bevy_ecs::name::Name::new("Startup-Dummy-Window"),
            DummyWindowMarker,
            Window {
                title: "areka startup (dummy)".to_string(),
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_TOOLWINDOW,
            },
            // 可視・クリック可能にする最小サイズのみ（位置・座標は主張しない）。
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(200.0)),
                    height: Some(Dimension::Px(150.0)),
                }),
                ..Default::default()
            },
            // ダブルクリックで自身を閉じられるようにする。
            OnPointerPressed(on_dummy_pressed),
        ))
        .id();

    // 可視・ヒット可能な診断用サーフェス（子 Rectangle）を 1 枚付ける。
    //
    // areka の窓は WUC/DComp GPU 合成（`WS_EX_NOREDIRECTIONBITMAP`）で描画されるため、描画内容が
    // 無い窓は合成結果が完全透明になり画面に見えず、手動ダブルクリック（task 2.2 の受け入れ）の
    // 標的にできない。不透明な単色矩形を窓いっぱい（`flex_grow`）に貼ることで、窓が実際に描画され
    // 観測・操作可能になる。これはマスコット／ゴースト絵ではなく liveness プローブが必要とする
    // 診断サーフェスに過ぎず、「ゴースト内容なし」の設計意図（DD5）に反しない。
    //
    // 既定 `HitTest`（Bounds ＝ ヒット可能）のため、この矩形上のダブルクリックはヒットし、窓 entity
    // の `OnPointerPressed(on_dummy_pressed)` へ **bubble** して close を発火する（proven mock pattern：
    // `create_shell_window`／`clickthrough_two_rects` と同型）。矩形は窓内レイアウト（intra-window）
    // であって画面配置（screen-placement）ではない＝`WindowPos` 位置も座標／DPI ロジックも増やさない
    // （2026-07-05 placement リジェクト再発防止・R2.5）。
    world.spawn((
        bevy_ecs::name::Name::new("Startup-Dummy-Surface"),
        Rectangle::new(),
        // 不透明な中間グレー（診断用に明確に視認できる liveness サーフェス）。
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        }),
        BoxStyle {
            flex_grow: Some(1.0),
            ..Default::default()
        },
        ChildOf(dummy),
    ));

    dummy
}

/// OnPointerPressed ハンドラ: ダブルクリック（左）でダミー窓を despawn する（task 2.2）。
///
/// `Phase::Bubble` で `DoubleClick::Left` を検出し、`DummyWindowMarker` を持つ全 entity を
/// despawn して true を返す。それ以外は false。`Phase::Tunnel` は無視する。
/// despawn → `on_window_handle_remove`（wintf
/// `crates/wintf/src/ecs/window/window_handle.rs`）→ `PostMessageW(WM_CLOSE)` → `DestroyWindow`
/// → `WindowRegistry` 空遷移 → `run()` 復帰、という wintf の作法に委ねる（自前 wndproc も
/// 手書き `PostMessageW(WM_CLOSE)` も書かない）。
fn on_dummy_pressed(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(state) => {
            if state.double_click == DoubleClick::Left {
                tracing::info!("ダミー窓ダブルクリック検出 — ダミー窓を閉じます");

                // ダミー窓 entity を収集して despawn（on_window_handle_remove → WM_CLOSE →
                // DestroyWindow → WindowRegistry 空遷移 → run() 復帰）。
                let dummies: Vec<Entity> = world
                    .query_filtered::<Entity, With<DummyWindowMarker>>()
                    .iter(world)
                    .collect();
                for e in dummies {
                    world.despawn(e);
                }

                return true;
            }
            false
        }
    }
}

/// 起動窓シーム（task 6.2・要件 1.4・design「main.rs seam」）: `prepare_ghost_windows`
/// 成功時は本物のゴースト窓（キャラ窓＋バルーン窓）を生成し、準備失敗時は検証用
/// ダミー窓へフォールバックする（旧 replace-me シームの差し替え本体）。
///
/// - 成功時: `spawn_ghost_windows` を既存 ECS コマンド経路（`EcsWorld::spawn` の async
///   タスク → `CommandSender` → Input スケジュールで World 適用＝ダミー窓と同経路）で
///   実行し、`register_ghost_windows_click_through` を `FrameFinalize` schedule へ結線する
///   （emo-present donor と同じ結線位置・task 5.2）。
/// - 失敗時（fixture 不在等）: `MountError::StartPointMissing` 系は `warn!`・他は `error!`
///   の上で `spawn_dummy_window` へフォールバックする（DD14・骨格の boot→loop→exit と
///   smoke 完走を維持。`spawn_dummy_window`／`DummyWindowMarker` は退役せず残置）。
/// - **暫定の終了手段**（design「main.rs seam」note）: emo2-boot 装着前の本物ゴースト窓は
///   描画内容なし＝WUC/DComp GPU 合成で不可視・ヒットなしのため、対話的 close 不能が
///   正しい状態。終了は smoke ゲート（`AREKA_APP_SMOKE_EXIT_MS`）または Ctrl+C。
///
/// 準備（`prepare_ghost_windows`）は同期実行し、I/O はここで完結・Send な値のみを ECS
/// コマンドへ運ぶ。呼び出しスレッドは `WinApp::new()` 済みの MTA UI スレッド＝COM
/// 初期化済み（measure の WIC 前提を満たす）。署名は `(&WinApp, &ConfigInputs)`
/// （design の Revalidation Trigger として本タスクで変更）。
fn open_startup_window(app: &WinApp, cfg: &ConfigInputs) {
    match placement::prepare_ghost_windows(&cfg.ghost_root, &cfg.balloon_root) {
        Ok(prepared) => {
            // MonitorSnapshot（task 8.1・DD15 基盤）: 起動時の実モニタ work area 集合を
            // 忠実転写した Resource（物理 px・Send な純粋データ）。bottom 吸着ドラッグ
            // （4.7・task 8.2）が消費する。セッション内固定＝M1 受容
            // （WM_DISPLAYCHANGE 追随は後続・DD15）。
            let snapshot = placement::follow::MonitorSnapshot::from_monitors(
                &wintf::ecs::window::monitor::enumerate_monitors(),
            );

            // clickthrough 登録 system を FrameFinalize へ結線（task 5.2 の donor slot・
            // emo-present と同位置）。`Added<WindowHandle>` 駆動のため窓 spawn より先に
            // 結線しても取りこぼさない（registry NonSend は WinApp::run が挿入・5.2 learnings）。
            app.world().borrow_mut().add_systems(
                FrameFinalize,
                placement::spawn::register_ghost_windows_click_through,
            );

            // `EcsWorld::spawn` の async タスク → CommandSender → Input スケジュールで
            // World 適用という既存 ECS コマンド経路（ダミー窓と同型）で本物窓を組み立てる。
            app.world().borrow().spawn(|tx: CommandSender| async move {
                let _ = tx.send(Box::new(move |world: &mut World| {
                    world.insert_resource(snapshot);
                    let windows = placement::spawn::spawn_ghost_windows(
                        world,
                        &prepared.placements,
                        &prepared.titles,
                    );
                    // マウス入力ハンドラ装着（areka-P0-input-events・依存方向 input_events→
                    // placement）: placement は `crate::` パスを持てない（example の `#[path]`
                    // include で成立させるため）ゆえ、キャラ窓へのポインタハンドラ結線は
                    // input_events 側が担う。spawn 直後の同一 World-mutation クロージャ内で
                    // 同期実行するため、キャラ窓は既に存在し async race はない。
                    input_events::attach_char_pointer_handlers(world);
                    let scopes: Vec<usize> = windows.scopes().collect();
                    tracing::info!(
                        ?scopes,
                        "本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓）"
                    );
                }));
            });
        }
        Err(err) => {
            // フォールバック分類（DD14・log-first）: 起点不在（fixture 不在等の想定内）は
            // warn!・それ以外（読取不能・採寸失敗・モニタ 0 台等）は error!。どちらも
            // ダミー窓へフォールバックし骨格の boot→loop→exit を維持する。
            if is_benign_placement_error(&err) {
                tracing::warn!(
                    error = %err,
                    "窓配置の準備起点が見つかりません（fixture 不在等の想定内事象）——検証用ダミー窓へフォールバックします"
                );
            } else {
                tracing::error!(
                    error = %err,
                    "窓配置の準備に失敗しました——検証用ダミー窓へフォールバックします"
                );
            }
            app.world().borrow().spawn(|tx: CommandSender| async move {
                let _ = tx.send(Box::new(|world: &mut World| {
                    spawn_dummy_window(world);
                    tracing::info!("検証用ダミー窓を開きました（placement フォールバック）");
                }));
            });
        }
    }

    // env ゲート付き自動 close 機構（CI smoke・task 2.3・R4.1）。
    // `AREKA_APP_SMOKE_EXIT_MS` が有効なミリ秒値のときだけ、VSync relay と同じ
    // `wintf::executor::spawn_local`＋world `Weak` 作法で **一発の** async タスクを投入する
    // （ECS システムではない）。env 未設定・不正なら発火せず、ダミー窓は利用者の
    // ダブルクリック despawn（task 2.2 経路）を待ち続ける。
    if let Some(ms) = smoke_exit_ms() {
        // WinApp が strong 所有者を保持するため、この Weak は shutdown まで upgrade 可能。
        let world_weak = std::rc::Rc::downgrade(&app.world());
        tracing::info!(
            env = SMOKE_EXIT_ENV,
            delay_ms = ms,
            "smoke 自動 close ゲート有効 — 起動窓（ダミー窓／ゴースト窓）を指定 ms 後に despawn します"
        );
        wintf::executor::spawn_local(async move {
            // 指定 ms を async スリープ（async-io は既存依存・tokio 不要）。
            async_io::Timer::after(std::time::Duration::from_millis(ms)).await;
            // shutdown 済みなら strong 所有者は消えており upgrade は None ＝ no-op。
            let Some(world) = world_weak.upgrade() else {
                tracing::debug!("smoke 自動 close: world 既に drop 済み（shutdown）— no-op");
                return;
            };
            // await を跨いで borrow を保持しない TIGHT スコープで despawn する。
            {
                let mut ecs = world.borrow_mut();
                let w = ecs.world_mut();
                let count = despawn_smoke_targets(w);
                tracing::info!(
                    count,
                    "smoke 自動 close: 起動窓（ダミー窓／ゴースト窓）を despawn しました"
                );
            }
        });
    }
}

/// smoke 自動 close の despawn 標的を despawn する（task 6.2 で
/// `Or<(With<DummyWindowMarker>, With<GhostWindowMarker>)>` へ拡張・design「main.rs seam」）。
///
/// ダミー窓（フォールバック経路）と本物のゴースト窓（placement 経路）のどちらの構成でも
/// CI smoke（`AREKA_APP_SMOKE_EXIT_MS`）が完走できるよう、両 marker を単一 query で狙う。
/// despawn 件数を返す（標的なしは 0・no-op 安全）。bare `World` だけで動き headless
/// 単体テスト可能（`seam_tests`）。
fn despawn_smoke_targets(world: &mut World) -> usize {
    let targets: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<DummyWindowMarker>, With<placement::spawn::GhostWindowMarker>)>>()
        .iter(world)
        .collect();
    let count = targets.len();
    for e in targets {
        world.despawn(e);
    }
    count
}

/// `PlacementError` を「起点不在（良性・`warn!` どまり）」と「それ以外（予期しない・`error!`）」
/// へ分類する純粋関数（task 6.2・design「main.rs seam」・DD14）。
///
/// `default_ghost_root()` はプレースホルダ subpath であり、この開発サンドボックスでは
/// 実在しないのが常態（＝`MountError::StartPointMissing` は想定内の事象）。それ以外
/// （読取不能・shell 不在・descript I/O・採寸失敗・モニタ 0 台・将来追加の
/// `#[non_exhaustive]` variant）は真に予期しない失敗として区別する
/// （`is_benign_boot_error` と同じ分類方針）。
fn is_benign_placement_error(err: &placement::PlacementError) -> bool {
    matches!(
        err,
        placement::PlacementError::Mount(
            areka_parsers::package::MountError::StartPointMissing { .. }
        )
    )
}

/// 自動 close ゲートを有効化する環境変数名（`AREKA_` 冠規約・記憶 areka-runtime-env-naming）。
const SMOKE_EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";

/// 与えられた値から自動 close の遅延ミリ秒を解釈する純粋ヘルパ（env アクセスなし・単体テスト可能）。
///
/// - `None`／空／空白のみ／非数値／負値／`u64` 溢れ → `None`（ゲート OFF＝タスク不投入）。
/// - 周辺空白をトリムした非負整数 → `Some(ms)`（`"0"` は 0ms＝即時発火として受理）。
///
/// `u64::from_str` は負号・小数点・非数字を弾き、範囲外を `Err` にするため、負値・溢れは
/// 自然に `None` へ落ちる（不正入力はゲート OFF に倒す）。
fn smoke_exit_ms_from(value: Option<&str>) -> Option<u64> {
    let trimmed = value.map(str::trim)?;
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// 環境変数 [`SMOKE_EXIT_ENV`] から自動 close の遅延ミリ秒を読む。
fn smoke_exit_ms() -> Option<u64> {
    smoke_exit_ms_from(std::env::var(SMOKE_EXIT_ENV).ok().as_deref())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// `open_startup_window` シーム（task 2.2）の headless 単体テスト。
///
/// ランタイム結線（`open_startup_window`）は生きた `WinApp` を要し headless では駆動できないため、
/// TDD は headless で駆動可能な 2 部品（ダミー窓ビルダ `spawn_dummy_window` と despawn ハンドラ
/// `on_dummy_pressed`）で回す。自動 close ゲート `smoke_exit_ms_from` は純粋・env 非依存で検証する。
#[cfg(test)]
mod startup_window_tests {
    use super::*;
    use wintf::ecs::WindowPos;

    /// ダミー窓の press イベントを作る。
    fn pressed_event(double_click: DoubleClick) -> PointerState {
        PointerState {
            double_click,
            ..Default::default()
        }
    }

    /// ビルダは最小の窓 entity を生成する: マーカー・Window・WindowStyle・BoxStyle サイズ・
    /// OnPointerPressed を持ち、**WindowPos の位置主張を一切持たない**（配置・座標・DPI 非主張）。
    #[test]
    fn dummy_window_has_minimal_components_and_no_position_claim() {
        let mut world = World::new();
        let dummy = spawn_dummy_window(&mut world);

        // マーカー: despawn/auto-close がダミーのみを狙える。
        assert!(world.get::<DummyWindowMarker>(dummy).is_some());

        // 窓が存在するのに必要な最小コンポーネント。
        assert!(world.get::<Window>(dummy).is_some());
        assert!(world.get::<WindowStyle>(dummy).is_some());

        // 可視・クリック可能な最小サイズ（BoxStyle）を持つ。
        let box_style = world.get::<BoxStyle>(dummy).expect("BoxStyle");
        assert!(box_style.size.is_some(), "ダミー窓は最小サイズを持つべき");

        // ダブルクリック despawn 用の observer を持つ。
        assert!(world.get::<OnPointerPressed>(dummy).is_some());

        // 配置・座標・DPI を一切主張しない: ビルダは WindowPos を明示挿入せず、
        // 具体座標を主張しない。wintf の `on_window_add` フックが CreateWindow 前提として
        // `WindowPos::default()`（位置＝CW_USEDEFAULT ＝「Windows 既定に委ねる」＝座標非主張）を
        // 自動挿入するため、存在する WindowPos は必ず既定値と一致するはずである
        // （＝ビルダ由来の座標ロジックがない証明）。placement は window-placement の領分であり、
        // ダミー窓はそこへ踏み込まない（2026-07-05 placement リジェクト再発防止・R2.5）。
        if let Some(wp) = world.get::<WindowPos>(dummy) {
            assert_eq!(
                *wp,
                WindowPos::default(),
                "ダミー窓は具体座標を主張してはならない（既定 CW_USEDEFAULT のみ許容・liveness プローブに限る）"
            );
        }
        // 念のため: もし位置を持つなら CW_USEDEFAULT（座標非主張の番兵）であること。
        if let Some(pos) = world.get::<WindowPos>(dummy).and_then(|wp| wp.position) {
            assert_eq!(
                (pos.x, pos.y),
                (CW_USEDEFAULT, CW_USEDEFAULT),
                "ダミー窓の位置は CW_USEDEFAULT（既定placement・座標非主張）に限る"
            );
        }
    }

    /// ビルダはダミー窓に **可視・ヒット可能な子 Rectangle** を 1 枚付ける。
    ///
    /// areka の窓は WUC/DComp GPU 合成（`WS_EX_NOREDIRECTIONBITMAP`）で描画されるため、
    /// 描画内容が無い窓は完全透明で画面に見えず、手動ダブルクリックの標的にできない。
    /// 不透明な診断用 Rectangle（`Brushes` 付き）を子（`ChildOf` = ダミー窓）として持つことで
    /// 窓が実際にレンダリングされる（描画内容を持つ）ことを証明する。既定 `HitTest`（Bounds）で
    /// 子はヒット可能＝子上のダブルクリックが窓の `OnPointerPressed` へ bubble する（proven mock pattern）。
    #[test]
    fn dummy_window_has_visible_rectangle_child() {
        let mut world = World::new();
        let dummy = spawn_dummy_window(&mut world);

        // Rectangle + Brushes を持つ子 entity が 1 枚あり、その ChildOf 親がダミー窓であること。
        let mut query = world.query::<(&Rectangle, &Brushes, &ChildOf)>();
        let children: Vec<_> = query.iter(&world).collect();
        assert_eq!(children.len(), 1, "ダミー窓は可視の子 Rectangle を 1 枚持つべき");

        let (_rect, _brushes, child_of) = children[0];
        assert_eq!(
            child_of.parent(),
            dummy,
            "可視 Rectangle の親はダミー窓であるべき（描画内容 = liveness surface）"
        );
    }

    /// ダブルクリック（左）でマーカー付きダミー窓を despawn し true を返す。
    /// マーカーを持たない entity は残す。
    #[test]
    fn double_click_left_despawns_all_dummy_windows() {
        let mut world = World::new();
        let dummy = world.spawn(DummyWindowMarker).id();
        let dummy2 = world.spawn(DummyWindowMarker).id();
        let other = world.spawn_empty().id();

        let ev = Phase::Bubble(pressed_event(DoubleClick::Left));
        let handled = on_dummy_pressed(&mut world, dummy, dummy, &ev);

        assert!(handled);
        assert!(world.get_entity(dummy).is_err());
        assert!(world.get_entity(dummy2).is_err());
        assert!(world.get_entity(other).is_ok());
    }

    /// 左以外のダブルクリックでは despawn しない（false）。
    #[test]
    fn non_left_double_click_does_not_despawn_dummy() {
        let mut world = World::new();
        let dummy = world.spawn(DummyWindowMarker).id();

        for dc in [DoubleClick::None, DoubleClick::Right, DoubleClick::Middle] {
            let ev = Phase::Bubble(pressed_event(dc));
            assert!(!on_dummy_pressed(&mut world, dummy, dummy, &ev));
        }
        assert!(world.get_entity(dummy).is_ok());
    }

    /// Tunnel フェーズのダブルクリックは無視する（false・despawn しない）。
    #[test]
    fn tunnel_phase_double_click_is_ignored_for_dummy() {
        let mut world = World::new();
        let dummy = world.spawn(DummyWindowMarker).id();

        let ev = Phase::Tunnel(pressed_event(DoubleClick::Left));
        assert!(!on_dummy_pressed(&mut world, dummy, dummy, &ev));
        assert!(world.get_entity(dummy).is_ok());
    }

    // -- 自動 close ゲート `smoke_exit_ms_from`（純粋・env 非依存・task 2.3・R4.1） --

    /// env 未設定（`None`）ではゲート発火なし（`None`）＝タスク不投入。
    #[test]
    fn smoke_exit_ms_unset_yields_none() {
        assert_eq!(smoke_exit_ms_from(None), None);
    }

    /// 空文字・空白のみは発火なし（`None`）。
    #[test]
    fn smoke_exit_ms_empty_or_whitespace_yields_none() {
        assert_eq!(smoke_exit_ms_from(Some("")), None);
        assert_eq!(smoke_exit_ms_from(Some("   ")), None);
        assert_eq!(smoke_exit_ms_from(Some("\t")), None);
    }

    /// 非数値は発火なし（`None`）。
    #[test]
    fn smoke_exit_ms_non_numeric_yields_none() {
        assert_eq!(smoke_exit_ms_from(Some("abc")), None);
        assert_eq!(smoke_exit_ms_from(Some("12ms")), None);
        assert_eq!(smoke_exit_ms_from(Some("1.5")), None);
    }

    /// `"0"` は即時発火（0ms）として `Some(0)`。周辺空白はトリムして受理する。
    #[test]
    fn smoke_exit_ms_zero_yields_some_zero() {
        assert_eq!(smoke_exit_ms_from(Some("0")), Some(0));
        assert_eq!(smoke_exit_ms_from(Some("  0  ")), Some(0));
    }

    /// 正の整数はその値をミリ秒として受理する。
    #[test]
    fn smoke_exit_ms_positive_yields_some() {
        assert_eq!(smoke_exit_ms_from(Some("500")), Some(500));
        assert_eq!(smoke_exit_ms_from(Some(" 1500 ")), Some(1500));
    }

    /// 負値・`u64` 溢れは発火なし（`None`）＝不正入力はゲート OFF。
    #[test]
    fn smoke_exit_ms_negative_or_overflow_yields_none() {
        assert_eq!(smoke_exit_ms_from(Some("-1")), None);
        // u64::MAX + 1（20 桁）は溢れて None。
        assert_eq!(smoke_exit_ms_from(Some("18446744073709551616")), None);
    }
}

/// main.rs シーム（task 6.2）の headless 単体テスト。
///
/// シーム結線そのもの（`open_startup_window`）は生きた `WinApp` を要するため、
/// TDD は headless で駆動可能な決定論部品——フォールバック分類
/// `is_benign_placement_error` と smoke 自動 close の despawn 標的
/// `despawn_smoke_targets`——で回す。結線の実証は実プロセス smoke
/// （`tests/smoke_boot_loop_exit.rs`・両方向）が担う。
#[cfg(test)]
mod seam_tests {
    use super::*;
    use crate::placement::PlacementError;
    use crate::placement::spawn::GhostWindowMarker;
    use areka_parsers::package::MountError;
    use std::path::PathBuf;

    /// `PlacementError::Mount(StartPointMissing)`（fixture 不在という想定内の事象）は
    /// 良性（`warn!` どまり）と分類される（design「main.rs seam」・DD14）。
    #[test]
    fn placement_start_point_missing_is_benign() {
        let err = PlacementError::Mount(MountError::StartPointMissing {
            expected: PathBuf::from("ghost/master/descript.txt"),
        });
        assert!(is_benign_placement_error(&err));
    }

    /// それ以外の `PlacementError`（読取不能・shell 不在・descript I/O・採寸・モニタ 0 台）は
    /// 真に予期しない失敗として良性ではない（`error!`）と分類される。
    #[test]
    fn placement_other_errors_are_not_benign() {
        let unreadable = PlacementError::Mount(MountError::StartPointUnreadable {
            path: PathBuf::from("ghost/master/descript.txt"),
            kind: std::io::ErrorKind::PermissionDenied,
        });
        assert!(!is_benign_placement_error(&unreadable));

        let shell_missing = PlacementError::Mount(MountError::ShellDirMissing {
            expected: PathBuf::from("ghost/master/shell/master"),
        });
        assert!(!is_benign_placement_error(&shell_missing));

        let descript = PlacementError::DescriptRead {
            path: PathBuf::from("shell/master/descript.txt"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "boom"),
        };
        assert!(!is_benign_placement_error(&descript));

        let measure = PlacementError::Measure {
            scope: 0,
            reason: "合成失敗".to_string(),
        };
        assert!(!is_benign_placement_error(&measure));

        let monitor = PlacementError::Monitor {
            reason: "0 台".to_string(),
        };
        assert!(!is_benign_placement_error(&monitor));
    }

    /// smoke 自動 close の despawn 標的は `Or<(With<DummyWindowMarker>,
    /// With<GhostWindowMarker>)>`（task 6.2 拡張）: ダミー窓・ゴースト窓の両方を
    /// despawn し、無関係 entity は残す。
    #[test]
    fn despawn_smoke_targets_hits_dummy_and_ghost_only() {
        let mut world = World::new();
        let dummy = world.spawn(DummyWindowMarker).id();
        let ghost = world.spawn(GhostWindowMarker).id();
        let other = world.spawn_empty().id();

        let count = despawn_smoke_targets(&mut world);

        assert_eq!(count, 2, "ダミー窓＋ゴースト窓の 2 entity を despawn すべき");
        assert!(world.get_entity(dummy).is_err());
        assert!(world.get_entity(ghost).is_err());
        assert!(world.get_entity(other).is_ok());
    }

    /// 標的なしの World では 0 を返し何も壊さない（冪等・no-op 安全）。
    #[test]
    fn despawn_smoke_targets_empty_world_is_noop() {
        let mut world = World::new();
        let other = world.spawn_empty().id();
        assert_eq!(despawn_smoke_targets(&mut world), 0);
        assert!(world.get_entity(other).is_ok());
    }
}

#[cfg(test)]
mod config_input_tests {
    use super::{ConfigInputs, default_balloon_root, default_ghost_root, resolve_config_inputs};
    use std::path::PathBuf;

    /// argv[1]/argv[2] が両方あるとき、両ルートを引数値でそのまま採用する（R3.3）。
    #[test]
    fn both_args_present_adopts_both() {
        let args = vec![
            "areka.exe".to_string(),
            "C:/custom/ghost".to_string(),
            "C:/custom/balloon".to_string(),
        ];
        let cfg = resolve_config_inputs(&args);
        assert_eq!(cfg.ghost_root, PathBuf::from("C:/custom/ghost"));
        assert_eq!(cfg.balloon_root, PathBuf::from("C:/custom/balloon"));
    }

    /// 引数なし（argv[0] のみ）のとき、両ルートとも既定へフォールバックする（R3.4）。
    #[test]
    fn no_args_uses_both_defaults() {
        let args = vec!["areka.exe".to_string()];
        let cfg = resolve_config_inputs(&args);
        assert_eq!(cfg.ghost_root, default_ghost_root());
        assert_eq!(cfg.balloon_root, default_balloon_root());
    }

    /// ghost のみ引数ありのとき、ghost は採用・balloon は既定にフォールバックする（R3.3/3.4）。
    #[test]
    fn ghost_only_arg_adopts_ghost_defaults_balloon() {
        let args = vec!["areka.exe".to_string(), "C:/custom/ghost".to_string()];
        let cfg = resolve_config_inputs(&args);
        assert_eq!(cfg.ghost_root, PathBuf::from("C:/custom/ghost"));
        assert_eq!(cfg.balloon_root, default_balloon_root());
    }

    /// 既定パスが `CARGO_MANIFEST_DIR` 相対で決定的に生成される（R3.4・DD1）。
    #[test]
    fn defaults_are_cargo_manifest_dir_relative_and_deterministic() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // 既定は CARGO_MANIFEST_DIR 配下にある（相対アンカー）。
        assert!(
            default_ghost_root().starts_with(&manifest),
            "ghost default must be under CARGO_MANIFEST_DIR: {:?}",
            default_ghost_root()
        );
        assert!(
            default_balloon_root().starts_with(&manifest),
            "balloon default must be under CARGO_MANIFEST_DIR: {:?}",
            default_balloon_root()
        );
        // 決定的: 呼び出しごとに同一値を返す。
        assert_eq!(default_ghost_root(), default_ghost_root());
        assert_eq!(default_balloon_root(), default_balloon_root());
    }

    /// `ConfigInputs` は解決済みルートパスを保持する（型の存在確認）。
    #[test]
    fn config_inputs_holds_resolved_roots() {
        let cfg = ConfigInputs {
            ghost_root: PathBuf::from("g"),
            balloon_root: PathBuf::from("b"),
        };
        assert_eq!(cfg.ghost_root, PathBuf::from("g"));
        assert_eq!(cfg.balloon_root, PathBuf::from("b"));
    }
}

/// ghost 結線ヘルパ（task 3.3）の headless 単体テスト。
///
/// `areka_ghost::boot`／`GhostRuntime::shutdown` 自体は実 I/O・実スレッドを伴うため、
/// ここでは純粋な組み立て・分類ロジック（`default_helper_exe_path`／`ghost_boot_options`／
/// `is_benign_boot_error`）だけを headless に検証する。実際の boot→shutdown 一巡は
/// 既存の実プロセス smoke テスト（`tests/smoke_boot_loop_exit.rs`）が証明する。
#[cfg(test)]
mod ghost_wiring_tests {
    use super::*;
    use areka_ghost::{GhostBootError, ShioriWiring, TickerMode};
    use areka_parsers::charset::DefaultEncoding;
    use areka_parsers::package::MountError;
    use std::path::PathBuf;

    /// `default_helper_exe_path` はファイル名 `shiori-host32-helper.exe` で終わるパスを返す
    /// （実際の親ディレクトリは実行環境依存のため、構造のみを確認する）。
    #[test]
    fn default_helper_exe_path_ends_with_expected_filename() {
        let path = default_helper_exe_path();
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("shiori-host32-helper.exe"),
            "helper exe path should end with the expected filename: {path:?}"
        );
    }

    /// `ghost_boot_options` は渡された `ghost_root`／`helper_exe` をそのまま
    /// `GhostBootOptions` へ写し、`ShioriWiring::Helper`・`DefaultEncoding::Ansi`・
    /// `TickerMode::Real` を選ぶ（design.md「main の ghost boot／shutdown 結線」）。
    #[test]
    fn ghost_boot_options_wires_expected_fields() {
        let ghost_root = PathBuf::from("C:/custom/ghost");
        let helper_exe = PathBuf::from("C:/custom/exe-dir/shiori-host32-helper.exe");

        let options = ghost_boot_options(ghost_root.clone(), helper_exe.clone());

        assert_eq!(options.ghost_root, ghost_root);
        assert_eq!(options.default_encoding, DefaultEncoding::Ansi);
        match options.shiori {
            ShioriWiring::Helper {
                helper_exe: actual, ..
            } => assert_eq!(actual, helper_exe),
            ShioriWiring::Custom(_) => panic!("expected ShioriWiring::Helper, got Custom"),
            // `InProc`（areka-P0-shiori4-test-ghost の第 3 結線）は本番 main では選ばれない
            // （要件 7.2: 本番結線は emo2＝Helper 経路のまま）。網羅性のためのみ列挙する。
            ShioriWiring::InProc => panic!("expected ShioriWiring::Helper, got InProc"),
        }
        match options.ticker {
            TickerMode::Real(cfg) => {
                assert_eq!(cfg.base_interval, std::time::Duration::from_millis(50));
                assert_eq!(cfg.kanade_interval, std::time::Duration::from_millis(1000));
            }
            TickerMode::Disabled => panic!("expected TickerMode::Real, got Disabled"),
        }
    }

    /// `MountError::StartPointMissing`（プレースホルダ ghost_root の不在という想定内の
    /// 事象）は良性と分類される（要件 8.2）。
    #[test]
    fn start_point_missing_is_classified_as_benign() {
        let err = GhostBootError::Mount(MountError::StartPointMissing {
            expected: PathBuf::from("ghost/master/descript.txt"),
        });
        assert!(is_benign_boot_error(&err));
    }

    /// `MountError::StartPointUnreadable`／`ShellDirMissing`（真に予期しない I/O 問題）は
    /// 良性ではないと分類される（要件 8.2）。
    #[test]
    fn other_mount_errors_are_not_classified_as_benign() {
        let unreadable = GhostBootError::Mount(MountError::StartPointUnreadable {
            path: PathBuf::from("ghost/master/descript.txt"),
            kind: std::io::ErrorKind::PermissionDenied,
        });
        assert!(!is_benign_boot_error(&unreadable));

        let shell_missing = GhostBootError::Mount(MountError::ShellDirMissing {
            expected: PathBuf::from("ghost/master/shell/master"),
        });
        assert!(!is_benign_boot_error(&shell_missing));
    }
}
