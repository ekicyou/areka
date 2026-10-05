//! `plan` の兄弟テストの続き——同梱バルーンの取り出し元が `extra\bal1` のような
//! 階層付きの相対パスのときの配置（要件 2.2〜2.5・2.8・5.6）。
//!
//! 助手は [`super`]（`plan_tests.rs`）から借りる。どの場面も `install.txt` の読み
//! （`parse_manifest`）から配置（`build_plan`）まで通す。取り出し元の値は読み手が
//! `/` 区切りへ正規化した形で届くので、`install.txt` には `\` 区切りで書く。

use super::*;

// ---- 助手 ----

/// ゴースト本体 `test-ghost` の `install.txt` に、同梱の行を足したもの。
fn ghost_install_txt(companion_lines: &[&str]) -> Vec<u8> {
    let mut lines = vec![
        "charset,UTF-8",
        "type,ghost",
        "name,emo",
        "directory,test-ghost",
    ];
    lines.extend_from_slice(companion_lines);
    install_txt(&lines)
}

/// 取り出し元が `extra\bal1` の同梱 1 件を持つ `install.txt`。
fn nested_install_txt() -> Vec<u8> {
    ghost_install_txt(&[
        "balloon.directory,test-balloon",
        "balloon.source.directory,extra\\bal1",
    ])
}

/// 根だけを借りた要求で計画を組む。
fn free_request(root: &WorkDir) -> InstallRequest<'_> {
    InstallRequest {
        root: root.path(),
        target_ghost: None,
    }
}

// ---- 固定入力 ----

/// 取り出し元 `extra/bal1` の中身と、兄弟のファイル `extra/readme.txt` を持つ書庫。
///
/// 途中のフォルダ `extra/` と取り出し元 `extra/bal1/` のフォルダのエントリも持つ。
fn nested_source_archive() -> NarBuilder {
    NarBuilder::new()
        .file("install.txt", &nested_install_txt())
        .done()
        .dir("extra")
        .file("extra/readme.txt", b"readme")
        .done()
        .dir("extra/bal1")
        .file("extra/bal1/descript.txt", b"balloon descript")
        .done()
        .file("extra/bal1/sub/s0.png", b"s0")
        .done()
}

/// `extra/` の中身が `bal1/` だけの書庫。フォルダのエントリを持つ形と持たない形を作る。
fn passing_through_archive(with_folder_entries: bool) -> NarBuilder {
    let builder = NarBuilder::new()
        .file("install.txt", &nested_install_txt())
        .done();
    let builder = if with_folder_entries {
        builder.dir("extra").dir("extra/bal1")
    } else {
        builder
    };
    builder
        .file("extra/bal1/descript.txt", b"balloon descript")
        .done()
        .file("ghost/master/descript.txt", b"ghost descript")
        .done()
}

/// `install.txt` に `Extra\BAL1` と書き、書庫では `extra/bal1/` と綴る書庫。
fn case_differs_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &ghost_install_txt(&[
                "balloon.directory,test-balloon",
                "balloon.source.directory,Extra\\BAL1",
            ]),
        )
        .done()
        .file("extra/bal1/descript.txt", b"balloon descript")
        .done()
}

/// 取り出し元 `extra/bal1` の隣に、綴りが前方一致するだけの `extra/bal10/` を持つ書庫。
fn longer_sibling_archive() -> NarBuilder {
    NarBuilder::new()
        .file("install.txt", &nested_install_txt())
        .done()
        .file("extra/bal1/descript.txt", b"balloon descript")
        .done()
        .file("extra/bal10/x.png", b"x")
        .done()
}

/// 取り出し元 `extra` と `extra/bal1` の 2 つの同梱が重なる書庫。
fn overlapping_sources_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &ghost_install_txt(&[
                "balloon.directory,outer-balloon",
                "balloon.source.directory,extra",
                "balloon0.directory,inner-balloon",
                "balloon0.source.directory,extra\\bal1",
            ]),
        )
        .done()
        .file("extra/bal1/descript.txt", b"balloon descript")
        .done()
}

// ---- 階層付きの取り出し元（要件 2.2・2.4） ----

