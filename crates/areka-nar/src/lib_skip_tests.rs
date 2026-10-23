//! 取り出し元の無い同梱のバルーンを飛ばす振る舞いを、公開の入口（`open` → `install`）で
//! 通して確かめる兄弟テスト（areka-P0-emily-ghost-verification 要件 1.1〜1.6・1.8・7.1・7.2）。
//!
//! 要件 7.1 の 6 場面（無印・番号付き・2 つのうち片方・フォルダのエントリだけ・シェルの
//! `.nar`・全部在る）ごとに、入った物・`<ゴースト>/balloon/<置き場>/` が無いこと・成功・
//! 飛ばした列の中身・警告の記録の件数と欄を判定する。あわせて、`*.source.directory` を
//! 書かない同梱の鍵と値（要件 1.4）と、飛ばした同梱の置き場に先に在った中身が変わらない
//! こと（要件 1.6）を見る。
//!
//! 記録は `log-capture-kit` で捕まえる。根と書庫の置き場は [`WorkDir`]（ワークツリーの
//! `target\` の下）が配る。

use super::tests::{files_under, tree};
use super::*;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{NarBuilder, WorkDir, install_txt};
use std::fs;

/// 飛ばした 1 件ごとに出る警告の文言（設計「書庫の入口の記録」の逐語）。
const SKIPPED: &str =
    "[areka_nar] companion balloon skipped: its source folder is not in the archive";
/// 成功の記録の文言。
const INSTALLED: &str = "[areka_nar] installed";

/// 1 回の走行の結果。
struct Run {
    /// 読んだ `.nar` のパス（記録の `archive` の欄と突き合わせる）。
    archive: PathBuf,
    outcome: Result<InstallOutcome, NarError>,
    records: Vec<CapturedEvent>,
}

impl Run {
    fn outcome(&self) -> &InstallOutcome {
        self.outcome.as_ref().expect("インストールは成功する")
    }

    fn with_message(&self, message: &str) -> Vec<&CapturedEvent> {
        self.records
            .iter()
            .filter(|event| event.message() == message)
            .collect()
    }

    fn at(&self, level: tracing::Level) -> usize {
        self.records
            .iter()
            .filter(|event| event.level == level)
            .count()
    }

    /// 成功の記録 1 件の `skipped` の欄。
    fn installed_skipped_field(&self) -> String {
        let installed = self.with_message(INSTALLED);
        assert_eq!(installed.len(), 1, "成功の記録は 1 件: {:?}", self.records);
        installed[0]
            .field("skipped")
            .unwrap_or_else(|| panic!("成功の記録に skipped の欄が無い: {:?}", installed[0]))
            .to_owned()
    }

    /// 飛ばした警告の記録が `expected` と同じ並び・同じ欄で出ている。
    fn assert_skip_records(&self, expected: &[SkippedCompanion]) {
        let warned = self.with_message(SKIPPED);
        assert_eq!(
            warned.len(),
            expected.len(),
            "飛ばした 1 件につき警告 1 件: {:?}",
            self.records
        );
        for (event, skip) in warned.iter().zip(expected) {
            assert_eq!(event.level, tracing::Level::WARN);
            assert_eq!(
                event.field_names_sorted(),
                ["archive", "directory", "key", "message", "source"],
                "警告の欄"
            );
            assert_eq!(
                event.field("archive"),
                Some(self.archive.display().to_string().as_str()),
                "警告に .nar のパス（ファイル名を含む）が載る"
            );
            assert_eq!(event.field_str("key"), Some(skip.key.as_str()));
            assert_eq!(event.field_str("source"), Some(skip.source.as_str()));
            assert_eq!(event.field_str("directory"), Some(skip.directory.as_str()));
        }
        assert_eq!(
            self.installed_skipped_field(),
            expected.len().to_string(),
            "成功の記録の飛ばした件数"
        );
        assert_eq!(
            self.at(tracing::Level::ERROR),
            0,
            "成功の一周に error の記録"
        );
    }
}

/// 書庫を根とは別の置き場に置き、記録を捕まえながら `open` → `install` を通す。
fn run(nar: &NarBuilder, shelf: &WorkDir, root: &Path, target_ghost: Option<&str>) -> Run {
    let archive = shelf.path().join("skip-test.nar");
    fs::write(&archive, nar.bytes()).expect("固定入力を置ける");
    let (outcome, records) = capture(|| {
        NarArchive::open(&archive)?.install(&InstallRequest {
            roots: &SingleRoot(root),
            target_ghost,
        })
    });
    Run {
        archive,
        outcome,
        records,
    }
}

/// ゴースト本体 `tester` に、`companions` の行を足した書庫。本体のファイルは 1 つ。
fn ghost(companions: &[&str]) -> NarBuilder {
    let mut lines = vec!["type,ghost", "name,Tester", "directory,tester"];
    lines.extend_from_slice(companions);
    NarBuilder::new()
        .file("install.txt", &install_txt(&lines))
        .done()
        .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
        .done()
}

