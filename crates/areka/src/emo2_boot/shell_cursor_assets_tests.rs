//! `shell_cursor_assets.rs` の兄弟テスト（shell-mouse-cursor tasks.md 3.1・要件 1.3・1.4・5.6〜5.8・
//! 6.3〜6.9・7.2）。
//!
//! シェルとゴーストはワークツリーの `target\test-roots\` の下に組み、`.cur` はテストの中で組んだ
//! 2 寸法（32・64）の小さな正しいファイルを置く（同梱の絵は読まない）。

use std::collections::BTreeMap;
use std::path::Path;

use areka_parsers::shell::parse_cursor_braces;
use bevy_ecs::prelude::*;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;
use wintf::ecs::cursor::{CursorAsk, CursorQuery, CursorReply, cursor_size_for_dpi};

use super::{CursorFile, ShellCursorAssets, build_shell_cursors};
use crate::emo2_boot::shell_cursor_table::{CursorScene, Shape, SystemCursor};
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

const TARGET: &str = "areka::emo2_boot::shell_cursor_assets";

/// 1 寸法ぶんの DIB（BITMAPINFOHEADER・32bpp の色・1bpp の抜き）。高さは色と抜きの 2 倍で書く。
fn dib(side: u32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(side as i32).to_le_bytes());
    b.extend_from_slice(&(2 * side as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 24]);
    b.extend(std::iter::repeat_n(0xFFu8, (side * side * 4) as usize));
    let mask_row = side.div_ceil(32) * 4;
    b.extend(std::iter::repeat_n(0u8, (mask_row * side) as usize));
    b
}

/// 32 と 64 の 2 項目を持つ正しい `.cur`。
fn two_size_cur() -> Vec<u8> {
    let images = [dib(32), dib(64)];
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 2, 0, 2, 0]);
    let mut offset = 6 + 16 * images.len();
    for (side, image) in [32u8, 64].iter().zip(&images) {
        out.extend_from_slice(&[*side, *side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&(image.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += image.len();
    }
    for image in &images {
        out.extend_from_slice(image);
    }
    out
}

/// シェルのフォルダと `ghost\master` を `target\` の下に組む。
struct Fixture {
    _root: TempPath,
    shell: std::path::PathBuf,
    ghost: std::path::PathBuf,
}

fn fixture(label: &str) -> Fixture {
    let root = TempPath::under_target(label);
    let shell = root.path().join("shell").join("master");
    let ghost = root.path().join("ghost").join("master");
    std::fs::create_dir_all(&shell).unwrap();
    std::fs::create_dir_all(&ghost).unwrap();
    Fixture {
        _root: root,
        shell,
        ghost,
    }
}

fn put(dir: &Path, name: &str, bytes: &[u8]) {
    std::fs::write(dir.join(name), bytes).unwrap();
}

fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn build(fx: &Fixture, surfaces: &str, ghost_kv: &[(&str, &str)]) -> ShellCursorAssets {
    build_shell_cursors(
        &parse_cursor_braces(surfaces),
        &kv(ghost_kv),
        &fx.shell,
        &fx.ghost,
    )
}

fn key(scope: u32, region: &str, scene: CursorScene) -> (u32, String, CursorScene) {
    (scope, region.to_string(), scene)
}

fn mine(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events.iter().filter(|e| e.target == TARGET).collect()
}

fn warns_for<'a>(events: &'a [CapturedEvent], file: &str) -> Vec<&'a CapturedEvent> {
    mine(events)
        .into_iter()
        .filter(|e| e.level == Level::WARN && e.field_str("file") == Some(file))
        .collect()
}

#[test]
fn shell_cur_is_used_and_sized_per_dpi_once() {
    let fx = fixture("cursor-assets-shell");
    put(&fx.shell, "a.cur", &two_size_cur());
    let assets = build(&fx, "sakura.cursor\n{\nmouseup0,Head,a.cur\n}\n", &[]);

    assert_eq!(
        assets.rows.rows.get(&key(0, "Head", CursorScene::Up)),
        Some(&Shape::File(0))
    );
    assert!(matches!(assets.files.as_slice(), [CursorFile::Cur(_)]));

    let (small, large) = (cursor_size_for_dpi(96), cursor_size_for_dpi(192));
    assert_ne!(small, large);
    let first = assets.cursor_for(0, small).expect("96 の寸法で作れる");
    let second = assets.cursor_for(0, large).expect("192 の寸法で作れる");
    assert_ne!(first, second);
    // 2 回目は作り直さない（同じカーソルが返り、控えは寸法 2 つのまま）。
    assert_eq!(assets.cursor_for(0, small), Some(first));
    assert_eq!(assets.cursor_for(0, large), Some(second));
    assert_eq!(assets.sized.borrow().len(), 2);
    // 番号の無いファイルは引けない。
    assert_eq!(assets.cursor_for(1, small), None);
}

