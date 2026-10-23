//! 書き出しの走り（`offline_dump::run`）の分岐の試験と、その試験が使う検体の部品
//! （`offline_dump_test_support`）の確かめ（spec: areka-P0-offline-surface-dump 要件 12.6）。
//!
//! 部品で組んだゴーストを、書き出しの走りが使うのと同じ今のシェルの読み込み
//! （`load_shell_target`）で、窓を出さずに読む。

use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::{BindSet, ComposeError, Composer, PatternState};
use areka_emo_present::load_shell_target;
use log_capture_kit::capture;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::offline_dump_test_support::{self as fixture, ShellSpec};

fn com_init() {
    // SAFETY: テストスレッドごとの COM 初期化。結果は既初期化でも続けてよいので捨てる。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// research.md §10 の形を 1 つのシェルに持つ検体が、今のシェルの読み込みで窓なしに読める。
/// 絵の欠けの面はほかの層で組め、中身の無い面は読み込みに残って合成だけが error 1 行で失敗する。
#[test]
fn the_variety_ghost_loads_with_the_current_shell_loader_without_a_window() {
    com_init();
    let root = fixture::build_ghost(
        "offline-variety",
        Some(&["name,g_variety"]),
        &[fixture::variety_shell("master")],
    );
    let ghost_descript = std::fs::read(root.path().join("ghost/master/descript.txt"))
        .expect("ゴーストの descript.txt が在る");
    assert_eq!(ghost_descript, b"charset,UTF-8\r\nname,g_variety\r\n");

    let arm = WicDecoderArm::new().expect("WIC の工場が作れる");
    let (target, load_events) =
        capture(|| load_shell_target(&root.path().join("shell/master"), &arm));
    let target = target.expect("検体のシェルは読める");
    let load_errors: Vec<_> = load_events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(
        load_errors.len(),
        1,
        "読み込みの error は欠けた絵の 1 行だけ: {load_events:?}"
    );
    assert!(
        load_errors[0]
            .field("error")
            .is_some_and(|e| e.contains("nofile.png")),
        "欠けた絵の error: {:?}",
        load_errors[0]
    );
    assert_eq!(
        target.bake_errors().len(),
        1,
        "焼く段で落ちるのは欠けた絵 1 枚だけ: {:?}",
        target.bake_errors()
    );

    let world = target.build_world();
    let ids: Vec<u32> = world.surface_ids().collect();
    assert_eq!(
        ids,
        fixture::VARIETY_SURFACES,
        "中身の無い面も読み込みに残る"
    );

    let mut composer = Composer::new();
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    for id in fixture::VARIETY_SURFACES
        .into_iter()
        .filter(|&id| id != fixture::EMPTY)
    {
        let pic = composer
            .compose(&world, target.atlas(), id, &binds, &pattern)
            .unwrap_or_else(|e| panic!("面 {id} は組める: {e:?}"));
        assert!(pic.width() > 0 && pic.height() > 0, "面 {id} の絵が空");
    }
    let (empty, events) =
        capture(|| composer.compose(&world, target.atlas(), fixture::EMPTY, &binds, &pattern));
    assert!(
        matches!(empty, Err(ComposeError::EmptyComposition(id)) if id == fixture::EMPTY),
        "中身の無い面の合成は失敗する: {empty:?}"
    );
    let errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .count();
    assert_eq!(errors, 1, "合成の失敗の error は 1 行: {events:?}");
}

/// ゴーストの descript.txt を置かない検体と、名前の違うシェルを 2 つ持つ検体を組める。
#[test]
fn the_ghost_without_descript_keeps_every_shell_folder() {
    let second = ShellSpec {
        folder: "second",
        descript: &["name,第二"],
        ..fixture::variety_shell("second")
    };
    let root = fixture::build_ghost(
        "offline-two-shells",
        None,
        &[fixture::variety_shell("master"), second],
    );
    assert!(root.path().join("ghost/master").is_dir());
    assert!(!root.path().join("ghost/master/descript.txt").exists());
    let descript = std::fs::read_to_string(root.path().join("shell/second/descript.txt"))
        .expect("2 つ目のシェルの descript.txt");
    assert_eq!(
        descript,
        "charset,UTF-8\r\nseriko.use_self_alpha,1\r\nname,第二\r\n"
    );
    for folder in ["master", "second"] {
        let shell = root.path().join("shell").join(folder);
        assert!(shell.join("surfaces.txt").is_file(), "{folder}");
        assert!(
            !shell.join("surface6.png").exists(),
            "中身の無い面の絵は置かない"
        );
        let corner = image::open(shell.join(fixture::CORNER_IMAGE))
            .expect("1 色塗りの PNG を読める")
            .into_rgba8();
        assert_eq!(corner.dimensions(), (4, 4));
        assert!(
            corner.pixels().all(|p| p.0 == fixture::CORNER_RGBA),
            "α 付きの 1 色"
        );
    }
}
