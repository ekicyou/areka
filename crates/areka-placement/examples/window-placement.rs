//! areka window-placement 実 DPI 受け入れ example
//!
//! placement パイプライン（`prepare_ghost_windows` → `spawn_ghost_windows`）で本番ゴースト
//! （emo2 fixture）の 2 スコープ×（キャラ窓＋バルーン窓）＝4 窓を配置・生成し、emo-present
//! donor（`crates/areka/examples/emo-present.rs`）と**同型の装着経路**（`EmoPresenter::attach_target` →
//! `apply(ShowSurface)`）で以下を装着する:
//!
//! - **scope0 キャラ窓**: emo2 `surface0`（むらさき・原寸 434×687）
//! - **scope1 キャラ窓**: emo2 `surface10`（エモ・原寸 336×400・ukadoc 正典の相方既定サーフェス）
//! - **各スコープのバルーン窓**: そのスコープのバルーンの面 0（scope0 は `balloons0.png`・原寸 400×224、
//!   scope1 は `balloonk0.png`・原寸 288×203・`areka_emo_present::build_balloon_target` 経由。
//!   測りがスコープごとの絵で窓の大きさを決めるので、貼る絵もスコープごとに選ぶ）
//!
//! 原寸は作者基準 DPI 96（emo2 は宣言なし＝正典既定）での物理 px。窓と絵はモニタ DPI ÷ 96 倍の
//! 物理 px になる（dpi=120 なら scope0 のキャラ窓は 543×859）。
//!
//! # 層境界
//!
//! 配置の crate（`crates/areka-placement/src/`）は `EmoPresenter` を **import しない**。装着コードは全て
//! 本 example 内にある。配置は crate の公開面（`areka_placement::…`）で使う。
//!
//! # TargetId 対応（example の裁量）
//!
//! placement は scope→`TargetId` の写像を規定しないため、本 example は
//! `char(scope) = TargetId(2*scope)`／`balloon(scope) = TargetId(2*scope+1)` を採る。
//!
//! # 使い方
//! ```text
//! cargo run -p areka-placement --example window-placement
//! ```
//!
//! ## 終了手段
//!
//! - **Ctrl+C**（コンソールから起動した場合）。窓のダブルクリックでは終わらない（ダブルクリックを
//!   受けるのは本体の `areka_input` の結線で、この example は付けない）
//! - **`AREKA_APP_SMOKE_EXIT_MS=<ms>`**: 指定 ms 後に全ゴースト窓を自動 despawn して正常終了する
//!   env ゲート（本体の `crates/areka/src/smoke_exit.rs` のゲートと同名。hands-off のビルド/起動確認用。
//!   例: `AREKA_APP_SMOKE_EXIT_MS=5000 cargo run -p areka-placement --example window-placement`）
//!
//! # 手動観測プロトコル
//!
//! 本プロトコルは開発者が手動で行う実 DPI 受け入れ検証（headless では代替不能・emo-present の
//! 実 DPI 手順の先行例に倣う）。以下 ①〜⑤ を実施し記録する:
//!
//! 1. **① per-monitor v2・dpi≠96 で実行する**: DPI awareness は wintf `WinApp` 初期化が
//!    プロセスに設定する（`SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`）。
//!    Windows「設定 → システム → ディスプレイ → 拡大縮小」で対象モニタのスケールを
//!    **125%（dpi=120）** または 150%/200%（dpi=144/192）にした状態で本 example を実行する。
//! 2. **② 初期配置が画面内**: scope0（むらさき）が **primary work area の右下**
//!    （右端・タスクバー上端）に、scope1（エモ）が **その左隣**（scope0 の左端から
//!    scope1 の窓の幅ぶん左・下端揃え）に、いずれも**画面内**に出現する（右下へ沈む・画面外へ
//!    はみ出す、のいずれも起きない）。窓の大きさは原寸のモニタ DPI ÷ 96 倍で、窓と表示された絵の
//!    外形が一致する（窓だけ・絵だけが大きくならない）。
//! 3. **③ キャラ窓ドラッグでバルーン追従**: キャラ窓を（透明部を除く）全面どこでも掴んで
//!    ドラッグでき、対応するバルーン窓が配置時の相対 offset を保って追従する。
//! 4. **④ モニタ境界を跨ぐドラッグで消失しない**: マルチモニタ環境で（DPI の異なる）モニタ
//!    境界を跨いでドラッグしても、窓が消失・吹き飛び・誤クランプしない
//!    （DragConstraint の誤釘付け欠陥の再発を防ぐ檻）。
//! 5. **⑤ 結果と実 DPI 値を記録する**: ②〜④ の pass/fail と使用した実 DPI 値
//!    （例 dpi=120/144/192）を記録する。**dpi=96 のみの確認は不合格**（dpi=96 では単位混在バグが
//!    自己整合して隠れるため、非 96 DPI の実走記録をもって受け入れとする）。
//!
//! # 観測注記（受け入れ判定の対象外）
//!
//! - **scope1 バルーンが scope0 キャラ窓に重なって出現するのは正常**: emo2 実測値
//!   （`kero.balloon.alignment,right`・両スコープ `defaultx,0`）では配置の幾何により
//!   `char_x(1) = char_x(0) − w1`・`balloon_x(1) = char_x(1) + w1 = char_x(0)` となり、
//!   scope1 のバルーンが scope0 キャラ窓に重畳する。これは暫定規則（重なり回避なし・暫定 offset）の**正常挙動**であり
//!   配置破綻ではない。正式なバルーン配置規則は balloon 表示系が所有する。
//! - **重畳域でバルーンが先に掴まれるのも正常**: 重畳域ではバルーン
//!   （scope1 のバルーンの絵 balloonk0.png）の不透明部が αマスクで先にヒットするため、scope0 キャラでなく
//!   バルーンが掴まれる。
//! - **scope0（surface0.png）に吹き出しとセリフ文字が見えるのは正常**: emo2 fixture の
//!   surface0 はバルーン＋サンプルセリフを**焼き込んだ**挨拶用デフォルト立ち絵であり、
//!   画像忠実表示の結果（テキスト描画機能ではない・バグではない）。
//!
//! # DPI 表示契約（donor と同一）
//!
//! 窓のクライアント領域は、surface の原寸を k 倍した物理 px に一致する（k＝DPI ÷ 作者基準 DPI。
//! 窓の大きさは採寸が primary モニタの DPI で、絵は presenter が窓の DPI で k を導く）。
//! placement パイプラインは座標・寸法を**物理 px 単一通貨**で運び、
//! `BoxStyle`（論理 DIP）・`DragConstraint` を一切使わない。

