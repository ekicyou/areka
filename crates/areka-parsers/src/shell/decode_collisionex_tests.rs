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
/// 理由が 2 つ当てはまる行は System Flows の図で先の理由 1 つだけ（未知の形→数→奇数→整数／region→欄が多い）。
#[test]
fn faults_record_once_and_only_extra_fields_enter_the_list() {
    let cases: [(&str, Option<CollisionExFault>, bool); 18] = [
        // 2 つの理由が重なる行: 先の理由だけ
        (
            "collisionex0,a,polygon,1,2,3,4,5,6,x",
            Some(CollisionExFault::OddCoordinates),
            false,
        ),
        (
            "collisionex0,a,rect,1,x,3",
            Some(CollisionExFault::MissingCoordinates),
            false,
        ),
        (
            "collisionex0,a,star",
            Some(CollisionExFault::UnknownShape),
            false,
        ),
        (
            "collisionex0,a,region,,1,2,3,false,x",
            Some(CollisionExFault::BadRegion),
            false,
        ),
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

fn region(file: &str, rgb: (u8, u8, u8), invert: bool) -> CollisionShape {
    CollisionShape::Region(CollisionRegion {
        file: file.into(),
        red: rgb.0,
        green: rgb.1,
        blue: rgb.2,
        invert,
        x: 0,
        y: 0,
    })
}

/// 5 つの形の欄が型へ運ばれる。形の名前は大小文字を区別せず、名前は空も大小文字もそのまま（1.1〜1.7・1.10）。
#[test]
fn each_shape_carries_its_fields_and_names_stay_verbatim() {
    let (list, _) = surface0(
        "collisionex0,,RECT,-1,-2,3,4
collisionex1,MiXed,Ellipse,1,2,3,4
collisionex2,,cIrCle,-5,6,7
collisionex3,Poly,POLYGON,9,8,7,6,5,4,3,2,
collisionex4,File,Region,Dir/A.png,0,128,255",
    );
    assert_eq!(
        list,
        vec![
            coll(0, "", rect(-1, -2, 3, 4)),
            coll(
                1,
                "MiXed",
                CollisionShape::Ellipse(CollisionBounds {
                    left: 1,
                    top: 2,
                    right: 3,
                    bottom: 4
                })
            ),
            coll(
                2,
                "",
                CollisionShape::Circle {
                    x: -5,
                    y: 6,
                    radius: 7
                }
            ),
            // 頂点は書いた順・行末の `,` は欄に数えない
            coll(
                3,
                "Poly",
                CollisionShape::Polygon(vec![(9, 8), (7, 6), (5, 4), (3, 2)])
            ),
            coll(4, "File", region("Dir/A.png", (0, 128, 255), false)),
        ]
    );
}

/// 領域反転は大小文字を区別せず、省略は false（1.7）。
#[test]
fn region_invert_is_case_insensitive_and_defaults_to_false() {
    let (list, issues) = surface0(
        "collisionex0,a,region,f.png,1,2,3
collisionex1,b,region,f.png,1,2,3,False
collisionex2,c,region,f.png,1,2,3,TRUE
collisionex3,d,region,f.png,1,2,3,tRuE
collisionex4,e,region,f.png,1,2,3,false,",
    );
    let inverts: Vec<bool> = list
        .iter()
        .map(|c| match &c.shape {
            CollisionShape::Region(r) => r.invert,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(inverts, [false, false, true, true, false]);
    assert!(
        issues
            .iter()
            .all(|i| matches!(i, ShellIssue::ShapesInactive { .. })),
        "{issues:?}"
    );
}

/// 読めない行の後の行が読まれ続け、collision* と交互に書いた並びが書いた順（1.8・4.8）。
#[test]
fn rows_after_unreadable_rows_keep_being_read_in_written_order() {
    let (list, issues) = surface0(
        "collisionex0,a,rect,1,1,2,2
collisionex,x,rect,1,1,2,2
collision1,0,0,5,5,b
collisionexzz,x,rect,1,1,2,2
collisionex2,c,rect,3,3,4,4
collisionex99999999999,x,rect,1,1,2,2
collision3,0,0,6,6,d
collisionex4,e,star,1,2,3,4
collisionex5,f,rect,7,7,8,8",
    );
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["a", "b", "c", "d", "f"]);
    assert_eq!(
        issues,
        vec![
            ShellIssue::UnreadableNumber {
                heading: "surface0".into(),
                text: "collisionex,x,rect,1,1,2,2".into()
            },
            ShellIssue::UnreadableNumber {
                heading: "surface0".into(),
                text: "collisionex99999999999,x,rect,1,1,2,2".into()
            },
            ex(
                "collisionex4,e,star,1,2,3,4",
                CollisionExFault::UnknownShape
            ),
        ]
    );
}

/// `surface.append*` の中でも同じに読み、記録の見出しは append の見出し（1.8・4.9）。
#[test]
fn rows_inside_surface_append_are_read_and_recorded_under_its_heading() {
    let text = parse_all(
        "surface.append0
{
collisionex0,a,circle,1,2,3
collisionex1,b,rect,1,2,3
collision2,0,0,1,1,c
collisionex3,d,rect,1,2,3,4,
}
",
    );
    let append = &text.shell.appends[0].collisions;
    let names: Vec<&str> = append.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["a", "c", "d"]);
    assert_eq!(
        text.issues,
        vec![
            ShellIssue::CollisionEx {
                heading: "surface.append0".into(),
                text: "collisionex1,b,rect,1,2,3".into(),
                fault: CollisionExFault::MissingCoordinates,
            },
            ShellIssue::ShapesInactive {
                heading: "surface.append0".into(),
                text: "collisionex0,a,circle,1,2,3".into(),
                counts: ShapeCounts {
                    circle: 1,
                    ..ShapeCounts::default()
                },
            },
        ]
    );
}

/// まだ働かない形の記録は最後に 1 件・並びへ入れた形だけを数え・最初の行を運ぶ。使わない行は数えない（4.7）。
#[test]
fn shapes_inactive_is_last_exactly_one_and_counts_only_kept_shapes() {
    let (_, issues) = surface0(
        "collisionex0,r,rect,1,2,3,4
collisionex1,bad,ellipse,1,2
collisionex2,p,polygon,0,0,1,1,2,2
collisionex3,e,ellipse,0,0,9,9,extra
collisionex4,g,region,f.png,1,2,3
collisionex5,h,circle,1,2,3",
    );
    assert_eq!(issues.len(), 3, "{issues:?}");
    assert_eq!(
        issues[0],
        ex(
            "collisionex1,bad,ellipse,1,2",
            CollisionExFault::MissingCoordinates
        )
    );
    assert_eq!(
        issues[1],
        ex(
            "collisionex3,e,ellipse,0,0,9,9,extra",
            CollisionExFault::ExtraFields
        )
    );
    assert_eq!(
        issues[2],
        ShellIssue::ShapesInactive {
            heading: "surface0".into(),
            text: "collisionex2,p,polygon,0,0,1,1,2,2".into(),
            counts: ShapeCounts {
                ellipse: 1,
                circle: 1,
                polygon: 1,
                region: 1
            },
        }
    );
}

/// まだ働かない形の記録はシェル全体で 1 件・ブレスをまたいで数え、最初のブレスの最初の行を運ぶ（4.7）。
#[test]
fn shapes_inactive_is_one_record_for_the_whole_shell_across_sections() {
    let text = parse_all(
        "surface0
{
collisionex0,a,ellipse,0,0,9,9
}
surface1
{
collisionex0,b,star,1
collisionex1,c,circle,1,2,3
}
surface.append0
{
collisionex0,d,polygon,0,0,1,1,2,2
collisionex1,e,region,f.png,1,2,3
}
",
    );
    assert_eq!(
        text.issues,
        vec![
            ShellIssue::CollisionEx {
                heading: "surface1".into(),
                text: "collisionex0,b,star,1".into(),
                fault: CollisionExFault::UnknownShape,
            },
            ShellIssue::ShapesInactive {
                heading: "surface0".into(),
                text: "collisionex0,a,ellipse,0,0,9,9".into(),
                counts: ShapeCounts {
                    ellipse: 1,
                    circle: 1,
                    polygon: 1,
                    region: 1
                },
            },
        ]
    );
}
