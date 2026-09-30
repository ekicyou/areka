//! 写しの兄弟テスト（design「Testing Strategy / 写し」・要件 9.1〜9.3・9.8）。

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use areka_update::{FailReason, FetchError, ManifestName, Progress, Stuck, UpdateOutcome};

use super::*;

const SEP: &str = "\u{1}";

/// 送る 19 語の全部（定数の綴りの判定用）。
const ALL_19: [&str; 19] = [
    ON_UPDATE_PROCESS_EXEC,
    ON_UPDATE_BEGIN,
    ON_UPDATE_READY,
    ON_UPDATE_DOWNLOAD_BEGIN,
    ON_UPDATE_MD5_BEGIN,
    ON_UPDATE_MD5_COMPLETE,
    ON_UPDATE_MD5_FAILURE,
    ON_UPDATE_COMPLETE,
    ON_UPDATE_FAILURE,
    ON_UPDATE_OTHER_BEGIN,
    ON_UPDATE_OTHER_READY,
    ON_UPDATE_OTHER_DOWNLOAD_BEGIN,
    ON_UPDATE_OTHER_MD5_BEGIN,
    ON_UPDATE_OTHER_MD5_COMPLETE,
    ON_UPDATE_OTHER_MD5_FAILURE,
    ON_UPDATE_OTHER_COMPLETE,
    ON_UPDATE_OTHER_FAILURE,
    ON_UPDATE_RESULT,
    ON_UPDATE_RESULT_EX,
];

/// 正典に在るが送らない 7 語の失敗理由（要件 4.3）。
const UNSENT_WORDS: [&str; 7] = [
    "too slow",
    "artificial",
    "readonly",
    "virusdetect",
    "toomanyredirect",
    "parse",
    "downloading",
];

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_owned()).collect()
}

fn io_err() -> io::Error {
    io::Error::other("x")
}

#[test]
fn nineteen_names_are_distinct_allowed_by_kanade_and_exclude_unsent_events() {
    let set: BTreeSet<&str> = ALL_19.into_iter().collect();
    assert_eq!(set.len(), 19);
    for id in ALL_19 {
        assert!(areka_kanade::events::is_allowed_event_id(id), "{id}");
    }
    for unsent in [
        "OnUpdateCheckComplete",
        "OnUpdateCheckFailure",
        "OnUpdateCheckResult",
        "OnUpdateCheckResultEx",
        "OnUpdateResultExplorer",
    ] {
        assert!(!set.contains(unsent), "{unsent}");
    }
}

#[test]
fn ghost_uses_on_update_and_shell_balloon_use_on_update_other() {
    let ghost = EventNames::for_kind(TargetKind::Ghost);
    assert_eq!(
        [
            ghost.begin,
            ghost.ready,
            ghost.download_begin,
            ghost.md5_begin,
            ghost.md5_complete,
            ghost.md5_failure,
            ghost.complete,
            ghost.failure,
        ],
        [
            "OnUpdateBegin",
            "OnUpdateReady",
            "OnUpdate.OnDownloadBegin",
            "OnUpdate.OnMD5CompareBegin",
            "OnUpdate.OnMD5CompareComplete",
            "OnUpdate.OnMD5CompareFailure",
            "OnUpdateComplete",
            "OnUpdateFailure",
        ]
    );
    for kind in [TargetKind::Shell, TargetKind::Balloon] {
        let other = EventNames::for_kind(kind);
        assert_eq!(
            [
                other.begin,
                other.ready,
                other.download_begin,
                other.md5_begin,
                other.md5_complete,
                other.md5_failure,
                other.complete,
                other.failure,
            ],
            [
                "OnUpdateOtherBegin",
                "OnUpdateOtherReady",
                "OnUpdateOther.OnDownloadBegin",
                "OnUpdateOther.OnMD5CompareBegin",
                "OnUpdateOther.OnMD5CompareComplete",
                "OnUpdateOther.OnMD5CompareFailure",
                "OnUpdateOtherComplete",
                "OnUpdateOtherFailure",
            ],
            "{kind:?}"
        );
    }
}

