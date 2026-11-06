//! presenter 系テストモジュールが共有する補助（フィクスチャ・期待値・ログ捕捉）。
//!
//! **期待値はすべて原寸（native）である。** 表示の拡大縮小は wintf の描画経路（`Arrangement.scale`
//! ＝D2D 変換行列）が持つので、presenter が抱えるバイト列は k に依らず `Composer::compose` の
//! 出力そのものになった。ゆえに本 module の golden 生成は拡大率を引数に取らず、CPU リサンプラを
//! 一切呼ばない（spec: areka-P0-present-gpu-transform-scale・要件 6.1／6.3・設計 付録 A′ #8）。

use super::*;

use std::num::NonZeroU32;
use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, DecodedImage,
    LoopCount, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake, bake_with_limits,
};
use areka_emo_compose::{BindSet, ComposeMethod, FilmSheet, PatternFrame};
use areka_parsers::shell::{
    Animation, AppendTarget, Collision, CollisionName, DefRef, DrawMethod, Element, ElementPath,
    Interval, Pattern, Shell, Surface,
};

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::widget::bitmap_source::AlphaMask;
use wintf::ecs::{Arrangement, GraphicsCommandList, GraphicsCore, WucGraphicsResource};

// ── GPU/WUC フィクスチャ（wintf の wuc_resource.rs のテストと同一方針）────────────────────
// 本番 UI スレッドは MTA（メモリ「areka WUC は MTA スレッドで動く」）。WucGraphicsResource::new は
// DQTAT_COM_NONE（apartment 不変）でディスパッチャを組むため、COM を MTA 初期化してから呼ぶ。

/// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World を組む。
///
/// `EmoPresenter` が本番で読むのは `GraphicsCore` だけになった（装着が純 ECS になり
/// `Compositor` を要さない）。`WucGraphicsResource` は本番同様に載せたままにする——
/// `presenter_hide_contract_tests.rs` がそこから compositor を読んで SpriteVisual を作るためである。
pub(super) fn make_world_with_gpu() -> World {
    // 各テストは専用スレッドで走る。MTA を初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// 窓 entity を **`DPI` component 付き**で作る（design「Testing Strategy > Integration Tests」の
/// テスト World 前提）。
///
/// 本番の窓生成は必ず `DPI` を付与する（wintf が `GetDpiForWindow` の実値で補正する）。テストで
/// component を省くと要件 1.4 の縮退（error! ＋ k=1.0）が「正常系のふり」で緑になってしまうため、
/// **明示挿入を規律とする**。96 挿入＝恒等 k、192 挿入＝k=2/1。縮退分岐そのものは
/// `show_surface_without_dpi_component_degrades_to_identity`（DPI 不在専用テスト）で檻に入れる。
pub(super) fn spawn_window_with_dpi(world: &mut World, dpi: u16) -> Entity {
    world.spawn(DPI::from_dpi(dpi, dpi)).id()
}

/// 窓 entity の `DPI` component を差し替える（モニタ跨ぎ移動・表示スケール変更の決定論的代替）。
pub(super) fn set_window_dpi(world: &mut World, window: Entity, dpi: u16) {
    world.entity_mut(window).insert(DPI::from_dpi(dpi, dpi));
}

/// `build_target_assets` と同一入力の **native 合成**バイト列とその外形（表示の golden）。
///
/// presenter が抱えるのは原寸バイトそのもので、拡大は描画側の係数に移った。ゆえに golden は
/// 窓 DPI がいくつであっても同一であり、拡大率を引数に取らない。「読み戻しが偶然それらしい
/// 寸法になった」ではなく **合成バイトそのもの**を固定する点は変わらない。
pub(super) fn native_golden(
    emo_world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
) -> (Vec<u8>, (u32, u32)) {
    native_golden_with(
        emo_world,
        atlas,
        surface_id,
        &BindSet::default(),
        &PatternState::default(),
    )
}

/// [`native_golden`] の一般形（**任意の bind 集合・pattern** で合成する）。
pub(super) fn native_golden_with(
    emo_world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> (Vec<u8>, (u32, u32)) {
    let mut composer = Composer::new();
    let native = composer
        .compose(emo_world, atlas, surface_id, binds, pattern)
        .expect("golden 用の native 合成は Ok");
    (native.bytes().to_vec(), (native.width(), native.height()))
}

/// `ShowSurface` を適用し、reply をそのまま返す（受信できなければ panic）。
pub(super) fn show_with(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
) -> PresentOutcome {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id,
            binds,
            pattern,
            reply: Some(tx),
        },
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("reply（ShowSurface）を受信できない")
}

/// [`show_with`] を適用し、reply が `Ok(())` であることを確認する。
pub(super) fn show_with_ok(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
) {
    let outcome = show_with(presenter, world, target, surface_id, binds, pattern);
    assert!(
        matches!(outcome, Ok(())),
        "ShowSurface（surface {surface_id}）が Ok でない: {outcome:?}"
    );
}

/// 有効 `ShowSurface`（bind・コマ無し）を適用し、reply が `Ok(())` であることを確認する。
pub(super) fn show_ok(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
) {
    show_with_ok(
        presenter,
        world,
        target,
        surface_id,
        BindSet::default(),
        PatternState::default(),
    );
}

/// `Hide` を適用し、reply が `Ok(())` であることを確認する。
pub(super) fn hide_ok(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::Hide {
            target,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "Hide が Ok でない"
    );
}

/// `InvalidateCache`（控えを捨てる）を適用し、reply が `Ok(())` であることを確認する。
pub(super) fn invalidate_ok(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::InvalidateCache {
            target,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "InvalidateCache が Ok でない"
    );
}

/// 表示の記録の失敗点 `at` を注入したまま [`show_with`] を適用し、reply を返す。
///
/// 注入はこの呼び出しの中だけで、抜けるときに守り（`arm_display_fault` の戻り値）が降ろす。
pub(super) fn show_with_armed_fault(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
    at: crate::display::DisplayFault,
) -> PresentOutcome {
    let _armed = crate::display::arm_display_fault(at);
    show_with(presenter, world, target, surface_id, binds, pattern)
}

/// 装着済み target の (枠の面 entity, 文字層スロット entity)。
///
/// presenter の私有状態（`visible`）だけを見ると「照会は false を返すが entity は可視」という
/// 食い違いを見逃すため、可視性を主張するテストはここから実 component 値を読む。表示が一度
/// 成立するまで mount は生成されないので、初回の `ShowSurface` より後に呼ぶこと。
pub(super) fn mount_entities(presenter: &EmoPresenter, target: TargetId) -> (Entity, Entity) {
    let mount = presenter
        .targets
        .get(&target)
        .expect("装着済み target")
        .mount
        .as_ref()
        .expect("表示確立後は mount が生成済み");
    (mount.surface_entity(), mount.text_slot())
}

/// 枠の面 entity の `Arrangement` を `(寸, 係数)` で読む（**論理寸＝原寸・係数＝拡大率 k**）。
///
/// 両方を 1 つの値で読むのは、片方だけを見ると「寸へ k を焼き込み係数を 1.0 に据え置く」旧形と
/// 区別できないためである。物理寸そのもの（`GlobalArrangement.bounds`）は wintf の伝播段
/// （`PostLayout`）が導くので、丸めの一致は `display_tests.rs` の T-N3 が表で固定する。
pub(super) fn arrangement_of(world: &World, surface_entity: Entity) -> ((u32, u32), (f32, f32)) {
    let arr = world
        .get::<Arrangement>(surface_entity)
        .expect("surface entity に Arrangement が無い");
    (
        (arr.size.width as u32, arr.size.height as u32),
        (arr.scale.x, arr.scale.y),
    )
}

/// 枠の面 entity へ供給済みの αマスク寸（未供給なら `None`）。
pub(super) fn mask_dims(world: &World, surface_entity: Entity) -> Option<(u32, u32)> {
    world
        .get::<AlphaMaskResource>(surface_entity)
        .and_then(|r| r.mask().map(|m| (m.width(), m.height())))
}

/// マスクの立っているビット数・落ちているビット数。
pub(super) fn mask_bit_counts(mask: &AlphaMask) -> (usize, usize) {
    let mut hits = 0;
    let mut misses = 0;
    for y in 0..mask.height() {
        for x in 0..mask.width() {
            if mask.is_hit(x, y) {
                hits += 1;
            } else {
                misses += 1;
            }
        }
    }
    (hits, misses)
}

/// 表示成立点の行の固定文言（`show.rs` の `info!`・実機サインオフ契約）。
pub(super) const INFO_LINE_MESSAGE: &str = "apply(ShowSurface): 表示・マスクを更新";

/// 重ね直しの範囲の行の固定文言（`show.rs`・要件 3.5）。
pub(super) const RANGE_LINE_MESSAGE: &str = "apply(ShowSurface): 重ね直しの範囲";

// ── ComposedSurface 生成補助 ────────────────────────────────────────────────────────────
// `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接焼けない。
// 上流公開 API（atlas bake → EmoWorld → Composer::compose）で本物を合成して得る。

pub(crate) fn elem(path: &str, x: i64, y: i64) -> Element {
    Element {
        layer: 0,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

pub(crate) fn surface(id: u32, elements: Vec<Element>) -> Surface {
    Surface {
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

pub(crate) fn shell_of(surfaces: Vec<Surface>) -> Shell {
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

/// 座標由来のグラデーション（premultiplied BGRA）。α は画素ごとに `alpha(x, y)` で、色は α を掛けて
/// 焼く（`B,G,R ≤ A` の不変条件を崩さない）。α=255 なら色はそのまま、α=0 なら画素は 0。
pub(super) fn gradient_with_alpha(
    w: u32,
    h: u32,
    salt: u8,
    alpha: impl Fn(u32, u32) -> u8,
) -> Vec<u8> {
    let mut img: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let a = alpha(x, y);
            let pm = |c: u8| ((c as u16 * a as u16) / 255) as u8;
            img.extend_from_slice(&[
                pm((x as u8).wrapping_mul(3).wrapping_add(salt)),
                pm((y as u8).wrapping_mul(5).wrapping_add(salt)),
                pm(((x + y) as u8).wrapping_mul(7).wrapping_add(salt)),
                a,
            ]);
        }
    }
    img
}

/// 全不透明の [`gradient_with_alpha`]。
pub(super) fn gradient(w: u32, h: u32, salt: u8) -> Vec<u8> {
    gradient_with_alpha(w, h, salt, |_, _| 0xFF)
}

/// `w×h` の単色不透明（BGR を与える）。
pub(super) fn solid(w: u32, h: u32, bgr: [u8; 3]) -> Vec<u8> {
    (0..w * h)
        .flat_map(|_| [bgr[0], bgr[1], bgr[2], 0xFF])
        .collect()
}

/// `shell/master` に置いた絵 `(名前, 幅, 高さ, BGRA)` で `shell` の面を atlas に焼き、
/// `EmoWorld` へ組んで束ねる（atlas bake → `EmoWorld` → `bind_atlas`）。
pub(crate) fn bake_world(
    shell: Shell,
    images: Vec<(&str, u32, u32, Vec<u8>)>,
) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let mut dec = MemoryDecoder::new();
    for (name, w, h, bgra) in images {
        dec.insert(base.join(name), w, h, w * 4, bgra, true);
    }
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない: {:?}",
        baked.errors
    );
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// surface 1000 = 単一 element（`w×h` 全不透明・座標由来グラデーション）の `(EmoWorld, AtlasTable)`
/// と、同一入力を `Composer::compose` で直接合成した golden バイト列を返す。
///
/// α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は正確に `w×h`。golden は presenter が
/// 内部で辿るのと同一の world/atlas から作るため、readback とのバイト一致が二重に決定論的になる。
pub(super) fn build_target_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable, Vec<u8>) {
    let (world, atlas) = bake_world(
        shell_of(vec![surface(1000, vec![elem("p.png", 0, 0)])]),
        vec![("p.png", w, h, gradient(w, h, salt))],
    );
    let golden = native_golden(&world, &atlas, 1000).0;
    (world, atlas, golden)
}

/// 有効 surface 1000（`w×h` 全不透明 element）＋ 定義層皆無で外形 0×0 に退化する surface 7000
/// （element/animation ゼロ）を**同一 target**へ載せる `(EmoWorld, AtlasTable)` を返す。
///
/// surface 7000 は plan_extent_tests.rs の `build_plan_no_layers_degenerate_is_empty_composition`
/// と同型の構成（bind 済み world に存在するが合成外形 0×0）で、`Composer::compose` が
/// `Err(ComposeError::EmptyComposition(7000))` を返す。単なる全透明 element は非ゼロ外形の
/// `Ok`（`all_transparent_surface_is_ok_transparent_nonzero_extent`）で EmptyComposition では
/// ないため、退化は「定義層皆無 → 0×0」の経路で作る。
pub(super) fn build_assets_with_valid_and_empty(
    w: u32,
    h: u32,
    salt: u8,
) -> (EmoWorld, AtlasTable) {
    // surface 1000: 全不透明 element 1 本。surface 7000: element/animation ゼロ（定義層皆無）。
    bake_world(
        shell_of(vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(7000, Vec::new()),
        ]),
        vec![("p.png", w, h, gradient(w, h, salt))],
    )
}

/// surface 1000／3000 = 同 `w×h`・全不透明・**別バイト**（別 element・別 salt）を持つ単一
/// world の `(EmoWorld, AtlasTable)` と、各面の直接合成 golden 2 本を返す（build_target_assets の
/// 複面版）。
///
/// 両面とも α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は両面とも正確に `w×h`
/// （＝同寸）。ゆえに外形変化の経路を踏まずに「同寸・異 id 再 Show」だけを固定できる。
/// golden は presenter が内部で辿るのと同一 world/atlas から作るため readback とのバイト一致が
/// 二重に決定論的。2 面の golden が別物であることを fixture 自身が assert する（R6.1 の回帰檻前提）。
pub(super) fn build_two_face_assets(w: u32, h: u32) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
    let (world, atlas) = bake_world(
        shell_of(vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(3000, vec![elem("q.png", 0, 0)]),
        ]),
        vec![
            ("p.png", w, h, gradient(w, h, 0x11)),
            ("q.png", w, h, gradient(w, h, 0x77)),
        ],
    );
    let golden_1000 = native_golden(&world, &atlas, 1000).0;
    let golden_3000 = native_golden(&world, &atlas, 3000).0;
    assert_ne!(
        golden_1000, golden_3000,
        "fixture 前提: 同寸でも 2 面のバイトが異ならなければ再表示の回帰檻にならない"
    );

    (world, atlas, golden_1000, golden_3000)
}

