//! 見本 `fold-samples` の写しの式と書き先の順のテスト。
//!
//! 引数の読み（`parse_args`）の 1 本を除き、どれも [`WorkDir`] の下に組んだ偽のリポジトリ根で
//! `fold_one` を回し、本物の `vendors/` には触らない。`git` を呼ぶ `run` は通らない（追跡の
//! 一覧は引数で渡す）。

use super::*;
use sample_ghost_kit::{WorkDir, install_txt};

/// 偽の根の中の、同梱バルーンつきの小さな展開形（`install.txt`・本体・同梱バルーンの取り出し元の 3 つを持つ形）。返すのは
/// 展開形のフォルダと、その追跡ファイルの一覧。
fn ghost_with_companion(fake_root: &Path) -> (PathBuf, Vec<String>) {
    ghost_with_companion_at(fake_root, "tiny-balloon-src", "tiny-balloon-src")
}

/// [`ghost_with_companion`] の取り出し元を選べる形。`declared` は `install.txt` の
/// `balloon.source.directory` の値、`on_disk` は追跡ファイルの側の綴り（大小が違ってよい）。
fn ghost_with_companion_at(
    fake_root: &Path,
    declared: &str,
    on_disk: &str,
) -> (PathBuf, Vec<String>) {
    let source = fake_root.join("unfolded").join("tiny");
    let files: [(String, Vec<u8>); 4] = [
        (
            "install.txt".to_owned(),
            install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,ためしゴースト",
                "directory,tiny-ghost",
                "balloon.directory,tiny-balloon",
                &format!("balloon.source.directory,{declared}"),
            ]),
        ),
        (
            "ghost/master/descript.txt".to_owned(),
            b"ghost descript".to_vec(),
        ),
        (
            format!("{on_disk}/descript.txt"),
            b"balloon descript".to_vec(),
        ),
        (format!("{on_disk}/balloon/s0.png"), b"s0".to_vec()),
    ];
    let mut tracked = Vec::new();
    for (relative, bytes) in files {
        let path = source.join(relative.replace('/', MAIN_SEPARATOR_STR));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        tracked.push(relative);
    }
    (source, tracked)
}

#[test]
fn a_ghost_with_a_companion_balloon_round_trips_without_mismatches() {
    let work = WorkDir::new().unwrap();
    let fake_root = work.path().join("repo");
    let (source, tracked) = ghost_with_companion(&fake_root);
    let scratch = fake_root.join("target").join("fold-samples");

    let mismatches = fold_one(&fake_root, &scratch, &source, &tracked, false).unwrap();

    assert_eq!(mismatches, 0, "同梱バルーンの写し先が展開器の置き方と違う");
}

#[test]
fn fold_one_never_writes_the_vendored_archive() {
    let work = WorkDir::new().unwrap();
    let fake_root = work.path().join("repo");
    let (source, tracked) = ghost_with_companion(&fake_root);
    let scratch = fake_root.join("target").join("fold-samples");
    let vendored = vendored_nar(&fake_root, "tiny");
    fs::create_dir_all(vendored.parent().unwrap()).unwrap();
    let sentinel = b"sentinel: the previous good archive".to_vec();
    fs::write(&vendored, &sentinel).unwrap();

    fold_one(&fake_root, &scratch, &source, &tracked, false).unwrap();

    assert_eq!(
        fs::read(&vendored).unwrap(),
        sentinel,
        "fold_one が確かめの前に vendors/ の .nar を書き替えた"
    );
    assert!(
        folded_nar(&scratch, "tiny").is_file(),
        "畳んだ .nar が作業の置き場に無い"
    );
}

