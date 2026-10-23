//! 取り寄せの置き場と照合の兄弟テスト（spec: `areka-P0-sample-url-fetch` 要件 1.5・1.6・
//! 3.3〜3.6・3.9・4.1・4.2）。
//!
//! 照合は私有の名前空間（[`crate::WorkDir`]）と [`crate::NarBuilder`] で組んだ書庫で回し、
//! 期待値の SHA-256 は [`crate::sha256::sha256_hex`] で求める。通信しない。

use super::*;
use crate::sha256::sha256_hex;
use crate::{NarBuilder, SampleError, WorkDir, install_txt};

/// 最小のゴースト 1 体の `.nar` のバイト列。`mark` は長さを変えずに中身を変える目印。
fn tiny_nar(name: &str, mark: &[u8]) -> Vec<u8> {
    let dir = WorkDir::new().expect("組む場所は取れるはず");
    let nar = dir.path().join(format!("{name}.nar"));
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                &format!("name,{name}"),
                &format!("directory,{name}"),
            ]),
        )
        .done()
        .file("mark.txt", mark)
        .done()
        .write_to(&nar)
        .expect("固定入力の .nar を置けるはず");
    std::fs::read(&nar).expect("組んだ .nar は読めるはず")
}

/// 私有の名前空間の置き場へ書庫を置く。
fn place(namespace: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    put(&fetched_path_in(namespace, name), bytes)
}

/// 私有の名前空間の置き場へハッシュのファイルを置く。
fn place_checksum(namespace: &Path, name: &str, text: &str) -> PathBuf {
    put(&fetched_checksum_path_in(namespace, name), text.as_bytes())
}

fn put(path: &Path, bytes: &[u8]) -> PathBuf {
    std::fs::create_dir_all(path.parent().expect("置き場には親がある")).expect("棚を作れるはず");
    std::fs::write(path, bytes).expect("置き場へ書けるはず");
    path.to_path_buf()
}

/// sha256sum の形の 1 行（`<hash>  <name>.nar` と LF）。期待値は 2 の SHA-256 で求める。
fn sum_line(bytes: &[u8], name: &str) -> String {
    format!("{}  {name}.nar\n", sha256_hex(bytes))
}

fn is_ascii_only(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii() && !b.is_ascii_control())
}

// ---- 置き場の位置と綴り ----

#[test]
fn shelf_positions_are_namespace_fetched_name() {
    let ns = Path::new(r"C:\x\nar");
    assert_eq!(fetched_path_in(ns, "x"), ns.join("fetched").join("x.nar"));
    assert_eq!(
        fetched_checksum_path_in(ns, "x"),
        ns.join("fetched").join("x.nar.sha256")
    );
}

#[test]
fn checksum_url_appends_suffix() {
    assert_eq!(CHECKSUM_SUFFIX, ".sha256");
    assert_eq!(checksum_url("https://x/a.nar"), "https://x/a.nar.sha256");
}

// ---- ハッシュのファイルの読み ----

#[test]
fn parse_checksum_reads_sha256sum_line() {
    let upper = GOOD_SHA.to_ascii_uppercase();
    for text in [
        format!("{GOOD_SHA}  x.nar\n"),
        format!("{upper}  x.nar\n"),
        format!("\u{FEFF}{GOOD_SHA}  x.nar\r\n"),
        format!("{GOOD_SHA}\n"),
        format!("  {GOOD_SHA} *other-name.nar"),
    ] {
        assert_eq!(parse_checksum(&text).as_deref(), Some(GOOD_SHA), "{text:?}");
    }
}

#[test]
fn parse_checksum_rejects_bad_forms() {
    let short = &GOOD_SHA[..63];
    let non_hex = format!("{short}g");
    let long = format!("{GOOD_SHA}0");
    for text in [
        format!("{short}  x.nar\n"),
        format!("{non_hex}  x.nar\n"),
        format!("{long}  x.nar\n"),
        String::new(),
        "\n".to_owned(),
    ] {
        assert_eq!(parse_checksum(&text), None, "{text:?}");
    }
}

