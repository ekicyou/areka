//! # draw_readback_test — UI スレッド上の描画・読み戻しの通し構造検証（task 10.2）
//!
//! design.md「Testing Strategy > Integration Tests #4（描画実行）」の正典述語を、
//! **通し経路**（sink emit → UI ドレイン → `present_frame` → `read_back`）で檻化する
//! （R7.3/R8.5/R9.1/R9.3）:
//!
//! 1. **単調増加**: 可視グリフ数が増えるほど供給面の非透明ピクセルが単調に増加する。
//! 2. **Clear 後の全域透明**: `Clear` cue の通し適用（sink 経由＝`clear_cache` 込み）後、
//!    供給面は全域透明（premultiplied 全 0）へ戻る。
//! 3. **有効領域（validrect）外に非透明ピクセルなし**: design の構造（COM 層「装着」正典:
//!    物理寸＝`ceil(validrect 寸 × k)`・`Arrangement` offset＝validrect 原点 × k）により、
//!    テキストのインクは合成結果上 validrect の外に存在し得ない。これを観測可能な構造
//!    述語に落とす——(a) 供給面が validrect 寸**のみ**を覆う（`surface.size()`／readback
//!    長）、(b) 提示位置が validrect 原点に一致（`Arrangement`）、(c) 面内のインクが
//!    クランプ正準の書字開始角（validrect-local 原点側）から始まる（領域オフセットの
//!    二重適用＝面内で (36,46) へずれる変異を殺す）。インクの行矩形はみ出し（縦書きで
//!    数 px 超え得る——task 6.3 申し送り）は供給面クリップで validrect 内に閉じる。
//!
//! 6.3 の in-source 檻（draw.rs）が **DrawExecutor 単体**（layout/canvas/render を
//! テストが直結）で同種の述語を張るのに対し、本テストは結線層〜COM 層の**実配線**
//! （`spawn_emo_text` の実 pump ドレイン・`present_frame` の装着/描画/提示）を通す。
//!
//! - GPU/WUC は headless 実資源（`GraphicsCore::new()`・WARP 可）＝ attach_wiring_test
//!   と同一方針。時刻は常に注入（`talk_time`）・実時間 sleep 不使用（決定論）。
//! - R9.1: 横書きテストは実 `EmoPresenter` の公開経路（`text_slot_view` →
//!   `register_actor_view`）で予約スロットへ到達する。
//! - R9.3: グリフ更新フレームは供給面の Present のみで完結する（World の entity 数
//!   不変・`GraphicsCommandList` 不挿入＝バルーン surface 本体の再合成を強要しない）。
//! - R8.5: M1 の実挙動はテキスト描画のみ＝グリフ未リビール時（可視 0）と Clear 後に
//!   インク源が存在しない（Image/Surface シーム住人の inert は draw.rs 6.3 檻が正典・
//!   本テストはその通し側の帰結「テキスト以外のインク源なし」を assert する）。

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::{EmoPresenter, PresentCommand, PresentOutcome, TargetId};
use areka_emo_text::actor::{
    ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame, spawn_emo_text,
};
use areka_emo_text::state::TextLayerConfig;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{
    Arrangement, DPI, GraphicsCommandList, GraphicsCore, Visual, VisualGraphics,
    WucGraphicsResource,
};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

// ── 幾何定数（fixture 実測値系の非退化 validrect——負値=反対辺の実解決を通す） ──────────

/// バルーン画像原寸（image px・k=1.0 恒常ゆえ物理原寸と同値）。
const IMAGE: (u32, u32) = (200, 150);
/// validrect 定義（top 46 / bottom -56 / left 36 / right -44——fixture 正準の実値）。
/// 解決結果: 絶対 (36,46)-(156,94) ＝ 原点 (36,46)・寸 (120,48)。
const VR_ORIGIN: (f32, f32) = (36.0, 46.0);
const VR_SIZE: (u32, u32) = (120, 48);
/// 既定フォント高さ 12（image px）・行送り pitch = ceil(12 × 1.25) = 15。
const FONT_H: u32 = 12;
const PITCH: u32 = 15;

