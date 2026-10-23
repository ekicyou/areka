//! 書き出しの走り（`offline_dump::run`）の分岐の試験と、その試験が使う検体の部品
//! （`offline_dump_test_support`）の確かめ（spec: areka-P0-offline-surface-dump 要件 12.6）。
//!
//! 部品で組んだゴーストを、書き出しの走りが使うのと同じ今のシェルの読み込み
//! （`load_shell_target`）で、窓を出さずに読む。

use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::{BindSet, ComposeError, Composer, PatternState};
use areka_emo_present::load_shell_target;
use log_capture_kit::{CapturedEvent, capture};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::offline_dump_test_support::{self as fixture, ShellSpec};
use super::{read_scope, run};
use crate::cli::OfflineDumpRequest;
use crate::log_history::capture_tests::hold;

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

/// 走りを 1 回回し、終了コードと記録を返す（写しの窓はプロセスに 1 つなので、窓の試験と同じ排他を握る）。
fn run_logged(req: &OfflineDumpRequest) -> (i32, Vec<CapturedEvent>) {
    let _window = hold();
    capture(|| run(req))
}

fn request(ghost_dir: &std::path::Path, error_log: &std::path::Path) -> OfflineDumpRequest {
    OfflineDumpRequest {
        ghost_dir: Some(ghost_dir.display().to_string()),
        error_log: Some(error_log.display().to_string()),
        ..OfflineDumpRequest::default()
    }
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 読んだシェルのフォルダ（読み込みまで進んだ走りの info から）。
fn loaded_shell_dir(events: &[CapturedEvent]) -> Option<String> {
    let loaded = events_named(events, "offline_dump_shell_loaded");
    assert!(loaded.len() <= 1, "シェルの読み込みは 1 度: {events:?}");
    loaded.first().map(|e| {
        e.field("shell_dir")
            .expect("読んだシェルの欄")
            .trim_matches('"')
            .to_string()
    })
}

fn finished(events: &[CapturedEvent]) -> &CapturedEvent {
    let last = events.last().expect("記録がある");
    assert_eq!(
        last.field_str("event"),
        Some("offline_dump_finished"),
        "終わりの info が最後: {events:?}"
    );
    assert_eq!(last.level, tracing::Level::INFO);
    last
}

/// ゴーストの読み込みの失敗（フォルダが無い・descript.txt が無い・値なし）は error を出し、
/// シェルを読まず、CSV を作らずに 0 を返す（2.6・1.8・8.4）。
#[test]
fn ghost_failures_return_zero_without_reading_a_shell_or_writing_csv() {
    let root = fixture::build_ghost(
        "offline-run-no-descript",
        None,
        &[fixture::variety_shell("master")],
    );
    let csv = root.path().join("errors.csv");
    let cases = [
        (
            "folder",
            request(&root.path().join("nothing"), &csv),
            "offline_dump_ghost_unreadable",
        ),
        (
            "descript",
            request(root.path(), &csv),
            "offline_dump_ghost_unreadable",
        ),
        (
            "value",
            OfflineDumpRequest {
                ghost_dir: None,
                ..request(root.path(), &csv)
            },
            "offline_dump_ghost_missing_value",
        ),
    ];
    for (case, req, event) in cases {
        let (code, events) = run_logged(&req);
        assert_eq!(code, 0, "{case}");
        let errors = events_named(&events, event);
        assert_eq!(errors.len(), 1, "{case}: {events:?}");
        assert_eq!(errors[0].level, tracing::Level::ERROR, "{case}");
        assert_eq!(loaded_shell_dir(&events), None, "{case}: シェルを読まない");
        assert_eq!(
            events[0].field_str("event"),
            Some("offline_dump_started"),
            "{case}"
        );
        assert_eq!(finished(&events).field("exit_code"), Some("0"), "{case}");
        assert!(!csv.exists(), "{case}: CSV を作らない");
    }
}

/// ゴーストのフォルダの末尾の `\` の有無で同じ（既定の）シェルを読む（2.1・2.3）。
#[test]
fn a_trailing_backslash_on_the_ghost_folder_reads_the_same_shell() {
    let root = fixture::build_ghost(
        "offline-run-trailing",
        Some(&["name,g"]),
        &[fixture::variety_shell("master")],
    );
    let csv = root.path().join("errors.csv");
    let plain = request(root.path(), &csv);
    let slashed = OfflineDumpRequest {
        ghost_dir: Some(format!("{}\\", root.path().display())),
        ..request(root.path(), &csv)
    };
    let (_, a) = run_logged(&plain);
    let (_, b) = run_logged(&slashed);
    assert_eq!(finished(&a).field("written"), Some("0"));
    let (a, b) = (loaded_shell_dir(&a), loaded_shell_dir(&b));
    assert!(a.as_deref().is_some_and(|d| d.ends_with("master")), "{a:?}");
    assert_eq!(a, b);
}

/// `--dump-shell` は名前でもフォルダ名でも見つかり、`random` はただの名前として探す。見つからなければ
/// Error を 1 件残してシェルを読まない（2.4・2.5）。
#[test]
fn dump_shell_is_found_by_name_or_folder_and_a_miss_is_one_error() {
    let second = ShellSpec {
        folder: "second",
        descript: &["name,第二"],
        ..fixture::variety_shell("second")
    };
    let root = fixture::build_ghost(
        "offline-run-dump-shell",
        Some(&["name,g"]),
        &[fixture::variety_shell("master"), second],
    );
    let csv = root.path().join("errors.csv");
    for want in ["第二", "second", "SECOND"] {
        let req = OfflineDumpRequest {
            shell: Some(want.to_string()),
            ..request(root.path(), &csv)
        };
        let (_, events) = run_logged(&req);
        let dir = loaded_shell_dir(&events);
        assert!(
            dir.as_deref().is_some_and(|d| d.ends_with("second")),
            "{want}: {dir:?}"
        );
    }
    for want in ["nothing", "random", "lastinstalled"] {
        let req = OfflineDumpRequest {
            shell: Some(want.to_string()),
            ..request(root.path(), &csv)
        };
        let (_, events) = run_logged(&req);
        let missing = events_named(&events, "offline_dump_shell_missing");
        assert_eq!(missing.len(), 1, "{want}: {events:?}");
        assert_eq!(missing[0].level, tracing::Level::ERROR);
        assert_eq!(
            loaded_shell_dir(&events),
            None,
            "{want}: 既定のシェルへ代えない"
        );
    }
}

/// 既定のシェルのフォルダが無ければ走りが Error を 1 件残す（2.7）。
#[test]
fn a_missing_shell_folder_is_one_error() {
    let root = fixture::build_ghost("offline-run-no-shell", Some(&["name,g"]), &[]);
    let (_, events) = run_logged(&request(root.path(), &root.path().join("errors.csv")));
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    assert_eq!(
        errors[0].field_str("event"),
        Some("offline_dump_shell_unreadable")
    );
    assert_eq!(loaded_shell_dir(&events), None);
}

/// スコープは無ければ 0、0 以上の整数として読めなければ 0 と Warning（3.8・3.10）。
#[test]
fn the_scope_defaults_to_zero_and_an_unreadable_value_warns() {
    let (zero, events) = capture(|| read_scope(None));
    assert_eq!(zero, 0);
    assert!(events.is_empty(), "{events:?}");
    let (two, events) = capture(|| read_scope(Some("2")));
    assert_eq!(two, 2);
    assert!(events.is_empty(), "{events:?}");
    for raw in ["x", "-1", " 1", "1.5"] {
        let (scope, events) = capture(|| read_scope(Some(raw)));
        assert_eq!(scope, 0, "{raw}");
        assert_eq!(events.len(), 1, "{raw}: {events:?}");
        assert_eq!(events[0].level, tracing::Level::WARN, "{raw}");
        assert_eq!(
            events[0].field_str("event"),
            Some("offline_dump_scope_unreadable"),
            "{raw}"
        );
    }
}

/// スコープの読みは窓を開けた直後・`--dump-shell` の前（CSV の行の順が design と同じ）。
#[test]
fn the_scope_warning_comes_before_the_dump_shell_error() {
    let root = fixture::build_ghost(
        "offline-run-order",
        Some(&["name,g"]),
        &[fixture::variety_shell("master")],
    );
    let req = OfflineDumpRequest {
        scope: Some("x".to_string()),
        shell: Some("nothing".to_string()),
        ..request(root.path(), &root.path().join("errors.csv"))
    };
    let (_, events) = run_logged(&req);
    let order: Vec<_> = events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .filter_map(|e| e.field_str("event"))
        .collect();
    assert_eq!(
        order,
        [
            "offline_dump_scope_unreadable",
            "offline_dump_shell_missing"
        ]
    );
}
