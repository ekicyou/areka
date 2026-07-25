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
//! task 5.1 で以下を追加する:
//!
//! - **起動時 golden バイト一致 assert（R6.2/R6.7/R8.2/R8.3）**: 各 target の初回 `apply(ShowSurface)`
//!   直後に `EmoPresenter::read_back` で swap chain backbuffer を CPU 読み戻しし、その surface を
//!   直接合成した golden `ComposedSurface::bytes()` と **完全一致**することを `assert`（不一致は loud に
//!   panic）する。供給面が正当に未生成なら warn してスキップする。詳細は `assert_startup_golden` を参照。
//!
//! task 5.2（不透明域クリック捕捉の観測）は `on_shell_pressed` の毎押下 `info!` ログを、task 5.3
//! （実 DPI 実行）は下記「実 DPI（dpi≠96）実行手順」を、本 example がそれぞれ観測シーム／手順として
//! 提供する。ただし実クリック操作と実 DPI 実走の記録自体は開発者の手動観測である（別タスク）。
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
//! ## 実 DPI（dpi≠96）実行手順（task 5.3・R1.6/R2.5/R6.5）
//!
//! 本手順は開発者が手動で行う実 DPI 検証（headless では代替不能）。**dpi=96 のみの実行では task 5.3
//! は完了しない** — 実際に dpi≠96 で走らせて結果を記録することが完了条件である。
//!
//! 1. **非 96 DPI を用意する**: Windows「設定 → システム → ディスプレイ → 拡大縮小」で対象モニタの
//!    スケールを 150% または 200%（dpi=144/192）に設定する。あるいは既にそのスケールで動いている
//!    モニタへ窓を移動する。
//! 2. **起動する**: `cargo run -p areka --example emo-present` を実行する。
//! 3. **観測する**（3 点を確認する）:
//!    - (a) **表示等倍（R1.6）**: シェル surface とバルーン枠が **surface 原寸の物理 px** で描かれる
//!      （ぼやけ／アップスケール無し）。窓のクライアント領域寸は合成結果の物理 px（`WindowPos.size` へ
//!      直接与えた値・DPI 表示契約）に一致し、スケール倍率で膨れない。
//!    - (b) **起動時 golden 不 panic（task 5.1・R6.2/R8.2）**: 非 96 DPI でも `assert_startup_golden`
//!      が両 target で通る（panic せず「起動時 golden バイト一致を確認」ログが出る）。swap chain
//!      readback は表示面の物理 px をそのまま読み戻すため、DPI に依らずバイト一致するはずである。
//!    - (c) **クリック座標一致（R2.5）**: キャラクタの不透明域をクリックすると task 5.2 の
//!      「不透明域クリックを捕捉」ログが発火し、透明域のクリックは背後へ透過する（ログ不発）。
//!      当たり判定境界が実 DPI で見た目の絵柄と一致する（R2.5 の恒等変換: bounds==αマスク==surface
//!      原寸で、DPI スケールによる座標ずれが生じない）。
//! 4. **記録する**: 上記 (a)(b)(c) の結果と使用した実 DPI 値（例 dpi=144/192）を記録する。
//!    **再掲**: dpi=96 のみは不十分 — dpi≠96 の実走記録をもって task 5.3 完了とする。
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
use areka_emo_compose::{BindSet, ComposeError, ComposedSurface, Composer, EmoWorld, PatternState};
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId, build_balloon_target};

// ---------------------------------------------------------------------------
// Constants / fixture paths
// ---------------------------------------------------------------------------

/// シェル窓の初期位置（物理 px・スクリーン座標）。
const SHELL_INITIAL_X: i32 = 400;
const SHELL_INITIAL_Y: i32 = 200;

/// まばたき開閉の周期（秒）。この周期で目開き ⇄ 目閉じ を surface1000 上でトグルする（R6.4 の切替観測）。
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

