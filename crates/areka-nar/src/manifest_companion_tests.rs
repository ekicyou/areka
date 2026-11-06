//! `manifest` の兄弟テストの続き——同時インストールのバルーンと再インストールの
//! 規則、そして読み飛ばしの記録（要件 3.12〜3.16）。
//!
//! 助手と固定入力は [`super`]（`manifest_tests.rs`）から借りる。分けたのは
//! 1 ファイル 1,000 行の上限に収めるためで、固定入力は 1 つも削っていない。
//!
//! 警告は必ず**順序つきの列**そのもので突き合わせる。「含む」で測ると、
//! 同じ警告が二重に出ても 1 件落ちても緑のまま通る。

use super::*;

// ---- 同時インストールのバルーン（要件 3.12） ----

/// `balloon.directory` と `balloon.source.directory` を読む（要件 3.12・検体 emo2 の形）。
#[test]
fn reads_the_balloon_companion_with_its_own_source_directory() {
    let manifest = parsed_ghost(&[
        "balloon.directory,emo2-kakukaku",
        "balloon.source.directory,kakukaku-src",
    ]);
    assert_eq!(
        manifest.companions,
        vec![Companion {
            key: "balloon".to_owned(),
            directory: "emo2-kakukaku".to_owned(),
            source_directory: "kakukaku-src".to_owned(),
            existing: ExistingPolicy::Overlay,
            source_written: Some("kakukaku-src".to_owned()),
        }]
    );
    assert_eq!(manifest.warnings, vec![]);
}

/// `shell` でも同時インストールを読む（要件 3.12 は ghost と shell の両方）。
#[test]
fn reads_companions_for_a_shell_too() {
    let manifest = parsed(&[
        "type,shell",
        "name,かくかく",
        "directory,kakukaku",
        "balloon.directory,kakukaku-balloon",
    ]);
    assert_eq!(manifest.companions.len(), 1);
    assert_eq!(manifest.warnings, vec![]);
}

/// `balloonN` は無印 → 0 → 1… の探索の順に読み、欠番で打ち切る（要件 1.1・1.3）。
///
/// `balloon1` が無いので `balloon2` と `balloon10` は読まれず、鍵の名前順に
/// 「探索で読まなかった」として記録される（要件 6.3）。
#[test]
fn reads_numbered_balloon_companions_in_search_order() {
    let manifest = parsed_ghost(&[
        "balloon2.directory,two",
        "balloon10.directory,ten",
        "balloon0.directory,zero",
        "balloon.directory,plain",
    ]);
    let keys: Vec<&str> = manifest
        .companions
        .iter()
        .map(|companion| companion.key.as_str())
        .collect();
    assert_eq!(keys, vec!["balloon", "balloon0"]);
    assert_eq!(
        manifest.warnings,
        vec![
            ManifestWarning::CompanionNotSearched {
                key: "balloon10.directory".to_owned()
            },
            ManifestWarning::CompanionNotSearched {
                key: "balloon2.directory".to_owned()
            },
        ]
    );
}

/// `balloon` の直後が数字でなければ同時インストールではない（要件 3.12 の境界）。
///
/// 片側だけ（`balloon0` が通ること）を測っても境界は決まらない。通らない側も並べる。
#[test]
fn balloon_followed_by_a_non_digit_is_not_a_companion() {
    for key in ["balloons", "balloon-1", "balloon_0", "balloonx", "baloon"] {
        let manifest = parsed_ghost(&[&format!("{key}.directory,x")]);
        assert_eq!(
            manifest.companions,
            vec![],
            "{key}.directory を同時インストールとして読んでいる"
        );
        assert_eq!(
            manifest.warnings,
            vec![ManifestWarning::IgnoredKey {
                key: format!("{key}.directory")
            }],
            "{key}.directory が知らないキーとして記録されない"
        );
    }
}