/// `Progress` 6 変種 × ゴースト（manual）／シェル（script）の名・件数・Reference（要件 9.1）。
#[test]
fn progress_maps_to_events_for_ghost_and_shell() {
    for (kind, reason, prefix) in [
        (TargetKind::Ghost, UpdateReason::Manual, "OnUpdate"),
        (TargetKind::Shell, UpdateReason::Script, "OnUpdateOther"),
    ] {
        let tail = Tail::new(kind, reason);
        let (k, r) = (kind.as_ref_str(), reason.as_ref_str());
        let names = EventNames::for_kind(kind);
        let n = Numbering::from_useorigin1(None);
        let map = |p: &Progress| -> Vec<(String, Vec<String>)> {
            progress_events(p, names, n, tail)
                .into_iter()
                .map(|(id, refs)| (id.to_owned(), refs))
                .collect()
        };
        let ev = |suffix: &str, refs: &[&str]| (format!("{prefix}{suffix}"), s(refs));

        let fetched = Progress::ManifestFetched {
            name: ManifestName::Updates2Dau,
        };
        assert_eq!(map(&fetched), []);

        let diff = Progress::DiffDecided {
            files: s(&["a.txt", "dic/b.dic"]),
        };
        assert_eq!(
            map(&diff),
            [ev("Ready", &["1", "a.txt,dic/b.dic", "", k, r])]
        );
        let empty = Progress::DiffDecided { files: Vec::new() };
        assert_eq!(map(&empty), []);

        let download = Progress::DownloadBegin {
            file: "dic/b.dic".to_owned(),
            index: 1,
            total: 2,
        };
        assert_eq!(
            map(&download),
            [ev(".OnDownloadBegin", &["dic/b.dic", "1", "1", k, r])]
        );

        let refs = ["a.txt", "exp", "act", k, r];
        let matched = Progress::Md5Compared {
            file: "a.txt".to_owned(),
            expected: "exp".to_owned(),
            actual: "act".to_owned(),
            matched: true,
        };
        assert_eq!(
            map(&matched),
            [
                ev(".OnMD5CompareBegin", &refs),
                ev(".OnMD5CompareComplete", &refs)
            ]
        );
        let missed = Progress::Md5Compared {
            file: "a.txt".to_owned(),
            expected: "exp".to_owned(),
            actual: "act".to_owned(),
            matched: false,
        };
        assert_eq!(
            map(&missed),
            [
                ev(".OnMD5CompareBegin", &refs),
                ev(".OnMD5CompareFailure", &refs)
            ]
        );

        let committed = Progress::Committed {
            placed: s(&["a.txt"]),
        };
        assert_eq!(map(&committed), []);
        let deleted = Progress::Deleted {
            removed: vec![PathBuf::from("old.txt")],
        };
        assert_eq!(map(&deleted), []);
    }
}

/// `useorigin1` の `1`・`0`・返事なし（と空）で番号と件数の読み（要件 2.13・9.2）。
#[test]
fn useorigin1_shifts_ready_count_and_download_numbers() {
    let tail = Tail::new(TargetKind::Ghost, UpdateReason::Script);
    let names = EventNames::for_kind(TargetKind::Ghost);
    for (value, ready0, index1, last2) in [
        (Some("1"), "3", "1", "3"),
        (Some("0"), "2", "0", "2"),
        (None, "2", "0", "2"),
        (Some(""), "2", "0", "2"),
    ] {
        let n = Numbering::from_useorigin1(value);
        let ready = progress_events(
            &Progress::DiffDecided {
                files: s(&["a", "b", "c"]),
            },
            names,
            n,
            tail,
        );
        assert_eq!(ready[0].1[0], ready0, "{value:?}");
        let download = progress_events(
            &Progress::DownloadBegin {
                file: "a".to_owned(),
                index: 0,
                total: 3,
            },
            names,
            n,
            tail,
        );
        assert_eq!(
            (&download[0].1[1][..], &download[0].1[2][..]),
            (index1, last2),
            "{value:?}"
        );
    }
}

