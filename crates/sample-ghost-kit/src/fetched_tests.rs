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

/// 私有の名前空間の置き場へバイト列を置く。
fn place(namespace: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = fetched_path_in(namespace, name);
    std::fs::create_dir_all(path.parent().expect("置き場には親がある")).expect("棚を作れるはず");
    std::fs::write(&path, bytes).expect("置き場へ書けるはず");
    path
}

fn is_ascii_only(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii() && !b.is_ascii_control())
}

// ---- 置き場の位置 ----

#[test]
fn shelf_position_is_namespace_fetched_name_nar() {
    let ns = Path::new(r"C:\x\nar");
    assert_eq!(fetched_path_in(ns, "x"), ns.join("fetched").join("x.nar"));
}

// ---- 照合の 4 通り ----

#[test]
fn absent_archive_is_missing() {
    let ns = WorkDir::new().expect("私有の名前空間");
    let err = verified_nar_in(ns.path(), "x", GOOD_SHA, 1).expect_err("無ければ失敗");
    match &err {
        SampleError::FetchedNarMissing { sample, path } => {
            assert_eq!(*sample, "x");
            assert_eq!(path, &fetched_path_in(ns.path(), "x"));
        }
        other => panic!("FetchedNarMissing のはず: {other:?}"),
    }
    let text = err.to_string();
    assert!(text.contains("\"x\""), "{text}");
    assert!(
        text.contains(&ascii(&fetched_path_in(ns.path(), "x").to_string_lossy())),
        "{text}"
    );
    assert!(text.contains("sample-fetch.ps1"), "{text}");
    assert!(text.contains("vendors/sample_ghost/README.md"), "{text}");
    assert!(is_ascii_only(&text), "{text}");
}

#[test]
fn size_difference_is_mismatch_with_both_hashes() {
    let ns = WorkDir::new().expect("私有の名前空間");
    let good = tiny_nar("x", b"good");
    let bad = tiny_nar("x", b"longer");
    assert_ne!(good.len(), bad.len(), "較正: 大きさが違う");
    let registered: &'static str = Box::leak(sha256_hex(&good).into_boxed_str());
    place(ns.path(), "x", &bad);
    let err = verified_nar_in(ns.path(), "x", registered, good.len() as u64)
        .expect_err("大きさ違いは失敗");
    match &err {
        SampleError::FetchedNarMismatch {
            expected_size,
            actual_size,
            expected_sha256,
            actual_sha256,
            ..
        } => {
            assert_eq!(*expected_size, good.len() as u64);
            assert_eq!(*actual_size, bad.len() as u64);
            assert_eq!(*expected_sha256, registered);
            assert_eq!(actual_sha256, &sha256_hex(&bad));
        }
        other => panic!("FetchedNarMismatch のはず: {other:?}"),
    }
    let text = err.to_string();
    assert!(text.contains(registered), "{text}");
    assert!(text.contains(&sha256_hex(&bad)), "{text}");
    assert!(text.contains("not used"), "{text}");
    assert!(text.contains("sample-fetch.ps1"), "{text}");
    assert!(is_ascii_only(&text), "{text}");
}

#[test]
fn same_size_one_byte_difference_is_mismatch() {
    let ns = WorkDir::new().expect("私有の名前空間");
    let good = tiny_nar("x", b"good");
    let mut bad = good.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    let registered: &'static str = Box::leak(sha256_hex(&good).into_boxed_str());
    place(ns.path(), "x", &bad);
    let err = verified_nar_in(ns.path(), "x", registered, good.len() as u64)
        .expect_err("1 バイト違いは失敗");
    let SampleError::FetchedNarMismatch {
        actual_size,
        actual_sha256,
        ..
    } = &err
    else {
        panic!("FetchedNarMismatch のはず: {err:?}");
    };
    assert_eq!(*actual_size, good.len() as u64);
    assert_eq!(actual_sha256, &sha256_hex(&bad));
    let text = err.to_string();
    assert!(
        text.contains(registered) && text.contains(&sha256_hex(&bad)),
        "{text}"
    );
}

#[test]
fn matching_archive_returns_shelf_path() {
    let ns = WorkDir::new().expect("私有の名前空間");
    let good = tiny_nar("x", b"good");
    let registered: &'static str = Box::leak(sha256_hex(&good).into_boxed_str());
    let placed = place(ns.path(), "x", &good);
    assert_eq!(
        verified_nar_in(ns.path(), "x", registered, good.len() as u64).expect("合えば通る"),
        placed
    );
}

// ---- 表示が ASCII ----

#[test]
fn display_escapes_non_ascii_paths() {
    let path = PathBuf::from("C:\\\u{3042}\\x.nar");
    let missing = SampleError::FetchedNarMissing {
        sample: "x",
        path: path.clone(),
    }
    .to_string();
    let mismatch = SampleError::FetchedNarMismatch {
        sample: "x",
        path,
        expected_size: 1,
        actual_size: 2,
        expected_sha256: "a",
        actual_sha256: "b".to_owned(),
    }
    .to_string();
    for text in [&missing, &mismatch] {
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
    fetched_row_form("x", "https://example.invalid/x.nar", GOOD_SHA, 1).expect("良い行は通る");
}

#[test]
fn row_form_rejects_each_bad_input() {
    let short = &GOOD_SHA[..63];
    let upper = GOOD_SHA.to_ascii_uppercase();
    let bad: [(&str, &str, &str, u64); 6] = [
        ("x", "http://example.invalid/x.nar", GOOD_SHA, 1),
        ("x", "https://example.invalid/a b.nar", GOOD_SHA, 1),
        ("x", "https://example.invalid/x.nar", &upper, 1),
        ("x", "https://example.invalid/x.nar", short, 1),
        ("x", "https://example.invalid/x.nar", GOOD_SHA, 0),
        ("--x", "https://example.invalid/x.nar", GOOD_SHA, 1),
    ];
    for (name, url, sha256, size) in bad {
        assert!(
            fetched_row_form(name, url, sha256, size).is_err(),
            "落ちるはず: {name} {url} {sha256} {size}"
        );
    }
}

// ---- 掃除は置き場を走査しない ----

#[test]
fn fetched_shelf_is_outside_work_and_cache() {
    let ns = Path::new(r"C:\x\nar");
    let path = fetched_path_in(ns, "x");
    for shelf in ["work", "cache", "manual"] {
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
            sha256: GOOD_SHA,
            size: 1,
        },
    };
    let path = crate::fetched_archive_path(&fetched)
        .expect("名前空間は決まる")
        .expect("URL の行には置き場がある");
    assert!(path.is_absolute(), "{}", path.display());
    assert!(
        path.ends_with(Path::new("fetched").join("x.nar")),
        "{}",
        path.display()
    );
}
