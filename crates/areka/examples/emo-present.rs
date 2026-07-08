//! areka emo-present 観測 example（task 4.2）
//!
//! `areka_emo_present::EmoPresenter` を使い、**メモリ供給のスワップチェーン**（swap chain）を
//! WUC（Windows.UI.Composition）表示面へ載せて 2 窓を表示する観測用 example。mock-shell donor
//! （`examples/mock-shell.rs`）から窓生成（WS_POPUP・透過 ex-style）・クリック透過機構への窓登録
//! （`register_click_through_windows`・`Added<WindowHandle>`）・アプリ起動骨格を移植し、**表示内容の
//! 供給機構だけ**を `BitmapSource`（ファイルパス widget）から `EmoPresenter`（メモリ供給の swap chain）
//! へ差し替える。
//!
//! - **シェル窓**（target 0）: emo2 `surface0`（`surface0.png` 単一 element）。
//! - **バルーン窓**（target 1）: `balloons0.png`（`areka_emo_present::build_balloon_target` 経由）。
//!
//! `main.rs`（本番アプリ骨格）は一切変更しない（R6.6）。本 example は task 4.2 のスコープに絞り、
//! **surface0 ＋バルーン枠の常時表示**のみを行う（surface 切替タイマー＝task 4.3・起動時 golden
//! assert＝task 5.1 は含めない）。
//!
//! # 使い方
//! ```text
//! cargo run -p areka --example emo-present
//! ```
//! シェルをダブルクリックすると全窓を閉じて終了する。
//!
//! # DPI 表示契約（R1.6・design「DPI 表示契約」）
//!
//! emo-present が装着した窓のクライアント領域は **surface 原寸（物理 px）に一致**する。DPI による
//! 拡縮は行わない（等倍）。ゆえに窓サイズは `BoxStyle`/taffy 論理レイアウトを経由せず、合成結果の
//! 物理 px を `WindowPos.size` へ直接与える。**dpi≠96 のモニタ／スケーリング設定での実行確認**は
//! task 5.3 の領分（本 example の rustdoc に手順を蓄積していく）。dpi=96 のみの確認は不十分である。
//!
//! # UI スレッド適用（R7.2・design「M-boot は example が直接 apply を呼ぶ」）
//!
//! `EmoPresenter` は COM/GPU 資源を内包する `!Send`（NonSend）で、`attach_target`/`apply` は
//! UI スレッド（NonSend 到達可能スレッド）から `&mut World` で呼ぶ。窓生成・アセット構築・presenter
//! 生成は起動時コマンド（`CommandSender` 経由・UI スレッドで適用）で一度に行い、GPU 資源
//! （`GraphicsCore`/`WucGraphicsResource`）が揃ったフレームで `boot_present_system`（排他 system・
//! `&mut World`）が `attach_target`→`apply(ShowSurface)` を **各 target 高々 1 回**駆動する。将来の
//! kanade/seriko 結線では `PresentCommand`（`Send` 所有）を channel 経由で受け、同じ `apply` を呼ぶ
//! 形へ無改変で移行できる。

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;

use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::layout::HitTest;
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::{
    FrameFinalize, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos, WindowStyle,
    WucGraphicsResource,
};
use wintf::*;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, PackConfig, SetId, SurfaceSet, UseSelfAlpha, WicDecoderArm, bake,
};
use areka_emo_compose::{BindSet, Composer, EmoWorld};
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId, build_balloon_target};

// ---------------------------------------------------------------------------
// Constants / fixture paths
// ---------------------------------------------------------------------------

/// シェル窓の初期位置（物理 px・スクリーン座標）。
const SHELL_INITIAL_X: i32 = 400;
const SHELL_INITIAL_Y: i32 = 200;

/// バルーン窓のシェル右端からの間隔（task 4.2 は固定オフセット。descript 駆動配置は task 4.3）。
const BALLOON_GAP_X: i32 = 15;

/// fixture ルート（emo2）を `CARGO_MANIFEST_DIR`（`crates/areka`）相対で解決する。
/// fixtures は別クレート `crates/pilot` 配下ゆえワークスペース相対 `../pilot/...` を辿る
/// （emo-atlas の emo2 統合テストと同一アンカー規約）。
fn emo2(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
        .join(rel)
}

// ---------------------------------------------------------------------------
// Marker Components
// ---------------------------------------------------------------------------

/// シェル窓を識別するマーカー（クリック透過登録・終了 despawn の標的）。
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct ShellWindowMarker;

/// バルーン窓を識別するマーカー。
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct BalloonWindowMarker;

// ---------------------------------------------------------------------------
// Boot resource（NonSend・EmoPresenter を内包）
// ---------------------------------------------------------------------------