/// シェル target のまばたき巡回状態（[`CYCLE_INTERVAL_SECS`] 周期で開閉し、各遷移で `apply` を 1 回発行）。
///
/// さくらスクリプト `\s[1000]\![bind,腕,組み,1]\![bind,紅,差し,0]\![bind,口,‥‥,1]\![bind,眉,悲しみ,1]`
/// `\![bind,目,通常,1]\![bind,まばたき,通常,1]` 相当を seriko 代役でハンドコンパイルした表情を、
/// **まばたきアニメーションを指令切替で手動再現**する形で表示する（アニメ＝SERIKO ループは seriko 別 spec・未実装）。
/// 共通表情（腕組み=1101・口‥‥=1206・目通常=1302・眉悲しみ=1502・髪飾りリボン=1800／紅なし）は据え置き、
/// まばたき通常（1400）の有無だけをトグルする（1400 は静止合成で閉じまぶた(1412)を乗せる＝目を閉じる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CycleState {
    /// 目開き: まばたき bind 無し（目通常 1302 の開き目）。
    EyesOpen,
    /// 目閉じ: まばたき通常 1400 を加える（閉じまぶた 1412 が乗る）。
    EyesClosed,
}

impl CycleState {
    /// 次の巡回状態へ進める（開き⇔閉じのトグル）。
    fn next(self) -> Self {
        match self {
            CycleState::EyesOpen => CycleState::EyesClosed,
            CycleState::EyesClosed => CycleState::EyesOpen,
        }
    }

