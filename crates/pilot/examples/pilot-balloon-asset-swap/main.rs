//! 先進坑 pilot-balloon-asset-swap（使い捨て）。
//!
//! 走っているバルーン窓を閉じずに、present のバルーン資産だけを別のバルーンへ差し替えたとき、
//! どのフレームにも崩れ（混在・空・古い絵の残り・大きさの食い違い）が出ないかを確かめる。
//! 一次記録は隣の `README.md`（3 幕）。
//!
//! 実行: `cargo run -p pilot --example pilot-balloon-asset-swap`
//! 上限時間: `AREKA_APP_SMOKE_EXIT_MS`（ミリ秒・空／非数値は既定 90 秒）。
//!
//! 終了コード: 0＝完了・1＝上限時間で打ち切り・2＝初期化の失敗・3＝較正不合格。
//!
//! 窓の器は手本 `crates/areka/examples/emo-present.rs`（とその同名フォルダ）から
//! バルーン窓 1 つ分を写した。起動は `run()` の前に同期で組む（手本の非同期投函は使わない）。

mod capture;
mod observe;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsWindowVisible, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    WS_VISIBLE,
};

use wintf::WinApp;
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::layout::HitTest;
use wintf::ecs::{
    FrameFinalize, GraphicsCore, Point, SizeI, Update, Window, WindowHandle, WindowPos,
    WindowStyle, WucGraphicsResource,
};

use areka_emo_atlas::{AtlasTable, WicDecoderArm};
use areka_emo_compose::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState};
use areka_emo_present::{
    DEFAULT_AUTHOR_DPI, EmoPresenter, PresentCommand, TargetId, build_balloon_target,
};
use sample_ghost_kit::SampleRoot;

/// 上限時間の環境変数（本体 `areka` の smoke と同じ名前）。
const EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";
/// 上限時間の既定（design Key Decision 10）。
const DEFAULT_EXIT_MS: u64 = 90_000;
/// 窓の固定位置（物理 px・スクリーン座標）。
const WINDOW_POS: Point = Point { x: 160, y: 160 };
/// バルーン窓の表示先 id（手本と同じ 1）。
const BALLOON_TARGET: TargetId = TargetId(1);

// ---------------------------------------------------------------------------
// 終了の理由と終了コード（design §Runner・Key Decision 7）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Completed／CalibrationFailed は 2.4・3.1 で使う
enum ExitReason {
    Completed,
    CalibrationFailed,
    Deadline,
    InitFailure,
}

fn exit_code(reason: ExitReason) -> i32 {
    match reason {
        ExitReason::Completed => 0,
        ExitReason::Deadline => 1,
        ExitReason::InitFailure => 2,
        ExitReason::CalibrationFailed => 3,
    }
}

/// `AREKA_APP_SMOKE_EXIT_MS` → 上限時間（ms）。空・非数値は既定。
fn exit_ms_from(value: Option<&str>) -> u64 {
    value
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_EXIT_MS)
}

fn exit_ms_from_env() -> u64 {
    exit_ms_from(std::env::var(EXIT_ENV).ok().as_deref())
}

// ---------------------------------------------------------------------------
// 検体の資産（design §Runner Service Interface）
// ---------------------------------------------------------------------------

/// 検体 2 つの資産と、判別に使う面の合成結果（premultiplied BGRA・原寸）。
#[allow(dead_code)] // b・face_* は 2.x・3.x の観測と差し替えで使う
struct Assets {
    /// StayseeBalloon
    a: (EmoWorld, AtlasTable),
    /// emo2-kakukaku
    b: (EmoWorld, AtlasTable),
    /// Staysee balloons0（335x205）
    face_a0: ComposedSurface,
    /// Staysee balloons2（335x395）
    face_a2: ComposedSurface,
    /// kakukaku balloons0（400x224）
    face_b0: ComposedSurface,
}

