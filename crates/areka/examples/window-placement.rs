//! areka window-placement 実 DPI 受け入れ example（areka-P0-window-placement task 7.1）
//!
//! placement パイプライン（`prepare_ghost_windows` → `spawn_ghost_windows`）で本番ゴースト
//! （emo2 fixture）の 2 スコープ×（キャラ窓＋バルーン窓）＝4 窓を配置・生成し、emo-present
//! donor（`examples/emo-present.rs`）と**同型の装着経路**（`EmoPresenter::attach_target` →
//! `apply(ShowSurface)`）で以下を装着する（要件 3.1/3.5/4.1/4.2/4.3/4.5）:
//!
//! - **scope0 キャラ窓**: emo2 `surface0`（むらさき・434×687 物理 px）
//! - **scope1 キャラ窓**: emo2 `surface10`（エモ・336×400 物理 px・ukadoc 正典の相方既定サーフェス）
//! - **両バルーン窓**: balloon target（`balloons0.png`・400×224 物理 px・
//!   `areka_emo_present::build_balloon_target` 経由）
//!
//! # 層境界（6.3）
//!
//! `crates/areka/src/placement/` 本体は `EmoPresenter` を **import しない**。装着コードは全て
//! 本 example 内にあり、areka-emo-present は dev-dependency として example からのみ使う。
//! placement モジュールへは `#[path]` include でアクセスする（areka は bin crate で lib target を
//! 持たないため。placement の非テストコードは `super::`／外部 crate のみを参照し `crate::` パスを
//! 持たないので、example の私有モジュールとして include 可能——production コードは無改変）。
//!
//! # TargetId 対応（example の裁量）
//!
//! placement は scope→`TargetId` の写像を規定しない（design「Data Models」）ため、本 example は
//! `char(scope) = TargetId(2*scope)`／`balloon(scope) = TargetId(2*scope+1)` を採る。
//!
//! # 使い方
//! ```text
//! cargo run -p areka --example window-placement
//! ```
//!
//! ## 終了手段
//!
//! - **キャラ窓／バルーン窓の不透明域をダブルクリック**: 全ゴースト窓
//!   （`GhostWindowMarker`）を despawn → `WindowRegistry` 空遷移 → `run()` 正常復帰
//!   （`spawn_ghost_windows` 組込みの `on_ghost_pressed`・placement task 5.1）
//! - **Ctrl+C**（コンソールから起動した場合）
//! - **`AREKA_APP_SMOKE_EXIT_MS=<ms>`**: 指定 ms 後に全ゴースト窓を自動 despawn して正常終了する
//!   env ゲート（main.rs の smoke ゲートと同名・同義。hands-off のビルド/起動確認用。
//!   例: `AREKA_APP_SMOKE_EXIT_MS=5000 cargo run -p areka --example window-placement`）
//!
//! # 手動観測プロトコル（3.1/3.5/4.1〜4.5・design「examples/window-placement.rs（実 DPI 受け入れ）」）
//!
//! 本プロトコルは開発者が手動で行う実 DPI 受け入れ検証（headless では代替不能・emo-present の
//! 実 DPI 手順の先行例に倣う）。以下 ①〜⑤ を実施し記録する:
//!
//! 1. **① per-monitor v2・dpi≠96 で実行する**: DPI awareness は wintf `WinApp` 初期化が
//!    プロセスに設定する（`SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`）。
//!    Windows「設定 → システム → ディスプレイ → 拡大縮小」で対象モニタのスケールを
//!    **125%（dpi=120）** または 150%/200%（dpi=144/192）にした状態で本 example を実行する。
//! 2. **② 初期配置が画面内**: scope0（むらさき・434×687）が **primary work area の右下**
//!    （右端・タスクバー上端）に、scope1（エモ・336×400）が **その左隣**（scope0 の左端から
//!    surface 幅ぶん左・下端揃え）に、いずれも**画面内**に出現する（右下へ沈む・画面外へ
//!    はみ出す・スケール倍率で膨れる、のいずれも起きない）。
//! 3. **③ キャラ窓ドラッグでバルーン追従**: キャラ窓を（透明部を除く）全面どこでも掴んで
//!    ドラッグでき、対応するバルーン窓が配置時の相対 offset を保って追従する。
//! 4. **④ モニタ境界を跨ぐドラッグで消失しない**: マルチモニタ環境で（DPI の異なる）モニタ
//!    境界を跨いでドラッグしても、窓が消失・吹き飛び・誤クランプしない
//!    （07-05 の DragConstraint 誤釘付け欠陥の再発檻）。
//! 5. **⑤ 結果と実 DPI 値を記録する**: ②〜④ の pass/fail と使用した実 DPI 値
//!    （例 dpi=120/144/192）を記録する。**dpi=96 のみの確認は不合格**（3.5・
//!    dpi=96 では単位混在バグが自己整合して隠れるため、非 96 DPI の実走記録をもって
//!    受け入れとする）。
//!
//! # 観測注記（受け入れ判定の対象外・設計討議 #1 確定）
//!
//! - **scope1 バルーンが scope0 キャラ窓に重なって出現するのは正常**: emo2 実測値
//!   （`kero.balloon.alignment,right`・両スコープ `defaultx,0`）では P5 幾何により
//!   `balloon_x(1) = char_x(0) − w0 + w1` となり、scope1 のバルーンが scope0 キャラ窓に
//!   重畳する。これは暫定規則（2.9 重なり回避なし・4.4 暫定 offset）の**正常挙動**であり
//!   配置破綻ではない。正式なバルーン配置規則は balloon 表示系の後続 spec が所有する。
//! - **重畳域でバルーンが先に掴まれるのも正常**: 重畳域ではバルーン不透明部
//!   （balloons0.png 内側 A=255）が αマスクで先にヒットするため、scope0 キャラでなく
//!   バルーンが掴まれる。
//! - **scope0（surface0.png）に吹き出しとセリフ文字が見えるのは正常**: emo2 fixture の
//!   surface0 はバルーン＋サンプルセリフを**焼き込んだ**挨拶用デフォルト立ち絵であり、
//!   画像忠実表示の結果（テキスト描画機能ではない・バグではない）。
//!
//! # DPI 表示契約（donor と同一）
//!
//! 窓のクライアント領域は surface 原寸（物理 px）に一致する（等倍・DPI 拡縮なし）。
//! placement パイプラインは座標・寸法を**物理 px 単一通貨**で運び（design U1〜U5）、
//! `BoxStyle`（論理 DIP）・`DragConstraint` を一切使わない。

