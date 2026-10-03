//! 起動した exe のリンクを辿る判断（`follow_exe_links`）の決定論テスト（要件 6.1〜6.3・6.5〜6.7）。
//!
//! 実物のシンボリックリンクは作らない（管理者でないと作れない）。偽の `probe`（パス → 結果の表）を
//! 注入して判断の分岐を全部踏み、`probe` が読んだパスの列も見る。

use std::path::{Path, PathBuf};

use super::*;

/// 表の 1 行の中身。表に無いパスは「リンクではない」。
enum Fake {
    /// リンクで、先はこの綴り。
    Link(&'static str),
    /// 読めない（理由の文）。
    Bad(&'static str),
}

/// 表を偽の `probe` にして `follow_exe_links` を回し、戻りと `probe` が読んだパスの列を返す。
fn run(exe: &str, table: &[(&str, Fake)]) -> ((PathBuf, Option<ExeLinkWarning>), Vec<PathBuf>) {
    let mut calls = Vec::new();
    let mut probe = |path: &Path| {
        calls.push(path.to_path_buf());
        match table.iter().find(|(p, _)| Path::new(p) == path) {
            Some((_, Fake::Link(t))) => LinkProbe::Target(PathBuf::from(t)),
            Some((_, Fake::Bad(r))) => LinkProbe::Unreadable(r.to_string()),
            None => LinkProbe::NotALink,
        }
    };
    let out = follow_exe_links(Path::new(exe), &mut probe);
    (out, calls)
}

/// 分岐 1: リンクでない → 入力の綴りそのまま・警告なし・読むのは 1 回（要件 6.3）。
/// 普通の綴りと、`subst` を模した別のドライブ文字の綴りのどちらも書き換えない。
#[test]
fn not_a_link_keeps_the_spelling() {
    for exe in [r"C:\x\areka.exe", r"S:\x\areka.exe"] {
        let (out, calls) = run(exe, &[]);
        assert_eq!(out, (PathBuf::from(exe), None), "{exe}");
        assert_eq!(calls, vec![PathBuf::from(exe)], "{exe}");
    }
}

/// 分岐 2: 絶対の先を 1 段 → 先のパス・警告なし（要件 6.1）。
#[test]
fn absolute_target_one_hop() {
    let (out, calls) = run(
        r"C:\links\areka.exe",
        &[(r"C:\links\areka.exe", Fake::Link(r"D:\pkg\areka.exe"))],
    );
    assert_eq!(out, (PathBuf::from(r"D:\pkg\areka.exe"), None));
    assert_eq!(calls.len(), 2);
}

/// 分岐 3: 相対の先 → リンクの親と結合し `..` を畳んだ絶対パス（要件 6.2）。
#[test]
fn relative_target_is_joined_to_the_link_parent_and_folded() {
    let (out, calls) = run(
        r"C:\links\sub\areka.exe",
        &[(r"C:\links\sub\areka.exe", Fake::Link(r"..\pkg\areka.exe"))],
    );
    assert_eq!(out, (PathBuf::from(r"C:\links\pkg\areka.exe"), None));
    assert_eq!(calls[1], PathBuf::from(r"C:\links\pkg\areka.exe"));
}

/// 分岐 4: リンクのリンク（2 段・2 段目は相対）→ 最後のパス（要件 6.2）。
#[test]
fn two_hop_chain_reaches_the_last_target() {
    let (out, calls) = run(
        r"C:\links\areka.exe",
        &[
            (r"C:\links\areka.exe", Fake::Link(r"D:\mid\areka.exe")),
            (r"D:\mid\areka.exe", Fake::Link(r"..\real\areka.exe")),
        ],
    );
    assert_eq!(out, (PathBuf::from(r"D:\real\areka.exe"), None));
    assert_eq!(calls.len(), 3);
}

/// 分岐 5: 輪になったリンク → `TooManyHops { limit: 32 }`・戻りは入力（要件 6.6）。
/// 上限に達したら次を読まない＝`probe` はちょうど 32 回。
#[test]
fn cycle_stops_at_the_hop_limit_without_reading_further() {
    let (out, calls) = run(
        r"C:\a\areka.exe",
        &[
            (r"C:\a\areka.exe", Fake::Link(r"C:\b\areka.exe")),
            (r"C:\b\areka.exe", Fake::Link(r"C:\a\areka.exe")),
        ],
    );
    assert_eq!(
        out,
        (
            PathBuf::from(r"C:\a\areka.exe"),
            Some(ExeLinkWarning::TooManyHops {
                limit: 32,
                last: PathBuf::from(r"C:\a\areka.exe"),
            })
        )
    );
    assert_eq!(EXE_LINK_MAX_HOPS, 32);
    assert_eq!(calls.len(), 32);
}

/// 分岐 6: 途中で読めない → `Unreadable { link, reason }`・戻りは途中の先でなく入力（要件 6.6）。
#[test]
fn unreadable_link_falls_back_to_the_input() {
    let (out, _) = run(
        r"C:\links\areka.exe",
        &[
            (r"C:\links\areka.exe", Fake::Link(r"D:\pkg\areka.exe")),
            (r"D:\pkg\areka.exe", Fake::Bad("access denied")),
        ],
    );
    assert_eq!(
        out,
        (
            PathBuf::from(r"C:\links\areka.exe"),
            Some(ExeLinkWarning::Unreadable {
                link: PathBuf::from(r"D:\pkg\areka.exe"),
                reason: "access denied".to_string(),
            })
        )
    );
}

/// 分岐 7: 先が `\\?\C:\…` の綴り → `VerbatimPrefix`・戻りは入力（要件 6.5）。
#[test]
fn verbatim_prefixed_target_falls_back_to_the_input() {
    let (out, _) = run(
        r"C:\links\areka.exe",
        &[(r"C:\links\areka.exe", Fake::Link(r"\\?\C:\pkg\areka.exe"))],
    );
    assert_eq!(
        out,
        (
            PathBuf::from(r"C:\links\areka.exe"),
            Some(ExeLinkWarning::VerbatimPrefix {
                path: PathBuf::from(r"\\?\C:\pkg\areka.exe"),
            })
        )
    );
}
