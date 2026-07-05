#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! areka モック実装
//!
//! wintfフレームワークを使用したデスクトップマスコット「ぱすたさん」の最小動作実装。
//! - シェルウィンドウ: キャラクター画像（320×420px）の透過表示
//! - バルーンウィンドウ: 縦書きテキスト表示
//! - ドラッグ移動: シェルをドラッグするとバルーンが追従
//! - ダブルクリック終了: シェルをダブルクリックで全ウィンドウ終了

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;
use wintf::ecs::Point;
use wintf::ecs::drag::{DragConfig, DragEvent, OnDrag};
use wintf::ecs::layout::{
    BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, HitTest, LengthPercentageAuto, Rect,
};
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::{TextDirection, Typewriter, TypewriterTalk, TypewriterToken};
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::{
    ChildOf, FrameFinalize, FrameTime, SetWindowPosCommand, Window, WindowHandle, WindowPos,
    WindowStyle,
};
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
// Constants
// ---------------------------------------------------------------------------

/// バルーンのシェルに対するオフセット (x: シェル幅320 + 間隔15)
const BALLOON_OFFSET_X: i32 = 335;
const BALLOON_OFFSET_Y: i32 = 0;

/// シェルウィンドウの初期位置
const SHELL_INITIAL_X: i32 = 400;
const SHELL_INITIAL_Y: i32 = 200;

/// シェル画像パス（CARGO_MANIFEST_DIRからの相対）
const SHELL_IMAGE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shell/base.png");

/// バルーンに表示するテキスト
const BALLOON_TEXT: &str = "\
みんながもってる、記憶の糸。\n\
\n\
生まれてから、続いている、\n\
長い長い、一本の道。\n\
\n\
そう、きっと、一本道。\n\
いつか来る、終わりの日まで。\n\
\n\
ぱすた";

// ---------------------------------------------------------------------------
// Config Inputs (task 2.1)
// ---------------------------------------------------------------------------

/// 構成入力（解決済みルートパス）。
///
/// ゴースト／バルーンのルートパスを保持する。決定のみで実在は保証しない
/// （マウント・descript.txt 読取・`areka-parsers` 呼び出しは一切行わない・R6.1）。
///
/// task 3.1 で `main()` から結線・ログ出力されるまで未使用のため `dead_code` を許容する。
#[allow(dead_code)] // wired in task 3.1
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigInputs {
    ghost_root: std::path::PathBuf,
    balloon_root: std::path::PathBuf,
}

/// ゴーストルートの既定パス（`CARGO_MANIFEST_DIR` 相対・DD1）。
///
/// 既存の `SHELL_IMAGE_PATH` と同じ `env!("CARGO_MANIFEST_DIR")` 手法で決定的に生成する。
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
///
/// task 3.1 で `main()` から呼ばれるまで未使用のため `dead_code` を許容する。
#[allow(dead_code)] // wired in task 3.1
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

/// シェルウィンドウを識別するマーカーコンポーネント
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct ShellWindowMarker;

/// バルーンウィンドウを識別するマーカーコンポーネント
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct BalloonWindowMarker;

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

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // 非同期タスクでUI構築
    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    // クリック透過機構への窓登録システムを結線（task 4.1）。UISetup の create_windows が
    // WindowHandle を付与した後の FrameFinalize で走らせ、同一 tick 内で Added を捉える。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_click_through_windows);

    // 操作ガイド出力
    println!();
    println!("areka モック実装 — ぱすたさん");
    println!("================================");
    println!("  ドラッグ移動: シェル画像を左クリック & ドラッグ");
    println!("  終了:         シェル画像をダブルクリック");
    println!();

    // リファレンス脳の実走デモ（要件 6.1/6.8）。環境変数 `AREKA_SHIORI_DEMO` が有効な
    // ときのみ main スレッドで同期駆動する（既定 OFF）。診断目的のため失敗しても通常
    // 起動を中断せず、`mgr.run()` の UI 立ち上げを阻害しないよう必ずその前に完走させる。
    if let Err(e) = shiori_demo::run_demo_if_enabled() {
        tracing::error!(error = %e, "[main] shiori reference demo failed");
    }

    // ブロッキングメッセージループ
    mgr.run()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Async Setup
// ---------------------------------------------------------------------------

