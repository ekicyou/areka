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
//! cargo run -p areka-emo-text --example emo-text-typewriter-demo -- --emoji   # 色つきの絵文字
//! ```
//!
//! 注: これは対話デモ（自動 pass/fail 判定は行わない）。描画・タイプライター進行・
//! ダブルクリック終了を目視確認するためのもの。既存の観測 example（emo-text-layer.rs・
//! 自動判定つき）とは別物。

use std::cell::RefCell;
use std::path::PathBuf;
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

// 文字の層の例と同じ検体の読みと組み立てを引く（写しを持たない）。子は上の `use` 束縛を
// `use super::{…};` で引く。`examples/` 直下に置くと別のターゲットになるので子のフォルダから引く。
#[path = "emo-text-layer/fixture.rs"]
mod fixture;
#[path = "emo-text-layer/setup.rs"]
mod setup;

use self::fixture::{LINE1, LINE2, LINE3, load_balloon_model, shared_balloon_dir};
use self::setup::{
    balloon_assets, balloon_model, balloon_size, create_balloon_window, decoder, gpu_ready,
    setup_abort, show_balloon,
};

/// `--emoji` のときの台詞（色つきの絵文字と、人が 1 文字と見る単位＝クラスタの目視用）。
/// 家族（ZWJ）・国旗・肌色・異体字セレクタ・キーキャップ・結合文字（か゚）と、既定フォントが持つ ♥。
const EMOJI_LINES: [&str; 3] = [
    "おはよ😀👍🏻",
    "家族👨\u{200d}👩\u{200d}👧と🇯🇵❤\u{fe0f}",
    "1\u{fe0f}\u{20e3}番か\u{309a}♥やで",
];

fn lines() -> [&'static str; 3] {
    if std::env::args().any(|a| a == "--emoji") {
        EMOJI_LINES
    } else {
        [LINE1, LINE2, LINE3]
    }
}

/// 見やすいタイプ速度（秒/グリフ・既定 0.05 より遅く）。
const CHAR_WAIT: f64 = 0.12;
/// 1 サイクル長（タイプ完了＋余韻・秒）。超えたら Clear して打ち直す。
const CYCLE_SECS: f64 = 5.0;
/// バルーン窓の初期位置（物理 px・スクリーン座標）。
const BALLOON_POS: (i32, i32) = (360, 220);

fn actor() -> ActorKey {
    ActorKey::from("0")
}
/// デモ用 cue。reveal は配送 duration 由来（`interval = duration / N`）ゆえ、Text cue へ
/// `N × CHAR_WAIT` を焼き込むと 1 グリフあたり CHAR_WAIT 秒の typewriter 進行になる
/// （全 cue at=0.0 なので reveal は連続的に累積）。
fn cue(at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => areka_sakura::cluster::cluster_count(t) as f64 * CHAR_WAIT,
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
    /// GPU 資源を待ち始めた時刻（届かないまま上限を過ぎたら準備の失敗）。
    gpu_wait_start: Option<std::time::Instant>,
    /// 現サイクルのタイプ開始フレーム時刻。
    cycle_start: f64,
    /// 現サイクルの cue を流し終えたか。
    fed: bool,
    /// 提示フレームの失敗を `error!` で残し終えたか（最初の 1 回だけ `error!`）。
    present_err_logged: bool,
}

fn main() -> windows::core::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new().inspect_err(|e| error!(error = %e, "WinApp の作成に失敗"))?;
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

    mgr.run()
        .inspect_err(|e| error!(error = %e, "メッセージループが失敗で終わった"))?;
    println!("[emo-text-typewriter-demo] 終了しました。おほほ、ごきげんよう！");
    Ok(())
}

fn build_and_spawn(world: &mut World) {
    let decoder = decoder();
    let model = balloon_model(false);
    let assets = balloon_assets(&decoder);
    let (w, h) = balloon_size(&assets);

    let win = create_balloon_window(world, "typewriter", BALLOON_POS.0, BALLOON_POS.1, w, h);
    // 窓自体は透過。バルーン本体（子・HitTest::alpha_mask）がダブルクリックを受け、
    // bubble で窓の OnPointerPressed（bubble 相で double_click==Left を検出）へ届く。
    world
        .entity_mut(win)
        .insert(OnPointerPressed(on_balloon_pressed));

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
        gpu_wait_start: None,
        cycle_start: 0.0,
        fed: false,
        present_err_logged: false,
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
    if !gpu_ready(world, &mut demo.gpu_wait_start) {
        return;
    }
    if let Err(msg) = show_balloon(
        &mut demo.presenter,
        world,
        TargetId(0),
        demo.win,
        demo.assets.take(),
    ) {
        setup_abort(&msg);
    }

    let Some(view) = demo.presenter.text_slot_view(TargetId(0)) else {
        setup_abort("表示確立後の text_slot_view が None");
    };
    demo.runtime
        .borrow_mut()
        .register_actor_view(actor(), &view, &demo.model);

    // 実モニタ DPI と合成スケール k をログ（k は窓のモニタ DPI ÷ 96・200% で 2.0）。
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
        let [line1, line2, line3] = lines();
        rt.apply_cue(&cue(0.0, CueCommand::Text(line1.into())));
        rt.apply_cue(&cue(0.0, CueCommand::NewLine { ratio: 1.0 }));
        rt.apply_cue(&cue(0.0, CueCommand::Text(line2.into())));
        rt.apply_cue(&cue(0.0, CueCommand::NewLine { ratio: 1.0 }));
        rt.apply_cue(&cue(0.0, CueCommand::Text(line3.into())));
        demo.fed = true;
    }

    // 毎フレーム、フレーム経過時間 t を注入して提示（typewriter が実時間で animate）。
    {
        let mut rt = demo.runtime.borrow_mut();
        // 最初の 1 回は `error!`（例の失敗は `error!` を残す）・以後は毎フレーム繰り返しうるので `warn!`。
        if let Err(e) = present_frame(&mut rt, world, t) {
            if demo.present_err_logged {
                warn!(error = %e, "提示フレームで失敗（次フレーム再試行）");
            } else {
                error!(error = %e, "提示フレームで失敗（次フレーム再試行・以後は warn）");
                demo.present_err_logged = true;
            }
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
