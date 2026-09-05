//! `io/files.rs` の在中テスト。
//!
//! 守るのは 3 つ。⑴ 復帰文字を落とす整形が逐語で正しいこと（設計 D-6）。⑵ 読み込みが
//! その整形を実際に通っていること——本物の repo のファイル（この crate 自身の
//! `Cargo.toml`。`core.autocrlf` が効く環境では作業ツリーに復帰文字が入る）を読んで
//! 1 個も残っていないことを確かめる。⑶ 失敗が黙って通らないこと——読めないファイルの
//! 相手には探したパスを載せた失敗が返る（要件 6.12）。
//!
//! **ファイルは 1 つも作らず、一時ディレクトリも使わない**（設計 File Structure Plan）。
//! 整形は `&str` → `String` の純粋な関数に切り出してあるので、書き出しの本文の形も
//! ファイルを作らずに確かめられる。書き出しの失敗経路だけは、親ディレクトリが実在
//! しない場所を相手にする（この呼び出しは何も作らない）。

use std::path::PathBuf;

use super::*;

/// 本物の作業ツリーにある、この crate 自身の manifest。
fn own_manifest() -> PathBuf {
    crate::io::paths::workspace_root()
        .join("crates")
        .join("ukadoc-survey")
        .join("Cargo.toml")
}

// ---- 復帰文字を落とす整形（読み書きに共通）----

#[test]
fn strip_cr_turns_crlf_into_lf() {
    assert_eq!(strip_cr("a\r\nb\r\nc"), "a\nb\nc");
}

#[test]
fn strip_cr_drops_a_lone_carriage_return() {
    assert_eq!(strip_cr("a\rb"), "ab");
}

#[test]
fn strip_cr_leaves_lf_only_text_untouched() {
    let body = "[ledger]\ndomain = \"shiori\"\n\npages = []\n";
    assert_eq!(strip_cr(body), body);
}

#[test]
fn strip_cr_leaves_no_carriage_return_behind() {
    let body = "\r\n見出し\r\n\r\n本文\r";
    assert!(
        !strip_cr(body).contains('\r'),
        "復帰文字が残っている: {:?}",
        strip_cr(body)
    );
}

/// 落とすのは復帰文字だけで、行の数は 1 行も増減しない。
#[test]
fn strip_cr_keeps_the_line_count() {
    let body = "1 行目\r\n2 行目\r\n3 行目\r\n";
    let got = strip_cr(body);
    assert_eq!(body.matches('\n').count(), got.matches('\n').count());
    assert_eq!(got, "1 行目\n2 行目\n3 行目\n");
}

/// 多バイト文字の本文でも中身を壊さない（台帳も報告も日本語が本体）。
#[test]
fn strip_cr_keeps_multibyte_text() {
    assert_eq!(
        strip_cr("壊れ方: 黙って壊れる。\r\n縮退の登記。\r\n"),
        "壊れ方: 黙って壊れる。\n縮退の登記。\n"
    );
}

/// 復帰文字を落としても復帰文字が現れることはない（冪等）。
#[test]
fn strip_cr_is_idempotent() {
    let body = "a\r\nb\rc\n";
    assert_eq!(strip_cr(&strip_cr(body)), strip_cr(body));
}

// ---- 書き出す本文の形（改行だけ）----

#[test]
fn lf_body_writes_only_line_feeds() {
    assert_eq!(lf_body("a\r\nb\r\n"), "a\nb\n");
    assert!(!lf_body("a\r\nb\r\n").contains('\r'));
}

/// 呼び出し側が復帰文字を混ぜても、ファイルへ渡る本文には入らない。
#[test]
fn lf_body_and_read_normalized_agree_on_shape() {
    let body = "見出し\r\n\r\n本文\r\n";
    assert_eq!(lf_body(body), strip_cr(body));
}

// ---- 読み込みが整形を通っていること（本物のファイルで確かめる）----

#[test]
fn read_normalized_returns_a_body_without_carriage_returns() {
    let path = own_manifest();
    let body = read_normalized(&path).expect("自分の manifest は読めるはず");
    assert!(
        !body.contains('\r'),
        "復帰文字が残っている: {}",
        path.display()
    );
    // 否定の主張だけだと、空文字列を返す実装でも緑になる。中身があることを対で示す。
    assert!(
        body.contains("[package]") && body.contains("ukadoc-survey"),
        "manifest の中身が読めていない: {}",
        path.display()
    );
    assert!(
        body.lines().count() > 5,
        "行が少なすぎる: {}",
        path.display()
    );
}

