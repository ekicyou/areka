//! `manifest` の兄弟テストの続き——同時インストールのバルーンを ukadoc
//! 「Install設定」の順に探す読み方（無印 → 0 → 1…・欠番で打ち切り）。
//!
//! 助手は [`super`]（`manifest_tests.rs`）から借りる。どの場面も、同梱の列
//! （接頭辞の並び）と記録の列を**完全一致**で突き合わせる。「含む」で測ると、
//! 読まれないはずの番号が紛れても、記録が二重に出ても緑のまま通る。

use super::*;

/// 同梱の列を接頭辞の並びにする。
fn companion_keys(manifest: &InstallManifest) -> Vec<&str> {
    manifest
        .companions
        .iter()
        .map(|companion| companion.key.as_str())
        .collect()
}

/// 「探索で読まなかった」の記録を鍵ごとに作る。
fn not_searched(key: &str) -> ManifestWarning {
    ManifestWarning::CompanionNotSearched {
        key: key.to_owned(),
    }
}

/// 欠番で打ち切る。`balloon0` と `balloon2` だけなら入るのは `balloon0` だけ（要件 1.3）。
///
/// 打ち切りの後ろの番号は、鍵ごとに 1 件「探索で読まなかった」を記録する（要件 6.3・6.5）。
#[test]
fn stops_the_search_at_the_first_missing_number() {
    let manifest = parsed_ghost(&[
        "balloon0.directory,zero",
        "balloon2.directory,two",
        "balloon2.refresh,1",
    ]);
    assert_eq!(companion_keys(&manifest), vec!["balloon0"]);
    assert_eq!(
        manifest.warnings,
        vec![
            not_searched("balloon2.directory"),
            not_searched("balloon2.refresh"),
        ]
    );
}

/// 無印が無くても打ち切らず、`balloon0` から続ける（要件 1.4・6.8）。
#[test]
fn continues_from_balloon0_when_the_plain_balloon_is_absent() {
    let manifest = parsed_ghost(&["balloon1.directory,one", "balloon0.directory,zero"]);
    assert_eq!(companion_keys(&manifest), vec!["balloon0", "balloon1"]);
    assert_eq!(manifest.warnings, vec![]);
}

/// 無印と `balloon0` は別のバルーンとして両方入る（要件 1.5）。
#[test]
fn reads_the_plain_balloon_and_balloon0_as_two_companions() {
    let manifest = parsed_ghost(&["balloon0.directory,zero", "balloon.directory,plain"]);
    assert_eq!(companion_keys(&manifest), vec!["balloon", "balloon0"]);
    assert_eq!(
        manifest
            .companions
            .iter()
            .map(|companion| companion.directory.as_str())
            .collect::<Vec<_>>(),
        vec!["plain", "zero"]
    );
    assert_eq!(manifest.warnings, vec![]);
}

/// 並びは数の順。`balloon2` は `balloon10` より前に来る（要件 1.1・1.7）。
///
/// 鍵の名前順なら `balloon10` が先に来る。入力の行も名前順と逆の向きに書いておく。
#[test]
fn orders_balloon2_before_balloon10() {
    let mut lines: Vec<String> = (0..=10)
        .rev()
        .map(|n| format!("balloon{n}.directory,b{n}"))
        .collect();
    lines.push("balloon.directory,plain".to_owned());
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    let manifest = parsed_ghost(&borrowed);

    let mut expected = vec!["balloon".to_owned()];
    expected.extend((0..=10).map(|n| format!("balloon{n}")));
    assert_eq!(companion_keys(&manifest), expected);
    assert_eq!(manifest.warnings, vec![]);
}

/// 先頭に 0 を付けた番号は数えない（要件 1.6・6.3）。
///
/// `balloon01` は `balloon1` の綴りではないので、`balloon0` の次で打ち切られる。
/// `balloon00` だけなら `balloon0` が無いので 1 件も入らない。どちらも
/// `*.directory` の行が在るので、知らない鍵ではなく「探索で読まなかった」になる。
#[test]
fn does_not_count_a_number_written_with_leading_zeros() {
    let manifest = parsed_ghost(&["balloon0.directory,zero", "balloon01.directory,one"]);
    assert_eq!(companion_keys(&manifest), vec!["balloon0"]);
    assert_eq!(manifest.warnings, vec![not_searched("balloon01.directory")]);

    let manifest = parsed_ghost(&["balloon00.directory,zero"]);
    assert_eq!(companion_keys(&manifest), Vec::<&str>::new());
    assert_eq!(manifest.warnings, vec![not_searched("balloon00.directory")]);
}

/// `*.directory` の行が無い番号は見つからなかった扱いで、そこで打ち切る（要件 1.9・6.4）。
///
/// 断片（`balloon1.source.directory` だけ）は今どおり知らない鍵として記録する。
/// 打ち切りの後ろの `balloon2.directory` は「探索で読まなかった」になる。
#[test]
fn a_numbered_fragment_without_its_directory_stops_the_search() {
    let manifest = parsed_ghost(&[
        "balloon0.directory,zero",
        "balloon1.source.directory,one-src",
        "balloon2.directory,two",
    ]);
    assert_eq!(companion_keys(&manifest), vec!["balloon0"]);
    assert_eq!(
        manifest.warnings,
        vec![
            ManifestWarning::IgnoredKey {
                key: "balloon1.source.directory".to_owned()
            },
            not_searched("balloon2.directory"),
        ]
    );
}

/// 無印の断片では打ち切らず、`balloon0` を読む（要件 1.4・1.9・6.4）。
#[test]
fn a_plain_fragment_without_its_directory_does_not_stop_the_search() {
    let manifest = parsed_ghost(&[
        "balloon.source.directory,plain-src",
        "balloon0.directory,zero",
    ]);
    assert_eq!(companion_keys(&manifest), vec!["balloon0"]);
    assert_eq!(
        manifest.warnings,
        vec![ManifestWarning::IgnoredKey {
            key: "balloon.source.directory".to_owned()
        }]
    );
}

/// 打ち切りの後ろの鍵は値を見ない。壊れた値でも断らずに記録して続ける（要件 6.3）。
#[test]
fn a_broken_value_after_the_search_stops_is_recorded_not_refused() {
    let manifest = parsed_ghost(&[
        "balloon0.directory,zero",
        "balloon2.directory,../x",
        "balloon2.source.directory,../..",
    ]);
    assert_eq!(companion_keys(&manifest), vec!["balloon0"]);
    assert_eq!(
        manifest.warnings,
        vec![
            not_searched("balloon2.directory"),
            not_searched("balloon2.source.directory"),
        ]
    );
}