// ---- 照合 ----

#[test]
fn absent_archive_is_missing_with_archive_path() {
    let ns = WorkDir::new().expect("私有の名前空間");
    let good = tiny_nar("x", b"good");
    place_checksum(ns.path(), "x", &sum_line(&good, "x"));
    let err = verified_nar_in(ns.path(), "x").expect_err("書庫が無ければ失敗");
    let nar = fetched_path_in(ns.path(), "x");
    match &err {
        SampleError::FetchedNarMissing { sample, path } => {
            assert_eq!(*sample, "x");
            assert_eq!(path, &nar);
        }
        other => panic!("FetchedNarMissing のはず: {other:?}"),
    }
    let text = err.to_string();
    assert!(text.contains(&ascii(&nar.to_string_lossy())), "{text}");
    assert!(text.contains("x.nar.sha256"), "{text}");
}

#[test]
fn absent_checksum_is_missing_with_checksum_path() {
    let ns = WorkDir::new().expect("私有の名前空間");
    place(ns.path(), "x", &tiny_nar("x", b"good"));
    let err = verified_nar_in(ns.path(), "x").expect_err("ハッシュのファイルが無ければ失敗");
    match &err {
        SampleError::FetchedNarMissing { sample, path } => {
            assert_eq!(*sample, "x");
            assert_eq!(path, &fetched_checksum_path_in(ns.path(), "x"));
        }
        other => panic!("FetchedNarMissing のはず: {other:?}"),
    }
}

#[test]
fn malformed_checksum_is_invalid() {
    let ns = WorkDir::new().expect("私有の名前空間");
    place(ns.path(), "x", &tiny_nar("x", b"good"));
    let sum = place_checksum(ns.path(), "x", "not a hash\n");
    let err = verified_nar_in(ns.path(), "x").expect_err("形が違えば失敗");
    match &err {
        SampleError::FetchedChecksumInvalid { sample, path } => {
            assert_eq!(*sample, "x");
            assert_eq!(path, &sum);
        }
        other => panic!("FetchedChecksumInvalid のはず: {other:?}"),
    }
    let text = err.to_string();
    assert!(text.contains("sha256sum"), "{text}");
}

#[test]
fn one_byte_difference_is_mismatch_with_both_hashes() {
    let ns = WorkDir::new().expect("私有の名前空間");
    let good = tiny_nar("x", b"good");
    let mut bad = good.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    let nar = place(ns.path(), "x", &bad);
    place_checksum(ns.path(), "x", &sum_line(&good, "x").to_ascii_uppercase());
    let err = verified_nar_in(ns.path(), "x").expect_err("1 バイト違いは失敗");
    let SampleError::FetchedNarMismatch {
        sample,
        path,
        expected_sha256,
        actual_sha256,
    } = &err
    else {
        panic!("FetchedNarMismatch のはず: {err:?}");
    };
    assert_eq!(*sample, "x");
    assert_eq!(path, &nar);
    assert_eq!(expected_sha256, &sha256_hex(&good), "小文字へ揃える");
    assert_eq!(actual_sha256, &sha256_hex(&bad));
    let text = err.to_string();
    assert!(
        text.contains(&sha256_hex(&good)) && text.contains(&sha256_hex(&bad)),
        "{text}"
    );
    assert!(text.contains("not used"), "{text}");
}

#[test]
fn matching_archive_returns_shelf_path() {
    let ns = WorkDir::new().expect("私有の名前空間");
    let good = tiny_nar("x", b"good");
    let placed = place(ns.path(), "x", &good);
    place_checksum(ns.path(), "x", &sum_line(&good, "x"));
    assert_eq!(verified_nar_in(ns.path(), "x").expect("合えば通る"), placed);
}

// ---- 表示が ASCII ----

