//! 行から書庫のパスを決める解決（[`sample_nar_for`]）の兄弟テスト。
//!
//! 登記表の URL の行（ぱすた）は取り寄せた書庫が要るので、ここでは固定入力の行
//! [`URL_ROW`] で作り、名前空間は私有のもの（[`private_namespace`]）を [`sample_nar_for`] へ
//! 渡す。窓口の入口は書庫のバイトを解決から受ける（原本の作りはバイトを、複製と手で使う根は
//! バイトを返す閉包を受ける）ので、ここでも解決の答えを同じ形で渡す（照合に落ちれば `?` で
//! 止まり、原本の作りへは渡らない）。

use super::cache_tests::{contents, private_namespace, shelf, tiny_ghost_bytes, whole_ghost_tree};
use super::*;
use crate::fetched::{fetched_checksum_path_in, fetched_path_in};
use crate::sha256::sha256_hex;
use crate::{Sample, SampleKind, SampleSource};

/// URL の行の固定入力。URL は使わない（窓口は通信しない）。
static URL_ROW: Sample = Sample {
    name: "url-row-probe",
    kind: SampleKind::Ghost,
    balloons: &[],
    source: SampleSource::Fetched {
        url: "https://example.invalid/url-row-probe.nar",
    },
};

/// リポジトリに置く行の固定入力。
static VENDORED_ROW: Sample = Sample {
    name: "vendored-row-probe",
    kind: SampleKind::Ghost,
    balloons: &[],
    source: SampleSource::Vendored,
};

/// 最小のゴーストの書庫を組み、私有の名前空間の置き場へ書庫とハッシュのファイルを置く。
/// `matching` が偽ならハッシュのファイルに別のバイト列の SHA-256 を書く（合わない置き場）。
fn place_on_shelf(namespace: &Path, mark: &[u8], matching: bool) -> Vec<u8> {
    let bytes = tiny_ghost_bytes(URL_ROW.name, mark);
    let nar = fetched_path_in(namespace, URL_ROW.name);
    std::fs::create_dir_all(nar.parent().expect("置き場には親がある")).expect("棚を作れるはず");
    std::fs::write(&nar, &bytes).expect("書庫を置けるはず");
    std::fs::write(
        fetched_checksum_path_in(namespace, URL_ROW.name),
        format!(
            "{}  {}.nar\n",
            sha256_hex(if matching { &bytes } else { b"other bytes" }),
            URL_ROW.name
        ),
    )
    .expect("ハッシュのファイルを置けるはず");
    bytes
}

fn resolve(namespace: &Path) -> Result<(PathBuf, Vec<u8>), SampleError> {
    sample_nar_for(&URL_ROW, || Ok(namespace.to_path_buf()))
}

/// 窓口の入口へ渡す、確かめたバイトの出どころ。
fn verified(namespace: &Path) -> impl Fn() -> Result<Vec<u8>, SampleError> + '_ {
    move || resolve(namespace).map(|(_, bytes)| bytes)
}

#[test]
fn a_vendored_row_resolves_to_the_repository_nar_without_asking_for_the_namespace() {
    // 固定入力の行の `.nar` は置いていないので読めないが、読みに行く先はリポジトリの綴り。
    let refused = sample_nar_for(&VENDORED_ROW, || {
        panic!("リポジトリに置く行は置き場を見ない")
    });
    assert!(
        matches!(&refused, Err(SampleError::Io { path, .. })
            if *path == nar_dir().join("vendored-row-probe.nar")),
        "{refused:?}"
    );
    // 登記表の Vendored の行は全部この形。読んだバイトも返す。
    for sample in crate::SAMPLES
        .iter()
        .filter(|s| s.source == SampleSource::Vendored)
    {
        let (nar, bytes) = sample_nar(sample.name).expect("登記済みの名前は引ける");
        assert_eq!(
            nar,
            nar_dir().join(format!("{}.nar", sample.name)),
            "{}",
            sample.name
        );
        assert_eq!(
            bytes,
            std::fs::read(&nar).expect("読める"),
            "{}",
            sample.name
        );
    }
}

#[test]
fn a_url_row_with_a_matching_checksum_hands_out_the_whole_tree() {
    let ns = private_namespace();
    let mark = b"fetched";
    place_on_shelf(ns.path(), mark, true);

    let (nar, bytes) = resolve(ns.path()).expect("合う置き場の書庫は通る");
    assert_eq!(nar, fetched_path_in(ns.path(), URL_ROW.name));
    assert_eq!(
        bytes,
        std::fs::read(&nar).expect("読める"),
        "確かめたバイトも返す"
    );

    let copy = fresh_root_in(ns.path(), URL_ROW.name, verified(ns.path())).expect("複製を配れる");
    assert_eq!(contents(copy.path()), whole_ghost_tree(URL_ROW.name, mark));
    let manual =
        manual_root_in(ns.path(), URL_ROW.name, verified(ns.path())).expect("手で使う根を配れる");
    assert_eq!(contents(&manual), whole_ghost_tree(URL_ROW.name, mark));
}

