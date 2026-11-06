//! 取り出したフォルダのファイル名の集合・絵の寸法・名前の重なりを固定する。
//!
//! 断りの無い要件の番号と design.md の参照は spec `default-balloon-selfmade` のもの（別の spec のものはその場で名を書く）。
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **12.2**・**2.1**・**2.2**・**2.4**・**2.6**・**2.7**・
//! **3.1**・**3.2**・**3.4**・**4.1**・**5.2**・**7.1**・**10.3**）。

use sample_ghost_kit::SAMPLES;

use crate::support::fixture::png_native_size;

use super::test_support::{BALLOON_NAME, EXPECTED_FILE_NAMES, EXPECTED_PNG_SIZES, balloon_root};

/// フォルダ直下が 23 ファイルの表と過不足なく一致し、サブフォルダも空のファイルも無い。
///
/// 集合の一致で `.pna`・GIF・2 倍の絵・`online1` 以降が無いこと（要件 2.4・2.6・3.2・3.4）も決まる。
/// `.pna` と GIF は名指しの失敗にするため先に見る。
#[test]
fn folder_holds_exactly_the_23_files() {
    let root = balloon_root();
    let entries = std::fs::read_dir(&root).unwrap_or_else(|e| {
        panic!(
            "areka-balloon のフォルダ {} を読めない（このフォルダの実在が前提）: {e}",
            root.display()
        )
    });

    let mut names: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("{} の列挙中に失敗した: {e}", root.display()));
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry
            .metadata()
            .unwrap_or_else(|e| panic!("{} の種別を取れない: {e}", root.join(&name).display()));
        assert!(
            !meta.is_dir(),
            "{}: サブフォルダは置かない（実在: `{name}`）",
            root.display()
        );
        assert!(
            meta.len() > 0,
            "{}: 空のファイルである",
            root.join(&name).display()
        );
        let lower = name.to_ascii_lowercase();
        assert!(
            !lower.ends_with(".pna") && !lower.ends_with(".gif"),
            "{}: `.pna` と GIF は置かない（要件 2.4・3.4）",
            root.join(&name).display()
        );
        names.push(name);
    }
    names.sort();

    let extra: Vec<&String> = names
        .iter()
        .filter(|n| !EXPECTED_FILE_NAMES.contains(&n.as_str()))
        .collect();
    let missing: Vec<&&str> = EXPECTED_FILE_NAMES
        .iter()
        .filter(|n| !names.iter().any(|m| m == *n))
        .collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "{}: ファイルの集合が表の {} 本と食い違う（実測 {} 本）。余分 {:?}／欠落 {:?}",
        root.display(),
        EXPECTED_FILE_NAMES.len(),
        names.len(),
        extra,
        missing
    );
}

/// PNG の絵 13 枚の寸法が IHDR と一致する（要件 2.1・2.2・3.1・4.1）。
#[test]
fn png_native_sizes_are_pinned() {
    let root = balloon_root();
    for (name, width, height) in EXPECTED_PNG_SIZES {
        let path = root.join(name);
        assert_eq!(
            png_native_size(&path),
            (width, height),
            "{}: 寸法が期待 {width}×{height} と違う",
            path.display()
        );
    }
}

/// 名前 `areka-balloon` が、登記のほかの名前と同梱バルーンの名前のどれとも、大小を無視して
/// 重ならない（要件 5.2）。自分の行は登記にちょうど 1 つ在る（要件 10.3）。
#[test]
fn name_does_not_collide_case_insensitively() {
    let own_rows = SAMPLES.iter().filter(|s| s.name == BALLOON_NAME).count();
    assert_eq!(
        own_rows, 1,
        "登記表に `{BALLOON_NAME}` の行がちょうど 1 つ在ること"
    );
    let others: Vec<&str> = SAMPLES
        .iter()
        .filter(|s| s.name != BALLOON_NAME)
        .map(|s| s.name)
        .chain(SAMPLES.iter().flat_map(|s| s.balloons.iter().copied()))
        .collect();
    assert!(
        others.contains(&"bottle"),
        "比べる名前に同梱バルーン `bottle` が無い（比べる相手の取り方が壊れている）: {others:?}"
    );
    let clashes: Vec<&&str> = others
        .iter()
        .filter(|n| n.eq_ignore_ascii_case(BALLOON_NAME))
        .collect();
    assert!(
        clashes.is_empty(),
        "`{BALLOON_NAME}` が大小を無視してほかの名前と重なる: {clashes:?}"
    );
}
