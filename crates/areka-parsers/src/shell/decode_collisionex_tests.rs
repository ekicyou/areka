//! `collisionex` の記録の型の檻（重さと文言・要件 4.7・4.9）。`decode.rs` の末尾から差し込む。
//! task 4.1 の読み手の檻もこの同じファイルへ足す（差し込みはもうある）。

use crate::shell::{
    Collision, CollisionBounds, CollisionExFault, CollisionName, CollisionRegion, CollisionShape,
    ShapeCounts, ShellIssue, parse_all,
};

fn rect(left: i64, top: i64, right: i64, bottom: i64) -> CollisionShape {
    CollisionShape::Rect(CollisionBounds {
        left,
        top,
        right,
        bottom,
    })
}

fn coll(index: u32, name: &str, shape: CollisionShape) -> Collision {
    Collision {
        index,
        shape,
        name: CollisionName::new(name.to_string()),
    }
}

fn surface0(body: &str) -> (Vec<Collision>, Vec<ShellIssue>) {
    let text = parse_all(&format!(
        "surface0
{{
{body}
}}
"
    ));
    (text.shell.surfaces[0].collisions.clone(), text.issues)
}

fn ex(text: &str, fault: CollisionExFault) -> ShellIssue {
    ShellIssue::CollisionEx {
        heading: "surface0".into(),
        text: text.into(),
        fault,
    }
}

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

/// クローディアのように collisionex と collision を混ぜると、5 つの形が書いた順に並ぶ（1.1〜1.8・5.1）。
#[test]
fn claudia_like_collisionex_rows_enter_the_list_in_written_order() {
    let (list, issues) = surface0(
        "collisionex0,Head,rect,10,20,30,40
         collision1,1,2,3,4,Bust
         collisionex2,Foot,ELLIPSE,0,0,9,9
         collisionex3,c,Circle,5,6,7
         collisionex4,Fan,polygon,0,0,10,0,5,5,
         collisionex5,,region,a/b.png,0,255,7
         collisionex6,R,region,m.png,1,2,3,TRUE",
    );
    let region = |file: &str, invert| {
        CollisionShape::Region(CollisionRegion {
            file: file.into(),
            red: if invert { 1 } else { 0 },
            green: if invert { 2 } else { 255 },
            blue: if invert { 3 } else { 7 },
            invert,
            x: 0,
            y: 0,
        })
    };
    assert_eq!(
        list,
        vec![
            coll(0, "Head", rect(10, 20, 30, 40)),
            coll(1, "Bust", rect(1, 2, 3, 4)),
            coll(
                2,
                "Foot",
                CollisionShape::Ellipse(CollisionBounds {
                    left: 0,
                    top: 0,
                    right: 9,
                    bottom: 9
                })
            ),
            coll(
                3,
                "c",
                CollisionShape::Circle {
                    x: 5,
                    y: 6,
                    radius: 7
                }
            ),
            coll(
                4,
                "Fan",
                CollisionShape::Polygon(vec![(0, 0), (10, 0), (5, 5)])
            ),
            coll(5, "", region("a/b.png", false)),
            coll(6, "R", region("m.png", true)),
        ]
    );
    // まだ働かない形の記録が最後に 1 件（楕円 1・円 1・多角形 1・region 2・最初の行は楕円）。
    assert_eq!(
        issues,
        vec![ShellIssue::ShapesInactive {
            heading: "surface0".into(),
            text: "collisionex2,Foot,ELLIPSE,0,0,9,9".into(),
            counts: ShapeCounts {
                ellipse: 1,
                circle: 1,
                polygon: 1,
                region: 2
            },
        }]
    );
}

