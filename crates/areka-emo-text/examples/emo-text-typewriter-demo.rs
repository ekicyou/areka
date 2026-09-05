#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # emo-text-typewriter-demo — バルーン＋タイプライター表示のインタラクティブデモ
//!
//! emo2 fixture のバルーン枠を実表示し、挨拶テキストを **実時間タイプライター**
//! （配送 duration 由来の interval＝`CHAR_WAIT` 秒/グリフ・注入時刻＝フレーム経過時間で駆動）で表示する。
//! テキストは表示しきったあと少し間を置いて全消去し、繰り返す（ループ）。
//!
//! **バルーンをダブルクリックするとデモが終了する**（`OnPointerPressed` の bubble 相で
//! `PointerState.double_click == Left` を検出→窓を despawn→`run()` 復帰）。
//!
//! ```text
//! cargo run -p areka-emo-text --example emo-text-typewriter-demo
//! ```
//!
//! 注: これは対話デモ（自動 pass/fail 判定は行わない）。描画・タイプライター進行・
//! ダブルクリック終了を目視確認するためのもの。既存の観測 example（emo-text-layer.rs・
//! 自動判定つき）とは別物。

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

use wintf::WinApp;
use wintf::ecs::layout::HitTest;
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::{
    FrameFinalize, FrameTime, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos,
    WindowStyle, WucGraphicsResource,
};

use areka_actor::reply_channel;
use areka_emo_atlas::{AtlasTable, WicDecoderArm};
use areka_emo_compose::{BindSet, Composer, EmoWorld};
use areka_emo_present::{
    EmoPresenter, PresentCommand, PresentOutcome, TargetId, build_balloon_target,
};
use areka_emo_text::actor::{TextLayerRuntime, present_frame};
use areka_emo_text::state::TextLayerConfig;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

// ── 挨拶テキスト（さっきの画像と同じ・boot.pasta 起動朝 由来） ──
const LINE1: &str = "おっはよー！";
const LINE2: &str = "めっちゃええ朝やん！";
const LINE3: &str = "今日もいくでー！";

/// 見やすいタイプ速度（秒/グリフ・既定 0.05 より遅く）。
const CHAR_WAIT: f64 = 0.12;
/// 1 サイクル長（タイプ完了＋余韻・秒）。超えたら Clear して打ち直す。
const CYCLE_SECS: f64 = 5.0;
/// バルーン窓の初期位置（物理 px・スクリーン座標）。
const BALLOON_POS: (i32, i32) = (360, 220);

fn shared_balloon_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku")
}

fn read_decoded(path: &Path) -> Option<String> {
    match std::fs::read(path) {
        Ok(bytes) => Some(decode(&bytes, DefaultEncoding::Utf8)),
        Err(e) => {
            error!(path = %path.display(), error = %e, "balloon descript の読取に失敗");
            None
        }
    }
}

fn load_balloon_model() -> Option<BalloonModel> {
    let dir = shared_balloon_dir();
    let base = read_decoded(&dir.join("descript.txt"))?;
    let overlay = read_decoded(&dir.join("balloons0s.txt"))?;
    Some(parse_str(&base, Some(&overlay)))
}

fn actor() -> ActorKey {
    ActorKey::from("0")
}
/// デモ用 cue。reveal は配送 duration 由来（`interval = duration / N`）ゆえ、Text cue へ
/// `N × CHAR_WAIT` を焼き込むと 1 グリフあたり CHAR_WAIT 秒の typewriter 進行になる
/// （全 cue at=0.0 なので reveal は連続的に累積・旧 char_wait 定数と機能等価）。
fn cue(at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => t.chars().count() as f64 * CHAR_WAIT,
        _ => 0.0,
    };
    TalkCue {
        at,
        actor: actor(),
        command,
        duration,
    }
}

/// ダブルクリック終了要求（`OnPointerPressed` が立て、駆動 system が回収して窓を閉じる）。
#[derive(Resource, Default)]
struct QuitRequested(bool);

