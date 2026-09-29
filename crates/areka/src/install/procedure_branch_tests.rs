//! 手続きの分かれ道のテスト（design「Testing Strategy / 手続き」「Error Categories and Responses」・
//! 要件 2.8・2.9・3.2・3.6・3.7・4.5〜4.8・4.11・5.1・5.4・5.6・5.7・9.2・11.3・11.5・12.2・12.12）。
//!
//! 宛先違い・利用条件・失敗・複数の書庫・途中でやめる の各場合で、送られるイベント・書庫の
//! 終わり方・記録・宛先に書かれたものを判定する。偽の口の上で本物の `areka-nar` を走らせる。

use std::path::{Path, PathBuf};

use areka_nar::SurvivingTree;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{Damage, NarBuilder, install_txt};
use temp_path_kit::TempPath;

use super::procedure_test_support::{
    Call, FakePorts, OverwriteScript, RUNNING, RUNNING_ALIAS, balloon_nar, ghost_nar, order_of,
    shell_nar, supplement_nar, write_nar,
};
use super::*;

/// 締めの知らせになりうるイベント（`OnGhostTermsAccept` と `OnInstallBegin` 以外）。
const CLOSING: [&str; 5] = [
    "OnInstallCompleteEx",
    "OnInstallComplete",
    "OnInstallRefuse",
    "OnGhostTermsDecline",
    "OnInstallFailure",
];

/// 書庫を並べた依頼を偽の口で一周させ、記録ごと返す。
fn run_many(
    ports: &mut FakePorts,
    builders: Vec<NarBuilder>,
) -> (Vec<ArchiveEnd>, Vec<CapturedEvent>) {
    let dir = TempPath::new("install-branch-nar");
    let paths: Vec<PathBuf> = builders
        .into_iter()
        .enumerate()
        .map(|(index, builder)| write_nar(&dir, &format!("archive{index}.nar"), builder))
        .collect();
    let order = order_of(&paths.iter().map(PathBuf::as_path).collect::<Vec<_>>());
    capture(|| run_order(&order, ports))
}

fn run_one(ports: &mut FakePorts, builder: NarBuilder) -> (Vec<ArchiveEnd>, Vec<CapturedEvent>) {
    run_many(ports, vec![builder])
}

/// `event` 欄が `name` の記録。
fn logged<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|event| event.field_str("event") == Some(name))
        .collect()
}

fn errors(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|event| event.level == tracing::Level::ERROR)
        .count()
}

fn asks(ports: &FakePorts) -> usize {
    ports
        .calls
        .iter()
        .filter(|call| matches!(call, Call::Ask(_)))
        .count()
}

/// 展開の口（よそ・起動中のゴースト）が呼ばれた回数。
fn installs(ports: &FakePorts) -> usize {
    ports
        .calls
        .iter()
        .filter(|call| matches!(call, Call::Elsewhere(_) | Call::Overwrite(_)))
        .count()
}

/// 根の下に在るものを全部（フォルダも）。
fn entries_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("根の下を読める") {
            let path = entry.expect("項目を読める").path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            found.push(path);
        }
    }
    found.sort();
    found
}

/// `install.txt` の行だけを選べる書庫（中身は 1 ファイル）。
fn nar_with(lines: &[&str]) -> NarBuilder {
    NarBuilder::new()
        .file("install.txt", &install_txt(lines))
        .done()
        .file("descript.txt", b"charset,UTF-8\r\nname,X\r\n")
        .done()
}

fn with_terms(builder: NarBuilder) -> NarBuilder {
    builder
        .file("terms.txt", b"charset,UTF-8\r\nterms body\r\n")
        .done()
}

// ---- 宛先違い（要件 3.2・3.6・3.7）----

