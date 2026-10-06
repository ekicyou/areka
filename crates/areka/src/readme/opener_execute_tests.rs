//! 開く処理の記録・失敗・順序のテスト（areka-P0-open-external-tags task 3.3・要件 2.5・3.3・4.8・
//! 5.4・6.4・7.2・7.3・7.4・7.6・7.8・10.5）。
//!
//! `execute`・`serve` はテストのスレッドで呼ぶ（`log_capture_kit` は呼んだスレッドの記録しか拾わない）。
//! 記録の種別は本物の `log_history::draft` で判定する。OS は偽物だけ。

use std::sync::mpsc;

use areka_parsers::sakura::JUMP_TAG_CARRIER;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::super::destination::{Destination, classify};
use super::super::opener_test_support::{FakeOs, build_ghost_root};
use super::{OpenContext, OpenJob, execute, serve};
use crate::log_history::{Draft, FieldText, Kind, TARGET_ERROR, draft};

/// 一時フォルダの最小のゴースト（`ghost/master/readme.txt` を置く）と、その文脈。
struct Fixture {
    _tmp: TempPath,
    ctx: OpenContext,
}

impl Fixture {
    fn new() -> Self {
        let tmp = TempPath::new("open-ext-execute");
        let root = build_ghost_root(&tmp, ("g", "G"), &[("s", "S")], &[]);
        let ghost_dir = root.ghost_dir("g");
        std::fs::write(ghost_dir.join("ghost/master/readme.txt"), b"x").unwrap();
        let ctx = OpenContext {
            ghost: "G".to_owned(),
            ghost_dir,
            baseware: Some(root),
        };
        Fixture { _tmp: tmp, ctx }
    }

    fn job(&self, name: &str, args: &[&str]) -> OpenJob {
        OpenJob {
            destination: dest(name, args),
            context: self.ctx.clone(),
        }
    }
}

/// 台本のタグの形から本物の規則で行き先を作る。
fn dest(name: &str, args: &[&str]) -> Destination {
    classify(name, args).expect("開く系").expect("受理される")
}

/// 1 件を実行し、記録と偽物を返す。
fn run(job: OpenJob, mut os: FakeOs) -> (Vec<CapturedEvent>, FakeOs) {
    let ((), events) = capture(|| execute(&mut os, job));
    (events, os)
}

/// 開く処理の行（`event` が `open_external` で始まる）だけ。
fn open_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| val(e, "event").is_some_and(|v| v.starts_with("open_external")))
        .collect()
}

/// 欄の値（文字列で渡した欄は生の値、`%` で渡した欄は表示の形）。
fn val<'a>(ev: &'a CapturedEvent, name: &str) -> Option<&'a str> {
    ev.field_str(name).or_else(|| ev.field(name))
}

/// 本物の振り分けへ通した下書き。
fn draft_of(ev: &CapturedEvent) -> Draft {
    draft(
        ev.level,
        &ev.target,
        ev.fields.iter().map(|(name, v)| FieldText {
            name,
            debug: &v.debug,
            raw: v.str_raw.as_deref(),
        }),
    )
    .unwrap_or_else(|| panic!("種別に当たらない: {ev:?}"))
}

/// 失敗の行がちょうど 1 行・`error` 種別・ゴースト名付き・理由が `reason` であることを判定する。
fn assert_one_failure(events: &[CapturedEvent], reason: &str) -> CapturedEvent {
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "error! はちょうど 1 行: {events:?}");
    let ev = errors[0];
    assert_eq!(ev.target, TARGET_ERROR);
    assert_eq!(val(ev, "event"), Some("open_external_failed"));
    assert_eq!(val(ev, "reason"), Some(reason));
    let d = draft_of(ev);
    assert_eq!((d.kind, d.name.as_str()), (Kind::Error, "G"));
    ev.clone()
}