/// デモ状態（NonSend・UI スレッド所有）。
struct Demo {
    presenter: EmoPresenter,
    win: Entity,
    assets: Option<(EmoWorld, AtlasTable)>,
    model: BalloonModel,
    runtime: Rc<RefCell<TextLayerRuntime>>,
    attached: bool,
    /// 現サイクルのタイプ開始フレーム時刻。
    cycle_start: f64,
    /// 現サイクルの cue を流し終えたか。
    fed: bool,
}

fn main() -> windows::core::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();
    world
        .borrow_mut()
        .world_mut()
        .insert_resource(QuitRequested::default());

    // UI スレッドでアセット構築＋窓生成＋Demo 挿入。
    world.borrow().spawn(move |tx| async move {
        let _ = tx.send(Box::new(move |world: &mut World| {
            build_and_spawn(world);
        }));
    });
    world.borrow_mut().add_systems(FrameFinalize, drive_system);

    println!();
    println!("areka emo-text  typewriter デモ");
    println!("================================");
    println!("  バルーンに挨拶がタイプライター表示され、少し間を置いて打ち直します。");
    println!("  ★ バルーンを「ダブルクリック」するとデモを終了します。");
    println!();

    mgr.run()?;
    println!("[emo-text-typewriter-demo] 終了しました。おほほ、ごきげんよう！");
    Ok(())
}

fn setup_abort(msg: &str) -> ! {
    error!("{msg}");
    println!("[emo-text-typewriter-demo] セットアップ失敗: {msg}");
    std::process::exit(1);
}

fn build_and_spawn(world: &mut World) {
    let Ok(decoder) = WicDecoderArm::new() else {
        setup_abort("WicDecoderArm 生成に失敗（COM 未初期化？）");
    };
    let Some(model) = load_balloon_model() else {
        setup_abort("balloon descript の読取/解釈に失敗（共有 fixture の配置を確認）");
    };
    let balloon_dir = shared_balloon_dir();
    let Ok(assets) = build_balloon_target(&balloon_dir, &decoder, 0) else {
        setup_abort("バルーン枠アセットの構築に失敗");
    };
    // 窓寸 ≔ balloon surface0 の合成原寸（物理 px・等倍表示）。
    let (w, h) = match Composer::new().compose(
        &assets.0,
        &assets.1,
        0,
        &BindSet::default(),
        &areka_emo_compose::PatternState::default(),
    ) {
        Ok(cs) => (cs.width(), cs.height()),
        Err(e) => setup_abort(&format!("balloon surface0 の採寸合成に失敗: {e}")),
    };
    if w == 0 || h == 0 {
        setup_abort("balloon surface0 の合成外形が 0 寸");
    }

    let win = world
        .spawn((
            Name::new("Typewriter-Balloon"),
            Window {
                title: "areka emo-text typewriter demo".to_string(),
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point {
                    x: BALLOON_POS.0,
                    y: BALLOON_POS.1,
                }),
                size: Some(SizeI {
                    width: w as i32,
                    height: h as i32,
                }),
                ..Default::default()
            },
            // 窓自体はヒット対象外（透過）。バルーン本体（emo-present の surface entity・
            // 子・HitTest::alpha_mask）がダブルクリックを受け、bubble で窓の
            // OnPointerPressed へ届く。
            HitTest::none(),
            // ダブルクリック終了ハンドラ（bubble 相で double_click==Left を検出）。
            OnPointerPressed(on_balloon_pressed),
        ))
        .id();

    world.insert_non_send(Demo {
        presenter: EmoPresenter::new(),
        win,
        assets: Some(assets),
        model,
        // タイプ速度は配送 duration 由来（cue ヘルパが Text へ N×CHAR_WAIT を焼き込む）。
        // config は line_gap のみゆえ既定でよい。
        runtime: Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        attached: false,
        cycle_start: 0.0,
        fed: false,
    });
    info!(w, h, "emo-text-typewriter-demo: 窓生成とアセット構築を完了");
}

/// ダブルクリック終了ハンドラ。バルーン本体（子）が hit → bubble で窓（本ハンドラ）へ。
fn on_balloon_pressed(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // 単クリック（double_click==None）は無視し、ダブルクリックのみで終了する。
    if ev.is_bubble() && ev.value().double_click == DoubleClick::Left {
        info!("emo-text-typewriter-demo: バルーンのダブルクリックを検出 — 終了します");
        if let Some(mut q) = world.get_resource_mut::<QuitRequested>() {
            q.0 = true;
        }
        return true; // 消費
    }
    false
}

