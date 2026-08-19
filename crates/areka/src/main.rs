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
// Config Inputs / Ghost Wiring
// ---------------------------------------------------------------------------

/// 起動時の構成入力解決と ghost 結線ヘルパ。
///
/// 本ファイルが 1,000 行規約を超えたため切り出した。挙動は不変で、可視性を
/// `pub(crate)` へ広げただけである（詳細はモジュール doc を参照）。
mod boot_config;

pub(crate) use boot_config::{
    ConfigInputs, default_app_profile_dir, default_helper_exe_path, ghost_boot_options,
    is_benign_boot_error, resolve_config_inputs,
};

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
    // 戻り値は配置準備が **1 度だけ**読んだ作者基準 DPI（areka-P0-emo-dpi-scaling task 4.3・
    // design Flow 3 手順1）。同じ値が採寸の k₀ と直下の `wire_emo2_boot`（→`attach_target`）
    // の双方へ渡ることで、採寸と表示が別々の宣言を見る食い違いを構造的に排除する。
    let author_dpi = open_startup_window(&app, &cfg);

    // emo2 統合結線（task 5.2・design「エントリポイント / main.rs＋wire_emo2_boot」・DD-7）:
    // UI 基盤・起動窓の後で完成済み 5 トラック（seriko／sakura／emo-present／emo-text／actor）を
    // 束ねる実 sink 結線を試みる。`wired=true` なら実 sink boot が成立し、ghost／seriko ハンドルを
    // 終了処理へ運ぶ。`wired=false`（asset 組立失敗・boot 失敗等）は現行の `LogSink`×2 フォール
    // バック boot へ倒し、既存 smoke 前提・非致命 boot 意味論を温存する（R7.1/7.3・DD-7）。
    // 配置準備が失敗した経路（fixture 不在等・上で warn!/error! 済み）は宣言値を読めていない。
    // 正典既定（96/96）へ縮退したことを観測可能にしたうえで結線を続行する（log-first・
    // 表示を失わない。なお準備が倒れている以上、この経路の `wire_emo2_boot` も通常は
    // 同じ起点不在で `wired=false` へ倒れる）。
    let author_dpi = author_dpi.unwrap_or_else(|| {
        tracing::warn!(
            shell_author_dpi = placement::AuthorDpi::DEFAULT.shell,
            balloon_author_dpi = placement::AuthorDpi::DEFAULT.balloon,
            "作者基準 DPI を配置準備から取得できません（準備失敗）——正典既定へ縮退して結線します"
        );
        placement::AuthorDpi::DEFAULT
    });
    let outcome = emo2_boot::wire_emo2_boot(
        &app,
        &cfg.ghost_root,
        &cfg.balloon_root,
        &helper_exe,
        author_dpi,
    );
    let (ghost_runtime, seriko_handle, loop_ticker) = if outcome.wired {
        tracing::info!("実 sink 結線で起動しました（emo2-boot wire 成立・SERIKO ループ ticker 稼働）");
        // マウス配信資源を World へ結線（task 3.1・design「main.rs＋wire_mouse_input」・
        // DD-IE-9）: kanade Sender クローンで MouseWiring（NonSend・Presenter）を挿入する。
        // 挿入は wire_emo2_boot 成功後＝Emo2Wiring 挿入済みゆえ presenter 経由の region 解決が
        // 成立する（Emo2Wiring 挿入と同位置・同型・self-gating）。窓へのハンドラ登録は task 3.2。
        if let Some(runtime) = outcome.ghost.as_ref() {
            let sender = runtime.kanade().clone();
            input_events::wire_mouse_input(app.world().borrow_mut().world_mut(), sender);
            // 位置永続の World 結線（task 6.2・design C4/C5・要件 1.9）: wire_mouse_input とは
            // 別行の additive 挿入。ゴースト窓を保持する同一 World（`wire_mouse_input` と同経路）へ
            // sylphya publisher clone を持つ PersistWiring（NonSend）を差し、DragEnd→persist_entries の
            // write-through 導管を確立する。
            insert_persist_wiring(
                app.world().borrow_mut().world_mut(),
                runtime.sylphya_publisher().clone(),
            );
        }
        // バルーン選択肢対話配線を World へ結線（task 6.2・design「main.rs＋wire_balloon_choice」・
        // R4.3/5.5/8.1）: mpsc チャネル生成＋`BalloonWiring`／`ChoiceSelectionInbox`（NonSend）挿入＋
        // `clear_balloon_hover_on_leave` の Input スケジュール登録（`dispatch_pointer_events` 後）を
        // 1 回・同期で行う（`wire_mouse_input` と同じ boot スロット＝schedule 実行外の World 変更ゆえ
        // `Schedules` 変更が安全に成立する）。ハンドラ装着は `open_startup_window` の spawn 直後
        // （`attach_balloon_pointer_handlers`）が担う。balloon ハンドラは `Emo2Wiring` を self-gate する
        // ため wired 経路でのみ意味を持つ（`wire_mouse_input` と同じ gating・DD-IE-9 前例）。
        input_events::balloon::wire_balloon_choice(app.world().borrow_mut().world_mut());
        // 選択確定通知の受信結線（areka-P0-choice-select-events task 5・design C1 ChoiceDrain・
        // Req1.1/1.2/1.5/1.6/3.7）: 直上 `wire_balloon_choice` が挿入した `ChoiceSelectionInbox`
        // （precondition）を毎フレーム drain し kanade へ全件転送する排他システムを登録する。
        // 位置・様式は `wire_mouse_input`（上方の同 boot スロット）と同型——kanade Sender クローンを
        // 持つ NonSend 資源挿入＋schedule 実行外の 1 回・同期呼出。
        if let Some(runtime) = outcome.ghost.as_ref() {
            input_events::choice_drain::wire_choice_drain(
                app.world().borrow_mut().world_mut(),
                runtime.kanade().clone(),
            );
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
                // 位置永続の World 結線（task 6.2・design C4/C5・要件 1.9）: fallback boot でも
                // 生きた runtime があれば wired 経路と同型に PersistWiring（NonSend）を同一 World へ
                // 挿入する（両経路で DragEnd→persist_entries の write-through 導管を確立）。
                insert_persist_wiring(
                    app.world().borrow_mut().world_mut(),
                    runtime.sylphya_publisher().clone(),
                );
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

/// 復元マージのシーム抽出（task 6.1・design C4・要件 1.4/1.5/5.1/6.1）。
///
/// `open_startup_window` の Ok アームが `spawn_ghost_windows` へ渡す placements を、起動時に
/// 先読みした永続 entries で差し替える（保存位置優先・毎起動 live 再射影）。`open_startup_window`
/// は実 `WinApp` を要してテスト困難ゆえ、純粋シーム（load→apply）を本ヘルパへ抽出し単体で
/// 檻に入れる（IO は [`placement::persist::load_restored_state`] の 1 点のみ・merge は純関数）。
///
/// `default_encoding` は boot 結線・`source.rs` と同一の [`areka_parsers::charset::DefaultEncoding`]
/// を渡すこと（mount 解決の一貫性のため）。呼び出し側は `DefaultEncoding::Ansi` を渡す。
fn restore_merged_placements(
    ghost_root: &std::path::Path,
    placements: Vec<placement::resolver::ScopePlacement>,
    snapshot: &placement::follow::MonitorSnapshot,
    default_encoding: areka_parsers::charset::DefaultEncoding,
) -> (
    Vec<placement::resolver::ScopePlacement>,
    std::collections::BTreeSet<usize>,
) {
    // 唯一の IO 点（design C1・A1 シーム）: mount 解決 → Ghost スコープ永続 entries 先読み。
    let entries = placement::persist::load_restored_state(ghost_root, default_encoding);
    // resolver 既定のキャラ位置を merge 前に控える（scg 7.3 の「既定配置か否か」判定の基準）。
    let defaults: Vec<(usize, placement::resolver::PointPx)> =
        placements.iter().map(|p| (p.scope, p.char_pos)).collect();
    // 純関数 merge（永続不書込・保存位置優先 → project_restore → balloon 導出）。
    let merged = placement::persist::apply_restored_placements(placements, &entries, snapshot);
    // 起動時関門（areka-P0-windowposition-limit design C6・要件 2.2/4.7/4.9/5.5/6.1）:
    // 経路①（spawn 初期値）と経路②（復元 merge）はどちらもこの合流点の出力を消費するため、
    // ここ 1 点で両方が被覆される。merge **後**に置くのは保存値優先の合流規則（4.7）を
    // 一切変えないためであり、補正は `balloon_pos`（表示位置）だけに作用する
    // ——`balloon_offset`（論理相対位置）は生値のまま（DD6・補正を焼き付けない）。
    let merged = placement::balloon_limit::apply_balloon_limit(merged, snapshot);
    // 保存位置が採用された（＝resolver 既定から動いた）スコープ集合。これらは**利用者の意思に
    // よる配置**であって既定配置ではないため、連鎖の再解決から常に除外される（scg 7.3）。
    // 保存値がたまたま既定と同値だった場合は差が出ないが、その位置は既定そのものゆえ
    // 既定配置として扱って差し支えない。
    let restored: std::collections::BTreeSet<usize> = merged
        .iter()
        .zip(defaults.iter())
        .filter(|(m, (_, d))| m.char_pos != *d)
        .map(|(m, _)| m.scope)
        .collect();
    (merged, restored)
}

/// `PersistWiring`（NonSend）を、ゴースト窓を保持する同一 World へ挿入するシーム抽出
/// （task 6.2・design C4/C5・要件 1.9）。
///
/// wired 経路（実 sink 結線成立）と fallback boot 経路（`LogSink`×2）の**両方**が、生きた
/// ghost runtime が存在するときにこのヘルパで `runtime.sylphya_publisher().clone()` を World の
/// NonSend リソースとして挿入する（`MouseWiring`／`Emo2Wiring` の NonSend 先例に倣う）。以降、
/// follow.rs の DragEnd 観測点が [`placement::persist::persist_entries`] 経由でこの publisher の
/// clone 送信端から保存 entries を write-through 投函できる（World レベルの配線導管）。
///
/// 生きた runtime が無い経路（wired の `None` ghost・fallback の `Err`・prepare 失敗のダミー窓）
/// では挿入しない＝従来どおり永続結線なし（`persist_entries` は `PersistWiring` 不在で debug!＋
/// no-op へ縮退・6.2）。挿入は純粋な World 変異ゆえ headless 単体テスト可能（`insert_non_send`
/// を薄く包み `#[cfg(test)]` の檻に入れる）。
fn insert_persist_wiring(world: &mut World, publisher: areka_sylphya::SylphyaPublisher) {
    world.insert_non_send(placement::persist::PersistWiring { publisher });
}

/// 起動時モニタスナップショットの構築＋出力シーム（areka-P0-dpi-window-vanish task 1.2・
/// 要件 1.1・design D12「areka 構築点を正典」）。
///
/// [`placement::follow::MonitorSnapshot`] は **placement の全判断が読む権威**（work area
/// 解決・アンカー射影・可視性判定がすべてこの Resource を引く）である。したがって
/// 要件 1.1 の正典出力点はこの構築点——ここで観測した値だけが「以後の判断が実際に見た値」
/// であり、他所で列挙し直した値ではない（D12）。
///
/// 出力は共有ヘルパ [`placement::diag::log_monitor_snapshot`] 1 本で、呼出点タグ
/// [`placement::MONITOR_SNAPSHOT_CONTEXT`] を名乗る。`prepare_ghost_windows` の列挙点も
/// 同じヘルパを別タグで呼ぶため、語彙は共有したままログ上で出所を弁別でき、
/// 両者の食い違いは grep 突合で検出できる（D12: 専用の突合機構は新設しない）。
///
/// 観測を足すだけで snapshot の中身は一切変えない（D2: 観測増設は Req 2.7 の
/// 「変更」に数えない）。生きた `WinApp` を要する `open_startup_window` から切り出した
/// シームゆえ、合成モニタで headless 檻に入る。
fn boot_monitor_snapshot(
    monitors: &[wintf::ecs::window::monitor::Monitor],
) -> placement::follow::MonitorSnapshot {
    placement::diag::log_monitor_snapshot(
        &placement::monitor_records(monitors),
        placement::MONITOR_SNAPSHOT_CONTEXT,
    );
    placement::follow::MonitorSnapshot::from_monitors(monitors)
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
///
/// 戻り値は準備が読んだ作者基準 DPI（`Some`＝準備成功時のみ・areka-P0-emo-dpi-scaling
/// task 4.3）。descript 読取は準備の中で 1 度だけ行われ、その値が (a) 採寸の k₀ と
/// (b) 呼び手（`main`）経由で `wire_emo2_boot`→`attach_target` の双方へ配られる
/// （design Flow 3 手順1「1 度だけ読む」）。準備失敗時は `None`（呼び手が正典既定へ縮退）。
fn open_startup_window(app: &WinApp, cfg: &ConfigInputs) -> Option<placement::AuthorDpi> {
    let author_dpi = match placement::prepare_ghost_windows(&cfg.ghost_root, &cfg.balloon_root) {
        Ok(prepared) => {
            // MonitorSnapshot（task 8.1・DD15 基盤）: 起動時の実モニタ work area 集合を
            // 忠実転写した Resource（物理 px・Send な純粋データ）。bottom 吸着ドラッグ
            // （4.7・task 8.2）が消費する。セッション内固定＝M1 受容
            // （WM_DISPLAYCHANGE 追随は後続・DD15）。
            // 構築と同時に全モニタの観測を 1 回出す（areka-P0-dpi-window-vanish 要件 1.1 の
            // **正典出力点**・D12）。既定 OFF・診断 `RUST_LOG` でのみ点灯する。
            let snapshot =
                boot_monitor_snapshot(&wintf::ecs::window::monitor::enumerate_monitors());

            // clickthrough 登録 system を FrameFinalize へ結線（task 5.2 の donor slot・
            // emo-present と同位置）。`Added<WindowHandle>` 駆動のため窓 spawn より先に
            // 結線しても取りこぼさない（registry NonSend は WinApp::run が挿入・5.2 learnings）。
            app.world().borrow_mut().add_systems(
                FrameFinalize,
                placement::spawn::register_ghost_windows_click_through,
            );

            // ゴースト窓ペアの重なり管理を同じ確定段へ結線（areka-P0-ghost-window-zorder
            // task 3.2・要件 1.1/5.6/6.1）。実行時ストラテジ（既定＝案 A・補助浮上なし）の
            // 明示挿入と、確立系 → 維持系の順での `FrameFinalize` 登録を
            // `wire_zorder_pair` 1 本にまとめてある（登録内容と理由は同関数の doc）。
            // clickthrough 登録と同じく `Added<WindowHandle>` 起点ゆえ、窓 spawn より
            // 先に結線しても取りこぼさない。
            placement::spawn::wire_zorder_pair(app.world().borrow_mut().world_mut());

            // 復元マージ（design C4・要件 1.4）: snapshot 構築直後・spawn closure へ渡す前に、
            // 永続先読み（load_restored_state）→ 純関数 merge（apply_restored_placements）で
            // 保存位置を反映した placements を得る。`prepared` を placements/titles へ分解し、
            // merge 済み placements（value 渡し）と titles を closure へ move する
            // （default_encoding は boot 結線・source.rs と同一の Ansi＝mount 解決の一貫性）。
            // 作者基準 DPI は `prepared` 分解の前に取り出して呼び手へ返す（`Copy` 値の転記）。
            let author_dpi = prepared.author_dpi;
            let (placements, restored_scopes) = restore_merged_placements(
                &cfg.ghost_root,
                prepared.placements,
                &snapshot,
                areka_parsers::charset::DefaultEncoding::Ansi,
            );
            let titles = prepared.titles;

            // `EcsWorld::spawn` の async タスク → CommandSender → Input スケジュールで
            // World 適用という既存 ECS コマンド経路（ダミー窓と同型）で本物窓を組み立てる。
            app.world().borrow().spawn(|tx: CommandSender| async move {
                let _ = tx.send(Box::new(move |world: &mut World| {
                    world.insert_resource(snapshot);
                    let windows = placement::spawn::spawn_ghost_windows(
                        world,
                        &placements,
                        &titles,
                    );
                    // 保存位置が復元されたスコープは既定配置ではない（scg 7.3）。台帳の
                    // 既定位置を落として連鎖の再解決から常に除外する——さもないと次回起動で
                    // 利用者のドラッグ位置が隣接位置へ引き戻される。spawn が Resource として
                    // 挿した実体を直接標す（戻り値の clone を触っても Resource へは効かない）。
                    if !restored_scopes.is_empty()
                        && let Some(mut gw) =
                            world.get_resource_mut::<placement::spawn::GhostWindows>()
                    {
                        for scope in &restored_scopes {
                            gw.clear_default_char_pos(*scope);
                        }
                    }
                    // マウス入力ハンドラ装着（areka-P0-input-events・依存方向 input_events→
                    // placement）: placement は `crate::` パスを持てない（example の `#[path]`
                    // include で成立させるため）ゆえ、キャラ窓へのポインタハンドラ結線は
                    // input_events 側が担う。spawn 直後の同一 World-mutation クロージャ内で
                    // 同期実行するため、キャラ窓は既に存在し async race はない。
                    input_events::attach_char_pointer_handlers(world);
                    // バルーン窓へポインタハンドラを装着（task 6.2・`attach_char_pointer_handlers`
                    // 直後・R4.3/5.5）: `BalloonWindowMarker` 窓へ `OnPointerMoved`／`OnPointerPressed`
                    // を post-spawn 挿入する（標的はバルーン窓のみ＝キャラ窓配線の非退行・R4.3）。同一
                    // `&mut World` クロージャ内で同期実行するためバルーン窓は既に存在し async race は
                    // ない（キャラ窓ハンドラ装着と同型のタイミング契約）。
                    input_events::balloon::attach_balloon_pointer_handlers(world);
                    let scopes: Vec<usize> = windows.scopes().collect();
                    tracing::info!(
                        ?scopes,
                        "本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓）"
                    );
                }));
            });
            Some(author_dpi)
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
            // 宣言値を読めていない（descript へ到達していない）——呼び手が既定へ縮退する。
            None
        }
    };

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

    author_dpi
}

/// smoke 自動 close の despawn 標的を despawn する（task 6.2 で
/// `Or<(With<DummyWindowMarker>, With<GhostWindowMarker>)>` へ拡張・design「main.rs seam」）。
///
/// ダミー窓（フォールバック経路）と本物のゴースト窓（placement 経路）のどちらの構成でも
/// CI smoke（`AREKA_APP_SMOKE_EXIT_MS`）が完走できるよう、両 marker を単一 query で狙う。
/// 標的として拾った件数を返す（標的なしは 0・no-op 安全）。bare `World` だけで動き headless
/// 単体テスト可能（`seam_tests`）。
///
/// # 存在確認（task 7.3・Req 6.2/6.3・design「変更ファイル > main.rs」）
///
/// query で集めた標的は**ループ実行中に**破棄済みへ変わり得る——bevy の連鎖 despawn
/// （`Children` は `LINKED_SPAWN` の関係対象＝親の despawn が子孫へ再帰する）を先行の
/// 1 体が引き起こせば、後続のイテレーションは既に無効な `Entity` を叩く。`World::despawn`
/// はその場合 `log` の `warn!`（`Could not despawn entity: …`）を出す（`bevy_ecs-0.18.1`
/// `src/world/mod.rs:1462-1469`）。**これは終了処理の正常終了系**であり、警告として残すと
/// 良性ノイズが本物の異常を埋める（Req 6.2）。
///
/// task 3.2 が消費側 4 入口（`follow.rs` の `resize_window_to`／`resize_window_keep_position`・
/// `frame.rs` の `resnap_with`／`reconcile_reported_sizes`）へ敷いたのと**同じ区別**を
/// despawn の**呼出点そのもの**へも敷く: entity 不在＝正常終了系ゆえ
/// [`DESPAWNED_SKIP_TAG`](placement::diag::DESPAWNED_SKIP_TAG) の `debug!` で当該標的を
/// 打ち切り、**残りの標的は処理し切る**（Req 6.3）。
///
/// 戻り値の意味は「標的として拾った件数」のまま変えない——連鎖で消えた標的も掃除後には
/// 存在しないため、`smoke 自動 close` の `count=` が示す「消えた起動窓の数」は不変である。
fn despawn_smoke_targets(world: &mut World) -> usize {
    let targets: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<DummyWindowMarker>, With<placement::spawn::GhostWindowMarker>)>>()
        .iter(world)
        .collect();
    let count = targets.len();
    for e in targets {
        if world.get_entity(e).is_err() {
            tracing::debug!(
                entity = ?e,
                "{} smoke 自動 close: 標的 entity は既に破棄済み（despawn・連鎖破棄）→ \
                 正常系として打ち切り（残りの標的は継続）",
                placement::diag::DESPAWNED_SKIP_TAG
            );
            continue;
        }
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
#[path = "main_startup_window_tests.rs"]
mod startup_window_tests;

/// main.rs シーム（task 6.2）の headless 単体テスト。
///
/// シーム結線そのもの（`open_startup_window`）は生きた `WinApp` を要するため、
/// TDD は headless で駆動可能な決定論部品——フォールバック分類
/// `is_benign_placement_error` と smoke 自動 close の despawn 標的
/// `despawn_smoke_targets`——で回す。結線の実証は実プロセス smoke
/// （`tests/smoke_boot_loop_exit.rs`・両方向）が担う。
#[cfg(test)]
#[path = "main_seam_tests.rs"]
mod seam_tests;

#[cfg(test)]
#[path = "main_config_input_tests.rs"]
mod config_input_tests;

/// ghost 結線ヘルパ（task 3.3）の headless 単体テスト。
///
/// `areka_ghost::boot`／`GhostRuntime::shutdown` 自体は実 I/O・実スレッドを伴うため、
/// ここでは純粋な組み立て・分類ロジック（`default_helper_exe_path`／`ghost_boot_options`／
/// `is_benign_boot_error`）だけを headless に検証する。実際の boot→shutdown 一巡は
/// 既存の実プロセス smoke テスト（`tests/smoke_boot_loop_exit.rs`）が証明する。
#[cfg(test)]
#[path = "main_ghost_wiring_tests.rs"]
mod ghost_wiring_tests;

/// 復元マージシーム（task 6.1・design C4・要件 1.4/1.5）の headless 単体テスト。
///
/// `open_startup_window` は実 `WinApp`（実 UI ランタイム）を要するためテスト困難ゆえ、
/// その Ok アームが `spawn_ghost_windows` へ渡す placements を作る純粋シーム
/// （`restore_merged_placements`＝`load_restored_state`→`apply_restored_placements`）を
/// 抽出して檻に入れる。植えた sylphya.toml の保存位置が既定位置に優先して merge 済み
/// placements の char_pos へ載ること（1.4）／永続不在なら既定 placement に恒等（1.5）を
/// 証明する（＝spawn される窓の初期位置が保存位置になる結線の証明）。
#[cfg(test)]
#[path = "main_restore_seam_tests.rs"]
mod restore_seam_tests;

/// `PersistWiring` 挿入シーム（task 6.2・design C4/C5・要件 1.9）の headless 単体テスト。
///
/// wired／fallback 両経路が使う挿入ヘルパ `insert_persist_wiring` を檻に入れる。
/// シーム結線そのもの（`main` の boot 経路分岐）は生きた `WinApp` を要するため、TDD は
/// headless で駆動可能な挿入ヘルパで回す。実 publisher（`spawn_sylphya`＋共有 fake IO）を
/// headless World へ挿入し、(a) NonSend リソース `PersistWiring` が存在すること、(b) その
/// World 越しの `persist_entries` 投函が barrier 後に別ハンドルの `load_scope` で読み戻せる
/// （＝World レベルの配線導管が正しく確立され DragEnd→file の World シームが成立している）
/// ことを証明する。DragEnd→file の完全な end-to-end は task 8.2 が担う。
#[cfg(test)]
#[path = "main_persist_wiring_seam_tests.rs"]
mod persist_wiring_seam_tests;

/// 起動時モニタスナップショット出力シーム（areka-P0-dpi-window-vanish task 1.2・
/// 要件 1.1・design D12「areka 構築点を正典」）の headless 単体テスト。
///
/// `open_startup_window` は生きた `WinApp` を要してテスト困難ゆえ、その Ok アームが
/// `MonitorSnapshot` を組む点＝**placement の全判断が読む権威の構築点**を
/// [`boot_monitor_snapshot`] へ抽出し、合成モニタ（混在 DPI・負座標・3200 超）で檻に入れる。
/// 実モニタも実 GPU も要さない。
#[cfg(test)]
#[path = "main_monitor_snapshot_seam_tests.rs"]
mod monitor_snapshot_seam_tests;
