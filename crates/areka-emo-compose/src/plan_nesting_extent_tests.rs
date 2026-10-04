//! 外形に element定義の子を置いた範囲を含める（要件 2.5・2.6・7.1）。
//!
//! 外形の再帰も element定義の子へ進み、位置を足す。外形はコマに依らない静的な量のまま。飛ばす
//! element定義（無い番号・範囲を超える数・先祖へ戻る参照）は外形にも数えず、記録は命令の経路の
//! `debug!` だけ（外形の経路は黙って飛ばし、合成 1 回につき同じ飛ばしを 2 度記録しない）。

use std::path::Path;

use areka_emo_atlas::{AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, bake};
use areka_parsers::shell::parse;

use super::test_support::{elem, surface};
use super::{BlitOp, Extent, build_plan};
use crate::bind::BindSet;
use crate::error::ComposeError;
use crate::log_capture::capture_logs;
use crate::method::ComposeMethod;
use crate::pattern::{PatternFrame, PatternState};
use crate::world::EmoWorld;

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

/// 画像の名前と原寸（検体の PNG と同じ寸法・全面不透明）。
const SIZES: &[(&str, (u32, u32))] = &[
    ("body.png", (100, 200)),
    ("eye.png", (20, 10)),
    ("eye_closed.png", (20, 10)),
    ("part.png", (10, 10)),
    ("mouth.png", (16, 8)),
    ("mouth_open.png", (16, 8)),
    ("surface2.png", (100, 200)),
    ("surface11.png", (12, 12)),
    ("big.png", (300, 300)),
];

fn atlas() -> AtlasTable {
    let base = Path::new("shell/master");
    let surfaces = vec![surface(
        0,
        SIZES.iter().map(|(r, _)| elem(0, r, 0, 0)).collect(),
    )];
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
    result.table
}

/// 文面を面の表にして `top` の計画を建てる。命令列と記録も返す。
fn plan(
    text: &str,
    top: u32,
    pattern: &PatternState,
) -> (Result<Extent, ComposeError>, Vec<BlitOp>, String) {
    let atlas = atlas();
    let mut world = EmoWorld::build_with_images(&parse(text), &Default::default());
    world.bind_atlas(&atlas, SetId(0));
    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let mut result = None;
    let logs = capture_logs(|| {
        result = Some(build_plan(
            &mut ops,
            &mut visited,
            &world,
            &atlas,
            top,
            &BindSet::default(),
            pattern,
        ));
    });
    assert!(visited.is_empty(), "先祖の積み上げは走査後に空へ戻る");
    (result.unwrap(), ops, logs)
}

fn extent(text: &str, top: u32) -> Extent {
    let (result, _, logs) = plan(text, top, &PatternState::default());
    assert!(!logs.contains("level=WARN"), "{logs}");
    result.expect("計画が建つ")
}

/// 親の画像の右下へはみ出す子は、はみ出した分まで外形を広げる（要件 2.5）。
#[test]
fn child_overflowing_parent_widens_extent() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,95,195\n}\n\
                surface10\n{\nelement0,overlay,part.png,0,0\n}\n";
    assert_eq!(extent(text, 0), Extent { w: 105, h: 205 });
}

/// 多段の入れ子は各段の X,Y を足した位置で外形に入る（要件 2.3・2.5）。
#[test]
fn multi_level_children_add_every_offset() {
    let text = "surface0\n{\nelement0,overlay,part.png,0,0\nelement1,overlay,30,5,5\n}\n\
                surface30\n{\nelement0,overlay,part.png,0,0\nelement1,overlay,31,3,4\n}\n\
                surface31\n{\nelement0,overlay,part.png,0,0\nelement1,overlay,32,2,1\n}\n\
                surface32\n{\nelement0,overlay,big.png,0,0\n}\n";
    // 32 は (5,5)+(3,4)+(2,1)=(10,10) に 300×300。
    assert_eq!(extent(text, 0), Extent { w: 310, h: 310 });
}

/// 検体: 親 B（surface1）は子 10 を (90,50) に置き（右へはみ出す）、surface.append1 で 3 段の
/// 入れ子 30 を (60,100) に置く。子 10 のまばたき（random）は外形に入らない（要件 2.5・2.6）。
#[test]
fn fixture_parent_b_extent_includes_overflowing_child() {
    // body 100×200 ∪ eye (90,50)+20×10 ∪ part (60,100)〜(65,105)+10×10。
    assert_eq!(extent(FIXTURE, 1), Extent { w: 110, h: 200 });
}

/// 画像を持たず子だけを置く親でも計画が建つ（`EmptyComposition` にならない・要件 2.5）。
#[test]
fn parent_with_only_children_builds_a_plan() {
    let text = "surface0\n{\nelement0,overlay,10,3,4\n}\n\
                surface10\n{\nelement0,overlay,part.png,0,0\n}\n";
    let (result, ops, _) = plan(text, 0, &PatternState::default());
    assert_eq!(result, Ok(Extent { w: 13, h: 14 }));
    assert_eq!(ops.len(), 1, "子の画像 1 枚が命令になる");
}

fn frame(surface_id: u32, x: i64, y: i64) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x,
        y,
    }
}

/// 部品のコマが進んでも外形は動かない（コマに依らない静的な量・要件 2.6）。
#[test]
fn part_frames_do_not_move_extent() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,5,5\n}\n\
                surface10\n{\nelement0,overlay,part.png,0,0\n\
                animation0.interval,random,2\nanimation0.pattern0,overlay,12,50,0,0\n}\n\
                surface12\n{\nelement0,overlay,big.png,0,0\n}\n";
    let (still, still_ops, _) = plan(text, 0, &PatternState::default());
    let mut moving = PatternState::default();
    moving.set_part(10, 0, frame(12, 40, 40));
    let (moved, moved_ops, _) = plan(text, 0, &moving);
    assert_eq!(still, Ok(Extent { w: 100, h: 200 }));
    assert_eq!(moved, still, "部品のコマで外形が変わらない");
    assert_ne!(moved_ops, still_ops, "コマは命令には効いている");
}

/// 飛ばす element定義（無い番号・範囲を超える数・自分自身・相互の循環）は外形に数えず、記録は
/// 命令の経路の `debug!` だけ（合成 1 回につき 1 件ずつ・外形の経路は `warn!` も出さない）。
#[test]
fn skipped_children_do_not_count_and_log_once_per_compose() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,9999,500,500\n\
                element2,overlay,4294967296,500,500\nelement3,overlay,0,500,500\n\
                element4,overlay,61,0,0\n}\n\
                surface61\n{\nelement0,overlay,part.png,0,0\nelement1,overlay,62,0,0\n}\n\
                surface62\n{\nelement0,overlay,part.png,0,0\nelement1,overlay,61,500,500\n}\n";
    let (result, _, logs) = plan(text, 0, &PatternState::default());
    assert_eq!(result, Ok(Extent { w: 100, h: 200 }));
    assert!(!logs.contains("level=WARN"), "{logs}");
    let skips = logs
        .lines()
        .filter(|l| l.contains("子のサーフェスを置かない"))
        .count();
    assert_eq!(skips, 4, "{logs}");
}

/// 入れ子の無いシェルの外形は本 spec の前と同じ（画像と着せ替えの pattern0 の和集合・要件 7.1）。
#[test]
fn non_nested_shell_extent_unchanged() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\n\
                animation1.interval,bind\nanimation1.pattern0,overlay,12,0,90,195\n}\n\
                surface12\n{\nelement0,overlay,eye_closed.png,0,0\n}\n";
    assert_eq!(extent(text, 0), Extent { w: 110, h: 205 });
}
