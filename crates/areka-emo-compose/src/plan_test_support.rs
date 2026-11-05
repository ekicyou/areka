//! 命令列ほかのテストが引く部品（クレートの直下・`#[cfg(test)]` 限定）。
//!
//! 面の組み立て・コマ・当たり判定の領域・頁と表・塗りの絵・画素の読み・PNG の原寸・焼き・記録の
//! 読み取り・命令の名前引き・命令列を引く口（[`ops`]）を 1 か所に置く。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, AtlasEntry,
    AtlasKey, AtlasPage, AtlasTable, DecodedImage, ElementId, LoopCount, MemoryDecoder, PackConfig,
    Placement, Point, Rect, SetId, Size, SurfaceSet, UseSelfAlpha, bake, bake_with_limits,
};
use areka_parsers::shell::{
    Animation, AppendTarget, Collision, CollisionName, DefRef, DrawMethod, Element, ElementPath,
    Interval, Pattern, Shell, SortOrder, Surface, SurfaceAppend, parse,
};

use crate::bind::BindSet;
use crate::composed::ComposedSurface;
use crate::method::ComposeMethod;
use crate::normalized::SurfaceMaster;
use crate::pattern::{PatternFrame, PatternState};
use crate::plan::{BlitOp, PlanScratch, derive_ops};
use crate::world::EmoWorld;

// ---- 合成モデルビルダ（task 4 テストと同一パターン・実 fixture パースは統合 task 8 送り）----

/// element 1 本（layer/path/x/y 指定）。
pub(crate) fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

/// element のみを持つ最小 surface（collision/animation 空・単一ターゲット）。
pub(crate) fn surface(id: u32, elements: Vec<Element>) -> Surface {
    surface_with_anims(id, elements, Vec::new())
}

/// ターゲット記述子とボディ element 群から plain surface 定義を組み立てる。
pub(crate) fn surface_def(id: u32, targets: Vec<AppendTarget>, elements: Vec<Element>) -> Surface {
    Surface {
        id,
        targets,
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

/// element＋animation を持つ surface（collision 空・単一ターゲット）。
pub(crate) fn surface_with_anims(
    id: u32,
    elements: Vec<Element>,
    animations: Vec<Animation>,
) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations,
    }
}

/// bind animation 1 本（interval=`Interval::Bind`・pattern0 が入れ子 surface_id を x,y で参照）。
///
/// pattern index は 0 を pattern0（先頭）とし、識別用に index=5 の余分な pattern も加えて
/// 「pattern0＝最小 index の pattern」を選ぶ実装を突く（疎 index 許容・model 契約）。
pub(crate) fn bind_anim(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
    Animation {
        id,
        interval: Interval::Bind,
        interval_written: true,
        patterns: vec![
            Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: ref_surface_id,
                target: ref_surface_id.to_string(),
                wait: 0,
                x,
                y,
            },
            // pattern0 以外は本 task では未使用（pattern0 のみ合成対象・要件 5.2）。
            Pattern {
                index: 5,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 999_999,
                target: "999999".into(),
                wait: 0,
                x: 7,
                y: 7,
            },
        ],
    }
}

/// bind animation を「指定 index の pattern 1 本だけ」で組む（pattern0=index0 の有無を制御する）。
///
/// `bind_anim` は index=0 と index=5 を持つが、本ヘルパは **単一 index** の pattern のみを載せる。
/// index を 1 以上にすれば「pattern0（index==0）を持たない再生アニメ」（まばたき相当）を作れる。
pub(crate) fn single_pattern_anim(
    id: u32,
    interval: Interval,
    index: u32,
    ref_surface_id: i64,
) -> Animation {
    Animation {
        id,
        interval,
        interval_written: true,
        patterns: vec![Pattern {
            index,
            method: DrawMethod::new("overlay".to_string()),
            surface_id: ref_surface_id,
            target: ref_surface_id.to_string(),
            wait: 0,
            x: 0,
            y: 0,
        }],
    }
}