/// 取り出し元の全ての段が剥がれ、配下が宛先の配下へそのままの相対の位置で入る（要件 2.2）。
#[test]
fn a_nested_source_is_stripped_of_every_segment() {
    let root = root_with_ghost("other");
    let plan = plan_of(nested_source_archive(), &free_request(&root));

    assert_eq!(plan.len(), 2, "本体と同梱の 2 配置になる");
    let companion = &plan[1];
    assert_eq!(companion.kind, ElementKind::Balloon);
    assert_eq!(companion.name, "test-balloon");
    assert_eq!(
        companion.destination,
        root.path().join("balloon").join("test-balloon"),
        "宛先に取り出し元の値が継ぎ足されている"
    );
    assert_eq!(companion.target_ghost, None);
    assert_eq!(companion.existing, ExistingPolicy::Overlay);
    assert!(!companion.skip_top_level_install_txt);
    assert_eq!(
        companion.files,
        vec![(4, "descript.txt".to_owned()), (5, "sub/s0.png".to_owned()),]
    );
    assert_eq!(companion.dirs, vec!["sub"]);
}

/// 取り出し元の兄弟のファイルは本体に残り、取り出し元の中身は本体に 1 件も無い（要件 2.4）。
///
/// 途中のフォルダ `extra/` のエントリは本体から除かれるが、兄弟のファイルの親として
/// 同じフォルダが作られる。
#[test]
fn a_sibling_of_the_nested_source_stays_with_the_body() {
    let root = root_with_ghost("other");
    let plan = plan_of(nested_source_archive(), &free_request(&root));

    let body = &plan[0];
    assert_eq!(
        body.files,
        vec![
            (0, "install.txt".to_owned()),
            (2, "extra/readme.txt".to_owned()),
        ]
    );
    assert_eq!(body.dirs, vec!["extra"]);
    let companion_entries: Vec<usize> = plan[1].files.iter().map(|(index, _)| *index).collect();
    for (index, relative) in &body.files {
        assert!(
            !companion_entries.contains(index),
            "エントリ {index}（{relative}）が本体と同梱の両方に居る"
        );
    }
}

/// 途中のフォルダの配下に本体のものが無ければ、本体に途中のフォルダは作られない。
/// 書庫がフォルダのエントリを持つかどうかで結果が変わらない（要件 2.4）。
#[test]
fn a_folder_on_the_way_to_the_source_is_not_made_on_the_body_side() {
    let root = root_with_ghost("other");
    let request = free_request(&root);
    let with_entries = plan_of(passing_through_archive(true), &request);
    let without_entries = plan_of(passing_through_archive(false), &request);

    let body_of = |plan: &[Placement]| {
        (
            relative_files(&plan[0])
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            plan[0].dirs.clone(),
        )
    };
    let (files, dirs) = body_of(&with_entries);
    assert_eq!(files, vec!["install.txt", "ghost/master/descript.txt"]);
    assert_eq!(dirs, vec!["ghost", "ghost/master"]);
    assert_eq!(
        body_of(&with_entries),
        body_of(&without_entries),
        "フォルダのエントリの有無で本体の配置が変わった"
    );
    for plan in [&with_entries, &without_entries] {
        assert_eq!(relative_files(&plan[1]), vec!["descript.txt"]);
        assert!(plan[1].dirs.is_empty());
    }
}

// ---- 突き合わせ（要件 2.3） ----

/// `install.txt` の `Extra\BAL1` が書庫の `extra/bal1/` に当たる（各段を大小を無視して比べる）。
#[test]
fn every_segment_of_the_source_matches_ignoring_case() {
    let root = root_with_ghost("other");
    let plan = plan_of(case_differs_archive(), &free_request(&root));

    assert_eq!(
        relative_files(&plan[0]),
        vec!["install.txt"],
        "大小違いの取り出し元が本体に残っている"
    );
    assert!(
        plan[0].dirs.is_empty(),
        "本体に {:?} が作られる",
        plan[0].dirs
    );
    assert_eq!(
        relative_files(&plan[1]),
        vec!["descript.txt"],
        "大小違いの取り出し元から取り出せていない"
    );
}

/// 段ごとに比べるので、`extra/bal10/x.png` は `extra/bal1` の配下ではなく本体に残る（要件 2.3・2.4）。
#[test]
fn a_segment_with_a_longer_name_is_not_under_the_source() {
    let root = root_with_ghost("other");
    let plan = plan_of(longer_sibling_archive(), &free_request(&root));

    assert_eq!(
        relative_files(&plan[0]),
        vec!["install.txt", "extra/bal10/x.png"]
    );
    assert_eq!(plan[0].dirs, vec!["extra", "extra/bal10"]);
    assert_eq!(relative_files(&plan[1]), vec!["descript.txt"]);
    assert!(plan[1].dirs.is_empty());
}

