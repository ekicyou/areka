//! `error`・`outcome` の兄弟テスト。失敗の語彙が 11 で閉じ宣言順に並ぶこと、
//! 表示が診断に要る値（期待と実際）を落とさないこと、`rolled_back()`・`file()` が
//! 理由ごとに正しい値を返すことを判定する（要件 7.3・7.4）。
//!
//! 一周の固定入力から 11 変種すべてに到達できることの判定は `lib_tests.rs` が
//! `ALL_KINDS` と突き合わせて行う。

use super::*;
use crate::outcome::ManifestName;
use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;

fn target() -> PathBuf {
    PathBuf::from(r"C:\ghost\emo2")
}

fn io_err(msg: &str) -> io::Error {
    io::Error::other(msg.to_string())
}

/// 宣言順に 1 つずつ組んだ全変種の見本。
fn samples_in_declaration_order() -> Vec<FailReason> {
    vec![
        FailReason::TargetMissing {
            path: PathBuf::from(r"C:\ghost\emo2"),
        },
        FailReason::InvalidHomeurl {
            homeurl: "ftp://example.com/".to_string(),
        },
        FailReason::ManifestMissing {},
        FailReason::ManifestFetch {
            name: ManifestName::Updates2Dau,
            source: FetchError::Timeout,
        },
        FailReason::LocalUnreadable {
            path: target().join("ghost/master/a.txt"),
            source: io_err("読めない"),
        },
        FailReason::WorkArea {
            path: PathBuf::from(r"C:\ghost\emo2\.update-work\1-0"),
            source: io_err("作れない"),
        },
        FailReason::FileFetch {
            file: "ghost/master/b.txt".to_string(),
            source: FetchError::Connect,
        },
        FailReason::Md5Mismatch {
            file: "ghost/master/c.txt".to_string(),
            expected: "0123456789abcdef0123456789abcdef".to_string(),
            actual: "fedcba9876543210fedcba9876543210".to_string(),
        },
        FailReason::EscapesTarget {
            path: target().join("link/d.txt"),
        },
        FailReason::CommitWrite {
            path: target().join("shell/e.png"),
            source: io_err("使用中"),
        },
        FailReason::RollbackFailed {
            path: target().join("shell/f.png"),
            source: io_err("使用中"),
            restored: vec!["shell/g.png".to_string()],
            stuck: vec![Stuck {
                file: "shell/h.png".to_string(),
                source: io_err("拒否"),
            }],
        },
    ]
}

#[test]
fn all_kinds_has_eleven_entries() {
    assert_eq!(FailReason::ALL_KINDS.len(), 11, "失敗の語彙は 11 で閉じる");
}

#[test]
fn all_kinds_are_unique() {
    let unique: BTreeSet<&&str> = FailReason::ALL_KINDS.iter().collect();
    assert_eq!(unique.len(), FailReason::ALL_KINDS.len());
}

#[test]
fn all_kinds_matches_every_variant_in_declaration_order() {
    let observed: Vec<&'static str> = samples_in_declaration_order()
        .iter()
        .map(FailReason::kind)
        .collect();
    assert_eq!(observed, FailReason::ALL_KINDS);
    assert_eq!(
        FailReason::ALL_KINDS,
        [
            "TargetMissing",
            "InvalidHomeurl",
            "ManifestMissing",
            "ManifestFetch",
            "LocalUnreadable",
            "WorkArea",
            "FileFetch",
            "Md5Mismatch",
            "EscapesTarget",
            "CommitWrite",
            "RollbackFailed",
        ]
    );
}

#[test]
fn md5_mismatch_display_carries_file_expected_and_actual() {
    let shown = samples_in_declaration_order()[7].to_string();
    assert!(shown.contains("ghost/master/c.txt"), "{shown}");
    assert!(
        shown.contains("0123456789abcdef0123456789abcdef"),
        "期待値が落ちている: {shown}"
    );
    assert!(
        shown.contains("fedcba9876543210fedcba9876543210"),
        "実際の値が落ちている: {shown}"
    );
}