/// 改竄プローブ: `target` の面 1000（既定の bind・コマ）のスロットの絵を、[`build_two_face_assets`]
/// `(w, h)` の面 3000 の原寸合成へ差し替える（マスクは同じ bytes から作り、表示記録は `display`）。
///
/// キーも原寸も変えず絵だけを差し替えるので、その後の表示が面 3000 の絵になれば presenter は
/// スロットを引いている（合成し直していれば面 1000 の絵に戻る）。
pub(super) fn tamper_face_1000_slot(
    presenter: &mut EmoPresenter,
    target: TargetId,
    (w, h): (u32, u32),
    display: GraphicsCommandList,
) {
    let (world, atlas, _, _) = build_two_face_assets(w, h);
    let tampered = Composer::new()
        .compose(
            &world,
            &atlas,
            3000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("面 3000 の合成は Ok");
    let mask = std::sync::Arc::new(AlphaMask::from_pbgra32(
        tampered.bytes(),
        tampered.width(),
        tampered.height(),
        tampered.stride(),
    ));
    presenter
        .targets
        .get_mut(&target)
        .expect("装着済み target")
        .cache
        .insert(
            1000,
            BindSet::default(),
            PatternState::default(),
            tampered,
            mask,
            display,
        );
}

/// animation `anim_id` に surface `surf` の `Overlay` 現在コマ 1 枚を重ね位置 `(x, y)` で持つ非空 `PatternState`。
///
/// 非ゼロ `(x, y)` は SERIKO アニメの実 pattern（`surfaces.txt` の `animationN.patternM` が持つ
/// 座標）と同型であり、k 追従の相対配置檻が要求する**非対称な重ね位置**を作る。
pub(crate) fn pattern_overlay_at(anim_id: u32, surf: u32, x: i64, y: i64) -> PatternState {
    let mut p = PatternState::default();
    p.set(
        anim_id,
        PatternFrame {
            surface_id: surf,
            method: ComposeMethod::Overlay,
            x,
            y,
        },
    );
    p
}

/// surface 1000（`p.png` を (0,0)）に、着せ替え animation 2000（`Interval::Bind`）の pattern0 が
/// surface 5000 を `(x, y)` へ overlay する面（`BindSet::from_ids([2000])` で有効になる）。
pub(super) fn bind_overlay_surface(x: i64, y: i64) -> Surface {
    Surface {
        animations: vec![Animation {
            id: 2000,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 5000,
                target: "5000".into(),
                wait: 0,
                x,
                y,
            }],
        }],
        ..surface(1000, vec![elem("p.png", 0, 0)])
    }
}