#[test]
fn refused_accept_sends_only_refuse_writes_nothing_and_logs_no_error() {
    // 大文字小文字だけが違う `Kinoko` も一致しない。
    let cases = [
        ("よそのこ", "shell", "きがえ"),
        ("よそのこ", "supplement", "ついか"),
        ("Kinoko", "shell", "きがえ"),
    ];
    for (accept, kind, name) in cases {
        let builder = if kind == "shell" {
            shell_nar(accept)
        } else {
            supplement_nar(accept)
        };
        let mut ports = FakePorts::new();
        let before = entries_under(ports.root.path());
        let (ends, events) = run_one(&mut ports, with_terms(builder));

        assert_eq!(ends, vec![ArchiveEnd::Refused], "{kind}");
        assert_eq!(
            ports.raised(),
            vec![
                ("OnInstallBegin", Vec::new()),
                (
                    "OnInstallRefuse",
                    vec![accept.to_owned(), kind.to_owned(), name.to_owned()]
                ),
            ],
            "Reference0＝accept の値・1＝識別子・2＝name（3.2）"
        );
        assert_eq!(installs(&ports), 0, "展開の口を呼ばない");
        assert_eq!(asks(&ports), 0, "照合の前に利用条件を出さない");
        assert_eq!(
            entries_under(ports.root.path()),
            before,
            "宛先に 1 バイトも無い"
        );
        assert_eq!(errors(&events), 0, "宛先違いは失敗ではない（3.7）");
        let refused = logged(&events, "install_refused");
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].level, tracing::Level::WARN);
        assert_eq!(refused[0].field("accept"), Some(accept));
        assert!(logged(&events, "install_failed").is_empty());
    }
}

#[test]
fn shell_or_supplement_without_accept_fails_as_invalid_type() {
    for kind in ["shell", "supplement"] {
        let type_line = format!("type,{kind}");
        let mut ports = FakePorts::new();
        let before = entries_under(ports.root.path());
        let (ends, events) = run_one(
            &mut ports,
            nar_with(&["charset,UTF-8", &type_line, "name,なし", "directory,none"]),
        );

        assert_eq!(ends, vec![ArchiveEnd::Failed(FailureWord::InvalidType)]);
        assert_eq!(
            ports.raised(),
            vec![
                ("OnInstallBegin", Vec::new()),
                ("OnInstallFailure", vec!["invalid type".to_owned()]),
            ],
            "{kind}: accept の無い {kind} は invalid type（3.6）"
        );
        assert_eq!(installs(&ports), 0);
        assert_eq!(entries_under(ports.root.path()), before);
        let failed = logged(&events, "install_failed");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].level, tracing::Level::ERROR);
        assert_eq!(failed[0].field_str("kind"), Some("AcceptMissing"));
        assert_eq!(failed[0].field_str("phase"), Some("accept"));
        assert!(logged(&events, "install_refused").is_empty());
    }
}

// ---- 利用条件（要件 4.5〜4.8・4.11・11.3）----

#[test]
fn no_terms_file_asks_nothing_and_sends_no_terms_event() {
    let mut ports = FakePorts::new();
    let (ends, _) = run_one(&mut ports, ghost_nar("newbie", &[]));
    assert!(matches!(ends.as_slice(), [ArchiveEnd::Installed(_)]));
    assert_eq!(asks(&ports), 0, "画面を出さない（4.11）");
    let ids = ports.raised_ids();
    assert!(
        !ids.contains(&"OnGhostTermsAccept") && !ids.contains(&"OnGhostTermsDecline"),
        "{ids:?}"
    );
}

#[test]
fn accepted_terms_send_accept_without_references_and_install() {
    let mut ports = FakePorts::new();
    let (ends, events) = run_one(&mut ports, with_terms(ghost_nar("newbie", &[])));
    assert!(matches!(ends.as_slice(), [ArchiveEnd::Installed(_)]));
    assert_eq!(asks(&ports), 1);
    let raised = ports.raised();
    assert_eq!(raised[1], ("OnGhostTermsAccept", Vec::new()), "4.5");
    assert!(!ports.raised_ids().contains(&"OnGhostTermsDecline"));
    assert!(
        ports
            .under_root("ghost/newbie/ghost/master/descript.txt")
            .is_file()
    );
    let terms = logged(&events, "install_terms");
    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].field_str("answer"), Some("accept"));
}

