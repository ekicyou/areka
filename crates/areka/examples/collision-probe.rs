//! areka collision-geometry 実 DPI 受け入れ probe（areka-P0-collision-geometry task 4.1・要件 4.3/7.3）
//!
//! リゾルバ [`resolve_hit_region`] の座標契約（窓 client 物理 px ＝ サーフェス px・k=1.0）を、
//! **本番 emo2 表示**と**実 DPI（≠96）**で実測し証跡化する probe。純粋層の決定論檻（task 1）とは
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
//!    `text_slot_view(shell_target(0)).surface_size()`（現表示実寸）を読み、**本番の反映関数**
//!    [`placement::follow::resize_window_to`] で窓へ適用する（戻り値 `true` を assert）。placeholder を
//!    誤寸にするのは「最終一致が本番 resize 経路を通ってしか成立しない」ことを構図で保証するため。
//!    donor `emo-present.rs` の窓は `Anchored` 欠落で `resize_window_to` が不発（`follow.rs:554`）ゆえ、
//!    窓生成は `spawn_ghost_windows`（`Anchored` 無条件付与・`MonitorSnapshot` 挿入）を用いる。
//!
//! # 使い方
//! ```text
//! cargo run -p areka --example collision-probe
//! ```
//! キャラ窓の不透明域をダブルクリックすると全ゴースト窓を閉じて終了する（`AREKA_APP_SMOKE_EXIT_MS`
//! でも自動終了できる）。
//!
//! # プロトコル（acceptance-record と1:1 対応・task 4.2 が実 DPI≠96 で実施し記録する）
//!
//! ① **環境**: per-monitor v2・実 DPI ≠ 96（125%/150%/200% ＝ dpi 120/144/192 のいずれか2水準以上）。
//!    DPI awareness は wintf `WinApp` 初期化がプロセスに設定する。**dpi=96 のみの確認は不合格**
//!    （`window-placement.rs` 前例に倣う・dpi=96 では単位混在バグが自己整合して隠れるため）。
//!
//! ② **表示**: 本番 emo2 の `surface1000` を有効 bind 実値付きで shell target（scope=0 →
//!    `shell_target(0)`）へ実表示する。窓生成は `window-placement.rs` donor（`spawn_ghost_windows`＋
//!    `MonitorSnapshot` 挿入）、emo 起動系は `emo-present.rs` donor（fixture ロード→`EmoWorld` 構築→
//!    `attach_target`→`apply(ShowSurface)`）を合成する。上記「donor からの必須逸脱」3点は必ず逸脱させる。
//!
//! ③ **k=1.0 の assert（7.3(b)・自動・hard assert）**: 本番 resize 適用の**次フレーム以降**に assert する
//!    （`SetWindowPosCommand` は発行 tick の World 借用解放後に flush される＝`tick_bridge.rs:199-200`。
//!    同 tick 内の `GetClientRect` は旧寸を返すため、最低1フレーム進めてから assert する）。内容: Win32
//!    `GetClientRect(hwnd)` の client 矩形寸法が `text_slot_view(shell_target(0)).surface_size()`（現表示
//!    サーフェスの物理 px 原寸）と**一致**し、かつ `scale() == 1.0` であること。**assert は必ず実窓
//!    （`GetClientRect`）に対して行う**——`WindowPos.size` は enqueue 時点で bypass ミラー済み
//!    （`follow.rs:760-767`）のため、`WindowPos` を読むと SetWindowPos が一度も走らなくても緑になる偽檻
//!    となる。捕捉する欠陥クラス: 本番 resize 経路への dpi/96 再スケール・論理 px 解釈の混入
//!    （window-placement v1 を廃案にした実在欠陥クラス）。
//!
//! ④ **描画一致の anchor（自動・マウス経路非依存）**: [`EmoPresenter::read_back`] で Head
//!    （`93,62`–`271,130`）／Bust（`133,270`–`229,326`）各矩形の中心画素が**不透明**（実際に絵が
//!    描かれている）ことを assert する。**collision 値と実描画画素の対応**を機械的に固定する検査であり、
//!    マウスを介さないため ⑤ のトートロジー問題と独立に成立する。
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
//! **証明する**: (a) 表示側恒等契約と、それを本番で維持する反映経路（`resize_window_to`→SetWindowPos→
//! `GetClientRect`）の単位保存性（実 DPI 2水準・数値 assert）、(b) マウス経路の**空間**一致
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
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};

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
// Constants / fixture paths
// ---------------------------------------------------------------------------