    /// この状態でシェル target（`TargetId(0)`）へ発行する指令を組む。
    ///
    /// 切替は必ず `EmoPresenter::apply`（指令 API）経由で行うため、状態ごとの `PresentCommand` を
    /// ここで一元的に定義する（bypass しない）。共通表情に対しまばたき（1400）のみ差分する。
    fn command(self) -> PresentCommand {
        // 共通表情: 腕組み・口‥‥・目通常・眉悲しみ・髪飾りリボン（紅なし）。
        let binds = match self {
            CycleState::EyesOpen => BindSet::from_ids([1101, 1206, 1302, 1502, 1800]),
            CycleState::EyesClosed => BindSet::from_ids([1101, 1206, 1302, 1400, 1502, 1800]),
        };
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds,
            pattern: PatternState::default(),
            reply: None,
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
    /// シェル target の現在のまばたき状態（装着直後は `EyesOpen`＝surface1000 初回表示に一致）。
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
    println!("  シェル窓（target 0）: emo2 surface1000 の着せ替え表情（腕組み・悲しみ眉・‥‥口）");
    println!("    ＋ まばたきを {CYCLE_INTERVAL_SECS} 秒周期で開閉再現（bind 1400 の指令切替）");
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
        cycle_state: CycleState::EyesOpen,
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
    let (w, h) = match Composer::new().compose(&emo_world, &atlas, 0, &BindSet::default(), &PatternState::default()) {
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

    let (w, h) = match Composer::new().compose(&emo_world, &atlas, 0, &BindSet::default(), &PatternState::default()) {
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
        // 起動時 golden（task 5.1・R6.2/R8.2）: 初回表示は Surface0（surface_id=0・bind 無し）ゆえ、
        // その surface を **直接合成**した ComposedSurface を golden として先に採取する。attach_target が
        // アセットを move 消費するため、合成は move の前に行う（read_back との突き合わせは表示直後）。
        let shell_golden = Composer::new().compose(&emo_world, &atlas, 0, &BindSet::default(), &PatternState::default());
        match boot.presenter.attach_target(
            world, TargetId(0), boot.shell_window, emo_world, atlas,
            // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
            // 供給する（本 example は k=1.0 相当で従来と同一の表示寸・描画結果）。
            96,
        ) {
            Ok(()) => {
                // 起動時 golden 検証用に surface0（bind 無し）を先に表示する（本編の巡回は surface1000 の
                // まばたきゆえ、golden の基準となる surface0 はここで一度だけ明示表示する）。
                boot.presenter.apply(
                    world,
                    PresentCommand::ShowSurface {
                        target: TargetId(0),
                        surface_id: 0,
                        binds: BindSet::default(),
                        pattern: PatternState::default(),
                        reply: None,
                    },
                );
                // 起動時 golden バイト一致 assert（R6.2/R6.7/R8.2/R8.3）: swap chain readback ==
                // 直接合成の golden を full byte equality で検証する（不一致は loud に panic）。
                assert_startup_golden(&boot.presenter, TargetId(0), shell_golden, "shell surface0");
                // --- 手動デモ: さくらスクリプト相当の表情＋まばたきを指令切替で再現 ---
                // \s[1000]\![bind,腕,組み,1]\![bind,紅,差し,0]\![bind,口,‥‥,1]\![bind,眉,悲しみ,1]
                //         \![bind,目,通常,1]\![bind,まばたき,通常,1] を seriko 代役でハンドコンパイル。
                // 共通表情（腕組み=1101・口‥‥=1206・目通常=1302・眉悲しみ=1502・髪飾りリボン=1800／紅なし）を
                // surface1000 に合成表示し、まばたき（1400）を CYCLE_INTERVAL_SECS 周期で出し入れして目の開閉を
                // 手動再現する（EyesOpen⇔EyesClosed）。まばたきの時間駆動アニメ本体は seriko エンジン領分（別 spec・未実装）。
                boot.presenter.apply(world, boot.cycle_state.command()); // 初回＝EyesOpen（surface1000）
                let now = world.get_resource::<FrameTime>().map(|ft| ft.0).unwrap_or(0.0);
                boot.shell_cycling = true;
                boot.next_switch_at = now + CYCLE_INTERVAL_SECS;
            }
            Err(e) => tracing::error!(error = %e, "emo-present: シェル target の attach に失敗"),
        }
    }

    if let Some((emo_world, atlas)) = boot.balloon_assets.take() {
        // 起動時 golden（task 5.1・R6.2/R8.2）: バルーンの初回表示は surface_id=0・bind 無し。
        // attach_target が move 消費する前に golden を採取する。
        let balloon_golden = Composer::new().compose(&emo_world, &atlas, 0, &BindSet::default(), &PatternState::default());
        match boot.presenter.attach_target(
            world, TargetId(1), boot.balloon_window, emo_world, atlas,
            // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が balloon descript の
            // 実値を供給する（本 example は k=1.0 相当で従来と同一の表示寸・描画結果）。
            96,
        ) {
            Ok(()) => {
                boot.presenter.apply(
                    world,
                    PresentCommand::ShowSurface {
                        target: TargetId(1),
                        surface_id: 0,
                        binds: BindSet::default(),
                        pattern: PatternState::default(),
                        reply: None,
                    },
                );
                // 起動時 golden バイト一致 assert（R6.2/R6.7/R8.2/R8.3）。
                assert_startup_golden(&boot.presenter, TargetId(1), balloon_golden, "balloon surface0");
            }
            Err(e) => tracing::error!(error = %e, "emo-present: バルーン target の attach に失敗"),
        }
    }

    boot.attached = true;
    world.insert_non_send_resource(boot);
    tracing::info!("emo-present: 2 窓へ surface0/バルーン枠を装着・表示しました");
}

/// 起動時 golden バイト一致 assert（task 5.1・R6.2/R6.7/R8.2/R8.3）。
///
/// 初回表示直後に target の表示画素を `EmoPresenter::read_back`（swap chain backbuffer の CPU 読み戻し・
/// R8.3）で取得し、その surface を **直接合成**した golden [`ComposedSurface`] のバイト列（[`ComposedSurface::bytes`]）
/// と **完全一致**（full byte equality）することを検証する。これが「供給面（swap chain readback）と合成結果の
/// 一致」（R8.2）を決定論的に確かめる検証シーム（R6.7）である。
///
/// # 失敗を silent にしない（R6.2）
///
/// バイト長・内容のいずれかが食い違えば即 `panic!`／`assert_eq!` で loud に落とす（target id・期待/実測長・
/// 先頭相違 index を添える）。観測失敗を warn ログで握り潰さない。
///
/// # 正当な非表示のスキップ
///
/// golden 合成に失敗した場合、または供給面が未生成（`read_back` が [`areka_emo_present::PresentError`] を返す・
/// EmptyComposition degradation 等で chain 不在）の場合は、`panic` せず warn ログを出してスキップする（表示すべき
/// ものが正当に無いだけで観測失敗ではない）。通常の emo2 fixture は両 target とも表示するため assert が走る。
fn assert_startup_golden(
    presenter: &EmoPresenter,
    target: TargetId,
    golden: std::result::Result<ComposedSurface, ComposeError>,
    label: &str,
) {
    let golden = match golden {
        Ok(cs) => cs,
        Err(e) => {
            tracing::warn!(
                ?target,
                error = %e,
                "emo-present: golden 合成に失敗 — {label} の起動時 golden assert をスキップ"
            );
            return;
        }
    };

    let actual = match presenter.read_back(target) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(
                ?target,
                error = %e,
                "emo-present: 供給面が未生成（read_back 不可）— {label} の起動時 golden assert をスキップ（正当な非表示）"
            );
            return;
        }
    };

