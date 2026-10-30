//! `shell_cursor_table.rs` の兄弟テスト（shell-mouse-cursor tasks.md 2.1・要件 1.1〜1.4・1.6・5.1〜5.4・
//! 5.6〜5.8・6.4・6.6・7.2）。
//!
//! cursorブレスはテストの中の surfaces.txt の文面を `parse_cursor_braces` で転記して使う（検体は読まない）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_parsers::shell::parse_cursor_braces;

use super::{
    CursorIssue, CursorScene, GhostCursorKeys, PathReason, RowReason, Shape, ShapeReason,
    SystemCursor, candidate_paths, parse_shape, read_cursor_braces, read_ghost_cursor_keys,
};

type Rows = BTreeMap<(u32, String, CursorScene), Shape<String>>;

fn rows_of(text: &str) -> (Rows, Vec<CursorIssue>) {
    let (rows, issues) = read_cursor_braces(&parse_cursor_braces(text));
    (rows.rows, issues)
}

fn key(scope: u32, region: &str, scene: CursorScene) -> (u32, String, CursorScene) {
    (scope, region.to_string(), scene)
}

fn file(name: &str) -> Shape<String> {
    Shape::File(name.to_string())
}

fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn three_headings_map_to_scopes_and_five_scenes_are_read() {
    let (rows, issues) = rows_of(
        "\
sakura.cursor
{
mouseup0,Head,system:hand
mousedown0,Head,system:grip
mouserightdown0,Head,system:help
mousehover0,Head,system:wait
mousewheel0,Head,system:move
}
kero.cursor
{
mouseup0,Face,a.cur
}
char2.cursor
{
mouseup0,Tail,b.ani
}
",
    );
    assert_eq!(issues, vec![]);
    let want: BTreeMap<_, _> = [
        (
            key(0, "Head", CursorScene::Up),
            Shape::System(SystemCursor::Hand),
        ),
        (
            key(0, "Head", CursorScene::Down),
            Shape::System(SystemCursor::Grip),
        ),
        (
            key(0, "Head", CursorScene::RightDown),
            Shape::System(SystemCursor::Help),
        ),
        (
            key(0, "Head", CursorScene::Hover),
            Shape::System(SystemCursor::Wait),
        ),
        (
            key(0, "Head", CursorScene::Wheel),
            Shape::System(SystemCursor::Move),
        ),
        (key(1, "Face", CursorScene::Up), file("a.cur")),
        (key(2, "Tail", CursorScene::Up), file("b.ani")),
    ]
    .into_iter()
    .collect();
    assert_eq!(rows, want);
}

#[test]
fn unknown_cursor_heading_is_reported_and_its_rows_are_not_used() {
    let (rows, issues) = rows_of(
        "\
foo.cursor
{
mouseup0,Head,system:hand
}
charx.cursor
{
mouseup0,Head,system:hand
}
",
    );
    assert!(rows.is_empty());
    assert_eq!(
        issues,
        vec![
            CursorIssue::HeadingUnreadable {
                heading: "foo.cursor".into()
            },
            CursorIssue::HeadingUnreadable {
                heading: "charx.cursor".into()
            },
        ]
    );
}

#[test]
fn each_unreadable_reason_is_reported_and_other_rows_are_kept() {
    let (rows, issues) = rows_of(
        "\
sakura.cursor
{
mouseup0,Head
mouseover0,Head,system:hand
mouseupA,Head,system:hand
mouseup,Head,system:hand
mouseup0,,system:hand
mouseup0,Head,system:pointer
mouseup0,Head,hand.png
mousedown0,Head,system:grip
}
",
    );
    assert_eq!(
        rows,
        [(
            key(0, "Head", CursorScene::Down),
            Shape::System(SystemCursor::Grip)
        )]
        .into_iter()
        .collect()
    );
    let unreadable = |line: &str, reason| CursorIssue::RowUnreadable {
        heading: "sakura.cursor".into(),
        line: line.into(),
        reason,
    };
    assert_eq!(
        issues,
        vec![
            unreadable("mouseup0,Head", RowReason::TooFewFields),
            unreadable("mouseover0,Head,system:hand", RowReason::UnknownScene),
            unreadable("mouseupA,Head,system:hand", RowReason::NumberNotDigits),
            unreadable("mouseup,Head,system:hand", RowReason::NumberNotDigits),
            unreadable("mouseup0,,system:hand", RowReason::EmptyRegion),
            unreadable(
                "mouseup0,Head,system:pointer",
                RowReason::Shape(ShapeReason::UnknownSystemName)
            ),
            unreadable(
                "mouseup0,Head,hand.png",
                RowReason::Shape(ShapeReason::NotCurOrAni)
            ),
        ]
    );
}