/// 意図的に誤った placeholder 窓寸（donor 必須逸脱 #3）。surface1000 の実合成寸とは必ず異なる値を選び、
/// 最終一致が本番 resize 経路（`resize_window_to`）を通ってしか成立しないことを構図で保証する。
const PLACEHOLDER_SIZE: i32 = 100;

/// emo2 `surface1000` の Head 当たり判定矩形（`surfaces.txt`: `collision0,93,62,271,130,Head`・サーフェス px）。
const HEAD_RECT: (i64, i64, i64, i64) = (93, 62, 271, 130);
/// emo2 `surface1000` の Bust 当たり判定矩形（`surfaces.txt`: `collision1,133,270,229,326,Bust`・サーフェス px）。
const BUST_RECT: (i64, i64, i64, i64) = (133, 270, 229, 326);

/// 有効 bind 実値集合（donor 必須逸脱 #2・`emo-present.rs:170` 相当）。腕組み=1101・口‥‥=1206・
/// 目通常=1302・眉悲しみ=1502・髪飾りリボン=1800。`surface1000` は全パーツ bind 制御ゆえ既定 bind では
/// 全透明になる（狙える絵が出ない）。
const REAL_BIND_IDS: [u32; 5] = [1101, 1206, 1302, 1502, 1800];

/// smoke 自動 close の env ゲート名（main.rs／他 example と同名・`AREKA_` 冠規約）。
const SMOKE_EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";

/// emo2 fixture のゴーストルート（`CARGO_MANIFEST_DIR`＝`crates/areka` 相対・donor と同一アンカー規約）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（donor と同一規約）。
fn balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

// ---------------------------------------------------------------------------
// Boot resource（NonSend・EmoPresenter を内包・donor と同型）
// ---------------------------------------------------------------------------

/// probe の駆動フェーズ。GPU 資源到達で表示→本番 resize→（次フレーム以降で）k=1.0 assert、の順に進む。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbePhase {
    /// GPU 資源到達待ち。到達フレームで attach→表示→描画一致 anchor→本番 resize を駆動する。
    WaitingAttach,
    /// 本番 resize 済み。**次フレーム以降**（SetWindowPos flush 後）に `GetClientRect` で k=1.0 assert する。
    WaitingVerify,
    /// 自動 assert 完了。以降は手動プロトコル（⑤ 解決一致）を live ログしながら常駐する。
    Done,
}

/// 起動時に構築した presenter・装着待ちアセット・駆動フェーズを束ねる NonSend リソース
/// （`EmoPresenter` が `!Send` ゆえ本型も NonSend・donor `EmoBoot`/`PlacementBoot` と同作法）。
struct ProbeBoot {
    /// 提示段の統括ハブ（scope0 shell target を装着・表示する）。
    presenter: EmoPresenter,
    /// scope0 のキャラ窓 entity（`spawn_ghost_windows` が生成・placeholder 100×100 で spawn 済み）。
    char_window: Entity,
    /// scope0 shell アセット（`attach_target` で move 消費・装着後は `None`）。
    assets: Option<(EmoWorld, AtlasTable)>,
    /// 駆動フェーズ。
    phase: ProbePhase,
    /// 本番 resize で適用した現表示実寸（WaitingAttach で確定・k=1.0 assert のログ用）。
    surface_size: (u32, u32),
}

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

    // GPU 資源到達フレームで表示→本番 resize→k=1.0 assert を駆動する probe 起動 system。
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
    println!("  自動  : ③ k=1.0 assert（GetClientRect == surface_size ∧ scale==1.0）＋④ read_back 描画一致 anchor");
    println!("  手動  : ⑤ ゴーストの頭/胸/背景を目視で狙い、resolve 結果（Head/Bust/None）とペア列 Δ=(0,0) を記録");
    println!("  受け入れ: rustdoc プロトコル ①〜⑥（dpi≠96 を2水準必達・96 のみは不合格）");
    println!("  終了  : キャラ窓の不透明域をダブルクリック／AREKA_APP_SMOKE_EXIT_MS");
    println!();

    mgr.run()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Async Setup（UI スレッドで適用されるコマンド）
// ---------------------------------------------------------------------------