// ── hit_region_client の配線と縮退の檻（要件 1.4-1.7・DD-5）─────────────────
//
// 本節はすべて **GPU 非依存**である。`attach_target` は skeleton 登録のみで World にも GPU にも
// 触れないため素の `World::new()` で足り、判定に必要な状態（表示中サーフェス・実適用 k）は
// in-source テストの特権である**私有フィールドの直接構築**で作る。実表示（`ShowSurface`）で
// 作ろうとすると GPU/WUC 資源が要り、かつ「面はあるのに applied が無い」状態は表示成立点が
// 両者を同時に確定させるため**原理的に作れない**（DD-5 が「現行の公開 API 経由では到達不能な
// 防御分岐」と述べるとおり）。私有状態の直接構築だけがこの分岐を実行テストへ入れる手段である。

/// 当たり判定矩形 1 件（`areka-emo-compose/src/hit.rs` の檻と同一の作り方）。
fn hit_coll(index: u32, left: i64, top: i64, right: i64, bottom: i64, name: &str) -> Collision {
    Collision {
        index,
        left,
        top,
        right,
        bottom,
        name: CollisionName::new(name.to_string()),
    }
}

/// collision のみを持つ surface 1000 の `(EmoWorld, AtlasTable)`。
///
/// element を 1 つも持たないため atlas 焼きは空で、画像デコード・GPU・実描画のいずれも要さない
/// （本節は判定だけを見るので画素は不要）。矩形は `areka-emo-compose/src/hit.rs` の檻と同一の Head/Bust に、
/// **Bust と重なる Arm**（後定義＝画家則で手前）を足した 3 件で、重なり点の檻を成立させる。
fn build_collision_only_assets() -> (EmoWorld, AtlasTable) {
    bake_world(
        shell_of(vec![Surface {
            targets: vec![AppendTarget::Single(1000)],
            elements: Vec::new(),
            collisions: vec![
                hit_coll(0, 93, 62, 271, 130, "Head"),
                hit_coll(1, 133, 270, 229, 326, "Bust"),
                hit_coll(2, 200, 300, 400, 400, "Arm"),
            ],
            animations: Vec::new(),
        }]),
        Vec::new(),
    )
}

