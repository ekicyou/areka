//! 開く処理の解決のテスト（areka-P0-open-external-tags task 3.1・要件 1.1〜1.4・2.1〜2.4・3.1・
//! 4.1〜4.3・5.1・5.3・6.1・6.2・10.2）。
//!
//! 行き先は本物の規則（`classify`）で台本のタグの形から作り、偽の境界と一時フォルダで
//! 動詞・対象・引数・作業フォルダを判定する。OS は 1 度も呼ばない。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use areka_parsers::sakura::JUMP_TAG_CARRIER;
use temp_path_kit::TempPath;

use super::super::destination::{Destination, Target, classify};
use super::super::opener_test_support::{FakeOs, build_ghost_root};
use super::super::os_port::{OsCall, Verb};
use super::{OpenContext, OpenFailure, expand_env, resolve};

/// 一時フォルダの最小のゴーストと、その文脈。
struct Fixture {
    _tmp: TempPath,
    ctx: OpenContext,
}

impl Fixture {
    fn new() -> Self {
        let tmp = TempPath::new("open-ext-resolve");
        let root = build_ghost_root(&tmp, ("g", "G"), &[], &[]);
        let ctx = OpenContext {
            ghost: "G".to_owned(),
            ghost_dir: root.ghost_dir("g"),
            baseware: Some(root),
        };
        Fixture { _tmp: tmp, ctx }
    }

    /// `ghost/master`。
    fn master(&self) -> PathBuf {
        self.ctx.ghost_dir.join("ghost").join("master")
    }

    /// `ghost/master` の下にファイルを置き、その絶対パスを返す。
    fn file(&self, rel: &str) -> PathBuf {
        let p = self.master().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"x").unwrap();
        p
    }

    fn resolve(&self, dest: &Destination, os: &FakeOs) -> Result<OsCall, OpenFailure> {
        resolve(dest, &self.ctx, os)
    }
}

/// 台本のタグの形から本物の規則で行き先を作る。
fn dest(name: &str, args: &[&str]) -> Destination {
    classify(name, args).expect("開く系").expect("受理される")
}

fn want(verb: Verb, file: impl Into<OsString>, params: Option<&str>, dir: Option<&Path>) -> OsCall {
    OsCall {
        verb,
        file: file.into(),
        params: params.map(OsString::from),
        dir: dir.map(Path::to_path_buf),
    }
}

#[test]
fn jump_url_is_opened_as_is() {
    let f = Fixture::new();
    let got = f.resolve(
        &dest(JUMP_TAG_CARRIER, &["https://example.com/a"]),
        &FakeOs::default(),
    );
    assert_eq!(
        got,
        Ok(want(Verb::Open, "https://example.com/a", None, None))
    );
}

#[test]
fn jump_mailto_is_opened_as_is() {
    let f = Fixture::new();
    let got = f.resolve(
        &dest(JUMP_TAG_CARRIER, &["mailto:a@example.com"]),
        &FakeOs::default(),
    );
    assert_eq!(
        got,
        Ok(want(Verb::Open, "mailto:a@example.com", None, None))
    );
}

#[test]
fn jump_file_absolute_opens_with_its_folder_as_working_dir() {
    let f = Fixture::new();
    let p = f.file("abs.txt");
    let id = format!("file:///{}", p.display());
    let got = f.resolve(&dest(JUMP_TAG_CARRIER, &[&id]), &FakeOs::default());
    assert_eq!(got, Ok(want(Verb::Open, &p, None, Some(&f.master()))));
}

#[test]
fn jump_file_relative_is_joined_to_ghost_master() {
    let f = Fixture::new();
    let got = f.resolve(
        &dest(JUMP_TAG_CARRIER, &["file:///descript.txt"]),
        &FakeOs::default(),
    );
    let p = f.master().join("descript.txt");
    assert_eq!(got, Ok(want(Verb::Open, &p, None, Some(&f.master()))));
}

#[test]
fn jump_file_missing_is_not_found_at_the_joined_path() {
    let f = Fixture::new();
    let got = f.resolve(
        &dest(JUMP_TAG_CARRIER, &["file:///none.txt"]),
        &FakeOs::default(),
    );
    let Err(OpenFailure::NotFound(p)) = got else {
        panic!("見つからないはず: {got:?}");
    };
    assert_eq!(p, f.master().join("none.txt"));
}

