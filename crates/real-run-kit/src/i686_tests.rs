//! `i686` の兄弟のテスト。頭だけを作った 32bit と 64bit の偽の PE を `target\` の下に置き、
//! 探す順・各失敗の文・改名の競り合いの見直しを確かめる（本物の建てた成果物は使わない）。

use super::*;
use crate::test_support::panic_text;
use std::os::windows::fs::OpenOptionsExt;
use temp_path_kit::TempPath;

/// 試験の定め（本物の仲介の名前と混ざらない名前）。
const SPEC: I686Spec = I686Spec {
    file_name: "fake-helper.exe",
    env_var: "FAKE_HELPER_EXE",
    package: "fake-helper-package",
};

/// PE の頭だけのファイル。`e_lfanew`＝0x40 の先に `PE\0\0` と機械の種類。
/// `tag` は中身の見分け（どの元から写したかを確かめる）。
fn fake_pe(machine: u16, tag: &str) -> Vec<u8> {
    let mut b = vec![0u8; 0x40];
    b[0] = b'M';
    b[1] = b'Z';
    b[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
    b.extend_from_slice(b"PE\0\0");
    b.extend_from_slice(&machine.to_le_bytes());
    b.extend_from_slice(tag.as_bytes());
    b
}

fn write_pe(path: &Path, machine: u16, tag: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, fake_pe(machine, tag)).unwrap();
}

const I386: u16 = 0x014c;
const AMD64: u16 = 0x8664;

/// 1 回の場: `<root>\i686`（i686 の target）と `<root>\dest`（置き先）。
struct Bed {
    _root: TempPath,
    i686: PathBuf,
    dest_dir: PathBuf,
}

impl Bed {
    fn new() -> Self {
        let root = TempPath::under_target("rr-i686");
        let i686 = root.child("i686");
        let dest_dir = root.child("dest");
        std::fs::create_dir_all(&dest_dir).unwrap();
        Bed {
            _root: root,
            i686,
            dest_dir,
        }
    }
    fn debug(&self) -> PathBuf {
        self.i686.join("debug").join(SPEC.file_name)
    }
    fn release(&self) -> PathBuf {
        self.i686.join("release").join(SPEC.file_name)
    }
    fn dest(&self) -> PathBuf {
        self.dest_dir.join(SPEC.file_name)
    }
    fn other(&self, name: &str) -> PathBuf {
        self.i686.join(name)
    }
    fn ensure(&self, env: Option<&Path>) -> PathBuf {
        ensure(
            &SPEC,
            env.map(|p| p.as_os_str().to_owned()),
            &self.i686,
            &self.dest_dir,
        )
    }
}

/// 写された中身の見分けの札。
fn tag_of(path: &Path) -> String {
    String::from_utf8_lossy(&std::fs::read(path).unwrap()[0x46..]).into_owned()
}

/// 置き先に一時名の残りが無い。
fn no_tmp_left(dir: &Path) {
    let left: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|n| n.to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(left.is_empty(), "一時名が残っています: {left:?}");
}

// --- is_i686_pe ---

#[test]
fn is_i686_pe_reads_the_machine_type() {
    let bed = Bed::new();
    let x86 = bed.other("x86.exe");
    let x64 = bed.other("x64.exe");
    write_pe(&x86, I386, "x86");
    write_pe(&x64, AMD64, "x64");
    assert!(is_i686_pe(&x86));
    assert!(!is_i686_pe(&x64));
}

#[test]
fn is_i686_pe_is_false_for_missing_and_short_files() {
    let bed = Bed::new();
    assert!(!is_i686_pe(&bed.other("missing.exe")));
    // e_lfanew まで届かない。
    let short = bed.other("short.exe");
    std::fs::create_dir_all(short.parent().unwrap()).unwrap();
    std::fs::write(&short, b"MZ").unwrap();
    assert!(!is_i686_pe(&short));
    // e_lfanew が先を指すが、そこまで無い。
    let cut = bed.other("cut.exe");
    let mut b = fake_pe(I386, "");
    b.truncate(0x44);
    std::fs::write(&cut, b).unwrap();
    assert!(!is_i686_pe(&cut));
    // フォルダは読めない。
    assert!(!is_i686_pe(&bed.dest_dir));
}

// --- ensure: 置き先 ---

#[test]
fn an_env_var_pointing_at_a_non_i686_file_panics_even_when_the_neighbour_is_i686() {
    let bed = Bed::new();
    write_pe(&bed.dest(), I386, "dest");
    let env = bed.other("env64.exe");
    write_pe(&env, AMD64, "env");
    // 隣が 32bit でも環境変数を必ず確かめる（打ち間違いを黙って見逃さない）。
    let text = panic_text(|| {
        bed.ensure(Some(&env));
    });
    assert!(text.contains(SPEC.env_var), "{text}");
    assert!(text.contains(&env.display().to_string()), "{text}");
    assert_eq!(tag_of(&bed.dest()), "dest");
}