/// 非退化 validrect 付きのテスト用 BalloonModel。origin (0,0) は validrect 外＝
/// クランプ正準で書字開始角へ寄る（region.rs の fixture 檻と同じ正準経路）。
/// wordwrap 未指定＝折返し閾値は行末辺（横=right 156／縦=bottom 94）。
fn validrect_model(writing_mode: Option<&str>) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(46), Some(-56), Some(36), Some(-44)),
        Font::new(None, None, FontColor::new(None, None, None)),
        writing_mode.map(str::to_owned),
        None,
    )
}

// ── GPU/WUC フィクスチャ（attach_wiring_test と同一方針・headless） ─────────────────────

/// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World を組む。
/// 本番 UI スレッドは MTA（記憶: areka WUC は MTA スレッドで動く）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

// ── 実 EmoPresenter の合成入力（attach_wiring_test と同技法・上流公開 API で本物を組む） ──

fn elem(path: &str, x: i64, y: i64) -> Element {
    Element {
        layer: 0,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

fn shell_of(surfaces: Vec<Surface>) -> Shell {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    }
}

/// surface 1000 = 単一 element（`w×h` 全不透明）の `(EmoWorld, AtlasTable)`
/// （バルーン枠画像に相当する合成入力）。
fn build_target_assets(w: u32, h: u32) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![Surface {
        id: 1000,
        targets: vec![AppendTarget::Single(1000)],
        elements: vec![elem("p.png", 0, 0)],
        collisions: Vec::new(),
        animations: Vec::new(),
    }];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            img.push((x as u8).wrapping_mul(3));
            img.push((y as u8).wrapping_mul(5));
            img.push(((x + y) as u8).wrapping_mul(7));
            img.push(0xFF);
        }
    }
    dec.insert(base.join("p.png"), w, h, stride, img, true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない"
    );

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// `ShowSurface` を適用して表示を確立する（reply Ok を要求）。
fn show_surface(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: areka_emo_compose::PatternState::default(),
            reply: Some(tx),
        },
    );
    let outcome = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("reply（ShowSurface）を受信できない");
    assert!(matches!(outcome, Ok(())), "ShowSurface は Ok: {outcome:?}");
}

// ── 共通補助 ─────────────────────────────────────────────────────────────────────────

/// reveal 間隔（秒/グリフ）。reveal は配送 duration 由来（`interval = duration / N`）ゆえ、
/// Text cue へ `N × REVEAL_INTERVAL` を焼き込むと interval=0.05 の per-glyph 進行を得る
/// （旧 char_wait=0.05 既定と機能等価・進行観測は安全マージン付き時刻 0.11/0.16/… で行う）。
const REVEAL_INTERVAL: f64 = 0.05;

/// テスト用 cue。Text cue には配送 duration = `N × REVEAL_INTERVAL` を焼き込み
/// （reveal interval=0.05）、他コマンドは瞬時（duration=0）。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => t.chars().count() as f64 * REVEAL_INTERVAL,
        _ => 0.0,
    };
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration,
    }
}

/// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
/// （WM_QUIT は posted メッセージが尽きた後に配送——sink.rs/attach_wiring の決定論
/// パターン。sink 生存中は複数セッション実行できる＝actor.rs 檻ケース 3 の前例）。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0）。
fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 非透明ピクセルの外接矩形 (min_x, min_y, max_x, max_y)（インクなしは None）。
fn ink_bounds(bytes: &[u8], width: u32) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for (i, px) in bytes.chunks_exact(4).enumerate() {
        if px[3] != 0 {
            let (x, y) = (i as u32 % width, i as u32 / width);
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bounds
}

/// 装着済み actor の供給面 readback。
fn read_back(rt: &TextLayerRuntime, actor: &ActorKey) -> Vec<u8> {
    rt.surface(actor)
        .expect("装着済み actor の供給面")
        .read_back()
        .expect("read_back")
}

/// 供給面と提示位置の validrect 封じ込め構造（述語 3 の (a)(b)——design COM 層「装着」正典:
/// 物理寸＝ceil(validrect 寸 × k)・Arrangement offset＝validrect 原点 × k・k=1.0）。
/// 供給面が validrect **のみ**を覆い validrect 原点へ提示される＝合成結果上、テキストの
/// インクは validrect 外に存在し得ない（面外のインクは供給面クリップで消える）。
fn assert_validrect_containment(
    rt: &TextLayerRuntime,
    actor: &ActorKey,
    world: &World,
    slot: bevy_ecs::entity::Entity,
) {
    let surface = rt.surface(actor).expect("装着済み actor の供給面");
    assert_eq!(
        surface.size(),
        VR_SIZE,
        "供給面の物理寸は validrect 寸のみ（画像原寸 {IMAGE:?} ではない）"
    );
    let bytes = surface.read_back().expect("read_back");
    assert_eq!(
        bytes.len(),
        (VR_SIZE.0 * VR_SIZE.1 * 4) as usize,
        "readback は validrect 寸の BGRA 密配列＝validrect 外の画素は供給面に存在しない"
    );
    let arr = world
        .get::<Arrangement>(slot)
        .expect("slot に Arrangement（物理 px 直接）が装着される");
    assert_eq!(
        (arr.offset.x, arr.offset.y),
        VR_ORIGIN,
        "提示位置は validrect 原点 × k（k=1.0）＝供給面全体が validrect に一致する"
    );
    assert_eq!(
        (arr.size.width, arr.size.height),
        (VR_SIZE.0 as f32, VR_SIZE.1 as f32),
        "提示寸は validrect 寸 × k（k=1.0）"
    );
}

// ══ 横書き: 3 述語の通し檻（実 EmoPresenter 公開経路・R7.3/R8.5/R9.1/R9.3） ══════════════

/// 観測可能な完了状態（task 10.2・横書き）: 実 sink→UI ドレイン→present_frame→read_back の
/// 通し経路で、(1) 可視グリフ数増加に伴う非透明ピクセルの単調増加、(2) Clear 後の全域
/// 透明化、(3) 有効領域（validrect）外に非透明ピクセルが無いこと（封じ込め構造＋面内
/// 開始角）の 3 述語が green である。併せてスクロールの全域再描画（R7.3・残渣なし・
/// 決定論）と Present 完結（R9.3・World 非再合成）を檻化する。
#[test]
fn full_path_ink_monotonic_clear_transparent_and_contained_in_validrect() {
    let mut world = make_world_with_gpu();

    // ── R9.1: 実 EmoPresenter の公開経路（text_slot_view）で予約スロットへ到達する ──
    // 窓 entity には `DPI` component を**明示的に**載せる（areka-P0-emo-dpi-scaling task 3.2）。
    // component 不在は `derive_scale` の縮退分岐（error! ＋ k=1.0）を通るため、不在に頼ると
    // 縮退が正常経路になりすまして観測できない。作者基準 DPI 96 と揃えて k=1.0 を成立させる。
    let window = world.spawn(DPI::from_dpi(96, 96)).id();
    let (emo_world, atlas) = build_target_assets(IMAGE.0, IMAGE.1);
    let mut presenter = EmoPresenter::new();
    presenter
        // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
        // 供給する（本テストは窓 DPI 96 と揃えて k=1.0＝従来と同一の表示寸・描画結果）。
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_surface(&mut presenter, &mut world, TargetId(0));
    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    assert_eq!(
        view.surface_size(),
        IMAGE,
        "バルーン原寸＝画像原寸（k=1.0）"
    );
    let slot = view.slot();

    let actor = ActorKey::from("0");
    let model = validrect_model(None);
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    runtime
        .borrow_mut()
        .register_actor_view(actor.clone(), &view, &model);

    // ── セッション 1: 5 グリフ（at=0.1）を実 sink→UI ドレインで状態機械へ ──
    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
    sink.emit(cue("0", 0.1, CueCommand::Text("あいうえお".into())));
    pump_until_idle();

    // 装着フレーム（t=0.0 ＜ at=0.1 → 可視 0）: 装着は成立し、供給面は全域透明。
    // R8.5 の通し側: 実配線のインク源はテキストのみ＝グリフ未リビールなら面は空。
    {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 0.0).expect("装着フレーム（可視 0）");
        assert!(
            rt.is_attached(&actor),
            "cue 到着済み actor は可視 0 でも装着される"
        );
    }
    let entities_after_attach = world.entities().len();
    {
        let rt = runtime.borrow();
        let bytes = read_back(&rt, &actor);
        assert!(
            bytes.iter().all(|&b| b == 0),
            "可視 0 グリフの供給面は全域透明（テキスト以外のインク源が無い・R8.5 通し側）"
        );
        // ── 述語 3（構造）: validrect 封じ込め＋装着契約（R9.1/R9.3） ──
        assert_validrect_containment(&rt, &actor, &world, slot);
        let vg = world
            .get::<VisualGraphics>(slot)
            .expect("slot に VisualGraphics（emo 自前 brush）が装着される");
        assert!(vg.is_valid(), "装着された brush は有効");
        assert!(
            world.get::<GraphicsCommandList>(slot).is_none(),
            "GraphicsCommandList は挿入しない（wintf 描画系と競合しない・R9.3 構造）"
        );
    }

    // ── 述語 1: 注入時刻フレームでの単調増加（r = [0.1, 0.15, 0.20, 0.25, 0.30]） ──
    let mut counts = vec![0usize];
    for t in [0.11, 0.16, 0.21, 0.26, 0.31] {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, t).expect("リビール進行フレーム");
        drop(rt);
        counts.push(opaque_count(&read_back(&runtime.borrow(), &actor)));
    }
    for i in 1..counts.len() {
        assert!(
            counts[i] > counts[i - 1],
            "非透明ピクセルは可視グリフ数とともに単調増加する（通し経路）: {counts:?}"
        );
    }

    // ── 述語 3（面内 (c)）: インクはクランプ正準の書字開始角（validrect-local 原点側）
    //    から始まる——領域オフセットの二重適用（面内で (36,46) へずれ、下端の内容が
    //    validrect の外相当へ押し出される変異）を殺す ──
    {
        let rt = runtime.borrow();
        let (min_x, min_y, max_x, max_y) =
            ink_bounds(&read_back(&rt, &actor), VR_SIZE.0).expect("インクが描かれる");
        assert!(
            min_x < FONT_H && min_y < PITCH,
            "インクは書字開始角（validrect-local 原点側）から始まる: min=({min_x},{min_y})"
        );
        assert!(
            max_x < VR_SIZE.0 && max_y < VR_SIZE.1,
            "インクは供給面＝validrect 内に閉じる: max=({max_x},{max_y})"
        );
    }

    // ── セッション 2: 行 1〜2 を追加（3 行＝validrect 高 48 に収まる上限） ──
    sink.emit(cue("0", 1.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 1.0, CueCommand::Text("■".into())));
    sink.emit(cue("0", 1.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 1.0, CueCommand::Text("■".into())));
    pump_until_idle();
    let before = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 5.0).expect("全リビール済みフレーム（3 行）");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(
        opaque_count(&before) > counts[5],
        "追加行のインクが載る（あふれ前・3 行とも可視）"
    );
    // 決定論: 同一注入時刻の再提示はバイト一致（全域再描画に差分累積なし・R7.3）。
    {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 5.0).expect("同一時刻の再提示フレーム");
        drop(rt);
        assert_eq!(
            read_back(&runtime.borrow(), &actor),
            before,
            "同一入力→同一ピクセル（通し経路の決定論）"
        );
    }

    // ── R7.3: 行 3 追加であふれ→スクロール＝validrect 寸キャンバスへの全域再描画 ──
    sink.emit(cue("0", 6.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 6.0, CueCommand::Text("■".into())));
    pump_until_idle();
    let after = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 10.0).expect("あふれ後フレーム");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(
        opaque_count(&after) < opaque_count(&before),
        "スクロールで先頭行（5 グリフ）が全域再描画により消える（残渣なし）: before={} after={}",
        opaque_count(&before),
        opaque_count(&after)
    );
    let (_, min_y, _, max_y) = ink_bounds(&after, VR_SIZE.0).expect("スクロール後もインクあり");
    assert!(
        min_y < PITCH,
        "可視窓の先頭行は領域先頭へ詰めて描かれる（min_y={min_y}）"
    );
    assert!(
        max_y < VR_SIZE.1,
        "スクロール後もインクは validrect 内に閉じる"
    );

    // ── 述語 2: Clear の通し適用（sink 経由＝clear_cache 込み）→ 全域透明 ──
    sink.emit(cue("0", 11.0, CueCommand::Clear));
    sink.close();
    pump_until_idle();
    {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 12.0).expect("Clear 後フレーム");
        drop(rt);
        let rt = runtime.borrow();
        let bytes = read_back(&rt, &actor);
        assert_eq!(
            bytes.len(),
            (VR_SIZE.0 * VR_SIZE.1 * 4) as usize,
            "Clear 後も供給面は validrect 寸のまま"
        );
        assert!(
            bytes.iter().all(|&b| b == 0),
            "Clear 後は全域が透明（premultiplied 全 0）へ戻る"
        );
    }

    // ── R9.3: 装着後の全フレーム（進行・スクロール・Clear）は Present のみで完結 ──
    assert_eq!(
        world.entities().len(),
        entities_after_attach,
        "グリフ更新フレームは World に entity を追加しない（供給面の提示のみ・R9.3）"
    );
    assert!(
        world.get::<GraphicsCommandList>(slot).is_none(),
        "全フレーム経過後も GraphicsCommandList は不挿入のまま（surface 再合成を強要しない）"
    );
}