#[test]
fn later_row_wins_and_earlier_row_is_reported_as_unused() {
    let (rows, issues) = rows_of(
        "\
sakura.cursor
{
mouseup0,Head,system:hand
mouseup1,Head,a.cur
}
sakura.cursor
{
mouseup2,Head,b.cur
}
kero.cursor
{
mouseup0,Head,system:help
}
",
    );
    assert_eq!(
        rows.get(&key(0, "Head", CursorScene::Up)),
        Some(&file("b.cur"))
    );
    // 別のスコープの同じ当たり判定名は重ならない。
    assert_eq!(
        rows.get(&key(1, "Head", CursorScene::Up)),
        Some(&Shape::System(SystemCursor::Help))
    );
    assert_eq!(
        issues,
        vec![
            CursorIssue::RowReplaced {
                heading: "sakura.cursor".into(),
                line: "mouseup0,Head,system:hand".into(),
            },
            CursorIssue::RowReplaced {
                heading: "sakura.cursor".into(),
                line: "mouseup1,Head,a.cur".into(),
            },
        ]
    );
}

#[test]
fn region_names_are_kept_as_spelled_without_surface_check() {
    // 面の当たり判定と突き合わせない（1.6）・綴りどおり（大小で別の鍵）。
    let (rows, issues) = rows_of(
        "\
sakura.cursor
{
mouseup0,NoSuchRegion,system:no
mouseup1,head,system:cross
mouseup2,Head,system:text
}
",
    );
    assert_eq!(issues, vec![]);
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.get(&key(0, "NoSuchRegion", CursorScene::Up)),
        Some(&Shape::System(SystemCursor::No))
    );
    assert_eq!(
        rows.get(&key(0, "head", CursorScene::Up)),
        Some(&Shape::System(SystemCursor::Cross))
    );
}

#[test]
fn third_and_later_fields_are_rejoined_with_comma() {
    let (rows, issues) = rows_of(
        "\
sakura.cursor
{
mouseup0,Head,a,b.cur
}
",
    );
    assert_eq!(issues, vec![]);
    assert_eq!(
        rows.get(&key(0, "Head", CursorScene::Up)),
        Some(&file("a,b.cur"))
    );
}

#[test]
fn system_names_and_extensions_ignore_ascii_case() {
    let all = [
        ("arrow", SystemCursor::Arrow),
        ("cross", SystemCursor::Cross),
        ("finger", SystemCursor::Finger),
        ("grip", SystemCursor::Grip),
        ("hand", SystemCursor::Hand),
        ("help", SystemCursor::Help),
        ("move", SystemCursor::Move),
        ("no", SystemCursor::No),
        ("text", SystemCursor::Text),
        ("wait", SystemCursor::Wait),
    ];
    for (name, want) in all {
        assert_eq!(
            parse_shape(&format!("system:{name}")),
            Ok(Shape::System(want))
        );
    }
    assert_eq!(
        parse_shape("system:HAND"),
        Ok(Shape::System(SystemCursor::Hand))
    );
    assert_eq!(
        parse_shape("system:Arrow"),
        Ok(Shape::System(SystemCursor::Arrow))
    );
    assert_eq!(parse_shape("A.CUR"), Ok(file("A.CUR")));
    assert_eq!(parse_shape("dir\\b.Ani"), Ok(file("dir\\b.Ani")));
    assert_eq!(parse_shape("system:"), Err(ShapeReason::UnknownSystemName));
    assert_eq!(parse_shape("a.ico"), Err(ShapeReason::NotCurOrAni));
    assert_eq!(parse_shape("cur"), Err(ShapeReason::NotCurOrAni));
    assert_eq!(parse_shape(""), Err(ShapeReason::NotCurOrAni));
}