fn skipped(directory: &str, key: &str, source: &str) -> SkippedCompanion {
    SkippedCompanion {
        directory: directory.to_owned(),
        key: key.to_owned(),
        source: source.to_owned(),
    }
}

/// 入った物の (種類, 名前) の列。
fn installed(outcome: &InstallOutcome) -> Vec<(ElementKind, &str)> {
    outcome
        .installed
        .iter()
        .map(|element| (element.kind.clone(), element.name.as_str()))
        .collect()
}

/// ゴースト本体だけが入ったときの根のファイルの列。
fn body_files() -> Vec<String> {
    vec![
        "ghost/tester/ghost/master/descript.txt".to_owned(),
        "ghost/tester/install.txt".to_owned(),
    ]
}

// ---- 6 場面（要件 7.1） ----

/// ⑴ 無印の同梱の取り出し元が無い → ゴーストだけが入り、バルーンは置かれず、警告 1 件。
#[test]
fn a_plain_companion_without_its_source_is_skipped_and_warned() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    let run = run(
        &ghost(&[
            "balloon.directory,kaku",
            "balloon.source.directory,src-kaku",
        ]),
        &shelf,
        root,
        None,
    );

    let expected = [skipped("kaku", "balloon.source.directory", "src-kaku")];
    assert_eq!(installed(run.outcome()), [(ElementKind::Ghost, "Tester")]);
    assert_eq!(run.outcome().skipped, expected);
    assert!(
        !root.join("ghost/tester/balloon/kaku").exists(),
        "飛ばした置き場が作られた"
    );
    assert_eq!(files_under(root), body_files());
    run.assert_skip_records(&expected);
}

/// ⑵ 番号付き（`balloon0`）の同梱の取り出し元が無い → 同上（鍵は `balloon0.source.directory`）。
#[test]
fn a_numbered_companion_without_its_source_is_skipped_and_warned() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    let run = run(
        &ghost(&[
            "balloon0.directory,b0",
            "balloon0.source.directory,extra\\s0",
        ]),
        &shelf,
        root,
        None,
    );

    // 値は書かれたまま（区切りの `\` も変えない）。
    let expected = [skipped("b0", "balloon0.source.directory", "extra\\s0")];
    assert_eq!(installed(run.outcome()), [(ElementKind::Ghost, "Tester")]);
    assert_eq!(run.outcome().skipped, expected);
    assert!(
        !root.join("ghost/tester/balloon/b0").exists(),
        "飛ばした置き場が作られた"
    );
    assert_eq!(files_under(root), body_files());
    run.assert_skip_records(&expected);
}

/// ⑶ 同梱が 2 つで片方だけ取り出し元が無い → 在るほうは今どおり置かれ、警告 1 件。
#[test]
fn only_the_companion_without_its_source_is_skipped() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    let nar = ghost(&[
        "balloon.directory,kaku",
        "balloon.source.directory,kaku",
        "balloon0.directory,b0",
        "balloon0.source.directory,s0",
    ])
    .file("kaku/balloon.txt", b"kaku\r\n")
    .done();
    let run = run(&nar, &shelf, root, None);

    let expected = [skipped("b0", "balloon0.source.directory", "s0")];
    assert_eq!(
        installed(run.outcome()),
        [
            (ElementKind::Ghost, "Tester"),
            (ElementKind::Balloon, "kaku")
        ]
    );
    assert_eq!(run.outcome().skipped, expected);
    assert_eq!(
        fs::read(root.join("ghost/tester/balloon/kaku/balloon.txt")).expect("在るほうは置かれる"),
        b"kaku\r\n"
    );
    assert!(
        !root.join("ghost/tester/balloon/b0").exists(),
        "飛ばした置き場が作られた"
    );
    let mut files = body_files();
    files.insert(0, "ghost/tester/balloon/kaku/balloon.txt".to_owned());
    assert_eq!(files_under(root), files);
    run.assert_skip_records(&expected);
}

/// ⑷ 取り出し元のフォルダのエントリだけが在り配下が空 → 無いのと同じ扱い（要件 1.2）。
#[test]
fn a_source_with_only_its_folder_entry_is_skipped_like_an_absent_one() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    let nar = ghost(&[
        "balloon.directory,kaku",
        "balloon.source.directory,src-kaku",
    ])
    .dir("src-kaku/");
    let run = run(&nar, &shelf, root, None);

    let expected = [skipped("kaku", "balloon.source.directory", "src-kaku")];
    assert_eq!(installed(run.outcome()), [(ElementKind::Ghost, "Tester")]);
    assert_eq!(run.outcome().skipped, expected);
    assert!(
        !root.join("ghost/tester/balloon/kaku").exists(),
        "飛ばした置き場が作られた"
    );
    assert!(
        !root.join("ghost/tester/src-kaku").exists(),
        "取り出し元のフォルダのエントリが本体へ入った"
    );
    assert_eq!(files_under(root), body_files());
    run.assert_skip_records(&expected);
}