/// 使わない行は並びに入らず 1 件の記録、欄が多い行だけは前から使って 1 件の記録（4.1〜4.6・4.10）。
#[test]
fn faults_record_once_and_only_extra_fields_enter_the_list() {
    let cases: [(&str, Option<CollisionExFault>, bool); 14] = [
        (
            "collisionex0,a,star,1,2,3,4",
            Some(CollisionExFault::UnknownShape),
            false,
        ),
        (
            "collisionex0,a,rect,1,2,3",
            Some(CollisionExFault::MissingCoordinates),
            false,
        ),
        (
            "collisionex0,a,polygon,1,2,3,4",
            Some(CollisionExFault::MissingCoordinates),
            false,
        ),
        (
            "collisionex0,a,polygon,1,2,3,4,5,6,7",
            Some(CollisionExFault::OddCoordinates),
            false,
        ),
        (
            "collisionex0,a,rect,1,x,3,4",
            Some(CollisionExFault::NotInteger),
            false,
        ),
        (
            "collisionex0,a,rect,1,,3,4",
            Some(CollisionExFault::NotInteger),
            false,
        ),
        (
            "collisionex0,a,region,,1,2,3",
            Some(CollisionExFault::BadRegion),
            false,
        ),
        (
            "collisionex0,a,region,f.png,1,256,3",
            Some(CollisionExFault::BadRegion),
            false,
        ),
        (
            "collisionex0,a,region,f.png,1,2,3,yes",
            Some(CollisionExFault::BadRegion),
            false,
        ),
        (
            "collisionex0,a,region,f.png,1,2",
            Some(CollisionExFault::BadRegion),
            false,
        ),
        (
            "collisionex0,a,rect,1,2,3,4,x",
            Some(CollisionExFault::ExtraFields),
            true,
        ),
        (
            "collisionex0,a,circle,1,2,3,4",
            Some(CollisionExFault::ExtraFields),
            true,
        ),
        (
            "collisionex0,a,region,f.png,1,2,3,false,x",
            Some(CollisionExFault::ExtraFields),
            true,
        ),
        ("collisionex0,a,rect,1,2,3,4,", None, true),
    ];
    for (row, fault, kept) in cases {
        let (list, issues) = surface0(&format!(
            "{row}
collision9,0,0,1,1,next"
        ));
        assert_eq!(list.len(), if kept { 2 } else { 1 }, "{row}");
        assert_eq!(list.last().unwrap().name.as_str(), "next", "{row}");
        let records: Vec<_> = issues
            .into_iter()
            .filter(|i| !matches!(i, ShellIssue::ShapesInactive { .. }))
            .collect();
        assert_eq!(
            records,
            fault.map(|f| ex(row, f)).into_iter().collect::<Vec<_>>(),
            "{row}"
        );
    }
}

/// 番号が読めない行は collision* と同じ記録で、欄を `,` で繋いだ行を運ぶ（4.8・4.9）。
#[test]
fn unreadable_collisionex_number_records_the_joined_row() {
    let (list, issues) = surface0(
        "collisionex,a,rect,1,2,3,4
collisionex99999999999,a,rect,1,2,3,4
collisionexfoo,a,rect,1,2,3,4",
    );
    assert!(list.is_empty());
    assert_eq!(
        issues,
        vec![
            ShellIssue::UnreadableNumber {
                heading: "surface0".into(),
                text: "collisionex,a,rect,1,2,3,4".into()
            },
            ShellIssue::UnreadableNumber {
                heading: "surface0".into(),
                text: "collisionex99999999999,a,rect,1,2,3,4".into()
            },
        ]
    );
}

/// 矩形と collision* だけのシェルではまだ働かない形の記録を出さず、append の形は数える（4.7）。
#[test]
fn shapes_inactive_only_when_non_rect_shapes_were_kept() {
    let (_, issues) = surface0(
        "collisionex0,a,rect,1,2,3,4
collision1,1,2,3,4,b",
    );
    assert!(issues.is_empty(), "{issues:?}");
    let text = parse_all(
        "surface0
{
collisionex0,a,rect,1,2,3,4
}
surface.append0
{
collisionex1,b,circle,1,2,3
}
",
    );
    assert_eq!(text.shell.appends[0].collisions.len(), 1);
    assert_eq!(
        text.issues,
        vec![ShellIssue::ShapesInactive {
            heading: "surface.append0".into(),
            text: "collisionex1,b,circle,1,2,3".into(),
            counts: ShapeCounts {
                circle: 1,
                ..ShapeCounts::default()
            },
        }]
    );
}
