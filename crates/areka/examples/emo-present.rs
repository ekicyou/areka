//! areka emo-present 観測 example
//!
//! `areka_emo_present::EmoPresenter` を使い、**原寸バイトを記録した `GraphicsCommandList`** を
//! WUC（Windows.UI.Composition）表示面へ載せて 2 窓を表示する観測用 example。窓生成（WS_POPUP・
//! 透過 ex-style）・クリック透過機構への窓登録（`register_click_through_windows`・`Added<WindowHandle>`）・
//! アプリ起動骨格を持ち、表示内容は `EmoPresenter`（合成した原寸バイトをコマンドリストへ記録し、
//! 拡大は D2D の変換行列が掛ける経路）が供給する。
//!
//! - **シェル窓**（target 0）: emo2 `surface0` を照合のために 1 度出した後、`surface1000` の表情（下のまばたきの巡回）。
//! - **バルーン窓**（target 1）: `balloons0.png`（`areka_emo_present::build_balloon_target` 経由）。
//!
//! 観測できること:
//!
//! - **surface 切替の周期観測**: wintf フレームクロック（`FrameTime`）駆動のタイマーで
//!   シェル target（`TargetId(0)`）に `surface0` を出した後、`surface1000` のまばたき bind `1400` を
//!   数秒周期で加えては外す。切替は必ず `EmoPresenter::apply`（指令 API）経由で行う。
//! - **バルーンのアンカーオフセット配置**: shell descript の `sakura.balloon.offsetx/offsety`
//!   を配置の crate の `build_placement_config` で読み、あればそれを既定基準からの調整として適用する。emo2
//!   fixture は `sakura.balloon.alignment,left` を持つが offsetx/offsety は**無い**ため、実際に走るのは
//!   **既定整列**（バルーン右端＝シェル左端・上端揃え）の算出配置である。
//! - **起動時 golden バイト一致 assert**: 各 target の初回 `apply(ShowSurface)`
//!   直後に `EmoPresenter::read_back` で表示中エントリの **native 原寸バイト列**を読み、画面の外の描き手
//!   （画面と同じ hardware の装置）で描いた golden `ComposedSurface::bytes()` と **完全一致**することを `assert`（不一致は loud に panic）する。
//!   拡大は GPU 変換の領分ゆえ golden に k は掛けず、**k に依らず同一バイト・同一長**である。
//!   表示に渡した合成面が正当に無ければ warn してスキップする。詳細は `assert_startup_golden` を参照。
//! - **窓 client 寸の k reconcile**: 各 `apply` の直後に
//!   `EmoPresenter::take_pending_resize` を消費し、窓 `WindowPos` を k 適用後の物理 px へ合わせる
//!   （[`reconcile_present_sizes`]）。k=1.0 ではべき等 skip となる。
//! - **不透明域クリック捕捉**: `on_shell_pressed` が毎押下 `info!` ログを出す。
//!
//! # 使い方
//! ```text
//! cargo run --manifest-path <絶対パス>\Cargo.toml -p areka --example emo-present
//! ```
//! シェルをダブルクリックすると全窓を閉じて終了する。
//!
//! # DPI 表示契約
//!
//! 窓サイズは `BoxStyle`/taffy 論理レイアウトを経由せず、**合成結果の物理 px を `WindowPos.size` へ
//! 直接与える**。表示経路は計画（native 原寸）→ 描画の節へ原寸記録 → wintf の
//! `render_surface` が `SetTransform` で **×k**（k＝窓 DPI ÷ author_dpi）であり、窓 DPI が
//! author_dpi と異なれば表示は k 倍される（k=1.0 は「窓 DPI ＝ author_dpi」という一水準にすぎない）。
//!
//! 本 example は窓を **k 未適用の native 原寸**で生成する（起動時は実窓 DPI が未確定なため）。その後
//! **表示が成立した時点で窓 client を k 適用後の物理 px へ合わせる**（[`reconcile_present_sizes`]）。
//! `EmoPresenter::take_pending_resize` が積む「表示成立点の窓寸 reconcile 要求」を各 `apply` の直後に
//! 消費する流儀で、本番 boot（`runtime` の `frame_system` が drain の後段で直接呼ぶ
//! `reconcile_reported_sizes`）と同一である。これが無いと k≠1.0 の環境で
//! 「窓 client（native 原寸）＜ 表示内容（k 倍）」となり、拡大表示もクリック捕捉域も窓 client の外側が
//! 切り詰められて**手動観測が劣化する**（golden assert は `read_back` が合成メモの原寸直読みゆえ無影響）。
//!
//! **DPI の動的追従（`Changed<DPI>`／モニタ跨ぎ移動）は本 example の領分ではない** — それは本番
//! `runtime` の DPI 追従フェーズ（`run_dpi_phase`＝`refresh_scale`）が担う。本 example は
//! `refresh_scale` を呼ばないため、**起動前にスケーリングを設定してから**実行すること（下記手順 1）。
//!
//! ## 実 DPI（dpi≠96）実行手順
//!
//! 開発者が手動で行う実 DPI 検証（headless では代替不能）。dpi=96 のみの確認は不十分である。
//!
//! 1. **非 96 DPI を用意する（起動前に）**: Windows「設定 → システム → ディスプレイ → 拡大縮小」で
//!    対象モニタのスケールを 150% または 200%（dpi=144/192）に設定してから起動する。
//!    **起動後にモニタ跨ぎで窓を移動してはならない** — 本 example は `refresh_scale` を
//!    呼ばないため、移動後は「`apply_show` が毎回窓 DPI から k を再導出するシェルだけが追従し、
//!    再表示の無いバルーンは据え置き」という**非対称**になる。
//! 2. **起動する**: `cargo run --manifest-path <絶対パス>\Cargo.toml -p areka --example emo-present` を実行する。
//! 3. **観測する**（3 点を確認する）:
//!    - (a) **k 追従表示**: シェル surface とバルーン枠が **k 倍された物理 px**（150% なら 3/2 倍・
//!      200% なら 2 倍）で描かれる。`apply(ShowSurface): 表示・マスクを更新` ログの `k_ratio`／
//!      `native_w/h`／`scaled_w/h` が実 DPI と整合することで判定できる（目視だけに頼らない）。
//!      窓 client も同じ物理寸へ合う（`emo-present: 窓 client を k 適用後の物理寸へ reconcile` ログの
//!      `w/h` が上記 `scaled_w/h` と一致する＝切り詰めが無い）。
//!    - (b) **起動時 golden 不 panic**: 非 96 DPI でも `assert_startup_golden`
//!      が両 target で通る（panic せず「起動時 golden バイト一致を確認」ログが出る）。`read_back` が
//!      返すのは **native 原寸**ゆえ golden に k は掛けず、k に依らず同一バイト・同一長で比較する。
//!    - (c) **クリック捕捉**: キャラクタの不透明域をクリックすると「不透明域クリックを
//!      捕捉」ログが発火し、透明域のクリックは背後へ透過する（ログ不発）。クリックの透過は画面に
//!      出ている描画面の入れ物の 1 画素の α（wintf の `HitTestMode::SurfaceAlpha`）で決まるので、
//!      境界は実 DPI でも見た目の絵柄と一致する。
//!      なお領域名は `EmoPresenter::hit_region_client` が窓 client 物理 px から引く（本 example は
//!      領域名を引かない）。
//! 4. **記録する**: 上記 (a)(b)(c) の結果と使用した実 DPI 値（例 dpi=144/192）を記録する。
//!
//! # UI スレッド適用
//!
//! `EmoPresenter` は COM/GPU 資源を内包する `!Send`（NonSend）で、`attach_target`/`apply` は
//! UI スレッド（NonSend 到達可能スレッド）から呼ぶ（`&mut World` を要するのは `apply` だけ）。窓生成・アセット構築・presenter
//! 生成は起動時コマンド（`CommandSender` 経由・UI スレッドで適用）で一度に行い、GPU 資源
//! （`GraphicsCore`/`WucGraphicsResource`）が揃ったフレームで `boot_present_system`（排他 system・
//! `&mut World`）が `attach_target`→`apply(ShowSurface)` を **各 target 高々 1 回**駆動する。

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
    DPI, FrameFinalize, FrameTime, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos,
    WindowStyle, WucGraphicsResource,
};
use wintf::*;

