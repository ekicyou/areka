//! # attach_wiring_test — 予約スロットへの装着配線の統合テスト（task 8・R9.1/R9.2/R9.5）
//!
//! 2 つの target（sakura/kero 相当）を実 `EmoPresenter` に登録して表示を確立し、
//! `text_slot_view(target)`（emo-present の読み取り専用 additive 増分・R9.1/R9.2）で得た
//! view を結線 API（`register_actor_view`——view → `TextSlotBinding` → routing 登録）で
//! actor（\0／\1）へ振り分け（R9.5）、各 actor のテキストが対応するバルーンの予約スロット
//! へ**独立に**描画されることを readback で検証する。
//!
//! - emo-present は読み取り消費のみ（TextSlotView 経由）——本テストは emo-present を改変しない。
//! - GPU/WUC は headless 実資源（`GraphicsCore::new()`・WARP 可・presenter.rs テストと同一方針）。
//! - 時刻は常に注入（`talk_time`）・実時間 sleep 不使用（決定論）。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
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
use areka_emo_text::layout::{FixedMetrics, LayoutEngine, WrapPlan};
use areka_emo_text::state::TextLayerConfig;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use tracing::subscriber::Interest;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{DPI, GraphicsCommandList, GraphicsCore, VisualGraphics, WucGraphicsResource};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

// ── GPU/WUC フィクスチャ（emo-present presenter.rs テストと同一方針） ──────────────────

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

// ── 合成入力の生成補助（presenter.rs テストと同技法・上流公開 API で本物を組む） ─────────

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

/// surface 1000 = 単一 element（`w×h` 全不透明・座標由来グラデーション）の
/// `(EmoWorld, AtlasTable)` を返す（バルーン枠画像に相当する合成入力）。
fn build_target_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
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
            img.push((x as u8).wrapping_mul(3).wrapping_add(salt));
            img.push((y as u8).wrapping_mul(5).wrapping_add(salt));
            img.push(((x + y) as u8).wrapping_mul(7).wrapping_add(salt));
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

/// sakura/kero 相当の 2 target（原寸が異なる 2 バルーン）を表示確立まで組む。
/// 返り値: (presenter, window0, window1)。原寸は target0=(140,80)・target1=(120,60)。
fn setup_two_targets(world: &mut World) -> (EmoPresenter, Entity, Entity) {
    // 窓 entity には `DPI` component を**明示的に**載せる（areka-P0-emo-dpi-scaling task 3.2）。
    // component 不在は `derive_scale` の縮退分岐（error! ＋ k=1.0）を通るため、不在に頼ると
    // 縮退が正常経路になりすまして観測できない。作者基準 DPI 96 と揃えて k=1.0 を成立させる。
    let window0 = world.spawn(DPI::from_dpi(96, 96)).id();
    let window1 = world.spawn(DPI::from_dpi(96, 96)).id();

    let (emo_world0, atlas0) = build_target_assets(140, 80, 0x11);
    let (emo_world1, atlas1) = build_target_assets(120, 60, 0x22);

    let mut presenter = EmoPresenter::new();
    presenter
        // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
        // 供給する（本テストは窓 DPI 96 と揃えて k=1.0＝従来と同一の表示寸・描画結果）。
        .attach_target(world, TargetId(0), window0, emo_world0, atlas0, 96)
        .expect("attach_target(0) 失敗");
    presenter
        .attach_target(world, TargetId(1), window1, emo_world1, atlas1, 96)
        .expect("attach_target(1) 失敗");
    show_surface(&mut presenter, world, TargetId(0));
    show_surface(&mut presenter, world, TargetId(1));
    (presenter, window0, window1)
}

// ── k≠1 の単一 target フィクスチャ（task 7.3・R8.2 の `from_view` 檻用） ─────────────────

/// k≠1 檻の native 原寸（作者画像空間・k 不変であるべき寸）。
///
/// 140/80 とも偶数ゆえ k=2 では `scaled_extent` が端数を作らず、`round(physical / k)` が
/// native へ**厳密**に戻る（本檻が問うのは丸めではなく「どちらの寸を読むか」の向きゆえ、
/// 端数を混ぜて論点をぼかさない。端数丸めの檻は in-crate `binding_derives_image_size_*` が所有）。
const SCALED_NATIVE: (u32, u32) = (140, 80);