/// 与えた surface 群を登場順 definitions（plain のみ）で包む `Shell`（animation-sort 未指定＝既定 descend）。
pub(crate) fn shell_of(surfaces: Vec<Surface>) -> Shell {
    shell_of_with_sort(surfaces, None)
}

/// 与えた surface 群を `animation-sort` 指定つきで包む `Shell`。
pub(crate) fn shell_of_with_sort(
    surfaces: Vec<Surface>,
    animation_sort: Option<SortOrder>,
) -> Shell {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort,
        collision_sort: None,
        definitions,
    }
}

/// `Shell` を surface/append を混在させた登場順 `definitions` で組む。
///
/// `defs` は登場順に `DefRef` を並べたもの（呼び手が `DefRef::Surface(i)`／`DefRef::Append(i)`
/// を直接指定する）。surfaces/appends の各 Vec には index が既に対応している前提。
pub(crate) fn shell_mixed(
    surfaces: Vec<Surface>,
    appends: Vec<SurfaceAppend>,
    defs: Vec<DefRef>,
) -> Shell {
    Shell {
        surfaces,
        appends,
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions: defs,
    }
}

/// tightly-packed 2×2 の premultiplied BGRA・不透明（α=255）画像スペック。
pub(crate) fn opaque_2x2() -> (u32, u32, u32, Vec<u8>, bool) {
    let (w, h) = (2u32, 2u32);
    let stride = w * 4;
    let bgra = vec![
        10, 20, 30, 255, //
        40, 50, 60, 255, //
        70, 80, 90, 255, //
        100, 110, 120, 255,
    ];
    (w, h, stride, bgra, true)
}

/// `MemoryDecoder` へ base.join(rel) の実パスで不透明画像を登録する。
pub(crate) fn register(dec: &mut MemoryDecoder, base: &Path, rel: &str) {
    let (w, h, stride, bgra, has_alpha) = opaque_2x2();
    dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
}

/// 指定 rel_path 群を含む単一 SurfaceSet を bake し `AtlasTable` を得る（COM/WIC 非依存・SetId(0)）。
pub(crate) fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
    let elements: Vec<Element> = rels.iter().map(|r| elem(0, r, 0, 0)).collect();
    let surfaces = vec![surface(0, elements)];
    let mut dec = MemoryDecoder::new();
    for r in rels {
        register(&mut dec, base, r);
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let result = bake(&[set], &dec, PackConfig::default());
    assert!(result.errors.is_empty(), "bake セットアップは失敗しない");
    result.table
}

/// element を、その `path` の ElementId から逆引きするための対応表を作る（テスト補助）。
///
/// BlitOp は ElementId のみを保持するため、命令列の順序を element の識別子（ここでは path）
/// へ写像して検証する。bake 済み atlas は path→ElementId が一意ゆえ、ElementId→path も一意。
pub(crate) fn id_to_path<'a>(atlas: &AtlasTable, rels: &[&'a str]) -> Vec<(ElementId, &'a str)> {
    rels.iter()
        .map(|r| {
            let id = atlas
                .resolve(SetId(0), r)
                .unwrap_or_else(|| panic!("{r} は bake 済みで resolve できる"));
            (id, *r)
        })
        .collect()
}

/// 命令列を element の名前の列へ逆写像する（ElementId→path が一意な bake 前提）。
pub(crate) fn ops_to_paths(ops: &[BlitOp], map: &[(ElementId, &str)]) -> Vec<String> {
    ops.iter()
        .map(|op| {
            map.iter()
                .find(|(id, _)| *id == op.element)
                .map(|(_, p)| p.to_string())
                .expect("op の ElementId は既知")
        })
        .collect()
}

