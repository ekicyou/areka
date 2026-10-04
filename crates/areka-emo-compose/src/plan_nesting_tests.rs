//! element定義の子の合成（要件 1.6・1.7・1.8・2.1〜2.4・2.7・3.1〜3.4・7.1・8.3）。
//!
//! 層 (i) は element定義を番号の昇順（同じ番号は書いた順）に見て、画像は命令に、サーフェスの番号は
//! その場で子へ再帰する（位置は親の位置＋ element定義の X,Y）。先祖へ戻る・面の表に無い・範囲を
//! 超える数の element定義だけを飛ばし、`debug!` を 1 行出す。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use areka_emo_atlas::SetId;
use areka_parsers::shell::parse;

use super::derive_ops;
use super::test_support::{bake_atlas, id_to_path};
use crate::bind::BindSet;
use crate::log_capture::capture_logs;
use crate::method::ComposeMethod;
use crate::nesting::NestIssue;
use crate::pattern::{PatternFrame, PatternState};
use crate::world::EmoWorld;

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

/// 検体と小さな文面で使う画像の名前（どれも 2×2 の不透明）。
const RELS: &[&str] = &[
    "a.png",
    "b.png",
    "c.png",
    "body.png",
    "eye.png",
    "eye_closed.png",
    "part.png",
    "mouth.png",
    "mouth_open.png",
    "surface2.png",
    "surface11.png",
];