/// `attach_target` 済み（未表示）の target を 1 つ登録する。
pub(super) fn attach_hit_target(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas) = build_collision_only_assets();
    presenter.attach_target(target, window, emo_world, atlas, 96);
}

/// 私有状態を直接書いて「表示中サーフェスあり」を作る（GPU なしで R1.6 分岐へ到達する唯一の手段）。
pub(super) fn force_current_surface(
    presenter: &mut EmoPresenter,
    target: TargetId,
    surface_id: u32,
) {
    presenter
        .targets
        .get_mut(&target)
        .expect("attach 済み target")
        .current_surface_id = Some(surface_id);
}

/// 私有状態を直接書いて実適用 k を与える（表示成立点を GPU なしで代替する）。
pub(super) fn force_applied(presenter: &mut EmoPresenter, target: TargetId, k: Option<ScaleRatio>) {
    presenter
        .targets
        .get_mut(&target)
        .expect("attach 済み target")
        .applied = k;
}

// ── 動く絵を持つバルーン（`presenter_film_tests.rs`・`presenter_stage*_tests.rs` 共有）────────
//
// 面 0 は 8×4 の全透明の枠に 2 つの動く絵（回数つき `fin.png`（3 コマ）・終わりなし `end.png`
// （2 コマ））を横に並べたもの、面 1 は静止画 1 枚の 8×4。