/// 窓 DPI を指定して balloon 相当の単一 target（native 原寸 [`SCALED_NATIVE`]）を表示確立まで組む。
///
/// 作者基準 DPI は正典既定の 96（ukadoc・D1）ゆえ **k = window_dpi / 96**（192 → k=2）。
/// `setup_two_targets` は k=1.0 に固定されており、k=1 では `surface_size()` と `physical_size()`
/// が恒等で**取り違えが数値的に観測できない**——本フィクスチャはその盲点を潰すためにある。
fn setup_scaled_target(world: &mut World, window_dpi: u16) -> (EmoPresenter, Entity) {
    let window = world.spawn(DPI::from_dpi(window_dpi, window_dpi)).id();
    let (emo_world, atlas) = build_target_assets(SCALED_NATIVE.0, SCALED_NATIVE.1, 0x33);
    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target(0) 失敗");
    show_surface(&mut presenter, world, TargetId(0));
    (presenter, window)
}

// ── ログ捕捉ハーネス（R8.7 の檻・`crates/areka/src/placement/test_support.rs` の硬化版を移植） ──
//
// **素朴な `tracing::subscriber::with_default` だけでは非決定的に取りこぼす**（再現率 12%）。
// `with_default` はスレッドローカルだが callsite の interest キャッシュは**プロセス大域**で、
// 「最初に踏んだスレッドが勝つ」——subscriber を持たない別テストが当該 callsite を先に踏むと
// `NoSubscriber` → `Interest::never()` が焼き付き、以後イベントが捨てられる。対策は
// (1) probe dispatcher を **2 個**常駐させて `has_just_one` を恒久的に偽へ落とし、
// (2) 捕捉窓の**内側**で `rebuild_interest_cache()` を 1 回叩いて焼き残りを解毒すること。
// 単体実行や `--test-threads=1` では 0% ＝素朴な検証では絶対に見つからない毒ゆえ、
// 先例（areka placement / emo-compose log_capture）と同一の構造をここでも採る。

/// 捕捉した 1 イベント（level ＋ フィールド名 → Debug 表現）。
#[derive(Debug, Clone)]
struct LogEvent {
    level: tracing::Level,
    fields: BTreeMap<String, String>,
}

impl LogEvent {
    /// `message` フィールド（本文）。無ければ空文字（panic しない）。
    fn message(&self) -> &str {
        self.fields.get("message").map(String::as_str).unwrap_or("")
    }

    /// 構造化フィールドの Debug 表現。**欠落は失敗**（フィールド名も 7.4 の判定契約のうち）。
    fn field(&self, name: &str) -> &str {
        self.fields
            .get(name)
            .unwrap_or_else(|| panic!("ログフィールド `{name}` が無い: {:?}", self.fields))
    }
}

/// 全フィールドを Debug 表現で拾う visitor（`record_u64`／`record_f64`／`record_str` の
/// 既定実装がすべて `record_debug` へ転送するため 1 本で型を問わず捕捉できる）。
struct FieldGrab<'a>(&'a mut BTreeMap<String, String>);

impl tracing::field::Visit for FieldGrab<'_> {
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        self.0.insert(f.name().to_string(), format!("{v:?}"));
    }
}

/// イベントを溜めるだけの最小 subscriber。
#[derive(Clone, Default)]
struct CaptureSubscriber(Arc<Mutex<Vec<LogEvent>>>);