#[test]
fn cur_that_cannot_be_created_caches_none_and_warns_once_per_size() {
    let fx = fixture("cursor-assets-uncreatable");
    // 目録は正しいが絵の本体が DIB でない（作る段で断られる）。
    let mut bytes = two_size_cur();
    let body = 6 + 16 * 2;
    bytes[body..body + 4].copy_from_slice(&[1, 2, 3, 4]);
    put(&fx.shell, "odd.cur", &bytes);
    let assets = build(&fx, "sakura.cursor\n{\nmouseup0,Head,odd.cur\n}\n", &[]);
    let size = cursor_size_for_dpi(96);

    let (got, events) = capture(|| (assets.cursor_for(0, size), assets.cursor_for(0, size)));
    assert_eq!(got, (None, None));
    assert_eq!(
        assets.sized.borrow().get(&(0, size)).map(Option::is_none),
        Some(true)
    );
    let warns: Vec<_> = mine(&events)
        .into_iter()
        .filter(|e| e.level == Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "寸法ごとに 1 度だけ");
    assert_eq!(warns[0].field("size"), Some(size.to_string().as_str()));
}

#[test]
fn file_only_in_ghost_master_is_used_for_rows_and_descript() {
    let fx = fixture("cursor-assets-ghost");
    put(&fx.ghost, "g.cur", &two_size_cur());
    let assets = build(
        &fx,
        "sakura.cursor\n{\nmousedown0,Head,g.cur\n}\n",
        &[("mousecursor", "g.cur"), ("mousecursor.arrow", "system:no")],
    );

    assert_eq!(
        assets.rows.rows.get(&key(0, "Head", CursorScene::Down)),
        Some(&Shape::File(0))
    );
    assert_eq!(assets.ghost.finger, Some(Shape::File(0)));
    assert_eq!(assets.ghost.arrow, Some(Shape::System(SystemCursor::No)));
    assert_eq!(assets.files.len(), 1);
    assert!(assets.cursor_for(0, cursor_size_for_dpi(96)).is_some());
}

#[test]
fn shell_folder_wins_over_ghost_master() {
    let fx = fixture("cursor-assets-order");
    put(&fx.shell, "a.cur", &two_size_cur());
    put(&fx.ghost, "a.cur", b"broken");
    let (assets, events) =
        capture(|| build(&fx, "sakura.cursor\n{\nmouseup0,Head,a.cur\n}\n", &[]));
    assert_eq!(assets.files.len(), 1);
    assert!(warns_for(&events, "a.cur").is_empty());
}

#[test]
fn missing_broken_outside_warn_once_per_name_and_drop_rows() {
    let fx = fixture("cursor-assets-fail");
    put(&fx.shell, "bad.cur", b"not a cursor");
    put(&fx.shell, "bad.ani", b"not an animated cursor");
    put(&fx.shell, "ok.cur", &two_size_cur());
    let surfaces = "\
sakura.cursor
{
mouseup0,A,none.cur
mousedown0,A,none.cur
mouseup0,B,bad.cur
mousedown0,B,bad.cur
mouseup0,C,..\\up.cur
mousedown0,C,..\\up.cur
mouseup0,D,bad.ani
mouseup0,E,ok.cur
mouseup0,F,system:help
}
";
    let (assets, events) = capture(|| {
        build(
            &fx,
            surfaces,
            &[
                ("mousecursor.hand", "none.cur"),
                ("mousecursor.grip", "bad.cur"),
            ],
        )
    });

    for name in ["none.cur", "bad.cur", "..\\up.cur", "bad.ani"] {
        assert_eq!(warns_for(&events, name).len(), 1, "{name} は 1 度だけ");
    }
    // 探した場所（シェル → ghost\master）が無いファイルの記録に載る。
    let searched = warns_for(&events, "none.cur")[0]
        .field("searched")
        .unwrap()
        .to_string();
    assert!(
        searched.contains("shell") && searched.contains("ghost"),
        "{searched}"
    );
    // 作れなかった名前を使う行と差し替えは表から外れる（行が無いのと同じ既定）。
    let want: BTreeMap<_, _> = [
        (key(0, "E", CursorScene::Up), Shape::File(0)),
        (
            key(0, "F", CursorScene::Up),
            Shape::System(SystemCursor::Help),
        ),
    ]
    .into_iter()
    .collect();
    assert_eq!(assets.rows.rows, want);
    assert_eq!(assets.ghost.hand, None);
    assert_eq!(assets.ghost.grip, None);
    assert_eq!(assets.files.len(), 1);
}