/// 起動時に構築した presenter・窓・アセットを束ね、GPU 資源が揃うまで attach/apply を保留する
/// NonSend リソース（`EmoPresenter` が `!Send` ゆえ本型も NonSend）。
///
/// `boot_present_system` が GPU 資源（`GraphicsCore`/`WucGraphicsResource`）到達フレームで
/// `attach_target`→`apply(ShowSurface)` を各 target 高々 1 回駆動し、`attached` を立てる。
struct EmoBoot {
    presenter: EmoPresenter,
    shell_window: Entity,
    balloon_window: Entity,
    /// シェル target のアセット（`attach_target` で move 消費・装着後は `None`）。
    shell_assets: Option<(EmoWorld, AtlasTable)>,
    /// バルーン target のアセット（同上）。
    balloon_assets: Option<(EmoWorld, AtlasTable)>,
    /// 装着＋初回表示を済ませたか（毎フレームの remove/insert churn を避けるゲート）。
    attached: bool,
}

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    // tracing-subscriber 初期化（RUST_LOG 対応・既定 info・非UTF-8/不正構文は info へフォールバック）。
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // 非同期タスク経由で「窓生成＋アセット構築＋presenter 生成＋EmoBoot 挿入」を UI スレッドへ投函する。
    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    // クリック透過機構への窓登録（mock-shell donor から移植・Added<WindowHandle> で厳密 1 回）。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_click_through_windows);

    // GPU 資源到達フレームで attach_target→apply を駆動する起動 system（&mut World・UI スレッド）。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, boot_present_system);

    // 操作ガイド出力。
    println!();
    println!("areka emo-present 観測 example");
    println!("================================");
    println!("  シェル窓（target 0）: emo2 surface0");
    println!("  バルーン窓（target 1）: balloons0.png");
    println!("  終了: シェルをダブルクリック");
    println!();

    mgr.run()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Async Setup（UI スレッドで適用されるコマンド）
// ---------------------------------------------------------------------------

/// 起動セットアップコマンドを UI スレッドへ送る。
///
/// 送信するクロージャ本体は UI スレッド（MTA・COM 初期化済み）で実行されるため、その中で
/// `WicDecoderArm`（COM 必要）を生成し実 PNG をデコードしてアセットを組める。クロージャは
/// `Send` 境界（`BoxedCommand`）を満たすが、`!Send` な `EmoPresenter` はクロージャ本体内の
/// ローカルとして生成し `insert_non_send_resource` で World へ載せる（キャプチャしない）。
async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        build_and_spawn(world);
    }));
}

/// アセット構築・窓生成・presenter 生成・`EmoBoot` 挿入を一括で行う（UI スレッド）。
fn build_and_spawn(world: &mut World) {
    // 実 WIC デコーダ（COM 初期化済み UI スレッドで生成）。実 PNG を復号する（MemoryDecoder は test 専用）。
    let decoder = match WicDecoderArm::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "emo-present: WicDecoderArm 生成に失敗（COM 未初期化？）— 中止");
            return;
        }
    };

    // シェル・バルーンのアセットを**シェルと同一経路**で構築（parse→bake→build）。
    let shell = build_shell_target(&decoder);
    let balloon = build_balloon_assets(&decoder);

    // どちらも構築できなければ表示する窓が無い（log-first・誤成功なし）。
    if shell.is_none() && balloon.is_none() {
        tracing::error!("emo-present: シェル・バルーンのアセット構築が双方失敗 — 窓を生成しない");
        return;
    }

    // presenter は生成のみ（attach/apply は GPU 資源到達後に boot_present_system が駆動する）。
    let presenter = EmoPresenter::new();

    let mut boot = EmoBoot {
        presenter,
        shell_window: Entity::PLACEHOLDER,
        balloon_window: Entity::PLACEHOLDER,
        shell_assets: None,
        balloon_assets: None,
        attached: false,
    };

    // シェル窓（surface 原寸で採寸・物理 px）。
    if let Some((emo_world, atlas, w, h)) = shell {
        boot.shell_window = create_shell_window(world, SHELL_INITIAL_X, SHELL_INITIAL_Y, w, h);
        boot.shell_assets = Some((emo_world, atlas));
        // バルーンはシェル右端＋間隔に置く（task 4.2 の固定整列。descript 駆動は task 4.3）。
        let balloon_x = SHELL_INITIAL_X + w as i32 + BALLOON_GAP_X;
        if let Some((b_world, b_atlas, bw, bh)) = balloon {
            boot.balloon_window =
                create_balloon_window(world, balloon_x, SHELL_INITIAL_Y, bw, bh);
            boot.balloon_assets = Some((b_world, b_atlas));
        }
    } else if let Some((b_world, b_atlas, bw, bh)) = balloon {
        // シェル無しでもバルーンだけは表示する（degrade・log は build_shell_target 側で出済み）。
        boot.balloon_window =
            create_balloon_window(world, SHELL_INITIAL_X, SHELL_INITIAL_Y, bw, bh);
        boot.balloon_assets = Some((b_world, b_atlas));
    }

    world.insert_non_send_resource(boot);
    tracing::info!("emo-present: 窓生成とアセット構築を完了（GPU 資源到達で表示を装着）");
}