/// 起動セットアップコマンドを UI スレッドへ送る（donor `run_setup` と同型）。
async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        build_and_spawn(world);
    }));
}

/// 配置準備（placement 本体）→ placeholder 誤寸 spawn（placement 本体）→ `OnPointerMoved` 装着 →
/// 装着アセット構築（probe 側）→ `ProbeBoot` 挿入を一括で行う（UI スレッド）。
///
/// 失敗は log-first（`error!`）で中断する（受け入れ probe ゆえダミー窓フォールバックは持たない——失敗を
/// loud に観測させる。終了は Ctrl+C／smoke env）。
fn build_and_spawn(world: &mut World) {
    let ghost_root = emo2_root();
    let balloon_dir = balloon_root();

    // placement 本体の準備パイプライン（load→config→measure→primary work area→resolve・実モニタ列挙）。
    let mut prepared = match placement::prepare_ghost_windows(&ghost_root, &balloon_dir) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(
                error = %e,
                ghost_root = %ghost_root.display(),
                "collision-probe: 配置準備に失敗（実モニタ列挙が要る＝実 DPI モニタ上で実行すること）— 中止"
            );
            return;
        }
    };

    // MonitorSnapshot（bottom 吸着 resize が work area を引く消費用・`window-placement.rs:259-261` 前例）。
    world.insert_resource(placement::follow::MonitorSnapshot::from_monitors(
        &wintf::ecs::window::monitor::enumerate_monitors(),
    ));

    // donor 必須逸脱 #3: scope0 char を**意図的に誤った placeholder 寸**で spawn する（最終一致を本番
    // resize 経路のみで成立させる構図保証）。
    let Some(p0) = prepared.placements.iter_mut().find(|p| p.scope == 0) else {
        tracing::error!("collision-probe: scope0 の placement が無い（emo2 fixture 異常）— 中止");
        return;
    };
    tracing::info!(
        orig_w = p0.char_size.w,
        orig_h = p0.char_size.h,
        placeholder = PLACEHOLDER_SIZE,
        "collision-probe: scope0 char を placeholder 誤寸で spawn（本番 resize 経路のみで最終一致させる）"
    );
    p0.char_size = placement::resolver::SizePx {
        w: PLACEHOLDER_SIZE,
        h: PLACEHOLDER_SIZE,
    };

    // placement 本体の窓組立（markers・WindowPos・Anchored 無条件付与・DragConfig・double-click close）。
    let windows = spawn_ghost_windows(world, &prepared.placements, &prepared.titles);
    let Some(char_window) = windows.char_window(0) else {
        tracing::error!("collision-probe: scope0 char 窓 entity が無い — 中止");
        return;
    };

    // ⑤ マウス経路照合の観測シーム: probe 窓へ `OnPointerMoved` を1行装着（donor `OnPointerPressed`
    // 前例＝`emo-present.rs:523` と同型・placement 非改変）。ダブルクリック終了は spawn 付与の
    // `OnPointerPressed(on_ghost_pressed)` が担うため本 probe は移動ハンドラのみ追加する。
    world
        .entity_mut(char_window)
        .insert(OnPointerMoved(on_probe_pointer_moved));

    // shell アセット構築（emo-present donor と同経路: parse→bake→EmoWorld::build→bind_atlas）。
    let decoder = match WicDecoderArm::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "collision-probe: WicDecoderArm 生成に失敗（COM 未初期化？）— 中止（窓は配置済み）");
            return;
        }
    };
    let shell_dir = ghost_root.join("shell/master");
    let Some((emo_world, atlas)) = build_shell_target(&shell_dir, &decoder) else {
        tracing::error!("collision-probe: shell アセット構築に失敗 — 中止（窓は配置済み）");
        return;
    };

    world.insert_non_send_resource(ProbeBoot {
        presenter: EmoPresenter::new(),
        char_window,
        assets: Some((emo_world, atlas)),
        phase: ProbePhase::WaitingAttach,
        surface_size: (0, 0),
    });
    tracing::info!(
        "collision-probe: 窓生成・アセット構築完了（GPU 資源到達で surface1000 表示→本番 resize→k=1.0 assert）"
    );
}