use areka_emo_atlas::{AtlasTable, WicDecoderArm};
use areka_emo_compose::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState, ScaleRatio};
use areka_emo_present::{
    EmoPresenter, OffscreenRenderer, PresentCommand, PresentError, ScalePolicy, TargetId,
    build_balloon_target, derive_scale, load_shell_target,
};

// `#[path]` 必須: 素の `mod fixture;` は `examples/fixture.rs` を探しに行き、そこにファイルがあれば
// 新しい example ターゲットを生んでしまう。同じ理由で `emo-present/` の下に `main.rs` を置かない。
// 子は `use super::{…}` でこのルートの `use` 束縛を引く（再輸出が未使用にならない形）。
#[path = "emo-present/balloon.rs"]
mod balloon;
#[path = "emo-present/fixture.rs"]
mod fixture;
#[path = "emo-present/input.rs"]
mod input;
#[path = "emo-present/reconcile.rs"]
mod reconcile;
#[path = "emo-present/setup.rs"]
mod setup;
#[path = "emo-present/state.rs"]
mod state;
#[path = "emo-present/systems.rs"]
mod systems;
#[path = "emo-present/window.rs"]
mod window;

use self::balloon::compute_balloon_pos;
use self::fixture::{
    AUTHOR_DPI, CYCLE_INTERVAL, SHELL_INITIAL_X, SHELL_INITIAL_Y, emo2, emo2_balloon,
};
use self::input::on_shell_pressed;
use self::reconcile::{assert_startup_golden, cycle_present_system, reconcile_present_sizes};
use self::setup::run_setup;
use self::state::{BalloonWindowMarker, CycleState, EmoBoot, ShellWindowMarker};
use self::systems::{boot_present_system, register_click_through_windows};
use self::window::{create_balloon_window, create_shell_window};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_click_through_windows);

    world
        .borrow_mut()
        .add_systems(FrameFinalize, boot_present_system);

    world
        .borrow_mut()
        .add_systems(FrameFinalize, cycle_present_system);

    println!();
    println!("areka emo-present 観測 example");
    println!("================================");
    println!("  シェル窓（target 0）: emo2 surface1000 の着せ替え表情（腕組み・悲しみ眉・‥‥口）");
    println!("    ＋ まばたきを {CYCLE_INTERVAL} 周期で開閉再現（bind 1400 の指令切替）");
    println!("  バルーン窓（target 1）: balloons0.png（既定整列＝シェル左・上端揃え）");
    println!("  終了: シェルをダブルクリック");
    println!();

    mgr.run()?;

    Ok(())
}
