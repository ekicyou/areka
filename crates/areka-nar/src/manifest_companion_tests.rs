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
        }]
    );
    assert_eq!(manifest.warnings, vec![]);
}

/// `*.source.directory` が無ければ `*.directory` と同じ値にする（要件 3.12）。
#[test]
fn defaults_the_companion_source_directory_to_the_directory() {
    let manifest = parsed_ghost(&["balloon.directory,emo2-kakukaku"]);
    assert_eq!(manifest.companions.len(), 1);
    assert_eq!(manifest.companions[0].source_directory, "emo2-kakukaku");
}

/// 空の `*.source.directory` も「無い」と同じ扱い（要件 3.12）。
#[test]
fn an_empty_companion_source_directory_falls_back_to_the_directory() {
    let manifest = parsed_ghost(&[
        "balloon.directory,emo2-kakukaku",
        "balloon.source.directory,",
    ]);
    assert_eq!(manifest.companions[0].source_directory, "emo2-kakukaku");
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

/// `balloonN` の N は数字列。並びはキーの名前順で決まる（要件 3.12）。
///
/// `balloon10` が `balloon2` より前に来るのは名前順だから。数の順ではないが、
/// 同じ `install.txt` からは必ず同じ並びが出る。
#[test]
fn reads_numbered_balloon_companions_in_key_order() {
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
    assert_eq!(keys, vec!["balloon", "balloon0", "balloon10", "balloon2"]);
    assert_eq!(manifest.warnings, vec![]);
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

/// `balloon0` は通る（上の境界の反対側）。
#[test]
fn balloon_followed_by_digits_is_a_companion() {
    for key in ["balloon", "balloon0", "balloon9", "balloon10", "balloon007"] {
        let manifest = parsed_ghost(&[&format!("{key}.directory,x")]);
        assert_eq!(manifest.companions.len(), 1, "{key}.directory が読まれない");
        assert_eq!(manifest.companions[0].key, key);
        assert_eq!(manifest.warnings, vec![]);
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

/// 同時インストールのフォルダ名も 1 階層でなければならない（要件 3.9）。
#[test]
fn refuses_a_companion_directory_that_is_not_a_one_level_name() {
    assert_eq!(
        refused_ghost(&["balloon0.directory,../escape"]),
        RefuseReason::InvalidDirectoryName {
            key: "balloon0.directory".to_owned(),
            value: "../escape".to_owned(),
        }
    );
    assert_eq!(
        refused_ghost(&["balloon.directory,"]),
        RefuseReason::InvalidDirectoryName {
            key: "balloon.directory".to_owned(),
            value: String::new(),
        },
        "空のフォルダ名も 1 階層の名前ではない"
    );
}

/// 取り出し元のフォルダ名も 1 階層でなければならない（要件 3.9）。
#[test]
fn refuses_a_companion_source_directory_that_is_not_a_one_level_name() {
    assert_eq!(
        refused_ghost(&[
            "balloon.directory,emo2-kakukaku",
            "balloon.source.directory,sub/kakukaku",
        ]),
        RefuseReason::InvalidDirectoryName {
            key: "balloon.source.directory".to_owned(),
            value: "sub/kakukaku".to_owned(),
        }
    );
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

/// 警告は順序つきの列で、同じキーに 2 度は出ない（要件 3.13・3.15・3.16）。
///
/// 本体の再インストール → キーの名前順 → 同時インストールの名前順、の 3 段で並ぶ。
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
