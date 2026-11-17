//! `collisionex` の記録の型の檻（重さと文言・要件 4.7・4.9）。`decode.rs` の末尾から差し込む。
//! task 4.1 の読み手の檻もこの同じファイルへ足す（差し込みはもうある）。

use crate::shell::{CollisionExFault, ShapeCounts, ShellIssue};

#[test]
fn collisionex_arms_are_warn_and_name_heading_line_and_reason() {
    let used = ShellIssue::CollisionEx {
        heading: "surface0".into(),
        text: "collisionex0,a,rect,1,2,3,4,9".into(),
        fault: CollisionExFault::ExtraFields,
    };
    assert_eq!(used.severity(), tracing::Level::WARN);
    let m = used.message();
    assert!(
        m.contains("surface0") && m.contains("collisionex0,a,rect,1,2,3,4,9"),
        "{m}"
    );
    assert!(m.contains("前から求める数だけを使った"), "{m}");

    let unused = ShellIssue::CollisionEx {
        heading: "surface0".into(),
        text: "collisionex0,a,foo".into(),
        fault: CollisionExFault::UnknownShape,
    };
    assert_eq!(unused.severity(), tracing::Level::WARN);
    assert!(
        unused.message().contains("当たり判定にしない"),
        "{}",
        unused.message()
    );
}

#[test]
fn shapes_inactive_is_warn_and_names_counts_first_line_and_hit_id_pass() {
    let issue = ShellIssue::ShapesInactive {
        heading: "surface5".into(),
        text: "collisionex1,e,ellipse,0,0,9,9".into(),
        counts: ShapeCounts {
            ellipse: 32,
            circle: 1,
            polygon: 28,
            region: 2,
        },
    };
    assert_eq!(issue.severity(), tracing::Level::WARN);
    let m = issue.message();
    for needle in [
        "surface5",
        "collisionex1,e,ellipse,0,0,9,9",
        "楕円 32",
        "円 1",
        "多角形 28",
        "region 2",
        "hit-id-pass",
    ] {
        assert!(m.contains(needle), "{needle} not in {m}");
    }
}