/// shell dir の surfaces.txt から scope0 装着用の `(EmoWorld, AtlasTable)` を構築する（emo-present donor
/// `build_shell_target` と同経路）。失敗時は log-first で `None`。
fn build_shell_target(shell_dir: &Path, decoder: &WicDecoderArm) -> Option<(EmoWorld, AtlasTable)> {
    let surfaces_txt = shell_dir.join("surfaces.txt");
    let content = match std::fs::read_to_string(&surfaces_txt) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                path = %surfaces_txt.display(),
                error = %e,
                "collision-probe: shell surfaces.txt の読取に失敗"
            );
            return None;
        }
    };
    let shell = areka_parsers::shell::parse(&content);
    if shell.surfaces.is_empty() {
        tracing::error!("collision-probe: surfaces.txt が surface を 1 つも産まなかった");
        return None;
    }

    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: shell_dir,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], decoder, PackConfig::default());
    // emo2 shell は α 無し `purple/a/null.png` 1 枚が normalize seam として脱落する（既知・許容）。
    for err in &baked.errors {
        tracing::warn!(error = %err, "collision-probe: shell bake で脱落した element（surface1000 表示には無害）");
    }

    let mut emo_world = EmoWorld::build(&shell);
    emo_world.bind_atlas(&baked.table, SetId(0));
    Some((emo_world, baked.table))
}

// ---------------------------------------------------------------------------
// Probe boot system（GPU 到達→表示→本番 resize→k=1.0 assert）
// ---------------------------------------------------------------------------

/// GPU 資源到達フレームで surface1000 を表示し、④ 描画一致 anchor と ③ k=1.0 assert を駆動する probe 起動
/// system（`&mut World` 排他・donor `boot_present_system` と同作法: remove→駆動→insert）。
///
/// フェーズ機械（`ProbePhase`）で「表示＋本番 resize」と「（次フレーム以降の）`GetClientRect` 検証」を
/// 分離する——`SetWindowPosCommand` は発行 tick の World 借用解放後に flush されるため（`tick_bridge.rs:199-200`）、
/// 同 tick 内の `GetClientRect` は旧寸を返す。ゆえに resize と検証は別フレームに分ける。
fn boot_probe_system(world: &mut World) {
    let phase = match world.get_non_send_resource::<ProbeBoot>() {
        Some(b) => b.phase,
        None => return,
    };
    match phase {
        ProbePhase::WaitingAttach => attach_show_and_resize(world),
        ProbePhase::WaitingVerify => verify_k_equals_one(world),
        ProbePhase::Done => {}
    }
}

/// WaitingAttach: GPU 資源到達を待ち、attach→`apply(ShowSurface{1000, 実 bind})`→④ read_back 描画一致
/// anchor→本番 `resize_window_to`（戻り値 true を assert）を駆動して WaitingVerify へ進める。
fn attach_show_and_resize(world: &mut World) {
    // GPU 資源の準備待ち（未準備なら ProbeBoot を保持したまま次 tick へ）。
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    let mut boot = world
        .remove_non_send_resource::<ProbeBoot>()
        .expect("直上で存在確認済み");

    let target = target_map::shell_target(0);
    let Some((emo_world, atlas)) = boot.assets.take() else {
        // 装着アセットが無い（想定外）。前値保持で Done へ倒す。
        tracing::error!("collision-probe: 装着アセットが空（想定外）— 中止");
        boot.phase = ProbePhase::Done;
        world.insert_non_send_resource(boot);
        return;
    };

    if let Err(e) = boot.presenter.attach_target(
        world,
        target,
        boot.char_window,
        emo_world,
        atlas,
        // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
        // 供給する（本 example は k=1.0 相当で従来と同一の表示寸・当たり判定）。
        96,
    ) {
        tracing::error!(error = %e, "collision-probe: scope0 char target の attach に失敗 — 中止");
        boot.phase = ProbePhase::Done;
        world.insert_non_send_resource(boot);
        return;
    }

    // donor 必須逸脱 #1/#2: surface1000 を有効 bind 実値付きで表示する。
    boot.presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id: 1000,
            binds: BindSet::from_ids(REAL_BIND_IDS),
            pattern: PatternState::default(),
            reply: None,
        },
    );

    // 現表示実寸（=chain.size()）を読む。表示成立後は Some のはず（None は表示失敗＝中止）。
    let Some(view) = boot.presenter.text_slot_view(target) else {
        tracing::error!(
            "collision-probe: 表示成立後に text_slot_view が None（surface1000 の表示に失敗）— 中止"
        );
        boot.phase = ProbePhase::Done;
        world.insert_non_send_resource(boot);
        return;
    };
    let size = view.surface_size();
    boot.surface_size = size;
    tracing::info!(
        w = size.0,
        h = size.1,
        scale = view.scale(),
        "collision-probe: surface1000 表示成立・現表示実寸を取得（この寸へ本番 resize する）"
    );

    // ④ 描画一致の anchor（マウス非依存・read_back）: Head/Bust 中心画素が不透明であることを hard assert。
    assert_drawn_anchor(&boot.presenter, target, size);

    // donor 必須逸脱 #3: 本番の反映関数で placeholder 誤寸 → surface 実寸へ resize（戻り値 true を assert）。
    let ok = placement::follow::resize_window_to(
        world,
        boot.char_window,
        placement::resolver::SizePx {
            w: size.0 as i32,
            h: size.1 as i32,
        },
        // 表示実寸への再スナップ＝毎フレーム再スナップと同一の経路（Req 1.2・task 1.4）。
        placement::diag::PlacementRoute::Resnap,
    );
    assert!(
        ok,
        "collision-probe: resize_window_to が false（placeholder {PLACEHOLDER_SIZE}×{PLACEHOLDER_SIZE} → surface 実寸 {}×{} への本番 resize 経路が不発）",
        size.0, size.1
    );
    tracing::info!(
        "collision-probe: 本番 resize 経路（resize_window_to）適用 — 次フレーム以降に GetClientRect で k=1.0 を検証"
    );

    boot.phase = ProbePhase::WaitingVerify;
    world.insert_non_send_resource(boot);
}

