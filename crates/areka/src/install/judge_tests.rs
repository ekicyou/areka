//! 判断の分かれ目のテスト（design「Testing Strategy / 判断」・要件 11.2・11.4・11.6）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use areka_nar::{
    ExistingPolicy, Integrity, IoPhase, NarError, RefuseReason, UnsafeWhy, Unsupported,
};

use super::*;

const SEP: &str = "\u{1}";

fn manifest(
    kind: InstallKind,
    name: &str,
    directory: &str,
    accept: Option<&str>,
) -> InstallManifest {
    InstallManifest {
        kind,
        name: name.to_owned(),
        directory: directory.to_owned(),
        accept: accept.map(str::to_owned),
        existing: ExistingPolicy::Overlay,
        companions: Vec::new(),
        warnings: Vec::new(),
        charset_declared: None,
    }
}

/// 起動中のゴースト `emo2`（`sakura.name`＝えもこ・`install.accept`＝emo,えもつー）。
fn emo2() -> GhostFacts {
    GhostFacts {
        root: PathBuf::from(r"C:\areka"),
        folder: Some("emo2".to_owned()),
        name: "emo2".to_owned(),
        sakura_name: Some("えもこ".to_owned()),
        install_accept: vec!["emo".to_owned(), "えもつー".to_owned()],
    }
}

// ---- judge_accept（要件 3.1・3.4・3.5・3.6・11.2・12.9） ----

#[test]
fn accept_matching_sakura_name_is_accepted_with_running_ghost_folder() {
    let shell = manifest(InstallKind::Shell, "冬服", "winter", Some("えもこ"));
    assert_eq!(
        judge_accept(&shell, &emo2()),
        AcceptVerdict::Accepted {
            target_ghost: Some("emo2".to_owned())
        }
    );
}

#[test]
fn accept_matching_second_install_accept_name_is_accepted() {
    let supplement = manifest(
        InstallKind::Supplement,
        "追加辞書",
        "emo2",
        Some("えもつー"),
    );
    assert_eq!(
        judge_accept(&supplement, &emo2()),
        AcceptVerdict::Accepted {
            target_ghost: Some("emo2".to_owned())
        }
    );
}

#[test]
fn accept_differing_only_in_case_is_refused() {
    let shell = manifest(InstallKind::Shell, "冬服", "winter", Some("EMO"));
    assert_eq!(
        judge_accept(&shell, &emo2()),
        AcceptVerdict::Refused {
            accept: "EMO".to_owned()
        }
    );
}

#[test]
fn accept_matching_nothing_is_refused() {
    let shell = manifest(InstallKind::Shell, "冬服", "winter", Some("だれか"));
    assert_eq!(
        judge_accept(&shell, &emo2()),
        AcceptVerdict::Refused {
            accept: "だれか".to_owned()
        }
    );
}

#[test]
fn ghost_and_balloon_without_accept_are_accepted_without_target() {
    for kind in [InstallKind::Ghost, InstallKind::Balloon] {
        let item = manifest(kind, "なにか", "something", None);
        assert_eq!(
            judge_accept(&item, &emo2()),
            AcceptVerdict::Accepted { target_ghost: None },
            "{kind:?}"
        );
    }
}

#[test]
fn shell_and_supplement_without_accept_are_accept_missing() {
    for kind in [InstallKind::Shell, InstallKind::Supplement] {
        let item = manifest(kind, "なにか", "something", None);
        assert_eq!(
            judge_accept(&item, &emo2()),
            AcceptVerdict::AcceptMissing,
            "{kind:?}"
        );
    }
}

// ---- destination_of（要件 3.5・7.1・7.8） ----