#[test]
fn display_escapes_non_ascii_paths() {
    let path = PathBuf::from("C:\\\u{3042}\\x.nar");
    let texts = [
        SampleError::FetchedNarMissing {
            sample: "x",
            path: path.clone(),
        },
        SampleError::FetchedChecksumInvalid {
            sample: "x",
            path: path.clone(),
        },
        SampleError::FetchedNarMismatch {
            sample: "x",
            path,
            expected_sha256: "a".to_owned(),
            actual_sha256: "b".to_owned(),
        },
    ]
    .map(|err| err.to_string());
    for text in &texts {
        assert!(is_ascii_only(text), "{text}");
        assert!(text.contains(r"C:\\u{3042}\x.nar"), "{text}");
        assert!(text.contains("\"x\""), "{text}");
        assert!(text.contains("sample-fetch.ps1"), "{text}");
        assert!(text.contains("vendors/sample_ghost/README.md"), "{text}");
    }
}

#[test]
fn ascii_escapes_non_ascii_and_control_only() {
    assert_eq!(ascii("abc\\d e"), "abc\\d e");
    assert_eq!(ascii("\u{3042}\n"), "\\u{3042}\\u{000A}");
}

#[test]
fn fetch_command_is_absolute_pwsh_call() {
    let command = fetch_command();
    let script = command
        .strip_prefix("pwsh -NoProfile -File \"")
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| panic!("形が違う: {command}"));
    let script = Path::new(script);
    assert!(script.is_absolute(), "{command}");
    assert!(
        !script
            .components()
            .any(|c| c == std::path::Component::ParentDir),
        "{command}"
    );
    assert!(
        script.ends_with(Path::new("tools").join("sample-fetch.ps1")),
        "{command}"
    );
    assert!(
        script
            .parent()
            .and_then(Path::parent)
            .expect("根")
            .join("Cargo.toml")
            .is_file(),
        "{command}"
    );
}

// ---- 行の欄の形 ----

const GOOD_SHA: &str = "816d7183ab7c92fe1d601e386d5d60f9ca9f4238d265143a1b96915aa5bfb020";

#[test]
fn row_form_accepts_a_good_row() {
    fetched_row_form("x", "https://example.invalid/x.nar").expect("良い行は通る");
}

#[test]
fn row_form_rejects_each_bad_input() {
    let bad: [(&str, &str); 5] = [
        ("x", "http://example.invalid/x.nar"),
        ("x", "https://example.invalid/a b.nar"),
        ("x", "https://example.invalid/a\u{7}.nar"),
        ("x", ""),
        ("--x", "https://example.invalid/x.nar"),
    ];
    for (name, url) in bad {
        assert!(
            fetched_row_form(name, url).is_err(),
            "落ちるはず: {name} {url:?}"
        );
    }
}

// ---- 掃除は置き場を走査しない ----

#[test]
fn fetched_shelf_is_outside_work_and_cache() {
    let ns = Path::new(r"C:\x\nar");
    let path = fetched_path_in(ns, "x");
    for shelf in ["work", crate::devroot::CACHE, "manual"] {
        assert!(!path.starts_with(ns.join(shelf)), "{}", path.display());
    }
}

// ---- 行から置き場のパス ----

#[test]
fn archive_path_of_rows() {
    let vendored = crate::SAMPLES.first().expect("行がある");
    assert_eq!(vendored.source, crate::SampleSource::Vendored);
    assert_eq!(crate::fetched_archive_path(vendored).expect("決まる"), None);
    let fetched = crate::Sample {
        name: "x",
        kind: crate::SampleKind::Ghost,
        balloons: &[],
        source: crate::SampleSource::Fetched {
            url: "https://example.invalid/x.nar",
        },
    };
    assert_eq!(
        crate::fetched_checksum_path(vendored).expect("決まる"),
        None
    );
    let path = crate::fetched_archive_path(&fetched)
        .expect("名前空間は決まる")
        .expect("URL の行には置き場がある");
    let sum = crate::fetched_checksum_path(&fetched)
        .expect("名前空間は決まる")
        .expect("URL の行には置き場がある");
    assert!(path.is_absolute(), "{}", path.display());
    assert!(
        path.ends_with(Path::new("fetched").join("x.nar")),
        "{}",
        path.display()
    );
    assert_eq!(sum, path.with_file_name("x.nar.sha256"));
}
