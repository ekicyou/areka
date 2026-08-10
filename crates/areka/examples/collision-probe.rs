//! areka collision-geometry 実 DPI 受け入れ probe（初出＝`areka-P0-collision-geometry` task 4.1・
//! 要件 4.3/7.3／`areka-P0-collision-dpi-hittest` 要件 4.1-4.6 で **k 対応化**）
//!
//! リゾルバ [`resolve_hit_region`] の座標契約（**窓 client 物理 px を presenter が ÷k してサーフェス px
//! へ縮約する**）を、**本番 emo2 表示**と**実 DPI（≠96）**で実測し証跡化する probe。
//!
//! # 1 実行 = 1 DPI 水準（DPI 追従駆動は持たない）
//!
//! 実適用 k は**実行機の表示スケールがそのまま決める**——wintf は窓生成時に実モニタ DPI で `DPI`
//! component を初期化するため、初回表示の時点で実 k が確定する。ゆえに本 probe は DPI 追従駆動
//! （`run_dpi_phase` 相当）を**持たない**。要件 4.1 が要求する 2 水準は、OS の表示スケールを
//! 変更して（例 125% → 200%）**2 回実行**し、各実行の `physical=` が互いに異なることを記録上で
//! 照合して満たす。
//!
//! # 期待 k ゲート（env `AREKA_COLLISION_PROBE_EXPECT_K`）
//!
//! 水準ごとの期待 k を `"5/4"`（分数）または `"2"`（整数）で与えると、実適用 k
//! （[`EmoPresenter::applied_ratio`]）との厳密一致を hard assert する（環境設定ミスの即検出）。
//! **未指定時は assert せず実測ログのみ**——開発機（k=1.0 でも任意 k でも）でそのまま実行できる。
//!
//! # 常設 greppable ログ（要件 4.1/4.5）
//!
//! ```text
//! collision-probe: k=<num>/<den> native=<w>x<h> physical=<w>x<h>
//! collision-probe: client=(x,y) surface=(sx,sy) region=<name>
//! ```
//!
//! 前者は 1 実行に 1 回（表示成立時）、後者はカーソル解決ごとに出る。`surface=` は presenter が
//! 縮約した `HitRegion::surface_point` の値そのものであり、probe 側で ÷k を再実装しない。
//!
//! 純粋層の決定論檻（task 1）とは
//! 別の観測（steering `tech.md:61-63`「examples は手動検証とグラフィックス挙動確認の補助であり、
//! テストの代替ではない」）であり、両者は併存する（観測の2段構え）。
//!
//! # `#[path]` 私有 include（read-only 再利用・production 不改変）
//!
//! `areka` は bin crate（lib target なし）ゆえ `use areka::...` は不可能。本 probe は次の3モジュールを
//! `#[path]` で私有 include する（`window-placement.rs:107-113` と同型）:
//!
//! - `../src/emo2_boot/target_map.rs`（scope→`TargetId` 写像の正本・既に `crate::` フリー）
//! - `../src/emo2_boot/hit_region.rs`（[`resolve_hit_region`]／`HitRegion` 契約・非テストコードは
//!   `super::target_map` と外部 crate のみ参照＝`crate::` フリー）
//! - `../src/placement/mod.rs`（`spawn_ghost_windows`／`resize_window_to` の read-only 再利用）
//!
//! **`target_map` と `hit_region` は example クレートルートに宣言する**（入れ子にしない）。これにより
//! `hit_region.rs` 内部の `super::target_map` が本 probe の `target_map` へ解決される（design「配置の
//! 決定的制約」）。`#[allow(dead_code)]` は include 側指定（main.rs 専用の公開面を未使用警告にしない）。
//!
//! # donor からの必須逸脱（3点・これを欠く probe は構造的に無効・design Open Risk 6）
//!
//! 1. **表示対象＝`surface1000`**: emo2 で collision を持つのは `surface1000` のみ。`surface0`
//!    （`emo-present.rs`/`window-placement.rs` の既定）は collision を1つも持たず空集合しか観測できない。
//! 2. **`BindSet` は有効 bind の実値集合** `[1101, 1206, 1302, 1502, 1800]`（`emo-present.rs:170` 相当）:
//!    `surface1000` は静的 element を持たず全パーツが bind 制御ゆえ、`BindSet::default()` では全透明の窓が
//!    「成功」として出て人が狙える絵が無い（失敗としても観測されない）。
//! 3. **窓 client 寸は本番の窓寸規則で与える**: `spawn_ghost_windows` を**意図的に誤った placeholder 寸**
//!    （100×100）で呼んで窓を作り、`apply(ShowSurface{1000, binds})` 成立後に
//!    [`EmoPresenter::target_physical_size`]（**k 適用後の物理寸権威**）を読み、**本番の反映関数**
//!    [`placement::follow::resize_window_to`] で窓へ適用する（戻り値 `true` を assert）。読む照会を
//!    `text_slot_view(...).surface_size()`（native 原寸）にしてはならない——k≠1.0 では窓が原寸へ
//!    引き戻される（照会 1 トークンの取り違えが静かな欠陥になる）。placeholder を
//!    誤寸にするのは「最終一致が本番 resize 経路を通ってしか成立しない」ことを構図で保証するため。
//!    donor `emo-present.rs` の窓は `Anchored` 欠落で `resize_window_to` が不発（`follow.rs:554`）ゆえ、
//!    窓生成は `spawn_ghost_windows`（`Anchored` 無条件付与・`MonitorSnapshot` 挿入）を用いる。
//!
//! # 使い方
//! ```text
//! cargo run -p areka --example collision-probe
//! ```
//! **終了は `AREKA_APP_SMOKE_EXIT_MS`（有界 auto-exit）のみ**。目視プロトコル ⑤ の所要時間を見込んで
//! 寛大な値（例 `180000`＝3 分）を必ず与えること。
//!
//! **ダブルクリックでは終了しない**（2026-08-05 実機で確認）。本 probe が装着するポインタ
//! ハンドラは [`on_probe_pointer_moved`]（`OnPointerMoved`）**だけ**であり、`OnPointerPressed` は
//! 一切付いていない——`spawn_ghost_windows` が付けていた stand-in `on_ghost_pressed` は
//! `areka-P0-input-events` task 2.7 で**退役**し（`placement/spawn.rs` の当該コメント参照）、
//! 正典の脱出口（**Ctrl+左ダブルクリック**・DD-IE-7）は `input_events::attach_char_pointer_handlers`
//! （`pub(crate)`・内部が `crate::` パスを使うため example から `#[path]` include できない）が
//! 担うようになったためである。素のダブルクリックは本来 `KanadeMsg::Mouse(DoubleClick)` の送出契機で
//! あって終了契機ではない。probe への脱出口結線は本 spec の射程外（`input_events` の
//! `#[path]` include 可能化が要る）。
//!
//! # 起動時のパス（絶対パスの要否・要件 4.6）
//!
//! 実機サインオフの起動手順は [[areka-emo2-signoff-needs-absolute-paths]]（emo2 ゴーストルートと
//! `pasta.dll` を**絶対パスで**与える。相対パスだと `pasta.dll` の LOAD が `0x8007007E` で失敗する）に
//! 従う。ただし本 probe に**その制約はかからない**——理由は次の 2 点で、混同しないこと:
//!
//! 1. **本 probe の fixture ルートは常に絶対解決される**: [`emo2_root`] のアンカーはコンパイル時に
//!    埋め込まれる `CARGO_MANIFEST_DIR`（＝`crates/areka` の絶対パス）であり、続く `../pilot/...` は
//!    そのアンカーからの相対要素にすぎない。**プロセスの作業ディレクトリに依存しない**ため、どこから
//!    `cargo run` しても同一の emo2 fixture を読む（起動側で絶対パスを与える必要がない）。
//! 2. **本 probe は `pasta.dll` を一切ロードしない**: SHIORI（`pasta.dll`）も host-32 helper も起動しない
//!    ため、`0x8007007E`（DLL LOAD 失敗）の経路自体が存在しない。**読むものは少なくない**が、いずれも
//!    データファイルの読取であって DLL のロードではない——実際の読取先は次のとおり:
//!    - **ghost の `descript.txt`**（`placement/source.rs:142-143`・寛容読取）・**shell の
//!      `descript.txt`**（`source.rs:146-156`・読取失敗は致命）・**バルーンの `descript.txt`**
//!      （`source.rs:197-201`・作者基準 DPI の取得）——起点が `descript.txt` であることは
//!      [[areka-ghost-boot-descript-not-install]] の正典どおり
//!    - **shell の `surfaces.txt` と面画像**（採寸側 `placement/measure.rs:328-345` ＋ probe 自身の
//!      [`build_shell_target`]）
//!    - **バルーンパッケージの面定義・面画像**（`measure.rs:221` の `measure_balloon_surface0`）と
//!      **`windowposition` 定義**（`placement/mod.rs:341` の `apply_scope_windowpositions`）
//!
//!    ゆえに中断ログ「collision-probe: 配置準備に失敗」（`build_and_spawn` の early return）は
//!    `PlacementError::Mount`／`DescriptRead`／`Measure` 経由であり、**shell だけでなく ghost/balloon の
//!    `descript.txt` やバルーン資産も疑う**こと（`prepare_stages`＝`placement/mod.rs:307-348` が読取の全体像）。
//!
//! # 作者基準 DPI の出所（k 論証への影響・実測済み）
//!
//! 採寸経路（`prepare_stages`・`placement/mod.rs:319-322`）は **descript から読んだ実 author_dpi**
//! （shell=`seriko.dpi`／balloon=`dpi`）で起動時 k₀ を作るのに対し、本 probe は `attach_target` へ
//! **96 を直書き**しており `PreparedPlacement::author_dpi` を消費しない。emo2 fixture では両者が
//! **たまたま一致する**——fixture の shell descript に `seriko.dpi` 宣言が無く既定 96 になるためで、
//! これは in-repo テスト `placement/source.rs:534-538`
//! （`shell_author_dpi_emo2_fixture_is_default_96`）と実行ログ（`primary_dpi=192 shell_author_dpi=96
//! balloon_author_dpi=96` ／ `apply(ShowSurface): ... author_dpi=96 window_dpi=Some((192,192))`）の
//! 双方で確認できる。**`seriko.dpi` を宣言する別ゴーストへ差し替えると窓寸（採寸 k₀）と表示 k が
//! 食い違う**ため、fixture を変えるときは probe の直書き 96 を `prepared.author_dpi.shell` へ
//! 差し替えること。
//!
//! R4.6 が要求する「**本番ゴースト（実 emo2・実 `pasta.dll`）を絶対パスで起動**して行う実機確認」の脚は
//! **本 probe ではなく本番 emo2 実ゴースト実行（`areka` 本体・Task 8.2）が担う**。本 probe が担うのは
//! **ヒットテスト目視の脚**（実 DPI での当たり判定と目視の一致）であり、両者は同じ受け入れ記録の別項目で
//! ある。acceptance-record では両脚の実施条件を分けて記録し、本 probe の実行を「本番ゴーストを絶対パスで
//! 起動した証跡」として流用しないこと。
//!
//! # プロトコル（acceptance-record と1:1 対応・task 4.2 が実 DPI≠96 で実施し記録する）
//!
//! ① **環境**: per-monitor v2・実 DPI ≠ 96（125%/150%/200% ＝ dpi 120/144/192 のいずれか2水準以上）。
//!    DPI awareness は wintf `WinApp` 初期化がプロセスに設定する。**dpi=96 のみの確認は不合格**
//!    （`window-placement.rs` 前例に倣う・dpi=96 では単位混在バグが自己整合して隠れるため）。
//!    **1 実行 = 1 水準**ゆえ、OS 表示スケールを変更して 2 回実行し、各実行の `physical=` が互いに
//!    異なることを記録上で照合する（要件 4.1）。水準ごとに `AREKA_COLLISION_PROBE_EXPECT_K` を与えると
//!    「その水準で本当にその k が適用された」ことを probe 自身が hard assert する。
//!
//! ② **表示**: 本番 emo2 の `surface1000` を有効 bind 実値付きで shell target（scope=0 →
//!    `shell_target(0)`）へ実表示する。窓生成は `window-placement.rs` donor（`spawn_ghost_windows`＋
//!    `MonitorSnapshot` 挿入）、emo 起動系は `emo-present.rs` donor（fixture ロード→`EmoWorld` 構築→
//!    `attach_target`→`apply(ShowSurface)`）を合成する。上記「donor からの必須逸脱」3点は必ず逸脱させる。
//!
//! ③ **物理寸整合の assert（自動・hard assert）**: 本番 resize 適用の**次フレーム以降**に assert する
//!    （`SetWindowPosCommand` は発行 tick の World 借用解放後に flush される＝`tick_bridge.rs:199-200`。
//!    同 tick 内の `GetClientRect` は旧寸を返すため、最低1フレーム進めてから assert する）。内容: Win32
//!    `GetClientRect(hwnd)` の client 矩形寸法が [`EmoPresenter::target_physical_size`]（**k 適用後の
//!    物理寸権威**＝丸め単一権威 `ScaleRatio::scaled_extent` を通した値）と**一致**すること。加えて env
//!    `AREKA_COLLISION_PROBE_EXPECT_K` 指定時のみ、実適用 k（[`EmoPresenter::applied_ratio`]）が期待値と
//!    厳密一致することを assert する（未指定時は assert 無し）。**assert は必ず実窓（`GetClientRect`）に
//!    対して行う**——`WindowPos.size` は enqueue 時点で bypass ミラー済み（`follow.rs:760-767`）のため、
//!    `WindowPos` を読むと SetWindowPos が一度も走らなくても緑になる偽檻となる。捕捉する欠陥クラス:
//!    本番 resize 経路への dpi/96 再スケール・論理 px 解釈の混入・**native 原寸と物理寸の取り違え**
//!    （窓が原寸へ引き戻される静かな欠陥）。
//!
//! ④ **描画一致の anchor（自動・マウス経路非依存・描画証跡であって判定証跡ではない）**:
//!    [`EmoPresenter::read_back`] で Head（`93,62`–`271,130`）／Bust（`133,270`–`229,326`）各矩形の中心を
//!    **物理座標へ写像**（`ScaleRatio::scale_len` で ×k）した画素が**不透明**（実際に絵が描かれている）
//!    ことを assert する。read_back は k 適用後の供給面（`chain.size()` ＝ 物理寸）ゆえ写像が要る。
//!    anchor は写像後も矩形内側 **≥2px** にあることを併せて assert し、丸め差 1px と無関係に成立させる。
//!    **この検査は「collision 値の位置に絵が描かれている」ことしか語らない描画証跡であり、当たり判定の
//!    証跡ではない**（要件 4.4）——判定の証跡は ⑤ の目視由来経路のみである。記録様式でも両者を混ぜない。
//!
//! ⑤ **解決一致（7.3(a)・目視が証拠力の中核）**: 点は**実表示窓の client 座標経路からのみ取得する**——
//!    `GetCursorPos`（実カーソル位置・screen）→ `ScreenToClient` → 得られた client 点を
//!    `resolve_hit_region(presenter, 0, x, y)` へ渡し、結果を live にログする。操作者（人間または agent）は
//!    **画面に見えているゴーストの頭／胸／背景を目視で狙って**カーソルを動かし、「視覚上その部位に
//!    載っていること」と解決結果（`"Head"`／`"Bust"`／`None`）の一致を記録する。
//!    - **反トートロジー禁止条件（7.3(a)）**: **collision 実値から合成した screen 座標への
//!      `SetCursorPos`/`SendInput` を証跡としてはならない**。`ClientToScreen` と `ScreenToClient` は
//!      client 原点の平行移動の**厳密な逆写像**であるため、collision 由来の点を狙って撃ち戻すと「注入した
//!      点をそのまま読み戻して純関数へ渡す」＝要件が「証跡と認めない」と名指しした自己整合の罠に落ちる。
//!      狙点の供給源は**目視**（または ④ の read_back から導出した描画由来の点）に限る。本 probe は
//!      `SetCursorPos`/`SendInput` を一切呼ばない。
//!    - **マウス経路照合（設計討議#3）**: probe 窓へ `OnPointerMoved` ハンドラを1行装着し
//!      （`OnPointerPressed(on_shell_pressed)` の donor 前例＝`emo-present.rs:523`）、記録行ごとに**本番
//!      マウス経路**の `PointerState.client_point` と `ScreenToClient(GetCursorPos())` を**ペア列（client_x/y・
//!      s2c_x/y・Δx/Δy）**で実測表へ併記する。記録行の要求は **Δ=(0,0) の厳密一致**（両者とも同一 HWND の
//!      client 物理 px 整数＝丸め誤差の正当源が無い）。**ペア列は Head/Bust（不透明画素）行のみ**——背景
//!      None 行はクリック透過（`WS_EX_TRANSPARENT`＋`HTTRANSPARENT`）によりイベント自体が窓へ届かず、
//!      ペア欠測が正しい挙動である（脚注★・C-3 整合）。この照合はクエリ系 API 経路と WM_MOUSEMOVE lparam
//!      経路が OS 内部でのみ合流する非トートロジー検査である。
//!
//! ⑥ **判定**: 全項目 PASS かつ dpi≠96 を2水準で充足したことを acceptance-record の判定行に明記する。
//!
//! > ★脚注: ⑤ のペア列は不透明域（Head/Bust）行のみに現れる。背景（解決 `None`）域はクリック透過で
//! > `OnPointerMoved` が窓へ届かないため、その行にペア列が無いのは**正しい挙動**（欠測＝FAIL ではない）。
//!
//! # probe の証明範囲（設計討議#2/#3）
//!
//! **証明する**: (a) 表示側の k 整合契約（窓 client 物理寸 ≡ `target_physical_size`）と、それを本番で
//! 維持する反映経路（`resize_window_to`→SetWindowPos→`GetClientRect`）の単位保存性
//! （実 DPI 2水準・数値 assert）、(b) マウス経路の**空間**一致
//! （`client_point` ≡ `ScreenToClient`・静止記録行）。**証明しない**: (1) `frame.rs` の resnap 検知ループ
//! （completed `areka-P0-surface-resize-resnap` が所有）、(2) 窓**生成**経路の実 DPI 数値正しさ
//! （completed `areka-P0-window-placement` が所有）、(3) イベント配送〜SHIORI 一周＝撫でクラスタ合流
//! サインオフ（7.4）が所有。

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