#[test]
fn destination_kinds() {
    let ghost = emo2();
    let cases = [
        (InstallKind::Ghost, "emo2", Destination::RunningGhost),
        (InstallKind::Ghost, "EMO2", Destination::RunningGhost),
        (InstallKind::Ghost, "other", Destination::Elsewhere),
        (
            InstallKind::Supplement,
            "whatever",
            Destination::RunningGhost,
        ),
        (InstallKind::Shell, "emo2", Destination::Elsewhere),
        (InstallKind::Balloon, "emo2", Destination::Elsewhere),
    ];
    for (kind, directory, expected) in cases {
        let item = manifest(kind, "x", directory, Some("えもこ"));
        assert_eq!(
            destination_of(&item, &ghost),
            expected,
            "{kind:?} {directory}"
        );
    }
}

#[test]
fn destination_is_elsewhere_for_every_kind_when_ghost_has_no_folder() {
    let ghost = GhostFacts {
        folder: None,
        ..emo2()
    };
    for kind in [
        InstallKind::Ghost,
        InstallKind::Shell,
        InstallKind::Supplement,
        InstallKind::Balloon,
    ] {
        let item = manifest(kind, "x", "emo2", Some("えもこ"));
        assert_eq!(
            destination_of(&item, &ghost),
            Destination::Elsewhere,
            "{kind:?}"
        );
    }
}

// ---- failure_word（要件 5.2・5.3・11.4・12.10） ----

fn refused(reason: RefuseReason) -> NarError {
    NarError::Refused {
        archive: PathBuf::from(r"C:\in\a.nar"),
        reason,
    }
}