/// WaitingVerify（本番 resize の次フレーム以降）: 実窓 `GetClientRect` が `surface_size()` と一致し
/// `scale() == 1.0` であることを hard assert する（③ k=1.0 assert）。**`WindowPos.size` ではなく実窓に対して
/// 行う**（`WindowPos` は enqueue 時 bypass ミラー済みで偽緑になるため）。
fn verify_k_equals_one(world: &mut World) {
    let mut boot = world
        .remove_non_send_resource::<ProbeBoot>()
        .expect("直上で存在確認済み");

    let target = target_map::shell_target(0);

    // 現表示実寸と scale を再読（値の源＝emo 合成パイプライン）。
    let (size, scale) = match boot.presenter.text_slot_view(target) {
        Some(v) => (v.surface_size(), v.scale()),
        None => {
            tracing::error!("collision-probe: WaitingVerify で text_slot_view が None（想定外）— 中止");
            boot.phase = ProbePhase::Done;
            world.insert_non_send_resource(boot);
            return;
        }
    };

    // 実窓 client 矩形（GetClientRect・areka＋OS 窓パイプラインの出力）。WindowPos ミラーは読まない。
    let Some(handle) = world.get::<WindowHandle>(boot.char_window).copied() else {
        tracing::error!("collision-probe: char 窓に WindowHandle 未付与（GetClientRect 不能）— 中止");
        boot.phase = ProbePhase::Done;
        world.insert_non_send_resource(boot);
        return;
    };
    let mut rect = RECT::default();
    // SAFETY: Win32 境界。GetClientRect は hwnd と RECT への可変ポインタを要し、client 矩形を書き込むだけ。
    let got = unsafe { GetClientRect(handle.hwnd, &mut rect) };
    assert!(
        got.is_ok(),
        "collision-probe: GetClientRect が失敗（hwnd={:?}）",
        handle.hwnd
    );
    let client_w = (rect.right - rect.left) as u32;
    let client_h = (rect.bottom - rect.top) as u32;

    // ③ k=1.0 assert: 経路独立な単位保存性検査（emo 合成パイプライン ↔ areka＋OS 窓パイプライン）。
    assert_eq!(
        (client_w, client_h),
        size,
        "collision-probe: GetClientRect client 寸 {:?} が surface_size {:?} と不一致 — 本番 resize 経路の単位保存性違反（dpi/96 再スケール・論理 px 解釈の混入の疑い）",
        (client_w, client_h),
        size
    );
    assert_eq!(
        scale, 1.0,
        "collision-probe: TextSlotView::scale() が 1.0 でない（k=1.0 契約違反）: {scale}"
    );
    tracing::info!(
        client_w,
        client_h,
        surface_w = size.0,
        surface_h = size.1,
        scale,
        "collision-probe: ③ k=1.0 assert 通過（GetClientRect == surface_size ∧ scale==1.0）— 手動プロトコル ⑤ へ"
    );

    boot.phase = ProbePhase::Done;
    world.insert_non_send_resource(boot);
    tracing::info!(
        "collision-probe: 自動 assert（③④）完了。マウスで頭/胸/背景を目視で狙い解決結果とペア列を記録してください（⑤）"
    );
}

