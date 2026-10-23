//! 行から書庫のパスを決める解決（[`sample_nar`]）の兄弟テスト（spec: `areka-P0-sample-url-fetch`
//! 要件 1.4・3.1・3.2・3.4・3.8・3.9）。
//!
//! 登記表にはまだ URL の行が無い（ぱすたの行はタスク 4）ので、URL の行は固定入力の行
//! [`URL_ROW`] で作り、名前空間は私有のもの（[`private_namespace`]）を [`sample_nar_for`] へ
//! 渡す。窓口の 3 つの入口は `*_in(名前空間, 名前, &sample_nar(名前)?)` の形なので、ここでも
//! 同じ形で組む（照合に落ちれば `?` で止まり、原本の作りへは渡らない）。

use super::cache_tests::{contents, private_namespace, shelf, tiny_ghost_nar, whole_ghost_tree};
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
    let src = private_namespace();
    let bytes = std::fs::read(tiny_ghost_nar(src.path(), URL_ROW.name, mark))
        .expect("組んだ .nar は読めるはず");
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

fn resolve(namespace: &Path) -> Result<PathBuf, SampleError> {
    sample_nar_for(&URL_ROW, || Ok(namespace.to_path_buf()))
}

#[test]
fn a_vendored_row_resolves_to_the_repository_nar_without_asking_for_the_namespace() {
    assert_eq!(
        sample_nar_for(&VENDORED_ROW, || panic!(
            "リポジトリに置く行は置き場を見ない"
        ))
        .expect("リポジトリに置く行は照合なしで決まる"),
        nar_dir().join("vendored-row-probe.nar")
    );
    // 登記表の今の行は全部この形（今の 9 本の引き方は変わらない）。
    for sample in crate::SAMPLES
        .iter()
        .filter(|s| s.source == SampleSource::Vendored)
    {
        assert_eq!(
            sample_nar(sample.name).expect("登記済みの名前は引ける"),
            nar_dir().join(format!("{}.nar", sample.name)),
            "{}",
            sample.name
        );
    }
}

#[test]
fn an_unregistered_name_fails_as_an_unknown_sample() {
    assert!(matches!(
        sample_nar("no-such-sample"),
        Err(SampleError::UnknownSample { .. })
    ));
}

#[test]
fn a_url_row_with_a_matching_checksum_reaches_the_same_tree_as_a_current_row() {
    let ns = private_namespace();
    let mark = b"fetched";
    place_on_shelf(ns.path(), mark, true);

    let nar = resolve(ns.path()).expect("合う置き場の書庫は通る");
    assert_eq!(nar, fetched_path_in(ns.path(), URL_ROW.name));

    let copy =
        fresh_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap()).expect("複製を配れる");
    assert_eq!(contents(copy.path()), whole_ghost_tree(URL_ROW.name, mark));
    let manual = manual_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap())
        .expect("手で使う根を配れる");
    assert_eq!(contents(&manual), whole_ghost_tree(URL_ROW.name, mark));
}

#[test]
fn acquiring_the_same_fetched_archive_twice_uses_the_same_master_copy() {
    let ns = private_namespace();
    let bytes = place_on_shelf(ns.path(), b"twice", true);

    let first = cached_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap())
        .expect("1 回目は原本を作る");
    let second = cached_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap())
        .expect("2 回目は同じ原本を使う");
    assert_eq!(first, second);
    assert_eq!(
        shelf(ns.path(), CACHE),
        vec![format!("{}-{}", URL_ROW.name, stamp(&bytes))]
    );
}

#[test]
fn a_mismatching_archive_never_reaches_the_master_copy_shelf() {
    let ns = private_namespace();
    place_on_shelf(ns.path(), b"mismatch", false);

    let refused = resolve(ns.path()).and_then(|nar| cached_root_in(ns.path(), URL_ROW.name, &nar));
    assert!(
        matches!(refused, Err(SampleError::FetchedNarMismatch { .. })),
        "{refused:?}"
    );
    assert!(shelf(ns.path(), CACHE).is_empty());
}

#[test]
fn the_check_runs_on_every_acquire_even_when_the_master_copy_exists() {
    let ns = private_namespace();
    place_on_shelf(ns.path(), b"again", true);
    cached_root_in(ns.path(), URL_ROW.name, &resolve(ns.path()).unwrap()).expect("原本を作る");

    // 原本が在っても、置き場の書庫がハッシュのファイルと合わなくなれば通さない。
    std::fs::write(fetched_path_in(ns.path(), URL_ROW.name), b"tampered")
        .expect("書庫を書き換えられるはず");
    assert!(matches!(
        resolve(ns.path()),
        Err(SampleError::FetchedNarMismatch { .. })
    ));
}