fn build_assets(
    decoder: &WicDecoderArm,
    staysee_dir: &Path,
    kakukaku_dir: &Path,
) -> Result<Assets, String> {
    let build = |name: &str, dir: &Path| {
        build_balloon_target(dir, decoder, 0)
            .map_err(|e| format!("{name} の資産の構築に失敗（{}）: {e}", dir.display()))
    };
    let compose = |name: &str, (world, atlas): &(EmoWorld, AtlasTable), surface_id: u32| {
        let face = Composer::new()
            .compose(
                world,
                atlas,
                surface_id,
                &BindSet::default(),
                &PatternState::default(),
            )
            .map_err(|e| format!("{name} の面 {surface_id} の合成に失敗: {e}"))?;
        if face.width() == 0 || face.height() == 0 {
            return Err(format!("{name} の面 {surface_id} の合成外形が 0 寸"));
        }
        tracing::info!(
            name,
            surface_id,
            w = face.width(),
            h = face.height(),
            "判別に使う面を合成"
        );
        Ok(face)
    };
    let a = build("StayseeBalloon", staysee_dir)?;
    let b = build("emo2-kakukaku", kakukaku_dir)?;
    Ok(Assets {
        face_a0: compose("StayseeBalloon", &a, 0)?,
        face_a2: compose("StayseeBalloon", &a, 2)?,
        face_b0: compose("emo2-kakukaku", &b, 0)?,
        a,
        b,
    })
}

/// 判別対の標本点を導出し、点の数を集合ごとに出す。見分けられない対は `Err`（終了コード 2）。
fn signature(
    pair: &str,
    p: &ComposedSurface,
    q: &ComposedSurface,
) -> Result<observe::Signature, String> {
    let sig =
        observe::derive_signature(p, q).map_err(|e| format!("判別対 {pair} の標本点: {e}"))?;
    tracing::info!(
        pair,
        only_p = sig.only_p.len(),
        only_q = sig.only_q.len(),
        both = sig.both.len(),
        uses_only_p = sig.has_only_p(),
        "判別対の標本点"
    );
    Ok(sig)
}

// ---------------------------------------------------------------------------
// World に置く状態
// ---------------------------------------------------------------------------

#[derive(Component)]
struct BalloonWindow;

/// 走行の状態（上限時間と終了の理由）。`run()` の後に終了の理由を読む。
#[derive(Resource)]
struct Run {
    deadline: Instant,
    limit_ms: u64,
    exit: Option<ExitReason>,
}

/// presenter と資産（NonSend・UI スレッドだけが触る）。
struct Stage {
    presenter: EmoPresenter,
    #[allow(dead_code)] // 2.x・3.x で使う
    assets: Assets,
    /// 最初の表示に渡す StayseeBalloon の資産（`attach_target` が move で消費する）。
    ///
    /// `EmoWorld` は `Clone` でない（design の想定と違う）ので `assets.a` を複製できず、
    /// 起動時にもう 1 組を構築して持つ。`None` ＝表示済み。
    first: Option<(EmoWorld, AtlasTable)>,
    window: Entity,
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// クリック透過機構への窓登録（手本 `register_click_through_windows` と同じ）。
fn register_click_through(
    new_windows: Query<(Entity, &WindowHandle), (Added<WindowHandle>, With<BalloonWindow>)>,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (entity, wh) in new_windows.iter() {
        handle.register(entity, wh.hwnd);
        tracing::debug!(?entity, hwnd = ?wh.hwnd, "クリック透過機構へ窓を登録");
    }
}

/// 最初の表示（仮置き・3.1 で台本の起動の段へ移す）。
///
/// GPU 資源が揃ってから StayseeBalloon の面 0 を表示し、窓寸を絵に合わせる。
fn first_show_system(world: &mut World) {
    match world.get_non_send::<Stage>() {
        Some(s) if s.first.is_some() => {}
        _ => return,
    }
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .is_some_and(|r| r.is_valid());
    if !ready {
        return;
    }
    let mut stage = world.remove_non_send::<Stage>().expect("直上で確認済み");
    let (emo_world, atlas) = stage.first.take().expect("直上で確認済み");
    match stage.presenter.attach_target(
        world,
        BALLOON_TARGET,
        stage.window,
        emo_world,
        atlas,
        DEFAULT_AUTHOR_DPI,
    ) {
        Ok(()) => {
            stage.presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: BALLOON_TARGET,
                    surface_id: 0,
                    binds: BindSet::default(),
                    pattern: PatternState::default(),
                    reply: None,
                },
            );
            fit_window(&mut stage, world);
            tracing::info!("StayseeBalloon の面 0 を表示");
        }
        Err(e) => tracing::error!(error = %e, "StayseeBalloon の装着に失敗"),
    }
    world.insert_non_send(stage);
}