/// ④ 描画一致の anchor: `read_back` した表示画素の Head/Bust 各矩形中心が**不透明**であることを hard assert。
///
/// collision 値（surface px）の位置に実際に絵が描かれていることを機械的に固定する（マウス非依存）。
/// 合成ビットマップ原点 ≡ サーフェス画像原点は構造的に保証される（`compute_extent` は原点 (0,0) 固定）ため、
/// collision 座標を read_back の画素 index へ無変換で写せる。
fn assert_drawn_anchor(presenter: &EmoPresenter, target: TargetId, size: (u32, u32)) {
    let bytes = match presenter.read_back(target) {
        Ok(b) => b,
        Err(e) => panic!("collision-probe: read_back に失敗（④ 描画一致 anchor が取れない）: {e}"),
    };
    assert_pixel_opaque(&bytes, size, rect_center(HEAD_RECT), "Head");
    assert_pixel_opaque(&bytes, size, rect_center(BUST_RECT), "Bust");
}

/// 当たり判定矩形の中心（サーフェス px・切り捨て）。
fn rect_center((left, top, right, bottom): (i64, i64, i64, i64)) -> (u32, u32) {
    (
        ((left + right) / 2) as u32,
        ((top + bottom) / 2) as u32,
    )
}

/// `read_back`（`stride = width*4`・BGRA・上端起点の密配列）の画素 `(px, py)` が不透明（α=0xFF）であることを
/// hard assert する。範囲外・バイト長不足も loud に落とす（④ の観測失敗を silent にしない）。
fn assert_pixel_opaque(bytes: &[u8], (w, h): (u32, u32), (px, py): (u32, u32), label: &str) {
    assert!(
        px < w && py < h,
        "collision-probe: {label} 中心 ({px},{py}) が合成外形 {w}×{h} の外（構図破綻＝collision 値と表示寸の乖離）"
    );
    // BGRA 密配列: 画素 (px,py) の α は offset (py*w+px)*4 + 3。
    let idx = ((py * w + px) * 4 + 3) as usize;
    assert!(
        idx < bytes.len(),
        "collision-probe: {label} 中心 ({px},{py}) の α index={idx} が read_back バイト長 {} の外",
        bytes.len()
    );
    let alpha = bytes[idx];
    assert_eq!(
        alpha, 0xFF,
        "collision-probe: {label} 矩形中心 ({px},{py}) の画素が不透明でない（α={alpha}）— collision 値の位置に絵が描かれていない"
    );
}

// ---------------------------------------------------------------------------
// Pointer handler（⑤ マウス経路照合＋解決 live ログ）
// ---------------------------------------------------------------------------

