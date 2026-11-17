//! 見本 `fold-samples` の写しの式と書き先の順のテスト。
//!
//! どれも [`WorkDir`] の下に組んだ偽のリポジトリ根で `fold_one` を回し、本物の
//! `vendors/` には触らない。`git` を呼ぶ `main` は通らない（追跡の一覧は引数で渡す）。

use super::*;
use sample_ghost_kit::{WorkDir, install_txt};

/// 偽の根の中の、同梱バルーンつきの小さな展開形（`install.txt`・本体・同梱バルーンの取り出し元の 3 つを持つ形）。返すのは
/// 展開形のフォルダと、その追跡ファイルの一覧。
fn ghost_with_companion(fake_root: &Path) -> (PathBuf, Vec<String>) {
    let source = fake_root.join("unfolded").join("tiny");
    let files: [(&str, Vec<u8>); 4] = [
        (
            "install.txt",
            install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,ためしゴースト",
                "directory,tiny-ghost",
                "balloon.directory,tiny-balloon",
                "balloon.source.directory,tiny-balloon-src",
            ]),
        ),
        ("ghost/master/descript.txt", b"ghost descript".to_vec()),
        (
            "tiny-balloon-src/descript.txt",
            b"balloon descript".to_vec(),
        ),
        ("tiny-balloon-src/balloon/s0.png", b"s0".to_vec()),
    ];
    let mut tracked = Vec::new();
    for (relative, bytes) in files {
        let path = source.join(relative.replace('/', MAIN_SEPARATOR_STR));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        tracked.push(relative.to_owned());
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
