//! `delete.txt` の固定入力（要件 6.1〜6.7・9.5）。

use super::*;
use crate::testkit::{hold, junction, sjis, tree, txt};
use sample_ghost_kit::WorkDir;

/// `ERROR_SHARING_VIOLATION`（`hold` で掴まれた物を取り除こうとした）。
const SHARING_VIOLATION: i32 = 32;

struct Fixture {
    _work: WorkDir,
    root: PathBuf,
    target: PathBuf,
    target_real: PathBuf,
}

fn fixture() -> Fixture {
    let work = WorkDir::new().expect("作業フォルダ");
    let root = work.path().to_path_buf();
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();
    let target_real = fs::canonicalize(&target).unwrap();
    Fixture {
        _work: work,
        root,
        target,
        target_real,
    }
}

impl Fixture {
    fn put(&self, rel: &str, bytes: &[u8]) {
        let p = self.target.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    fn apply(&self, charset: &'static Encoding) -> DeleteReport {
        apply(&self.target, &self.target_real, charset)
    }
}

/// 警告から `DeleteLineIgnored` の（ファイル, 行, 理由）だけを抜く。
fn ignored(r: &DeleteReport) -> Vec<(String, usize, DeleteWhy)> {
    r.warnings
        .iter()
        .filter_map(|w| match w {
            UpdateWarning::DeleteLineIgnored { file, line, why } => {
                Some((file.clone(), *line, *why))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn rejects_absolute_dotdot_escape_and_work_area_and_leaves_them() {
    let f = fixture();
    // 外側の実物（絶対パスの行とジャンクション越しの行が指す先）。
    let outside = f.root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("victim.txt"), b"keep").unwrap();
    junction(&f.target.join("j"), &outside);
    f.put("a/b.txt", b"keep");
    f.put(".update-work/1-0/new/x.txt", b"keep");
    let abs = outside.join("victim.txt").display().to_string();
    let lines = [
        "\\victim.txt",
        "/victim.txt",
        abs.as_str(),
        "\\\\server\\share\\victim.txt",
        "a\\..\\a\\b.txt",
        "j\\victim.txt",
        ".update-work\\1-0\\new\\x.txt",
        ".\\",
        ".",
    ];
    f.put("delete.txt", &txt(&lines));
    let before_outside = tree(&outside);

    let r = f.apply(encoding_rs::SHIFT_JIS);

    let d = |line, why| ("delete.txt".to_string(), line, why);
    assert_eq!(
        ignored(&r),
        vec![
            d(1, DeleteWhy::Absolute),
            d(2, DeleteWhy::Absolute),
            d(3, DeleteWhy::Absolute),
            d(4, DeleteWhy::Absolute),
            d(5, DeleteWhy::DotDot),
            d(6, DeleteWhy::EscapesTarget),
            d(7, DeleteWhy::InsideWorkArea),
            d(8, DeleteWhy::EscapesTarget),
            d(9, DeleteWhy::EscapesTarget),
        ]
    );
    assert_eq!(r.warnings.len(), 9);
    assert!(r.removed.is_empty());
    assert!(r.undeletable.is_empty());
    assert_eq!(tree(&outside), before_outside);
    assert!(f.target.join("a/b.txt").is_file());
    assert!(f.target.join(".update-work/1-0/new/x.txt").is_file());
}

/// Windows は区切り要素の末尾の `.`・空白を落とし、`:` は NTFS のストリーム指定になる。
/// 検査の綴りと OS が開く物がずれる行は全部拒否され、木は 1 バイトも変わらない。
#[test]
fn windows_trailing_dot_space_and_colon_lines_are_rejected() {
    let f = fixture();
    f.put("ghost.txt", b"g");
    f.put("sub/x.txt", b"x");
    f.put(".update-work/1-0/new/x.txt", b"w");
    let lines = [
        " /",
        ". \\",
        "...\\",
        "sub\\.. \\",
        "./.update-work/",
        ".update-work./",
        ".update-work \\",
        ".update-work::$INDEX_ALLOCATION\\",
    ];
    f.put("delete.txt", &txt(&lines));
    let before = tree(&f.target);

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert_eq!(tree(&f.target), before);
    assert!(r.removed.is_empty(), "{:?}", r.removed);
    assert!(r.undeletable.is_empty(), "{:?}", r.undeletable);
    let got = ignored(&r);
    assert_eq!(r.warnings.len(), lines.len(), "{:?}", r.warnings);
    assert_eq!(
        got.iter().map(|(_, line, _)| *line).collect::<Vec<_>>(),
        (1..=lines.len()).collect::<Vec<_>>()
    );
}

/// 8.3 短縮名（`UPDATE~1`）やジャンクションで作業場所へ届く行は、綴りの検査を抜けても
/// 実パスの検査で外扱いになり、何も取り除かない。
#[test]
fn lines_reaching_the_work_area_by_another_name_are_rejected() {
    let f = fixture();
    f.put(".update-work/1-0/orig.txt", b"o");
    f.put("ghost.txt", b"g");
    junction(&f.target.join("to-area"), &f.target.join(".update-work"));
    let mut lines = vec!["to-area\\".to_owned(), "to-area\\1-0\\orig.txt".to_owned()];
    match crate::testkit::short_name(&f.target.join(".update-work")) {
        Some(short) => {
            lines.push(format!("{short}\\"));
            lines.push(format!("{short}\\1-0\\orig.txt"));
        }
        None => eprintln!("SKIP 短縮名の 2 行: .update-work に 8.3 短縮名が無い"),
    }
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    f.put("delete.txt", &txt(&refs));
    let before_area = tree(&f.target.join(".update-work"));

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert!(r.removed.is_empty(), "{:?}", r.removed);
    assert!(r.undeletable.is_empty(), "{:?}", r.undeletable);
    assert_eq!(
        ignored(&r),
        (1..=lines.len())
            .map(|line| ("delete.txt".to_string(), line, DeleteWhy::EscapesTarget))
            .collect::<Vec<_>>()
    );
    assert_eq!(r.warnings.len(), lines.len());
    assert_eq!(tree(&f.target.join(".update-work")), before_area);
    assert!(f.target.join("ghost.txt").is_file());
    assert!(f.target.join("to-area").exists());
}

#[test]
fn folder_line_removes_folder_with_contents() {
    let f = fixture();
    f.put("dic/old/a.txt", b"a");
    f.put("dic/old/deep/b.txt", b"b");
    f.put("dic/keep.txt", b"k");
    f.put("delete.txt", &txt(&["dic/old/"]));

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(r.undeletable.is_empty());
    assert_eq!(r.removed, vec![f.target.join("dic/old")]);
    assert!(!f.target.join("dic/old").exists());
    assert!(f.target.join("dic/keep.txt").is_file());
}

#[test]
fn kind_mismatch_in_both_directions_leaves_the_thing_and_warns() {
    let f = fixture();
    f.put("sub/inner.txt", b"i");
    f.put("file.txt", b"f");
    // 1 行目: ファイルの行がフォルダを指す。2 行目: フォルダの行がファイルを指す。
    f.put("delete.txt", &txt(&["sub", "file.txt\\"]));
    let before = tree(&f.target);

    let r = f.apply(encoding_rs::SHIFT_JIS);

    let got: Vec<_> = r
        .warnings
        .iter()
        .map(|w| match w {
            UpdateWarning::DeleteKindMismatch { file, line, path } => {
                (file.clone(), *line, path.clone())
            }
            other => panic!("種別の食い違いだけのはず: {other:?}"),
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("delete.txt".to_string(), 1, f.target.join("sub")),
            ("delete.txt".to_string(), 2, f.target.join("file.txt")),
        ]
    );
    assert!(r.removed.is_empty());
    assert!(r.undeletable.is_empty());
    assert_eq!(tree(&f.target), before);
}

#[test]
fn missing_things_do_nothing() {
    let f = fixture();
    f.put("keep.txt", b"k");
    f.put(
        "delete.txt",
        &txt(&["nothing.txt", "none\\", "no/such/deep.txt"]),
    );
    let before = tree(&f.target);

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(r.removed.is_empty());
    assert!(r.undeletable.is_empty());
    assert_eq!(tree(&f.target), before);
}

#[test]
fn blank_and_whitespace_lines_are_ignored_and_file_line_removes_one_file() {
    let f = fixture();
    f.put("a.txt", b"a");
    f.put("b.txt", b"b");
    f.put("delete.txt", b"\r\n   \r\n\t\r\na.txt\r\n\n");

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(r.removed, vec![f.target.join("a.txt")]);
    assert!(!f.target.join("a.txt").exists());
    assert!(f.target.join("b.txt").is_file());
}

#[test]
fn delete_files_orders_numerically_and_ignores_other_names() {
    let f = fixture();
    for name in [
        "delete10.txt",
        "delete2.txt",
        "delete.txt",
        "delete1.txt",
        "deletea.txt",
        "delete-1.txt",
        "delete.txt.bak",
        "undelete.txt",
        "delete2.txt.txt",
    ] {
        f.put(name, b"");
    }
    let got: Vec<_> = delete_files(&f.target)
        .unwrap()
        .into_iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        got,
        ["delete.txt", "delete1.txt", "delete2.txt", "delete10.txt"]
    );
}

#[test]
fn delete2_is_applied_before_delete10() {
    let f = fixture();
    f.put("two.txt", b"2");
    f.put("ten.txt", b"10");
    f.put("delete10.txt", &txt(&["ten.txt"]));
    f.put("delete2.txt", &txt(&["two.txt"]));

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(
        r.removed,
        vec![f.target.join("two.txt"), f.target.join("ten.txt")]
    );
}

#[test]
fn shift_jis_lines_are_decoded_with_the_manifest_charset() {
    let f = fixture();
    f.put("辞書/古い辞書.txt", b"x");
    f.put("delete.txt", &sjis("辞書\\古い辞書.txt\r\n"));

    // 較正: 別の文字コードで読むと当たらない（引き継いだ文字コードで復号している証拠）。
    let wrong = f.apply(encoding_rs::UTF_8);
    assert!(wrong.removed.is_empty());
    assert!(f.target.join("辞書/古い辞書.txt").is_file());

    let r = f.apply(encoding_rs::SHIFT_JIS);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(r.removed, vec![f.target.join("辞書\\古い辞書.txt")]);
    assert!(!f.target.join("辞書/古い辞書.txt").exists());
}

#[test]
fn unreadable_delete_txt_is_warned_and_skipped() {
    let f = fixture();
    // 同名のフォルダは読めない（読み取りは os error 5）。
    fs::create_dir_all(f.target.join("delete.txt")).unwrap();
    f.put("a.txt", b"a");
    f.put("delete1.txt", &txt(&["a.txt"]));

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
    assert!(matches!(
        &r.warnings[0],
        UpdateWarning::DeleteFileUnreadable { file, .. } if file == "delete.txt"
    ));
    assert_eq!(r.removed, vec![f.target.join("a.txt")]);
    assert!(f.target.join("delete.txt").is_dir());
}

#[test]
fn undeletable_is_listed_and_the_rest_goes_on() {
    let f = fixture();
    f.put("held.txt", b"h");
    f.put("free.txt", b"f");
    f.put("delete.txt", &txt(&["held.txt", "free.txt"]));
    let _held = hold(&f.target.join("held.txt"));

    let r = f.apply(encoding_rs::SHIFT_JIS);

    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(r.undeletable.len(), 1);
    assert_eq!(r.undeletable[0].path, f.target.join("held.txt"));
    assert_eq!(
        r.undeletable[0].source.raw_os_error(),
        Some(SHARING_VIOLATION)
    );
    assert_eq!(r.removed, vec![f.target.join("free.txt")]);
    assert!(f.target.join("held.txt").is_file());
}

#[test]
fn no_delete_files_means_nothing() {
    let f = fixture();
    f.put("a.txt", b"a");
    let r = f.apply(encoding_rs::SHIFT_JIS);
    assert!(r.warnings.is_empty() && r.removed.is_empty() && r.undeletable.is_empty());
    assert!(f.target.join("a.txt").is_file());
}