impl tracing::Subscriber for CaptureSubscriber {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = BTreeMap::new();
        event.record(&mut FieldGrab(&mut fields));
        self.0
            .lock()
            .expect("捕捉バッファの毒化なし")
            .push(LogEvent {
                level: *event.metadata().level(),
                fields,
            });
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// interest キャッシュへ `never` を焼かせないための常駐 dispatcher
/// （`register_callsite` が常に `sometimes` を返すことだけが仕事）。
struct InterestProbe;

impl tracing::Subscriber for InterestProbe {
    fn register_callsite(&self, _meta: &'static tracing::Metadata<'static>) -> Interest {
        Interest::sometimes()
    }
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        false
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, _event: &tracing::Event<'_>) {}
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// probe dispatcher を **2 個**プロセス寿命で常駐させる（冪等）。
/// 2 個必要なのは `has_just_one = (len <= 1)` ゆえ——1 個では毒の経路が残る。
fn ensure_interest_probes() {
    static PROBES: OnceLock<(tracing::Dispatch, tracing::Dispatch)> = OnceLock::new();
    PROBES.get_or_init(|| {
        (
            tracing::Dispatch::new(InterestProbe),
            tracing::Dispatch::new(InterestProbe),
        )
    });
}

/// クロージャ実行中に**現在のスレッド**で発火した tracing イベントを戻り値と共に返す。
fn capture_logs<R, F: FnOnce() -> R>(f: F) -> (R, Vec<LogEvent>) {
    ensure_interest_probes();
    let cap = CaptureSubscriber::default();
    let sink = cap.0.clone();
    let out = tracing::subscriber::with_default(cap, || {
        // probe 常駐前に焼かれた `never` の掃き残しを、窓が開いた**後**に確定的に潰す。
        tracing::callsite::rebuild_interest_cache();
        f()
    });
    let events = sink.lock().expect("捕捉バッファの毒化なし").clone();
    (out, events)
}

/// メッセージに `needle` を含むイベントが**ちょうど 1 件**在ることを主張して返す。
fn expect_one<'a>(events: &'a [LogEvent], needle: &str) -> &'a LogEvent {
    let hits: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "`{needle}` を含むログがちょうど 1 件ではない: {events:?}"
    );
    hits[0]
}

// ── 共通補助 ─────────────────────────────────────────────────────────────────────────

/// テスト用 cue。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

/// テスト用 BalloonModel（幾何は既定: validrect 未指定＝画像全域・font 未指定＝既定）。
fn geo_model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
/// （actor.rs 結線テストの決定論パターン——WM_QUIT は posted メッセージが尽きた後に配送）。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0）。
fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 装着済み actor の供給面 readback。
fn read_back(rt: &TextLayerRuntime, actor: &ActorKey) -> Vec<u8> {
    rt.surface(actor)
        .expect("装着済み actor の供給面")
        .read_back()
        .expect("read_back")
}

// ══ 観測可能な完了状態: 2 target 結線シナリオで actor 別バルーンへ独立描画（R9.5） ══

