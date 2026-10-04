//! シェルの読み込みが箱の表を作り、報告を 1 度だけ記録する檻（spec: areka-P0-shell-balloon・
//! 要件 10.1・10.2・10.4・10.5）。
//!
//! 見ているのは 3 つ——⑴ 検体（`areka-emo-text/tests/fixtures/shell-balloon/`）を読み込むと
//! [`ShellTarget::boxes`] から箱の表が引けること ⑵ 誤りのある文面でも読み込みが成功し、
//! 報告の各件が `warn!` で 1 度ずつ記録されること ⑶ 箱の無いシェルでは結果と記録が本 spec の
//! 前と同じであること。
//!
//! 検体の element定義の overlay が指す画像はフォルダに無い。焼く段の脱落の `warn!` が数を
//! 濁さないよう、復号器はメモリ上のものにその名前で絵を入れておく（復号器はパスで引くだけで、
//! ファイルの実在を見ない）。

use super::*;

use std::path::PathBuf;

use areka_emo_atlas::MemoryDecoder;
use areka_emo_compose::FontFollow;
use areka_parsers::shell::{parse, parse_boxes};
use temp_path_kit::TempPath;

use super::test_support::{CapturedEvent, capture_events};

/// 本モジュールが出す記録の宛先（既定の target＝モジュールパス）。
const SHELL_TARGET: &str = "areka_emo_present::shell_target";

/// 不透明 1×1 PBGRA の絵を `names` の名前で入れた復号器。
fn decoder_with(dir: &Path, names: &[&str]) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    for name in names {
        dec.insert(dir.join(name), 1, 1, 4, vec![10u8, 20, 30, 255], true);
    }
    dec
}

/// 宛先が権威のもので、水準が `level` の記録。
fn from_shell_target(events: &[CapturedEvent], level: tracing::Level) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == SHELL_TARGET && e.level == level)
        .collect()
}

/// 箱の検体のフォルダ（`areka-emo-text` の検体を読む・タスク 1）。
fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("areka-emo-text")
        .join("tests")
        .join("fixtures")
        .join("shell-balloon")
}

/// ⑴ 検体を読み込むと箱の表が引ける（要件 10.4）。記録は `warn!` 0 行・箱の数の `info!` 1 行。
#[test]
fn fixture_shell_loads_with_a_box_layout() {
    let dir = fixture_dir();
    let dec = decoder_with(&dir, &["body0.png", "body1.png", "body2.png", "body3.png"]);

    let (target, events) =
        capture_events(|| load_shell_target(&dir, &dec).expect("検体のシェルは読める"));
    let boxes = target.boxes();

    let summary = |id: u32| -> Vec<(u32, String, i64, i64)> {
        boxes
            .placements(id)
            .iter()
            .map(|p| (p.element, p.name.as_str().to_string(), p.x, p.y))
            .collect()
    };
    assert_eq!(
        summary(0),
        vec![(1, "tate".into(), 10, 20), (2, "yoko".into(), 150, 300)]
    );
    assert_eq!(summary(1), vec![(1, "tate".into(), 240, 40)]);
    assert_eq!(summary(2), vec![], "箱の無いサーフェス");
    assert_eq!(
        summary(3),
        vec![(1, "fuda".into(), 20, 400)],
        "surface.append*ブレスで足した箱"
    );

    let def = |id: u32, i: usize| {
        boxes
            .def(&boxes.placements(id)[i].name)
            .expect("定義が引ける")
    };
    assert_eq!(def(0, 0).size, (120, 240));
    assert_eq!(def(0, 1).size, (200, 80));
    assert_eq!(def(3, 0).follow, FontFollow::Balloon);
    assert!(
        target.box_report.issues.is_empty(),
        "正しく書いた検体は報告 0 件"
    );

    assert!(
        from_shell_target(&events, tracing::Level::WARN).is_empty(),
        "検体の読み込みは警告 0 行: {events:?}"
    );
    let infos: Vec<_> = from_shell_target(&events, tracing::Level::INFO)
        .into_iter()
        .filter(|e| e.message().contains("箱"))
        .collect();
    assert_eq!(infos.len(), 1, "箱の数の記録は 1 行: {infos:?}");
    assert_eq!(infos[0].field("braces"), Some("3"));
    assert_eq!(infos[0].field("surfaces"), Some("3"));
}

