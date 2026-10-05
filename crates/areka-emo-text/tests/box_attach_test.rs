//! # box_attach_test — 箱の面の窓の子としての装着を GPU の読み戻しで確かめる（task 7.8）
//!
//! シェル相当の 1 target を実 `EmoPresenter` で表示確立し（窓 DPI 192＝拡大率 2）、その
//! `text_slot_view` を毎フレームの箱の同期（`sync_boxes`）へ渡して、箱 2 つを持つスコープを
//! `present_frame` で提示する。確かめること（要件 1.6・3.1〜3.4・3.7・3.9・6.8・9.4）:
//!
//! - 窓の `Children` が「差し込み口 → element番号の大きい箱 → 小さい箱 → 絵」に並ぶ
//!   （element番号の大きい順の数え方はこの GPU の通しでしか踏めない・tasks.md 7.4 の申し送り）
//! - 箱の面の当たり判定は表示されている字の矩形の集まり（字の無い所は受けない・task 13）
//! - 読み戻しで箱ごとに独立に文字が描かれ、背景は透明（絵を描かない）
//! - 箱の位置が `(X + 領域の左, Y + 領域の上) × 拡大率`
//! - 箱の束の差し替えで箱の面の entity が消える
//!
//! 流儀は `attach_wiring_test.rs` と同じ（GPU/WUC は headless 実資源・WARP 可・時刻は注入）。
//! 箱の束はテストの中に持つ surfaces.txt の文面を畳んで作る（検体は読まない・tasks.md 1）。

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, BoxLayout, EmoWorld, fold_boxes};
use areka_emo_present::{EmoPresenter, PresentCommand, PresentOutcome, TargetId};
use areka_emo_text::actor::{SurfaceKeyResolver, TextLayerRuntime, present_frame};
use areka_emo_text::place::{PlaceKey, TextPlace};
use areka_emo_text::state::{SurfaceKeyOutcome, TextLayerConfig};
use areka_parsers::shell::{
    AppendTarget, DefRef, Element, ElementPath, Shell, Surface, parse, parse_boxes,
};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::{
    AlphaMaskResource, Arrangement, DPI, GraphicsCommandList, GraphicsCore, HitTest,
    VisualGraphics, WucGraphicsResource,
};

/// シェルの面の native 原寸（サーフェス 1000 の絵）。
const SHELL_NATIVE: (u32, u32) = (200, 120);

/// 窓 DPI 192 ／ 作者 DPI 96 ＝ 拡大率 2（k=1 では位置の × 拡大率を取り違えても数値が同じ）。
const K: f32 = 2.0;

/// 全部の文字が見えている時刻。
const LATE: f64 = 10.0;

/// サーフェス 1000 は絵（element0）の上に箱 a（element1・(10,20)・80×40・領域の左上 (6,4)）と
/// 箱 b（element2・(100,70)・60×30・領域は全域）を持つ。どちらもサーフェスからはみ出さない。
const SHELL: &str = "\
balloon.a
{
size,80,40
validrect.left,6
validrect.top,4
}
balloon.b
{
size,60,30
}
surface1000
{
element0,overlay,p.png,0,0
element1,balloon,a,10,20
element2,balloon,b,100,70
}
";

// ── GPU/WUC フィクスチャ（attach_wiring_test.rs と同じ） ─────────────────────────────

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