/// \0/\1 の 2 actor を sakura/kero 相当の 2 target へ振り分けて装着し、
/// (1) 実 sink→UI ドレイン→状態→フレーム提示の通し結線で両バルーンにインクが載る、
/// (2) 各 actor の供給面が対応 target のバルーン原寸（validrect 全域）に一致する、
/// (3) 両スロットに有効 brush が装着され GraphicsCommandList は不挿入（R9.1/R9.3 構造）、
/// (4) \0 への後続 cue が \1 の描画へ一切波及しない（独立性・バイト同一）、
/// (5) 装着経路が行レイアウト（PositionedLine＝クリック可能範囲の素材）を
///     choice-render が再利用できる構造のまま保つ（R9.4 シーム）——を檻化する。
#[test]
fn two_actors_are_routed_to_their_own_targets_and_draw_independently() {
    let mut world = make_world_with_gpu();
    let (presenter, window0, window1) = setup_two_targets(&mut world);

    // ── R9.1/R9.2: 公開経路（text_slot_view）で予約スロットへ到達する ──
    let view0 = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view(0) は Some");
    let view1 = presenter
        .text_slot_view(TargetId(1))
        .expect("表示確立後の text_slot_view(1) は Some");
    assert_ne!(
        view0.slot(),
        view1.slot(),
        "target ごとに独立の予約スロット"
    );
    assert_eq!(view0.window(), window0);
    assert_eq!(view1.window(), window1);
    assert_eq!(view0.surface_size(), (140, 80));
    assert_eq!(view1.surface_size(), (120, 60));

    // ── R9.5: 結線（actor→target 対応は結線側所有・view→binding→routing 登録） ──
    let sakura = ActorKey::from("0");
    let kero = ActorKey::from("1");
    let model = geo_model();
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    runtime
        .borrow_mut()
        .register_actor_view(sakura.clone(), &view0, &model);
    runtime
        .borrow_mut()
        .register_actor_view(kero.clone(), &view1, &model);

    // ── 通し結線: sink emit（\0/\1 交互）→ UI ドレイン → 状態機械 ──
    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
    sink.emit(cue("0", 0.0, CueCommand::Text("アヒルやアヒル".into())));
    sink.emit(cue("1", 0.0, CueCommand::Text("ふむ".into())));
    sink.close();
    pump_until_idle();

    // ── フレーム提示: 両 actor が各自の予約スロットへ装着され、独立に描画される ──
    {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 1.0).expect("提示フレーム（全リビール済み時刻）");
        assert!(
            rt.is_attached(&sakura),
            "\\0 は target0 のスロットへ装着される"
        );
        assert!(
            rt.is_attached(&kero),
            "\\1 は target1 のスロットへ装着される"
        );
    }
    let rt = runtime.borrow();

    // 各供給面の readback 寸が対応 target のバルーン原寸（validrect 全域・k=1.0）と一致
    // ＝ actor→target の振り分けが原寸レベルで正しい。
    let bytes0 = read_back(&rt, &sakura);
    let bytes1 = read_back(&rt, &kero);
    assert_eq!(
        bytes0.len(),
        (140 * 80 * 4) as usize,
        "\\0 の供給面は target0 原寸"
    );
    assert_eq!(
        bytes1.len(),
        (120 * 60 * 4) as usize,
        "\\1 の供給面は target1 原寸"
    );
    let ink0 = opaque_count(&bytes0);
    let ink1 = opaque_count(&bytes1);
    assert!(ink0 > 0, "\\0 のテキストが対応バルーンへ描画される");
    assert!(ink1 > 0, "\\1 のテキストが対応バルーンへ描画される");

    // 装着の構造契約（R9.1/R9.3）: 両スロットに有効 brush・GraphicsCommandList 不挿入。
    for (name, slot) in [("slot0", view0.slot()), ("slot1", view1.slot())] {
        let vg = world
            .get::<VisualGraphics>(slot)
            .unwrap_or_else(|| panic!("{name} に VisualGraphics（自前 brush）が装着される"));
        assert!(vg.is_valid(), "{name} の brush は有効");
        assert!(
            world.get::<GraphicsCommandList>(slot).is_none(),
            "{name} に GraphicsCommandList は挿入しない（wintf 描画系と競合しない）"
        );
    }
    drop(rt);

    // ── 独立性: \0 への後続 cue は \1 の描画へ一切波及しない ──
    {
        let mut rt = runtime.borrow_mut();
        rt.apply_cue(&cue("0", 1.0, CueCommand::Text("追記も出る".into())));
        present_frame(&mut rt, &mut world, 2.0).expect("追記後フレーム");
    }
    let rt = runtime.borrow();
    let ink0_after = opaque_count(&read_back(&rt, &sakura));
    assert!(
        ink0_after > ink0,
        "\\0 の追記でインクが増える: {ink0} -> {ink0_after}"
    );
    assert_eq!(
        read_back(&rt, &kero),
        bytes1,
        "\\0 の更新は \\1 の供給面へ波及しない（バイト同一＝独立描画）"
    );

    // ── R9.4 シーム: 装着配線後も、結線側が持つ同じ入力（view＋model）から行レイアウト
    //    （PositionedLine＝行矩形＋グリフ位置）を再導出できる構造のまま ──
    let binding = TextSlotBinding::from_view(&view0);
    let resolved = ResolvedBalloonText::resolve(&model, binding.image_size);
    let state = rt.state().actor_state(&sakura).expect("actor state");
    let visible = rt.state().visible_glyphs(&sakura, 2.0);
    assert!(visible > 0);
    let lines = LayoutEngine::layout(
        state.items(),
        visible,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert!(!lines.is_empty(), "行レイアウトが再導出できる");
    let placed: usize = lines.iter().map(|l| l.glyphs.len()).sum();
    assert_eq!(placed, visible, "可視グリフが全て行へ配置される（無損失）");
    for line in &lines {
        assert!(
            line.rect.right > line.rect.left && line.rect.bottom > line.rect.top,
            "行矩形は非退化（クリック可能範囲導出の素材・R9.4）"
        );
        for g in &line.glyphs {
            assert!(
                g.advance > 0.0,
                "グリフ送り幅（クリック範囲の幅素材）が得られる"
            );
        }
    }
}

// ══ 単一 actor のみの場合も当該 actor の target へ正しく装着される ══

