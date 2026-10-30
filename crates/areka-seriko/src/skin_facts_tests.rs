//! `\s`・数の `\b` の判定の決定論テスト（spec: mcp-strict-errors 要件 2.7）。事実は写しの形
//! （[`SkinFacts::new`]）で手で組み、検出点の答えを固定する。

use std::collections::{BTreeMap, BTreeSet};

use super::*;

/// スコープ 0 の絵は 0・3・10、スコープ 1 は 10・11、別名 `笑顔` は 3。
/// バルーンはスコープ 0 が 0・2、スコープ 1 が 1。スコープ 2 には何も無い。
fn facts() -> SkinFacts {
    let ids = |v: &[u32]| v.iter().copied().collect::<BTreeSet<u32>>();
    SkinFacts::new(
        BTreeMap::from([("笑顔".to_string(), vec![3])]),
        BTreeMap::from([(0, ids(&[0, 3, 10])), (1, ids(&[10, 11]))]),
        BTreeMap::from([(0, ids(&[0, 2])), (1, ids(&[1]))]),
    )
}

#[test]
fn surface_answers_by_detection_point() {
    let f = facts();
    let cases: [(u32, &str, Option<DetectionPoint>); 9] = [
        (0, "0", None),
        (0, "笑顔", None),
        (0, "-1", None),
        (0, "99999", Some(DetectionPoint::SurfaceAbsent)),
        (1, "3", Some(DetectionPoint::SurfaceAbsent)),
        (0, "無い名前", Some(DetectionPoint::SurfaceUnresolved)),
        (0, "-2", Some(DetectionPoint::SurfaceUnresolved)),
        (0, "4294967296", Some(DetectionPoint::SurfaceUnresolved)),
        // 絵の無いスコープは解けない名前も診ない。
        (2, "無い名前", None),
    ];
    for (scope, key, want) in cases {
        assert_eq!(surface_missing(&f, scope, key), want, "scope {scope} {key}");
    }
    assert_eq!(surface_missing(&f, 2, "99999"), None);
}

#[test]
fn balloon_answers_by_detection_point() {
    let f = facts();
    let cases: [(u32, &str, Option<DetectionPoint>); 8] = [
        (0, "0", None),
        (0, "2", None),
        (0, "-1", None),
        // 名前の形は文字の層の持ち分なのでここでは診ない。
        (0, "名前", None),
        (0, "5", Some(DetectionPoint::BalloonAbsent)),
        (1, "0", Some(DetectionPoint::BalloonAbsent)),
        (0, "-2", Some(DetectionPoint::BalloonInvalid)),
        // バルーンの無いスコープは診ない。
        (2, "-2", None),
    ];
    for (scope, key, want) in cases {
        assert_eq!(balloon_missing(&f, scope, key), want, "scope {scope} {key}");
    }
    assert_eq!(balloon_missing(&f, 2, "9"), None);
}

#[test]
fn facts_without_a_scope_do_not_answer_it() {
    let f = facts();
    assert_eq!(f.shell_has(2, 0), None);
    assert_eq!(f.balloon_has(2, 0), None);
    assert_eq!(f.shell_has(0, 3), Some(true));
    assert_eq!(f.shell_has(1, 3), Some(false));
    assert_eq!(f.resolve_surface("笑顔"), SurfaceTarget::Show(3));
}