#[test]
fn acquiring_the_same_fetched_archive_twice_uses_the_same_master_copy() {
    let ns = private_namespace();
    let bytes = place_on_shelf(ns.path(), b"twice", true);

    let first = cached_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap().1)
        .expect("1 回目は原本を作る");
    let second = cached_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap().1)
        .expect("2 回目は同じ原本を使う");
    assert_eq!(first, second);
    assert_eq!(
        shelf(ns.path(), CACHE),
        vec![format!("{}-{}", URL_ROW.name, stamp(&bytes))]
    );
}

#[test]
fn resolving_a_mismatching_archive_leaves_the_master_copy_shelf_empty() {
    let ns = private_namespace();
    place_on_shelf(ns.path(), b"mismatch", false);

    let refused =
        resolve(ns.path()).and_then(|(_, bytes)| cached_root_in(ns.path(), URL_ROW.name, &bytes));
    assert!(
        matches!(refused, Err(SampleError::FetchedNarMismatch { .. })),
        "{refused:?}"
    );
    assert!(shelf(ns.path(), CACHE).is_empty());
}

/// 掃除は作業の棚だけを走査し、取り寄せの棚の書庫とハッシュのファイルには触らない。複製を配る入口 `fresh_root_in` は取得のたびに掃除を走らせる。
#[test]
fn a_sweep_leaves_the_fetched_shelf_alone() {
    let ns = private_namespace();
    let bytes = place_on_shelf(ns.path(), b"shelf", true);
    let nar = fetched_path_in(ns.path(), URL_ROW.name);
    let checksum = fetched_checksum_path_in(ns.path(), URL_ROW.name);
    let checksum_text = std::fs::read(&checksum).expect("較正: ハッシュのファイルが実在すること");

    sweep(ns.path());
    let copy = fresh_root_in(ns.path(), URL_ROW.name, verified(ns.path())).expect("複製を配れる");

    assert_eq!(
        std::fs::read(&nar).expect("書庫が消されていないこと"),
        bytes,
        "掃除の後も取り寄せた書庫のバイトが同じこと"
    );
    assert_eq!(
        std::fs::read(&checksum).expect("ハッシュのファイルが消されていないこと"),
        checksum_text,
        "掃除の後もハッシュのファイルが同じこと"
    );
    drop(copy);
}

#[test]
fn resolving_again_after_the_master_copy_exists_still_checks_the_archive() {
    let ns = private_namespace();
    place_on_shelf(ns.path(), b"again", true);
    cached_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap().1).expect("原本を作る");

    // 原本が在っても、置き場の書庫がハッシュのファイルと合わなくなれば通さない。
    std::fs::write(fetched_path_in(ns.path(), URL_ROW.name), b"tampered")
        .expect("書庫を書き換えられるはず");
    assert!(matches!(
        resolve(ns.path()),
        Err(SampleError::FetchedNarMismatch { .. })
    ));
}

/// 子として起こされたことを伝える環境変数。
const CHILD_VAR: &str = "SAMPLE_GHOST_KIT_TAMPER_CHILD";

/// 子に走らせるテストの完全な名前。
const CHILD_TEST: &str = "devroot::fetched_tests::the_tampered_shelf_child_acquires_and_is_refused";

/// 窓口の入口 [`SampleRoot::acquire`] が、食い違う置き場の書庫を配らない。
///
/// `resolving_a_mismatching_archive_leaves_the_master_copy_shelf_empty` と
/// `resolving_again_after_the_master_copy_exists_still_checks_the_archive` は `sample_nar_for` までしか通らない。ここは自分の実行体を子として起こし、
/// 私有の `CARGO_TARGET_DIR` に食い違う棚を置いて、本物の入口 `acquire` を通す。
#[test]
fn acquire_refuses_a_tampered_fetched_archive() {
    if std::env::var(CHILD_VAR).is_ok() {
        return; // 子として起こされたときに孫を作らない。
    }
    let home = private_namespace();
    let namespace = home.path().join(NAMESPACE);
    let name = "pasta-in-windows";
    let nar = fetched_path_in(&namespace, name);
    std::fs::create_dir_all(nar.parent().expect("置き場には親がある")).expect("棚を作れるはず");
    std::fs::write(&nar, b"tampered archive").expect("書庫を置けるはず");
    std::fs::write(
        fetched_checksum_path_in(&namespace, name),
        format!("{}  {name}.nar\n", sha256_hex(b"the published archive")),
    )
    .expect("ハッシュのファイルを置けるはず");

    let out = std::process::Command::new(std::env::current_exe().expect("テストバイナリの場所"))
        .args(["--exact", CHILD_TEST, "--test-threads", "1"])
        .env("CARGO_TARGET_DIR", home.path())
        .env(CHILD_VAR, "1")
        .output()
        .expect("子を起こせること");
    assert!(
        out.status.success(),
        "子が食い違いの拒否を確かめられなかった: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("1 passed"),
        "較正: 子のテストが実際に走ったこと: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// 上の検査が起こす子。親の走行では環境変数が無いので何もしない。
#[test]
fn the_tampered_shelf_child_acquires_and_is_refused() {
    if std::env::var(CHILD_VAR).is_err() {
        return;
    }
    let refused = crate::SampleRoot::acquire("pasta-in-windows");
    assert!(
        matches!(refused, Err(SampleError::FetchedNarMismatch { .. })),
        "{refused:?}"
    );
}