/// 2 target を結線した上で \0 だけが発話する場合、\0 は自分の target へ装着・描画され、
/// 発話しない \1 の target スロットは無改変（予約 seam のまま・供給面も生成されない）。
#[test]
fn single_actor_attaches_only_to_its_own_target() {
    let mut world = make_world_with_gpu();
    let (presenter, _window0, _window1) = setup_two_targets(&mut world);
    let view0 = presenter.text_slot_view(TargetId(0)).expect("view0");
    let view1 = presenter.text_slot_view(TargetId(1)).expect("view1");

    let sakura = ActorKey::from("0");
    let kero = ActorKey::from("1");
    let model = geo_model();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_view(sakura.clone(), &view0, &model);
    rt.register_actor_view(kero.clone(), &view1, &model);

    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
    present_frame(&mut rt, &mut world, 1.0).expect("単一 actor フレーム");

    assert!(
        rt.is_attached(&sakura),
        "発話した \\0 は当該 target へ装着される"
    );
    assert!(!rt.is_attached(&kero), "発話しない \\1 は装着されない");
    assert!(rt.surface(&kero).is_none(), "\\1 の供給面は生成されない");
    assert!(
        opaque_count(&read_back(&rt, &sakura)) > 0,
        "\\0 のテキストが自分のバルーンへ描画される"
    );
    assert!(
        world
            .get::<VisualGraphics>(view0.slot())
            .is_some_and(|vg| vg.is_valid()),
        "\\0 のスロットには有効 brush が装着される"
    );
    assert!(
        world.get::<VisualGraphics>(view1.slot()).is_none(),
        "発話しない \\1 の target スロットは無改変（予約 seam のまま）"
    );
}

// ══ 未登録 actor の独立性: 結線済み actor の描画を乱さず蓄積のみ ══

/// \0 のみ結線した状態で \1 も発話する場合、present_frame は Ok のまま
/// \0 は正常に描画され、\1 は蓄積のみ（無損失・装着なし）で乱さない。
#[test]
fn unregistered_actor_accumulates_without_disturbing_registered_actor() {
    let mut world = make_world_with_gpu();
    let (presenter, _window0, _window1) = setup_two_targets(&mut world);
    let view0 = presenter.text_slot_view(TargetId(0)).expect("view0");

    let sakura = ActorKey::from("0");
    let kero = ActorKey::from("1");
    let model = geo_model();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_view(sakura.clone(), &view0, &model);

    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
    rt.apply_cue(&cue("1", 0.0, CueCommand::Text("未結線".into())));
    present_frame(&mut rt, &mut world, 1.0).expect("未登録 actor 混在でも frame は Ok");

    assert!(rt.is_attached(&sakura), "結線済み \\0 は装着される");
    assert!(!rt.is_attached(&kero), "未結線 \\1 は装着されない");
    assert!(
        opaque_count(&read_back(&rt, &sakura)) > 0,
        "\\0 の描画は未結線 actor の存在に乱されない"
    );
    let kero_state = rt
        .state()
        .actor_state(&kero)
        .expect("未結線 \\1 の状態は蓄積される（無損失）");
    assert_eq!(kero_state.items().len(), 3, "\\1 の cue は失われない");

    // 後追い結線（表示確立後の登録）→ 次フレームで再試行が装着する。
    let view1 = presenter.text_slot_view(TargetId(1)).expect("view1");
    rt.register_actor_view(kero.clone(), &view1, &model);
    present_frame(&mut rt, &mut world, 2.0).expect("後追い結線フレーム");
    assert!(
        rt.is_attached(&kero),
        "後追い結線の次フレームで \\1 も装着される"
    );
    assert!(
        opaque_count(&read_back(&rt, &kero)) > 0,
        "蓄積されていた \\1 のテキストが自分のバルーンへ描画される"
    );
}

// ══ task 7.3 ①: `TextSlotBinding::from_view` は **物理寸**を読む（native ではない・R8.2） ══