use bevy_ecs::prelude::*;
use sample_ghost_kit::SampleRoot;
use std::path::PathBuf;
use std::sync::LazyLock;
use tracing_subscriber::EnvFilter;
use windows::core::Result;

use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::{FrameFinalize, GraphicsCore, WucGraphicsResource};
use wintf::*;

use areka_emo_atlas::{AtlasTable, WicDecoderArm};
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_emo_present::{
    EmoPresenter, PresentCommand, ShellTarget, TargetId, build_balloon_target, load_shell_target,
};

use areka_placement::spawn::{
    GhostWindowMarker, register_ghost_windows_click_through, spawn_ghost_windows,
};

// ---------------------------------------------------------------------------
// Constants / fixture paths
// ---------------------------------------------------------------------------

/// smoke 自動 close の env ゲート名（本体の `smoke_exit.rs` と同名・`AREKA_` 冠規約）。
const SMOKE_EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";

/// emo2 検体。段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

fn emo2_root() -> PathBuf {
    EMO2.folder().to_path_buf()
}

fn balloon_root() -> PathBuf {
    EMO2.balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
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
    /// 作者基準 DPI（採寸が descript から読んだ値・キャラ窓はシェル、バルーン窓はバルーンの値）。
    author_dpi: u16,
    /// ログ用ラベル。
    label: String,
    /// target のアセット（`attach_target` で move 消費）。
    assets: (EmoWorld, AtlasTable),
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

    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_ghost_windows_click_through);

    world
        .borrow_mut()
        .add_systems(FrameFinalize, boot_present_system);

    install_smoke_exit(&mgr);

    println!();
    println!("areka window-placement 実 DPI 受け入れ example");
    println!("=================================================");
    println!("  scope0: キャラ窓 surface0（むらさき・work area 右下）＋バルーン窓（左隣）");
    println!(
        "  scope1: キャラ窓 surface10（エモ・scope0 の左）＋バルーン窓（右隣・scope0 に重畳＝正常）"
    );
    println!("  操作  : キャラ窓の不透明域ドラッグでバルーン追従／終了は Ctrl+C");
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
/// 失敗は log-first（`error!`）で中断する（受け入れ example ゆえ
/// 失敗を loud に観測させる。終了は Ctrl+C）。
fn build_and_spawn(world: &mut World) {
    let ghost_root = emo2_root();
    let balloon_dir = balloon_root();

    let prepared = match areka_placement::prepare_ghost_windows(&ghost_root, &balloon_dir) {
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

    // 実モニタ work area 集合を忠実転写した MonitorSnapshot を挿入する（bottom 吸着ドラッグの消費用・セッション内固定）。
    world.insert_resource(areka_placement::follow::MonitorSnapshot::from_monitors(
        &wintf::ecs::window::monitor::enumerate_monitors(),
    ));

    let windows = spawn_ghost_windows(world, &prepared.placements, &prepared.titles);

    let decoder = match WicDecoderArm::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "window-placement: WicDecoderArm 生成に失敗（COM 未初期化？）— 装着を中止（窓は配置済み）");
            return;
        }
    };

    // シェルの読み込みは一度だけ行う。`EmoWorld` は Clone 不可・`attach_target` が move 消費する
    // ため、target ごとに `ShellTarget::build_world` で組み直す（`AtlasTable` は Arc 共有ゆえ
    // clone は安価）。
    let shell_dir = ghost_root.join("shell/master");
    let Some(shell_target) = build_shell_material(&shell_dir, &decoder) else {
        tracing::error!(
            "window-placement: shell アセット素材の構築に失敗 — 装着を中止（窓は配置済み）"
        );
        return;
    };
    let shell_table = shell_target.atlas().clone();

    let mut pending = Vec::new();
    for scope in windows.scopes() {
        let Some(scope_u32) = target_scope(scope) else {
            tracing::error!(
                scope,
                "window-placement: スコープ番号が TargetId の u32 に収まらない — このスコープの装着を飛ばす（窓は配置済み）"
            );
            continue;
        };
        if let Some(char_e) = windows.char_window(scope) {
            let emo_world = shell_target.build_world();
            // 初期 surface id: 測りと同じ答え（窓寸法と表示 surface の原寸が一致する）。
            let surface_id = prepared
                .default_surfaces
                .initial_surface(scope, |id| emo_world.surface(id).is_some());
            pending.push(PendingTarget {
                target: TargetId(scope_u32 * 2),
                window: char_e,
                surface_id,
                author_dpi: prepared.author_dpi.shell,
                label: format!("scope{scope} char surface{surface_id}"),
                assets: (emo_world, shell_table.clone()),
            });
        }

        if let Some(balloon_e) = windows.balloon_window(scope) {
            match build_balloon_target(&balloon_dir, &decoder, scope_u32) {
                Ok((b_world, b_table)) => pending.push(PendingTarget {
                    target: TargetId(scope_u32 * 2 + 1),
                    window: balloon_e,
                    surface_id: 0,
                    author_dpi: prepared.author_dpi.balloon,
                    label: format!("scope{scope} balloon surface0"),
                    assets: (b_world, b_table),
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

    world.insert_non_send(PlacementBoot {
        presenter: EmoPresenter::new(),
        pending,
        attached: false,
    });
    tracing::info!("window-placement: 窓配置とアセット構築を完了（GPU 資源到達で表示を装着）");
}

/// shell dir から装着素材（[`ShellTarget`]＝焼いた絵と、面の表を必要な数だけ組めるもの）を
/// **シェル読み込みの権威**（`load_shell_target`）で組む（donor `load_shell_assets` と同経路。
/// 面の表の `build_world` は target ごとに呼び手が行う）。失敗時は log-first で `None`。
///
/// 一覧・`surfaces.txt` の読取と解析・面の画像の決定・焼くまでは権威が 1 回で行う
/// （`areka_emo_present::shell_target`）。文字コードの扱い（`charset` 宣言に従う）・焼く段で
/// 落ちた絵の `warn!` も権威側が持つので、この example は呼ぶだけである——本番の資産
/// （`areka_runtime::assets` の `build_shell_assets`）・採寸（`measure.rs` の `build_shell_assets`）と同じ入口を
/// 通るため、この example が見る絵は本番と食い違わない。
fn build_shell_material(
    shell_dir: &std::path::Path,
    decoder: &WicDecoderArm,
) -> Option<ShellTarget> {
    match load_shell_target(shell_dir, decoder) {
        Ok(target) => Some(target),
        Err(e) => {
            // load_shell_target は内部で error! 済み（一覧失敗／読取失敗／面 0 個）。文脈のみ添える。
            tracing::error!(
                dir = %shell_dir.display(),
                error = %e,
                "window-placement: shell の読み込みに失敗"
            );
            None
        }
    }
}

/// 窓のスコープの u32 の値（キャラ窓 `TargetId(2*scope)`・バルーン窓 `TargetId(2*scope+1)`・
/// `build_balloon_target` の第 3 引数）。`2*scope+1` が u32 に収まらないスコープは `None`
/// （切り詰めて別のスコープの target や絵にしない）。
fn target_scope(scope: usize) -> Option<u32> {
    let scope = u32::try_from(scope).ok()?;
    scope.checked_mul(2)?.checked_add(1).map(|_| scope)
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// GPU 資源到達フレームで `attach_target`→`apply(ShowSurface)` を各 target 高々 1 回駆動する
/// 起動 system（donor `boot_present_system` と同作法: `&mut World` 排他・remove→駆動→insert・
/// GPU 資源（`GraphicsCore`/`WucGraphicsResource`）が揃うまで保留し次 tick で再試行）。
fn boot_present_system(world: &mut World) {
    match world.get_non_send::<PlacementBoot>() {
        Some(b) if !b.attached => {}
        _ => return,
    }

    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    let mut boot = world
        .remove_non_send::<PlacementBoot>()
        .expect("直上で存在確認済み");

    for t in std::mem::take(&mut boot.pending) {
        let (emo_world, atlas) = t.assets;
        // 作者基準 DPI は採寸と同じ値（`PreparedPlacement::author_dpi`）なので、窓の大きさと表示の大きさが合う。
        boot.presenter
            .attach_target(t.target, t.window, emo_world, atlas, t.author_dpi);
        boot.presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: t.target,
                surface_id: t.surface_id,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: None,
            },
        );
        tracing::info!(target = ?t.target, label = %t.label, "window-placement: target を装着・表示");
    }

    boot.attached = true;
    world.insert_non_send(boot);
    tracing::info!("window-placement: 全 target の装着処理を完了");
}

// ---------------------------------------------------------------------------
// Smoke auto close（env ゲート・hands-off のビルド/起動確認用）
// ---------------------------------------------------------------------------

/// [`SMOKE_EXIT_ENV`] から自動 close の遅延ミリ秒を読む（ゲートを OFF にする値は本体の
/// `smoke_exit::smoke_exit_ms` と同じ: 未設定・空・非数値・負値。書かれているのに読めない値に
/// `warn!` を残すのはこの example だけ）。
fn smoke_exit_ms() -> Option<u64> {
    let value = std::env::var(SMOKE_EXIT_ENV).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.parse::<u64>() {
        Ok(ms) => Some(ms),
        Err(error) => {
            tracing::warn!(
                env = SMOKE_EXIT_ENV,
                value = %value,
                %error,
                "window-placement: smoke 自動 close の値がミリ秒の数として読めない — ゲートは OFF"
            );
            None
        }
    }
}

/// env ゲート付き smoke 自動 close を結線する（本体の `smoke_exit::arm` と同作法:
/// `wintf::executor::spawn_local`＋world `Weak`・一発の async タスク）。指定 ms 後に
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
        tracing::info!(
            count,
            "window-placement smoke 自動 close: ゴースト窓を despawn しました"
        );
    });
}

// 窓も GPU も使わない選びだけを確かめる（`[[example]] test = true` で `cargo test -p areka-placement` が走らせる）。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_whose_target_id_does_not_fit_u32_is_skipped() {
        assert_eq!(target_scope(1), Some(1));
        // 2*scope+1 が u32 に収まる最大のスコープまでは通し、その次から切り詰めずに外す。
        assert_eq!(target_scope(0x7FFF_FFFF), Some(0x7FFF_FFFF));
        assert_eq!(target_scope(0x8000_0000), None);
        assert_eq!(target_scope(1usize << 32), None);
    }
}
