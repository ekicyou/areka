//! 当たり判定の実 DPI 受け入れ probe（本番 emo2 の表示と実モニタ DPI ≠ 96 で、窓 client 物理 px→サーフェス px の縮約を実測して証跡化する）
//!
//! リゾルバ [`resolve_hit_region`] の座標契約は、窓 client 物理 px を presenter が ÷k してサーフェス px へ
//! 縮約することである。
//!
//! # 1 実行 = 1 DPI 水準
//!
//! 実適用 k は実行機の表示スケールがそのまま決める（wintf は窓生成時に実モニタ DPI で `DPI` component を
//! 初期化するため、初回表示の時点で確定する）ので、DPI 追従駆動は持たない。2 水準の確認は、OS の表示
//! スケールを変えて（例 125% → 200%）2 回実行し、各実行の `physical=` が互いに異なることを記録上で照合する。
//!
//! # 期待 k ゲート（env `AREKA_COLLISION_PROBE_EXPECT_K`）
//!
//! 水準ごとの期待 k を `"5/4"`（分数）または `"2"`（整数）で与えると、実適用 k
//! （[`EmoPresenter::applied_ratio`]）との厳密一致を hard assert する。未指定なら assert せず実測ログのみ
//! なので、開発機ではそのまま実行できる。
//!
//! # greppable ログ
//!
//! ```text
//! collision-probe: k=<num>/<den> native=<w>x<h> physical=<w>x<h>
//! collision-probe: client=(x,y) surface=(sx,sy) region=<name>
//! ```
//!
//! 前者は 1 実行に 1 回（表示成立時）、後者はカーソル解決ごとに出る。`surface=` は presenter が縮約した
//! `HitRegion::surface_point` の値そのままで、probe 側で ÷k を再実装しない。純粋層の決定論テストとは
//! 別の観測であり（examples は手動検証の補助でテストの代替ではない）、両者は併存する。
//!
//! # crate の公開面で読む
//!
//! `areka_runtime::target_map`（scope→`TargetId` 写像）と `areka_runtime::hit_region`
//! （[`resolve_hit_region`]／`HitRegion`）、配置の `areka_placement::…`（`spawn_ghost_windows`／
//! `resize_window_to`）はいずれも公開面から使う（`#[path]` の私有 include はしない）。`pointer.rs`・
//! `probe.rs` の `super::target_map`・`super::resolve_hit_region` は、ここの `use` が example
//! クレートルートへ持ち込んだ名前へ解決される。
//!
//! # donor（`emo-present.rs`・`window-placement.rs`）からの必須逸脱
//!
//! 1. **表示対象は `surface1000`**: emo2 で当たり判定を持つのは `surface1000` だけ。`surface0`（donor の
//!    既定）は当たり判定を 1 つも持たず空集合しか観測できない。
//! 2. **`BindSet` は有効 bind の実値集合** `[1101, 1206, 1302, 1502, 1800]`: `surface1000` は静的 element を
//!    持たず全パーツが bind 制御なので、`BindSet::default()` では全透明の窓が「成功」として出て狙える絵が無い。
//! 3. **窓 client 寸は本番の窓寸規則で与える**: `spawn_ghost_windows` を意図的に誤った placeholder 寸
//!    （100×100）で呼んで窓を作り、`apply(ShowSurface{1000, binds})` 成立後に
//!    [`EmoPresenter::target_physical_size`]（k 適用後の物理寸）を読み、本番の反映関数
//!    [`areka_placement::follow::resize_window_to`] で窓へ適用する（戻り値 `true` を assert）。最終一致が
//!    本番 resize 経路を通ってしか成立しないことを構図で保証するための誤寸である。読む照会を
//!    `text_slot_view(...).surface_size()`（native 原寸）にしてはならない——k≠1.0 では窓が原寸へ
//!    引き戻される。donor `emo-present.rs` の窓は `Anchored` 欠落で `resize_window_to` が不発なので、
//!    窓生成は `spawn_ghost_windows`（`Anchored` 無条件付与・`MonitorSnapshot` 挿入）を用いる。
//!
//! # 使い方
//!
//! ```text
//! cargo run -p areka-runtime --example collision-probe
//! ```
//!
//! 終了は `AREKA_APP_SMOKE_EXIT_MS`（有界 auto-exit）だけ。目視プロトコル ⑤ の所要時間を見込んで
//! 寛大な値（例 `180000`＝3 分）を必ず与えること。
//!
//! ダブルクリックでは終了しない。この probe が装着するポインタハンドラは `OnPointerMoved` だけで、
//! `OnPointerPressed` は付いていない（キャラ窓のポインタハンドラは本体の
//! `areka_input::attach_char_pointer_handlers` が装着するが、`pub(crate)` で内部が `crate::` パスを使うため
//! runtime の example からは使えない）。本番で利用者が終了を指示する道は右クリックメニューの「終了」
//! （`menu::request_close`）だけで、結線済みの Ctrl+左ダブルクリックはふつうの左ダブルクリックとして配る。
//!
//! # 起動時のパス（絶対パスの要否）
//!
//! 実機サインオフの emo2 ゴーストルートと `pasta.dll` は絶対パスで与える（相対だと `pasta.dll` の LOAD が
//! `0x8007007E` で失敗する）が、この probe にその制約はかからない。
//!
//! 1. fixture ルートは常に絶対解決される: `fixture::emo2_root` は `sample_ghost_kit` の
//!    `SampleRoot::acquire("emo2")` が用意した検体の絶対パスを返し、作業ディレクトリに依存しない。
//! 2. `pasta.dll` を一切ロードしない（SHIORI も host-32 helper も起動しない）ので、`0x8007007E` の経路が
//!    存在しない。読むのはデータファイルだけで、ghost・shell・バルーンの `descript.txt`、shell の
//!    `surfaces.txt` と面画像、バルーンパッケージの面定義・面画像、`windowposition` 定義である
//!    （`areka-placement` の `prepare_stages_for_shell` が全体像）。
//!
//! ゆえに中断ログ「collision-probe: 配置準備に失敗」（`build_and_spawn` の early return）は
//! `PlacementError::Mount`／`DescriptRead`／`Measure` 経由であり、shell だけでなく ghost/balloon の
//! `descript.txt` やバルーン資産も疑うこと。
//!
//! 「本番ゴースト（実 emo2・実 `pasta.dll`）を絶対パスで起動して行う実機確認」は本 probe ではなく本番の
//! `areka` 本体が担い、本 probe が担うのは実 DPI での当たり判定と目視の一致である。受け入れ記録では
//! 両者の実施条件を分けて記録し、本 probe の実行を本番ゴースト起動の証跡として流用しないこと。
//!
//! # プロトコル（実 DPI≠96 で実施し記録する）
//!
//! ① **環境**: per-monitor v2・実 DPI ≠ 96（125%/150%/200% ＝ dpi 120/144/192 のうち 2 水準以上）。
//!    DPI awareness は wintf `WinApp` 初期化が設定する。dpi=96 のみの確認は不合格（単位混在バグが
//!    自己整合して隠れるため）。水準ごとに `AREKA_COLLISION_PROBE_EXPECT_K` を与えると、その水準で本当に
//!    その k が適用されたことを probe 自身が hard assert する。
//!
//! ② **表示**: 本番 emo2 の `surface1000` を有効 bind 実値付きで shell target（scope=0 →
//!    `shell_target(0)`）へ実表示する。窓生成は `window-placement.rs` donor（`spawn_ghost_windows`＋
//!    `MonitorSnapshot` 挿入）、emo 起動系は `emo-present.rs` donor（fixture ロード→`EmoWorld` 構築→
//!    `attach_target`→`apply(ShowSurface)`）を合成し、上の必須逸脱 3 点は必ず逸脱させる。
//!
//! ③ **物理寸整合の assert（自動）**: 本番 resize 適用の次フレーム以降に assert する（`SetWindowPosCommand` は
//!    発行 tick の World 借用解放後に flush されるため、同 tick 内の `GetClientRect` は旧寸を返す）。
//!    Win32 `GetClientRect(hwnd)` の client 矩形寸法が [`EmoPresenter::target_physical_size`]
//!    （丸め単一権威 `ScaleRatio::scaled_extent` を通した値）と一致すること。assert は必ず実窓
//!    （`GetClientRect`）に対して行う——`WindowPos.size` は enqueue 時点で bypass ミラー済みなので、読むと
//!    SetWindowPos が一度も走らなくても緑になる。捕捉する欠陥は、本番 resize 経路への dpi/96 再スケール・
//!    論理 px 解釈の混入・native 原寸と物理寸の取り違え。
//!
//! ④ **描画一致の anchor（自動・マウス経路非依存）**: [`EmoPresenter::read_back`] で Head（`93,62`–`271,130`）／
//!    Bust（`133,270`–`229,326`）各矩形の中心の画素が不透明であることを assert する。read_back は native
//!    原寸の合成バイト列を返す（拡大は GPU 変換の領分）ので、当たり判定値をそのまま画素 index へ写せる。
//!    読み戻し長が k に依らず `native_w × native_h × 4` であることも assert し、anchor が矩形内側
//!    2px 以上にあることも確かめる（退化矩形で縁がたまたま不透明になるのを防ぐ）。これは「当たり判定値の
//!    位置に絵が描かれている」ことしか語らない描画証跡で、判定の証跡は ⑤ だけである。記録様式でも混ぜない。
//!
//! ⑤ **解決一致（目視が証拠力の中核）**: 点は実表示窓の client 座標経路からのみ取得する——`GetCursorPos`
//!    （screen）→ `ScreenToClient` → 得た client 点を `resolve_hit_region(presenter, 0, x, y)` へ渡して
//!    live にログする。操作者は画面のゴーストの頭／胸／背景を目視で狙ってカーソルを動かし、視覚上の部位と
//!    解決結果（`"Head"`／`"Bust"`／`None`）の一致を記録する。
//!    - **反トートロジー**: 当たり判定値から合成した screen 座標への `SetCursorPos`/`SendInput` を証跡と
//!      してはならない。`ClientToScreen` と `ScreenToClient` は client 原点の平行移動の厳密な逆写像なので、
//!      注入した点をそのまま読み戻して純関数へ渡す自己整合になる。狙点の供給源は目視（または ④ の
//!      read_back から導いた描画由来の点）に限り、本 probe は `SetCursorPos`/`SendInput` を一切呼ばない。
//!    - **マウス経路照合**: probe 窓へ `OnPointerMoved` ハンドラを 1 行装着し、記録行ごとに本番マウス経路の
//!      `PointerState.client_point` と `ScreenToClient(GetCursorPos())` をペア列（client_x/y・s2c_x/y・
//!      Δx/Δy）で併記する。要求は Δ=(0,0) の厳密一致（両者とも同一 HWND の client 物理 px 整数で、
//!      丸め誤差の正当源が無い）。ペア列は Head/Bust（不透明画素）行のみ——背景 None 行はクリック透過
//!      （`WS_EX_TRANSPARENT`＋`HTTRANSPARENT`）でイベントが窓へ届かず、ペア欠測が正しい挙動である。
//!      クエリ系 API 経路と WM_MOUSEMOVE lparam 経路が OS 内部でのみ合流する、非トートロジーの検査。
//!
//! ⑥ **判定**: 全項目 PASS かつ dpi≠96 を 2 水準で充足したことを acceptance-record の判定行に明記する。
//!
//! # 証明範囲
//!
//! 証明する: (a) 表示側の k 整合（窓 client 物理寸 ≡ `target_physical_size`）と、それを本番で維持する
//! 反映経路（`resize_window_to`→SetWindowPos→`GetClientRect`）の単位保存性（実 DPI 2 水準・数値 assert）、
//! (b) マウス経路の空間一致（`client_point` ≡ `ScreenToClient`・静止記録行）。
//! 証明しない: `frame/` の resnap 検知ループ、窓生成経路の実 DPI 数値正しさ、イベント配送から SHIORI
//! 一周までの撫でクラスタ合流サインオフ。