/// **7.1 で修理した production 契約の檻**（7.1 レビュー指摘・本 task が担当の正本）。
///
/// # なぜ in-crate では檻に入らないのか
///
/// [`TextSlotBinding::from_view`] の入力 `TextSlotView` は emo-present の**私有フィールド型**で
/// 公開コンストラクタを持たず、唯一の構築点が `EmoPresenter::text_slot_view`（実 GPU 表示確立が
/// 前提）である。ゆえに `surface_size()`（native）を渡す**旧実装へ戻す変異**は
/// `cargo test -p areka-emo-text` の全 in-crate ターゲットで誰にも殺されない（レビュアー実測）。
/// 本ケースは実 presenter から得た本物の view でその向きを固定する。
///
/// # なぜ k=2（窓 DPI 192／author 96）でなければならないのか
///
/// k=1 では `surface_size() == physical_size()` が**恒等**であり、正しい実装と旧実装が
/// 数値的に区別できない（＝檻が空虚になる）。既存の `setup_two_targets` は k=1.0 固定ゆえ
/// この盲点をそのまま踏んでいた（3.2+3.3 の申し送り「lib 自体は無傷」が偽になった経緯）。
///
/// 檻の内容は 3 点:
/// 1. `binding.surface_size == view.physical_size()`（物理原寸を保持する）、
/// 2. `binding.image_size == view.surface_size()`（image px 原寸＝作者画像空間は **k 不変**）、
/// 3. 供給面の実寸が `ceil(validrect 寸 × k)`＝native の k 倍（R8.2 の可観測な帰結）。
///    旧実装では image 空間が 1/k へ縮み、供給面が k に依らずほぼ一定になる——「k を変えても
///    文字が拡大されない」静かな欠陥（6.5 一次実走で実機検出された症状）そのもの。
#[test]
fn from_view_reads_physical_size_so_image_space_stays_k_invariant() {
    let mut world = make_world_with_gpu();
    let (presenter, _window) = setup_scaled_target(&mut world, 192);
    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");

    // ── 前提の非空虚性: k=2 ゆえ native 原寸と物理寸が実際に食い違っている ──
    assert_eq!(view.scale(), 2.0, "前提: 窓 DPI 192 ／ author 96 で k=2");
    assert_eq!(view.surface_size(), SCALED_NATIVE, "前提: native 原寸");
    assert_eq!(view.physical_size(), (280, 160), "前提: 物理寸＝native×2");
    assert_ne!(
        view.surface_size(),
        view.physical_size(),
        "前提: k≠1 で 2 つの寸が数値的に弁別できる（k=1 では恒等＝檻が空虚）"
    );

    // ── 檻 (1)(2): from_view が読むのは physical_size()——surface_size() ではない ──
    let binding = TextSlotBinding::from_view(&view);
    assert_eq!(
        binding.surface_size,
        view.physical_size(),
        "TextSlotBinding::new の第 4 引数は**物理原寸**（native を渡す旧実装へ戻していないか）"
    );
    assert_eq!(
        binding.image_size,
        view.surface_size(),
        "image px 原寸 = round(physical / k) は作者画像空間＝native と一致（k 不変）"
    );

    // ── 檻 (3): 供給面の実寸が k 倍で伸びる（R8.2 の可観測な帰結） ──
    let model = geo_model();
    let sakura = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_view(sakura.clone(), &view, &model);
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
    present_frame(&mut rt, &mut world, 1.0).expect("k=2 の初回提示（装着）");
    assert_eq!(
        rt.surface(&sakura).expect("装着済み供給面").size(),
        (280, 160),
        "供給面は ceil(validrect 寸 × k)＝native の k 倍（旧実装では k に依らず native 相当へ潰れる）"
    );
    assert!(
        opaque_count(&read_back(&rt, &sakura)) > 0,
        "k=2 の供給面へ実際に文字が描かれる（寸だけ合って空表示、を排除）"
    );
}

// ══ task 7.3 ②: k 再追従の**ログ観測契約**（R8.7・task 7.4 実機サインオフの判定源） ══

