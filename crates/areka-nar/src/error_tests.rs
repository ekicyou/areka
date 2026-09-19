//! `error` の兄弟テスト。拒否語彙が 13 で閉じていること、短い語が重複しないこと、
//! 宣言順の一覧と実際の変種が双方向で一致すること、詳細を持つ変種の表示が
//! その詳細を落とさないことを判定する（要件 9.2）。
//!
//! ここでは固定入力の `.nar` を通さない。実物のアーカイブから 13 変種すべてに
//! 到達できることの判定はタスク 4.4 が `ALL_KINDS` と突き合わせて行う。

use super::*;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// 宣言順に 1 つずつ組んだ全変種の見本。
///
/// 変種を足すと `ALL_KINDS` は宣言から自動で伸びるので、この一覧を直し忘れると
/// `all_kinds_matches_every_variant_in_declaration_order` が赤になる。
fn samples_in_declaration_order() -> Vec<RefuseReason> {
    vec![
        RefuseReason::CorruptArchive {
            detail: "EOCD が見つからない".to_string(),
        },
        RefuseReason::IntegrityMismatch {
            index: 7,
            name: "ghost/master/shiori.dll".to_string(),
            what: Integrity::Crc {
                expected: 0x1234_abcd,
                actual: 0x0000_0001,
            },
        },
        RefuseReason::UnsupportedEntry {
            index: 3,
            name: "readme.txt".to_string(),
            what: Unsupported::Compression(99),
        },
        RefuseReason::NameUndecodable {
            index: 4,
            raw_hex: "8140fe".to_string(),
            encoding: "Shift_JIS",
        },
        RefuseReason::SymlinkEntry {
            index: 5,
            name: "link".to_string(),
        },
        RefuseReason::UnsafePath {
            index: 6,
            name: "CON.txt".to_string(),
            why: UnsafeWhy::InvalidWindowsName("CON".to_string()),
        },
        RefuseReason::CaseCollision {
            a: "A.txt".to_string(),
            b: "a.txt".to_string(),
        },
        RefuseReason::MissingInstallTxt {
            top_level: vec!["emo2".to_string()],
        },
        RefuseReason::UnsupportedType {
            found: Some("plugin".to_string()),
        },
        RefuseReason::MissingRequiredKey { key: "directory" },
        RefuseReason::InvalidDirectoryName {
            key: "directory".to_string(),
            value: "../外".to_string(),
        },
        RefuseReason::CompanionSourceMissing {
            key: "balloon0".to_string(),
            source_directory: "kakukaku".to_string(),
        },
        RefuseReason::TargetGhostMissing {
            target: Some("emo2".to_string()),
        },
    ]
}

#[test]
fn all_kinds_has_thirteen_entries() {
    assert_eq!(
        RefuseReason::ALL_KINDS.len(),
        13,
        "拒否語彙は 13 変種で閉じる（要件 9.2）"
    );
}

#[test]
fn all_kinds_are_unique() {
    let unique: BTreeSet<&&str> = RefuseReason::ALL_KINDS.iter().collect();
    assert_eq!(
        unique.len(),
        RefuseReason::ALL_KINDS.len(),
        "短い語が重複している: {:?}",
        RefuseReason::ALL_KINDS
    );
}

#[test]
fn all_kinds_matches_every_variant_in_declaration_order() {
    let observed: Vec<&'static str> = samples_in_declaration_order()
        .iter()
        .map(RefuseReason::kind)
        .collect();
    assert_eq!(
        observed,
        RefuseReason::ALL_KINDS,
        "宣言順の一覧と実際の変種が食い違っている"
    );
}

#[test]
fn all_kinds_and_reachable_kinds_agree_in_both_directions() {
    let listed: BTreeSet<&str> = RefuseReason::ALL_KINDS.iter().copied().collect();
    let built: BTreeSet<&str> = samples_in_declaration_order()
        .iter()
        .map(RefuseReason::kind)
        .collect();
    let listed_only: Vec<&&str> = listed.difference(&built).collect();
    let built_only: Vec<&&str> = built.difference(&listed).collect();
    assert!(
        listed_only.is_empty(),
        "一覧にあるのに組めない短い語: {listed_only:?}"
    );
    assert!(
        built_only.is_empty(),
        "組めるのに一覧に無い短い語: {built_only:?}"
    );
}

