//! 件 26（`animated-surface-log-flood` 要件 9.1・9.4）: `bind+random` のアニメーションは、コマが
//! 無いときに pattern0 を静的に重ねない（再生しているあいだだけ描く）。
//!
//! ぱすたの animation1400 は `bind+random` で pattern0 が閉じた目（1410）。seriko が `-1` で欄を
//! 外した後も pattern0 が静的に積まれ、閉じた目が残っていた。純 `bind`（着せ替え）は今までどおり
//! bind が入っていれば pattern0 を静的に積む（陽性対照を同じ導出に置く）。

use super::test_support::*;
use super::*;
use crate::pattern::PatternFrame;
use areka_emo_atlas::{AtlasTable, SetId};
use areka_parsers::shell::{Animation, DrawMethod, Interval, Pattern};
use std::path::Path;

const RELS: [&str; 3] = ["eyebase.png", "closed.png", "half.png"];

fn overlay(index: u32, surface_id: i64) -> Pattern {
    Pattern {
        index,
        method: DrawMethod::new("overlay".to_string()),
        surface_id,
        target: surface_id.to_string(),
        wait: 0,
        x: 0,
        y: 0,
    }
}

/// host 1000 に純 bind の 1300（pattern0→1301 の目の土台）と、ぱすたの 1400 と同じ形の
/// `bind+random`（pattern0→1410 閉じた目・pattern1→1411 半目・pattern2→-1）を置く。
fn build() -> (EmoWorld, AtlasTable, Vec<(ElementId, &'static str)>) {
    let atlas = bake_atlas(Path::new("shell/master"), &RELS);
    let map = id_to_path(&atlas, &RELS);
    let blink = Animation {
        id: 1400,
        interval: Interval::BindRandom { k: 4 },
        interval_written: true,
        patterns: vec![overlay(0, 1410), overlay(1, 1411), overlay(2, -1)],
    };
    let host = surface_with_anims(1000, Vec::new(), vec![bind_anim(1300, 1301, 0, 0), blink]);
    let shell = shell_of(vec![
        host,
        surface(1301, vec![elem(0, "eyebase.png", 0, 0)]),
        surface(1410, vec![elem(0, "closed.png", 0, 0)]),
        surface(1411, vec![elem(0, "half.png", 0, 0)]),
    ]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas, map)
}

fn paths(pattern: &PatternState) -> Vec<&'static str> {
    let (world, atlas, map) = build();
    let mut ops = Vec::new();
    derive_ops(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1300, 1400]),
        pattern,
    );
    ops.iter()
        .map(|op| map.iter().find(|(id, _)| *id == op.element).unwrap().1)
        .collect()
}

fn frame_at(surface_id: u32) -> PatternState {
    let mut p = PatternState::default();
    p.set(
        1400,
        PatternFrame {
            surface_id,
            method: ComposeMethod::Overlay,
            x: 0,
            y: 0,
        },
    );
    p
}

/// コマが無い（`-1` で外した後・まだ鳴っていない）→ 閉じた目の pattern0 を積まない。純 bind の目の
/// 土台は積む（空同士の一致でない）。
#[test]
fn bind_random_without_frame_stacks_no_pattern0() {
    assert_eq!(paths(&PatternState::default()), vec!["eyebase.png"]);
}

/// 再生中はそのコマを描く（pattern0 の閉じた目も、コマとしてなら描く）。
#[test]
fn bind_random_with_frame_draws_the_frame() {
    assert_eq!(paths(&frame_at(1410)), vec!["eyebase.png", "closed.png"]);
    assert_eq!(paths(&frame_at(1411)), vec!["eyebase.png", "half.png"]);
}