/// 読み込みが整形を**実際に通している**ことを、復帰文字を持つ本物のファイルで確かめる。
///
/// この repo は `core.autocrlf` が効くので、既存の追跡ファイルは作業ツリーで復帰文字を
/// 持つ。持っているファイルに出会ったら、⑴ 返る本文に復帰文字が 1 個も無いこと
/// ⑵ 減った文字数が元の復帰文字の個数とちょうど一致すること（＝落としたのは復帰文字
/// だけで、他の文字は 1 つも消えていない）を確かめる。
///
/// 改行だけで取り出される環境（`core.autocrlf` が無効な clone）では復帰文字を持つ
/// ファイルが無く、この較正は素通りする。そこでも意味が残るよう、どのファイルでも
/// 成り立つ性質（復帰文字が無い・本文が空でない）は無条件に確かめる。
#[test]
fn read_normalized_actually_removes_the_carriage_returns_a_file_carries() {
    // 本 spec の実装より前から repo にある追跡ファイル（新しく書いたものではない）。
    let samples = [
        "Cargo.toml",
        "crates/areka-sylphya/src/lib.rs",
        "crates/log-capture-kit/tests/workspace_scan/mod.rs",
    ];
    let root = crate::io::paths::workspace_root();
    for sample in samples {
        let path = root.join(sample);
        let raw = std::fs::read(&path).unwrap_or_else(|err| panic!("読めない: {sample} ({err})"));
        let raw =
            String::from_utf8(raw).unwrap_or_else(|err| panic!("UTF-8 でない: {sample} ({err})"));
        let body =
            read_normalized(&path).unwrap_or_else(|err| panic!("読めない: {sample} ({err})"));

        assert!(!body.is_empty(), "本文が空: {sample}");
        assert!(!body.contains('\r'), "復帰文字が残っている: {sample}");

        let carried = raw.matches('\r').count();
        assert_eq!(
            raw.chars().count() - body.chars().count(),
            carried,
            "落ちた文字数が復帰文字の個数と合わない: {sample}"
        );
        assert_eq!(
            raw.matches('\n').count(),
            body.matches('\n').count(),
            "改行の個数が変わっている: {sample}"
        );
    }
}

// ---- 失敗が黙って通らないこと ----

#[test]
fn read_normalized_reports_the_path_it_could_not_read() {
    let missing = crate::io::paths::workspace_root()
        .join("crates")
        .join("ukadoc-survey")
        .join("this-file-does-not-exist.toml");
    assert!(!missing.exists(), "前提が崩れている: {}", missing.display());
    let err = read_normalized(&missing).expect_err("無いファイルは読めないはず");
    match &err {
        SurveyError::Io { path, reason } => {
            assert!(
                path.contains("this-file-does-not-exist.toml"),
                "探したパスが載っていない: {path}"
            );
            assert!(!reason.is_empty(), "理由が空");
        }
        other => panic!("読み書きの失敗として返るはず: {other:?}"),
    }
    assert!(
        err.to_string().contains("this-file-does-not-exist.toml"),
        "本文にパスが出ない: {err}"
    );
}

/// 書けない場所を相手にしても黙って成功しない。親ディレクトリが実在しないので、
/// この呼び出しはファイルもディレクトリも 1 つも作らない。
#[test]
fn write_lf_reports_the_path_it_could_not_write() {
    let target = crate::io::paths::workspace_root()
        .join("no-such-dir-for-ukadoc-survey-tests")
        .join("out.md");
    assert!(
        !target.parent().expect("親がある").exists(),
        "前提が崩れている: {}",
        target.display()
    );
    let err = write_lf(&target, "本文\n").expect_err("親が無い場所へは書けないはず");
    match &err {
        SurveyError::Io { path, reason } => {
            assert!(
                path.contains("out.md"),
                "書こうとしたパスが載っていない: {path}"
            );
            assert!(!reason.is_empty(), "理由が空");
        }
        other => panic!("読み書きの失敗として返るはず: {other:?}"),
    }
    assert!(
        !target.exists(),
        "何も作られていないはず: {}",
        target.display()
    );
}