/// シェル surface（emo2）を **シェル経路**（surfaces.txt→parse→bake→EmoWorld）で構築し、
/// surface0 の合成外形（物理 px）を添えて返す。失敗時は log-first で `None`。
fn build_shell_target(decoder: &WicDecoderArm) -> Option<(EmoWorld, AtlasTable, u32, u32)> {
    let base = emo2("shell/master");
    let surfaces_txt = base.join("surfaces.txt");
    let content = match std::fs::read_to_string(&surfaces_txt) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                path = %surfaces_txt.display(),
                error = %e,
                "emo-present: shell surfaces.txt の読取に失敗"
            );
            return None;
        }
    };
    let shell = areka_parsers::shell::parse(&content);
    if shell.surfaces.is_empty() {
        tracing::error!("emo-present: surfaces.txt が surface を 1 つも産まなかった");
        return None;
    }

    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: &base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], decoder, PackConfig::default());
    // emo2 shell は α 無し `purple/a/null.png` 1 枚が normalize seam として脱落する（既知・許容）。
    // surface0 は `surface0.png` のみを使うため合成に影響しない。他の脱落は制作者ミスの兆候ゆえ warn。
    for err in &baked.errors {
        tracing::warn!(error = %err, "emo-present: shell bake で脱落した element（surface0 表示には無害）");
    }

    let mut emo_world = EmoWorld::build(&shell);
    emo_world.bind_atlas(&baked.table, SetId(0));
    let atlas = baked.table;

    // surface0 を一度合成して窓の物理 px 外形を得る（DPI 表示契約: 窓クライアント寸 ≔ surface 原寸）。
    let (w, h) = match Composer::new().compose(&emo_world, &atlas, 0, &BindSet::default()) {
        Ok(cs) => (cs.width(), cs.height()),
        Err(e) => {
            tracing::error!(error = %e, "emo-present: shell surface0 の採寸合成に失敗");
            return None;
        }
    };
    if w == 0 || h == 0 {
        tracing::error!(w, h, "emo-present: shell surface0 の合成外形が 0 寸");
        return None;
    }
    Some((emo_world, atlas, w, h))
}

/// バルーン枠（`balloons0.png`）を `build_balloon_target`（シェルと同一経路）で構築し、
/// surface 0 の合成外形（物理 px）を添えて返す。失敗時は log-first で `None`。
fn build_balloon_assets(decoder: &WicDecoderArm) -> Option<(EmoWorld, AtlasTable, u32, u32)> {
    let dir = emo2("emo2-kakukaku");
    let (emo_world, atlas) = match build_balloon_target(&dir, decoder) {
        Ok(pair) => pair,
        Err(e) => {
            // build_balloon_target は内部で error! 済み（枠なし／bake 脱落）。ここは文脈を添えるのみ。
            tracing::error!(dir = %dir.display(), error = %e, "emo-present: バルーン target 構築に失敗");
            return None;
        }
    };

    let (w, h) = match Composer::new().compose(&emo_world, &atlas, 0, &BindSet::default()) {
        Ok(cs) => (cs.width(), cs.height()),
        Err(e) => {
            tracing::error!(error = %e, "emo-present: balloon surface0 の採寸合成に失敗");
            return None;
        }
    };
    if w == 0 || h == 0 {
        tracing::error!(w, h, "emo-present: balloon surface0 の合成外形が 0 寸");
        return None;
    }
    Some((emo_world, atlas, w, h))
}

// ---------------------------------------------------------------------------
// Window creation（mock-shell donor から移植・内容供給だけ差し替え）
// ---------------------------------------------------------------------------

/// シェル窓 Entity を構築する（WS_POPUP 透過窓・物理 px 採寸・αマスク当たりは emo-surface 子が担う）。
///
/// mock-shell と異なり `BitmapSource`／`BoxStyle` は使わない。表示内容は `EmoPresenter` が
/// `attach_target`→`apply` で装着する swap chain 供給面。窓クライアント寸は surface 原寸（物理 px）を
/// `WindowPos.size` へ直接与える（DPI 表示契約・taffy 非経由）。
fn create_shell_window(world: &mut World, x: i32, y: i32, w: u32, h: u32) -> Entity {
    world
        .spawn((
            Name::new("Emo-Shell-Window"),
            ShellWindowMarker,
            Window {
                title: "areka emo shell".to_string(),
                // WUC 合成固定。factory の compute_ex_style が WS_EX_LAYERED を剥がし
                // WS_EX_NOREDIRECTIONBITMAP を付与するため ex_style は据え置きでよい。
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point { x, y }),
                // 窓クライアント寸 ≔ surface 原寸（物理 px）。
                size: Some(SizeI {
                    width: w as i32,
                    height: h as i32,
                }),
                ..Default::default()
            },
            // 窓自身はヒット対象外（全面ヒットで透過を殺さない）。当たりは emo-surface 子（αマスク）が担う。
            HitTest::none(),
            // ダブルクリックで全窓を閉じて終了（手動観測の利便）。
            OnPointerPressed(on_shell_pressed),
        ))
        .id()
}