/// クリック透過機構への窓登録システム（task 4.1）。
///
/// WUC 化により ULW の自動 α ヒットテストが失われるため、機構が α を評価できるよう
/// shell/balloon の 2 窓を明示登録する。`WindowHandle` は `create_windows`（UISetup）が
/// HWND 生成後に付与するため `Added<WindowHandle>` で「HWND が付いた瞬間」を捉え、各窓を
/// 厳密に 1 回登録する（`register` は同一 Entity 再登録を dedupe するため冪等でもある）。
///
/// `ClickThroughRegistryHandle` は `WinApp::run` の結線で World へ NonSend リソースとして
/// 挿入される。本システムは `mgr.run()` 前に登録されるが tick は `run()` 開始後に回るため
/// 通常は存在する。ごく初期の tick で未挿入の可能性に備え `Option<NonSend<..>>` で防御し、
/// 未挿入なら no-op する（`Added` は次 tick で再度観測されるため取りこぼさない）。
/// `register` は `&self`（内部可変は `Rc<RefCell<..>>`）ゆえ `NonSend` で足りる。
/// 窓破棄時の除去は機構内 `prune_dead_targets`（Entity 生存確認）が担うため明示 remove は不要。
fn register_click_through_windows(
    new_windows: Query<
        (Entity, &WindowHandle),
        (
            Added<WindowHandle>,
            Or<(With<ShellWindowMarker>, With<BalloonWindowMarker>)>,
        ),
    >,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (entity, wh) in new_windows.iter() {
        handle.register(entity, wh.hwnd);
        tracing::debug!(?entity, "クリック透過機構へ窓を登録しました");
    }
}

/// 非同期タスクでシェル＋バルーンウィンドウを生成
async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        create_shell_window(world);
        create_balloon_window(world);
        tracing::info!("シェルウィンドウとバルーンウィンドウを生成しました");
    }));
}

// ---------------------------------------------------------------------------
// Shell Window
// ---------------------------------------------------------------------------

/// シェルウィンドウEntity構築
///
/// - WS_POPUP透過ウィンドウ（タイトルバーなし）
/// - BitmapSourceでキャラクター画像表示
/// - DragConfigでネイティブドラッグ有効化
/// - OnDrag / OnPointerPressedでインタラクション
fn create_shell_window(world: &mut World) -> Entity {
    let shell_entity = world
        .spawn((
            Name::new("Shell-Window"),
            ShellWindowMarker,
            Window {
                title: "areka shell".to_string(),
                // WUC 合成に固定化された。factory の compute_ex_style は合成モード選択を持たず、
                // 常に WS_EX_LAYERED を外し WS_EX_NOREDIRECTIONBITMAP を付与するため WindowStyle
                // の ex_style は据え置きでよい（下の WS_EX_LAYERED は factory が剥がす）。
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                // ex_style は factory が固定計算する（compute_ex_style が WS_EX_LAYERED を剥がす）。
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point {
                    x: SHELL_INITIAL_X,
                    y: SHELL_INITIAL_Y,
                }),
                ..Default::default()
            },
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(320.0)),
                    height: Some(Dimension::Px(420.0)),
                }),
                ..Default::default()
            },
            // 窓自身はヒット対象外（全面ヒットで透過を殺さない）。当たりは子の画像（α判定）が担う。
            HitTest::none(),
            DragConfig::default(),
            OnDrag(on_shell_drag),
            OnPointerPressed(on_shell_pressed),
        ))
        .id();

    // キャラクター画像 子Entity
    world.spawn((
        Name::new("Shell-Image"),
        BitmapSource::new(SHELL_IMAGE_PATH),
        // キャラの不透明ピクセルだけ受領・透明部は背面へ透過（αマスク自動生成の必須条件）。
        HitTest::alpha_mask(),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(320.0)),
                height: Some(Dimension::Px(420.0)),
            }),
            ..Default::default()
        },
        ChildOf(shell_entity),
    ));

    shell_entity
}

// ---------------------------------------------------------------------------
// Balloon Window
// ---------------------------------------------------------------------------

/// バルーンウィンドウEntity構築
///
/// - WS_POPUP透過ウィンドウ
/// - シェル右側に配置（+335px, +0px）
/// - 半透明クリーム色背景 + 縦書きTypewriterテキスト
fn create_balloon_window(world: &mut World) -> Entity {
    let balloon_entity = world
        .spawn((
            Name::new("Balloon-Window"),
            BalloonWindowMarker,
            Window {
                title: "areka balloon".to_string(),
                // WUC 合成に固定化された。factory の compute_ex_style は合成モード選択を持たず、
                // 常に WS_EX_LAYERED を外し WS_EX_NOREDIRECTIONBITMAP を付与するため据え置きでよい。
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                // ex_style は factory が固定計算する（compute_ex_style が WS_EX_LAYERED を剥がす）。
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point {
                    x: SHELL_INITIAL_X + BALLOON_OFFSET_X,
                    y: SHELL_INITIAL_Y + BALLOON_OFFSET_Y,
                }),
                ..Default::default()
            },
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(200.0)),
                    height: Some(Dimension::Px(350.0)),
                }),
                ..Default::default()
            },
        ))
        .id();

    // バルーン背景矩形（薄いクリーム色、半透明）
    let background = world
        .spawn((
            Name::new("Balloon-Background"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 0.95,
                a: 0.85,
            }),
            BoxStyle {
                flex_grow: Some(1.0),
                ..Default::default()
            },
            ChildOf(balloon_entity),
        ))
        .id();

    // 縦書き Typewriter テキスト
    let current_time = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);

    let tokens = build_typewriter_tokens(BALLOON_TEXT);

    world.spawn((
        Name::new("Balloon-Typewriter"),
        Typewriter {
            font_family: "メイリオ".to_string(),
            font_size: 16.0,
            direction: TextDirection::VerticalRightToLeft,
            default_char_wait: 0.08,
            ..Default::default()
        },
        Brushes::with_colors(
            D2D1_COLOR_F {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            },
            D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        ),
        TypewriterTalk::new(tokens, current_time),
        BoxStyle {
            flex_grow: Some(1.0),
            margin: Some(BoxMargin(Rect {
                left: LengthPercentageAuto::Px(8.0),
                right: LengthPercentageAuto::Px(8.0),
                top: LengthPercentageAuto::Px(8.0),
                bottom: LengthPercentageAuto::Px(8.0),
            })),
            ..Default::default()
        },
        ChildOf(background),
    ));

    balloon_entity
}

