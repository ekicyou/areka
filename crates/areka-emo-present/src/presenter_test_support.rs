use super::*;

use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, ComposeMethod, PatternFrame};
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};

use wintf::ecs::{GraphicsCore, WucGraphicsResource};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

// ── GPU/WUC フィクスチャ（chain.rs / mount.rs / wuc_resource.rs テストと同一方針）──────────
// 本番 UI スレッドは MTA（メモリ「areka WUC は MTA スレッドで動く」）。WucGraphicsResource::new は
// DQTAT_COM_NONE（apartment 不変）でディスパッチャを組むため、COM を MTA 初期化してから呼ぶ。

/// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World を組む。
///
/// `EmoPresenter` は供給面生成時に World から両資源を読む（compositor は `WucGraphicsResource` 由来）。
/// ゆえに本番同様、World へ両者を挿入した状態を作る。
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

/// `build_target_assets` と同一入力の **native 合成結果を `scale` 倍**した表示用サーフェスの
/// バイト列（k≠1 表示の golden）。
///
/// presenter が辿るのと同じ `Composer::compose`（native）→ `resample`（k 適用）の 2 段を、
/// テスト側で独立に再現する。「readback が偶然それらしい寸法になった」ではなく
/// **k 適用後のバイトそのもの**を固定する。
pub(super) fn scaled_golden(
    emo_world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    scale: ScaleRatio,
) -> (Vec<u8>, (u32, u32), (u32, u32)) {
    let g = scaled_golden_with(
        emo_world,
        atlas,
        surface_id,
        &BindSet::default(),
        &PatternState::default(),
        scale,
    );
    (g.scaled, g.native_size, g.scaled_size)
}

/// [`scaled_golden_with`] の返り値（k 適用**前後**のバイトと外形）。
pub(super) struct ScaledGolden {
    /// k 適用後（＝表示相当）のバイト列。
    pub(super) scaled: Vec<u8>,
    /// k 適用前（native 合成そのもの）のバイト列。
    pub(super) native: Vec<u8>,
    /// native 外形。
    pub(super) native_size: (u32, u32),
    /// k 適用後外形（`scaled_extent(scale, native_size)` と厳密一致する）。
    pub(super) scaled_size: (u32, u32),
}

/// [`scaled_golden`] の一般形（**任意の bind 集合・pattern** で合成してから k を 1 回掛ける）。
///
/// native バイトも返すのは、「k 適用後の画素が native のどの画素に由来するか」を座標で
/// 突き合わせる相対配置の檻（[`show_surface_scales_layered_bind_and_pattern_content_with_single_k`]）
/// が要るためである。
pub(super) fn scaled_golden_with(
    emo_world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
    scale: ScaleRatio,
) -> ScaledGolden {
    let mut composer = Composer::new();
    let native = composer
        .compose(emo_world, atlas, surface_id, binds, pattern)
        .expect("golden 用の native 合成は Ok");
    let native_size = (native.width(), native.height());
    let native_bytes = native.bytes().to_vec();
    let mut scaled = ComposedSurface::new(0, 0);
    resample(&native, scale, &mut scaled);
    let scaled_size = (scaled.width(), scaled.height());
    ScaledGolden {
        scaled: scaled.bytes().to_vec(),
        native: native_bytes,
        native_size,
        scaled_size,
    }
}

/// premultiplied BGRA 密配列（`stride = width * 4`）から 1 画素を取り出す（座標突合の読み口）。
pub(super) fn px_at(bytes: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    bytes[i..i + 4]
        .try_into()
        .expect("密配列ゆえ 4 バイト取り出せる")
}

/// 有効 `ShowSurface` を適用し、reply が `Ok(())` であることを確認する（テスト補助）。
pub(super) fn show_ok(presenter: &mut EmoPresenter, world: &mut World, target: TargetId, surface_id: u32) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "ShowSurface（surface {surface_id}）が Ok でない"
    );
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

// ── ComposedSurface 生成補助（chain.rs テストと同技法）──────────────────────────────────
// `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接焼けない。
// 上流公開 API（atlas bake → EmoWorld → Composer::compose）で本物を合成して得る。