#[test]
fn check_only_counts_a_one_byte_difference_from_the_vendored_archive() {
    let work = WorkDir::new().unwrap();
    let fake_root = work.path().join("repo");
    let (source, tracked) = ghost_with_companion(&fake_root);
    let folded = fake_root.join("target").join("fold-first");
    fold_one(&fake_root, &folded, &source, &tracked, false).unwrap();
    let vendored = vendored_nar(&fake_root, "tiny");
    fs::create_dir_all(vendored.parent().unwrap()).unwrap();
    fs::copy(folded_nar(&folded, "tiny"), &vendored).unwrap();
    // 書き替える前は 0（0 の側を確かめないと、何でも不一致に数える形を見逃す）。
    let same = fake_root.join("target").join("check-same");
    assert_eq!(
        fold_one(&fake_root, &same, &source, &tracked, true).unwrap(),
        0
    );
    // 元のツリーの 1 ファイルを 1 バイトだけ変え、`vendors/` の書庫と食い違わせる。
    fs::write(
        source
            .join("tiny-balloon-src")
            .join("balloon")
            .join("s0.png"),
        b"s1",
    )
    .unwrap();

    let scratch = fake_root.join("target").join("fold-samples");
    let mismatches = fold_one(&fake_root, &scratch, &source, &tracked, true).unwrap();

    assert!(mismatches >= 1, "1 バイト違う書庫を --check が一致と数えた");
}

#[test]
fn unknown_arguments_are_refused_not_ignored() {
    let args = |words: &[&str]| {
        words
            .iter()
            .map(|word| (*word).to_owned())
            .collect::<Vec<_>>()
    };

    let refused = parse_args(&args(&["--from", "x", "--chek"])).unwrap_err();
    assert_eq!(refused.to_string(), "unknown argument --chek");

    let (from, check_only) = parse_args(&args(&["--from", "x", "--check"])).unwrap();
    assert_eq!((from.as_deref(), check_only), (Some("x"), true));
}

/// 2 段の取り出し元を、段ごとに大小を畳んで照らす（大小の違う非 ASCII の名前を含む・
/// audit-test-kits かけ直し 5 回目 E1）。1 段目だけを ASCII で照らす形は、取り出し元の配下を
/// 本体の置き場へ写して不一致を数える。
#[test]
fn a_two_level_companion_source_with_other_case_round_trips_without_mismatches() {
    let work = WorkDir::new().unwrap();
    let fake_root = work.path().join("repo");
    let (source, tracked) =
        ghost_with_companion_at(&fake_root, "Src/ｂａｌｌｏｏｎ", "src/ＢＡＬＬＯＯＮ");
    let scratch = fake_root.join("target").join("fold-samples");

    let mismatches = fold_one(&fake_root, &scratch, &source, &tracked, false).unwrap();

    assert_eq!(
        mismatches, 0,
        "2 段の取り出し元の写し先が展開器の置き方と違う"
    );
}

/// 追跡ファイルが 0 件なら、panic でなく ASCII の文の `Err` で断る（終了コード 1・
/// audit-test-kits かけ直し 5 回目 E2）。
#[test]
fn no_tracked_files_is_an_ascii_error_not_a_panic() {
    let work = WorkDir::new().unwrap();
    let fake_root = work.path().join("repo");
    let (source, _) = ghost_with_companion(&fake_root);
    let scratch = fake_root.join("target").join("fold-samples");

    let refused = fold_one(&fake_root, &scratch, &source, &[], false)
        .unwrap_err()
        .to_string();

    assert!(
        refused.starts_with("no tracked files under ")
            && refused.ends_with(" (git add the unfolded tree first)"),
        "{refused}"
    );
}

/// 検体に居ない種別は、panic でなく ASCII の `unsupported kind` の `Err` で断る
/// （audit-test-kits かけ直し 5 回目 E3）。
#[test]
fn an_unsupported_kind_is_an_ascii_error_not_a_panic() {
    let manifest = InstallManifest {
        kind: InstallKind::Shell,
        name: "x".to_owned(),
        directory: "x".to_owned(),
        accept: None,
        existing: areka_nar::ExistingPolicy::Overlay,
        companions: Vec::new(),
        warnings: Vec::new(),
    };

    let refused = installed_path(Path::new("root"), &manifest, "a.txt").unwrap_err();

    assert!(
        refused.is_ascii() && refused.starts_with("unsupported kind Shell"),
        "{refused}"
    );
}