/// ⑵ 誤りのある定義を混ぜても読み込みは成功し、報告の各件が 1 度ずつ記録される（要件 10.1・10.2）。
#[test]
fn erroneous_boxes_still_load_and_each_issue_is_logged_once() {
    let dir = TempPath::new("shell-target-boxes-erroneous");
    std::fs::write(
        dir.child("surfaces.txt"),
        concat!(
            "charset,UTF-8\n",
            "balloon.ok\n{\nsize,10,10\n}\n",
            "balloon.nosize\n{\nfont.height,12\n}\n",
            "balloon.42\n{\nsize,10,10\n}\n",
            "balloon.odd\n{\nsize,10,10\nuse_self_alpha,1\n}\n",
            "surface0\n{\nelement0,overlay,base.png,0,0\n",
            "element1,balloon,ok,1,2\n",
            "element2,balloon,ghost,0,0\n",
            "element3,balloon,odd,x,5\n}\n",
            "surface.append9\n{\nelement1,balloon,ok,0,0\n}\n",
        ),
    )
    .expect("記述ファイル作成");
    let dec = decoder_with(dir.path(), &["base.png"]);

    let (target, events) = capture_events(|| {
        load_shell_target(dir.path(), &dec).expect("箱の誤りで読み込みは失敗しない")
    });

    // 誤りの無い定義で表示できる（要件 10.2）。
    let placed: Vec<&str> = target
        .boxes()
        .placements(0)
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(placed, vec!["ok"]);
    assert!(target.bake_errors().is_empty(), "前提: 絵は全部焼ける");

    // 前提: 報告は 6 件（size 無し・整数の名前・当てはまらないキー・名前のブレスが無い・
    // X が整数でない・追記先が無い）。
    let issues = &target.box_report.issues;
    assert_eq!(issues.len(), 6, "前提の報告: {issues:?}");

    let warns = from_shell_target(&events, tracing::Level::WARN);
    assert_eq!(
        warns.len(),
        issues.len(),
        "報告 1 件につき warn! 1 行: {warns:?}"
    );
    for warn in &warns {
        assert!(
            warn.message().starts_with("shell:"),
            "接頭辞は shell: {warn:?}"
        );
    }
    // 欄は報告の対象の欄そのまま（代表の 3 件）。
    let one = |needle: &str| -> &CapturedEvent {
        let hits: Vec<_> = warns
            .iter()
            .filter(|e| e.message().contains(needle))
            .collect();
        assert_eq!(hits.len(), 1, "`{needle}` の行は 1 行: {warns:?}");
        hits[0]
    };
    let missing = one("size");
    assert_eq!(missing.field_str("name"), Some("nosize"));
    let position = one("X・Y");
    assert_eq!(position.field("surface"), Some("0"));
    assert_eq!(position.field("element"), Some("3"));
    assert_eq!(position.field_str("name"), Some("odd"));
    assert_eq!(position.field_str("x"), Some("x"));
    assert_eq!(position.field_str("y"), Some("5"));
    let append = one("追記先");
    assert_eq!(append.field("surface"), Some("9"));
    assert_eq!(append.field_str("name"), Some("ok"));
}

/// ⑶ 箱の無いシェル: 結果と記録が本 spec の前と同じ（要件 10.5）。
#[test]
fn shell_without_boxes_keeps_result_and_records() {
    let text = "charset,UTF-8\nsurface0\n{\nelement0,overlay,base.png,0,0\n}\n";
    let dir = TempPath::new("shell-target-boxes-none");
    std::fs::write(dir.child("surfaces.txt"), text).expect("記述ファイル作成");
    let dec = decoder_with(dir.path(), &["base.png"]);

    let (target, events) =
        capture_events(|| load_shell_target(dir.path(), &dec).expect("シェルは読める"));

    assert!(target.boxes().is_empty());
    assert!(target.box_report.issues.is_empty());
    let messages: Vec<&str> = events
        .iter()
        .filter(|e| e.target == SHELL_TARGET)
        .map(CapturedEvent::message)
        .collect();
    assert_eq!(
        messages,
        vec!["shell: シェルの面の画像の一覧が終わった（R6.1）"],
        "記録は本 spec の前と同じ 1 行だけ"
    );

    // 既存の入口（空の転記へ委譲）と箱の入口が同じ結果になる。
    let core = build_shell_target(
        parse(text),
        select_surface_images::<&str>(&[]),
        dir.path(),
        &dec,
    );
    let with = build_shell_target_with_boxes(
        parse(text),
        &parse_boxes(text),
        select_surface_images::<&str>(&[]),
        dir.path(),
        &dec,
    );
    assert!(core.boxes().is_empty() && with.boxes().is_empty());
    assert_eq!(core.box_report, with.box_report);
    assert_eq!(
        core.build_world().surface_ids().collect::<Vec<_>>(),
        with.build_world().surface_ids().collect::<Vec<_>>()
    );
    assert_eq!(
        core.atlas().resolve(areka_emo_atlas::SetId(0), "base.png"),
        with.atlas().resolve(areka_emo_atlas::SetId(0), "base.png")
    );
}