use bevy_ecs::prelude::*;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;
use windows::core::Result;

use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::{FrameFinalize, GraphicsCore, WucGraphicsResource};
use wintf::*;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, PackConfig, SetId, SurfaceSet, UseSelfAlpha, WicDecoderArm, bake,
};
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId, build_balloon_target};

/// 窓配置機構本体（`crates/areka/src/placement/`）を example の私有モジュールとして include する。
///
/// areka は bin crate（lib target なし）のため `use areka::placement` はできない。placement の
/// 非テストコードは `super::`／外部 crate のみを参照する（`crate::` パスは `#[cfg(test)]` 内のみ）
/// ため、`#[path]` include で production コード無改変のままアクセスできる（6.3: 本体は
/// `EmoPresenter` を import しない——装着コードは全て本 example 側）。
/// `#[allow(dead_code)]` は「main.rs だけが消費する公開面（フォールバック用 API 等）」を
/// example ビルドで未使用警告にしないための include 側指定。
#[path = "../src/placement/mod.rs"]
#[allow(dead_code)]
mod placement;

use placement::spawn::{
    GhostWindowMarker, register_ghost_windows_click_through, spawn_ghost_windows,
};

// ---------------------------------------------------------------------------
// Constants / fixture paths
// ---------------------------------------------------------------------------

/// scope1 以降の初期 surface id（ukadoc 正典: 相方既定サーフェス＝10。
/// `placement::measure` の採寸規約と同値——採寸した surface と装着する surface を一致させる）。
const KERO_INITIAL_SURFACE_ID: u32 = 10;

/// smoke 自動 close の env ゲート名（main.rs と同名・`AREKA_` 冠規約）。
const SMOKE_EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";