#[test]
fn process_exec_begin_complete_failure_and_executing_refs() {
    assert_eq!(process_exec_refs(UpdateReason::Manual), ["manual"]);

    let tail = Tail::new(TargetKind::Balloon, UpdateReason::Script);
    assert_eq!(
        begin_refs("えもバルーン", Path::new(r"C:\areka\balloon\emo"), tail),
        s(&[
            "えもバルーン",
            r"C:\areka\balloon\emo",
            "",
            "balloon",
            "script"
        ])
    );

    let unchanged = UpdateOutcome::Unchanged {
        manifest: ManifestName::UpdatesTxt,
    };
    assert_eq!(
        complete_refs(&unchanged, tail),
        s(&["none", "", "", "balloon", "script"])
    );
    let updated = UpdateOutcome::Updated {
        manifest: ManifestName::Updates2Dau,
        placed: s(&["a.png", "descript.txt"]),
        removed: Vec::new(),
        undeletable: Vec::new(),
        leftovers: Vec::new(),
    };
    assert_eq!(
        complete_refs(&updated, tail),
        s(&["changed", "a.png,descript.txt", "", "balloon", "script"])
    );

    assert_eq!(
        failure_refs("md5 miss", Some("a.png"), tail),
        s(&["md5 miss", "a.png", "", "balloon", "script"])
    );
    assert_eq!(
        failure_refs("timeout", None, tail),
        s(&["timeout", "", "", "balloon", "script"])
    );
    assert_eq!(
        executing_refs(TargetKind::Shell, UpdateReason::Manual),
        s(&["executing", "", "", "shell", "manual"])
    );
}

/// 取得の失敗 8 種を 1 つずつ（網羅の分岐＝種類が増えればこのテストがビルドで止まる）。
fn fetch_samples() -> Vec<(FetchError, &'static str)> {
    vec![
        (FetchError::NotFound, "404"),
        (FetchError::Status { code: 503 }, "503"),
        (FetchError::NameResolution, "dns"),
        (FetchError::Connect, "connect"),
        (FetchError::Timeout, "timeout"),
        (FetchError::Tls, "tls"),
        (FetchError::TooLarge { limit: 1 }, "toolarge"),
        (FetchError::Other { code: 12_345 }, "http"),
    ]
}

fn fetch_ordinal(e: &FetchError) -> usize {
    match e {
        FetchError::NotFound => 0,
        FetchError::Status { .. } => 1,
        FetchError::NameResolution => 2,
        FetchError::Connect => 3,
        FetchError::Timeout => 4,
        FetchError::Tls => 5,
        FetchError::TooLarge { .. } => 6,
        FetchError::Other { .. } => 7,
    }
}
const FETCH_KINDS: usize = 8;

/// エンジンの失敗 11 種（取得の失敗を包む 2 種は取得の失敗ごと）を組み、期待の語を並べる。
fn fail_samples() -> Vec<(FailReason, String)> {
    let p = || PathBuf::from(r"C:\t\a");
    let mut out = vec![
        (FailReason::TargetMissing { path: p() }, "fileio".to_owned()),
        (
            FailReason::InvalidHomeurl {
                homeurl: "ftp://x/".to_owned(),
            },
            "paramerror".to_owned(),
        ),
        (FailReason::ManifestMissing {}, "404".to_owned()),
        (
            FailReason::LocalUnreadable {
                path: p(),
                source: io_err(),
            },
            "fileio".to_owned(),
        ),
        (
            FailReason::WorkArea {
                path: p(),
                source: io_err(),
            },
            "fileio".to_owned(),
        ),
        (
            FailReason::Md5Mismatch {
                file: "a".to_owned(),
                expected: "e".to_owned(),
                actual: "a".to_owned(),
            },
            "md5 miss".to_owned(),
        ),
        (FailReason::EscapesTarget { path: p() }, "fileio".to_owned()),
        (
            FailReason::CommitWrite {
                path: p(),
                source: io_err(),
            },
            "fileio".to_owned(),
        ),
        (
            FailReason::RollbackFailed {
                path: p(),
                source: io_err(),
                restored: Vec::new(),
                stuck: vec![Stuck {
                    file: "a".to_owned(),
                    source: io_err(),
                }],
            },
            "fileio".to_owned(),
        ),
    ];
    for (source, word) in fetch_samples() {
        out.push((
            FailReason::ManifestFetch {
                name: ManifestName::Updates2Dau,
                source: source.clone(),
            },
            word.to_owned(),
        ));
        out.push((
            FailReason::FileFetch {
                file: "a".to_owned(),
                source,
            },
            word.to_owned(),
        ));
    }
    out
}