const BALLOON_BASE: &str = "balloon";
pub(super) const T: TargetId = TargetId(7);

fn balloon_element(layer: u32, path: &str, x: i64) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x,
        y: 0,
    }
}

/// 4×4 の不透明な単色。
fn square(c: u8) -> DecodedImage {
    DecodedImage {
        width: 4,
        height: 4,
        stride: 16,
        bgra: [c, c / 2, 255 - c, 255].repeat(16),
        has_alpha: true,
    }
}

/// 4×4 の不透明な単色で、内側の 2×2（(1,1)〜(2,2)）だけが透明。
fn holed(c: u8) -> DecodedImage {
    let mut img = square(c);
    for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
        let at = (y * 16 + x * 4) as usize;
        img.bgra[at..at + 4].copy_from_slice(&[0, 0, 0, 0]);
    }
    img
}

fn film(dec: &mut MemoryDecoder, rel: &str, frames: Vec<DecodedImage>, laps: Option<u32>) {
    let loop_count = match laps {
        None => LoopCount::Infinite,
        Some(n) => LoopCount::Finite(NonZeroU32::new(n).unwrap()),
    };
    let first = frames[0].clone();
    let frames: Vec<AnimationFrame> = frames
        .into_iter()
        .map(|image| AnimationFrame {
            image,
            delay_ms: 100,
        })
        .collect();
    dec.insert_animated(
        Path::new(BALLOON_BASE).join(rel),
        AnimationInfo {
            width: 4,
            height: 4,
            frame_count: frames.len() as u32,
        },
        Ok(first),
        Ok(AnimatedImage { frames, loop_count }),
    );
}