/// 文面を面の表にし、`top` を合成した命令列を (画像の名前, X, Y) の列で返す。記録も返す。
fn compose(
    text: &str,
    images: &[(u32, &str)],
    top: u32,
    binds: &BindSet,
) -> (Vec<(&'static str, i64, i64)>, String) {
    compose_with(text, images, top, binds, &PatternState::default())
}

/// [`compose`] にコマの状態を渡す形。
fn compose_with(
    text: &str,
    images: &[(u32, &str)],
    top: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> (Vec<(&'static str, i64, i64)>, String) {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, RELS);
    let map = id_to_path(&atlas, RELS);
    let images: BTreeMap<u32, String> = images.iter().map(|(i, p)| (*i, p.to_string())).collect();
    let mut world = EmoWorld::build_with_images(&parse(text), &images);
    world.bind_atlas(&atlas, SetId(0));
    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let logs =
        capture_logs(|| derive_ops(&mut ops, &mut visited, &world, &atlas, top, binds, pattern));
    assert!(visited.is_empty(), "先祖の積み上げは走査後に空へ戻る");
    let named = ops
        .iter()
        .map(|op| {
            let name = map
                .iter()
                .find(|(id, _)| *id == op.element)
                .map(|(_, p)| *p)
                .expect("命令の ElementId は既知");
            let (x, y) = op.transform.offset();
            (name, x, y)
        })
        .collect();
    (named, logs)
}

fn ops_of(text: &str, top: u32) -> Vec<(&'static str, i64, i64)> {
    compose(text, &[], top, &BindSet::default()).0
}

/// 飛ばした element定義の `debug!` の行から (親, element, 指した欄) を取り出す。
fn skipped(logs: &str) -> Vec<(u32, u32, String)> {
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

/// 子の画像の左上が、親の element定義の X,Y と子の中の X,Y の和に来る（要件 2.1）。
#[test]
fn child_image_position_is_parent_plus_child_offset() {
    let ops = ops_of(
        "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,20,30\n}\n\
         surface10\n{\nelement0,overlay,eye.png,1,2\n}\n",
        0,
    );
    assert_eq!(ops, vec![("body.png", 0, 0), ("eye.png", 21, 32)]);
}

/// 子の絵は、親の element定義の番号の順に画像の間へ挟まる（同じ番号は書いた順・要件 2.2）。
#[test]
fn child_is_interleaved_by_parent_element_order() {
    let ops = ops_of(
        "surface0\n{\nelement2,overlay,b.png,0,0\nelement0,overlay,a.png,0,0\n\
         element1,overlay,10,5,5\nelement2,overlay,11,7,7\n}\n\
         surface10\n{\nelement0,overlay,c.png,0,0\nelement1,overlay,eye.png,1,1\n}\n\
         surface11\n{\nelement0,overlay,part.png,0,0\n}\n",
        0,
    );
    assert_eq!(
        ops,
        vec![
            ("a.png", 0, 0),
            ("c.png", 5, 5),
            ("eye.png", 6, 6),
            ("b.png", 0, 0),
            ("part.png", 7, 7),
        ]
    );
}

/// 3 段の入れ子は各段の X,Y を足した位置に描く（要件 2.3）。
#[test]
fn three_levels_sum_every_offset() {
    let ops = ops_of(FIXTURE, 30);
    assert_eq!(
        ops,
        vec![("part.png", 0, 0), ("part.png", 3, 4), ("part.png", 5, 5)]
    );
    // 親 A（surface0）から見た 32 は (5,5)+(3,4)+(2,1)=(10,10)。
    let (ops, _) = compose(FIXTURE, &[], 0, &BindSet::default());
    assert!(ops.contains(&("part.png", 10, 10)), "{ops:?}");
}

/// 同じ子を 1 つの親の 2 か所にも、2 つの親にも置ける（要件 1.6・1.7・3.4）。
#[test]
fn same_child_at_two_places_and_in_two_parents() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,0,0\n\
                element2,overlay,10,5,6\n}\n\
                surface1\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,7,8\n}\n\
                surface10\n{\nelement0,overlay,eye.png,1,1\n}\n";
    let (ops, logs) = compose(text, &[], 0, &BindSet::default());
    assert_eq!(
        ops,
        vec![("body.png", 0, 0), ("eye.png", 1, 1), ("eye.png", 6, 7)]
    );
    assert!(
        skipped(&logs).is_empty(),
        "同じ子の 2 度目は循環でない: {logs}"
    );
    assert_eq!(ops_of(text, 1), vec![("body.png", 0, 0), ("eye.png", 8, 9)]);
}

/// `element0` が子を指す親では、`surface*.png` を使わず子の絵で置き換える（要件 2.4）。
#[test]
fn element0_child_replaces_surface_image() {
    let (ops, _) = compose(
        "surface2\n{\nelement0,overlay,10,0,0\n}\n\
         surface10\n{\nelement0,overlay,eye.png,0,0\n}\n",
        &[(2, "surface2.png")],
        2,
        &BindSet::default(),
    );
    assert_eq!(ops, vec![("eye.png", 0, 0)]);
}

/// ブレスを持たずファイル名の慣習だけで建つ子も部品として置く（要件 1.8）。
#[test]
fn filename_convention_child_is_drawn() {
    let (ops, logs) = compose(
        "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,11,50,50\n}\n",
        &[(11, "surface11.png")],
        0,
        &BindSet::default(),
    );
    assert_eq!(ops, vec![("body.png", 0, 0), ("surface11.png", 50, 50)]);
    assert!(skipped(&logs).is_empty(), "{logs}");
}

/// pattern定義が指すサーフェスの中の子も、同じ規則で重ねる（要件 2.7）。
#[test]
fn child_inside_pattern_target_surface() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\n\
                animation100.interval,bind\nanimation100.pattern0,overlay,40,0,30,120\n}\n\
                surface40\n{\nelement0,overlay,mouth.png,0,0\nelement1,overlay,41,2,3\n}\n\
                surface41\n{\nelement0,overlay,mouth_open.png,0,0\n}\n";
    let (ops, _) = compose(text, &[], 0, &BindSet::from_ids([100]));
    assert_eq!(
        ops,
        vec![
            ("body.png", 0, 0),
            ("mouth.png", 30, 120),
            ("mouth_open.png", 32, 123),
        ]
    );
}

