//! 命令列ほかのテストが引く部品（クレートの直下・`#[cfg(test)]` 限定）。
//!
//! 面の組み立て・コマ・当たり判定の領域・頁と表・塗りの絵・画素の読み・PNG の原寸・焼き・記録の
//! 読み取り・命令の名前引き・命令列を引く口（[`ops`]）・見える部品を引く口・面の entity の部品引き・
//! 畳み込みだけを通した素の World を 1 か所に置く。

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, AtlasEntry,
    AtlasKey, AtlasPage, AtlasTable, BakeResult, DecodedImage, ElementId, LoopCount, MemoryDecoder,
    PackConfig, Placement, Point, Rect, SetId, Size, SurfaceSet, UseSelfAlpha, bake_with_limits,
};
use areka_parsers::shell::{
    Animation, AppendTarget, Collision, CollisionName, DefRef, DrawMethod, Element, ElementPath,
    Interval, Pattern, Shell, SortOrder, Surface, SurfaceAppend, parse,
};
use bevy_ecs::component::Component;
use bevy_ecs::world::World;

use crate::bind::BindSet;
use crate::composed::ComposedSurface;
use crate::error::ComposeError;
use crate::method::ComposeMethod;
use crate::nesting::NestTable;
use crate::normalized::SurfaceMaster;
use crate::pattern::{PatternFrame, PatternState};
use crate::plan::{BlitOp, Extent, PlanScratch, build_plan, derive_ops};
use crate::world::{AliasMap, EmoWorld, SurfaceIndex};

// ---- 合成モデルビルダ（正規化モデルを直に組む・実 fixture のパースは golden_tests）----

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
pub(crate) fn surface_def(targets: Vec<AppendTarget>, elements: Vec<Element>) -> Surface {
    Surface {
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
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations,
    }
}

/// pattern 1 本（位置 (0,0)・ウェイト 0・相手の欄は面の番号の綴り）。
pub(crate) fn pattern(index: u32, method: &str, surface_id: i64) -> Pattern {
    Pattern {
        index,
        method: DrawMethod::new(method.to_string()),
        surface_id,
        target: surface_id.to_string(),
        wait: 0,
        x: 0,
        y: 0,
    }
}

