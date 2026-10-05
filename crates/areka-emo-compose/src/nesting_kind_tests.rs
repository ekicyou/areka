//! 読み分け（要件 1.1・1.2・1.3・1.5・1.9・3.5）: 数字だけの欄はサーフェスの番号、
//! それ以外は今までどおり画像。番号の element は畳み込みで番号として正規化され、アトラスを引かない。

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface};

use super::{ElementKind, element_kind};
use crate::log_capture::capture_logs;
use crate::world::{AtlasBinding, EmoWorld, SurfaceIndex};

fn kind_of(s: &str) -> ElementKind {
    element_kind(&ElementPath::new(s.to_string()))
}

/// 半角の数字だけなら十進の番号。先頭の 0 は値に効かない。
#[test]
fn digits_only_reads_as_surface_number() {
    assert_eq!(kind_of("100"), ElementKind::Surface(100));
    assert_eq!(kind_of("0100"), ElementKind::Surface(100));
    assert_eq!(kind_of("0"), ElementKind::Surface(0));
    assert_eq!(kind_of("4294967295"), ElementKind::Surface(u32::MAX));
}

/// 数字以外が 1 字でも混ざる・空なら画像（今までどおり）。
#[test]
fn anything_else_reads_as_image() {
    for s in [
        "100.png",
        "+1",
        "-1",
        "１００",
        "",
        "1 0",
        "body.png",
        "a100",
    ] {
        assert_eq!(kind_of(s), ElementKind::Image, "{s:?} は画像");
    }
}

/// u32 に収まらない数字だけの欄は、画像でも番号でもない。
#[test]
fn digits_beyond_u32_are_out_of_range() {
    assert_eq!(kind_of("4294967296"), ElementKind::SurfaceOutOfRange);
    assert_eq!(kind_of("99999999999"), ElementKind::SurfaceOutOfRange);
}

const TEXT: &str = "\
surface1
{
element0,overlay,body.png,0,0
element1,overlay,100,40,60
element2,overlay,4294967296,0,0
}

surface100
{
element0,overlay,eye.png,0,0
}

surface.append1
{
element3,overlay,0100,5,6
}
";

/// `surface*`ブレスと `surface.append*`ブレスの両方で、数字だけの欄が番号として正規化される。
#[test]
fn fold_normalises_digits_as_number_in_both_braces() {
    let world = EmoWorld::build(&areka_parsers::shell::parse(TEXT));
    let kinds: Vec<(u32, ElementKind)> = world
        .surface(1)
        .expect("面 1")
        .elements
        .iter()
        .map(|e| (e.layer, e.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (0, ElementKind::Image),
            (1, ElementKind::Surface(100)),
            (2, ElementKind::SurfaceOutOfRange),
            // surface.append1 の element3。
            (3, ElementKind::Surface(100)),
        ]
    );
    assert_eq!(
        world.surface(100).expect("面 100").elements[0].kind,
        ElementKind::Image
    );
}

/// 画像だけで建つ面の土台の element は画像。
#[test]
fn base_image_element_is_image() {
    let shell = areka_parsers::shell::parse("");
    let images = [(7u32, "surface7.png".to_string())].into_iter().collect();
    let world = EmoWorld::build_with_images(&shell, &images);
    assert_eq!(
        world.surface(7).expect("面 7").elements[0].kind,
        ElementKind::Image
    );
}

/// 数字だけの名前の画像がアトラスに在っても、番号の element は引かれず、警告も出ない
/// （要件 1.5・3.5）。画像の element は今までどおり引かれる。
#[test]
fn number_elements_are_not_bound_and_emit_no_warning() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["body.png", "eye.png", "100", "0100"]);
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(TEXT));

    let logs = capture_logs(|| world.bind_atlas(&atlas, SetId(0)));
    assert!(!logs.contains("level=WARN"), "警告は 0 件: {logs}");

    let master = world.surface(1).expect("面 1");
    let binding = binding_of(&world, 1);
    assert_eq!(binding.0.len(), master.elements.len());
    for (e, b) in master.elements.iter().zip(&binding.0) {
        match e.kind {
            ElementKind::Image => assert_eq!(
                *b,
                atlas.resolve(SetId(0), e.path.as_str()),
                "画像 {} は引かれる",
                e.path.as_str()
            ),
            _ => assert_eq!(*b, None, "番号 {} は引かれない", e.path.as_str()),
        }
    }
    assert!(binding.0[0].is_some(), "body.png は焼かれていて引ける");
}

/// 指定の rel_path 群を焼いた単一セットのアトラス（COM/WIC 非依存）。
fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
    let elements: Vec<Element> = rels
        .iter()
        .map(|r| Element {
            layer: 0,
            path: ElementPath::new(r.to_string()),
            x: 0,
            y: 0,
        })
        .collect();
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let mut dec = MemoryDecoder::new();
    for r in rels {
        dec.insert(base.join(r), 1, 1, 4, vec![1, 2, 3, 255], true);
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let result = bake(&[set], &dec, PackConfig::default());
    assert!(result.errors.is_empty(), "焼くのは失敗しない");
    result.table
}

fn binding_of(world: &EmoWorld, id: u32) -> AtlasBinding {
    let w = world.world();
    let entity = w.resource::<SurfaceIndex>().0[&id];
    w.get::<AtlasBinding>(entity)
        .expect("AtlasBinding が入っている")
        .clone()
}