/// `take_pending_resize` → `WindowPos` で窓寸を絵に合わせる（位置は固定のまま）。
fn fit_window(stage: &mut Stage, world: &mut World) {
    let Some((w, h)) = stage.presenter.take_pending_resize(BALLOON_TARGET) else {
        return;
    };
    let size = SizeI {
        width: w as i32,
        height: h as i32,
    };
    let Some(mut wp) = world.get_mut::<WindowPos>(stage.window) else {
        tracing::error!(window = ?stage.window, "窓に WindowPos が無い — 窓寸を合わせられない");
        return;
    };
    if wp.size != Some(size) {
        wp.size = Some(size);
        tracing::debug!(w, h, "窓寸を絵に合わせた");
    }
}

/// 上限時間の検査（唯一の時計）。到達したら理由を出して窓を消し、`run()` を戻す。
///
/// 2.4 は打ち切りの集計（それまでの数）をここへ差し込む。
fn deadline_system(
    mut run: ResMut<Run>,
    windows: Query<(Entity, Option<&WindowHandle>), With<BalloonWindow>>,
    mut commands: Commands,
) {
    if run.exit.is_some() || Instant::now() < run.deadline {
        return;
    }
    run.exit = Some(ExitReason::Deadline);
    tracing::info!(
        limit_ms = run.limit_ms,
        "終了: 上限時間に到達（打ち切り）— 窓を消す"
    );
    for (e, wh) in &windows {
        if let Some(wh) = wh {
            // 窓が最後まで画面に出ていたことの控え（要件 3.7）。
            let mut rect = RECT::default();
            // SAFETY: 生きている窓の HWND（despawn はこの後）。
            let visible = unsafe { IsWindowVisible(wh.hwnd) }.as_bool();
            let got = unsafe { GetWindowRect(wh.hwnd, &mut rect) }.is_ok();
            tracing::debug!(hwnd = ?wh.hwnd, visible, got, ?rect, "消す直前の窓");
        }
        commands.entity(e).despawn();
    }
}

/// 記録の件数と絵の判別の内訳（2.4 の集計が入るまでの確かめ用）。
fn log_records(shared: &Mutex<observe::Shared>) {
    let s = shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut by = std::collections::BTreeMap::<String, u32>::new();
    for f in &s.frames {
        *by.entry(format!("{:?}", f.picture)).or_default() += 1;
    }
    tracing::info!(
        ticks = s.ticks.len(),
        frames = s.frames.len(),
        ?by,
        "tick とフレームの記録の件数（絵の判別の内訳）"
    );
}

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let reason = match boot_and_run() {
        Ok(reason) => reason,
        Err(e) => {
            tracing::error!(error = %e, "終了: 初期化に失敗（run() に入らない・run() 自体の失敗も含む）");
            ExitReason::InitFailure
        }
    };
    let code = exit_code(reason);
    tracing::info!(?reason, code, "終了コード");
    std::process::exit(code);
}

