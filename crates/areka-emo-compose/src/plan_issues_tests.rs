//! 形の誤りを集める関数 [`shape_issues`] と、面を組むたびの知らせが warn にならないことの試験
//! （areka-P0-offline-surface-dump 要件 7.5・design ShapeIssuesOnce）。

use std::path::Path;

use areka_emo_atlas::{AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, bake};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface, parse};

use super::{ShapeIssue, shape_issues};
use crate::Composer;
use crate::bind::BindSet;
use crate::log_capture::capture_logs;
use crate::method::ComposeMethod;
use crate::pattern::{PatternFrame, PatternState};
use crate::world::EmoWorld;

/// 循環（面 10 の bind が自分を指す）・描かないメソッド（面 0 の `replace` が面 20 を指す）を 1 つずつ
/// 持つ。`move`・`start`・負の番号の `replace` は誤りでない。
const TEXT: &str = "charset,UTF-8\r\n\
    surface0\r\n{\r\nelement0,overlay,base.png,0,0\r\n\
    animation0.interval,bind\r\nanimation0.pattern0,overlay,10,0,0,0\r\n\
    animation1.interval,bind\r\nanimation1.pattern0,replace,20,0,0,0\r\n\
    animation2.interval,sometimes\r\nanimation2.pattern0,move,0,0,5,5\r\nanimation2.pattern1,start,0,0,0,0\r\nanimation2.pattern2,replace,-1,0,0,0\r\n}\r\n\
    surface10\r\n{\r\nelement0,overlay,part.png,0,0\r\n\
    animation0.interval,bind\r\nanimation0.pattern0,overlay,10,0,1,1\r\n}\r\n\
    surface20\r\n{\r\nelement0,overlay,part.png,0,0\r\n}\r\n";

/// 循環と描かないメソッドの pattern がそれぞれ 1 件ずつ、面の番号の昇順で返る。記録は出さない。
#[test]
fn collects_cycle_and_undrawn_pattern_once_each() {
    let world = EmoWorld::build(&parse(TEXT));
    let mut issues = Vec::new();
    let logs = capture_logs(|| issues = shape_issues(&world));
    assert_eq!(
        issues,
        vec![
            ShapeIssue::UndrawnPattern {
                surface: 0,
                animation: 1,
                pattern: 0,
                method: "replace".to_string(),
            },
            ShapeIssue::Cycle {
                surface: 10,
                animation: 0,
                pattern: 0,
                target: 10,
            },
        ]
    );
    assert!(logs.trim().is_empty(), "集める関数は記録を出さない: {logs}");
}

/// 誤りの無いシェルは空。
#[test]
fn clean_shell_has_no_issues() {
    let world = EmoWorld::build(&parse(
        "charset,UTF-8\r\nsurface0\r\n{\r\nelement0,overlay,base.png,0,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlay,20,0,0,0\r\n}\r\n\
         surface20\r\n{\r\nelement0,overlay,part.png,0,0\r\n}\r\n",
    ));
    assert_eq!(shape_issues(&world), Vec::new());
}