/// emo2 fixture のゴーストルート（`CARGO_MANIFEST_DIR`＝`crates/areka` 相対・
/// donor emo-present／placement 統合テストと同一アンカー規約）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（placement task 4.1／6.1 テストの規約を踏襲）。
fn balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

// ---------------------------------------------------------------------------
// Boot resource（NonSend・EmoPresenter を内包・donor EmoBoot と同型）
// ---------------------------------------------------------------------------

/// 装着待ちの 1 target ぶんの記録（窓 entity・表示する surface id・アセット）。
struct PendingTarget {
    /// 装着先 target id（char=2*scope・balloon=2*scope+1）。
    target: TargetId,
    /// 装着先の窓 entity（`spawn_ghost_windows` が生成）。
    window: Entity,
    /// 初回 `apply(ShowSurface)` で表示する surface id。
    surface_id: u32,
    /// ログ用ラベル。
    label: String,
    /// target のアセット（`attach_target` で move 消費・装着後は `None`）。
    assets: Option<(EmoWorld, AtlasTable)>,
}

/// 起動時に構築した presenter・装着待ち target 群を束ね、GPU 資源が揃うまで attach/apply を
/// 保留する NonSend リソース（`EmoPresenter` が `!Send` ゆえ本型も NonSend・donor と同作法）。
struct PlacementBoot {
    presenter: EmoPresenter,
    pending: Vec<PendingTarget>,
    /// 装着＋初回表示を済ませたか（毎フレームの remove/insert churn を避けるゲート）。
    attached: bool,
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

    // 非同期タスク経由で「配置準備＋窓生成＋アセット構築＋presenter 生成」を UI スレッドへ
    // 投函する（UI スレッドは MTA・COM 初期化済み＝prepare の WIC 前提を満たす）。
    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    // clickthrough 登録 system（placement 本体の一般化 system・donor と同じ FrameFinalize 結線）。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_ghost_windows_click_through);

    // GPU 資源到達フレームで attach_target→apply を駆動する起動 system（donor と同作法）。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, boot_present_system);

    // env ゲート付き smoke 自動 close（hands-off のビルド/起動確認用・main.rs と同名 env）。
    install_smoke_exit(&mgr);