#[test]
fn open_file_absolute_and_relative() {
    let f = Fixture::new();
    let p = f.file(r"doc\manual.pdf");
    let dir = p.parent().unwrap();
    let os = FakeOs::default();
    let abs = p.to_str().unwrap();
    assert_eq!(
        f.resolve(&dest("open", &["file", abs]), &os),
        Ok(want(Verb::Open, &p, None, Some(dir)))
    );
    assert_eq!(
        f.resolve(&dest("open", &["file", r"doc\manual.pdf"]), &os),
        Ok(want(Verb::Open, &p, None, Some(dir)))
    );
}

#[test]
fn open_file_expands_env_vars_from_the_boundary() {
    let f = Fixture::new();
    let p = f.file("env.txt");
    let os = FakeOs::default().with_env("AREKA_T_DIR", f.master().to_str().unwrap());
    // 大文字小文字を区別せずに引く（境界の規則）。
    let got = f.resolve(&dest("open", &["file", r"%areka_t_dir%\env.txt"]), &os);
    assert_eq!(got, Ok(want(Verb::Open, &p, None, Some(&f.master()))));
}

#[test]
fn undefined_env_var_is_left_as_written() {
    let os = FakeOs::default().with_env("A", "1");
    assert_eq!(expand_env("x%NOPE%y", &os), "x%NOPE%y");
    assert_eq!(expand_env("%A%%NOPE%%A%", &os), "1%NOPE%1");
    assert_eq!(expand_env("50%", &os), "50%");
    assert_eq!(expand_env("%%", &os), "%%");
    // 未定義で据え置いた名前だけ（区切り無し）は、そのまま OS のパス探索へ渡る。
    let f = Fixture::new();
    let got = f.resolve(&dest("open", &["file", "%NOPE%tool.exe"]), &os);
    assert_eq!(got, Ok(want(Verb::Open, "%NOPE%tool.exe", None, None)));
}

#[test]
fn bare_name_in_ghost_master_resolves_to_that_file() {
    let f = Fixture::new();
    let p = f.file("tool.exe");
    let got = f.resolve(&dest("open", &["file", "tool.exe"]), &FakeOs::default());
    assert_eq!(got, Ok(want(Verb::Open, &p, None, Some(&f.master()))));
}

#[test]
fn bare_name_not_in_ghost_master_goes_to_os_path_search() {
    let f = Fixture::new();
    let got = f.resolve(&dest("open", &["file", "notepad.exe"]), &FakeOs::default());
    assert_eq!(got, Ok(want(Verb::Open, "notepad.exe", None, None)));
}

#[test]
fn missing_path_with_separator_is_not_found() {
    let f = Fixture::new();
    let got = f.resolve(
        &dest("open", &["file", r"sub\none.exe"]),
        &FakeOs::default(),
    );
    let Err(OpenFailure::NotFound(p)) = got else {
        panic!("見つからないはず: {got:?}");
    };
    assert_eq!(p, f.master().join(r"sub\none.exe"));
}

#[test]
fn open_browser_opens_url() {
    let f = Fixture::new();
    let got = f.resolve(&dest("open", &["browser", "http://a/"]), &FakeOs::default());
    assert_eq!(got, Ok(want(Verb::Open, "http://a/", None, None)));
}

#[test]
fn open_explorer_folder_opens_the_folder() {
    let f = Fixture::new();
    let docs = f.master().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    let got = f.resolve(&dest("open", &["explorer", "docs"]), &FakeOs::default());
    assert_eq!(got, Ok(want(Verb::Open, &docs, None, None)));
}

#[test]
fn open_explorer_file_selects_it_in_explorer() {
    let f = Fixture::new();
    let p = f.master().join("descript.txt");
    let got = f.resolve(
        &dest("open", &["explorer", "descript.txt"]),
        &FakeOs::default(),
    );
    let params = format!("/select,\"{}\"", p.display());
    assert_eq!(
        got,
        Ok(want(Verb::Open, "explorer.exe", Some(&params), None))
    );
}

#[test]
fn open_editor_uses_edit_verb_and_ignores_line() {
    let f = Fixture::new();
    let p = f.master().join("descript.txt");
    let got = f.resolve(
        &dest("open", &["editor", "descript.txt", "3"]),
        &FakeOs::default(),
    );
    assert_eq!(got, Ok(want(Verb::Edit, &p, None, None)));
}

