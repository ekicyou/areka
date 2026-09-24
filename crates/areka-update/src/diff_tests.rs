//! 差分の 4 形・読めないローカル・大きさの上限・外へ解決されるエントリ（要件 2.1・2.3・2.4・2.6・9.2）。

use super::*;
use crate::manifest::parse;
use crate::outcome::ManifestName;
use crate::testkit::{dau, junction, tree};
use sample_ghost_kit::WorkDir;

const MD5_ABC: &str = "900150983cd24fb0d6963f7d28e17f72";
const MD5_EMPTY: &str = "d41d8cd98f00b204e9800998ecf8427e";

struct Fixture {
    _work: WorkDir,
    target: PathBuf,
    target_real: PathBuf,
    outside: PathBuf,
}

fn fixture() -> Fixture {
    let work = WorkDir::new().expect("作業フォルダ");
    let target = work.path().join("target");
    let outside = work.path().join("outside");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let target_real = fs::canonicalize(&target).unwrap();
    Fixture {
        _work: work,
        target,
        target_real,
        outside,
    }
}

fn manifest(lines: &[&[&str]]) -> Manifest {
    let (m, warnings) = parse(ManifestName::Updates2Dau, &dau(lines, true));
    assert!(warnings.is_empty(), "固定入力に警告があってはならない");
    m
}

#[test]
fn missing_and_different_are_required_in_manifest_order_same_is_not() {
    let f = fixture();
    fs::create_dir_all(f.target.join("sub")).unwrap();
    fs::write(f.target.join("same.txt"), b"abc").unwrap();
    fs::write(f.target.join("sub/diff.txt"), b"abc").unwrap();
    // 定義に無いローカルのファイル（中身は定義のどの MD5 とも違う）。
    fs::write(f.target.join("extra.txt"), b"not in manifest").unwrap();
    fs::write(f.target.join("sub/extra2.bin"), [0u8, 1, 2]).unwrap();
    // 定義の順は名前順と逆（要取得の一覧が定義の順であることを見る）。
    let m = manifest(&[
        &["z_missing.txt", MD5_ABC],
        &["same.txt", MD5_ABC],
        &["sub/diff.txt", MD5_EMPTY],
        &["a_missing/deep.txt", MD5_ABC],
    ]);
    let before = tree(&f.target);

    let need = plan(&f.target, &f.target_real, &m).expect("差分を計算できる");

    assert_eq!(need, vec![0, 2, 3]);
    // 定義に無いファイルを含め、木はバイト単位で前後同一（2.3）。
    assert_eq!(tree(&f.target), before);
}

#[test]
fn nothing_required_when_all_entries_match() {
    let f = fixture();
    fs::write(f.target.join("a.txt"), b"abc").unwrap();
    fs::write(f.target.join("b.txt"), b"").unwrap();
    let m = manifest(&[&["a.txt", MD5_ABC], &["b.txt", MD5_EMPTY]]);

    assert_eq!(
        plan(&f.target, &f.target_real, &m).unwrap(),
        Vec::<usize>::new()
    );
}

#[test]
fn an_existing_but_unreadable_entry_fails_instead_of_becoming_required() {
    let f = fixture();
    fs::write(f.target.join("a.txt"), b"abc").unwrap();
    // 定義上はファイルの名前に、同名のフォルダが居る（在るのに読めない）。
    fs::create_dir_all(f.target.join("shell/surface0.png")).unwrap();
    let m = manifest(&[
        &["missing.txt", MD5_ABC],
        &["shell/surface0.png", MD5_ABC],
        &["a.txt", MD5_EMPTY],
    ]);

    match plan(&f.target, &f.target_real, &m) {
        Err(DiffFailure::Unreadable { path, .. }) => {
            assert_eq!(path, local_path(&f.target, "shell/surface0.png"));
        }
        Err(DiffFailure::Escapes { path }) => panic!("外へ解決されたと誤判定: {}", path.display()),
        Ok(need) => panic!("読めない物が要取得に化けた: {need:?}"),
    }
}

#[test]
fn size_limit_passes_at_the_limit_and_fails_one_past_it() {
    assert!(check_size(0, 10).is_ok());
    assert!(check_size(10, 10).is_ok());
    let err = check_size(11, 10).expect_err("上限＋1 は失敗");
    assert_eq!(err.kind(), std::io::ErrorKind::FileTooLarge);
    // 本番の上限は MD5 の前提（`bytes.len() <= u32::MAX`）と同じ。
    assert!(check_size(u64::from(u32::MAX), MAX_LOCAL_BYTES).is_ok());
    assert!(check_size(u64::from(u32::MAX) + 1, MAX_LOCAL_BYTES).is_err());
}

/// 綴りの検査（`InsideWorkArea`）を抜ける別名（ジャンクション・8.3 短縮名）で作業場所へ届く
/// エントリも外扱い（作業場所を定義から書き換えさせない）。
#[test]
fn an_entry_reaching_the_work_area_through_a_junction_fails_as_escaping() {
    let f = fixture();
    let area = f.target.join(crate::paths::WORK_DIR);
    fs::create_dir_all(area.join("1-0")).unwrap();
    fs::write(area.join("1-0/x.txt"), b"abc").unwrap();
    junction(&f.target.join("link"), &area);
    let m = manifest(&[&["plain.txt", MD5_ABC], &["link/1-0/x.txt", MD5_EMPTY]]);

    match plan(&f.target, &f.target_real, &m) {
        Err(DiffFailure::Escapes { path }) => {
            assert_eq!(path, local_path(&f.target, "link/1-0/x.txt"));
        }
        Err(DiffFailure::Unreadable { path, source }) => {
            panic!("読めないと誤判定: {}: {source}", path.display())
        }
        Ok(need) => panic!("作業場所のファイルを比較してしまった: {need:?}"),
    }
}

#[test]
fn an_entry_under_a_junction_to_outside_fails_as_escaping() {
    let f = fixture();
    fs::write(f.outside.join("o.txt"), b"abc").unwrap();
    junction(&f.target.join("link"), &f.outside);
    let outside_before = tree(&f.outside);
    // 較正: ジャンクションの手前のエントリは普通に判定される（無い → 要取得）。
    let m = manifest(&[&["plain.txt", MD5_ABC], &["link/o.txt", MD5_EMPTY]]);

    match plan(&f.target, &f.target_real, &m) {
        Err(DiffFailure::Escapes { path }) => {
            assert_eq!(path, local_path(&f.target, "link/o.txt"));
        }
        Err(DiffFailure::Unreadable { path, source }) => {
            panic!("読めないと誤判定: {}: {source}", path.display())
        }
        Ok(need) => panic!("外のファイルを比較してしまった: {need:?}"),
    }
    assert_eq!(tree(&f.outside), outside_before);
    // 同じ木で、ジャンクションを通らない定義なら通る（失敗の原因がジャンクションだけであること）。
    let ok = manifest(&[&["plain.txt", MD5_ABC]]);
    assert_eq!(plan(&f.target, &f.target_real, &ok).unwrap(), vec![0]);
}