/// probe 窓の `OnPointerMoved` ハンドラ（⑤・donor `on_shell_pressed`＝`emo-present.rs:523` と同型）。
///
/// 記録行ごとに **本番マウス経路** `PointerState.client_point` と **クエリ系** `ScreenToClient(GetCursorPos())`
/// をペア列（client_x/y・s2c_x/y・Δx/Δy）で live ログし、Δ=(0,0) の厳密一致を人手検証させる。加えて
/// s2c で得た**目視由来**の client 点を [`resolve_hit_region`] へ渡し解決結果（Head/Bust/None）を併記する。
///
/// **反トートロジー（7.3(a)）**: 狙点は `GetCursorPos`（実カーソル＝目視由来）からのみ得る。collision 実値
/// から合成した screen 座標の `SetCursorPos`/`SendInput` 注入は**行わない**（本ハンドラも一切呼ばない）。
///
/// ペア列は不透明域（Head/Bust）行のみに出る——透明域はクリック透過でイベントが窓へ届かず、本ハンドラが
/// 呼ばれない（欠測が正しい挙動・脚注★）。
fn on_probe_pointer_moved(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble 相のみ処理（Tunnel は伝播続行）。伝播は止めない（false）＝ダブルクリック終了ハンドラと共存。
    let Phase::Bubble(state) = ev else {
        return false;
    };
    // 本番マウス経路（WM_MOUSEMOVE lparam 直系）の client 物理 px。
    let client = state.client_point;

    // presenter・char 窓は ProbeBoot から得る（ハンドラ実行時は insert 済み＝Input schedule と
    // FrameFinalize は同一 tick 内で直列ゆえ remove 中と重ならない）。
    let Some(boot) = world.get_non_send_resource::<ProbeBoot>() else {
        return false;
    };
    let char_window = boot.char_window;

    // 当該窓の HWND（クエリ系経路 ScreenToClient に要る）。
    let Some(handle) = world.get::<WindowHandle>(char_window) else {
        return false;
    };
    let hwnd = handle.hwnd;

    // クエリ系経路: GetCursorPos（screen 物理）→ ScreenToClient（当該窓 client 物理）。
    let mut pt = POINT::default();
    // SAFETY: Win32 境界。GetCursorPos は現在のカーソル位置を pt へ書き込むだけ（副作用なし）。
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        tracing::warn!("collision-probe: GetCursorPos 失敗 — この行のペア列/解決をスキップ");
        return false;
    }
    // SAFETY: Win32 境界。ScreenToClient は hwnd と POINT への可変ポインタを要し、pt を client へ写す。
    if !unsafe { ScreenToClient(hwnd, &mut pt).as_bool() } {
        tracing::warn!("collision-probe: ScreenToClient 失敗 — この行のペア列/解決をスキップ");
        return false;
    }
    let (s2c_x, s2c_y) = (pt.x, pt.y);

    // ペア列 Δ（両者とも同一 HWND の client 物理 px 整数＝丸め誤差の正当源が無く、要求は Δ=(0,0) 厳密一致）。
    let dx = client.x - s2c_x;
    let dy = client.y - s2c_y;

    // 解決は**目視由来**の s2c 点（GetCursorPos 経由）で行う（反トートロジー条件）。
    let hit = resolve_hit_region(&boot.presenter, 0, s2c_x as i64, s2c_y as i64);

    tracing::info!(
        client_x = client.x,
        client_y = client.y,
        s2c_x,
        s2c_y,
        dx,
        dy,
        region = ?hit.region,
        "collision-probe: ⑤ マウス経路ペア列＋解決結果（Δ=(0,0) 厳密一致が要求・ペア列は不透明域行のみ）"
    );
    false
}

// ---------------------------------------------------------------------------
// Smoke auto close（env ゲート・hands-off のビルド/起動確認用・donor と同型）
// ---------------------------------------------------------------------------

/// [`SMOKE_EXIT_ENV`] から自動 close の遅延ミリ秒を読む（未設定・空・非数値・負値はゲート OFF）。
fn smoke_exit_ms() -> Option<u64> {
    let value = std::env::var(SMOKE_EXIT_ENV).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// env ゲート付き smoke 自動 close を結線する（window-placement donor `install_smoke_exit` と同作法）。
/// 指定 ms 後に全 [`GhostWindowMarker`] 窓を despawn → `WindowRegistry` 空遷移 → `run()` 正常復帰。
fn install_smoke_exit(app: &WinApp) {
    let Some(ms) = smoke_exit_ms() else {
        return;
    };
    let world_weak = std::rc::Rc::downgrade(&app.world());
    tracing::info!(
        env = SMOKE_EXIT_ENV,
        delay_ms = ms,
        "collision-probe: smoke 自動 close ゲート有効 — 全ゴースト窓を指定 ms 後に despawn します"
    );
    wintf::executor::spawn_local(async move {
        async_io::Timer::after(std::time::Duration::from_millis(ms)).await;
        // shutdown 済みなら strong 所有者は消えており upgrade は None ＝ no-op。
        let Some(world) = world_weak.upgrade() else {
            tracing::debug!("collision-probe smoke 自動 close: world 既に drop 済み — no-op");
            return;
        };
        let mut ecs = world.borrow_mut();
        let w = ecs.world_mut();
        let targets: Vec<Entity> = w
            .query_filtered::<Entity, With<GhostWindowMarker>>()
            .iter(w)
            .collect();
        let count = targets.len();
        for e in targets {
            w.despawn(e);
        }
        tracing::info!(count, "collision-probe smoke 自動 close: ゴースト窓を despawn しました");
    });
}