/// 再追従が決定論的に**ログで観測できる**ことを固定する（R8.7）。
///
/// task 7.4 の実機サインオフは、これらのフィールドを grep して判定する:
/// - 再追従 `info!`: `actor` ／ `k_old` ／ `k_new`（k の遷移そのもの）、
/// - 2 回目の装着 `info!`: `physical_size`（新 k の供給面寸）。
///
/// **判定は絶対値でなく比で行う**（7.4 本文）——ここでも `physical_size` の比が
/// `k_new / k_old` に一致することまで固定する。フィールド名の改名・ログ削除は 7.4 の判定基準を
/// 静かに壊すため、名前と値の双方を檻に入れる（[`LogEvent::field`] は欠落で panic する）。
///
/// 併せて「装着 `info!` は `ActorRender` 不在時のみ」（7.1 で doc を改訂した契約）も
/// 実測する: 同一 actor で装着ログが**2 回**出るのは、間に再追従による破棄が挟まったときだけ。
#[test]
fn scale_refresh_logs_k_transition_and_reattach_physical_size() {
    let mut world = make_world_with_gpu();
    // k=1.0 から始めて 2.0 へ跳ばす（実機の 100%→200% モニタ跨ぎに相当）。
    let (mut presenter, window) = setup_scaled_target(&mut world, 96);
    let view = presenter.text_slot_view(TargetId(0)).expect("view（k=1）");
    assert_eq!(view.scale(), 1.0, "前提: 窓 DPI 96 ／ author 96 で k=1");

    let model = geo_model();
    let sakura = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_view(sakura.clone(), &view, &model);
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));

    // ── 1 回目の装着ログ: physical_size は k=1 の native 寸 ──
    let attach_msg = "テキスト供給面を予約スロットへ装着した";
    let (_, attach_events) =
        capture_logs(|| present_frame(&mut rt, &mut world, 1.0).expect("初回提示（装着）"));
    let first_attach = expect_one(&attach_events, attach_msg);
    assert_eq!(
        first_attach.level,
        tracing::Level::INFO,
        "装着は info!（既定 RUST_LOG=info で実機ログに残る水準・log-first）"
    );
    assert_eq!(
        first_attach.field("physical_size"),
        "(140, 80)",
        "k=1 の供給面寸（7.4 の比較基準）"
    );

    // ── 窓 DPI を 192 へ（実機のモニタ跨ぎ相当）→ presenter の適用 k が 2.0 へ跳ぶ ──
    world.entity_mut(window).insert(DPI::from_dpi(192, 192));
    presenter
        .refresh_scale(&mut world, TargetId(0))
        .expect("可視 target の k 変化は新物理寸を報告する");
    let view2 = presenter.text_slot_view(TargetId(0)).expect("view（k=2）");
    assert_eq!(view2.scale(), 2.0, "前提: 再表示で適用 k が 2.0 へ跳ぶ");

    // ── 再追従ログ: actor ／ k_old ／ k_new（R8.7 の本体） ──
    let (refreshed, refresh_events) =
        capture_logs(|| rt.refresh_actor_scale(&sakura, &view2, &model));
    assert!(refreshed, "k 変化の再追従は true");
    let refresh_log = expect_one(&refresh_events, "文字層の k 再追従");
    assert_eq!(
        refresh_log.level,
        tracing::Level::INFO,
        "再追従は info!（実機 grep の水準・R8.7）"
    );
    assert_eq!(refresh_log.field("actor"), "0", "actor フィールド");
    assert_eq!(
        refresh_log.field("k_old").parse::<f64>().ok(),
        Some(1.0),
        "k_old フィールド（旧 k）"
    );
    assert_eq!(
        refresh_log.field("k_new").parse::<f64>().ok(),
        Some(2.0),
        "k_new フィールド（新 k）"
    );

    // ── 2 回目の装着ログ: physical_size が新 k 寸（比 = k_new / k_old = 2.0） ──
    let (_, reattach_events) =
        capture_logs(|| present_frame(&mut rt, &mut world, 1.0).expect("再追従後の提示"));
    let second_attach = expect_one(&reattach_events, attach_msg);
    assert_eq!(
        second_attach.field("physical_size"),
        "(280, 160)",
        "再装着の供給面寸は新 k 寸——140×80 に対し比 2.0（7.4 は絶対値でなくこの比で判定する）"
    );

    // ── 装着 info! は「ActorRender 不在時のみ」——同値 k の走査では 3 回目が出ない ──
    assert!(
        !rt.refresh_actor_scale(&sakura, &view2, &model),
        "同値 k の再追従は no-op（churn ガード・R8.5）"
    );
    let (_, idle_events) =
        capture_logs(|| present_frame(&mut rt, &mut world, 1.0).expect("同値 k 後の提示"));
    assert!(
        !idle_events.iter().any(|e| e.message().contains(attach_msg)),
        "破棄が挟まらないフレームでは装着 info! は出ない（毎フレーム再生成の禁止）: {idle_events:?}"
    );
}