#[test]
fn kind_is_the_variant_name() {
    assert_eq!(
        RefuseReason::MissingRequiredKey { key: "name" }.kind(),
        "MissingRequiredKey"
    );
    assert_eq!(
        RefuseReason::TargetGhostMissing { target: None }.kind(),
        "TargetGhostMissing"
    );
}

#[test]
fn integrity_mismatch_display_names_the_entry_and_both_numbers() {
    let shown = RefuseReason::IntegrityMismatch {
        index: 7,
        name: "ghost/master/shiori.dll".to_string(),
        what: Integrity::Crc {
            expected: 0x1234_abcd,
            actual: 0x0000_0001,
        },
    }
    .to_string();
    assert!(shown.contains('7'), "エントリ番号が落ちている: {shown}");
    assert!(
        shown.contains("ghost/master/shiori.dll"),
        "エントリ名が落ちている: {shown}"
    );
    assert!(shown.contains("1234abcd"), "期待値が落ちている: {shown}");
    assert!(shown.contains("00000001"), "実際の値が落ちている: {shown}");
}

#[test]
fn integrity_size_display_carries_both_lengths() {
    let shown = Integrity::Size {
        expected: 1024,
        actual: 7,
    }
    .to_string();
    assert!(shown.contains("1024") && shown.contains('7'), "{shown}");
}

#[test]
fn unsupported_entry_display_names_the_method_number() {
    let shown = RefuseReason::UnsupportedEntry {
        index: 3,
        name: "readme.txt".to_string(),
        what: Unsupported::Compression(99),
    }
    .to_string();
    assert!(shown.contains("99"), "圧縮方式の番号が落ちている: {shown}");
    assert!(shown.contains("readme.txt"), "{shown}");
}

#[test]
fn name_undecodable_display_carries_raw_bytes_and_encoding() {
    let shown = RefuseReason::NameUndecodable {
        index: 4,
        raw_hex: "8140fe".to_string(),
        encoding: "Shift_JIS",
    }
    .to_string();
    assert!(shown.contains("8140fe"), "生バイトが落ちている: {shown}");
    assert!(
        shown.contains("Shift_JIS"),
        "文字コード名が落ちている: {shown}"
    );
}

#[test]
fn unsafe_path_display_carries_the_reason_detail() {
    let shown = RefuseReason::UnsafePath {
        index: 6,
        name: "CON.txt".to_string(),
        why: UnsafeWhy::InvalidWindowsName("CON".to_string()),
    }
    .to_string();
    assert!(shown.contains("CON"), "{shown}");
    assert!(shown.contains("CON.txt"), "{shown}");
}

#[test]
fn missing_install_txt_display_lists_the_top_level() {
    let shown = RefuseReason::MissingInstallTxt {
        top_level: vec!["emo2".to_string(), "docs".to_string()],
    }
    .to_string();
    assert!(shown.contains("emo2") && shown.contains("docs"), "{shown}");
}

#[test]
fn unsupported_type_display_carries_what_was_found() {
    let shown = RefuseReason::UnsupportedType {
        found: Some("plugin".to_string()),
    }
    .to_string();
    assert!(shown.contains("plugin"), "{shown}");
}

/// `type` が無い場合（要件 3.6）の表示に Rust の綴り（`None`）を出さない。
#[test]
fn absent_values_are_shown_in_words_not_in_rust_syntax() {
    let no_type = RefuseReason::UnsupportedType { found: None }.to_string();
    assert_eq!(no_type, "対応していない種別: （指定なし）");

    let no_target = RefuseReason::TargetGhostMissing { target: None }.to_string();
    assert_eq!(no_target, "宛先のゴーストが無い: （指定なし）");

    let named_target = RefuseReason::TargetGhostMissing {
        target: Some("emo2".to_string()),
    }
    .to_string();
    assert_eq!(named_target, "宛先のゴーストが無い: emo2");
}

#[test]
fn case_collision_display_names_both_sides() {
    let shown = RefuseReason::CaseCollision {
        a: "A.txt".to_string(),
        b: "a.txt".to_string(),
    }
    .to_string();
    assert!(
        shown.contains("A.txt") && shown.contains("a.txt"),
        "{shown}"
    );
}

#[test]
fn companion_source_missing_display_names_key_and_source() {
    let shown = RefuseReason::CompanionSourceMissing {
        key: "balloon0".to_string(),
        source_directory: "kakukaku".to_string(),
    }
    .to_string();
    assert!(
        shown.contains("balloon0") && shown.contains("kakukaku"),
        "{shown}"
    );
}