pub(super) fn elem(path: &str, x: i64, y: i64) -> Element {
    Element {
        layer: 0,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

pub(super) fn surface(id: u32, elements: Vec<Element>) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

pub(super) fn shell_of(surfaces: Vec<Surface>) -> Shell {
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

/// surface 1000 = 単一 element（`w×h` 全不透明・座標由来グラデーション）の `(EmoWorld, AtlasTable)`
/// と、同一入力を `Composer::compose` で直接合成した golden バイト列を返す。
///
/// α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は正確に `w×h`。golden は presenter が
/// 内部で辿るのと同一の world/atlas から作るため、readback とのバイト一致が二重に決定論的になる。
pub(super) fn build_target_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable, Vec<u8>) {
    let base = Path::new("shell/master");
    let surfaces = vec![surface(1000, vec![elem("p.png", 0, 0)])];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let a: u8 = 0xFF;
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            img.push(b);
            img.push(g);
            img.push(r);
            img.push(a);
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
    assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    let atlas = baked.table;

    // golden: presenter と同一入力を直接合成（move 前に計算する）。
    let mut composer = Composer::new();
    let golden = composer
        .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
        .expect("静的 element 単体の合成は Ok");
    let golden_bytes = golden.bytes().to_vec();

    (world, atlas, golden_bytes)
}

/// surface 1000／3000 = 同 `w×h`・全不透明・**別バイト**（別 element・別 salt）を持つ単一
/// world の `(EmoWorld, AtlasTable)` と、各面の直接合成 golden 2 本を返す（build_target_assets の
/// 複面版）。
///
/// 両面とも α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は両面とも正確に `w×h`
/// （＝同寸）。ゆえに供給面（chain）リサイズ経路を踏まずに「同寸・異 id 再 Show」だけを固定できる。
/// golden は presenter が内部で辿るのと同一 world/atlas から作るため readback とのバイト一致が
/// 二重に決定論的。2 面の golden が別物であることを fixture 自身が assert する（R6.1 の回帰檻前提）。
pub(super) fn build_two_face_assets(w: u32, h: u32) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
    let base = Path::new("shell/master");
    let surfaces = vec![
        surface(1000, vec![elem("p.png", 0, 0)]),
        surface(3000, vec![elem("q.png", 0, 0)]),
    ];

    let stride = w * 4;
    let gradient = |salt: u8| -> Vec<u8> {
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.extend_from_slice(&[b, g, r, 0xFF]);
            }
        }
        img
    };

    let mut dec = MemoryDecoder::new();
    dec.insert(base.join("p.png"), w, h, stride, gradient(0x11), true);
    dec.insert(base.join("q.png"), w, h, stride, gradient(0x77), true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    let atlas = baked.table;

    let mut composer = Composer::new();
    let golden_1000 = composer
        .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
        .expect("面 1000 の合成は Ok")
        .bytes()
        .to_vec();
    let golden_3000 = composer
        .compose(&world, &atlas, 3000, &BindSet::default(), &PatternState::default())
        .expect("面 3000 の合成は Ok")
        .bytes()
        .to_vec();
    assert_ne!(
        golden_1000, golden_3000,
        "fixture 前提: 同寸でも 2 面のバイトが異ならなければ再表示の回帰檻にならない"
    );

    (world, atlas, golden_1000, golden_3000)
}

/// [`pattern_overlay`] の一般形（現在コマの重ね位置 `(x, y)` を指定する）。
///
/// 非ゼロ `(x, y)` は SERIKO アニメの実 pattern（`surfaces.txt` の `animationN.patternM` が持つ
/// 座標）と同型であり、k 追従の相対配置檻が要求する**非対称な重ね位置**を作る。
pub(super) fn pattern_overlay_at(anim_id: u32, surf: u32, x: i64, y: i64) -> PatternState {
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

// ── hit_region_client の配線と縮退の檻（タスク 3.2・要件 1.4-1.7・DD-5）─────────────────
//
// 本節はすべて **GPU 非依存**である。`attach_target` は skeleton 登録のみで World にも GPU にも
// 触れないため素の `World::new()` で足り、判定に必要な状態（表示中サーフェス・実適用 k）は
// in-source テストの特権である**私有フィールドの直接構築**で作る。実表示（`ShowSurface`）で
// 作ろうとすると GPU/WUC 資源が要り、かつ「面はあるのに applied が無い」状態は表示成立点が
// 両者を同時に確定させるため**原理的に作れない**（DD-5 が「現行の公開 API 経由では到達不能な
// 防御分岐」と述べるとおり）。私有状態の直接構築だけがこの分岐を実行テストへ入れる手段である。

use areka_parsers::shell::{Collision, CollisionName};

/// 当たり判定矩形 1 件（`hit.rs` の檻と同一の作り方）。
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
/// （本節は判定だけを見るので画素は不要）。矩形は `hit.rs` の檻と同一の Head/Bust に、
/// **Bust と重なる Arm**（後定義＝画家則で手前）を足した 3 件で、重なり点の檻を成立させる。
fn build_collision_only_assets() -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![Surface {
        id: 1000,
        targets: vec![AppendTarget::Single(1000)],
        elements: Vec::new(),
        collisions: vec![
            hit_coll(0, 93, 62, 271, 130, "Head"),
            hit_coll(1, 133, 270, 229, 326, "Bust"),
            hit_coll(2, 200, 300, 400, 400, "Arm"),
        ],
        animations: Vec::new(),
    }];

    let dec = MemoryDecoder::new();
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
        "element 無し surface の atlas bake は失敗しない: {:?}",
        baked.errors
    );

    (EmoWorld::build(&shell_of(surfaces)), baked.table)
}

/// `attach_target` 済み（未表示）の target を 1 つ登録する。
pub(super) fn attach_hit_target(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas) = build_collision_only_assets();
    presenter
        .attach_target(world, target, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
}

/// 私有状態を直接書いて「表示中サーフェスあり」を作る（GPU なしで R1.6 分岐へ到達する唯一の手段）。
pub(super) fn force_current_surface(presenter: &mut EmoPresenter, target: TargetId, surface_id: u32) {
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
