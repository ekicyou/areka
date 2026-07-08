//! areka emo-present 観測 example（task 4.2 ＋ 4.3）
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
//! `main.rs`（本番アプリ骨格）は一切変更しない（R6.6）。task 4.3 で以下を追加する:
//!
//! - **surface 切替の周期観測（R3.2・R6.4）**: wintf フレームクロック（`FrameTime`）駆動のタイマーで
//!   シェル target（`TargetId(0)`）を `surface0` → `surface1000`（bind `[1100,1200,1302]`）→ `Hide` →
//!   （反復）と数秒周期で巡回する。切替は必ず `EmoPresenter::apply`（指令 API）経由で行う。
//! - **バルーンのアンカーオフセット配置（R5.4）**: shell descript の `sakura.balloon.offsetx/offsety`
//!   を `areka_parsers::kv::parse_kv` で読み、あればそれを既定基準からの調整として適用する。emo2
//!   fixture は `sakura.balloon.alignment,left` を持つが offsetx/offsety は**無い**ため、実際に走るのは
//!   **既定整列**（バルーン右端＝シェル左端・上端揃え）の算出配置である（マジックギャップではなく計算）。
//!
//! 起動時 golden assert＝task 5.1・クリック観測＝task 5.2・実 DPI 記録＝task 5.3 は本 example の
//! スコープ外（別タスク）。
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
    FrameFinalize, FrameTime, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos,
    WindowStyle, WucGraphicsResource,
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

/// surface 切替の周期（秒）。数秒周期で `surface0` ⇄ `surface1000` ⇄ `Hide` を巡回する（R6.4）。
const CYCLE_INTERVAL_SECS: f64 = 2.5;

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
// Surface cycle（R6.4 の切替観測: surface0 ⇄ surface1000[binds] ⇄ Hide）
// ---------------------------------------------------------------------------

/// シェル target の巡回状態（数秒周期で遷移し、各遷移で `apply` を 1 回発行する）。
///
/// 初回表示は [`CycleState::Surface0`]（`boot_present_system` が装着直後に表示する状態）。以後
/// `cycle_present_system` が `next()` で `Surface1000`→`Hidden`→`Surface0`… と巡回させる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CycleState {
    /// emo2 `surface0`（bind 無し）。
    Surface0,
    /// emo2 `surface1000`（着せ替え bind `[1100,1200,1302]`＝腕/口/目の bindgroup default）。
    Surface1000,
    /// `\s[-1]` 相当の非表示。
    Hidden,
}

impl CycleState {
    /// 次の巡回状態へ進める。
    fn next(self) -> Self {
        match self {
            CycleState::Surface0 => CycleState::Surface1000,
            CycleState::Surface1000 => CycleState::Hidden,
            CycleState::Hidden => CycleState::Surface0,
        }
    }