/// テキストを TypewriterToken 列に変換
///
/// - 行境界は必ず `\n` トークンを挿入（縦書き時は次カラムへ移動）
/// - 空行は `\n` + `Wait(0.3)` で段落間ポーズを表現
fn build_typewriter_tokens(text: &str) -> Vec<TypewriterToken> {
    let mut tokens = Vec::new();

    for (i, line) in text.split('\n').enumerate() {
        // 2行目以降は前の行との間に改行を挿入
        if i > 0 {
            tokens.push(TypewriterToken::Text("\n".to_string()));
        }
        if line.is_empty() {
            // 空行 → 段落間ポーズ
            tokens.push(TypewriterToken::Wait(0.3));
        } else {
            tokens.push(TypewriterToken::Text(line.to_string()));
        }
    }

    tokens
}

// ---------------------------------------------------------------------------
// Event Handlers
// ---------------------------------------------------------------------------

/// OnDrag ハンドラ: シェルドラッグ時にバルーンを追従移動させる
///
/// DragConfig { move_window: true } によりシェルウィンドウ自体は
/// wndprocレベルで自動的に移動される。このハンドラではバルーンの
/// 位置をシェルに合わせて更新する。
fn on_shell_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(_) => {
            // シェルの現在位置を取得（wndprocレベルで既に更新済み）
            let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
                return false;
            };

            // バルーンのHWNDを取得してSetWindowPosCommandを発行
            let mut query = world.query_filtered::<&WindowHandle, With<BalloonWindowMarker>>();
            if let Some(handle) = query.iter(world).next() {
                // 不変条件（A1-V）: pos は wndproc が実ウィンドウ位置から更新する
                // 論理座標であり、Windows の仮想スクリーン座標範囲に収まるため、
                // オフセット加算が i32 を溢れることはない（溢れは入力源の異常）。
                debug_assert!(
                    pos.x.checked_add(BALLOON_OFFSET_X).is_some()
                        && pos.y.checked_add(BALLOON_OFFSET_Y).is_some(),
                    "shell window position out of virtual-screen range: {pos:?}"
                );
                let new_x = pos.x + BALLOON_OFFSET_X;
                let new_y = pos.y + BALLOON_OFFSET_Y;

                SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
                    handle.hwnd,
                    new_x,
                    new_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    None,
                ));
            }

            false
        }
    }
}

/// OnPointerPressed ハンドラ: ダブルクリックで全ウィンドウを終了する
///
/// Phase::Bubble で DoubleClick::Left を検出し、
/// ShellWindowMarker / BalloonWindowMarker を持つ全エンティティを despawn する。
/// despawn → on_window_handle_remove → PostMessage(WM_CLOSE) → PostQuitMessage(0)
fn on_shell_pressed(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(state) => {
            if state.double_click == DoubleClick::Left {
                tracing::info!("ダブルクリック検出 — アプリケーションを終了します");

                // 全ウィンドウエンティティを収集して despawn
                // （on_window_handle_remove → WM_CLOSE → PostQuitMessage）
                let windows: Vec<Entity> = world
                    .query_filtered::<Entity, Or<(With<ShellWindowMarker>, With<BalloonWindowMarker>)>>()
                    .iter(world)
                    .collect();
                for e in windows {
                    world.despawn(e);
                }

                return true;
            }
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

/// `resolve_config_inputs` の単体テスト（task 2.1）。
///
/// main.rs は既に `mod tests`（ファイルモジュール）を持つため、名前衝突を避けて
/// 別名のインラインモジュールに置く。純粋関数のため World/wintf を触らず、`&[String]`
/// を直接与えて 3 分岐＋既定決定性を検証する。
#[cfg(test)]
mod config_input_tests {
    use super::{ConfigInputs, resolve_config_inputs, default_ghost_root, default_balloon_root};
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
        let args = vec![
            "areka.exe".to_string(),
            "C:/custom/ghost".to_string(),
        ];
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