use std::path::{Path, PathBuf};

use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetCursorPos};
use windows::core::Result;

use wintf::ecs::pointer::{OnPointerMoved, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::{FrameFinalize, GraphicsCore, WindowHandle, WucGraphicsResource};
use wintf::*;

use areka_emo_atlas::{AtlasTable, WicDecoderArm};
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_emo_present::{EmoPresenter, PresentCommand, ScaleRatio, TargetId, load_shell_target};

use areka_placement::spawn::{
    GhostWindowMarker, register_ghost_windows_click_through, spawn_ghost_windows,
};
use areka_runtime::hit_region::resolve_hit_region;
use areka_runtime::target_map;

// `mod foo;` は `examples/foo.rs` を探して新しい example ターゲットを生むので、子の接続は
// `#[path]` が必須。子ディレクトリに `main.rs` は置かない（Cargo が重複ターゲット名で落ちる）。

#[path = "collision-probe/fixture.rs"]
mod fixture;
#[path = "collision-probe/pointer.rs"]
mod pointer;
#[path = "collision-probe/probe.rs"]
mod probe;
#[path = "collision-probe/ratio.rs"]
mod ratio;
#[path = "collision-probe/setup.rs"]
mod setup;
#[path = "collision-probe/smoke.rs"]
mod smoke;
#[path = "collision-probe/state.rs"]
mod state;

use self::fixture::{EXPECT_K_ENV, SMOKE_EXIT_ENV};
use self::probe::boot_probe_system;
use self::setup::run_setup;
use self::smoke::install_smoke_exit;

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // UI スレッド（MTA・COM 初期化済み＝WIC の前提）で配置準備・窓生成・アセット構築・presenter 生成を行う。
    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    // 透明域のイベントは背後へ透過し（⑤ の None 行にペア列が出ない根拠）、不透明域のみ窓へ届く。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_ghost_windows_click_through);

    world
        .borrow_mut()
        .add_systems(FrameFinalize, boot_probe_system);

    install_smoke_exit(&mgr);

    println!();
    println!("areka collision-geometry 実 DPI probe");
    println!("=======================================");
    println!("  表示  : emo2 surface1000（有効 bind 実値付き・scope0 キャラ窓）");
    println!(
        "  自動  : ③ 物理寸整合 assert（GetClientRect == target_physical_size）＋④ read_back 描画一致 anchor（native 座標で直読み）"
    );
    println!(
        "  手動  : ⑤ ゴーストの頭/胸/背景を目視で狙い、resolve 結果（Head/Bust/None）とペア列 Δ=(0,0) を記録"
    );
    println!(
        "  期待k : env {EXPECT_K_ENV}（例 5/4・2）指定時のみ実適用 k を hard assert（未指定なら実測ログのみ）"
    );
    println!(
        "  ログ  : `collision-probe: k=` / `collision-probe: client=` を grep して採取（1 実行 = 1 DPI 水準）"
    );
    println!("  受け入れ: rustdoc プロトコル ①〜⑥（dpi≠96 を2水準必達・96 のみは不合格）");
    println!(
        "  終了  : {SMOKE_EXIT_ENV} の有界 auto-exit のみ（ダブルクリックでは終了しない＝OnPointerPressed 未装着・本番の終わり方はメニューの「終了」）"
    );
    println!();

    mgr.run()?;

    Ok(())
}