#[test]
fn without_any_source_the_i686_neighbour_is_used() {
    let bed = Bed::new();
    write_pe(&bed.dest(), I386, "dest");
    let got = bed.ensure(None);
    assert_eq!(got, bed.dest());
    assert_eq!(tag_of(&got), "dest");
}

#[test]
fn a_newer_source_replaces_a_stale_i686_neighbour() {
    let bed = Bed::new();
    // 長さは同じ・更新時刻だけが古い隣（前の回に置いた写し）。
    write_pe(&bed.dest(), I386, "old");
    set_mtime(&bed.dest(), std::time::SystemTime::UNIX_EPOCH);
    write_pe(&bed.debug(), I386, "new");
    let got = bed.ensure(None);
    assert_eq!(tag_of(&got), "new");
    no_tmp_left(&bed.dest_dir);
}

#[test]
fn a_neighbour_matching_the_source_is_not_copied_again() {
    let bed = Bed::new();
    write_pe(&bed.debug(), I386, "debug");
    bed.ensure(None);
    // 長さと更新時刻を保ったまま隣の札だけ替える。2 回目に写せば札が戻る。
    let mtime = std::fs::metadata(bed.dest()).unwrap().modified().unwrap();
    write_pe(&bed.dest(), I386, "debuX");
    set_mtime(&bed.dest(), mtime);
    bed.ensure(None);
    assert_eq!(tag_of(&bed.dest()), "debuX");
}

fn set_mtime(path: &Path, t: std::time::SystemTime) {
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(t)
        .unwrap();
}

#[test]
fn dest_64bit_is_replaced_by_a_32bit_source() {
    let bed = Bed::new();
    write_pe(&bed.dest(), AMD64, "x64");
    write_pe(&bed.debug(), I386, "debug");
    let got = bed.ensure(None);
    assert_eq!(got, bed.dest());
    assert!(is_i686_pe(&got));
    assert_eq!(tag_of(&got), "debug");
    no_tmp_left(&bed.dest_dir);
}

// --- ensure: 環境変数 ---

#[test]
fn env_pointing_to_32bit_wins_over_debug_and_release() {
    let bed = Bed::new();
    let env = bed.other("env.exe");
    write_pe(&env, I386, "env");
    write_pe(&bed.debug(), I386, "debug");
    write_pe(&bed.release(), I386, "release");
    let got = bed.ensure(Some(&env));
    assert_eq!(tag_of(&got), "env");
    no_tmp_left(&bed.dest_dir);
}

#[test]
fn env_pointing_to_64bit_fails_with_the_value_and_does_not_fall_back() {
    let bed = Bed::new();
    let env = bed.other("env64.exe");
    write_pe(&env, AMD64, "env");
    write_pe(&bed.debug(), I386, "debug");
    let text = panic_text(|| {
        bed.ensure(Some(&env));
    });
    assert!(text.contains(SPEC.env_var), "{text}");
    assert!(text.contains(&env.display().to_string()), "{text}");
    assert!(!bed.dest().exists(), "黙って debug へ移った");
}

#[test]
fn env_pointing_to_nothing_fails_with_the_value_and_does_not_fall_back() {
    let bed = Bed::new();
    let env = bed.other("missing.exe");
    write_pe(&bed.release(), I386, "release");
    let text = panic_text(|| {
        bed.ensure(Some(&env));
    });
    assert!(text.contains(SPEC.env_var), "{text}");
    assert!(text.contains(&env.display().to_string()), "{text}");
    assert!(!bed.dest().exists(), "黙って release へ移った");
}

// --- ensure: debug → release ---

#[test]
fn debug_only_is_used() {
    let bed = Bed::new();
    write_pe(&bed.debug(), I386, "debug");
    assert_eq!(tag_of(&bed.ensure(None)), "debug");
}

#[test]
fn release_only_is_used() {
    let bed = Bed::new();
    write_pe(&bed.release(), I386, "release");
    assert_eq!(tag_of(&bed.ensure(None)), "release");
}

#[test]
fn debug_that_is_not_32bit_is_skipped_for_release() {
    let bed = Bed::new();
    write_pe(&bed.debug(), AMD64, "debug64");
    write_pe(&bed.release(), I386, "release");
    assert_eq!(tag_of(&bed.ensure(None)), "release");
}

// --- ensure: どこにも無い ---