/// bind animation 1 本（interval=`Interval::Bind`・pattern0 が入れ子 surface_id を x,y で参照）。
///
/// pattern index は 0 を pattern0（先頭）とし、index=5 の余分な pattern（面の表に無い 999999）も加える。
/// 純 `bind` は全部のコマをたどる（seriko-pattern-calls M6）ので pattern5 もたどるが、先が無いので何も積まない。
pub(crate) fn bind_anim(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
    Animation {
        id,
        interval: Interval::Bind,
        interval_written: true,
        patterns: vec![
            Pattern {
                x,
                y,
                ..pattern(0, "overlay", ref_surface_id)
            },
            // 面の表に無い先（たどっても何も積まない）。
            Pattern {
                x: 7,
                y: 7,
                ..pattern(5, "overlay", 999_999)
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
        patterns: vec![pattern(index, "overlay", ref_surface_id)],
    }
}

/// `surface.append` の定義 1 つ（対象と element/collision/animation）。
pub(crate) fn append_def(
    targets: Vec<AppendTarget>,
    elements: Vec<Element>,
    collisions: Vec<Collision>,
    animations: Vec<Animation>,
) -> SurfaceAppend {
    SurfaceAppend {
        targets,
        elements,
        collisions,
        animations,
    }
}

/// `surface<数字>.png` として置いた絵の対応（番号 → ファイル名）を組む。
pub(crate) fn image_map(entries: &[(u32, &str)]) -> BTreeMap<u32, String> {
    entries
        .iter()
        .map(|(id, file)| (*id, (*file).to_string()))
        .collect()
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

/// 指定 rel_path 群を含む単一 SurfaceSet を bake し `AtlasTable` を得る（COM/WIC 非依存・SetId(0)・どれも 2×2 の不透明）。
pub(crate) fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
    bake_atlas_sized(
        base,
        &rels
            .iter()
            .map(|r| (*r, (2, 2), Some((10, 20, 30))))
            .collect::<Vec<_>>(),
    )
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
    named_ops(ops, map)
        .into_iter()
        .map(|(p, _, _)| p.to_string())
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

/// `rels`（2×2 の不透明）を焼き、文面 `text`（ファイル名の慣習だけで面になる絵 `images`）の面の表を束縛して
/// `top` を合成した命令列を (画像の名前, X, Y) の列で返す。合成の間の記録も返す。
pub(crate) fn named_ops_of(
    text: &str,
    images: &[(u32, &str)],
    rels: &[&'static str],
    top: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> (Vec<(&'static str, i64, i64)>, String) {
    let atlas = bake_atlas(Path::new("shell/master"), rels);
    let world = bound_world(&parse(text), images, &atlas);
    let map = id_to_path(&atlas, rels);
    let mut out = Vec::new();
    let logs = crate::log_capture::capture_logs(|| out = ops(&world, &atlas, top, binds, pattern));
    (named_ops(&out, &map), logs)
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

/// 予算つきで命令列を引く口: 予算 `limit` の作業場 1 つ（合成器 1 つと同じ）で `surface_id` を 2 回合成し
/// （着せ替えもコマも無し）、最後の命令列と 2 回分の記録を返す（記録が合成器 1 つで 1 行かを見られる）。
pub(crate) fn ops_limited(
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    limit: usize,
) -> (Vec<BlitOp>, String) {
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let mut scratch = PlanScratch::with_limit(limit);
    let mut out = Vec::new();
    let logs = crate::log_capture::capture_logs(|| {
        for _ in 0..2 {
            out.clear();
            derive_ops(
                &mut out,
                &mut scratch,
                world,
                atlas,
                surface_id,
                &binds,
                &pattern,
            );
        }
    });
    (out, logs)
}

/// `needle` を含む `error!` の行の数（記録の読み取り）。
pub(crate) fn errors(logs: &str, needle: &str) -> usize {
    logs.lines()
        .filter(|l| l.contains("level=ERROR") && l.contains(needle))
        .count()
}

/// 入れ子の目印のサーフェス（どこからも指されない・目印の画像を 1 枚だけ持つ）。見える部品と合成の
/// 一致の檻（nesting_visible_tests・nesting_film_tests）が引く。
pub(crate) const MARKER: u32 = 900;
/// 目印のコマを置く animation の番号（どの文面にも無い番号）。
pub(crate) const MARK_ANIM: u32 = 9999;

/// `top` を一番上にしたときの見える部品（着せ替えは `binds`）。
pub(crate) fn visible_parts_of(
    table: &NestTable,
    top: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> Vec<u32> {
    let mut out = Vec::new();
    table.visible_parts(top, binds, pattern, &mut out);
    out
}

/// 計画を引く口（[`ops`] と並ぶ）: `surface_id` の計画を新しい命令列と作業場で建て、命令列と結果
/// （外形か失敗）を返す。
pub(crate) fn plan_of(
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> (Vec<BlitOp>, Result<Extent, ComposeError>) {
    let mut out = Vec::new();
    let mut scratch = PlanScratch::default();
    let result = build_plan(
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
    (out, result)
}

/// `surface_id` を (x,y) に置く Overlay のコマ（ほかの描き方は `PatternFrame { method, ..frame(..) }`）。
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

/// 2 エントリ（`ElementId(0)` は頁 0 の `dst_page`・`ElementId(1)` は頁 1 の `src_page`・どちらも 1×1 を (0,0)）の
/// AtlasTable を組む（下層 dst・上層 src の重ね合わせ）。
pub(crate) fn two_entry_table(dst_page: AtlasPage, src_page: AtlasPage) -> AtlasTable {
    let one = |page| AtlasEntry {
        original: Size { w: 1, h: 1 },
        placement: Some(Placement {
            page,
            uv_rect: Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            trim_offset: Point { x: 0, y: 0 },
        }),
    };
    let keys = ["dst.png", "src.png"].map(|rel| AtlasKey {
        set: SetId(0),
        rel_path: rel.into(),
    });
    AtlasTable::new(keys.into(), vec![one(0), one(1)], vec![dst_page, src_page])
}

/// 合成先 out の (x,y) 画素 [B,G,R,A] を読む（stride=w*4 前提・外形の外は読まない）。
pub(crate) fn px(out: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    assert!(x < out.width() && y < out.height(), "({x},{y}) は外形の外");
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
    let mut dec = MemoryDecoder::new();
    for (rel, (w, h), color) in specs {
        let (iw, ih, stride, bgra, has_alpha) = match color {
            Some((b, g, r)) => opaque_wxh(*w, *h, *b, *g, *r),
            None => transparent_wxh(*w, *h),
        };
        dec.insert(base.join(rel), iw, ih, stride, bgra, has_alpha);
    }
    bake_decoded(&dec, base, specs.iter().map(|(r, _, _)| *r))
}

/// `rels` を element に持つ面 0 で `dec` の絵（静止画と動く絵）を焼く（焼きの口・SetId(0)・焼けない絵は無い）。
pub(crate) fn bake_decoded<'a>(
    dec: &MemoryDecoder,
    base: &Path,
    rels: impl Iterator<Item = &'a str>,
) -> AtlasTable {
    let surfaces = vec![surface(0, rels.map(|rel| elem(0, rel, 0, 0)).collect())];
    let baked = bake_surfaces(&surfaces, base, dec);
    assert!(baked.errors.is_empty(), "{:?}", baked.errors);
    baked.table
}

/// 面の列 `surfaces` を `dec` の絵で焼く（SetId(0)・`use_self_alpha` は On・動く絵の上限は既定値）。
/// 焼けない絵は `errors` に残る。
pub(crate) fn bake_surfaces(surfaces: &[Surface], base: &Path, dec: &MemoryDecoder) -> BakeResult {
    let set = SurfaceSet {
        surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    bake_with_limits(
        &[set],
        dec,
        PackConfig::default(),
        AnimationLimits::default(),
    )
}

/// `(相対パス, (w,h))` の絵をどれも灰色の不透明で焼く（`shell/master` の下・大きさ指定の焼き）。
pub(crate) fn bake_gray(sizes: &[(&str, (u32, u32))]) -> AtlasTable {
    let specs: Vec<AtlasSpec> = sizes
        .iter()
        .map(|&(rel, size)| (rel, size, Some((64, 64, 64))))
        .collect();
    bake_atlas_sized(Path::new("shell/master"), &specs)
}

/// `shell` を面の表にし（`images` は `surface<数字>.png` として置いた絵（番号, 名前））、`atlas` を
/// 束縛した World を返す（束縛済みの World の口）。
pub(crate) fn bound_world(shell: &Shell, images: &[(u32, &str)], atlas: &AtlasTable) -> EmoWorld {
    let mut world = EmoWorld::build_with_images(shell, &image_map(images));
    world.bind_atlas(atlas, SetId(0));
    world
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

/// 静止画（相対パス, 幅, 高さ）を `base` の下に単色 9 の不透明で `MemoryDecoder` へ登録する。
pub(crate) fn insert_stills<'a>(
    dec: &mut MemoryDecoder,
    base: &Path,
    stills: impl IntoIterator<Item = (&'a str, u32, u32)>,
) {
    for (rel, w, h) in stills {
        dec.insert(base.join(rel), w, h, w * 4, solid(w, h, 9).bgra, true);
    }
}

/// 2×2 の動く絵を登録する（コマ i は色 10＋i・待ちは `delays`・`laps` 周で止まり `None` は終わりなし）。
pub(crate) fn insert_film_2x2(
    dec: &mut MemoryDecoder,
    path: PathBuf,
    delays: &[u32],
    laps: Option<u32>,
) {
    let frames = delays
        .iter()
        .enumerate()
        .map(|(i, &delay_ms)| AnimationFrame {
            image: solid(2, 2, 10 + i as u8),
            delay_ms,
        })
        .collect();
    let loop_count = laps.map_or(LoopCount::Infinite, |n| {
        LoopCount::Finite(NonZeroU32::new(n).expect("周の数は 1 以上"))
    });
    insert_film(dec, path, frames, loop_count);
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
    let table = bake_decoded(dec, base, rels);
    let world = bound_world(&parse(text), images, &table);
    assert!(world.film_skips().is_empty(), "{:?}", world.film_skips());
    (world, table)
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

// ---- 畳み込みだけを通した素の World（fold_tests・base_image_tests）----

/// 畳み込みが必ず読むリソース（`SurfaceIndex`・`AliasMap`）だけを備えた空 World を用意する
/// （`SurfaceImages` は無ければ画像だけの面が無いものとして読む）。
pub(crate) fn fresh_world() -> World {
    let mut world = World::new();
    world.insert_resource(SurfaceIndex::default());
    world.insert_resource(AliasMap::default());
    world
}

/// 面 `id` の entity に付いた部品 `T` を引く（面が無いか部品が付いていなければ `None`）。
pub(crate) fn component_of<T: Component>(world: &World, id: u32) -> Option<&T> {
    let entity = *world.resource::<SurfaceIndex>().0.get(&id)?;
    world.get::<T>(entity)
}

/// fold 結果の観測可能内容スナップショット（`EmoWorld` は derive Eq を持たないため手組み）。
///
/// (1) surface id 昇順（`SurfaceIndex` のキーを整列＝`EmoWorld::surface_ids` と同じ決定的順）に
/// `(id, SurfaceMaster)` を集める。`SurfaceMaster` は `PartialEq`（element/collision/animation の
/// 深い等価）。(2) alias は `AliasMap`（`BTreeMap`＝キー順が決定的）を丸ごと複製する。
/// この2値が等しければ、公開クエリ（`surface(id)`／`resolve_alias(key)`／`surface_ids()`）越しに
/// 観測できる内容がバイト等価であることを意味する（要件 1.5/10.1）。
pub(crate) type FoldSnapshot = (
    Vec<(u32, SurfaceMaster)>,
    std::collections::BTreeMap<String, Vec<u32>>,
);

/// 素の `World`（`fold_shell_with` 直呼び）から決定性比較用スナップショットを抽出する。
pub(crate) fn snapshot_world(world: &World) -> FoldSnapshot {
    // surface id を昇順に整列（`SurfaceIndex` の HashMap 反復順に依らず決定的）。
    let mut ids: Vec<u32> = world.resource::<SurfaceIndex>().0.keys().copied().collect();
    ids.sort_unstable();
    let surfaces: Vec<(u32, SurfaceMaster)> = ids
        .into_iter()
        .map(|id| {
            (
                id,
                component_of::<SurfaceMaster>(world, id)
                    .expect("id が常駐する")
                    .clone(),
            )
        })
        .collect();
    // alias は BTreeMap ゆえ複製すればキー順まで決定的。
    let aliases = world.resource::<AliasMap>().0.clone();
    (surfaces, aliases)
}
