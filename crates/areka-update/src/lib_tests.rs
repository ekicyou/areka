//! 公開面（`lib.rs`）の兄弟テスト（要件 3.1・3.3・7.4・8.1・8.4・10.8）。
//!
//! ⑴ 語彙の全数対応: 11 語の固定入力を 1 本で回し、得た `kind()` の集合が
//!    `FailReason::ALL_KINDS` と完全一致し、失敗ごとに `error` が 1 件で、その欄が
//!    戻り値と一致する（欄の判定は `run_tests.rs` の `Ran::failed` が持つ）。
//! ⑵ 字面の見張り: 本番のソース（`*_tests.rs` と `testkit.rs` を除く `src/*.rs`）の
//!    コード行（`//` で始まる行を除く）だけを見る。記録は `lib.rs` だけ・`error!` は
//!    1 か所・WinHTTP の API は `winhttp.rs` だけ・`unsafe` は `winhttp.rs` と `md5.rs`
//!    だけ・dev 専用の道具が本番の依存に無い。
//!
//! 字面の走査は `areka-nar/src/lib_tests.rs` の形を写す。各見張りの数え方の較正は
//! 同じテストの中に置く（空振り・何にでも当たる、のどちらでも恒真になるため）。

use super::*;
use crate::md5::md5_hex;
use crate::paths::WORK_DIR;
use crate::run_tests::{HOME, MD5_ABC, Ran, fixture, go, three_ending_with_existing, url};
use crate::testkit::{FakeFetch, Pinned, dau, hold, junction, pe_image, pin};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

// ---- 語彙の全数対応（7.4・8.1・8.4） ----

/// 失敗の語彙 1 語につき固定入力 1 つ。順は `ALL_KINDS` の宣言順。
fn cases() -> Vec<fn() -> Ran> {
    vec![
        // TargetMissing
        || {
            let f = fixture();
            go(HOME, &f.target.join("nope"), &FakeFetch::new())
        },
        // InvalidHomeurl
        || {
            let f = fixture();
            go("ftp://example.test/ghost/", &f.target, &FakeFetch::new())
        },
        // ManifestMissing
        || {
            let f = fixture();
            go(HOME, &f.target, &FakeFetch::new())
        },
        // ManifestFetch
        || {
            let f = fixture();
            let fetch = FakeFetch::new().fail(&url("updates2.dau"), FetchError::Timeout);
            go(HOME, &f.target, &fetch)
        },
        // LocalUnreadable（定義上はファイルの名前に同名のフォルダが居る）
        || {
            let f = fixture();
            fs::create_dir(f.target.join("a.txt")).unwrap();
            let fetch =
                FakeFetch::new().serve(&url("updates2.dau"), &dau(&[&["a.txt", MD5_ABC]], true));
            go(HOME, &f.target, &fetch)
        },
        // WorkArea（棚の名前を同名のファイルで塞ぐ）
        || {
            let f = fixture();
            let (fetch, _) = three_ending_with_existing(&f, b"A");
            fs::write(f.target.join(WORK_DIR), b"blocker").unwrap();
            go(HOME, &f.target, &fetch)
        },
        // FileFetch
        || {
            let f = fixture();
            let (fetch, _) = three_ending_with_existing(&f, b"A");
            go(
                HOME,
                &f.target,
                &fetch.fail(&url("sub/x.txt"), FetchError::Connect),
            )
        },
        // Md5Mismatch
        || {
            let f = fixture();
            let (fetch, _) = three_ending_with_existing(&f, b"A");
            go(
                HOME,
                &f.target,
                &fetch.serve(&url("sub/x.txt"), b"tampered"),
            )
        },
        // EscapesTarget（対象の外を指すジャンクション）
        || {
            let f = fixture();
            let link = f.target.join("link");
            junction(&link, &f.outside());
            let fetch = FakeFetch::new().serve(
                &url("updates2.dau"),
                &dau(&[&["link/x.txt", &md5_hex(b"new")]], true),
            );
            let ran = go(HOME, &f.target, &fetch);
            fs::remove_dir(&link).expect("ジャンクションを外せる");
            ran
        },
        // CommitWrite（3 件目の退避を拒ませる）
        || {
            let f = fixture();
            let (fetch, _) = three_ending_with_existing(&f, b"A");
            let held = hold(&f.target.join("b.txt"));
            let ran = go(HOME, &f.target, &fetch);
            drop(held);
            ran
        },
        // RollbackFailed（置いた PE の削除を拒ませる・手段は run_tests と同じ）
        || {
            let f = fixture();
            let pe = pe_image();
            let (fetch, _) = three_ending_with_existing(&f, &pe);
            let pinned: Rc<RefCell<Option<Pinned>>> = Rc::default();
            let fetch = {
                let (pinned, shelf, b_url) =
                    (pinned.clone(), f.target.join(WORK_DIR), url("b.txt"));
                fetch.on_get(move |u| {
                    if u == b_url {
                        let dir = fs::read_dir(&shelf).unwrap().next().unwrap().unwrap();
                        *pinned.borrow_mut() = Some(pin(&dir.path().join("new").join("a.dll")));
                    }
                })
            };
            let held = hold(&f.target.join("b.txt"));
            let ran = go(HOME, &f.target, &fetch);
            drop(held);
            assert!(pinned.borrow_mut().take().is_some(), "注入が走っていない");
            ran
        },
    ]
}