/// バルーン窓 Entity を構築する（シェルと同一機構・内容は EmoPresenter が装着）。
fn create_balloon_window(world: &mut World, x: i32, y: i32, w: u32, h: u32) -> Entity {
    world
        .spawn((
            Name::new("Emo-Balloon-Window"),
            BalloonWindowMarker,
            Window {
                title: "areka emo balloon".to_string(),
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point { x, y }),
                size: Some(SizeI {
                    width: w as i32,
                    height: h as i32,
                }),
                ..Default::default()
            },
            HitTest::none(),
        ))
        .id()
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// クリック透過機構への窓登録システム（mock-shell donor から移植）。
///
/// WUC 化により ULW の自動 α ヒットテストが失われるため、機構が α を評価できるよう shell/balloon の
/// 2 窓を明示登録する。`WindowHandle` は窓生成（UISetup）が HWND 生成後に付与するため
/// `Added<WindowHandle>` で「HWND が付いた瞬間」を捉え、各窓を厳密に 1 回登録する（`register` は
/// 同一 Entity 再登録を dedupe するため冪等でもある）。`ClickThroughRegistryHandle` は `WinApp::run`
/// の結線で NonSend リソースとして挿入される。ごく初期の tick で未挿入の可能性へ `Option` で防御する。
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
        tracing::debug!(?entity, "emo-present: クリック透過機構へ窓を登録");
    }
}

/// GPU 資源到達フレームで `attach_target`→`apply(ShowSurface)` を各 target 高々 1 回駆動する起動 system。
///
/// `EmoPresenter::apply`/`attach_target` は `&mut World` と NonSend な presenter を要するため排他 system
/// （`&mut World`）とし、`EmoBoot` を World から取り出して駆動する（借用衝突を避けるため remove→駆動→
/// insert）。GPU 資源（`GraphicsCore`/`WucGraphicsResource`）は wintf が窓生成後に遅延挿入するため、
/// 揃うまでは保留し次 tick で再試行する。装着後は `attached` で以降を no-op 化する。
fn boot_present_system(world: &mut World) {
    // 未挿入 or 装着済みなら何もしない（装着後の remove/insert churn を避ける）。
    match world.get_non_send_resource::<EmoBoot>() {
        Some(b) if !b.attached => {}
        _ => return,
    }

    // GPU 資源の準備待ち（未準備なら EmoBoot を保持したまま次 tick へ）。
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    let mut boot = world
        .remove_non_send_resource::<EmoBoot>()
        .expect("直上で存在確認済み");

    if let Some((emo_world, atlas)) = boot.shell_assets.take() {
        match boot
            .presenter
            .attach_target(world, TargetId(0), boot.shell_window, emo_world, atlas)
        {
            Ok(()) => boot.presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: TargetId(0),
                    surface_id: 0,
                    binds: BindSet::default(),
                    reply: None,
                },
            ),
            Err(e) => tracing::error!(error = %e, "emo-present: シェル target の attach に失敗"),
        }
    }

    if let Some((emo_world, atlas)) = boot.balloon_assets.take() {
        match boot
            .presenter
            .attach_target(world, TargetId(1), boot.balloon_window, emo_world, atlas)
        {
            Ok(()) => boot.presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: TargetId(1),
                    surface_id: 0,
                    binds: BindSet::default(),
                    reply: None,
                },
            ),
            Err(e) => tracing::error!(error = %e, "emo-present: バルーン target の attach に失敗"),
        }
    }

    boot.attached = true;
    world.insert_non_send_resource(boot);
    tracing::info!("emo-present: 2 窓へ surface0/バルーン枠を装着・表示しました");
}

// ---------------------------------------------------------------------------
// Event Handlers
// ---------------------------------------------------------------------------

/// OnPointerPressed ハンドラ: ダブルクリック（左）で全窓を despawn し終了する（mock-shell と同型）。
///
/// despawn → `on_window_handle_remove` → `PostMessage(WM_CLOSE)` → `WindowRegistry` 空遷移 →
/// `run()` 復帰、という wintf の作法に委ねる。
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
                tracing::info!("emo-present: ダブルクリック検出 — 全窓を閉じて終了します");
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