/// サーフェス 1000 ＝ 全不透明の絵 1 枚の `(EmoWorld, AtlasTable)`（シェルの絵に相当）。
fn build_shell_assets() -> (EmoWorld, AtlasTable) {
    let (w, h) = SHELL_NATIVE;
    let base = Path::new("shell/master");
    let surfaces = vec![Surface {
        id: 1000,
        targets: vec![AppendTarget::Single(1000)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new("p.png".to_string()),
            x: 0,
            y: 0,
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let mut dec = MemoryDecoder::new();
    dec.insert(
        base.join("p.png"),
        w,
        h,
        w * 4,
        vec![0x80; (w * h * 4) as usize],
        true,
    );
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake は失敗しない");
    let shell = Shell {
        definitions: (0..surfaces.len()).map(DefRef::Surface).collect(),
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
    };
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// シェルの target を拡大率 2 で表示確立まで組む。返り値: (presenter, 窓)。
fn setup_shell(world: &mut World) -> (EmoPresenter, Entity) {
    let window = world.spawn(DPI::from_dpi(192, 192)).id();
    let (emo_world, atlas) = build_shell_assets();
    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
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
    (presenter, window)
}

// ── 箱の束と cue ─────────────────────────────────────────────────────────────────────

fn layout() -> BoxLayout {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
}

/// 偽の閉包: 数字ならその番号を表示。
fn resolver() -> SurfaceKeyResolver {
    Box::new(|key: &str| {
        key.parse()
            .map_or(SurfaceKeyOutcome::Unresolved, SurfaceKeyOutcome::Show)
    })
}

fn cue(command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: actor(),
        command,
        duration: 0.0,
    }
}

fn actor() -> ActorKey {
    ActorKey::from("0")
}

/// スコープ 0 の箱 `name` の場所。
fn place(layout: &BoxLayout, name: &str) -> PlaceKey {
    let name = layout
        .placements(1000)
        .iter()
        .find(|p| p.name.as_str() == name)
        .expect("文面にある箱の名前")
        .name
        .clone();
    PlaceKey {
        actor: actor(),
        place: TextPlace::Box(name),
    }
}

/// `\b[name]` で行き先を替えて文字を書く。
fn write_to(rt: &mut TextLayerRuntime, name: &str, text: &str) {
    rt.apply_cue(&cue(CueCommand::BalloonSurface {
        key: name.to_owned(),
    }));
    rt.apply_cue(&cue(CueCommand::Text(text.into())));
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0）。
fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

fn read_back(rt: &TextLayerRuntime, place: &PlaceKey) -> Vec<u8> {
    rt.surface_at(place)
        .expect("提示済みの箱の面")
        .read_back()
        .expect("read_back")
}

/// 箱の面の entity の当たりのマスクで「内」の画素の数と、(左上, 最後の行, 右上) の内外。
fn mask_summary(world: &World, child: Entity) -> (usize, bool, bool, bool) {
    let mask = world
        .get::<AlphaMaskResource>(child)
        .and_then(AlphaMaskResource::mask)
        .expect("箱の面は当たりのマスクを持つ");
    let (w, h) = (mask.width(), mask.height());
    let inside = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| mask.is_hit(x, y))
        .count();
    let last_row = (0..w).any(|x| mask.is_hit(x, h - 1));
    (inside, mask.is_hit(1, 1), last_row, mask.is_hit(w - 1, 0))
}

fn children(world: &World, window: Entity) -> Vec<Entity> {
    world
        .get::<Children>(window)
        .expect("窓の子")
        .iter()
        .copied()
        .collect()
}

// ══ 観測できる完了の姿 ══════════════════════════════════════════════════════════════

/// 箱 2 つを持つスコープを提示すると、箱の面はシェルの窓の直接の子として
/// 「差し込み口 → element番号の大きい箱 → 小さい箱 → 絵」に並び、表示されている字の矩形で
/// ポインタを受け（字の無い所は受けない）、
/// `(X + 領域の左, Y + 領域の上) × 拡大率` に置かれ、箱ごとに独立に透明な背景へ文字を描く。
/// 箱の束を差し替えると箱の面の entity は消え、窓の子は差し込み口と絵だけに戻る。
#[test]
fn boxes_attach_as_window_children_in_element_order_and_draw_independently() {
    let mut world = make_world_with_gpu();
    let (presenter, window) = setup_shell(&mut world);
    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    assert_eq!(view.scale(), K, "前提: 窓 DPI 192 ／ 作者 96 で拡大率 2");
    let slot = view.slot();
    let picture = children(&world, window)
        .into_iter()
        .find(|e| {
            world
                .get::<Name>(*e)
                .is_some_and(|n| n.as_str() == "emo-surface")
        })
        .expect("絵の entity が窓の子に在る");
    assert_eq!(
        children(&world, window),
        vec![slot, picture],
        "前提: 箱の前の窓の子は差し込み口と絵だけ"
    );

    let layout = layout();
    let (box_a, box_b) = (place(&layout, "a"), place(&layout, "b"));
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_box_layout(&mut world, layout, resolver(), vec![]);
    rt.apply_cue(&cue(CueCommand::Emote { key: "1000".into() }));
    write_to(&mut rt, "a", "アヒル");
    write_to(&mut rt, "b", "ふむ");
    // 置き場所はシェルの窓がいま表示している絵の番号から導く（結線と同じく表示層に照会する）。
    let shown = presenter
        .current_surface_id(TargetId(0))
        .expect("前提: 絵 1000 を表示している");
    rt.sync_boxes(&mut world, &[(actor(), Some((view, shown)))]);
    present_frame(&mut rt, &mut world, LATE).expect("提示フレーム");

    // ── 並び（要件 3.7・3.9）: 差し込み口 → b（element2）→ a（element1）→ 絵 ──
    let child_of = |place: &PlaceKey| {
        rt.surface_at(place)
            .and_then(|s| s.window_child())
            .expect("箱の面は窓の子を持つ")
    };
    let (child_a, child_b) = (child_of(&box_a), child_of(&box_b));
    assert_eq!(
        children(&world, window),
        vec![slot, child_b, child_a, picture],
        "窓の子は 差し込み口 → element番号の大きい箱 → 小さい箱 → 絵"
    );

    // ── 当たり判定は表示されている字の矩形の集まり（要件 9.4・task 13）・装着の構造 ──
    for (name, child) in [("a", child_a), ("b", child_b)] {
        assert_eq!(
            world.get::<HitTest>(child).copied(),
            Some(HitTest::alpha_mask()),
            "箱 {name} の面はマスク（字の矩形の集まり）で当たりを決める"
        );
        let (inside, first_glyph, last_row, top_right) = mask_summary(&world, child);
        assert!(
            inside > 0 && first_glyph,
            "箱 {name}: 1 字目の矩形（面の左上）は受ける"
        );
        assert!(
            !last_row && !top_right,
            "箱 {name}: 1 行だけの短い文字の下と右の字の無い所は受けない"
        );
        assert!(
            world
                .get::<VisualGraphics>(child)
                .is_some_and(|vg| vg.is_valid()),
            "箱 {name} の面に有効な brush"
        );
        assert!(
            world.get::<GraphicsCommandList>(child).is_none(),
            "箱 {name} の面は wintf の描画経路を持たない"
        );
    }

    // ── 位置（要件 3.1〜3.4）: (X + 領域の左, Y + 領域の上) × 拡大率 ──
    let offset = |child: Entity| {
        let a = world.get::<Arrangement>(child).expect("Arrangement");
        (a.offset.x, a.offset.y, a.size.width, a.size.height)
    };
    assert_eq!(
        offset(child_a),
        (
            (10.0 + 6.0) * K,
            (20.0 + 4.0) * K,
            (80.0 - 6.0) * K,
            (40.0 - 4.0) * K
        ),
        "箱 a: 領域の左上 (6,4) を足して × 拡大率・大きさは領域 × 拡大率"
    );
    assert_eq!(
        offset(child_b),
        (100.0 * K, 70.0 * K, 60.0 * K, 30.0 * K),
        "箱 b: 領域は全域"
    );

    // ── 読み戻し（要件 1.6・3.4）: 箱ごとに文字が描かれ、背景は透明 ──
    let (bytes_a, bytes_b) = (read_back(&rt, &box_a), read_back(&rt, &box_b));
    for (name, bytes, (w, h)) in [
        ("a", &bytes_a, rt.surface_at(&box_a).expect("a").size()),
        ("b", &bytes_b, rt.surface_at(&box_b).expect("b").size()),
    ] {
        assert_eq!(bytes.len(), (w * h * 4) as usize);
        let ink = opaque_count(bytes);
        assert!(ink > 0, "箱 {name} に文字が描かれる");
        assert!(
            ink * 2 < (w * h) as usize,
            "箱 {name} の背景は透明（絵を描かない）: 非透明 {ink} / {}",
            w * h
        );
        let last_row = &bytes[((h - 1) * w * 4) as usize..];
        assert_eq!(
            opaque_count(last_row),
            0,
            "箱 {name} の 1 行だけの文字の下は透明のまま"
        );
    }

    // ── 独立性: b への追記は a の面へ波及しない ──
    let (inside_a, inside_b) = (
        mask_summary(&world, child_a).0,
        mask_summary(&world, child_b).0,
    );
    rt.apply_cue(&cue(CueCommand::Text("ふむふむ".into())));
    present_frame(&mut rt, &mut world, LATE + 1.0).expect("追記後フレーム");
    assert!(
        mask_summary(&world, child_b).0 > inside_b,
        "b の追記で b の字の矩形が増える"
    );
    assert_eq!(
        mask_summary(&world, child_a).0,
        inside_a,
        "b の追記は a の字の矩形を変えない"
    );
    assert!(
        opaque_count(&read_back(&rt, &box_b)) > opaque_count(&bytes_b),
        "b の追記で b のインクが増える"
    );
    assert_eq!(
        read_back(&rt, &box_a),
        bytes_a,
        "b の更新は a の面へ波及しない（バイト同一）"
    );
    assert_eq!(
        children(&world, window),
        vec![slot, child_b, child_a, picture],
        "提示を重ねても面は作り直されず並びも同じ"
    );

    // ── 箱の束の差し替え（要件 6.8）: 箱の面の entity が消える ──
    rt.set_box_layout(&mut world, self::layout(), resolver(), vec![]);
    for (name, child) in [("a", child_a), ("b", child_b)] {
        assert!(
            world.get_entity(child).is_err(),
            "箱 {name} の面の entity は消える"
        );
    }
    assert_eq!(
        children(&world, window),
        vec![slot, picture],
        "窓の子は差し込み口と絵だけに戻る"
    );
    assert!(rt.surface_at(&box_a).is_none() && rt.surface_at(&box_b).is_none());
    assert!(rt.shown_boxes(&actor()).is_empty(), "写しも空");
}