/// ⑸ シェルの `.nar`（宛先のゴーストあり）で同梱の取り出し元が無い → シェルが入り、警告 1 件。
#[test]
fn a_shell_with_a_missing_companion_source_installs_the_shell() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    fs::create_dir_all(root.join("ghost/host")).expect("宛先のゴーストを置ける");
    let nar = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "type,shell",
                "name,Shell",
                "directory,test-shell",
                "balloon.directory,kaku",
                "balloon.source.directory,src-kaku",
            ]),
        )
        .done()
        .file("surface0.png", b"png")
        .done();
    let run = run(&nar, &shelf, root, Some("host"));

    let expected = [skipped("kaku", "balloon.source.directory", "src-kaku")];
    assert_eq!(installed(run.outcome()), [(ElementKind::Shell, "Shell")]);
    assert_eq!(run.outcome().skipped, expected);
    assert!(
        !root.join("ghost/host/balloon/kaku").exists(),
        "飛ばした置き場が作られた"
    );
    assert_eq!(
        files_under(root),
        [
            "ghost/host/shell/test-shell/install.txt".to_owned(),
            "ghost/host/shell/test-shell/surface0.png".to_owned(),
        ]
    );
    run.assert_skip_records(&expected);
}

/// ⑹ 取り出し元が全部在る → 置き方・結果・記録が今と同じ（飛ばした列は空・警告 0 件・
/// 成功の記録の飛ばした件数 0）。
#[test]
fn companions_with_all_sources_install_as_before_with_no_skip_record() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    let nar = ghost(&[
        "balloon.directory,kaku",
        "balloon.source.directory,kaku",
        "balloon0.directory,b0",
        "balloon0.source.directory,s0",
    ])
    .file("kaku/balloon.txt", b"kaku\r\n")
    .done()
    .file("s0/balloon.txt", b"s0\r\n")
    .done();
    let run = run(&nar, &shelf, root, None);

    assert_eq!(
        installed(run.outcome()),
        [
            (ElementKind::Ghost, "Tester"),
            (ElementKind::Balloon, "kaku"),
            (ElementKind::Balloon, "b0"),
        ]
    );
    assert_eq!(run.outcome().skipped, []);
    let mut files = vec![
        "ghost/tester/balloon/b0/balloon.txt".to_owned(),
        "ghost/tester/balloon/kaku/balloon.txt".to_owned(),
    ];
    files.extend(body_files());
    assert_eq!(files_under(root), files);
    run.assert_skip_records(&[]);
    assert_eq!(run.at(tracing::Level::WARN), 0, "全部在る場面の警告は 0 件");
}

// ---- あわせて見る 2 つ ----

/// `*.source.directory` を書かない同梱で取り出し元が無い → 鍵は `balloon.directory`・
/// 値は置き場の名前（要件 1.4）。
#[test]
fn without_a_source_key_the_directory_key_and_name_are_recorded() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    let run = run(&ghost(&["balloon.directory,kaku"]), &shelf, root, None);

    let expected = [skipped("kaku", "balloon.directory", "kaku")];
    assert_eq!(installed(run.outcome()), [(ElementKind::Ghost, "Tester")]);
    assert_eq!(run.outcome().skipped, expected);
    assert!(!root.join("ghost/tester/balloon/kaku").exists());
    run.assert_skip_records(&expected);
}

/// 飛ばした同梱の置き場に先に在った中身は、入れた後も 1 バイトも変わらない（要件 1.6）。
#[test]
fn an_existing_folder_at_a_skipped_home_is_left_byte_identical() {
    let (shelf, work) = (WorkDir::new().unwrap(), WorkDir::new().unwrap());
    let root = work.path();
    let home = root.join("ghost/tester/balloon/kaku");
    fs::create_dir_all(home.join("sub")).expect("先の中身を置ける");
    fs::write(home.join("descript.txt"), b"old descript\r\n").expect("先の中身を置ける");
    fs::write(home.join("sub/s0.png"), b"old png").expect("先の中身を置ける");
    let before = tree(&home);
    assert_eq!(
        before.len(),
        3,
        "較正: 先の中身が木に映っている: {before:?}"
    );

    let run = run(
        &ghost(&[
            "balloon.directory,kaku",
            "balloon.source.directory,src-kaku",
        ]),
        &shelf,
        root,
        None,
    );

    let expected = [skipped("kaku", "balloon.source.directory", "src-kaku")];
    assert_eq!(installed(run.outcome()), [(ElementKind::Ghost, "Tester")]);
    assert_eq!(tree(&home), before, "飛ばした置き場の中身が変わった");
    run.assert_skip_records(&expected);
}