/// `balloon` の直後が数字なら同時インストールの形（上の境界の反対側）。
///
/// 単独で書いて読まれるのは、探索が最初に引く `balloon` と `balloon0` だけ。
/// `balloon9`・`balloon10`・`balloon007` は読まれないが、知らない鍵ではなく
/// 「探索で読まなかった」として記録する（同梱の形としては認めている・要件 1.3・1.6・6.3）。
#[test]
fn balloon_followed_by_digits_is_a_companion() {
    for key in ["balloon", "balloon0"] {
        let manifest = parsed_ghost(&[&format!("{key}.directory,x")]);
        assert_eq!(manifest.companions.len(), 1, "{key}.directory が読まれない");
        assert_eq!(manifest.companions[0].key, key);
        assert_eq!(manifest.warnings, vec![]);
    }
    for key in ["balloon9", "balloon10", "balloon007"] {
        let manifest = parsed_ghost(&[&format!("{key}.directory,x")]);
        assert_eq!(manifest.companions, vec![], "{key}.directory が読まれた");
        assert_eq!(
            manifest.warnings,
            vec![ManifestWarning::CompanionNotSearched {
                key: format!("{key}.directory")
            }],
            "{key}.directory が「探索で読まなかった」として記録されない"
        );
    }
}

/// 宛先（`*.directory`）の無い同時インストールの断片は知らないキーとして記録する。
///
/// `balloon.source.directory` だけでは、どこへ入れるのか決まらない（要件 3.12・3.16）。
#[test]
fn a_companion_key_without_its_directory_is_recorded_as_an_ignored_key() {
    let manifest = parsed_ghost(&[
        "balloon.source.directory,kakukaku-src",
        "balloon.refresh,1",
        "balloon.refreshundeletemask,keep.txt",
    ]);
    assert_eq!(manifest.companions, vec![]);
    assert_eq!(
        manifest.warnings,
        vec![
            ManifestWarning::IgnoredKey {
                key: "balloon.refresh".to_owned()
            },
            ManifestWarning::IgnoredKey {
                key: "balloon.refreshundeletemask".to_owned()
            },
            ManifestWarning::IgnoredKey {
                key: "balloon.source.directory".to_owned()
            },
        ]
    );
}

/// 長い知らないキーは、警告に先頭と長さだけを載せる（全体は載せない）。
#[test]
fn a_long_ignored_key_is_bounded_in_its_warning() {
    let key = "x".repeat(300);
    let manifest = parsed_ghost(&[&format!("{key},1")]);
    assert_eq!(
        manifest.warnings,
        vec![ManifestWarning::IgnoredKey {
            key: bounded_value(&key)
        }]
    );
    assert!(!manifest.warnings[0].to_string().contains(&key));
}

// ---- 扱わない同時インストールの種別（要件 3.13） ----

/// `balloon` 以外の同時インストールは警告して読み飛ばし、本体の展開は続ける（要件 3.13）。
#[test]
fn warns_and_skips_the_companion_kinds_areka_does_not_handle() {
    for prefix in [
        "headline",
        "plugin",
        "calendar.skin",
        "calendar.plugin",
        "headline0",
        "plugin3",
        "calendar.skin1",
        "calendar.plugin2",
    ] {
        let manifest = parsed_ghost(&[&format!("{prefix}.directory,x")]);
        assert_eq!(manifest.directory, "emo2", "本体の展開は続く");
        assert_eq!(manifest.companions, vec![]);
        assert_eq!(
            manifest.warnings,
            vec![ManifestWarning::UnsupportedCompanionKind {
                key: format!("{prefix}.directory")
            }],
            "{prefix}.directory の警告が違う"
        );
    }
}

/// 扱わない種別の付属キーも 1 件ずつ記録する。黙って落とさない（要件 3.13）。
#[test]
fn records_every_key_of_an_unhandled_companion_kind() {
    let manifest = parsed_ghost(&[
        "headline.directory,h",
        "headline.source.directory,h-src",
        "headline.refresh,1",
    ]);
    assert_eq!(
        manifest.warnings,
        vec![
            ManifestWarning::UnsupportedCompanionKind {
                key: "headline.directory".to_owned()
            },
            ManifestWarning::UnsupportedCompanionKind {
                key: "headline.refresh".to_owned()
            },
            ManifestWarning::UnsupportedCompanionKind {
                key: "headline.source.directory".to_owned()
            },
        ]
    );
}

// ---- ゴースト以外の同時インストール（要件 3.14） ----