    // 操作ガイド出力。
    println!();
    println!("areka window-placement 実 DPI 受け入れ example");
    println!("=================================================");
    println!("  scope0: キャラ窓 surface0（むらさき・work area 右下）＋バルーン窓（左隣）");
    println!("  scope1: キャラ窓 surface10（エモ・scope0 の左）＋バルーン窓（右隣・scope0 に重畳＝正常）");
    println!("  操作  : キャラ窓の不透明域ドラッグでバルーン追従／不透明域ダブルクリックで終了");
    println!("  受け入れ: rustdoc の手動観測プロトコル ①〜⑤（dpi≠96 必須・96 のみは不合格）");
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

/// 配置準備（placement 本体）→窓生成（placement 本体）→装着アセット構築（example 側）→
/// `PlacementBoot` 挿入を一括で行う（UI スレッド）。
///
/// 失敗は log-first（`error!`）で中断する（受け入れ example ゆえダミー窓フォールバックは
/// 持たない——失敗を loud に観測させる。終了は Ctrl+C）。
fn build_and_spawn(world: &mut World) {
    let ghost_root = emo2_root();
    let balloon_dir = balloon_root();

    // placement 本体の準備パイプライン（load→config→measure→primary work area→resolve）。
    let prepared = match placement::prepare_ghost_windows(&ghost_root, &balloon_dir) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(
                error = %e,
                ghost_root = %ghost_root.display(),
                "window-placement: 配置準備に失敗 — 中止（終了は Ctrl+C）"
            );
            return;
        }
    };
    for p in &prepared.placements {
        tracing::info!(
            scope = p.scope,
            char_pos = ?(p.char_pos.x, p.char_pos.y),
            char_size = ?(p.char_size.w, p.char_size.h),
            balloon_pos = ?(p.balloon_pos.x, p.balloon_pos.y),
            balloon_offset = ?(p.balloon_offset.x, p.balloon_offset.y),
            "window-placement: 解決済み配置（物理 px）"
        );
    }

    // placement 本体の窓組立（markers・WindowPos・DragConfig・BalloonFollow・double-click close）。
    let windows = spawn_ghost_windows(world, &prepared.placements, &prepared.titles);

    // ここから先は example 側の装着アセット構築（6.3: placement 本体は EmoPresenter 非依存）。
    // 実 WIC デコーダ（COM 初期化済み UI スレッドで生成・donor と同型）。
    let decoder = match WicDecoderArm::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "window-placement: WicDecoderArm 生成に失敗（COM 未初期化？）— 装着を中止（窓は配置済み）");
            return;
        }
    };

    // shell アセットの素材（parse＋bake）は一度だけ組む。`EmoWorld` は Clone 不可・
    // `attach_target` が move 消費するため、target ごとに `EmoWorld::build`＋`bind_atlas` で
    // 組み直す（`AtlasTable` は Arc 共有ゆえ clone は安価）。
    let shell_dir = ghost_root.join("shell/master"); // emo2 の shell dir（donor と同一）
    let Some((shell_parsed, shell_table)) = build_shell_material(&shell_dir, &decoder) else {
        tracing::error!("window-placement: shell アセット素材の構築に失敗 — 装着を中止（窓は配置済み）");
        return;
    };

    let mut pending = Vec::new();
    for scope in windows.scopes().collect::<Vec<_>>() {
        // 初期 surface id: scope0→0・scope1→10（`placement::measure` の採寸規約と同値＝
        // 窓寸法と表示 surface の原寸が一致する）。scope n≥2 は正典既定なし・id10 暫定＋warn。
        let surface_id = if scope == 0 {
            0
        } else {
            if scope >= 2 {
                tracing::warn!(
                    scope,
                    surface_id = KERO_INITIAL_SURFACE_ID,
                    "window-placement: scope n≥2 の初期 surface は正典既定なし（id10 暫定）"
                );
            }
            KERO_INITIAL_SURFACE_ID
        };

        // キャラ窓 target（TargetId = 2*scope）。
        if let Some(char_e) = windows.char_window(scope) {
            let mut emo_world = EmoWorld::build(&shell_parsed);
            emo_world.bind_atlas(&shell_table, SetId(0));
            pending.push(PendingTarget {
                target: TargetId((scope as u32) * 2),
                window: char_e,
                surface_id,
                label: format!("scope{scope} char surface{surface_id}"),
                assets: Some((emo_world, shell_table.clone())),
            });
        }

        // バルーン窓 target（TargetId = 2*scope+1・balloon surface0＝balloons0.png）。
        if let Some(balloon_e) = windows.balloon_window(scope) {
            match build_balloon_target(&balloon_dir, &decoder) {
                Ok((b_world, b_table)) => pending.push(PendingTarget {
                    target: TargetId((scope as u32) * 2 + 1),
                    window: balloon_e,
                    surface_id: 0,
                    label: format!("scope{scope} balloon surface0"),
                    assets: Some((b_world, b_table)),
                }),
                Err(e) => {
                    // build_balloon_target は内部で error! 済み。窓は配置済みのまま装着だけ欠く。
                    tracing::error!(
                        scope,
                        error = %e,
                        "window-placement: バルーン target 構築に失敗（窓は配置済み・装着なし）"
                    );
                }
            }
        }
    }

    world.insert_non_send_resource(PlacementBoot {
        presenter: EmoPresenter::new(),
        pending,
        attached: false,
    });
    tracing::info!("window-placement: 窓配置とアセット構築を完了（GPU 資源到達で表示を装着）");
}

