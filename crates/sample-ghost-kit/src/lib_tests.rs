//! 窓口 [`SampleRoot`](super::SampleRoot) と登記表 [`SAMPLES`](super::SAMPLES) の自己テスト
//! （要件 1.1・1.3・1.4・1.5）。
//!
//! 段 ① の窓口は「登記の 1 行から追跡済みの展開形のパスを組む」だけなので、主張は 4 本——
//! 登記した検体のフォルダが実在する／未登録名は既知の 4 つを含む失敗になる／同梱していない
//! バルーン名は既知の一覧を含む失敗になる／組み立てが**登記の行から導かれている**
//! （検体ごとの第 2 の表を持っていない）。
//!
//! 「借用しか返さないので取得した値を捨てるとコンパイルできない」ことは、コンパイル自体を
//! 判定に使うので rustdoc の `compile_fail` 例（[`SampleRoot::folder`](super::SampleRoot::folder)
//! と [`SampleRoot::balloon`](super::SampleRoot::balloon) の doc）が固定する。その対として、
//! 2 行だけ違う「値を持ったまま使う」形が**実際に通る**例を隣に置いてあるので、
//! `compile_fail` が別の理由（import 違い・署名違い）で通る恒真ではないと分かる。

use super::*;

/// 登記した 4 つの検体は、段 ① の時点で全て実在するフォルダを指す（要件 1.1）。
///
/// 実在しないパスを返す読み口は欠陥なので、窓口が黙って空振りしないことをここで固定する。
#[test]
fn every_registered_sample_points_at_an_existing_folder() {
    assert_eq!(SAMPLES.len(), 4, "段 ① の登記は検体 4 つ");
    for sample in SAMPLES {
        let acquired = SampleRoot::acquire(sample.name).expect("登記済みの名前は取得できるはず");
        let folder = acquired.folder();
        assert!(
            folder.is_dir(),
            "検体 {} のフォルダが実在しない: {}",
            sample.name,
            folder.display()
        );
        assert!(
            folder.is_absolute(),
            "窓口は絶対パスを返す: {}",
            folder.display()
        );
        for balloon in sample.balloons {
            let path = acquired
                .balloon(balloon)
                .expect("登記済みの同梱バルーンは引けるはず");
            assert!(
                path.is_dir(),
                "同梱バルーン {balloon} のフォルダが実在しない: {}",
                path.display()
            );
        }
    }
}

/// 未登録の検体名は、既知の名前 4 つを含む理由付きの失敗になる（要件 1.4）。
#[test]
fn unknown_sample_fails_with_all_four_known_names() {
    let err = SampleRoot::acquire("no-such-ghost").expect_err("未登録名は失敗するはず");
    let SampleError::UnknownSample { requested, known } = &err else {
        panic!("未登録名の失敗は UnknownSample であるはず: {err:?}");
    };
    assert_eq!(requested, "no-such-ghost");
    assert_eq!(
        known,
        &[
            "emo2",
            "R_POST_and_KOMAINU",
            "emo2-kakukaku-offsetdpi",
            "emo2-kakukaku-wplimit",
        ]
    );
    let shown = err.to_string();
    for name in [
        "emo2",
        "R_POST_and_KOMAINU",
        "emo2-kakukaku-offsetdpi",
        "emo2-kakukaku-wplimit",
    ] {
        assert!(shown.contains(name), "失敗の表示に {name} が無い: {shown}");
    }
}

/// 同時にインストールしていないバルーン名は、既知の一覧を含む失敗になる（要件 1.4）。
///
/// 同梱バルーンを持たない検体では既知の一覧が**空**になることも明示して確かめる
/// （引き算で導ける 0 は沈黙と同じなので書く）。
#[test]
fn unknown_balloon_fails_with_the_known_balloon_list() {
    let emo2 = SampleRoot::acquire("emo2").expect("emo2 は登記済み");
    let err = emo2
        .balloon("emo2-kakukaku-nope")
        .expect_err("同梱していない名前は失敗するはず");
    let SampleError::UnknownBalloon {
        sample,
        requested,
        known,
    } = &err
    else {
        panic!("同梱外の失敗は UnknownBalloon であるはず: {err:?}");
    };
    assert_eq!(*sample, "emo2");
    assert_eq!(requested, "emo2-kakukaku-nope");
    assert_eq!(known, &["emo2-kakukaku"]);
    assert!(
        err.to_string().contains("emo2-kakukaku"),
        "失敗の表示に既知の名前が無い"
    );

    let plain = SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済み");
    let err = plain
        .balloon("emo2-kakukaku")
        .expect_err("同梱 0 の検体では何も引けない");
    let SampleError::UnknownBalloon { known, .. } = &err else {
        panic!("UnknownBalloon であるはず: {err:?}");
    };
    assert!(
        known.is_empty(),
        "同梱バルーンを持たない検体の既知一覧は空: {known:?}"
    );
}

/// 検体を 1 つ足す作業が「`.nar` を 1 つ置く」と「登記に 1 行」の 2 手に収まる（要件 1.5）。
///
/// 判定は「返すパスが登記の行（`checked_in_parent` と `name`）から**そのまま導かれている**」
/// こと。検体ごとの第 2 の表や専用関数があればこの等式は破れる。
#[test]
fn folder_is_derived_from_the_registry_row_alone() {
    for sample in SAMPLES {
        let acquired = SampleRoot::acquire(sample.name).expect("登記済み");
        assert_eq!(
            acquired.folder(),
            workspace_root()
                .join(sample.checked_in_parent)
                .join(sample.name),
            "検体 {} のパスが登記の行から導かれていない",
            sample.name
        );
        for balloon in sample.balloons {
            assert_eq!(
                acquired.balloon(balloon).expect("登記済み"),
                acquired.folder().join(balloon),
                "段 ① の同梱バルーンは検体フォルダの直下（要件 1.3）"
            );
        }
    }
}

/// 種別と同梱バルーン名が登記されている（要件 1.3・段 ③ の往復検査の入力）。
#[test]
fn registry_records_kind_and_bundled_balloons() {
    let by_name = |name: &str| SAMPLES.iter().find(|s| s.name == name).expect("登記済み");
    assert_eq!(by_name("emo2").kind, SampleKind::Ghost);
    assert_eq!(by_name("emo2").balloons, &["emo2-kakukaku"]);
    assert_eq!(by_name("R_POST_and_KOMAINU").kind, SampleKind::Ghost);
    assert!(
        by_name("R_POST_and_KOMAINU").balloons.is_empty(),
        "R_POST_and_KOMAINU は同梱バルーン 0"
    );
    for name in ["emo2-kakukaku-offsetdpi", "emo2-kakukaku-wplimit"] {
        assert_eq!(by_name(name).kind, SampleKind::Balloon);
        assert!(by_name(name).balloons.is_empty(), "{name} は同梱バルーン 0");
    }
}