#[test]
fn declined_suppressed_or_unavailable_terms_send_only_decline_and_write_nothing() {
    let cases = [
        (YesNo::No, "decline", tracing::Level::INFO),
        (YesNo::Suppressed, "suppressed", tracing::Level::WARN),
        (YesNo::Unavailable, "unavailable", tracing::Level::ERROR),
    ];
    for (answer, word, level) in cases {
        for builder in [ghost_nar("newbie", &[]), balloon_nar()] {
            let mut ports = FakePorts::new();
            ports.terms_answer = answer;
            let before = entries_under(ports.root.path());
            let (ends, events) = run_one(&mut ports, with_terms(builder));

            assert_eq!(ends, vec![ArchiveEnd::Declined], "{answer:?}");
            assert_eq!(
                ports.raised(),
                vec![
                    ("OnInstallBegin", Vec::new()),
                    ("OnGhostTermsDecline", Vec::new()),
                ],
                "{answer:?}: Decline だけ・OnInstallFailure は 0 件（4.6・4.7・4.8）"
            );
            assert_eq!(asks(&ports), 1);
            assert_eq!(installs(&ports), 0, "{answer:?}: 展開の口を呼ばない");
            assert_eq!(
                entries_under(ports.root.path()),
                before,
                "{answer:?}: 宛先に 1 バイトも無い"
            );
            let terms = logged(&events, "install_terms");
            assert_eq!(terms.len(), 1, "{answer:?}");
            assert_eq!(terms[0].field_str("answer"), Some(word));
            assert_eq!(terms[0].level, level, "{answer:?}");
            assert_eq!(
                errors(&events),
                usize::from(answer == YesNo::Unavailable),
                "{answer:?}: error! は出せなかったときの 1 件だけ"
            );
            assert!(logged(&events, "install_failed").is_empty());
        }
    }
}

// ---- 失敗（要件 5.1・5.4・5.6・12.2）----

#[test]
fn read_failures_send_failure_with_the_single_table_word_and_ask_nothing() {
    let dir = TempPath::new("install-branch-missing");
    // (書庫・語・段)。無いファイルは I/O の読み取りの段、他は検査（`open`）で撥ねる。
    let cases: [(Option<NarBuilder>, FailureWord, &str); 4] = [
        (
            Some(with_terms(ghost_nar("newbie", &[])).damage(Damage::NoEocd)),
            FailureWord::Extraction,
            "open",
        ),
        // 無いファイル。
        (None, FailureWord::Extraction, "read"),
        (
            Some(with_terms(nar_with(&[
                "charset,UTF-8",
                "type,skin",
                "name,きせかえ",
                "directory,skin",
            ]))),
            FailureWord::Unsupported,
            "open",
        ),
        (
            Some(with_terms(nar_with(&[
                "charset,UTF-8",
                "name,かたなし",
                "directory,none",
            ]))),
            FailureWord::InvalidType,
            "open",
        ),
    ];
    for (builder, word, phase) in cases {
        let mut ports = FakePorts::new();
        let before = entries_under(ports.root.path());
        let (ends, events) = match builder {
            Some(builder) => run_one(&mut ports, builder),
            None => {
                let order = order_of(&[&dir.child("missing.nar")]);
                capture(|| run_order(&order, &mut ports))
            }
        };

        assert_eq!(ends, vec![ArchiveEnd::Failed(word)]);
        assert_eq!(
            ports.raised(),
            vec![
                ("OnInstallBegin", Vec::new()),
                ("OnInstallFailure", vec![word.as_ref_str().to_owned()]),
            ],
            "{word:?}: Reference0 は表の語 1 つだけ・1 以降は 0 個（5.1）"
        );
        assert_eq!(asks(&ports), 0, "{word:?}: 画面を出さない（5.4）");
        assert_eq!(installs(&ports), 0);
        assert_eq!(entries_under(ports.root.path()), before);
        let failed = logged(&events, "install_failed");
        assert_eq!(failed.len(), 1, "{word:?}");
        assert_eq!(failed[0].field_str("phase"), Some(phase), "{word:?}");
        assert_eq!(failed[0].field("rolled_back"), Some("true"));
        assert_eq!(failed[0].field_str("word"), Some(word.as_ref_str()));
        assert!(logged(&events, "install_survivor").is_empty());
    }
}