#[test]
fn nowhere_fails_with_the_searched_places_and_the_build_command() {
    let bed = Bed::new();
    // 64bit の debug は「在る」が 32bit ではないので見つからない扱い。
    write_pe(&bed.debug(), AMD64, "debug64");
    let text = panic_text(|| {
        bed.ensure(None);
    });
    assert!(text.contains(SPEC.file_name), "{text}");
    assert!(text.contains(SPEC.env_var), "{text}");
    assert!(text.contains(&bed.debug().display().to_string()), "{text}");
    assert!(
        text.contains(&bed.release().display().to_string()),
        "{text}"
    );
    assert!(
        text.contains("cargo build -p fake-helper-package --target i686-pc-windows-msvc"),
        "{text}"
    );
}

// --- place: 一時名＋改名と、改名の競り合いの見直し ---

/// 削除の共有を許さずに開く＝改名で置き換えられない（動いている子が使っている状態）。
fn hold(path: &Path) -> std::fs::File {
    const FILE_SHARE_READ: u32 = 0x1;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
        .unwrap()
}

#[test]
fn rename_failure_passes_silently_when_dest_is_the_same_32bit_copy() {
    let bed = Bed::new();
    let src = bed.other("src.exe");
    write_pe(&src, I386, "src");
    // `std::fs::copy` は更新時刻も写すので、長さと更新時刻が元と同じ写しになる。
    std::fs::copy(&src, bed.dest()).unwrap();
    let _held = hold(&bed.dest());
    let mut out = Vec::new();
    place(&src, &bed.dest(), &mut out);
    assert!(out.is_empty(), "{:?}", String::from_utf8_lossy(&out));
    assert_eq!(tag_of(&bed.dest()), "src");
    no_tmp_left(&bed.dest_dir);
}

#[test]
fn rename_failure_passes_with_an_ascii_line_when_dest_is_another_32bit_copy() {
    let bed = Bed::new();
    let src = bed.other("src.exe");
    write_pe(&src, I386, "src");
    // 置き先のフォルダ名を ASCII の外の字にする（行のパスの部分が写しを通ることを見る）。
    let dest_dir = bed.dest_dir.join("置き先");
    let dest = dest_dir.join(SPEC.file_name);
    write_pe(&dest, I386, "other-process");
    let _held = hold(&dest);
    let mut out = Vec::new();
    place(&src, &dest, &mut out);
    let text = String::from_utf8(out).unwrap();
    let line = text
        .strip_suffix('\n')
        .expect("古い写しのまま走る旨の 1 行");
    assert_eq!(line.lines().count(), 1, "{text:?}");
    // 先に置かれた物がそのまま使われる。
    assert_eq!(tag_of(&dest), "other-process");
    assert!(line.is_ascii() && line.contains("\\u{"), "{line}");
    assert!(
        line.starts_with("real-run-kit: could not replace ")
            && line.ends_with("; using the existing 32-bit copy"),
        "{line}"
    );
    assert!(line.contains(&ascii(&dest.display().to_string())), "{line}");
    no_tmp_left(&dest_dir);
}

#[test]
fn rename_failure_fails_with_the_reason_when_dest_is_not_32bit() {
    let bed = Bed::new();
    let src = bed.other("src.exe");
    write_pe(&src, I386, "src");
    write_pe(&bed.dest(), AMD64, "x64");
    let held = hold(&bed.dest());
    let text = panic_text(|| {
        place(&src, &bed.dest(), &mut Vec::new());
    });
    drop(held);
    assert!(text.contains(&bed.dest().display().to_string()), "{text}");
    assert!(text.contains("改名"), "{text}");
    no_tmp_left(&bed.dest_dir);
}

/// 一時名を消せないとき（ほかが削除の共有なしで開いている）は ASCII の 1 行、消せた・既に無いは無言。
#[test]
fn remove_tmp_reports_only_a_failure_other_than_not_found() {
    let bed = Bed::new();
    // フォルダ名を ASCII の外の字にする（行のパスの部分が写しを通ることを見る）。
    let tmp = bed.dest_dir.join("置き先").join("held.tmp");
    std::fs::create_dir_all(tmp.parent().unwrap()).unwrap();
    std::fs::write(&tmp, b"x").unwrap();
    let held = hold(&tmp);
    let line = remove_tmp(&tmp).expect("消せない旨の行");
    drop(held);
    assert!(line.is_ascii() && line.contains("\\u{"), "{line}");
    assert!(
        line.starts_with("real-run-kit: could not remove ")
            && line.contains(&ascii(&tmp.display().to_string())),
        "{line}"
    );
    assert_eq!(remove_tmp(&tmp), None);
    assert!(!tmp.exists());
    assert_eq!(remove_tmp(&tmp), None);
}