/// 起動（同期）→ `run()` → 終了の理由。`Err` は初期化の失敗（`run()` に入っていない）。
fn boot_and_run() -> Result<ExitReason, String> {
    let limit_ms = exit_ms_from_env();
    let app = WinApp::new().map_err(|e| format!("WinApp の初期化に失敗: {e}"))?;

    // SampleRoot は run() の後まで生かす（Drop で複製の木が消える）。
    let staysee =
        SampleRoot::acquire("StayseeBalloon").map_err(|e| format!("検体 StayseeBalloon: {e}"))?;
    let emo2 = SampleRoot::acquire("emo2").map_err(|e| format!("検体 emo2: {e}"))?;
    let kakukaku_dir = emo2
        .balloon("emo2-kakukaku")
        .map_err(|e| format!("検体 emo2 の同梱バルーン: {e}"))?;
    let decoder = WicDecoderArm::new().map_err(|e| format!("WicDecoderArm の生成に失敗: {e:?}"))?;
    let assets = build_assets(&decoder, staysee.folder(), kakukaku_dir)?;
    let first = build_balloon_target(staysee.folder(), &decoder, 0)
        .map_err(|e| format!("StayseeBalloon の資産（最初の表示用）の構築に失敗: {e}"))?;
    let sigs: Arc<[observe::Signature; 2]> = Arc::new([
        signature("(A0,B0)", &assets.face_a0, &assets.face_b0)?,
        signature("(A0,A2)", &assets.face_a0, &assets.face_a2)?,
    ]);
    let shared = Arc::new(Mutex::new(observe::Shared::default()));

    let world = app.world();
    {
        let mut w = world.borrow_mut();
        let (aw, ah) = (assets.face_a0.width(), assets.face_a0.height());
        let window = w
            .world_mut()
            .spawn((
                Name::new("Pilot-Balloon-Window"),
                BalloonWindow,
                Window {
                    title: "pilot-balloon-asset-swap".to_string(),
                    ..Default::default()
                },
                WindowStyle {
                    style: WS_POPUP | WS_VISIBLE,
                    ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                },
                WindowPos {
                    position: Some(WINDOW_POS),
                    size: Some(SizeI {
                        width: aw as i32,
                        height: ah as i32,
                    }),
                    ..Default::default()
                },
                // 窓自身はヒット対象外。当たりは emo-surface 子（α マスク）が担う。
                HitTest::none(),
            ))
            .id();
        tracing::info!(
            ?window,
            x = WINDOW_POS.x,
            y = WINDOW_POS.y,
            w = aw,
            h = ah,
            limit_ms,
            "バルーン窓を生成"
        );
        w.world_mut().insert_non_send(Stage {
            presenter: EmoPresenter::new(),
            assets,
            first: Some(first),
            window,
        });
        w.world_mut().insert_resource(Run {
            deadline: Instant::now() + Duration::from_millis(limit_ms),
            limit_ms,
            exit: None,
        });
        w.world_mut().insert_resource(observe::Observer {
            window,
            sigs: sigs.clone(),
            active: observe::PAIR_ASSET,
            shared: shared.clone(),
        });
        w.add_systems(Update, first_show_system);
        w.add_systems(
            FrameFinalize,
            (
                register_click_through,
                deadline_system,
                // tick の最後（窓を消した tick は記録しない）。
                observe::tick_record_system
                    .after(register_click_through)
                    .after(deadline_system),
            ),
        );
    }

    let capture = capture::Capture::start(shared.clone(), sigs)?;
    let ran = app.run();
    capture.stop();
    // run() の失敗は初期化ではないが、窓も記録も失われているので初期化の失敗と同じ 2 に倒す。
    ran.map_err(|e| format!("run() が失敗: {e}"))?;
    log_records(&shared);

    let reason = world.borrow().world().resource::<Run>().exit;
    drop((staysee, emo2));
    Ok(reason.unwrap_or_else(|| {
        tracing::error!("run() が終了の理由なしに戻った（窓が外から閉じられた？）");
        ExitReason::InitFailure
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_ms_falls_back_on_empty_or_non_numeric() {
        assert_eq!(exit_ms_from(None), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("")), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("abc")), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("5000")), 5000);
    }
}
