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
//!   **表示経路と同じ 2 段変換**（直接合成＝native 原寸 → `resample`＝実適用 k）へ通した golden
//!   `ComposedSurface::bytes()` と **完全一致**することを `assert`（不一致は loud に panic）する。
//!   供給面が正当に未生成なら warn してスキップする。詳細は `assert_startup_golden` を参照。
//!
//! `areka-P0-emo-dpi-scaling` task 5.2 で以下を追加する:
//!
//! - **窓 client 寸の k reconcile（R7.1/R7.2）**: 各 `apply` の直後に
//!   `EmoPresenter::take_pending_resize` を消費し、窓 `WindowPos` を k 適用後の物理 px へ合わせる
//!   （[`reconcile_present_sizes`]）。k=1.0 ではべき等 skip となり従来と挙動同一。詳細は下記
//!   「DPI 表示契約」を参照。
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
//! # DPI 表示契約（R1.6・design「DPI 表示契約」＋ emo-dpi-scaling による更新）
//!
//! 窓サイズは `BoxStyle`/taffy 論理レイアウトを経由せず、**合成結果の物理 px を `WindowPos.size` へ
//! 直接与える**（この点は不変）。ただし「DPI による拡縮は行わない（等倍）」という当初の契約は
//! `areka-P0-emo-dpi-scaling` が**上書き**した — 現在の表示経路は compose（native 原寸）→ resample
//! （k＝窓 DPI ÷ author_dpi）であり、窓 DPI が author_dpi と異なれば表示は k 倍される（k=1.0 は
//! 「窓 DPI ＝ author_dpi」という**一水準**であって恒常の契約ではない）。
//!
//! 本 example は窓を **k 未適用の native 原寸**で生成する（起動時は実窓 DPI が未確定なため）。その後
//! **表示が成立した時点で窓 client を k 適用後の物理 px へ合わせる**（`areka-P0-emo-dpi-scaling`
//! task 5.2・[`reconcile_present_sizes`]）——`EmoPresenter::take_pending_resize` が積む「表示成立点の
//! 窓寸 reconcile 要求」を各 `apply` の直後に消費する流儀で、本番 boot（`emo2_boot` の
//! `emo2_frame_system` が drain の後段で直接呼ぶ `reconcile_reported_sizes`）と同一である。これが
//! 無いと k≠1.0 の環境で
//! 「窓 client（native 原寸）＜ 表示内容（k 倍）」となり、拡大表示もクリック捕捉域も窓 client の外側が
//! 切り詰められて**手動観測が劣化する**（golden assert は `read_back` が backbuffer 直読みゆえ無影響）。
//!
//! **DPI の動的追従（`Changed<DPI>`／モニタ跨ぎ移動）は本 example の領分ではない** — それは本番
//! `emo2_boot` の DPI 追従フェーズ（`run_dpi_phase`＝`refresh_scale`）が担う。本 example は
//! `refresh_scale` を呼ばないため、**起動前にスケーリングを設定してから**実行すること（下記手順 1）。
//!
//! **dpi≠96 のモニタ／スケーリング設定での実行確認**は task 5.3 の領分（本 example の rustdoc に手順を
//! 蓄積していく）。dpi=96 のみの確認は不十分である。
//!
//! ## 実 DPI（dpi≠96）実行手順（task 5.3・R1.6/R2.5/R6.5）
//!
//! 本手順は開発者が手動で行う実 DPI 検証（headless では代替不能）。**dpi=96 のみの実行では task 5.3
//! は完了しない** — 実際に dpi≠96 で走らせて結果を記録することが完了条件である。
//!
//! 1. **非 96 DPI を用意する（起動前に）**: Windows「設定 → システム → ディスプレイ → 拡大縮小」で
//!    対象モニタのスケールを 150% または 200%（dpi=144/192）に設定してから起動する。
//!    **起動後にモニタ跨ぎで窓を移動してはならない** — 上記のとおり本 example は `refresh_scale` を
//!    呼ばないため、移動後は「`apply_show` が毎回窓 DPI から k を再導出するシェルだけが追従し、
//!    再表示の無いバルーンは据え置き」という**非対称**になる。これは本 example が DPI 動的追従を
//!    持たないことの現れであって欠陥ではない（動的追従の観測は本番 boot ＝ task 6.5 の領分）。
//! 2. **起動する**: `cargo run -p areka --example emo-present` を実行する。
//! 3. **観測する**（3 点を確認する）:
//!    - (a) **k 追従表示**: シェル surface とバルーン枠が **k 倍された物理 px**（150% なら 3/2 倍・
//!      200% なら 2 倍）で描かれる。`apply(ShowSurface): 表示・マスクを更新` ログの `k_ratio`／
//!      `native_w/h`／`scaled_w/h` が実 DPI と整合することで判定できる（目視だけに頼らない）。
//!      窓 client も同じ物理寸へ合う（`emo-present: 窓 client を k 適用後の物理寸へ reconcile` ログの
//!      `w/h` が上記 `scaled_w/h` と一致する＝切り詰めが無い・emo-dpi-scaling task 5.2）。
//!    - (b) **起動時 golden 不 panic（task 5.1・R6.2/R8.2）**: 非 96 DPI でも `assert_startup_golden`
//!      が両 target で通る（panic せず「起動時 golden バイト一致を確認」ログが出る）。swap chain
//!      readback は**表示面の物理 px＝ k 適用後**を読み戻すため、golden 側も `resample` で同じ k を
//!      掛けてから比較する（k=1.0 なら resample を経ない素通しで従来と同一バイト）。
//!    - (c) **クリック捕捉**: キャラクタの不透明域をクリックすると task 5.2 の「不透明域クリックを
//!      捕捉」ログが発火し、透明域のクリックは背後へ透過する（ログ不発）。αマスクは表示バッファと
//!      **同一 bytes 由来**（＝k 適用後）ゆえ、クリック透過の境界は実 DPI でも見た目の絵柄と一致する。
//!      窓 client も k 適用後の物理寸へ揃う（上記 (a)）ため、捕捉域が窓 client の外側で切り詰められて
//!      「透明域だ」と誤認する経路は無い（emo-dpi-scaling task 5.2 以前はこれが起きていた）。
//!      なお `hit_region`（領域名解決）の座標系は native px であり、k≠1.0 での点÷k は下流
//!      `areka-P0-collision-dpi-hittest` の領分（本 example は領域名を引かない）。
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
    DPI, FrameFinalize, FrameTime, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos,
    WindowStyle, WucGraphicsResource,
};
use wintf::*;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, PackConfig, SetId, SurfaceSet, UseSelfAlpha, WicDecoderArm, bake,
};
use areka_emo_compose::{
    BindSet, ComposeError, ComposedSurface, Composer, EmoWorld, PatternState, ScaleRatio, resample,
};
use areka_emo_present::{
    EmoPresenter, PresentCommand, ScalePolicy, TargetId, build_balloon_target, derive_scale,
};

// ---------------------------------------------------------------------------
// Module wiring（本体分割・areka-P0-file-slimming タスク 8.11）
// ---------------------------------------------------------------------------
//
// 本ファイルは example バイナリターゲットの**クレートルート**なので、素の `mod fixture;` は
// `examples/fixture.rs` を探しに行き、そこにファイルがあれば新しい example ターゲットを生んで
// しまう。ゆえに接続は `#[path]` 必須である。同じ理由で `emo-present/` の下に `main.rs` を
// 置いてはならない（`crates/pilot/examples/shiori-host-32/main.rs` の形はターゲットになる）。
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
use self::fixture::{AUTHOR_DPI, CYCLE_INTERVAL_SECS, SHELL_INITIAL_X, SHELL_INITIAL_Y, emo2};
use self::input::on_shell_pressed;
use self::reconcile::{assert_startup_golden, cycle_present_system, reconcile_present_sizes};
use self::setup::run_setup;
use self::state::{BalloonWindowMarker, CycleState, EmoBoot, ShellWindowMarker};
use self::systems::{boot_present_system, register_click_through_windows};
use self::window::{create_balloon_window, create_shell_window};

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