#[test]
fn install_failure_sends_extraction_and_logs_each_survivor_with_seven_days() {
    for survivors in [0, 2] {
        let mut ports = FakePorts::new();
        let trees: Vec<SurvivingTree> = (0..survivors)
            .map(|k| SurvivingTree {
                destination: ports.under_root(&format!("ghost/dest{k}")),
                path: ports.under_root(&format!("work/old-{k}")),
            })
            .collect();
        ports.elsewhere_error = Some(trees.clone());
        let (ends, events) = run_one(&mut ports, ghost_nar("newbie", &[]));

        assert_eq!(ends, vec![ArchiveEnd::Failed(FailureWord::Extraction)]);
        assert_eq!(
            ports.raised(),
            vec![
                ("OnInstallBegin", Vec::new()),
                ("OnInstallFailure", vec!["extraction".to_owned()]),
            ]
        );
        assert_eq!(asks(&ports), 0, "画面を出さない（5.4）");
        assert!(
            ports
                .calls
                .iter()
                .all(|call| !matches!(call, Call::Record(_))),
            "入らなかった物は覚えない"
        );
        let failed = logged(&events, "install_failed");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].field_str("kind"), Some("Io"));
        assert_eq!(failed[0].field_str("phase"), Some("commit"));
        assert_eq!(
            failed[0].field("rolled_back"),
            Some(if survivors == 0 { "true" } else { "false" })
        );
        let logged_survivors = logged(&events, "install_survivor");
        assert_eq!(logged_survivors.len(), survivors, "宛先ごとに 1 件（5.6）");
        for (event, tree) in logged_survivors.iter().zip(&trees) {
            assert_eq!(event.level, tracing::Level::ERROR);
            assert_eq!(
                event.field("destination"),
                Some(tree.destination.display().to_string().as_str())
            );
            assert_eq!(
                event.field("kept_at"),
                Some(tree.path.display().to_string().as_str())
            );
            assert!(
                event.message().contains("7 日で消えます"),
                "{}",
                event.message()
            );
        }
    }
}

// ---- 複数の書庫（要件 2.8・2.9・9.2・11.5・12.12）----

#[test]
fn two_archives_all_installed_end_with_one_complete_all_listing_both() {
    let mut ports = FakePorts::new();
    let (ends, _) = run_many(&mut ports, vec![ghost_nar("newbie", &[]), balloon_nar()]);

    assert!(
        matches!(
            ends.as_slice(),
            [ArchiveEnd::Installed(_), ArchiveEnd::Installed(_)]
        ),
        "{ends:?}"
    );
    let ghost_at = ports.under_root("ghost/newbie").display().to_string();
    let balloon_at = ports.under_root("balloon/round").display().to_string();
    let raised = ports.raised();
    assert_eq!(
        raised.last(),
        Some(&(
            "OnInstallCompleteAll",
            vec![
                ["ghost", "balloon"].join("\u{1}"),
                ["あたらしい", "まるい"].join("\u{1}"),
                [ghost_at.as_str(), balloon_at.as_str()].join("\u{1}"),
            ]
        )),
        "最後に 1 回・2 本ぶんを byte 値 1 区切りで（2.8）"
    );
    assert_eq!(
        ports
            .raised_ids()
            .iter()
            .filter(|id| **id == "OnInstallCompleteAll")
            .count(),
        1
    );
    assert_eq!(
        ports
            .raised_ids()
            .iter()
            .filter(|id| **id == "OnInstallBegin")
            .count(),
        2,
        "並んだ順に 1 本ずつ（9.2）"
    );
}

#[test]
fn complete_all_is_not_sent_unless_two_or_more_all_installed() {
    let cases: [(&str, Vec<NarBuilder>, usize); 5] = [
        ("1 本だけ", vec![ghost_nar("newbie", &[])], 1),
        (
            "2 本目が失敗",
            vec![
                ghost_nar("newbie", &[]),
                balloon_nar().damage(Damage::NoEocd),
            ],
            2,
        ),
        (
            "1 本目が失敗",
            vec![
                balloon_nar().damage(Damage::NoEocd),
                ghost_nar("newbie", &[]),
            ],
            2,
        ),
        (
            "1 本目が宛先違い",
            vec![shell_nar("よそのこ"), ghost_nar("newbie", &[])],
            2,
        ),
        (
            "2 本目が拒否",
            vec![ghost_nar("newbie", &[]), with_terms(balloon_nar())],
            2,
        ),
    ];
    for (label, builders, count) in cases {
        let mut ports = FakePorts::new();
        ports.terms_answer = YesNo::No;
        let (ends, _) = run_many(&mut ports, builders);
        assert_eq!(ends.len(), count, "{label}: 失敗しても次の書庫へ進む");
        assert!(
            !ports.raised_ids().contains(&"OnInstallCompleteAll"),
            "{label}: {:?}（2.9）",
            ports.raised_ids()
        );
    }
}

// ---- 途中でやめる（要件 2.3・8.6 の 0 件）----