/// `balloon`／`supplement` に同時インストールが書かれていれば警告して読み飛ばす（要件 3.14）。
///
/// 種別の判定が先に効くので、扱わない種別（`headline`）もここでは
/// 「ゴースト以外に書かれている」として記録する。理由を 2 つ並べても
/// 利用者の作業は変わらないので、1 件のキーに 1 件の警告で閉じる。
#[test]
fn warns_and_skips_companions_written_on_a_balloon_or_a_supplement() {
    for kind in ["balloon", "supplement"] {
        let manifest = parsed(&[
            &format!("type,{kind}"),
            "name,かくかく",
            "directory,kakukaku",
            "balloon0.directory,x",
            "headline.directory,y",
        ]);
        assert_eq!(manifest.companions, vec![], "{kind} に同梱は付かない");
        assert_eq!(
            manifest.warnings,
            vec![
                ManifestWarning::CompanionOnNonGhost {
                    key: "balloon0.directory".to_owned()
                },
                ManifestWarning::CompanionOnNonGhost {
                    key: "headline.directory".to_owned()
                },
            ],
            "type,{kind} の警告が違う"
        );
    }
}

/// ゴースト以外の同時インストールは、フォルダ名が壊れていても拒否にはしない（要件 3.14）。
///
/// 読まない値を検査すると、本体は正しいのに展開が止まる。
#[test]
fn a_broken_companion_directory_on_a_balloon_is_skipped_not_refused() {
    let manifest = parsed(&[
        "type,balloon",
        "name,かくかく",
        "directory,kakukaku",
        "balloon.directory,../escape",
    ]);
    assert_eq!(manifest.companions, vec![]);
    assert_eq!(
        manifest.warnings,
        vec![ManifestWarning::CompanionOnNonGhost {
            key: "balloon.directory".to_owned()
        }]
    );
}

// ---- 再インストール（要件 3.15） ----

/// `refresh` が `1` のときだけ全消去。それ以外は重ね置き（要件 3.15）。
#[test]
fn only_refresh_1_replaces_everything_else_overlays() {
    assert_eq!(
        parsed_ghost(&["refresh,1"]).existing,
        ExistingPolicy::Replace { keep: vec![] }
    );
    for value in ["0", "true", "yes", "2", "01", " ", ""] {
        assert_eq!(
            parsed_ghost(&[&format!("refresh,{value}")]).existing,
            ExistingPolicy::Overlay,
            "refresh,{value} が重ね置きにならない"
        );
    }
    assert_eq!(parsed_ghost(&[]).existing, ExistingPolicy::Overlay);
}

/// 除外マスクはコロンで分け、前後の空白を落とし、空の要素は捨てる（要件 3.15）。
#[test]
fn splits_the_mask_on_colons_trimming_and_dropping_empty_elements() {
    let manifest = parsed_ghost(&[
        "refresh,1",
        "refreshundeletemask,profile.txt: notes.txt ::readme.md:",
    ]);
    assert_eq!(
        manifest.existing,
        ExistingPolicy::Replace {
            keep: vec![
                "profile.txt".to_owned(),
                "notes.txt".to_owned(),
                "readme.md".to_owned(),
            ]
        }
    );
    assert_eq!(manifest.warnings, vec![]);
}

/// マスクの要素は 1 階層のファイル名。パス指定は警告して読み飛ばす（要件 3.15）。
#[test]
fn warns_and_drops_a_mask_element_that_is_not_a_one_level_file_name() {
    let manifest = parsed_ghost(&[
        "refresh,1",
        "refreshundeletemask,keep.txt:sub/deep.txt:..:CON",
    ]);
    assert_eq!(
        manifest.existing,
        ExistingPolicy::Replace {
            keep: vec!["keep.txt".to_owned()]
        }
    );
    assert_eq!(
        manifest.warnings,
        vec![
            ManifestWarning::InvalidMaskEntry {
                key: "refreshundeletemask".to_owned(),
                value: "sub/deep.txt".to_owned(),
            },
            ManifestWarning::InvalidMaskEntry {
                key: "refreshundeletemask".to_owned(),
                value: "..".to_owned(),
            },
            ManifestWarning::InvalidMaskEntry {
                key: "refreshundeletemask".to_owned(),
                value: "CON".to_owned(),
            },
        ]
    );
}