#[test]
fn same_name_on_two_rows_is_read_once() {
    let fx = fixture("cursor-assets-shared");
    put(&fx.shell, "a.cur", &two_size_cur());
    let assets = build(
        &fx,
        "sakura.cursor\n{\nmouseup0,Head,a.cur\nmousedown0,Head,a.cur\n}\nkero.cursor\n{\nmouseup0,Face,a.cur\n}\n",
        &[("mousecursor", "a.cur")],
    );
    assert_eq!(assets.files.len(), 1);
    assert_eq!(assets.rows.rows.len(), 3);
    assert!(assets.rows.rows.values().all(|s| *s == Shape::File(0)));
    assert_eq!(assets.ghost.finger, Some(Shape::File(0)));
}

#[test]
fn table_reports_are_logged_at_the_design_levels() {
    let fx = fixture("cursor-assets-levels");
    let surfaces = "\
sakura.cursor
{
mouseup0,Head
mouseup0,Head,system:hand
mouseup0,Head,system:grip
}
other.cursor
{
mouseup0,Head,system:hand
}
";
    let ghost = [
        ("mousecursor", "system:hand"),
        ("cursor", "system:grip"),
        ("mousecursor.arrow", "x.bmp"),
        ("mousecursor.text", "system:text"),
        ("mousecursor.wait", "system:wait"),
    ];
    let (_, events) = capture(|| build(&fx, surfaces, &ghost));
    let events = mine(&events);
    let count = |level: Level| events.iter().filter(|e| e.level == level).count();
    // 見出し・行・重なり・descript の値・両方の綴り＝warn 5。text・wait＝info 2。読み終わり＝debug 1。
    assert_eq!(count(Level::WARN), 5);
    assert_eq!(count(Level::INFO), 2);
    assert_eq!(count(Level::DEBUG), 1);
    let done = events.iter().find(|e| e.level == Level::DEBUG).unwrap();
    assert_eq!(done.field("rows"), Some("1"));
    assert_eq!(done.field("files"), Some("0"));
}

fn no_answer(_: &World, _: Entity, _: &CursorAsk) -> Option<CursorReply> {
    None
}

#[test]
fn hand_to_refreshes_char_windows_then_drops_the_old_assets() {
    let fx = fixture("cursor-assets-swap");
    put(&fx.shell, "a.cur", &two_size_cur());
    let mut world = World::new();
    world.insert_resource(CursorQuery(no_answer));
    let chars = [
        world.spawn(CharWindowMarker { scope: 0 }).id(),
        world.spawn(CharWindowMarker { scope: 1 }).id(),
    ];
    world.spawn(BalloonWindowMarker { scope: 0 });

    build(&fx, "sakura.cursor\n{\nmouseup0,Old,a.cur\n}\n", &[]).hand_to(&mut world);
    let new = build(&fx, "sakura.cursor\n{\nmouseup0,New,system:hand\n}\n", &[]);
    let ((), events) = capture(|| new.hand_to(&mut world));

    let rows = &world.non_send::<ShellCursorAssets>().rows.rows;
    assert!(rows.contains_key(&key(0, "New", CursorScene::Up)));
    assert!(!rows.contains_key(&key(0, "Old", CursorScene::Up)));

    // キャラクターの窓ごとに出し直し（WindowHandle の無い窓は wintf が trace を残して止まる）が先、
    // 前の資源を捨てるのが後。
    let refreshed: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.message() == "push_cursor: no WindowHandle")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(refreshed.len(), chars.len());
    let dropped = events
        .iter()
        .position(|e| e.target == TARGET && e.level == Level::DEBUG)
        .expect("前の資源を捨てた記録");
    assert_eq!(events[dropped].field("files"), Some("1"));
    assert!(refreshed.iter().all(|&i| i < dropped));
}

#[test]
fn hand_to_into_an_empty_world_has_nothing_to_drop() {
    let fx = fixture("cursor-assets-first");
    let mut world = World::new();
    let assets = build(&fx, "", &[]);
    let ((), events) = capture(|| assets.hand_to(&mut world));
    assert!(world.get_non_send::<ShellCursorAssets>().is_some());
    assert!(mine(&events).is_empty());
}
