//! シェル全体の外形 [`shell_bounds`] の試験（areka-P0-offline-surface-dump 要件 4.3）。
//!
//! 数は SSP の測り（research.md §10.6・`g_anim` の 18×14・`g_size` の 12×14）と同じ形の検体で固める。

use std::path::Path;

use areka_emo_atlas::{AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, bake};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface, parse};

use super::{ShellBounds, shell_bounds};
use crate::Composer;
use crate::bind::BindSet;
use crate::log_capture::capture_logs;
use crate::pattern::PatternState;
use crate::world::EmoWorld;

/// 画像の名前と原寸（全面不透明）。
const SIZES: &[(&str, (u32, u32))] = &[
    ("base16.png", (16, 8)),
    ("base8.png", (8, 8)),
    ("red4.png", (4, 4)),
    ("green4.png", (4, 4)),
];

/// 文面を面の表にし、アトラスを当てて返す。
fn world_of(text: &str) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements: SIZES
            .iter()
            .map(|(r, _)| Element {
                layer: 0,
                path: ElementPath::new(r.to_string()),
                x: 0,
                y: 0,
            })
            .collect(),
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let mut dec = MemoryDecoder::new();
    for (rel, (w, h)) in SIZES {
        dec.insert(
            base.join(rel),
            *w,
            *h,
            w * 4,
            vec![64u8; (w * 4 * h) as usize],
            true,
        );
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: areka_emo_atlas::AlphaParams {
            use_self_alpha: areka_emo_atlas::UseSelfAlpha::On,
        },
    };
    let result = bake(&[set], &dec, PackConfig::default());
    assert!(result.errors.is_empty(), "bake セットアップは失敗しない");
    let atlas = result.table;
    let mut world = EmoWorld::build(&parse(text));
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas)
}

/// どの面の合成の絵の大きさも外形の右下を超えない（design ShellCanvasBounds の Invariants）。
fn assert_every_surface_fits(world: &EmoWorld, atlas: &AtlasTable, b: ShellBounds) {
    let mut composer = Composer::new();
    for id in world.surface_ids() {
        let Ok(out) = composer.compose(
            world,
            atlas,
            id,
            &BindSet::default(),
            &PatternState::default(),
        ) else {
            continue;
        };
        assert!(
            out.width() as i64 <= b.right as i64 && out.height() as i64 <= b.bottom as i64,
            "surface{id} の絵 {}x{} が外形 {b:?} の右下を超える",
            out.width(),
            out.height()
        );
    }
}

/// `g_anim` と同じ形: 面 20〜22 の負の座標・はみ出しの element と、面 23 の runonce の pattern
/// （4,4 に 8×8）が、はみ出しの無い面 0 を含むシェル全体の外形を x −2〜16・y −2〜12（18×14）にする。
#[test]
fn g_anim_shape_is_18x14() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n}\r\n\
         surface20\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,red4.png,-2,-2\r\nelement2,overlay,green4.png,6,6\r\n}\r\n\
         surface21\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,green4.png,6,6\r\n}\r\n\
         surface22\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,red4.png,-2,-2\r\n}\r\n\
         surface23\r\n{\r\nelement0,overlay,base8.png,0,0\r\nanimation0.interval,runonce\r\nanimation0.pattern0,overlay,100,50,4,4\r\n}\r\n\
         surface100\r\n{\r\nelement0,overlay,base8.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_eq!(
        b,
        ShellBounds {
            left: -2,
            top: -2,
            right: 16,
            bottom: 12
        }
    );
    assert_eq!((b.right - b.left, b.bottom - b.top), (18, 14));
    assert_every_surface_fits(&world, &atlas, b);
}

/// `g_size` と同じ形: 面 1 の −2,−3 の 4×4 と、面 2 の `never` の pattern（6,7 に 4×4）が範囲に入り
/// x −2〜10・y −3〜11（12×14）になる。
#[test]
fn g_size_shape_with_never_is_12x14() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base8.png,0,0\r\n}\r\n\
         surface1\r\n{\r\nelement0,overlay,base8.png,0,0\r\nelement1,overlay,red4.png,-2,-3\r\n}\r\n\
         surface2\r\n{\r\nelement0,overlay,base8.png,0,0\r\nanimation0.interval,never\r\nanimation0.pattern0,overlay,101,0,6,7\r\n}\r\n\
         surface101\r\n{\r\nelement0,overlay,green4.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_eq!(
        b,
        ShellBounds {
            left: -2,
            top: -3,
            right: 10,
            bottom: 11
        }
    );
    assert_every_surface_fits(&world, &atlas, b);
}

