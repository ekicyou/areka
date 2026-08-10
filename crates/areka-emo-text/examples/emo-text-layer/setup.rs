use super::{
    BindSet, Composer, Demo, EmoPresenter, Entity, HitTest, KERO_GAP_Y, Name, Observations, Point,
    Rc, RefCell, SAKURA_POS, SizeI, TextLayerConfig, TextLayerRuntime, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE, WicDecoderArm, Window, WindowPos,
    WindowStyle, World, build_balloon_target, error, info, load_balloon_model, shared_balloon_dir,
};

// ---------------------------------------------------------------------------
// セットアップ（UI スレッド・COM 初期化済み）
// ---------------------------------------------------------------------------

/// 失敗時は log-first で真因を出し、観測不能として loud に終了する（誤 PASS を作らない）。
fn setup_abort(msg: &str) -> ! {
    error!("{msg}");
    println!("[emo-text-layer] FAIL (セットアップ失敗: {msg})");
    std::process::exit(1);
}

/// アセット構築・窓生成・`Demo` 挿入を一括で行う（UI スレッド・emo-present example と同型）。
pub(super) fn build_and_spawn(world: &mut World, vertical: bool, hold: bool) {
    let Ok(decoder) = WicDecoderArm::new() else {
        setup_abort("WicDecoderArm 生成に失敗（COM 未初期化？）");
    };

    // balloon descript（2 層マージ）: --vertical は parse 入力だけを変種へ差し替える。
    let Some(model) = load_balloon_model(vertical) else {
        setup_abort("balloon descript の読取/解釈に失敗");
    };

    // バルーン枠アセット×2（\0/\1 target）: 共有 fixture をシェルと同一経路で構築。
    let balloon_dir = shared_balloon_dir();
    let (Ok(assets0), Ok(assets1)) = (
        build_balloon_target(&balloon_dir, &decoder, 0),
        build_balloon_target(&balloon_dir, &decoder, 0),
    ) else {
        setup_abort("バルーン枠アセットの構築に失敗（共有 fixture の配置を確認）");
    };

    // 窓寸 ≔ balloon surface0 の合成原寸（物理 px・DPI 表示契約＝等倍）。
    let (w, h) = match Composer::new().compose(
        &assets0.0,
        &assets0.1,
        0,
        &BindSet::default(),
        &areka_emo_compose::PatternState::default(),
    ) {
        Ok(cs) => (cs.width(), cs.height()),
        Err(e) => {
            setup_abort(&format!("balloon surface0 の採寸合成に失敗: {e}"));
        }
    };
    if w == 0 || h == 0 {
        setup_abort("balloon surface0 の合成外形が 0 寸");
    }

    let win0 = create_balloon_window(world, "sakura", SAKURA_POS.0, SAKURA_POS.1, w, h);
    let win1 = create_balloon_window(
        world,
        "kero",
        SAKURA_POS.0,
        SAKURA_POS.1 + h as i32 + KERO_GAP_Y,
        w,
        h,
    );

    world.insert_non_send_resource(Demo {
        presenter: EmoPresenter::new(),
        win0,
        win1,
        assets0: Some(assets0),
        assets1: Some(assets1),
        model,
        attached: false,
        runtime: Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
        sink: None,
        _drain: None,
        resolved: None,
        metrics: None,
        dims: (0, 0),
        talk_start: 0.0,
        stage: 0,
        fed: false,
        obs: Observations::default(),
        finished: false,
        hold,
        prev_lbutton: false,
        last_click_time: -10.0,
    });
    info!(w, h, vertical, hold, "emo-text-layer: 窓生成とアセット構築を完了（GPU 資源到達で装着）");
}

/// バルーン窓 Entity を構築する（emo-present example の balloon 窓と同型・物理 px 採寸）。
fn create_balloon_window(
    world: &mut World,
    label: &str,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Entity {
    world
        .spawn((
            Name::new(format!("EmoText-Balloon-{label}")),
            Window {
                title: format!("areka emo-text balloon ({label})"),
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point { x, y }),
                size: Some(SizeI {
                    width: w as i32,
                    height: h as i32,
                }),
                ..Default::default()
            },
            HitTest::none(),
        ))
        .id()
}