#[test]
fn every_fail_kind_has_a_fixture_and_is_recorded_once_with_matching_fields() {
    let mut kinds = Vec::new();
    for case in cases() {
        let ran = case();
        // error がちょうど 1 件で、欄が戻り値と一致する（Ran::failed が判定する）。
        let err = ran.failed();
        kinds.push(err.reason.kind());
    }
    let got: BTreeSet<&str> = kinds.iter().copied().collect();
    let all: BTreeSet<&str> = FailReason::ALL_KINDS.iter().copied().collect();
    assert_eq!(got, all, "固定入力から得た語彙が全数一覧と食い違う");
    assert_eq!(kinds.len(), all.len(), "同じ語を 2 度数えている: {kinds:?}");
    assert_eq!(all.len(), 11, "語彙は 11 語");
}

// ---- 字面の見張り（3.1・3.3・8.1・10.8） ----

/// 本番のソース（`*_tests.rs` と `testkit.rs` を除く）を (ファイル名, コード行) で返す。
///
/// 一覧を手で綴らずに毎回数え直す。`//` で始まる行（doc コメントを含む）は落とす——
/// 記録の仕組みや WinHTTP を説明する散文まで違反に数えると、見張りが説明を禁じてしまう。
fn production_sources() -> Vec<(String, String)> {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut out: Vec<(String, String)> = fs::read_dir(dir)
        .expect("本クレートの src を読める")
        .map(|child| child.expect("要素を読める").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".rs") && !name.ends_with("_tests.rs"))
        .filter(|name| name != "testkit.rs")
        .map(|name| {
            let text = fs::read_to_string(dir.join(&name)).expect("ソースを読める");
            (name, code(&text))
        })
        .collect();
    out.sort();
    out
}

/// `//` で始まる行を落とした残り。
fn code(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `pred` が真のファイル名の一覧。
fn files_where(pred: impl Fn(&str) -> bool) -> Vec<String> {
    production_sources()
        .into_iter()
        .filter(|(_, text)| pred(text))
        .map(|(name, _)| name)
        .collect()
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// 識別子として独立した `word` の出現数（`unsafe_component` の `unsafe` は数えない）。
fn word_count(text: &str, word: &str) -> usize {
    text.match_indices(word)
        .filter(|(i, _)| {
            let before = text[..*i].chars().next_back();
            let after = text[i + word.len()..].chars().next();
            !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
        })
        .count()
}

/// WinHTTP の API の綴り（`WinHttp[A-Z]…` のうち公開型 `WinHttpFetch` を除く、または取り込み）。
fn spells_winhttp_api(text: &str) -> bool {
    text.contains("windows::Win32::Networking::WinHttp")
        || text.match_indices("WinHttp").any(|(i, _)| {
            let ident: String = text[i..].chars().take_while(|&c| is_ident(c)).collect();
            ident.chars().nth(7).is_some_and(|c| c.is_ascii_uppercase()) && ident != "WinHttpFetch"
        })
}

/// 走査の較正。対象が実在し、除くべき物を除いていることを確かめる。
#[test]
fn the_scan_looks_at_real_production_files_only() {
    let names: Vec<String> = production_sources().into_iter().map(|(n, _)| n).collect();
    for expected in ["lib.rs", "winhttp.rs", "md5.rs", "commit.rs", "fetch.rs"] {
        assert!(
            names.iter().any(|n| n == expected),
            "{expected} が走査に無い: {names:?}"
        );
    }
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with("_tests.rs") || n == "testkit.rs"),
        "テストの道具まで走査している: {names:?}"
    );
    assert_eq!(code("a\n  /// doc\n//! m\nb // tail"), "a\nb // tail");
}