fn drive_system(world: &mut World) {
    // ダブルクリック終了要求を最優先で回収する。
    if world
        .get_resource::<QuitRequested>()
        .map(|q| q.0)
        .unwrap_or(false)
    {
        if let Some(demo) = world.remove_non_send::<Demo>() {
            world.despawn(demo.win); // registry 空遷移で run() が復帰する。
        }
        return;
    }

    let Some(mut demo) = world.remove_non_send::<Demo>() else {
        return;
    };
    if !demo.attached {
        try_attach(&mut demo, world);
    } else {
        drive_typewriter(&mut demo, world);
    }
    world.insert_non_send(demo);
}

/// GPU 資源到達フレームで attach→ShowSurface→結線を 1 回だけ行う。
fn try_attach(demo: &mut Demo, world: &mut World) {
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    let Some((emo_world, atlas)) = demo.assets.take() else {
        setup_abort("アセットが二重消費された（構造不変の破れ）");
    };
    if let Err(e) = demo.presenter.attach_target(
        world,
        TargetId(0),
        demo.win,
        emo_world,
        atlas,
        // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
        // 供給する（本 example は k=1.0 相当で従来と同一の表示寸・描画結果）。
        96,
    ) {
        setup_abort(&format!("attach_target に失敗: {e}"));
    }
    let (tx, rx) = reply_channel::<PresentOutcome>();
    demo.presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 0,
            binds: BindSet::default(),
            pattern: areka_emo_compose::PatternState::default(),
            reply: Some(tx),
        },
    );
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => {}
        other => setup_abort(&format!("ShowSurface が成立しない: {other:?}")),
    }

    let Some(view) = demo.presenter.text_slot_view(TargetId(0)) else {
        setup_abort("表示確立後の text_slot_view が None");
    };
    demo.runtime
        .borrow_mut()
        .register_actor_view(actor(), &view, &demo.model);

    // 実モニタ DPI をログ（物理 1:1・k=1.0 恒常）。
    if let Some(handle) = world.get::<WindowHandle>(demo.win) {
        info!(
            dpi = handle.get_dpi(),
            scale_k = view.scale(),
            "emo-text-typewriter-demo: 実モニタ DPI"
        );
    }

    demo.cycle_start = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);
    demo.attached = true;
    info!("emo-text-typewriter-demo: 装着・結線完了 — タイプライター開始（ダブルクリックで終了）");
}

/// 実時間タイプライター駆動（毎フレーム）。cue を流し、フレーム経過時間を注入して提示する。
fn drive_typewriter(demo: &mut Demo, world: &mut World) {
    let now = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);
    let t = now - demo.cycle_start;

    // サイクル頭で挨拶 cue を一度だけ流す（全 at=0.0＝リビールは配送 duration で連続進行）。
    if !demo.fed {
        let mut rt = demo.runtime.borrow_mut();
        rt.apply_cue(&cue(0.0, CueCommand::Text(LINE1.into())));
        rt.apply_cue(&cue(0.0, CueCommand::NewLine { ratio: 1.0 }));
        rt.apply_cue(&cue(0.0, CueCommand::Text(LINE2.into())));
        rt.apply_cue(&cue(0.0, CueCommand::NewLine { ratio: 1.0 }));
        rt.apply_cue(&cue(0.0, CueCommand::Text(LINE3.into())));
        demo.fed = true;
    }

    // 毎フレーム、フレーム経過時間 t を注入して提示（typewriter が実時間で animate）。
    {
        let mut rt = demo.runtime.borrow_mut();
        if let Err(e) = present_frame(&mut rt, world, t) {
            warn!(error = %e, "提示フレームで失敗（次フレーム再試行）");
        }
    }

    // サイクル満了で全消去し、次サイクルへ（打ち直しループ）。
    if t > CYCLE_SECS {
        demo.runtime
            .borrow_mut()
            .apply_cue(&cue(t, CueCommand::Clear));
        demo.cycle_start = now;
        demo.fed = false;
    }
}