#[test]
fn install_port_closing_sends_no_closing_notice_and_abandons() {
    // よその宛先・起動中のゴーストの宛先のどちらの口が閉じても同じ。
    let cases: [(&str, NarBuilder); 2] = [
        ("elsewhere", ghost_nar("newbie", &[])),
        ("overwrite", supplement_nar(RUNNING_ALIAS)),
    ];
    for (port, builder) in cases {
        let mut ports = FakePorts::new();
        ports.elsewhere_closed = true;
        ports.overwrite = OverwriteScript::Closed;
        let (ends, events) = run_many(&mut ports, vec![builder, balloon_nar()]);

        assert_eq!(
            ends,
            vec![ArchiveEnd::Abandoned],
            "{port}: 以後の書庫は扱わない"
        );
        assert_eq!(
            ports.raised_ids(),
            vec!["OnInstallBegin"],
            "{port}: 締めの知らせは 0 件"
        );
        assert!(!ports.under_root("ghost/newbie").exists());
        assert!(!ports.under_root("balloon/round").exists());
        assert_eq!(logged(&events, "install_abandoned").len(), 1);
        assert_eq!(errors(&events), 0, "{port}: 終了で止めるのは失敗ではない");
    }
    // 起動中のゴーストの口が「もう起動中でない」と返した後、よその口が閉じた場合。
    let mut ports = FakePorts::new();
    ports.overwrite = OverwriteScript::NotRunning;
    ports.elsewhere_closed = true;
    let (ends, _) = run_one(&mut ports, supplement_nar(RUNNING_ALIAS));
    assert_eq!(ends, vec![ArchiveEnd::Abandoned]);
    assert_eq!(ports.raised_ids(), vec!["OnInstallBegin"]);
    assert!(
        !ports
            .under_root(&format!("ghost/{RUNNING}/ghost/master/extra.dic"))
            .exists()
    );
}

#[test]
fn raise_closing_at_any_event_stops_every_later_event() {
    // (閉じるイベント, 書庫, 利用条件の答え)
    let cases: [(&'static str, NarBuilder, YesNo); 6] = [
        ("OnInstallBegin", ghost_nar("newbie", &[]), YesNo::Yes),
        (
            "OnGhostTermsAccept",
            with_terms(ghost_nar("newbie", &[])),
            YesNo::Yes,
        ),
        (
            "OnGhostTermsDecline",
            with_terms(ghost_nar("newbie", &[])),
            YesNo::No,
        ),
        ("OnInstallRefuse", shell_nar("よそのこ"), YesNo::Yes),
        (
            "OnInstallFailure",
            ghost_nar("newbie", &[]).damage(Damage::NoEocd),
            YesNo::Yes,
        ),
        ("OnInstallCompleteEx", ghost_nar("newbie", &[]), YesNo::Yes),
    ];
    for (closing, builder, answer) in cases {
        let mut ports = FakePorts::new();
        ports.terms_answer = answer;
        ports.replies.insert(closing, Raised::Closed);
        let (ends, _) = run_many(&mut ports, vec![builder, balloon_nar()]);

        assert_eq!(ends, vec![ArchiveEnd::Abandoned], "{closing}");
        let ids = ports.raised_ids();
        assert_eq!(
            ids.last(),
            Some(&closing),
            "{closing}: 閉じた後は 0 件 {ids:?}"
        );
        assert_eq!(
            ids.iter().filter(|id| **id == "OnInstallBegin").count(),
            1,
            "{closing}: 2 本目は始めない"
        );
        assert!(
            ids.iter().filter(|id| CLOSING.contains(id)).count() <= 1,
            "{closing}: {ids:?}"
        );
        if closing == "OnGhostTermsAccept" {
            assert_eq!(installs(&ports), 0, "受諾を送れなければ展開しない");
        }
    }
}

#[test]
fn missing_ghost_facts_abandons_before_anything_is_written() {
    let mut ports = FakePorts::new();
    ports.facts = None;
    let before = entries_under(ports.root.path());
    let (ends, _) = run_one(&mut ports, with_terms(ghost_nar("newbie", &[])));
    assert_eq!(ends, vec![ArchiveEnd::Abandoned]);
    assert_eq!(ports.raised_ids(), vec!["OnInstallBegin"]);
    assert_eq!(asks(&ports), 0);
    assert_eq!(entries_under(ports.root.path()), before);
}