/// 記録を綴る本番のソースは `lib.rs` だけ（二重記録を避ける・8.1）。
#[test]
fn only_the_public_surface_writes_records() {
    assert_eq!(files_where(|t| t.contains("tracing")), ["lib.rs"]);
}

/// `error!` は本番のソース全体で 1 か所（`log_failure`）。
#[test]
fn the_failure_record_is_fired_from_one_place() {
    let counted: Vec<(String, usize)> = production_sources()
        .into_iter()
        .map(|(name, text)| (name, text.matches("error!").count()))
        .filter(|(_, n)| *n > 0)
        .collect();
    assert_eq!(counted, [("lib.rs".to_string(), 1)]);
    assert_eq!("tracing::error!(".matches("error!").count(), 1);
}

/// WinHTTP の API を綴るのは取得口の実装だけ（3.1・3.3）。
#[test]
fn only_the_fetch_implementation_spells_winhttp() {
    assert_eq!(files_where(spells_winhttp_api), ["winhttp.rs"]);
    // 較正——拾うべき物を拾い、公開型と大文字の定数は拾わない。
    assert!(spells_winhttp_api("unsafe { WinHttpOpen(a) }"));
    assert!(spells_winhttp_api(
        "use windows::Win32::Networking::WinHttp::X;"
    ));
    assert!(!spells_winhttp_api(
        "pub use winhttp::{MAX_BODY_BYTES, WinHttpFetch};"
    ));
    assert!(!spells_winhttp_api(
        "12002 => FetchError::Timeout, // ERROR_WINHTTP_TIMEOUT"
    ));
}

/// `unsafe` は取得口と MD5 だけ・MD5 は 1 か所。
#[test]
fn unsafe_lives_only_in_the_os_boundaries() {
    let counted: Vec<(String, usize)> = production_sources()
        .into_iter()
        .map(|(name, text)| (name, word_count(&text, "unsafe")))
        .filter(|(_, n)| *n > 0)
        .collect();
    let names: Vec<&str> = counted.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["md5.rs", "winhttp.rs"], "{counted:?}");
    assert_eq!(
        counted[0].1, 1,
        "md5.rs の unsafe は BCryptHash の 1 呼出だけ"
    );
    // 較正——識別子の一部は数えず、ブロックと関数は数える。
    assert_eq!(word_count("if unsafe_component(rel) {", "unsafe"), 0);
    assert_eq!(word_count("unsafe { f() } unsafe fn g()", "unsafe"), 2);
}

/// `Cargo.toml` の本番の依存の節に並ぶ dev 専用の道具。
fn dev_tools_in_production_deps(manifest: &str) -> Vec<String> {
    let mut section = "";
    let mut found = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line;
        } else if section.ends_with("dependencies]")
            && !section.ends_with("dev-dependencies]")
            && !line.starts_with('#')
        {
            for tool in ["log-capture-kit", "sample-ghost-kit"] {
                if line.contains(tool) {
                    found.push(format!("{section} {tool}"));
                }
            }
        }
    }
    found
}

/// dev 専用の道具が本番の依存に無い（10.8）。
#[test]
fn dev_only_tools_stay_out_of_production_dependencies() {
    let manifest = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml を読める");
    assert!(
        manifest.contains("[dev-dependencies]\n") || manifest.contains("[dev-dependencies]\r\n")
    );
    assert_eq!(
        dev_tools_in_production_deps(&manifest),
        Vec::<String>::new()
    );
    // 較正——本番の節なら拾い、dev の節なら拾わない。
    let sample = "[dependencies]\nlog-capture-kit = { path = \"x\" }\n\
                  [dev-dependencies]\nsample-ghost-kit = { path = \"y\" }\n";
    assert_eq!(
        dev_tools_in_production_deps(sample),
        ["[dependencies] log-capture-kit"]
    );
}
