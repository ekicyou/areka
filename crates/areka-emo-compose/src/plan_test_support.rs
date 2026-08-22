use super::*;
use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::{
    Animation, AppendTarget, DefRef, DrawMethod, Element, ElementPath, Interval, Pattern, Shell,
    SortOrder, Surface,
};
use std::path::Path;

// ---- 合成モデルビルダ（task 4 テストと同一パターン・実 fixture パースは統合 task 8 送り）----

/// element 1 本（layer/path/x/y 指定）。
pub(super) fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

/// element のみを持つ最小 surface（collision/animation 空・単一ターゲット）。
pub(super) fn surface(id: u32, elements: Vec<Element>) -> Surface {
    surface_with_anims(id, elements, Vec::new())
}

/// element＋animation を持つ surface（collision 空・単一ターゲット）。
pub(super) fn surface_with_anims(
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
pub(super) fn bind_anim(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
    Animation {
        id,
        interval: Interval::Bind,
        patterns: vec![
            Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: ref_surface_id,
                wait: 0,
                x,
                y,
            },
            // pattern0 以外は本 task では未使用（pattern0 のみ合成対象・要件 5.2）。
            Pattern {
                index: 5,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 999_999,
                wait: 0,
                x: 7,
                y: 7,
            },
        ],
    }
}

/// 与えた surface 群を登場順 definitions（plain のみ）で包む `Shell`（animation-sort 未指定＝既定 descend）。
pub(super) fn shell_of(surfaces: Vec<Surface>) -> Shell {
    shell_of_with_sort(surfaces, None)
}

/// 与えた surface 群を `animation-sort` 指定つきで包む `Shell`。
pub(super) fn shell_of_with_sort(
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

/// tightly-packed 2×2 の premultiplied BGRA・不透明（α=255）画像スペック。
pub(super) fn opaque_2x2() -> (u32, u32, u32, Vec<u8>, bool) {
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
pub(super) fn register(dec: &mut MemoryDecoder, base: &Path, rel: &str) {
    let (w, h, stride, bgra, has_alpha) = opaque_2x2();
    dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
}

/// 指定 rel_path 群を含む単一 SurfaceSet を bake し `AtlasTable` を得る（COM/WIC 非依存・SetId(0)）。
pub(super) fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
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
pub(super) fn id_to_path<'a>(atlas: &AtlasTable, rels: &[&'a str]) -> Vec<(ElementId, &'a str)> {
    rels.iter()
        .map(|r| {
            let id = atlas
                .resolve(SetId(0), r)
                .unwrap_or_else(|| panic!("{r} は bake 済みで resolve できる"));
            (id, *r)
        })
        .collect()
}
