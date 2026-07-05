#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! areka 本番アプリ骨格
//!
//! wintf フレームワークを使用したデスクトップマスコット「ぱすたさん」の本番アプリ骨格。
//! この骨格は「アプリ起動の器」に徹する:
//! - 構造化ロギング初期化（RUST_LOG フォールバック）・パニックハンドラ設定
//! - 構成入力（ゴースト／バルーンのルートパス）の解決とログ出力（マウントはしない）
//! - UI ランタイム起動（`WinApp::new()`）・SHIORI 実走デモの env-gate 呼び口
//! - replace-me シーム（`open_startup_window`・本仕様では検証用ダミー窓を開く）
//! - `main` 自身が所有するメッセージループ（`app.run()`）とダミー窓 close での正常終了
//!
//! 本物のゴースト窓生成・配置・DPI 対応は下流仕様（ghost-setup／window-placement）の領分であり、
//! 骨格は座標・配置ロジックを一切持たない。旧モック UI は `examples/mock-shell.rs` へ退避済み。

use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;
use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension};
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::{Window, WindowStyle};
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

    // UI ランタイム起動（COM/DPI 初期化・World 生成・shutdown hook 結線）（R2.4）。
    let app = WinApp::new()?;

    // リファレンス脳の実走デモ（要件 5.3/5.4）。環境変数 `AREKA_SHIORI_DEMO` が有効な
    // ときのみ main スレッドで同期駆動する（既定 OFF）。診断目的のため失敗しても通常
    // 起動を中断せず、`app.run()` の UI 立ち上げを阻害しないよう必ずその前に完走させる。
    if let Err(e) = shiori_demo::run_demo_if_enabled() {
        tracing::error!(error = %e, "[main] shiori reference demo failed");
    }

    // replace-me シーム（R4.2）: 本仕様では検証用ダミー窓を 1 枚開き、`main` 所有の
    // `app.run()` ループに空遷移の heartbeat を与える。下流はここを本物のゴースト窓生成へ置換する。
    open_startup_window(&app);

    // `main` 所有のブロッキングメッセージループ（R2.4/R4.1）。ダミー窓が閉じられると
    // `WindowRegistry` が空へ遷移し `run()` が `Ok` を返して正常終了する（DD7 改定）。
    app.run()?;

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
    world
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
        .id()
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

/// replace-me シーム（task 2.2・R4.2）: 後続仕様（ghost-setup／window-placement）がここを
/// 本物のゴースト窓生成へ置き換える差し込み点。本仕様ではその本体が最小の検証用ダミー窓を
/// 1 枚開き、`main` 所有の `app.run()` ループに空遷移の heartbeat を与える（boot→loop→exit の実証）。
///
/// 署名が `&WinApp`（`&mut` でない）で足りる根拠: 窓生成は `WinApp::world()`（`&self`）→
/// `EcsWorld::spawn`（`&self`）経由＝現 `create_shell_window` 系の窓生成と同型で ECS 内部
/// 可変性を用いるため。投機的に `&mut` を先取りしない（下流が本物窓生成でより強い借用を
/// 要すれば、それはシーム署名変更の Revalidation Trigger）。
fn open_startup_window(app: &WinApp) {
    // `EcsWorld::spawn` の async タスク → CommandSender → Input スケジュールで World 適用
    // という ECS コマンド経路でダミー窓ビルダを走らせる。
    app.world().borrow().spawn(|tx: CommandSender| async move {
        let _ = tx.send(Box::new(|world: &mut World| {
            spawn_dummy_window(world);
            tracing::info!("検証用ダミー窓を開きました（replace-me シーム）");
        }));
    });

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
            "smoke 自動 close ゲート有効 — ダミー窓を指定 ms 後に despawn します"
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
                let dummies: Vec<Entity> = w
                    .query_filtered::<Entity, With<DummyWindowMarker>>()
                    .iter(w)
                    .collect();
                let count = dummies.len();
                for e in dummies {
                    w.despawn(e);
                }
                tracing::info!(count, "smoke 自動 close: ダミー窓を despawn しました");
            }
        });
    }
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