/// `refresh` が `1` でなければマスクは読まない。使わない値で警告を出さない（要件 3.15）。
#[test]
fn does_not_read_the_mask_when_refresh_is_not_1() {
    let manifest = parsed_ghost(&["refresh,0", "refreshundeletemask,sub/deep.txt"]);
    assert_eq!(manifest.existing, ExistingPolicy::Overlay);
    assert_eq!(
        manifest.warnings,
        vec![],
        "読まない値の不備は利用者の作業を変えない"
    );
}

/// 同時インストールのバルーンにも同じ規則が効く（要件 3.15）。
#[test]
fn reads_the_refresh_and_mask_of_each_companion() {
    let manifest = parsed_ghost(&[
        "balloon0.directory,zero",
        "balloon0.refresh,1",
        "balloon0.refreshundeletemask,keep.txt:sub/deep.txt",
        "balloon1.directory,one",
        "balloon1.refresh,0",
    ]);
    assert_eq!(
        manifest.companions[0].existing,
        ExistingPolicy::Replace {
            keep: vec!["keep.txt".to_owned()]
        }
    );
    assert_eq!(manifest.companions[1].existing, ExistingPolicy::Overlay);
    assert_eq!(
        manifest.warnings,
        vec![ManifestWarning::InvalidMaskEntry {
            key: "balloon0.refreshundeletemask".to_owned(),
            value: "sub/deep.txt".to_owned(),
        }]
    );
}

/// **本設計の決定**——`supplement` では `refresh` を読まない（要件 3.15・設計の裁定）。
///
/// サプリメントの重ね置き先はゴースト本体なので、全消去は本体ごと消しかねない。
/// 正典はこの組み合わせに沈黙しており、安全側（重ね置き）へ倒す。正典に反する
/// 判断ではなく、正典が決めていない穴を埋める判断なので、ukadoc を引いて
/// 「直す」対象ではない。
#[test]
fn ignores_refresh_for_a_supplement_by_design_decision_not_by_canon() {
    let manifest = parsed(&[
        "type,supplement",
        "name,追加辞書",
        "directory,extra",
        "refresh,1",
        "refreshundeletemask,keep.txt",
    ]);
    assert_eq!(
        manifest.existing,
        ExistingPolicy::Overlay,
        "全消去にはしない"
    );
    assert_eq!(
        manifest.warnings,
        vec![ManifestWarning::RefreshIgnoredForSupplement],
        "読み飛ばしたことは 1 度だけ記録する"
    );
}

/// マスクだけが書かれていても同じく記録する。黙って落とさない（要件 3.15）。
#[test]
fn records_the_ignored_refresh_for_a_supplement_that_only_writes_the_mask() {
    let manifest = parsed(&[
        "type,supplement",
        "name,追加辞書",
        "directory,extra",
        "refreshundeletemask,keep.txt",
    ]);
    assert_eq!(
        manifest.warnings,
        vec![ManifestWarning::RefreshIgnoredForSupplement]
    );
}

/// `refresh` を書かないサプリメントでは警告も出ない（上の警告が広すぎないことの対照）。
#[test]
fn a_supplement_without_refresh_gets_no_warning() {
    let manifest = parsed(&["type,supplement", "name,追加辞書", "directory,extra"]);
    assert_eq!(manifest.existing, ExistingPolicy::Overlay);
    assert_eq!(manifest.warnings, vec![]);
}

// ---- 知らないキー（要件 3.16） ----

/// 知らないキーは読み飛ばし、キー名を記録する。`bootghost` もここに入る（要件 3.16）。
#[test]
fn records_every_unknown_key_including_bootghost() {
    let manifest = parsed_ghost(&[
        "bootghost,emo2",
        "homeurl,https://example.invalid",
        "craftman,誰か",
    ]);
    assert_eq!(
        manifest.warnings,
        vec![
            ManifestWarning::IgnoredKey {
                key: "bootghost".to_owned()
            },
            ManifestWarning::IgnoredKey {
                key: "craftman".to_owned()
            },
            ManifestWarning::IgnoredKey {
                key: "homeurl".to_owned()
            },
        ],
        "記録はキーの名前順に並ぶ"
    );
}