/// 命令列を (画像の名前, X, Y) の列で返す（命令の名前引き）。
pub(crate) fn named_ops<'a>(
    ops: &[BlitOp],
    map: &[(ElementId, &'a str)],
) -> Vec<(&'a str, i64, i64)> {
    ops.iter()
        .map(|op| {
            let name = map
                .iter()
                .find(|(id, _)| *id == op.element)
                .map(|(_, p)| *p)
                .expect("命令の ElementId は既知");
            let (x, y) = op.transform.offset();
            (name, x, y)
        })
        .collect()
}

/// 命令列を引く口: `surface_id` を合成した命令列を新しい `Vec` で返す。
pub(crate) fn ops(
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> Vec<BlitOp> {
    let mut out = Vec::new();
    let mut scratch = PlanScratch::default();
    derive_ops(
        &mut out,
        &mut scratch,
        world,
        atlas,
        surface_id,
        binds,
        pattern,
    );
    assert!(
        scratch.visited.is_empty(),
        "先祖の積み上げは走査後に空へ戻る"
    );
    out
}

/// `surface_id` を (x,y) に置く Overlay のコマ。
pub(crate) fn frame(surface_id: u32, x: i64, y: i64) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x,
        y,
    }
}

/// 当たり判定の領域 1 つ。
pub(crate) fn coll(
    index: u32,
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
    name: &str,
) -> Collision {
    Collision {
        index,
        left,
        top,
        right,
        bottom,
        name: CollisionName::new(name.to_string()),
    }
}

/// 当たり判定の領域の列だけを持つ面 1000。
pub(crate) fn master_with(collisions: Vec<Collision>) -> SurfaceMaster {
    SurfaceMaster {
        id: 1000,
        elements: Vec::new(),
        collisions,
        animations: Vec::new(),
    }
}

/// premultiplied BGRA 1 画素頁を組む（stride = width*4・tightly packed）。
pub(crate) fn page_1px(b: u8, g: u8, r: u8, a: u8) -> AtlasPage {
    AtlasPage {
        width: 1,
        height: 1,
        stride: 4,
        bytes: Arc::from(vec![b, g, r, a]),
    }
}

/// 単一エントリ（`ElementId(0)`）＋単一頁の AtlasTable を組む（original は uv.w×uv.h）。
pub(crate) fn single_entry_table(page: AtlasPage, uv: Rect, trim: Point) -> AtlasTable {
    let keys = vec![AtlasKey {
        set: SetId(0),
        rel_path: "e.png".into(),
    }];
    let entries = vec![AtlasEntry {
        original: Size { w: uv.w, h: uv.h },
        placement: Some(Placement {
            page: 0,
            uv_rect: uv,
            trim_offset: trim,
        }),
    }];
    AtlasTable::new(keys, entries, vec![page])
}

/// 合成先 out の (x,y) 画素 [B,G,R,A] を読む（stride=w*4 前提）。
pub(crate) fn px(out: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    let i = (y * out.stride() + x * 4) as usize;
    let b = out.bytes();
    [b[i], b[i + 1], b[i + 2], b[i + 3]]
}

/// 全面が 1 色・不透明の `w`×`h`（色 `c` でコマを見分ける）。
pub(crate) fn solid(w: u32, h: u32, c: u8) -> DecodedImage {
    DecodedImage {
        width: w,
        height: h,
        stride: w * 4,
        bgra: [c, c, c, 255].repeat((w * h) as usize),
        has_alpha: true,
    }
}

/// PNG ファイルの IHDR から `(width, height)` を読む（COM/WIC 非依存の実原寸取得）。
///
/// PNG は先頭 8 バイトが署名、続く IHDR チャンク（length[4]+"IHDR"[4]）の直後に width[4]・
/// height[4]（ビッグエンディアン u32）が並ぶ。ゆえに width はオフセット 16、height は 20 から読める。
pub(crate) fn png_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("実 PNG を読めない: {}: {e}", path.display()));
    assert!(
        bytes.len() >= 24 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR",
        "PNG 署名／IHDR が期待どおり: {}",
        path.display()
    );
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    assert!(w > 0 && h > 0, "実 PNG 原寸は非ゼロ: {}", path.display());
    (w, h)
}

/// tightly-packed w×h の premultiplied BGRA・不透明（α=255・単色 (b,g,r)）画像スペックを組む。
pub(crate) fn opaque_wxh(w: u32, h: u32, b: u8, g: u8, r: u8) -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = w * 4;
    (w, h, stride, [b, g, r, 255].repeat((w * h) as usize), true)
}