/// 文面を面の表にし、アトラスを当てて返す。
fn world_of(text: &str) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let rels = ["base.png", "part.png"];
    let surfaces = vec![Surface {
        id: 0,
        targets: vec![AppendTarget::Single(0)],
        elements: rels
            .iter()
            .map(|r| Element {
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
    for rel in rels {
        dec.insert(base.join(rel), 2, 2, 8, vec![255u8; 16], true);
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
    let mut world = EmoWorld::build(&parse(text));
    world.bind_atlas(&result.table, SetId(0));
    (world, result.table)
}

/// 同じ面を何度組んでも、形の誤り（入れ子の循環・描かないメソッドの pattern0 とコマ）の知らせは
/// warn にならず debug に留まり、絵は毎回同じ（打ち切り・飛ばしの振る舞いは変わらない）。
#[test]
fn composing_repeatedly_adds_no_warn() {
    let (world, atlas) = world_of(TEXT);
    let binds = BindSet::from_ids([0, 1]);
    let mut pattern = PatternState::default();
    pattern.set(
        2,
        PatternFrame {
            surface_id: 20,
            method: ComposeMethod::Replace,
            x: 0,
            y: 0,
        },
    );
    let mut composer = Composer::new();
    let mut shots = Vec::new();
    let logs = capture_logs(|| {
        for _ in 0..3 {
            for id in [0, 10] {
                let out = composer
                    .compose(&world, &atlas, id, &binds, &pattern)
                    .expect("組める");
                shots.push((out.width(), out.height(), out.bytes().to_vec()));
            }
        }
    });
    assert!(
        !logs.contains("level=WARN"),
        "合成は warn を出さない: {logs}"
    );
    for kind in [
        "循環",
        "非 Overlay method の bind pattern0",
        "非 Overlay method の現在コマ",
    ] {
        assert!(
            logs.lines()
                .any(|l| l.contains("level=DEBUG") && l.contains(kind)),
            "{kind} は debug に残る: {logs}"
        );
    }
    assert_eq!(shots[0..2], shots[2..4], "絵は毎回同じ");
    assert_eq!(shots[0..2], shots[4..6], "絵は毎回同じ");
}

/// 面 0 が面 1 を置き（element定義と pattern定義）、面 1 の `move` が面 0 を指しても循環でない。
/// `move` はサーフェスの番号を無視する（ukadoc）ので辺にならず、形の誤りと入れ子の報告が同じに判じる。
#[test]
fn move_back_to_parent_is_not_a_cycle() {
    let world = EmoWorld::build(&parse(
        "charset,UTF-8\r\nsurface0\r\n{\r\nelement0,overlay,base.png,0,0\r\nelement1,overlay,1,0,0\r\n\
         animation0.interval,bind\r\nanimation0.pattern0,overlay,1,0,0,0\r\n}\r\n\
         surface1\r\n{\r\nelement0,overlay,part.png,0,0\r\n\
         animation0.interval,sometimes\r\nanimation0.pattern0,move,0,0,5,5\r\n}\r\n",
    ));
    assert_eq!(shape_issues(&world), Vec::new());
    assert_eq!(world.nest_report().issues, Vec::new());
}

/// 知らない `animation-sort`（シェル全体に 1 つの欄）を持つ、面 2 つのシェル。
const UNKNOWN_SORT: &str = "charset,UTF-8\r\nanimation-sort,sideways\r\n\
    surface0\r\n{\r\nelement0,overlay,base.png,0,0\r\n\
    animation0.interval,bind\r\nanimation0.pattern0,overlay,20,0,0,0\r\n}\r\n\
    surface20\r\n{\r\nelement0,overlay,part.png,0,0\r\n}\r\n";

/// 知らない `animation-sort` は面がいくつあっても原文つきで 1 件だけ返る。
#[test]
fn unknown_animation_sort_is_one_issue() {
    let world = EmoWorld::build(&parse(UNKNOWN_SORT));
    assert_eq!(
        shape_issues(&world),
        vec![ShapeIssue::UnknownAnimationSort {
            value: "sideways".to_string(),
        }]
    );
}

/// 知らない `animation-sort` の面を何度組んでも warn は出ず、知らせは debug に留まり、絵は毎回同じ。
#[test]
fn composing_unknown_sort_repeatedly_adds_no_warn() {
    let (world, atlas) = world_of(UNKNOWN_SORT);
    let binds = BindSet::from_ids([0]);
    let pattern = PatternState::default();
    let mut composer = Composer::new();
    let mut shots = Vec::new();
    let logs = capture_logs(|| {
        for _ in 0..3 {
            let out = composer
                .compose(&world, &atlas, 0, &binds, &pattern)
                .expect("組める");
            shots.push(out.bytes().to_vec());
        }
    });
    assert!(
        !logs.contains("level=WARN"),
        "合成は warn を出さない: {logs}"
    );
    assert!(
        logs.lines()
            .any(|l| l.contains("level=DEBUG") && l.contains("未知の animation-sort")),
        "知らない sort は debug に残る: {logs}"
    );
    assert_eq!(shots[0], shots[1], "絵は毎回同じ");
    assert_eq!(shots[0], shots[2], "絵は毎回同じ");
}