    /// この状態へ遷移する際にシェル target（`TargetId(0)`）へ発行する指令を組む。
    ///
    /// 切替は必ず `EmoPresenter::apply`（指令 API）経由で行うため、状態ごとの `PresentCommand` を
    /// ここで一元的に定義する（bypass しない）。
    fn command(self) -> PresentCommand {
        match self {
            CycleState::Surface0 => PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 0,
                binds: BindSet::default(),
                reply: None,
            },
            CycleState::Surface1000 => PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::from_ids([1100, 1200, 1302]),
                reply: None,
            },
            CycleState::Hidden => PresentCommand::Hide {
                target: TargetId(0),
                reply: None,
            },
        }
    }
}

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
    /// シェル target が装着され巡回対象となったか（未装着シェルでは巡回しない）。
    shell_cycling: bool,
    /// シェル target の現在の巡回状態（装着直後は `Surface0`＝初回表示に一致）。
    cycle_state: CycleState,
    /// 次の切替を行う `FrameTime` 絶対時刻（秒）。装着完了時に `now + CYCLE_INTERVAL_SECS` で確定する。
    next_switch_at: f64,
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

    // フレームクロック駆動でシェル surface を数秒周期で巡回させる system（&mut World・UI スレッド）。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, cycle_present_system);

    // 操作ガイド出力。
    println!();
    println!("areka emo-present 観測 example");
    println!("================================");
    println!("  シェル窓（target 0）: emo2 surface0 ⇄ surface1000[binds] ⇄ 非表示 を数秒周期で巡回");
    println!("  バルーン窓（target 1）: balloons0.png（既定整列＝シェル左・上端揃え）");
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
        shell_cycling: false,
        cycle_state: CycleState::Surface0,
        next_switch_at: 0.0,
    };

    // シェル窓（surface 原寸で採寸・物理 px）。
    if let Some((emo_world, atlas, w, h)) = shell {
        boot.shell_window = create_shell_window(world, SHELL_INITIAL_X, SHELL_INITIAL_Y, w, h);
        boot.shell_assets = Some((emo_world, atlas));
        // バルーンはアンカーオフセット（R5.4）で配置する。descript に offsetx/offsety があれば
        // それを既定基準からの調整として適用し、無指定なら既定整列（バルーン右端＝シェル左端・
        // 上端揃え）を算出する（emo2 fixture は無指定ゆえ後者が実際に走る）。
        if let Some((b_world, b_atlas, bw, bh)) = balloon {
            let (balloon_x, balloon_y) = compute_balloon_pos(SHELL_INITIAL_X, SHELL_INITIAL_Y, bw);
            boot.balloon_window = create_balloon_window(world, balloon_x, balloon_y, bw, bh);
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
// Balloon anchor offset（R5.4・design「バルーン正典整理」）
// ---------------------------------------------------------------------------

/// バルーン窓の左上（物理 px・スクリーン座標）を算出する（R5.4）。
///
/// 基準は shell descript の正典整列: X「バルーンの右端がサーフェス左端に揃う位置」＋ Y「バルーン
/// 上端＝サーフェス上端」。`sakura.balloon.offsetx/offsety` があればこの基準からの調整として加算し、
/// 無指定なら基準そのもの（既定整列＝バルーン右端＝シェル左端・上端揃え）を返す。マジックギャップは
/// 用いず、シェル位置とバルーン幅から計算する。
///
/// - `shell_x`/`shell_y`: シェル窓左上（物理 px）。
/// - `balloon_w`: バルーン surface 原寸幅（物理 px）。
fn compute_balloon_pos(shell_x: i32, shell_y: i32, balloon_w: u32) -> (i32, i32) {
    // 既定基準: バルーン右端 = シェル左端 → 左上 x = シェル左端 − バルーン幅。上端揃え → y = シェル上端。
    let base_x = shell_x - balloon_w as i32;
    let base_y = shell_y;

    match read_balloon_offset() {
        Some((ox, oy)) => {
            tracing::info!(
                offsetx = ox,
                offsety = oy,
                "emo-present: descript の sakura.balloon.offsetx/offsety を既定基準へ適用"
            );
            (base_x + ox, base_y + oy)
        }
        None => {
            tracing::info!(
                base_x,
                base_y,
                "emo-present: balloon offset 無指定 — 既定整列（右端＝シェル左端・上端揃え）で配置"
            );
            (base_x, base_y)
        }
    }
}

/// shell descript（`shell/master/descript.txt`）から `sakura.balloon.offsetx/offsety` を読む。
///
/// 読取失敗・両キー欠如・整数化不能はいずれも `None`（既定整列へフォールバック）で返す（log-first・
/// panic しない）。emo2 fixture は両キーとも持たないため通常は `None` が返る（既定整列が走る）。
/// 部分指定（片方のみ）は仕様外ゆえ安全側で `None` とする。
fn read_balloon_offset() -> Option<(i32, i32)> {
    let path = emo2("shell/master/descript.txt");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "emo-present: descript.txt の読取に失敗 — 既定整列へフォールバック"
            );
            return None;
        }
    };

    let kv = areka_parsers::kv::parse_kv(&text);
    let ox = kv
        .get("sakura.balloon.offsetx")
        .and_then(|s| s.parse::<i32>().ok());
    let oy = kv
        .get("sakura.balloon.offsety")
        .and_then(|s| s.parse::<i32>().ok());

    match (ox, oy) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    }
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
            Ok(()) => {
                // 初回表示は Surface0（cycle_state の初期値と一致）。
                boot.presenter.apply(world, boot.cycle_state.command());
                // シェルが装着できた場合のみ巡回を有効化し、最初の切替時刻を確定する。
                let now = world.get_resource::<FrameTime>().map(|ft| ft.0).unwrap_or(0.0);
                boot.shell_cycling = true;
                boot.next_switch_at = now + CYCLE_INTERVAL_SECS;
            }
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

/// フレームクロック駆動でシェル target を数秒周期で巡回させる system（R3.2/R6.4）。
///
/// wintf の `FrameTime`（f64 秒・毎フレーム更新）を基準に経過を測り、[`CYCLE_INTERVAL_SECS`] を跨いだ
/// フレームで [`CycleState::next`] へ進めて対応する [`PresentCommand`] を `EmoPresenter::apply`（指令 API）で
/// 発行する（bypass しない）。装着（`boot_present_system`）が済み、かつシェルが巡回対象（`shell_cycling`）の
/// ときのみ動く。`apply`/presenter は `&mut World` と NonSend を要するため排他 system とし、切替が起きる
/// フレームだけ `EmoBoot` を remove→駆動→insert する（未到達フレームは peek のみで churn を避ける）。
fn cycle_present_system(world: &mut World) {
    // 現在時刻（フレームクロック）。未挿入時は 0.0（切替は起きない）。
    let now = world.get_resource::<FrameTime>().map(|ft| ft.0).unwrap_or(0.0);

    // 装着済み・巡回対象・切替時刻到達を peek で確認（未到達なら remove/insert しない）。
    let due = match world.get_non_send_resource::<EmoBoot>() {
        Some(b) if b.attached && b.shell_cycling => now >= b.next_switch_at,
        _ => return,
    };
    if !due {
        return;
    }

    let mut boot = world
        .remove_non_send_resource::<EmoBoot>()
        .expect("直上で存在確認済み");

    boot.cycle_state = boot.cycle_state.next();
    boot.next_switch_at = now + CYCLE_INTERVAL_SECS;
    let cmd = boot.cycle_state.command();
    boot.presenter.apply(world, cmd);
    tracing::info!(state = ?boot.cycle_state, "emo-present: シェル surface を切替");

    world.insert_non_send_resource(boot);
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