// ══ 縦書き（vertical_rl）変種: 3 述語＋横方向スクロール（R6 系の軸切替を通し経路で） ══════

/// emo-present `VisualMount` と同型の予約スロット（surface.rs/actor.rs テストと同型・
/// R9.1 公開経路は横書きテストが檻化済みのため、縦書き変種は軽量 rig で通しを張る）。
fn spawn_reserved_slot(world: &mut World) -> (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-layer-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();
    (window, slot)
}

/// 観測可能な完了状態（task 10.2・縦書き変種）: 縦書き（vertical_rl）でも 3 述語が
/// green である——(1) 単調増加、(2) Clear 後全域透明、(3) validrect 封じ込め（縦書きの
/// インク行矩形はみ出し——task 6.3 申し送り——も供給面クリップで validrect 内に閉じる）。
/// 列は validrect 右端から左へ流れ、あふれは横方向スクロール（先頭列の消失）になる。
#[test]
fn vertical_rl_full_path_reveals_scrolls_horizontally_and_clears_within_validrect() {
    let mut world = make_world_with_gpu();
    let (window, slot) = spawn_reserved_slot(&mut world);

    let actor = ActorKey::from("0");
    let model = validrect_model(Some("vertical_rl"));
    let binding = TextSlotBinding::new(slot, window, 1.0, IMAGE);
    let resolved = ResolvedBalloonText::resolve(&model, binding.image_size);
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    runtime
        .borrow_mut()
        .register_actor(actor.clone(), binding, resolved);

    // ── セッション 1: 列 0＝■■（at=0.0）＋列 1〜7＝■ 各 1（at=0.5）
    //    → 8 列 × pitch 15 = 120 ＝ validrect 幅ちょうど（あふれ前の上限） ──
    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
    sink.emit(cue("0", 0.0, CueCommand::Text("■■".into())));
    for _ in 0..7 {
        sink.emit(cue("0", 0.5, CueCommand::NewLine { ratio: 1.0 }));
        sink.emit(cue("0", 0.5, CueCommand::Text("■".into())));
    }
    pump_until_idle();

    // ── 述語 1（縦書き）: r = [0.0, 0.05] → t=0.0 で 1 グリフ・t=0.06 で 2 グリフ ──
    let n1 = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 0.0).expect("縦書き 1 グリフ目フレーム");
        drop(rt);
        opaque_count(&read_back(&runtime.borrow(), &actor))
    };
    assert!(n1 > 0, "縦書き 1 グリフ目のインクが載る");
    // 書字開始角＝validrect 右上（軸読み替え正準表）: 列 0 は面の右端側
    // （x ∈ [幅−pitch, 幅)）に現れる——横書き配置（左端開始）との取り違えを殺す。
    {
        let rt = runtime.borrow();
        let (min_x, min_y, max_x, max_y) =
            ink_bounds(&read_back(&rt, &actor), VR_SIZE.0).expect("インクあり");
        assert!(
            min_x >= VR_SIZE.0 - PITCH - FONT_H,
            "縦書きの列 0 は validrect-local 右端側から始まる（min_x={min_x}）"
        );
        assert!(
            min_y < PITCH,
            "列内は上→下＝1 グリフ目は上端側（min_y={min_y}）"
        );
        // 述語 3: インクの行矩形はみ出し（縦書きで数 px 超え得る）も供給面クリップで
        // validrect 内に閉じる——readback 外接が面内で完結する。
        assert!(
            max_x < VR_SIZE.0 && max_y < VR_SIZE.1,
            "縦書きインクは供給面＝validrect 内に閉じる: max=({max_x},{max_y})"
        );
    }
    let n2 = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 0.06).expect("縦書き 2 グリフ目フレーム");
        drop(rt);
        opaque_count(&read_back(&runtime.borrow(), &actor))
    };
    assert!(
        n2 > n1,
        "縦書きでも非透明ピクセルは可視グリフ数とともに増加する: {n1} -> {n2}"
    );

    // 全リビール（8 列・■×9）＝あふれ前の基準。述語 3 の封じ込め構造も縦書きで成立。
    let before = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 10.0).expect("全リビール済みフレーム（8 列）");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(opaque_count(&before) > n2, "8 列分のインクが載る");
    {
        let rt = runtime.borrow();
        assert_validrect_containment(&rt, &actor, &world, slot);
        let (min_x, _, max_x, _) = ink_bounds(&before, VR_SIZE.0).expect("インクあり");
        assert!(
            min_x < PITCH && max_x >= VR_SIZE.0 - PITCH - FONT_H,
            "列は右端（列 0）から左端（列 7）まで右→左に流れる: min_x={min_x} max_x={max_x}"
        );
    }

    // ── R7.3×縦書き: 列 8 追加であふれ→横方向スクロール（先頭列＝■■ の消失） ──
    sink.emit(cue("0", 11.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 11.0, CueCommand::Text("■".into())));
    pump_until_idle();
    let after = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 20.0).expect("あふれ後フレーム（縦書き）");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(
        opaque_count(&after) < opaque_count(&before),
        "横方向スクロールで先頭列（■■）が全域再描画により消える: before={} after={}",
        opaque_count(&before),
        opaque_count(&after)
    );
    let (_, _, max_x, max_y) = ink_bounds(&after, VR_SIZE.0).expect("スクロール後もインクあり");
    assert!(
        max_x >= VR_SIZE.0 - PITCH - FONT_H,
        "可視窓の先頭列は右端へ詰めて描かれる（max_x={max_x}）"
    );
    assert!(
        max_x < VR_SIZE.0 && max_y < VR_SIZE.1,
        "スクロール後も縦書きインクは validrect 内に閉じる"
    );

    // ── 述語 2（縦書き）: Clear の通し適用 → 全域透明 ──
    sink.emit(cue("0", 21.0, CueCommand::Clear));
    sink.close();
    pump_until_idle();
    let mut rt = runtime.borrow_mut();
    present_frame(&mut rt, &mut world, 22.0).expect("Clear 後フレーム（縦書き）");
    drop(rt);
    let bytes = read_back(&runtime.borrow(), &actor);
    assert_eq!(bytes.len(), (VR_SIZE.0 * VR_SIZE.1 * 4) as usize);
    assert!(
        bytes.iter().all(|&b| b == 0),
        "縦書きでも Clear 後は全域が透明へ戻る"
    );
}