/// 面 0（回数つき＋終わりなし・当たり判定 1 つ）と面 1（静止画）の面の表。
pub(super) fn build_balloon() -> (EmoWorld, AtlasTable) {
    let surfaces = vec![
        Surface {
            targets: vec![areka_parsers::shell::AppendTarget::Single(0)],
            elements: vec![
                balloon_element(0, "frame.png", 0),
                balloon_element(1, "fin.png", 0),
                balloon_element(2, "end.png", 4),
            ],
            collisions: vec![Collision {
                index: 0,
                left: 4,
                top: 0,
                right: 8,
                bottom: 4,
                name: CollisionName::new("Body".to_string()),
            }],
            animations: Vec::new(),
        },
        Surface {
            targets: vec![areka_parsers::shell::AppendTarget::Single(1)],
            elements: vec![balloon_element(0, "s1.png", 0)],
            collisions: Vec::new(),
            animations: Vec::new(),
        },
    ];
    let mut dec = MemoryDecoder::new();
    // 外形は element の原寸で決まる（置いた X は足さない）ので、8×4 の全透明の枠を敷いて横に並べる。
    dec.insert(
        Path::new(BALLOON_BASE).join("frame.png"),
        8,
        4,
        32,
        vec![0; 128],
        true,
    );
    film(
        &mut dec,
        "fin.png",
        vec![square(20), square(90), square(160)],
        Some(1),
    );
    film(&mut dec, "end.png", vec![square(200), holed(240)], None);
    // 8×4 の不透明な単色（色が 1 つなので 4×4 の 2 枚ぶんをそのまま並べてよい）。
    let still = square(60).bgra.repeat(2);
    dec.insert(
        Path::new(BALLOON_BASE).join("s1.png"),
        8,
        4,
        32,
        still,
        true,
    );
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BALLOON_BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake_with_limits(
        &[set],
        &dec,
        PackConfig::default(),
        AnimationLimits::default(),
    );
    assert!(baked.errors.is_empty(), "{:?}", baked.errors);
    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// (回数つきの子, 終わりなしの子)。
fn sheets(world: &EmoWorld) -> (FilmSheet, FilmSheet) {
    let all: Vec<&FilmSheet> = world.film_sheets().collect();
    assert_eq!(
        all.len(),
        2,
        "前提: 面 0 の動く絵が 2 つとも子へ分解されている"
    );
    let fin = all.iter().find(|s| s.laps.is_some()).expect("回数つき");
    let end = all.iter().find(|s| s.laps.is_none()).expect("終わりなし");
    ((*fin).clone(), (*end).clone())
}

/// 回数つきのコマ `fin`・終わりなしのコマ `end` を欄に載せたコマ（`None` は載せない＝経過 0）。
pub(super) fn frames(world: &EmoWorld, fin: Option<usize>, end: Option<usize>) -> PatternState {
    let (f, e) = sheets(world);
    let mut p = PatternState::default();
    if let Some(i) = fin {
        p.set_film(f.id, f.frames[i]);
    }
    if let Some(i) = end {
        p.set_film(e.id, e.frames[i]);
    }
    p
}

pub(super) fn golden(surface_id: u32, binds: &BindSet, pattern: &PatternState) -> Vec<u8> {
    let (world, atlas) = build_balloon();
    native_golden_with(&world, &atlas, surface_id, binds, pattern).0
}

pub(super) fn attach(
    presenter: &mut EmoPresenter,
    world: &mut World,
    ownership: VisibilityOwnership,
) {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas) = build_balloon();
    presenter.attach_target(T, window, emo_world, atlas, 96);
    presenter
        .set_visibility_ownership(T, ownership)
        .expect("set_visibility_ownership");
}

/// 指令を 1 つ適用し、応答がちょうど 1 回 `Ok` で返ることを確かめる。
pub(super) fn send(
    presenter: &mut EmoPresenter,
    world: &mut World,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: T,
            surface_id,
            binds,
            pattern,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.try_recv(), Ok(Some(Ok(())))),
        "応答が 1 回 Ok で返っていない"
    );
    assert!(
        matches!(rx.try_recv(), Err(areka_actor::ReplyError::Dropped)),
        "応答が 2 回以上返る（または送り手が残っている）"
    );
}

/// 記録の行のフィールドを引く（無ければ全フィールドを添えて落とす＝欠落が判る形）。
pub(super) fn field(ev: &CapturedEvent, name: &str) -> String {
    ev.field(name)
        .unwrap_or_else(|| panic!("行にフィールド `{name}` が無い: {:?}", ev.fields))
        .to_string()
}

/// (表示が成立した回数, そのうち合成した回数)。
pub(super) fn applies(events: &[CapturedEvent]) -> (usize, usize) {
    let infos: Vec<_> = events
        .iter()
        .filter(|e| e.message() == INFO_LINE_MESSAGE)
        .collect();
    let composed = infos
        .iter()
        .filter(|e| e.field("cache_hit") == Some("false"))
        .count();
    (infos.len(), composed)
}