/// 知っているキーは 1 件も記録に出ない（上の記録が広すぎないことの対照）。
#[test]
fn never_records_a_key_it_actually_reads() {
    let manifest = parsed(&[
        "charset,UTF-8",
        "type,ghost",
        "name,えも",
        "directory,emo2",
        "accept,pasta",
        "refresh,1",
        "refreshundeletemask,keep.txt",
        "balloon.directory,emo2-kakukaku",
        "balloon.source.directory,emo2-kakukaku",
        "balloon.refresh,1",
        "balloon.refreshundeletemask,keep.txt",
    ]);
    assert_eq!(manifest.warnings, vec![]);
}

/// 警告は順序つきの列で、同じ警告が 2 度は出ない（要件 3.13・3.15・3.16）。
///
/// 本体の再インストール → キーの名前順 → 同時インストールごと（探索の順）、の 3 段で並ぶ。
/// 「含む」で測ると、この順序も重複の有無も測れない。
#[test]
fn warnings_come_back_as_an_ordered_sequence_with_no_duplicates() {
    let manifest = parsed_ghost(&[
        "refresh,1",
        "refreshundeletemask,a/b",
        "zzz,unknown",
        "aaa,unknown",
        "headline.directory,h",
        "balloon.directory,emo2-kakukaku",
        "balloon.refresh,1",
        "balloon.refreshundeletemask,c/d",
    ]);
    assert_eq!(
        manifest.warnings,
        vec![
            ManifestWarning::InvalidMaskEntry {
                key: "refreshundeletemask".to_owned(),
                value: "a/b".to_owned(),
            },
            ManifestWarning::IgnoredKey {
                key: "aaa".to_owned()
            },
            ManifestWarning::UnsupportedCompanionKind {
                key: "headline.directory".to_owned()
            },
            ManifestWarning::IgnoredKey {
                key: "zzz".to_owned()
            },
            ManifestWarning::InvalidMaskEntry {
                key: "balloon.refreshundeletemask".to_owned(),
                value: "c/d".to_owned(),
            },
        ]
    );
}

/// 2 つの同梱が大小だけ違う同じ宛先を指す書庫は、2 つ目の宛先の名前の拒みとして
/// 書庫ごと断る（#7・要件 9.9）。黙って片方で上書きすると、もう片方の中身が消える。
#[test]
fn two_companions_naming_the_same_folder_refuse_the_archive() {
    assert_eq!(
        refused_ghost(&["balloon.directory,foo", "balloon0.directory,FOO"]),
        RefuseReason::InvalidDirectoryName {
            key: "balloon0.directory".to_owned(),
            value: "FOO".to_owned(),
        }
    );
}

// ---- 探す同梱の数の上限（要件 9.15 ⑶） ----

/// `balloon0` から `balloon{count-1}` までの宛先の行を作る。
fn numbered_balloons(count: usize) -> Vec<String> {
    (0..count)
        .map(|n| format!("balloon{n}.directory,b{n}"))
        .collect()
}

/// ちょうど上限の数の同梱は全部探し、上限による読み飛ばしは出ない。
#[test]
fn searches_every_companion_up_to_the_cap() {
    let lines = numbered_balloons(MAX_COMPANIONS);
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    let manifest = parsed_ghost(&borrowed);
    assert_eq!(manifest.companions.len(), 64);
    assert_eq!(manifest.warnings, vec![]);
}

/// 上限を 1 つ超えた同梱は探さず、その鍵を「探索で読まなかった」と記録する。
#[test]
fn records_the_companion_beyond_the_cap_as_not_searched() {
    let lines = numbered_balloons(MAX_COMPANIONS + 1);
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    let manifest = parsed_ghost(&borrowed);
    assert_eq!(manifest.companions.len(), 64);
    assert_eq!(
        manifest.companions.last().map(|c| c.key.as_str()),
        Some("balloon63")
    );
    assert_eq!(
        manifest.warnings,
        vec![ManifestWarning::CompanionNotSearched {
            key: "balloon64.directory".to_owned(),
        }]
    );
}