/// 失敗理由の表（要件 4.2・9.8）: 11 種と 8 種の全部が語に写り、判定した種類の数が全種類の数と等しい。
#[test]
fn failure_table_covers_all_eleven_and_eight_kinds() {
    let mut fail_kinds = BTreeSet::new();
    for (reason, word) in fail_samples() {
        assert_eq!(failure_word(&reason), word, "{}", reason.kind());
        fail_kinds.insert(reason.kind());
    }
    let all: BTreeSet<&str> = FailReason::ALL_KINDS.iter().copied().collect();
    assert_eq!(fail_kinds, all);
    assert_eq!(fail_kinds.len(), 11);

    let mut fetch_kinds = BTreeSet::new();
    for (error, word) in fetch_samples() {
        assert_eq!(fetch_word(&error), word, "{error:?}");
        fetch_kinds.insert(fetch_ordinal(&error));
    }
    assert_eq!(fetch_kinds.len(), FETCH_KINDS);
}

/// 正典に在るが送らない 7 語が、どの写しからも出ない（要件 4.3）。
#[test]
fn unsent_seven_words_never_appear() {
    let words: Vec<String> = fail_samples()
        .iter()
        .map(|(reason, _)| failure_word(reason))
        .chain(fetch_samples().iter().map(|(e, _)| fetch_word(e)))
        .chain(executing_refs(TargetKind::Ghost, UpdateReason::Script).into_iter())
        .collect();
    for word in &words {
        assert!(!UNSENT_WORDS.contains(&word.as_str()), "{word}");
    }
}

fn spec(kind: TargetKind, name: &str) -> TargetSpec {
    TargetSpec {
        kind,
        dir: PathBuf::from(r"C:\areka\x"),
        name: name.to_owned(),
        descript_homeurl: None,
    }
}

/// 成功と失敗の混ざった 3 対象の総括の 2 形（要件 3.2・3.3・9.3）。飛ばした対象は載らない。
#[test]
fn summary_refs_in_both_forms_for_mixed_three_targets() {
    let (ghost, shell, balloon) = (
        spec(TargetKind::Ghost, "emo2"),
        spec(TargetKind::Shell, "master"),
        spec(TargetKind::Balloon, "emo-gs"),
    );
    let changed = TargetEnd::Changed(2);
    let md5 = TargetEnd::Failed {
        word: "md5 miss".to_owned(),
        file: Some("a.png".to_owned()),
    };
    let unchanged = TargetEnd::Unchanged;
    let skipped = TargetEnd::Skipped;
    let ends = [
        (&ghost, &changed),
        (&shell, &md5),
        (&shell, &skipped),
        (&balloon, &unchanged),
    ];
    assert_eq!(
        summary_refs(SummaryKind::Result, &ends),
        [
            format!("ghost{SEP}OK{SEP}2"),
            format!("shell{SEP}NG{SEP}md5 miss{SEP}a.png"),
            format!("balloon{SEP}OK{SEP}0"),
        ]
    );

    let timeout = TargetEnd::Failed {
        word: "timeout".to_owned(),
        file: None,
    };
    let one = TargetEnd::Changed(1);
    let ends = [(&ghost, &timeout), (&shell, &one), (&balloon, &unchanged)];
    assert_eq!(
        summary_refs(SummaryKind::ResultEx, &ends),
        [
            format!("emo2{SEP}ghost{SEP}NG{SEP}timeout"),
            format!("master{SEP}shell{SEP}OK{SEP}1"),
            format!("emo-gs{SEP}balloon{SEP}OK{SEP}0"),
        ]
    );
    assert_eq!(SEPARATOR, SEP);
}