#[test]
fn ghost_mousecursor_spelling_wins_over_cursor_and_is_reported() {
    let (keys, issues) =
        read_ghost_cursor_keys(&kv(&[("mousecursor", "a.cur"), ("cursor", "system:help")]));
    assert_eq!(keys.finger, Some(file("a.cur")));
    assert_eq!(
        issues,
        vec![CursorIssue::GhostBothSpellings {
            ignored: "system:help".into()
        }]
    );
}

#[test]
fn ghost_either_spelling_alone_is_the_finger_replacement() {
    let (keys, issues) = read_ghost_cursor_keys(&kv(&[("mousecursor", "system:cross")]));
    assert_eq!(keys.finger, Some(Shape::System(SystemCursor::Cross)));
    assert_eq!(issues, vec![]);
    let (keys, issues) = read_ghost_cursor_keys(&kv(&[("cursor", "old.cur")]));
    assert_eq!(keys.finger, Some(file("old.cur")));
    assert_eq!(issues, vec![]);
}

#[test]
fn ghost_hand_grip_arrow_and_unused_text_wait() {
    let (keys, issues) = read_ghost_cursor_keys(&kv(&[
        ("mousecursor.hand", "h.cur"),
        ("mousecursor.grip", "system:grip"),
        ("mousecursor.arrow", "ar.ani"),
        ("mousecursor.text", "system:text"),
        ("mousecursor.wait", "w.ani"),
        ("name", "ghost"),
    ]));
    assert_eq!(issues, vec![]);
    assert_eq!(
        keys,
        GhostCursorKeys {
            finger: None,
            hand: Some(file("h.cur")),
            grip: Some(Shape::System(SystemCursor::Grip)),
            arrow: Some(file("ar.ani")),
            unused: vec!["mousecursor.text", "mousecursor.wait"],
        }
    );
    let (empty, issues) = read_ghost_cursor_keys(&kv(&[("name", "ghost")]));
    assert_eq!(empty, GhostCursorKeys::default());
    assert_eq!(issues, vec![]);
}

#[test]
fn ghost_unreadable_values_are_reported_and_not_used() {
    let (keys, issues) = read_ghost_cursor_keys(&kv(&[
        ("mousecursor", "system:pointer"),
        ("mousecursor.hand", "h.png"),
        ("mousecursor.text", "t.bmp"),
    ]));
    assert_eq!(keys, GhostCursorKeys::default());
    assert_eq!(
        issues,
        vec![
            CursorIssue::GhostValueUnreadable {
                key: "mousecursor",
                value: "system:pointer".into(),
                reason: ShapeReason::UnknownSystemName,
            },
            CursorIssue::GhostValueUnreadable {
                key: "mousecursor.hand",
                value: "h.png".into(),
                reason: ShapeReason::NotCurOrAni,
            },
            CursorIssue::GhostValueUnreadable {
                key: "mousecursor.text",
                value: "t.bmp".into(),
                reason: ShapeReason::NotCurOrAni,
            },
        ]
    );
}

#[test]
fn candidate_paths_refuse_names_outside_the_folder() {
    let dirs = [Path::new("C:\\shell"), Path::new("C:\\ghost\\master")];
    for name in [
        "C:\\x\\a.cur",
        "C:a.cur",
        "\\a.cur",
        "/a.cur",
        "..\\a.cur",
        "sub\\..\\..\\a.cur",
    ] {
        assert_eq!(
            candidate_paths(name, &dirs),
            Err(PathReason::OutsideFolder),
            "{name}"
        );
    }
}

#[test]
fn candidate_paths_are_shell_then_ghost_master() {
    let dirs = [Path::new("C:\\shell"), Path::new("C:\\ghost\\master")];
    assert_eq!(
        candidate_paths("cur\\a.cur", &dirs),
        Ok(vec![
            PathBuf::from("C:\\shell\\cur\\a.cur"),
            PathBuf::from("C:\\ghost\\master\\cur\\a.cur"),
        ])
    );
    // `..` を含む綴りでも、上のフォルダへ出る部分でなければ断らない。
    assert_eq!(
        candidate_paths("a..b.cur", &dirs),
        Ok(vec![
            PathBuf::from("C:\\shell\\a..b.cur"),
            PathBuf::from("C:\\ghost\\master\\a..b.cur")
        ])
    );
}
