//! 無い番号と循環の報告（要件 1.8・1.9・3.1・3.2・3.4・8.1・8.3）。
//!
//! 無い番号: element定義が指した番号が面の表に無い、または範囲を超える数。
//! 循環: element定義の辺 (親, element, 子) について、子から element定義の辺とすべての pattern定義の辺
//! （欄 2 が animation の番号になる 7 語は除く）をたどって親へ戻れるなら 1 件。
//! 並びは親の番号 → element定義の番号の昇順（同じ番号は書いた順）。

use std::collections::BTreeMap;

use areka_parsers::shell::parse;

use super::{NestIssue, NestReport};
use crate::log_capture::capture_logs;
use crate::world::EmoWorld;

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

fn report_of(text: &str) -> Vec<NestIssue> {
    EmoWorld::build(&parse(text)).nest_report().issues
}

fn missing(surface: u32, element: u32, target: &str) -> NestIssue {
    NestIssue::MissingTarget {
        surface,
        element,
        target: target.to_string(),
    }
}

fn cycle(surface: u32, element: u32, target: u32) -> NestIssue {
    NestIssue::Cycle {
        surface,
        element,
        target,
    }
}

/// 無い番号・範囲を超える数が無い番号として、欄の原文のまま載る（要件 3.1・1.9）。画像の element定義は載らない。
#[test]
fn missing_number_and_out_of_range_are_missing_targets() {
    let issues = report_of(
        "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,0100,0,0\n\
         element2,overlay,4294967296,0,0\nelement3,overlay,1,0,0\n}\n\
         surface1\n{\nelement0,overlay,eye.png,0,0\n}\n",
    );
    assert_eq!(
        issues,
        vec![missing(0, 1, "0100"), missing(0, 2, "4294967296")]
    );
}

/// 自分自身・2 つの相互・element定義と pattern定義が混ざった循環が、辺 1 本につき 1 件載る（要件 3.2）。
///
/// 5→5＝自分自身／6→7・7→6＝2 つの相互／8 →(element) 9 →(pattern) 8＝混ざった循環
/// （pattern定義の辺そのものは報告に載らない）。
#[test]
fn self_mutual_and_mixed_cycles_are_listed() {
    let issues = report_of(
        "surface5\n{\nelement0,overlay,5,0,0\n}\n\
         surface6\n{\nelement1,overlay,7,0,0\n}\n\
         surface7\n{\nelement2,overlay,6,0,0\n}\n\
         surface8\n{\nelement3,overlay,9,0,0\n}\n\
         surface9\n{\nelement0,overlay,eye.png,0,0\n\
         animation0.interval,random,2\nanimation0.pattern0,overlay,8,50,0,0\n}\n",
    );
    assert_eq!(
        issues,
        vec![
            cycle(5, 0, 5),
            cycle(6, 1, 7),
            cycle(7, 2, 6),
            cycle(8, 3, 9)
        ]
    );
}

/// 描画メソッドに依らず、すべての pattern定義の辺をたどる。欄 2 が animation の番号になる 7 語の辺と、
/// 負の番号は辺にしない。pattern定義だけの循環は報告に載らない（今の合成のたびの warn のまま）。
#[test]
fn pattern_edges_skip_the_seven_animation_id_words() {
    let words = [
        "start",
        "stop",
        "alternativestart",
        "alternativestop",
        "parallelstart",
        "parallelstop",
        "insert",
    ];
    for word in words {
        let text = format!(
            "surface0\n{{\nelement0,overlay,1,0,0\n}}\n\
             surface1\n{{\nanimation0.interval,random,2\nanimation0.pattern0,{word},0,50,0,0\n\
             animation0.pattern1,overlay,-1,50,0,0\n}}\n"
        );
        assert_eq!(
            report_of(&text),
            vec![],
            "{word} の欄 2 は animation の番号"
        );
    }
    // 7 語以外は動かない描画メソッド（reduce）でも辺になる。
    let issues = report_of(
        "surface0\n{\nelement0,overlay,1,0,0\n}\n\
         surface1\n{\nanimation0.interval,random,2\nanimation0.pattern0,reduce,0,50,0,0\n}\n",
    );
    assert_eq!(issues, vec![cycle(0, 0, 1)]);
    // pattern定義だけの循環（2→3→2）は element定義の辺が無いので載らない。
    let issues = report_of(
        "surface2\n{\nanimation0.interval,random,2\nanimation0.pattern0,overlay,3,50,0,0\n}\n\
         surface3\n{\nanimation0.interval,random,2\nanimation0.pattern0,overlay,2,50,0,0\n}\n",
    );
    assert_eq!(issues, vec![]);
}

/// 同じ子を 2 つの経路から置いても循環にならない（要件 3.4）。同じ親の中で 2 度置いても同じ。
#[test]
fn same_child_by_two_paths_is_not_a_cycle() {
    let issues = report_of(
        "surface0\n{\nelement0,overlay,1,0,0\nelement1,overlay,2,0,0\nelement2,overlay,3,0,0\n\
         element3,overlay,3,9,9\n}\n\
         surface1\n{\nelement0,overlay,3,0,0\n}\n\
         surface2\n{\nelement0,overlay,3,0,0\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,1,50,0,0\n}\n\
         surface3\n{\nelement0,overlay,eye.png,0,0\n}\n",
    );
    assert_eq!(issues, vec![]);
}

/// ファイル名の慣習だけで建つ子は無い番号にならない（要件 1.8）。画像の対応が無ければ無い番号。
#[test]
fn filename_convention_child_is_not_missing() {
    let text = "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,11,0,0\n}\n";
    let images = BTreeMap::from([(11, "surface11.png".to_string())]);
    let with_image = EmoWorld::build_with_images(&parse(text), &images);
    assert_eq!(with_image.nest_report(), NestReport::default());
    assert_eq!(report_of(text), vec![missing(0, 1, "11")]);
}

/// 同じ番号の element定義が複数あるときは書いた順。親の番号 → element定義の番号の昇順で並ぶ。
#[test]
fn order_is_parent_then_element_then_source_order() {
    let issues = report_of(
        "surface9\n{\nelement4,overlay,77,0,0\nelement1,overlay,9,0,0\n}\n\
         surface3\n{\nelement2,overlay,88,0,0\nelement2,overlay,3,0,0\nelement2,overlay,66,0,0\n}\n",
    );
    assert_eq!(
        issues,
        vec![
            missing(3, 2, "88"),
            cycle(3, 2, 3),
            missing(3, 2, "66"),
            cycle(9, 1, 9),
            missing(9, 4, "77"),
        ]
    );
}

/// 検体: 仕込んだ誤り（無い番号 2・循環の辺 3）が 1 件につき 1 件ずつ決まった順で並び、他は 0 件。
/// 報告を作る間は記録を 1 行も出さない（要件 8.1 の記録は入口の役目）。
#[test]
fn fixture_yields_exactly_the_planted_issues() {
    let images = BTreeMap::from([
        (2, "surface2.png".to_string()),
        (11, "surface11.png".to_string()),
    ]);
    let world = EmoWorld::build_with_images(&parse(FIXTURE), &images);
    let mut report = NestReport::default();
    let logs = capture_logs(|| report = world.nest_report());
    assert_eq!(
        report.issues,
        vec![
            missing(50, 1, "9999"),
            missing(50, 2, "4294967296"),
            cycle(60, 1, 60),
            cycle(61, 1, 62),
            cycle(62, 1, 61),
        ]
    );
    assert!(logs.is_empty(), "報告は記録を出さない: {logs}");
}
