use super::{
    AtlasTable, BalloonModel, BindSet, Composer, Duration, EmoPresenter, EmoWorld, Entity,
    GraphicsCore, HitTest, Name, Point, PresentCommand, PresentOutcome, SizeI, TargetId,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE, WicDecoderArm, Window,
    WindowPos, WindowStyle, World, WucGraphicsResource, build_balloon_target, error,
    load_balloon_model, reply_channel, shared_balloon_dir,
};

// ---------------------------------------------------------------------------
// セットアップの部品（UI スレッド・COM 初期化済み）
//
// 文字の層の例とタイプライターの例が `#[path]` で共に引く。子は上の `use super::{…};` で
// 親の束縛を引くので、どちらの親もここで使う名前を `use` しておく。
// ---------------------------------------------------------------------------

/// 失敗時は log-first で真因を出し、観測不能として loud に終了する（誤 PASS を作らない）。
pub(super) fn setup_abort(msg: &str) -> ! {
    error!("{msg}");
    println!(
        "[{}] FAIL (セットアップ失敗: {msg})",
        env!("CARGO_BIN_NAME")
    );
    std::process::exit(1);
}

/// WIC のデコーダを作る（失敗は打ち切り）。
pub(super) fn decoder() -> WicDecoderArm {
    let Ok(decoder) = WicDecoderArm::new() else {
        setup_abort("WicDecoderArm 生成に失敗（COM 未初期化？）");
    };
    decoder
}

/// balloon descript（2 層マージ）を読む（失敗は打ち切り）。
pub(super) fn balloon_model(vertical: bool) -> BalloonModel {
    let Some(model) = load_balloon_model(vertical) else {
        setup_abort("balloon descript の読取/解釈に失敗（共有 fixture の配置を確認）");
    };
    model
}

/// 共有 fixture のバルーン枠アセットをシェルと同一経路で構築する（失敗は打ち切り）。
pub(super) fn balloon_assets(decoder: &WicDecoderArm) -> (EmoWorld, AtlasTable) {
    let Ok(assets) = build_balloon_target(&shared_balloon_dir(), decoder, 0) else {
        setup_abort("バルーン枠アセットの構築に失敗（共有 fixture の配置を確認）");
    };
    assets
}

/// 窓寸 ≔ balloon surface0 の合成原寸（物理 px・DPI 表示契約＝等倍・失敗は打ち切り）。
pub(super) fn balloon_size(assets: &(EmoWorld, AtlasTable)) -> (u32, u32) {
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
    (w, h)
}

/// バルーン窓 Entity を構築する（emo-present example の balloon 窓と同型・物理 px 採寸）。
///
/// 窓自体はヒット対象外（透過）。バルーン本体（emo-present の surface entity・子）が当たりを受ける。
pub(super) fn create_balloon_window(
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

/// GPU 資源の到達を待つ上限（秒）。
const GPU_WAIT_SECS: f64 = 30.0;

/// GPU 資源が届いたか（装着してよいフレームか）。
///
/// 届かないまま [`GPU_WAIT_SECS`] を過ぎたら準備の失敗として `setup_abort` で打ち切る（`--hold`
/// でも同じ・装置を作れないと装着の後の watchdog が走らず、例は何も出さずに止まらない）。
/// `waiting_since` は待ち始めた時刻で、呼び手が持つ。
pub(super) fn gpu_ready(world: &World, waiting_since: &mut Option<std::time::Instant>) -> bool {
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready
        && waiting_since
            .get_or_insert_with(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
            > GPU_WAIT_SECS
    {
        setup_abort(&format!(
            "GPU の資源（GraphicsCore／WUC）が {GPU_WAIT_SECS} 秒たっても揃わない"
        ));
    }
    ready
}

/// attach_target → ShowSurface（reply で成立を要求）。失敗は文で返し、呼び手が記録する。
pub(super) fn show_balloon(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    window: Entity,
    assets: Option<(EmoWorld, AtlasTable)>,
) -> Result<(), String> {
    let Some((emo_world, atlas)) = assets else {
        return Err(format!(
            "{target:?} のアセットが二重消費された（構造不変の破れ）"
        ));
    };
    presenter.attach_target(
        target, window, emo_world, atlas,
        // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
        // 供給する（k は窓のモニタ DPI ÷ 96・100% で 1.0、200% で 2.0）。
        96,
    );
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id: 0,
            binds: BindSet::default(),
            pattern: areka_emo_compose::PatternState::default(),
            reply: Some(tx),
        },
    );
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => Ok(()),
        other => Err(format!("{target:?} の ShowSurface が成立しない: {other:?}")),
    }
}