#[test]
fn open_mailer_adds_mailto_once() {
    let f = Fixture::new();
    let os = FakeOs::default();
    assert_eq!(
        f.resolve(&dest("open", &["mailer", "a@example.com"]), &os),
        Ok(want(Verb::Open, "mailto:a@example.com", None, None))
    );
    // 既に付いていれば（大文字小文字を区別しない）二重にしない。
    assert_eq!(
        f.resolve(&dest("open", &["mailer", "MailTo:a@example.com"]), &os),
        Ok(want(Verb::Open, "MailTo:a@example.com", None, None))
    );
}

/// 目録で引く用の根（ゴースト `g`＝`G`・シェル 2 つ・バルーン 2 つ）と、その文脈。
///
/// シェル `a` の name は `b`、シェル `b` の name は `Other`＝名前が先に当たることを見る。
fn named_fixture() -> (TempPath, OpenContext) {
    let tmp = TempPath::new("open-ext-named");
    let root = build_ghost_root(
        &tmp,
        ("g", "G"),
        &[("a", "b"), ("b", "Other")],
        &[("ba", "bb"), ("bb", "OtherB")],
    );
    let ctx = OpenContext {
        ghost: "G".to_owned(),
        ghost_dir: root.ghost_dir("g"),
        baseware: Some(root),
    };
    (tmp, ctx)
}

fn named(kind: &str, name: &str) -> Destination {
    dest("open", &["explorer", kind, name])
}

#[test]
fn named_ghost_by_name_then_folder() {
    let (_tmp, ctx) = named_fixture();
    let os = FakeOs::default();
    let dir = ctx.ghost_dir.clone();
    assert_eq!(
        resolve(&named("ghost", "G"), &ctx, &os),
        Ok(want(Verb::Open, &dir, None, None))
    );
    assert_eq!(
        resolve(&named("ghost", "g"), &ctx, &os),
        Ok(want(Verb::Open, &dir, None, None))
    );
}

#[test]
fn named_shell_by_name_then_folder() {
    let (_tmp, ctx) = named_fixture();
    let os = FakeOs::default();
    let shell = |f: &str| ctx.ghost_dir.join("shell").join(f);
    // name の `b` はフォルダ `b` より先にシェル `a` に当たる。
    assert_eq!(
        resolve(&named("shell", "b"), &ctx, &os),
        Ok(want(Verb::Open, shell("a"), None, None))
    );
    assert_eq!(
        resolve(&named("shell", "a"), &ctx, &os),
        Ok(want(Verb::Open, shell("a"), None, None))
    );
    assert_eq!(
        resolve(&named("shell", "Other"), &ctx, &os),
        Ok(want(Verb::Open, shell("b"), None, None))
    );
}

#[test]
fn named_balloon_by_name_then_folder() {
    let (_tmp, ctx) = named_fixture();
    let os = FakeOs::default();
    let root = ctx.baseware.clone().unwrap();
    assert_eq!(
        resolve(&named("balloon", "bb"), &ctx, &os),
        Ok(want(Verb::Open, root.balloon_dir("ba"), None, None))
    );
    assert_eq!(
        resolve(&named("balloon", "ba"), &ctx, &os),
        Ok(want(Verb::Open, root.balloon_dir("ba"), None, None))
    );
    assert_eq!(
        resolve(&named("balloon", "OtherB"), &ctx, &os),
        Ok(want(Verb::Open, root.balloon_dir("bb"), None, None))
    );
}

#[test]
fn named_miss_case_and_special_names_are_no_match() {
    let (_tmp, ctx) = named_fixture();
    let os = FakeOs::default();
    // 大文字小文字を区別する・特別な名前は解かない。
    for (kind, name) in [
        ("ghost", "nope"),
        ("ghost", "random"),
        ("shell", "OTHER"),
        ("shell", "random"),
        ("balloon", "otherb"),
        ("balloon", "lastinstalled"),
    ] {
        let dest = named(kind, name);
        let Target::NamedFolder { store, .. } = dest.target else {
            panic!("目録の行き先");
        };
        assert_eq!(
            resolve(&dest, &ctx, &os),
            Err(OpenFailure::NoMatch {
                store,
                name: name.to_owned()
            }),
            "{kind},{name}"
        );
    }
}

#[test]
fn named_without_baseware_root_fails() {
    let (_tmp, mut ctx) = named_fixture();
    ctx.baseware = None;
    let os = FakeOs::default();
    for (kind, name) in [("ghost", "G"), ("shell", "a"), ("balloon", "ba")] {
        assert_eq!(
            resolve(&named(kind, name), &ctx, &os),
            Err(OpenFailure::NoBasewareRoot),
            "{kind}"
        );
    }
}