/// 循環でも命令列は有限で、先祖へ戻る element定義だけが抜ける。切った辺はどれも 3.2 の報告に在り、
/// 警告は出さない（読み込みのときに出ている・合成の中は `debug!`・要件 3.2・3.3）。
#[test]
fn cycles_are_finite_and_cut_edges_are_reported() {
    let images = [(2, "surface2.png"), (11, "surface11.png")];
    let report: BTreeSet<(u32, u32, String)> = {
        let images: BTreeMap<u32, String> =
            images.iter().map(|(i, p)| (*i, p.to_string())).collect();
        EmoWorld::build_with_images(&parse(FIXTURE), &images)
            .nest_report()
            .issues
            .into_iter()
            .filter_map(|issue| match issue {
                NestIssue::Cycle {
                    surface,
                    element,
                    target,
                } => Some((surface, element, target.to_string())),
                NestIssue::MissingTarget { .. } => None,
            })
            .collect()
    };
    let expected_ops = [
        (60, vec![("part.png", 0, 0)]),
        (61, vec![("part.png", 0, 0), ("part.png", 0, 0)]),
        (62, vec![("part.png", 0, 0), ("part.png", 0, 0)]),
    ];
    let mut cut_all = BTreeSet::new();
    for (top, expected) in expected_ops {
        let (ops, logs) = compose(FIXTURE, &images, top, &BindSet::default());
        assert_eq!(ops, expected, "一番上 {top}");
        assert!(
            !logs.contains("level=WARN"),
            "element定義の循環は警告しない: {logs}"
        );
        let cut = skipped(&logs);
        assert_eq!(cut.len(), 1, "切る辺は 1 本・1 行: {logs}");
        for edge in cut {
            assert!(report.contains(&edge), "切った辺 {edge:?} が報告に在る");
            cut_all.insert(edge);
        }
    }
    assert_eq!(cut_all, report, "3 つの一番上で報告の辺を全部切る");
}

/// 無い番号・範囲を超える数の element定義だけが抜け、同じサーフェスの他の element定義は描く（要件 3.1・1.9）。
#[test]
fn only_the_missing_element_is_dropped() {
    let (ops, logs) = compose(
        "surface0\n{\nelement0,overlay,a.png,0,0\nelement1,overlay,9999,0,0\n\
         element2,overlay,4294967296,0,0\nelement3,overlay,b.png,1,1\n}\n",
        &[],
        0,
        &BindSet::default(),
    );
    assert_eq!(ops, vec![("a.png", 0, 0), ("b.png", 1, 1)]);
    assert_eq!(
        skipped(&logs),
        vec![(0, 1, "9999".to_string()), (0, 2, "4294967296".to_string())]
    );
    assert!(!logs.contains("level=WARN"), "合成の中は debug!: {logs}");
}

// ── 部品の段のコマの合流（task 4.2・要件 5.1・5.2・5.10・5.12・5.13） ──

/// `surface_id` を (x,y) に置く Overlay のコマ。
fn frame(surface_id: u32, x: i64, y: i64) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x,
        y,
    }
}

/// 子 10 を (20,30) に置く親 0。子と親は同じ番号の animation0 を持つ。
const PART_TEXT: &str = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,20,30\n\
                         animation0.interval,random,2\nanimation0.pattern0,overlay,13,50,1,1\n}\n\
                         surface10\n{\nelement0,overlay,eye.png,0,0\n\
                         animation0.interval,random,2\nanimation0.pattern0,overlay,12,50,0,0\n}\n\
                         surface12\n{\nelement0,overlay,eye_closed.png,0,0\n}\n\
                         surface13\n{\nelement0,overlay,c.png,0,0\n}\n";

/// 部品のコマは部品の段で使い、一番上の同じ番号のコマとは混ざらない（要件 5.1・5.10）。
#[test]
fn part_frames_are_used_in_the_part_stage_only() {
    let binds = BindSet::default();
    let run = |pattern: &PatternState| compose_with(PART_TEXT, &[], 0, &binds, pattern).0;
    let base = vec![("body.png", 0, 0), ("eye.png", 20, 30)];
    assert_eq!(run(&PatternState::default()), base);

    // 部品 10 の animation0 のコマ → 子の段で、子の位置に重なる。
    let mut part = PatternState::default();
    part.set_part(10, 0, frame(12, 0, 0));
    assert_eq!(
        run(&part),
        vec![
            ("body.png", 0, 0),
            ("eye.png", 20, 30),
            ("eye_closed.png", 20, 30)
        ]
    );

    // 一番上の animation0 のコマは一番上の段だけ（子の段へ漏れない）。
    let mut top = PatternState::default();
    top.set(0, frame(13, 1, 1));
    assert_eq!(
        run(&top),
        vec![("body.png", 0, 0), ("eye.png", 20, 30), ("c.png", 1, 1)]
    );

    // 両方の欄に同じ番号 0 のコマ → それぞれの段に 1 つずつ。
    let mut both = part.clone();
    both.set(0, frame(13, 1, 1));
    assert_eq!(
        run(&both),
        vec![
            ("body.png", 0, 0),
            ("eye.png", 20, 30),
            ("eye_closed.png", 20, 30),
            ("c.png", 1, 1),
        ]
    );

    // 一番上の番号 0 を部品の欄に置いても、一番上の段は部品の欄を見ない。
    let mut misplaced = PatternState::default();
    misplaced.set_part(0, 0, frame(13, 1, 1));
    assert_eq!(run(&misplaced), base);
}