/// はみ出しの無いシェルは土台の大きさのまま。
#[test]
fn shell_without_overflow_is_base_size() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n}\r\n\
         surface1\r\n{\r\nelement0,overlay,base16.png,0,0\r\nelement1,overlay,red4.png,12,4\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_eq!(
        b,
        ShellBounds {
            left: 0,
            top: 0,
            right: 16,
            bottom: 8
        }
    );
    assert_every_surface_fits(&world, &atlas, b);
}

/// 入れ子（element定義の子・pattern の先の面がさらに pattern を持つ）は位置を足して数え、負の番号と
/// 描かないメソッドの pattern は数えず、循環（element定義も pattern も）は黙って打ち切る（記録を出さない）。
/// pattern の循環は `sometimes` に置く: `walk` はどの interval も辿るが、`compute_extent` は bind・`always`
/// だけを辿るので、bind・`always` の循環なら出る warn（task 3.2 まで）がここでは出ない。
#[test]
fn nesting_adds_offsets_and_skips_non_drawing() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base8.png,0,0\r\n\
         element1,overlay,30,-5,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlay,40,0,0,10\r\n\
         animation1.interval,sometimes\r\nanimation1.pattern0,overlay,-1,0,500,500\r\nanimation1.pattern1,move,101,0,500,500\r\nanimation1.pattern2,start,101,0,500,500\r\n}\r\n\
         surface30\r\n{\r\nelement0,overlay,red4.png,0,0\r\n}\r\n\
         surface40\r\n{\r\nanimation0.interval,always\r\nanimation0.pattern0,overlay,41,0,3,0\r\n}\r\n\
         surface41\r\n{\r\nelement0,overlay,green4.png,0,0\r\nelement1,overlay,41,0,0\r\n}\r\n\
         surface101\r\n{\r\nelement0,overlay,green4.png,0,0\r\nanimation0.interval,sometimes\r\nanimation0.pattern0,overlay,101,0,0,0\r\n}\r\n",
    );
    let mut b = None;
    let logs = capture_logs(|| b = Some(shell_bounds(&world, &atlas)));
    // x: 30 を −5 に置く → −5。y: 0→40 を (0,10)・40→41 を (3,0) で 41 の 4×4 は (3,10)〜(7,14)。
    assert_eq!(
        b,
        Some(ShellBounds {
            left: -5,
            top: 0,
            right: 8,
            bottom: 14
        })
    );
    assert!(logs.trim().is_empty(), "外形は記録を出さない: {logs}");
}

/// bind の pattern0 は描画メソッドを問わず面の絵の大きさに数えられる（`compute_extent`）。`overlayfast` で
/// 外へはみ出しても、外形はその絵を収める（Invariants）。
#[test]
fn bind_pattern0_non_overlay_still_fits() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlayfast,10,0,30,0\r\n}\r\n\
         surface10\r\n{\r\nelement0,overlay,red4.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_every_surface_fits(&world, &atlas, b);
}

/// `always` の pattern も描画メソッドを問わず面の絵の大きさに数えられる。`interpolate` で外へ
/// はみ出しても、外形はその絵を収める（Invariants）。
#[test]
fn always_pattern_non_overlay_still_fits() {
    let (world, atlas) = world_of(
        "charset,UTF-8\r\n\
         surface0\r\n{\r\nelement0,overlay,base16.png,0,0\r\n\
         animation0.interval,always\r\nanimation0.pattern0,interpolate,10,100,0,20\r\n}\r\n\
         surface10\r\n{\r\nelement0,overlay,red4.png,0,0\r\n}\r\n",
    );
    let b = shell_bounds(&world, &atlas);
    assert_every_surface_fits(&world, &atlas, b);
}
