//! 行き先の規則のテスト（areka-P0-open-external-tags task 2.1・要件 1.1・1.4・1.7・2.6・
//! 3.2・4.7・5.2・6.3・8.2・8.3・10.3）。
//!
//! 設計の「分類の表」の全行を 1 つの表で判定する。

use super::{OpenKind, Rejection, Store, Target, classify};
use areka_parsers::sakura::JUMP_TAG_CARRIER;

/// 1 行の期待値。
enum Want {
    /// 受理：行き先・書かれた綴り・元のタグの組み直し・種類。
    Open(Target, &'static str, &'static str, OpenKind),
    /// 断り：元のタグの組み直し・理由。
    Reject(&'static str, Rejection),
    /// 開く系の対象外（`None`）。
    Skip,
}

fn url(s: &str) -> Target {
    Target::Url(s.to_owned())
}

fn named(store: Store, name: &str) -> Target {
    Target::NamedFolder {
        store,
        name: name.to_owned(),
    }
}

/// (コマンド名, 引数の列, 期待値)。
fn table() -> Vec<(&'static str, Vec<&'static str>, Want)> {
    use Want::*;
    let j = JUMP_TAG_CARRIER;
    vec![
        // ── \j[ID] の 3 形（前置きは大文字小文字を区別しない）
        (
            j,
            vec!["http://a/"],
            Open(
                url("http://a/"),
                "http://a/",
                "\\j[http://a/]",
                OpenKind::Url,
            ),
        ),
        (
            j,
            vec!["HTTPS://A/b"],
            Open(
                url("HTTPS://A/b"),
                "HTTPS://A/b",
                "\\j[HTTPS://A/b]",
                OpenKind::Url,
            ),
        ),
        (
            j,
            vec!["mailto:a@b.c"],
            Open(
                Target::Mail("mailto:a@b.c".into()),
                "mailto:a@b.c",
                "\\j[mailto:a@b.c]",
                OpenKind::Mail,
            ),
        ),
        (
            j,
            vec!["MailTo:a@b.c"],
            Open(
                Target::Mail("MailTo:a@b.c".into()),
                "MailTo:a@b.c",
                "\\j[MailTo:a@b.c]",
                OpenKind::Mail,
            ),
        ),
        (
            j,
            vec!["file:///descript.txt"],
            Open(
                Target::Path("descript.txt".into()),
                "file:///descript.txt",
                "\\j[file:///descript.txt]",
                OpenKind::File,
            ),
        ),
        (
            j,
            vec!["File:///C:\\a b\\c.txt"],
            Open(
                Target::Path("C:\\a b\\c.txt".into()),
                "File:///C:\\a b\\c.txt",
                "\\j[File:///C:\\a b\\c.txt]",
                OpenKind::File,
            ),
        ),
        // ID は第 1 引数だけ（綴りは全引数から組み直す）
        (
            j,
            vec!["http://a/", "x"],
            Open(
                url("http://a/"),
                "http://a/",
                "\\j[http://a/,x]",
                OpenKind::Url,
            ),
        ),
        // ── file:/// の残りが空
        (
            j,
            vec!["file:///"],
            Reject("\\j[file:///]", Rejection::MissingArgument),
        ),
        // ── 3 形以外・空・ID なし
        (
            j,
            vec!["ftp://a/"],
            Reject("\\j[ftp://a/]", Rejection::UnknownJumpId),
        ),
        (
            j,
            vec!["http:/a"],
            Reject("\\j[http:/a]", Rejection::UnknownJumpId),
        ),
        (
            j,
            vec!["anchor"],
            Reject("\\j[anchor]", Rejection::UnknownJumpId),
        ),
        (j, vec![""], Reject("\\j[]", Rejection::UnknownJumpId)),
        (j, vec![], Reject("\\j[]", Rejection::UnknownJumpId)),
        // ── \![open,file,X]
        (
            "open",
            vec!["file", "shell/master/surface0.png"],
            Open(
                Target::Program("shell/master/surface0.png".into()),
                "shell/master/surface0.png",
                "\\![open,file,shell/master/surface0.png]",
                OpenKind::File,
            ),
        ),
        (
            "open",
            vec!["file", "%TEMP%\\a.txt"],
            Open(
                Target::Program("%TEMP%\\a.txt".into()),
                "%TEMP%\\a.txt",
                "\\![open,file,%TEMP%\\a.txt]",
                OpenKind::File,
            ),
        ),
        // ── \![open,browser,X]
        (
            "open",
            vec!["browser", "https://x/"],
            Open(
                url("https://x/"),
                "https://x/",
                "\\![open,browser,https://x/]",
                OpenKind::Url,
            ),
        ),
        // ── \![open,explorer,X]
        (
            "open",
            vec!["explorer", "C:\\x"],
            Open(
                Target::Folder("C:\\x".into()),
                "C:\\x",
                "\\![open,explorer,C:\\x]",
                OpenKind::Folder,
            ),
        ),
        // ── \![open,explorer,種類,名前] の 3 種
        (
            "open",
            vec!["explorer", "ghost", "emo2"],
            Open(
                named(Store::Ghost, "emo2"),
                "ghost,emo2",
                "\\![open,explorer,ghost,emo2]",
                OpenKind::Folder,
            ),
        ),
        (
            "open",
            vec!["explorer", "balloon", "SSP既定"],
            Open(
                named(Store::Balloon, "SSP既定"),
                "balloon,SSP既定",
                "\\![open,explorer,balloon,SSP既定]",
                OpenKind::Folder,
            ),
        ),
        (
            "open",
            vec!["explorer", "shell", "master"],
            Open(
                named(Store::Shell, "master"),
                "shell,master",
                "\\![open,explorer,shell,master]",
                OpenKind::Folder,
            ),
        ),
        // ── headline／plugin／知らない種類
        (
            "open",
            vec!["explorer", "headline", "x"],
            Reject(
                "\\![open,explorer,headline,x]",
                Rejection::UnsupportedStore("headline".into()),
            ),
        ),
        (
            "open",
            vec!["explorer", "plugin", "x"],
            Reject(
                "\\![open,explorer,plugin,x]",
                Rejection::UnsupportedStore("plugin".into()),
            ),
        ),
        (
            "open",
            vec!["explorer", "foo", "x"],
            Reject(
                "\\![open,explorer,foo,x]",
                Rejection::UnsupportedStore("foo".into()),
            ),
        ),
        // 名前が空
        (
            "open",
            vec!["explorer", "ghost", ""],
            Reject("\\![open,explorer,ghost,]", Rejection::MissingArgument),
        ),
        // ── \![open,editor,X,表示行]（表示行は読まない）
        (
            "open",
            vec!["editor", "a.txt", "12"],
            Open(
                Target::Edit("a.txt".into()),
                "a.txt",
                "\\![open,editor,a.txt,12]",
                OpenKind::Editor,
            ),
        ),
        (
            "open",
            vec!["editor", "a.txt"],
            Open(
                Target::Edit("a.txt".into()),
                "a.txt",
                "\\![open,editor,a.txt]",
                OpenKind::Editor,
            ),
        ),
        // ── \![open,mailer,X]
        (
            "open",
            vec!["mailer", "test@example.com"],
            Open(
                Target::Mail("test@example.com".into()),
                "test@example.com",
                "\\![open,mailer,test@example.com]",
                OpenKind::Mail,
            ),
        ),
        // ── 引数なし・空（5 つの形）
        (
            "open",
            vec!["file"],
            Reject("\\![open,file]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["file", ""],
            Reject("\\![open,file,]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["browser"],
            Reject("\\![open,browser]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["browser", ""],
            Reject("\\![open,browser,]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["explorer"],
            Reject("\\![open,explorer]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["explorer", ""],
            Reject("\\![open,explorer,]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["editor"],
            Reject("\\![open,editor]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["editor", "", "3"],
            Reject("\\![open,editor,,3]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["mailer"],
            Reject("\\![open,mailer]", Rejection::MissingArgument),
        ),
        (
            "open",
            vec!["mailer", ""],
            Reject("\\![open,mailer,]", Rejection::MissingArgument),
        ),
        // ── 対象外
        ("open", vec!["readme"], Skip),
        ("open", vec!["readme", "ghost", "emo2"], Skip),
        ("open", vec!["help"], Skip),
        ("open", vec![], Skip),
        ("change", vec!["ghost", "emo2"], Skip),
        ("\\f", vec!["bold"], Skip),
    ]
}

#[test]
fn classify_follows_every_row_of_the_table() {
    for (name, args, want) in table() {
        let got = classify(name, &args);
        let at = format!("{name} {args:?}");
        match (want, got) {
            (Want::Skip, None) => {}
            (Want::Open(target, written, tag, kind), Some(Ok(d))) => {
                assert_eq!(d.target, target, "{at}");
                assert_eq!(d.written, written, "{at}");
                assert_eq!(d.tag, tag, "{at}");
                assert_eq!(d.target.kind(), kind, "{at}");
            }
            (Want::Reject(tag, reason), Some(Err(r))) => {
                assert_eq!(r.tag, tag, "{at}");
                assert_eq!(r.reason, reason, "{at}");
            }
            (_, got) => panic!("{at}: 期待と違う結果 {got:?}"),
        }
    }
}

#[test]
fn classify_takes_string_args_as_well() {
    let args = vec![String::from("browser"), String::from("https://x/")];
    let d = classify("open", &args).unwrap().unwrap();
    assert_eq!(d.target, url("https://x/"));
}

#[test]
fn open_kind_strings_are_the_log_field_values() {
    let all = [
        (OpenKind::Url, "url"),
        (OpenKind::File, "file"),
        (OpenKind::Folder, "folder"),
        (OpenKind::Mail, "mail"),
        (OpenKind::Editor, "editor"),
    ];
    for (kind, s) in all {
        assert_eq!(kind.as_str(), s);
    }
}