/// 子の中の着せ替えの pattern0 は、全段で同じ着せ替えの集合で出る（要件 5.12）。
#[test]
fn bind_pattern0_inside_child_follows_the_bind_set() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,20,30\n}\n\
                surface10\n{\nelement0,overlay,eye.png,0,0\n\
                animation100.interval,bind\nanimation100.pattern0,overlay,40,0,2,3\n}\n\
                surface40\n{\nelement0,overlay,mouth.png,0,0\n}\n";
    assert_eq!(
        compose(text, &[], 0, &BindSet::from_ids([100])).0,
        vec![
            ("body.png", 0, 0),
            ("eye.png", 20, 30),
            ("mouth.png", 22, 33)
        ]
    );
    assert_eq!(
        compose(text, &[], 0, &BindSet::default()).0,
        vec![("body.png", 0, 0), ("eye.png", 20, 30)]
    );
}

/// 着せ替えから外れた部品のコマは合流せず、有効ならコマが pattern0 を置き換える。描画メソッドの
/// 番人も一番上と同じ（要件 5.12・一番上と同じ 1 本の経路）。
#[test]
fn part_frames_go_through_the_same_bind_and_method_rules() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,10,20,30\n}\n\
                surface10\n{\nelement0,overlay,eye.png,0,0\n\
                animation100.interval,bind\nanimation100.pattern0,overlay,40,0,2,3\n}\n\
                surface40\n{\nelement0,overlay,mouth.png,0,0\n}\n\
                surface41\n{\nelement0,overlay,mouth_open.png,0,0\n}\n";
    let mut pattern = PatternState::default();
    pattern.set_part(10, 100, frame(41, 1, 1));
    let (ops, logs) = compose_with(text, &[], 0, &BindSet::default(), &pattern);
    assert_eq!(ops, vec![("body.png", 0, 0), ("eye.png", 20, 30)]);
    assert!(
        logs.contains("bind 集合に属さない"),
        "外れたコマを捨てた記録: {logs}"
    );
    assert_eq!(
        compose_with(text, &[], 0, &BindSet::from_ids([100]), &pattern).0,
        vec![
            ("body.png", 0, 0),
            ("eye.png", 20, 30),
            ("mouth_open.png", 21, 31)
        ]
    );

    // 描画メソッドが動かない部品のコマは描かず、pattern0 も置き換えたまま（一番上と同じ）。
    let mut replace = PatternState::default();
    replace.set_part(
        10,
        100,
        PatternFrame {
            method: ComposeMethod::Replace,
            ..frame(41, 1, 1)
        },
    );
    let (ops, logs) = compose_with(text, &[], 0, &BindSet::from_ids([100]), &replace);
    assert_eq!(ops, vec![("body.png", 0, 0), ("eye.png", 20, 30)]);
    assert!(logs.contains("非 Overlay method の現在コマ"), "{logs}");
}

/// 多段の内側の部品のコマも使い、部品の欄のコマは、その部品が出ている段の絵だけを変える（要件 5.13）。
#[test]
fn inner_multi_level_part_frames_are_used() {
    let binds = BindSet::default();
    let (base, _) = compose(FIXTURE, &[], 0, &binds);
    let mut pattern = PatternState::default();
    pattern.set_part(32, 0, frame(12, 1, 1));
    let (ops, _) = compose_with(FIXTURE, &[], 0, &binds, &pattern);
    let mut expected = base.clone();
    let at = expected
        .iter()
        .position(|op| *op == ("part.png", 10, 10))
        .expect("32 は (10,10)")
        + 1;
    expected.insert(at, ("eye_closed.png", 11, 11));
    assert_eq!(ops, expected);

    // 子 10 は一番上 30 の絵に出ていない → 10 の部品のコマは 30 の命令列を変えない。
    let mut elsewhere = PatternState::default();
    elsewhere.set_part(10, 0, frame(12, 0, 0));
    assert_eq!(
        compose_with(FIXTURE, &[], 30, &binds, &elsewhere).0,
        compose(FIXTURE, &[], 30, &binds).0
    );
}