#[test]
fn refused_display_carries_the_archive_path_and_the_reason() {
    let shown = NarError::Refused {
        archive: PathBuf::from(r"C:\samples\emo2.nar"),
        reason: RefuseReason::MissingRequiredKey { key: "directory" },
    }
    .to_string();
    // 書式は設計の逐語どおり。`PathBuf` は thiserror が `.display()` を通すので、
    // Debug の引用符は付かない（付いたらここが赤くなる）。
    assert_eq!(
        shown,
        r"C:\samples\emo2.nar: 拒否: 必須キー directory が無い"
    );
}

#[test]
fn io_display_carries_phase_path_and_source() {
    let shown = NarError::Io {
        archive: PathBuf::from(r"C:\samples\emo2.nar"),
        phase: IoPhase::Commit,
        path: PathBuf::from(r"C:\root\ghost\emo2"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "使用中"),
        committed: Vec::new(),
        rolled_back: true,
    }
    .to_string();
    // ここも設計の逐語の書式。2 つの `PathBuf` が引用符無しで出る。
    assert!(
        shown.starts_with(r"C:\samples\emo2.nar: Commit で I/O に失敗: C:\root\ghost\emo2: "),
        "書式が崩れている: {shown}"
    );
    assert!(shown.contains("使用中"), "元の失敗が落ちている: {shown}");
}

#[test]
fn io_keeps_what_was_committed_and_whether_it_was_rolled_back() {
    let element = InstalledElement {
        kind: ElementKind::Balloon,
        name: "emo2-kakukaku".to_string(),
        path: PathBuf::from(r"C:\root\balloon\emo2-kakukaku"),
        target_ghost: None,
        existing: ExistingState::New,
    };
    let err = NarError::Io {
        archive: PathBuf::from(r"C:\samples\emo2.nar"),
        phase: IoPhase::Rollback,
        path: PathBuf::from(r"C:\root\ghost\emo2"),
        source: std::io::Error::other("失敗"),
        committed: vec![element.clone()],
        rolled_back: false,
    };
    match err {
        NarError::Io {
            phase,
            committed,
            rolled_back,
            ..
        } => {
            assert_eq!(phase, IoPhase::Rollback);
            assert_eq!(committed, vec![element]);
            assert!(!rolled_back);
        }
        other => panic!("Io を期待した: {other}"),
    }
}

/// 警告の変種名。ワイルドカードの腕を持たないので、変種を足すとここが
/// コンパイルエラーになる＝「5 種」の主張が恒真にならない。
fn warning_name(warning: &ManifestWarning) -> &'static str {
    match warning {
        ManifestWarning::UnsupportedCompanionKind { .. } => "UnsupportedCompanionKind",
        ManifestWarning::CompanionOnNonGhost { .. } => "CompanionOnNonGhost",
        ManifestWarning::IgnoredKey { .. } => "IgnoredKey",
        ManifestWarning::RefreshIgnoredForSupplement => "RefreshIgnoredForSupplement",
        ManifestWarning::InvalidMaskEntry { .. } => "InvalidMaskEntry",
    }
}

#[test]
fn manifest_warnings_are_five_and_carry_their_key() {
    let warnings = [
        ManifestWarning::UnsupportedCompanionKind {
            key: "headline0".to_string(),
        },
        ManifestWarning::CompanionOnNonGhost {
            key: "balloon.directory".to_string(),
        },
        ManifestWarning::IgnoredKey {
            key: "bootghost".to_string(),
        },
        ManifestWarning::RefreshIgnoredForSupplement,
        ManifestWarning::InvalidMaskEntry {
            key: "refreshundeletemask".to_string(),
            value: "..".to_string(),
        },
    ];
    let names: Vec<&'static str> = warnings.iter().map(warning_name).collect();
    assert_eq!(
        names,
        [
            "UnsupportedCompanionKind",
            "CompanionOnNonGhost",
            "IgnoredKey",
            "RefreshIgnoredForSupplement",
            "InvalidMaskEntry",
        ],
        "マニフェストの警告は 5 種（設計）"
    );
    assert!(warnings[0].to_string().contains("headline0"));
    assert!(warnings[1].to_string().contains("balloon.directory"));
    assert!(warnings[2].to_string().contains("bootghost"));
    assert!(!warnings[3].to_string().is_empty());
    let last = warnings[4].to_string();
    assert!(
        last.contains("refreshundeletemask") && last.contains(".."),
        "{last}"
    );
}