// ---- 配下が空（要件 2.5） ----

/// 階層付きの取り出し元の配下が書庫に無ければ、鍵と正規化した取り出し元を載せて断る。
///
/// 書庫に無い形・フォルダのエントリだけが在る形・綴りが前方一致する兄弟だけが在る形の 3 つ。
#[test]
fn refuses_a_nested_source_with_nothing_under_it() {
    let root = root_with_ghost("other");
    let request = free_request(&root);
    let shapes = [
        (
            "書庫に無い",
            NarBuilder::new()
                .file("install.txt", &nested_install_txt())
                .done()
                .file("extra/readme.txt", b"readme")
                .done(),
        ),
        (
            "フォルダのエントリだけ",
            NarBuilder::new()
                .file("install.txt", &nested_install_txt())
                .done()
                .dir("extra")
                .dir("extra/bal1"),
        ),
        (
            "前方一致する兄弟だけ",
            NarBuilder::new()
                .file("install.txt", &nested_install_txt())
                .done()
                .file("extra/bal10/x.png", b"x")
                .done(),
        ),
    ];
    for (label, archive) in shapes {
        assert_eq!(
            refusal_of(archive, &request),
            RefuseReason::CompanionSourceMissing {
                key: "balloon.source.directory".to_owned(),
                source_directory: "extra/bal1".to_owned(),
            },
            "{label} の形が断られない"
        );
    }
}

// ---- 重なる取り出し元（要件 2.8・1.7） ----

/// `extra` と `extra/bal1` の 2 つの同梱は、それぞれ自分の配下の全てを受け取る。
/// 本体には何も残らず、同梱は探索の順（無印 → `balloon0`）で並ぶ。
#[test]
fn overlapping_sources_each_receive_everything_under_them() {
    let root = root_with_ghost("other");
    let plan = plan_of(overlapping_sources_archive(), &free_request(&root));

    assert_eq!(plan.len(), 3);
    assert_eq!(relative_files(&plan[0]), vec!["install.txt"]);
    assert!(
        plan[0].dirs.is_empty(),
        "本体に {:?} が作られる",
        plan[0].dirs
    );

    assert_eq!(plan[1].name, "outer-balloon");
    assert_eq!(relative_files(&plan[1]), vec!["bal1/descript.txt"]);
    assert_eq!(plan[1].dirs, vec!["bal1"]);

    assert_eq!(plan[2].name, "inner-balloon");
    assert_eq!(relative_files(&plan[2]), vec!["descript.txt"]);
    assert!(plan[2].dirs.is_empty());
}

// ---- 根の外へ出ない（要件 5.6・8.2） ----

/// 上の受理される全ての計画の全ての宛先・ファイル・フォルダが根の配下に収まる。
///
/// 歩いた数を固定する。計画が痩せても恒真で緑になる道を塞ぐため。
#[test]
fn every_path_of_every_nested_source_plan_stays_under_the_root() {
    let root = root_with_ghost("other");
    let request = free_request(&root);
    let accepted = [
        ("nested-source", nested_source_archive()),
        (
            "passing-through-with-entries",
            passing_through_archive(true),
        ),
        (
            "passing-through-without-entries",
            passing_through_archive(false),
        ),
        ("case-differs", case_differs_archive()),
        ("longer-sibling", longer_sibling_archive()),
        ("overlapping-sources", overlapping_sources_archive()),
    ];
    let mut total = 0;
    for (label, archive) in accepted {
        let plan = plan_of(archive, &request);
        for placement in &plan {
            assert!(
                !placement.files.is_empty(),
                "{label} の配置 {:?} が 1 件もファイルを置かない",
                placement.destination
            );
        }
        let (outside, checked) = escapes(root.path(), &plan);
        assert!(outside.is_empty(), "{label} が根の外へ出る: {outside:?}");
        total += checked;
    }
    assert_eq!(
        total, 40,
        "数えた道筋の数が変わった（数を固定しないと、計画が痩せても恒真で緑になる）"
    );
}