use areka_emo_atlas::{
    AlphaParams, AtlasTable, PackConfig, SetId, SurfaceSet, UseSelfAlpha, WicDecoderArm, bake,
};
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_emo_present::{EmoPresenter, PresentCommand, ScaleRatio, TargetId};

/// scope→`TargetId` 写像の正本を私有 include する（`super::target_map` 解決の要ゆえクレートルート宣言）。
#[path = "../src/emo2_boot/target_map.rs"]
#[allow(dead_code)]
mod target_map;

/// [`resolve_hit_region`]／`HitRegion` 契約を私有 include する。内部の `super::target_map` は上の
/// `target_map` へ解決される（クレートルート宣言・入れ子にしない）。
#[path = "../src/emo2_boot/hit_region.rs"]
#[allow(dead_code)]
mod hit_region;

/// 窓配置機構本体（`spawn_ghost_windows`／`resize_window_to`）を read-only 再利用（production 無改変）。
/// placement 非テストコードは `super::`／外部 crate のみを参照する（`crate::` は `#[cfg(test)]` 内のみ）ため
/// `#[path]` include で成立する（`window-placement.rs:107-113` と同型）。
#[path = "../src/placement/mod.rs"]
#[allow(dead_code)]
mod placement;

use hit_region::resolve_hit_region;
use placement::spawn::{GhostWindowMarker, register_ghost_windows_click_through, spawn_ghost_windows};