    let expected = golden.bytes();

    // まず長さで loud に落とす（相違の一次要因を明示）。
    assert_eq!(
        actual.len(),
        expected.len(),
        "起動時 golden 不一致 [{label} / {target:?}]: read_back バイト長 {} が golden バイト長 {} と不一致 — swap chain readback が合成結果と食い違う（R6.2/R8.2 観測失敗）",
        actual.len(),
        expected.len(),
    );

    // full byte equality: 先頭相違 index を添えて loud に panic する。
    if let Some(idx) = actual.iter().zip(expected.iter()).position(|(a, b)| a != b) {
        panic!(
            "起動時 golden 不一致 [{label} / {target:?}]: 先頭相違 index={idx} (read_back=0x{:02X}, golden=0x{:02X}, len={}) — swap chain readback が合成結果とバイト不一致（R6.2/R8.2/R8.3 観測失敗）",
            actual[idx],
            expected[idx],
            actual.len(),
        );
    }

    tracing::info!(
        ?target,
        len = actual.len(),
        "emo-present: 起動時 golden バイト一致を確認（{label}）"
    );
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
    // ComposeCache は合成入力（surface id＋bind 集合）をキーにするため、同一 surface1000 でも
    // binds が異なる目開き/目閉じは自然にミス＝再合成される（InvalidateCache の手動発行は不要）。
    let cmd = boot.cycle_state.command();
    boot.presenter.apply(world, cmd);
    tracing::info!(state = ?boot.cycle_state, "emo-present: シェル surface を切替");

    world.insert_non_send_resource(boot);
}

// ---------------------------------------------------------------------------
// Event Handlers
// ---------------------------------------------------------------------------

/// OnPointerPressed ハンドラ: 不透明域クリックの捕捉ログ（task 5.2）＋ダブルクリック（左）終了。
///
/// **不透明域クリック捕捉の観測シーム（task 5.2・R2.2/R2.3/R6.3）**: クリック透過機構は αマスク
/// （`AlphaMask::is_hit`）に従って `WS_EX_TRANSPARENT` を動的トグルするため、pointer-pressed
/// イベントが本窓へ到達したこと自体が「クリックが不透明（αマスク有効）域へ着地した」証拠である
/// （透明域のクリックは背後プロセスへ透過し本ハンドラには**到達しない**）。ゆえに毎押下（単クリック
/// 含む）で 1 行だけ `info!` を出し捕捉を記録する（不在＝透明域透過の観測）。
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
            // 不透明域クリック捕捉ログ（task 5.2 の観測シーム・毎押下 1 行）。到達＝不透明域着地の証拠。
            tracing::info!(
                client_x = state.client_point.x,
                client_y = state.client_point.y,
                local_x = state.local_point.x,
                local_y = state.local_point.y,
                "emo-present: 不透明域クリックを捕捉（target=shell・αマスク有効域に着地＝透明域は背後へ透過し不到達）"
            );
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