/// tightly-packed w×h の premultiplied BGRA・**全透明**（α=0）画像スペックを組む。
///
/// trim は bbox を得られず `placement: None` になるが、`original` は (w,h) を記録する。
pub(crate) fn transparent_wxh(w: u32, h: u32) -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = w * 4;
    (w, h, stride, vec![0u8; (stride * h) as usize], true)
}

/// 大きさ指定の焼きの 1 件: (相対パス, (w,h), 単色 (b,g,r) か全透明 None)。
pub(crate) type AtlasSpec<'a> = (&'a str, (u32, u32), Option<(u8, u8, u8)>);

/// [`AtlasSpec`] 群から `AtlasTable` を bake する（大きさ指定の焼き）。
///
/// `color` が `Some((b,g,r))` なら不透明単色、`None` なら全透明（placement None）。
pub(crate) fn bake_atlas_sized(base: &Path, specs: &[AtlasSpec]) -> AtlasTable {
    let elements: Vec<Element> = specs.iter().map(|(r, _, _)| elem(0, r, 0, 0)).collect();
    let surfaces = vec![surface(0, elements)];
    let mut dec = MemoryDecoder::new();
    for (rel, (w, h), color) in specs {
        let (iw, ih, stride, bgra, has_alpha) = match color {
            Some((b, g, r)) => opaque_wxh(*w, *h, *b, *g, *r),
            None => transparent_wxh(*w, *h),
        };
        dec.insert(base.join(rel), iw, ih, stride, bgra, has_alpha);
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let result = bake(&[set], &dec, PackConfig::default());
    assert!(result.errors.is_empty(), "bake セットアップは失敗しない");
    result.table
}

/// 動く絵を `MemoryDecoder` へ登録する（大きさは 1 枚目のコマの絵から）。
pub(crate) fn insert_film(
    dec: &mut MemoryDecoder,
    path: PathBuf,
    frames: Vec<AnimationFrame>,
    loop_count: LoopCount,
) {
    let first = frames[0].image.clone();
    dec.insert_animated(
        path,
        AnimationInfo {
            width: first.width,
            height: first.height,
            frame_count: frames.len() as u32,
        },
        Ok(first),
        Ok(AnimatedImage { frames, loop_count }),
    );
}

/// `rels` を element に持つ面 0 で `dec` の絵（静止画と動く絵）を焼き、`text` を面の表にして束縛する
/// （動く絵の焼き）。`images` は `surface<数字>.png` として置いた絵（番号, 名前）。
pub(crate) fn bake_films_and_bind<'a>(
    dec: &MemoryDecoder,
    base: &Path,
    rels: impl Iterator<Item = &'a str>,
    text: &str,
    images: &[(u32, &str)],
) -> (EmoWorld, AtlasTable) {
    let surfaces = vec![surface(0, rels.map(|rel| elem(0, rel, 0, 0)).collect())];
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake_with_limits(
        &[set],
        dec,
        PackConfig::default(),
        AnimationLimits::default(),
    );
    assert!(baked.errors.is_empty(), "{:?}", baked.errors);
    let images: BTreeMap<u32, String> = images.iter().map(|(i, p)| (*i, p.to_string())).collect();
    let mut world = EmoWorld::build_with_images(&parse(text), &images);
    world.bind_atlas(&baked.table, SetId(0));
    assert!(world.film_skips().is_empty(), "{:?}", world.film_skips());
    (world, baked.table)
}

/// 飛ばした element定義の `debug!` の行から (親, element, 指した欄) を取り出す（記録の読み取り）。
pub(crate) fn skipped(logs: &str) -> Vec<(u32, u32, String)> {
    logs.lines()
        .filter(|l| l.contains("level=DEBUG") && l.contains("子のサーフェスを置かない"))
        .map(|l| {
            let field = |name: &str| -> String {
                let start = l.find(&format!(" {name}=")).expect(name) + name.len() + 2;
                l[start..]
                    .split(' ')
                    .next()
                    .unwrap()
                    .trim_matches('"')
                    .to_string()
            };
            (
                field("surface_id").parse().unwrap(),
                field("element").parse().unwrap(),
                field("child"),
            )
        })
        .collect()
}