/// shell dir の surfaces.txt から装着素材（parse 済み `Shell`＋bake 済み `AtlasTable`）を組む
/// （donor `build_shell_target` と同経路: read→parse→bake。`EmoWorld` の build＋bind は
/// target ごとに呼び手が行う）。失敗時は log-first で `None`。
fn build_shell_material(
    shell_dir: &std::path::Path,
    decoder: &WicDecoderArm,
) -> Option<(areka_parsers::shell::Shell, AtlasTable)> {
    let surfaces_txt = shell_dir.join("surfaces.txt");
    let content = match std::fs::read_to_string(&surfaces_txt) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                path = %surfaces_txt.display(),
                error = %e,
                "window-placement: shell surfaces.txt の読取に失敗"
            );
            return None;
        }
    };
    let shell = areka_parsers::shell::parse(&content);
    if shell.surfaces.is_empty() {
        tracing::error!("window-placement: surfaces.txt が surface を 1 つも産まなかった");
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
        tracing::warn!(error = %err, "window-placement: shell bake で脱落した element（surface0/10 表示には無害）");
    }
    Some((shell, baked.table))
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// GPU 資源到達フレームで `attach_target`→`apply(ShowSurface)` を各 target 高々 1 回駆動する
/// 起動 system（donor `boot_present_system` と同作法: `&mut World` 排他・remove→駆動→insert・
/// GPU 資源（`GraphicsCore`/`WucGraphicsResource`）が揃うまで保留し次 tick で再試行）。
fn boot_present_system(world: &mut World) {
    // 未挿入 or 装着済みなら何もしない（装着後の remove/insert churn を避ける）。
    match world.get_non_send_resource::<PlacementBoot>() {
        Some(b) if !b.attached => {}
        _ => return,
    }

    // GPU 資源の準備待ち（未準備なら PlacementBoot を保持したまま次 tick へ）。
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    let mut boot = world
        .remove_non_send_resource::<PlacementBoot>()
        .expect("直上で存在確認済み");

    for t in boot.pending.iter_mut() {
        let Some((emo_world, atlas)) = t.assets.take() else {
            continue;
        };
        match boot
            .presenter
            .attach_target(world, t.target, t.window, emo_world, atlas)
        {
            Ok(()) => {
                boot.presenter.apply(
                    world,
                    PresentCommand::ShowSurface {
                        target: t.target,
                        surface_id: t.surface_id,
                        binds: BindSet::default(),
                        reply: None,
                    },
                );
                tracing::info!(target = ?t.target, label = %t.label, "window-placement: target を装着・表示");
            }
            Err(e) => {
                tracing::error!(target = ?t.target, label = %t.label, error = %e, "window-placement: target の attach に失敗");
            }
        }
    }

    boot.attached = true;
    world.insert_non_send_resource(boot);
    tracing::info!("window-placement: 全 target の装着処理を完了");
}

// ---------------------------------------------------------------------------
// Smoke auto close（env ゲート・hands-off のビルド/起動確認用）
// ---------------------------------------------------------------------------

/// [`SMOKE_EXIT_ENV`] から自動 close の遅延ミリ秒を読む（main.rs の smoke ゲートと同義:
/// 未設定・空・非数値・負値はゲート OFF）。
fn smoke_exit_ms() -> Option<u64> {
    let value = std::env::var(SMOKE_EXIT_ENV).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// env ゲート付き smoke 自動 close を結線する（main.rs `open_startup_window` の smoke ゲートと
/// 同作法: `wintf::executor::spawn_local`＋world `Weak`・一発の async タスク）。指定 ms 後に
/// 全 [`GhostWindowMarker`] 窓を despawn → `WindowRegistry` 空遷移 → `run()` 正常復帰。
fn install_smoke_exit(app: &WinApp) {
    let Some(ms) = smoke_exit_ms() else {
        return;
    };
    let world_weak = std::rc::Rc::downgrade(&app.world());
    tracing::info!(
        env = SMOKE_EXIT_ENV,
        delay_ms = ms,
        "window-placement: smoke 自動 close ゲート有効 — 全ゴースト窓を指定 ms 後に despawn します"
    );
    wintf::executor::spawn_local(async move {
        async_io::Timer::after(std::time::Duration::from_millis(ms)).await;
        // shutdown 済みなら strong 所有者は消えており upgrade は None ＝ no-op。
        let Some(world) = world_weak.upgrade() else {
            tracing::debug!("window-placement smoke 自動 close: world 既に drop 済み — no-op");
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
        tracing::info!(count, "window-placement smoke 自動 close: ゴースト窓を despawn しました");
    });
}