/// 14 種を 1 つずつ組み、要件 5.2 の表の語と突き合わせる。`UnsupportedType` は `found` の有無で 2 通り。
#[test]
fn every_refuse_kind_maps_to_its_canonical_word() {
    use FailureWord as W;
    let s = String::new;
    let cases = [
        (RefuseReason::CorruptArchive { detail: s() }, W::Extraction),
        (
            RefuseReason::IntegrityMismatch {
                index: 0,
                name: s(),
                what: Integrity::Inflate,
            },
            W::Extraction,
        ),
        (
            RefuseReason::NameUndecodable {
                index: 0,
                raw_hex: s(),
                encoding: "Shift_JIS",
            },
            W::Extraction,
        ),
        (
            RefuseReason::MissingInstallTxt { top_level: vec![] },
            W::InvalidType,
        ),
        (
            RefuseReason::UnsupportedType { found: None },
            W::InvalidType,
        ),
        (
            RefuseReason::MissingRequiredKey { key: "name" },
            W::InvalidType,
        ),
        (
            RefuseReason::InvalidDirectoryName {
                key: s(),
                value: s(),
            },
            W::InvalidType,
        ),
        (
            RefuseReason::CompanionSourceMissing {
                key: s(),
                source_directory: s(),
            },
            W::InvalidType,
        ),
        (
            RefuseReason::TargetGhostMissing { target: None },
            W::InvalidType,
        ),
        (
            RefuseReason::UnsupportedEntry {
                index: 0,
                name: s(),
                what: Unsupported::Encrypted,
            },
            W::Unsupported,
        ),
        (
            RefuseReason::UnsupportedType {
                found: Some("plugin".to_owned()),
            },
            W::Unsupported,
        ),
        (
            RefuseReason::SymlinkEntry {
                index: 0,
                name: s(),
            },
            W::Unsupported,
        ),
        (
            RefuseReason::UnsafePath {
                index: 0,
                name: s(),
                why: UnsafeWhy::DotDot,
            },
            W::Unsupported,
        ),
        (
            RefuseReason::PathTooLong {
                index: 0,
                length: 300,
                limit: 200,
                head: s(),
            },
            W::Unsupported,
        ),
        (
            RefuseReason::CaseCollision { a: s(), b: s() },
            W::Unsupported,
        ),
    ];
    let mut judged = BTreeSet::new();
    for (reason, expected) in cases {
        let kind = reason.kind();
        assert_eq!(failure_word(&refused(reason)), expected, "{kind}");
        judged.insert(kind);
    }
    assert_eq!(judged.len(), RefuseReason::ALL_KINDS.len());
    assert_eq!(
        judged,
        RefuseReason::ALL_KINDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn every_io_phase_is_extraction() {
    for phase in [
        IoPhase::Read,
        IoPhase::Stage,
        IoPhase::Commit,
        IoPhase::Rollback,
    ] {
        let error = NarError::Io {
            archive: PathBuf::from(r"C:\in\a.nar"),
            phase,
            path: PathBuf::from(r"C:\areka\ghost\emo2"),
            source: std::io::Error::other("使用中"),
            committed: Vec::new(),
            rolled_back: true,
            survivors: Box::new([]),
        };
        assert_eq!(failure_word(&error), FailureWord::Extraction, "{phase:?}");
    }
}

#[test]
fn failure_words_are_the_three_canonical_spellings() {
    assert_eq!(FailureWord::Extraction.as_ref_str(), "extraction");
    assert_eq!(FailureWord::InvalidType.as_ref_str(), "invalid type");
    assert_eq!(FailureWord::Unsupported.as_ref_str(), "unsupported");
}

// ---- kind_word・Reference（要件 2.4・2.5・2.6・3.2・12.7・11.1） ----

#[test]
fn kind_words_are_the_four_canonical_identifiers() {
    assert_eq!(kind_word(&ElementKind::Ghost), "ghost");
    assert_eq!(kind_word(&ElementKind::Shell), "shell");
    assert_eq!(kind_word(&ElementKind::Balloon), "balloon");
    assert_eq!(kind_word(&ElementKind::Supplement), "supplement");
}

fn item(kind: &ElementKind, name: &str, place: &str) -> InstalledItem {
    InstalledItem {
        kind: kind_word(kind),
        name: name.to_owned(),
        place: place.to_owned(),
    }
}

/// 書庫 5 種: (install.txt, installed の順の物, Ex の期待, 旧仕様の期待)。
fn five_archives() -> Vec<(
    InstallManifest,
    Vec<InstalledItem>,
    [String; 3],
    [String; 3],
)> {
    let ghost_place = r"C:\areka\ghost\hana";
    let balloon_place = r"C:\areka\balloon\hanab";
    let j = |parts: &[&str]| parts.join(SEP);
    vec![
        (
            manifest(InstallKind::Ghost, "はな", "hana", None),
            vec![item(&ElementKind::Ghost, "はな", ghost_place)],
            ["ghost".into(), "はな".into(), ghost_place.into()],
            ["ghost".into(), "はな".into(), String::new()],
        ),
        (
            manifest(InstallKind::Ghost, "はな", "hana", None),
            vec![
                item(&ElementKind::Ghost, "はな", ghost_place),
                item(&ElementKind::Balloon, "hanab", balloon_place),
            ],
            [
                j(&["ghost", "balloon"]),
                j(&["はな", "hanab"]),
                j(&[ghost_place, balloon_place]),
            ],
            ["ghost".into(), "はな".into(), "hanab".into()],
        ),
        (
            manifest(InstallKind::Balloon, "はなバルーン", "hanab", None),
            vec![item(&ElementKind::Balloon, "はなバルーン", balloon_place)],
            [
                "balloon".into(),
                "はなバルーン".into(),
                balloon_place.into(),
            ],
            ["balloon".into(), "はなバルーン".into(), String::new()],
        ),
        (
            manifest(InstallKind::Shell, "冬服", "winter", Some("えもこ")),
            vec![item(
                &ElementKind::Shell,
                "冬服",
                r"C:\areka\ghost\emo2\shell\winter",
            )],
            [
                "shell".into(),
                "冬服".into(),
                r"C:\areka\ghost\emo2\shell\winter".into(),
            ],
            ["shell".into(), "冬服".into(), String::new()],
        ),
        (
            manifest(InstallKind::Supplement, "追加辞書", "emo2", Some("えもこ")),
            vec![item(
                &ElementKind::Supplement,
                "追加辞書",
                r"C:\areka\ghost\emo2",
            )],
            [
                "supplement".into(),
                "追加辞書".into(),
                r"C:\areka\ghost\emo2".into(),
            ],
            ["supplement".into(), "追加辞書".into(), String::new()],
        ),
    ]
}

#[test]
fn complete_ex_and_legacy_refs_for_five_archives() {
    for (manifest, items, ex, legacy) in five_archives() {
        assert_eq!(complete_ex_refs(&items), ex.to_vec(), "{}", manifest.name);
        assert_eq!(
            complete_legacy_refs(&manifest.name, &items),
            legacy.to_vec(),
            "{}",
            manifest.name
        );
    }
}

/// 同梱バルーンが 2 つ（balloon・balloon0）: Ex は 3 件ぶん、旧仕様は正典どおり先頭の 2 件だけ。
#[test]
fn two_companion_balloons_legacy_carries_only_the_first_two_items() {
    let ghost = r"C:\areka\ghost\hana";
    let first = r"C:\areka\balloon\hanab";
    let second = r"C:\areka\balloon\hanab2";
    let items = vec![
        item(&ElementKind::Ghost, "はな", ghost),
        item(&ElementKind::Balloon, "hanab", first),
        item(&ElementKind::Balloon, "hanab2", second),
    ];
    assert_eq!(
        complete_ex_refs(&items),
        vec![
            ["ghost", "balloon", "balloon"].join(SEP),
            ["はな", "hanab", "hanab2"].join(SEP),
            [ghost, first, second].join(SEP),
        ]
    );
    let legacy = complete_legacy_refs("はな", &items);
    assert_eq!(legacy, vec!["ghost", "はな", "hanab"]);
    assert!(!legacy[2].contains(SEP));
}

#[test]
fn refuse_refs_carry_accept_identifier_and_name() {
    let expected_kinds = ["ghost", "ghost", "balloon", "shell", "supplement"];
    for ((manifest, _, _, _), kind) in five_archives().into_iter().zip(expected_kinds) {
        assert_eq!(
            refuse_refs("だれか", &manifest),
            vec!["だれか".to_owned(), kind.to_owned(), manifest.name.clone()]
        );
    }
}

#[test]
fn ghost_with_balloon_never_appears() {
    for (manifest, items, _, _) in five_archives() {
        let all: Vec<String> = complete_ex_refs(&items)
            .into_iter()
            .chain(complete_legacy_refs(&manifest.name, &items))
            .chain(refuse_refs("x", &manifest))
            .collect();
        assert!(
            all.iter().all(|r| !r.contains(" with ")),
            "{}: {all:?}",
            manifest.name
        );
    }
}

// ---- script_request（要件 1.5・1.6・1.7・11.6） ----

#[test]
fn script_request_six_cases() {
    let absolute = r"C:\Users\u\Downloads\hana.nar";
    assert_eq!(
        script_request(&["path", absolute]),
        Ok(PathBuf::from(absolute))
    );
    assert_eq!(
        script_request(&["path", r"Downloads\hana.nar"]),
        Err(ScriptRefusal::Relative {
            path: r"Downloads\hana.nar".to_owned()
        })
    );
    assert_eq!(script_request(&["path", ""]), Err(ScriptRefusal::Empty));
    assert_eq!(script_request(&["path"]), Err(ScriptRefusal::Empty));
    assert_eq!(
        script_request(&["url", "https://example.com/hana.nar"]),
        Err(ScriptRefusal::NotPath {
            found: "url".to_owned()
        })
    );
    assert_eq!(
        script_request(&["file", absolute]),
        Err(ScriptRefusal::NotPath {
            found: "file".to_owned()
        })
    );
    assert_eq!(
        script_request(&[]),
        Err(ScriptRefusal::NotPath {
            found: String::new()
        })
    );
}

#[test]
fn script_request_ignores_arguments_after_the_path() {
    let absolute = r"C:\in\hana.nar";
    assert_eq!(
        script_request(&["path", absolute, "--extra"]),
        Ok(Path::new(absolute).to_path_buf())
    );
}
