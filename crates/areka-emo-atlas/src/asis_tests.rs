use std::path::Path;

use areka_parsers::shell::{Animation, DrawMethod, Interval, Pattern, Surface};

use super::asis_paths;
use crate::test_support::{elem, full, image, params, set, surface};
use crate::{
    AtlasTable, MemoryDecoder, PackConfig, PictureKind, SetId, SurfaceSet, UseSelfAlpha, bake,
};

const KEY: [u8; 4] = [1, 2, 3, 255];
/// 乗算しない 3×1: 左上は透過色、中は α 128 の緑、右の縁は α 0 の赤。
const STRAIGHT: [[u8; 4]; 3] = [KEY, [0, 100, 0, 128], [0, 0, 200, 0]];
/// 同じ絵の乗算済みの答え。
const PREMULTIPLIED: [[u8; 4]; 3] = [KEY, [0, 50, 0, 128], [0, 0, 0, 0]];
/// `asis` 用の絵の答え（どの `use_self_alpha` でも同じ）。
const ASIS: [[u8; 4]; 3] = [KEY, [0, 100, 0, 255], [0, 0, 200, 255]];

fn asis_elem(layer: u32, path: &str) -> areka_parsers::shell::Element {
    let mut e = elem(layer, path);
    e.method = DrawMethod::new("asis".to_string());
    e
}

fn anim(interval: Interval, method: &str, surface_id: i64) -> Animation {
    Animation {
        id: 0,
        interval,
        patterns: vec![Pattern {
            index: 0,
            method: DrawMethod::new(method.to_string()),
            surface_id,
            target: surface_id.to_string(),
            wait: 0,
            x: 0,
            y: 0,
        }],
    }
}

fn with_animations(mut s: Surface, animations: Vec<Animation>) -> Surface {
    s.animations = animations;
    s
}

fn paths_of(surfaces: &[Surface]) -> Vec<String> {
    let base = Path::new("shell");
    asis_paths(&[set(surfaces, base)])
        .into_iter()
        .map(|(_, p)| p)
        .collect()
}

#[test]
fn element_rows_with_asis_are_collected_and_others_are_not() {
    let surfaces = vec![surface(
        0,
        vec![elem(0, "body.png"), asis_elem(1, "glass.png")],
    )];
    assert_eq!(paths_of(&surfaces), ["glass.png"]);
}

#[test]
fn asis_patterns_of_normal_and_bind_animations_collect_the_single_picture_of_the_face() {
    let surfaces = vec![
        with_animations(
            surface(0, vec![elem(0, "surface0.png")]),
            vec![
                anim(Interval::Runonce, "asis", 10),
                anim(Interval::Bind, "ASIS", 11),
                anim(Interval::Runonce, "overlay", 12),
            ],
        ),
        surface(10, vec![elem(0, "surface10.png")]),
        surface(11, vec![elem(0, "surface11.png")]),
        surface(12, vec![elem(0, "surface12.png")]),
    ];
    assert_eq!(paths_of(&surfaces), ["surface10.png", "surface11.png"]);
}

#[test]
fn asis_element_rows_naming_a_face_collect_its_single_picture_not_the_number() {
    let surfaces = vec![
        surface(0, vec![asis_elem(0, "10"), asis_elem(1, "11")]),
        surface(10, vec![elem(0, "surface10.png")]),
        surface(11, vec![elem(0, "a.png"), elem(1, "b.png")]),
    ];
    assert_eq!(paths_of(&surfaces), ["surface10.png"]);
}

#[test]
fn a_face_with_several_pictures_is_drawn_from_its_elements_and_gets_no_asis_picture() {
    let surfaces = vec![
        with_animations(
            surface(0, vec![elem(0, "surface0.png")]),
            vec![anim(Interval::Runonce, "asis", 10)],
        ),
        surface(10, vec![elem(1, "part.png")]),
        surface(10, vec![elem(0, "surface10.png")]),
    ];
    assert!(paths_of(&surfaces).is_empty());
}

#[test]
fn a_set_without_asis_gets_no_asis_picture() {
    let surfaces = vec![with_animations(
        surface(0, vec![elem(0, "surface0.png")]),
        vec![anim(Interval::Runonce, "overlay", 0)],
    )];
    assert!(paths_of(&surfaces).is_empty());
}

fn decoder(base: &Path, rel: &str) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    dec.insert(base.join(rel), 3, 1, 12, PREMULTIPLIED.concat(), true);
    dec.insert_unmultiplied(base.join(rel), image(3, 1, true, &STRAIGHT));
    dec
}

fn bake_one(surfaces: &[Surface], use_self_alpha: UseSelfAlpha) -> AtlasTable {
    let base = Path::new("shell");
    let sets = [SurfaceSet {
        surfaces,
        base_dir: base,
        alpha_params: params(use_self_alpha),
    }];
    let result = bake(&sets, &decoder(base, "glass.png"), PackConfig::default());
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    result.table
}

fn assert_full_size_asis(table: &AtlasTable) {
    let id = table
        .resolve_asis(SetId(0), "glass.png")
        .expect("asis picture baked");
    assert_eq!(table.key(id).kind, PictureKind::Asis);
    let placement = table.entry(id).placement.clone().expect("placed");
    assert_eq!((placement.trim_offset.x, placement.trim_offset.y), (0, 0));
    assert_eq!((placement.uv_rect.w, placement.uv_rect.h), (3, 1));
    assert_eq!(full(table, id), ASIS);
}

#[test]
fn asis_picture_keeps_the_transparent_edge_the_normal_picture_trims() {
    let surfaces = vec![surface(0, vec![asis_elem(0, "glass.png")])];
    let table = bake_one(&surfaces, UseSelfAlpha::On);
    let normal = table.resolve(SetId(0), "glass.png").expect("normal baked");
    let trimmed = table.entry(normal).placement.clone().expect("placed");
    assert_eq!((trimmed.uv_rect.w, trimmed.uv_rect.h), (2, 1));
    assert_full_size_asis(&table);
}

#[test]
fn asis_picture_keeps_the_key_colour_the_normal_picture_clears() {
    let surfaces = vec![surface(0, vec![asis_elem(0, "glass.png")])];
    let table = bake_one(&surfaces, UseSelfAlpha::Off);
    let normal = table.resolve(SetId(0), "glass.png").expect("normal baked");
    assert_eq!(full(&table, normal)[0][3], 0);
    assert_full_size_asis(&table);
}

#[test]
fn normal_lookup_is_unchanged_by_the_asis_picture() {
    let asis = vec![surface(0, vec![asis_elem(0, "glass.png")])];
    let overlay = vec![surface(0, vec![elem(0, "glass.png")])];
    for use_self_alpha in [UseSelfAlpha::On, UseSelfAlpha::Off] {
        let with = bake_one(&asis, use_self_alpha);
        let without = bake_one(&overlay, use_self_alpha);
        let id = with.resolve(SetId(0), "glass.png").expect("normal baked");
        assert_eq!(with.key(id).kind, PictureKind::Normal);
        let base = without.resolve(SetId(0), "glass.png").unwrap();
        assert_eq!(full(&with, id), full(&without, base));
        assert_eq!(without.resolve_asis(SetId(0), "glass.png"), None);
        assert_eq!((with.len(), without.len()), (2, 1));
    }
}