#[test]
fn resolve_failures_log_one_error_and_never_call_os() {
    let f = Fixture::new();
    let cases: &[(&str, &[&str], &str)] = &[
        // 行き先が無い（要件 2.5・5.4・4.8）。
        ("open", &["file", "nope.exe\\x"], "not_found"),
        ("open", &["editor", "nope.txt"], "not_found"),
        ("open", &["explorer", "nope"], "not_found"),
        (JUMP_TAG_CARRIER, &["file:///nope.txt"], "not_found"),
        // 名前に当たらない（要件 4.8）。
        ("open", &["explorer", "shell", "無い"], "no_match"),
    ];
    for (name, args, reason) in cases {
        let (events, os) = run(f.job(name, args), FakeOs::default());
        let ev = assert_one_failure(&events, reason);
        assert_eq!(os.calls.len(), 0, "{name} {args:?}: OS を呼ばない");
        assert!(val(&ev, "kind").is_some());
        assert!(val(&ev, "tag").is_some());
        assert!(val(&ev, "destination").is_some());
        assert!(
            open_events(&events)
                .iter()
                .all(|e| e.level == tracing::Level::ERROR)
        );
    }

    // 根が無い（要件 4.8）。
    let mut job = f.job("open", &["explorer", "ghost", "G"]);
    job.context.baseware = None;
    let (events, os) = run(job, FakeOs::default());
    assert_one_failure(&events, "no_baseware_root");
    assert_eq!(os.calls.len(), 0);
}

#[test]
fn not_found_logs_the_written_spelling_kind_and_tag() {
    let f = Fixture::new();
    let (events, _) = run(f.job("open", &["editor", "nope.txt"]), FakeOs::default());
    let ev = assert_one_failure(&events, "not_found");
    assert_eq!(val(&ev, "kind"), Some("editor"));
    assert_eq!(val(&ev, "destination"), Some("nope.txt"));
    assert!(val(&ev, "tag").is_some_and(|t| t.contains("editor")));
}

#[test]
fn os_refusals_log_info_then_one_error_with_code() {
    let f = Fixture::new();
    for code in [2u32, 1155] {
        let (events, os) = run(
            f.job("open", &["browser", "https://example.com/"]),
            FakeOs::returning(&[Err(code)]),
        );
        assert_eq!(os.calls.len(), 1, "OS へは 1 度だけ渡す");
        let opened: Vec<_> = open_events(&events);
        assert_eq!(opened.len(), 2, "info の後に error!: {events:?}");
        assert_eq!(opened[0].level, tracing::Level::INFO);
        assert_eq!(val(opened[0], "event"), Some("open_external"));
        let ev = assert_one_failure(&events, "os");
        assert_eq!(ev.field("code"), Some(code.to_string().as_str()));
        assert_eq!(val(&ev, "destination"), Some("https://example.com/"));
        assert_eq!(val(&ev, "kind"), Some("url"));
    }
}

#[test]
fn success_logs_one_status_info_with_all_fields_and_no_error() {
    let f = Fixture::new();
    let (events, os) = run(f.job("open", &["editor", "readme.txt"]), FakeOs::default());
    assert_eq!(os.calls.len(), 1);
    assert!(events.iter().all(|e| e.level != tracing::Level::ERROR));
    let opened = open_events(&events);
    assert_eq!(opened.len(), 1, "{events:?}");
    let ev = opened[0];
    assert_eq!(ev.level, tracing::Level::INFO);
    assert_eq!(val(ev, "event"), Some("open_external"));
    assert_eq!(val(ev, "kind"), Some("editor"));
    let path = f
        .ctx
        .ghost_dir
        .join("ghost")
        .join("master")
        .join("readme.txt");
    assert_eq!(val(ev, "destination"), Some(path.to_str().unwrap()));
    assert_eq!(val(ev, "ghost"), Some("G"));
    assert!(val(ev, "tag").is_some_and(|t| t.contains("readme.txt")));
    assert_eq!(val(ev, "verb"), Some("edit"));
    assert_eq!(draft_of(ev).kind, Kind::Status);
}

#[test]
fn serve_handles_jobs_in_sent_order_until_the_sender_is_dropped() {
    let f = Fixture::new();
    let (tx, rx) = mpsc::channel();
    let urls = [
        "https://a.example/",
        "https://b.example/",
        "https://c.example/",
    ];
    for u in urls {
        tx.send(f.job("open", &["browser", u])).unwrap();
    }
    drop(tx);
    let mut os = FakeOs::default();
    serve(rx, &mut os);
    let files: Vec<_> = os.calls.iter().map(|c| c.file.clone()).collect();
    assert_eq!(files, urls.map(std::ffi::OsString::from));
}