#[test]
fn fetch_failures_display_carry_the_file_and_the_cause() {
    let shown = FailReason::FileFetch {
        file: "ghost/master/b.txt".to_string(),
        source: FetchError::Status { code: 503 },
    }
    .to_string();
    assert!(
        shown.contains("ghost/master/b.txt") && shown.contains("503"),
        "{shown}"
    );

    let shown = FailReason::ManifestFetch {
        name: ManifestName::UpdatesTxt,
        source: FetchError::TooLarge { limit: 1024 },
    }
    .to_string();
    assert!(
        shown.contains("updates.txt") && shown.contains("1024"),
        "{shown}"
    );

    let shown = FetchError::Other { code: 12345 }.to_string();
    assert!(shown.contains("12345"), "{shown}");
}

#[test]
fn every_display_is_non_empty() {
    for reason in samples_in_declaration_order() {
        assert!(!reason.to_string().is_empty(), "{}", reason.kind());
    }
}

fn error_with(reason: FailReason) -> UpdateError {
    UpdateError {
        homeurl: "http://example.com/emo2/".to_string(),
        target: target(),
        stage: Stage::Commit,
        reason,
        leftovers: Vec::new(),
        work: None,
    }
}

#[test]
fn rolled_back_is_false_only_for_rollback_failed() {
    let observed: Vec<(&str, bool)> = samples_in_declaration_order()
        .into_iter()
        .map(|r| {
            let kind = r.kind();
            (kind, error_with(r).rolled_back())
        })
        .collect();
    for (kind, rolled_back) in observed {
        assert_eq!(rolled_back, kind != "RollbackFailed", "{kind}");
    }
}

#[test]
fn file_names_the_cause_where_known() {
    let observed: Vec<(&str, Option<String>)> = samples_in_declaration_order()
        .into_iter()
        .map(|r| {
            let kind = r.kind();
            (kind, error_with(r).file().map(str::to_string))
        })
        .collect();
    let expected: Vec<(&str, Option<String>)> = vec![
        ("TargetMissing", None),
        ("InvalidHomeurl", None),
        ("ManifestMissing", None),
        ("ManifestFetch", Some("updates2.dau".into())),
        ("LocalUnreadable", Some("ghost/master/a.txt".into())),
        ("WorkArea", None),
        ("FileFetch", Some("ghost/master/b.txt".into())),
        ("Md5Mismatch", Some("ghost/master/c.txt".into())),
        ("EscapesTarget", Some("link/d.txt".into())),
        ("CommitWrite", Some("shell/e.png".into())),
        ("RollbackFailed", Some("shell/f.png".into())),
    ];
    assert_eq!(observed, expected);
}

/// パスを持つ理由は対象フォルダからの相対名だけを返し、利用者の絶対パスを出さない。
#[test]
fn file_is_relative_to_target_or_none() {
    let at_target = error_with(FailReason::LocalUnreadable {
        path: target(),
        source: io_err("読めない"),
    });
    assert_eq!(at_target.file(), None, "対象フォルダそのものは None");

    let outside = error_with(FailReason::EscapesTarget {
        path: PathBuf::from(r"C:\elsewhere\x.txt"),
    });
    assert_eq!(outside.file(), None, "配下でなければ None");

    let under = error_with(FailReason::CommitWrite {
        path: target().join("ghost/master/a.txt"),
        source: io_err("使用中"),
    });
    assert_eq!(under.file(), Some("ghost/master/a.txt"));
}

#[test]
fn update_error_display_carries_url_target_stage_and_reason() {
    let mut err = error_with(FailReason::ManifestMissing {});
    err.stage = Stage::Download { index: 1, total: 3 };
    let shown = err.to_string();
    assert!(shown.contains("http://example.com/emo2/"), "{shown}");
    assert!(shown.contains(r"C:\ghost\emo2"), "{shown}");
    assert!(shown.contains("Download") && shown.contains('3'), "{shown}");
    assert!(
        shown.contains(&FailReason::ManifestMissing {}.to_string()),
        "{shown}"
    );
}

#[test]
fn manifest_name_file_names() {
    assert_eq!(ManifestName::Updates2Dau.file_name(), "updates2.dau");
    assert_eq!(ManifestName::UpdatesTxt.file_name(), "updates.txt");
}