/// 外から所有されるバルーンを面 0（経過 0）で確立する（見えないまま）。
pub(super) fn established(world: &mut World) -> EmoPresenter {
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, world, VisibilityOwnership::External);
    send(
        &mut presenter,
        world,
        0,
        BindSet::default(),
        PatternState::default(),
    );
    assert_eq!(presenter.target_visible(T), Some(false));
    presenter
}

/// 外から所有されるバルーンを面 0（経過 0）で出し、合図を今の出番の世代に追い付かせる
/// （seriko が新しい出番を知った後の状態・task 4.3）。
pub(super) fn shown(world: &mut World) -> EmoPresenter {
    let mut presenter = established(world);
    presenter.show_target(world, T).expect("show_target");
    assert_eq!(presenter.target_visible(T), Some(true));
    let generation = presenter.stage_generation(T).expect("装着済み");
    ack(&mut presenter, world, T, generation);
    presenter
}

/// 合図 `StageAck` を 1 つ適用する（応答なし）。
pub(super) fn ack(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    generation: u64,
) {
    presenter.apply(world, PresentCommand::StageAck { target, generation });
}

pub(super) fn hide(presenter: &mut EmoPresenter, world: &mut World) {
    presenter.apply(
        world,
        PresentCommand::Hide {
            target: T,
            reply: None,
        },
    );
}

// ── ログ捕捉ハーネス（presenter 系テストモジュール共有）────────────────────────────────
//
// 捕捉窓そのものは本 module に持たず、硬化機構の唯一の定義元 `log-capture-kit` へ委譲する
// （spec: areka-P0-test-cage-determinism・要件 1.5／2.2）。ここに在った最小 subscriber
// （`CaptureSubscriber`）と visitor は撤去した——同型が本 crate の `scale.rs`・`balloon.rs`・
// `presenter/timing.rs` の檻にも写し取られており、写し損ねた側だけが静かに嘘をつく形だった。
//
// # 「スレッドローカルゆえ安全」は誤り（起きるのは混入ではなく取りこぼし）
//
// `tracing::subscriber::with_default` が差し替えるのはスレッドローカルの既定 dispatcher
// だけで、そこは確かにスレッドごとに独立している。しかし「そのログを評価するか」を決める
// callsite の **interest キャッシュはプロセス全体で 1 つ**であり、その発行点をプロセス内で
// 最初に踏んだスレッドの判定が焼き付く。捕捉窓を持たないスレッドの既定は `NoSubscriber` で
// 判定は「不要」なので、先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を
// 差していても以後そのイベントは早期 return で捨てられる。ゆえに危険なのは「他テストの
// イベントを取り込むこと」ではなく、**自分が見るはずのイベントを取りこぼすこと**である。
// 不在の主張は捕捉 0 件のまま静かに緑になり（偽陰性）、存在の主張は捕捉 0 件で確率的に
// 赤になる（偽陽性）。
//
// [`log_capture_kit::capture`] はこれを ⑴ プロセス寿命の probe 常駐 ⑵ 窓の内側での interest
// 再計算 ⑶ 窓の内側で発火する対照イベント（番兵）による空振り検出、の 3 点で塞ぐ。番兵は
// 返却前に取り除かれるので、呼出側の件数・主張は変わらない。捕捉されるのは呼出スレッドで
// 同期的に発火したイベントだけである点は移行前と同じで、`set_global_default` によるプロセス
// 大域の据え付けは使わない。機序の逐条解説は `log_capture_kit` の crate doc と同 crate の
// `src/probe.rs` にある。
//
// presenter 系の複数テストモジュールが同じ道具を要するため、steering `structure.md` の
// 「テーマ間で共有するヘルパは `<stem>_test_support.rs` へ集約する」に従い、引き口をここへ
// 1 本化する（`field_names()` は正準型の `field_names_sorted()` が同じ列を返す）。

pub(super) use log_capture_kit::{CapturedEvent, capture};
