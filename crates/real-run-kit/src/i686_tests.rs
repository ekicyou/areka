//! `i686` の兄弟のテスト。頭だけを作った 32bit と 64bit の偽の PE を `target\` の下に置き、
//! 探す順・各失敗の文・改名の競り合いの見直しを確かめる（本物の建てた成果物は使わない）。

use super::*;
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

/// 閉包が panic した文を返す（panic しなければ失敗）。
fn panic_text(f: impl FnOnce() + std::panic::UnwindSafe) -> String {
    let err = std::panic::catch_unwind(f).expect_err("panic するはず");
    err.downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
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
fn dest_already_32bit_is_returned_as_is_without_looking_at_sources() {
    let bed = Bed::new();
    write_pe(&bed.dest(), I386, "dest");
    // 元を見に行けば環境変数の無い指し先で panic するはず＝見に行かないことの証。
    let got = bed.ensure(Some(&bed.other("missing.exe")));
    assert_eq!(got, bed.dest());
    assert_eq!(tag_of(&got), "dest");
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

// --- ensure: 環境変数（2.2・2.3） ---

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

// --- ensure: debug → release（2.2） ---

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

// --- ensure: どこにも無い（2.4） ---

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

#[test]
fn helper_spec_names_the_real_helper() {
    assert_eq!(HELPER.file_name, "shiori-host32-helper.exe");
    assert_eq!(HELPER.env_var, "HOST32_HELPER_EXE");
    assert_eq!(HELPER.package, "shiori-host32-helper");
}

// --- place: 一時名＋改名と、改名の競り合いの見直し（2.6） ---

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
fn rename_failure_passes_when_dest_is_already_32bit() {
    let bed = Bed::new();
    let src = bed.other("src.exe");
    write_pe(&src, I386, "src");
    write_pe(&bed.dest(), I386, "other-process");
    let _held = hold(&bed.dest());
    place(&src, &bed.dest());
    // 先に置かれた物がそのまま使われる。
    assert_eq!(tag_of(&bed.dest()), "other-process");
    no_tmp_left(&bed.dest_dir);
}

#[test]
fn rename_failure_fails_with_the_reason_when_dest_is_not_32bit() {
    let bed = Bed::new();
    let src = bed.other("src.exe");
    write_pe(&src, I386, "src");
    write_pe(&bed.dest(), AMD64, "x64");
    let held = hold(&bed.dest());
    let text = panic_text(|| place(&src, &bed.dest()));
    drop(held);
    assert!(text.contains(&bed.dest().display().to_string()), "{text}");
    assert!(text.contains("改名"), "{text}");
    no_tmp_left(&bed.dest_dir);
}