// ---------------------------------------------------------------------------
// 本体分割の接続（`areka-P0-file-slimming` task 8.12・純移動）
//
// **上の 3 つの `#[path = "../src/..."]` 私有 include はこのファイルに残す。** 相対パスは宣言元
// ファイルのディレクトリ（`crates/areka/examples/`）から解決されるため、サブディレクトリの子へ
// 移すと `../src/...` が届かなくなる（本タスクの最重要制約）。
//
// サンプルのルートファイルは**自分のバイナリターゲットのクレートルート**であり、素の `mod foo;`
// は `examples/foo.rs` を探して**新しい example ターゲットを生む**。ゆえに子の接続も `#[path]`
// が必須である。子ディレクトリに `main.rs` は置かない（置くと Cargo が重複ターゲット名で落ちる）。
// ---------------------------------------------------------------------------

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
    // tracing-subscriber 初期化（RUST_LOG 対応・既定 info・不正構文は info へフォールバック）。
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // 「配置準備＋窓生成＋アセット構築＋presenter 生成」を UI スレッド（MTA・COM 初期化済み＝WIC 前提を
    // 満たす）へ投函する（donor と同型）。
    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    // clickthrough 登録 system（placement 本体の一般化 system・donor と同じ FrameFinalize 結線）。
    // 透明域のイベントは背後へ透過し（⑤ の None 行にペア列が出ない根拠）、不透明域のみ窓へ届く。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_ghost_windows_click_through);

    // GPU 資源到達フレームで表示→本番 resize→物理寸整合 assert を駆動する probe 起動 system。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, boot_probe_system);

    // env ゲート付き smoke 自動 close（hands-off のビルド/起動確認用・main.rs と同名 env）。
    install_smoke_exit(&mgr);

    // 操作ガイド出力。
    println!();
    println!("areka collision-geometry 実 DPI probe");
    println!("=======================================");
    println!("  表示  : emo2 surface1000（有効 bind 実値付き・scope0 キャラ窓）");
    println!(
        "  自動  : ③ 物理寸整合 assert（GetClientRect == target_physical_size）＋④ read_back 描画一致 anchor（物理座標へ写像）"
    );
    println!("  手動  : ⑤ ゴーストの頭/胸/背景を目視で狙い、resolve 結果（Head/Bust/None）とペア列 Δ=(0,0) を記録");
    println!(
        "  期待k : env {EXPECT_K_ENV}（例 5/4・2）指定時のみ実適用 k を hard assert（未指定なら実測ログのみ）"
    );
    println!("  ログ  : `collision-probe: k=` / `collision-probe: client=` を grep して採取（1 実行 = 1 DPI 水準）");
    println!("  受け入れ: rustdoc プロトコル ①〜⑥（dpi≠96 を2水準必達・96 のみは不合格）");
    println!(
        "  終了  : {SMOKE_EXIT_ENV} の有界 auto-exit のみ（ダブルクリックでは終了しない＝OnPointerPressed 未装着）"
    );
    println!();

    mgr.run()?;

    Ok(())
}
