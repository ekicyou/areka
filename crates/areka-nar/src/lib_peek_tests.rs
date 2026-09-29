//! 書庫の目次だけを読む口（[`peek_install_txt`]）の兄弟テスト（設計のテスト 21・要件 2.2・9.3）。
//!
//! 判定するのは 3 つ。
//!
//! ⑴ 6 検体（在る・大文字の `INSTALL.TXT`・下の階層にだけ在る・構造の壊れたバイト列・
//!    存在しないパス・`..` のエントリを含む）がそれぞれ設計どおりの答えを返す。
//! ⑵ 探し方が手続き（[`NarArchive::open`]）と同じ——**この検体に限って** 3 組を並べる。
//! ⑶ 問い合わせは記録を 1 件も出さない（同じ検体で `open` が `error!` を出す対照つき）。
//!
//! 根は OS の一時フォルダではなく [`WorkDir`] が配る。

use super::*;
use log_capture_kit::capture;
use sample_ghost_kit::{Damage, NarBuilder, WorkDir, install_txt};
use std::fs;

/// 受理されるゴーストの `install.txt` を `name` の名前で最上位に持つ書庫。
fn with_manifest_named(name: &str) -> NarBuilder {
    NarBuilder::new()
        .file(
            name,
            &install_txt(&["type,ghost", "name,テスト", "directory,tester"]),
        )
        .done()
        .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
        .done()
}

/// `install.txt` が包みフォルダ 1 段の下にだけ在る書庫。
fn wrapped_only() -> NarBuilder {
    NarBuilder::new()
        .file(
            "wrap/install.txt",
            &install_txt(&["type,ghost", "name,テスト", "directory,tester"]),
        )
        .done()
}

/// 最上位に `install.txt` が在るが、親へ遡るエントリも在る書庫（名前の検証で撥ねられる）。
fn with_dot_dot() -> NarBuilder {
    with_manifest_named("install.txt").file("../x", b"x").done()
}

/// 書庫をファイルに置いて、そのパスを返す。
fn put(work: &WorkDir, name: &str, builder: &NarBuilder) -> PathBuf {
    let path = work.path().join(name);
    fs::write(&path, builder.bytes()).expect("固定入力を置ける");
    path
}

fn is_unsafe_dot_dot(error: &NarError) -> bool {
    matches!(
        error,
        NarError::Refused {
            reason: RefuseReason::UnsafePath {
                why: UnsafeWhy::DotDot,
                ..
            },
            ..
        }
    )
}

/// ⑴ と ⑶: 6 検体の答え。全部を 1 つの捕捉の中で問い合わせ、記録が 0 件であることも見る。
#[test]
fn six_samples_answer_as_designed_without_any_record() {
    let work = WorkDir::new().expect("根を借りられる");
    let present = put(&work, "present.nar", &with_manifest_named("install.txt"));
    let upper = put(&work, "upper.nar", &with_manifest_named("INSTALL.TXT"));
    let wrapped = put(&work, "wrapped.nar", &wrapped_only());
    let broken = put(
        &work,
        "broken.nar",
        &with_manifest_named("install.txt").damage(Damage::NoEocd),
    );
    let missing = work.path().join("存在しない.nar");
    let dot_dot = put(&work, "dotdot.nar", &with_dot_dot());

    let (answers, records) = capture(|| {
        [&present, &upper, &wrapped, &broken, &missing, &dot_dot].map(|p| peek_install_txt(p))
    });
    let [present, upper, wrapped, broken, missing, dot_dot] = answers;

    assert!(matches!(present, Ok(true)), "最上位に在る: {present:?}");
    assert!(matches!(upper, Ok(true)), "大文字でも在る: {upper:?}");
    assert!(
        matches!(wrapped, Ok(false)),
        "下の階層にだけ在るのは無い: {wrapped:?}"
    );
    assert!(
        matches!(
            broken,
            Err(NarError::Refused {
                reason: RefuseReason::CorruptArchive { .. },
                ..
            })
        ),
        "EOCD の無いバイト列は構造の拒否: {broken:?}"
    );
    assert!(
        matches!(
            missing,
            Err(NarError::Io {
                phase: IoPhase::Read,
                ..
            })
        ),
        "存在しないパスは読み取りの失敗: {missing:?}"
    );
    let dot_dot = dot_dot.expect_err("最上位に install.txt が在っても名前の拒否が先");
    assert!(is_unsafe_dot_dot(&dot_dot), "名前の拒否: {dot_dot:?}");
    assert!(records.is_empty(), "問い合わせの記録は 0 件: {records:?}");
}

/// ⑵ と ⑶: 同じ検体で手続きの開き方と答えが食い違わない。`open` は失敗の記録を出す
/// （捕捉そのものが空振りしていない対照）。
#[test]
fn the_answer_agrees_with_how_the_procedure_opens_the_same_archive() {
    let work = WorkDir::new().expect("根を借りられる");

    // install.txt が在る ⇒ open は MissingInstallTxt 以外。
    let present = put(&work, "present.nar", &with_manifest_named("install.txt"));
    assert!(matches!(peek_install_txt(&present), Ok(true)));
    let opened = NarArchive::open(&present).err();
    assert!(
        !matches!(
            opened,
            Some(NarError::Refused {
                reason: RefuseReason::MissingInstallTxt { .. },
                ..
            })
        ),
        "在ると答えた書庫を手続きが「無い」と断らない: {opened:?}"
    );

    // 下の階層にだけ在る ⇒ open も MissingInstallTxt。
    let wrapped = put(&work, "wrapped.nar", &wrapped_only());
    assert!(matches!(peek_install_txt(&wrapped), Ok(false)));
    let (opened, open_records) = capture(|| NarArchive::open(&wrapped).err());
    assert!(
        matches!(
            opened,
            Some(NarError::Refused {
                reason: RefuseReason::MissingInstallTxt { .. },
                ..
            })
        ),
        "無いと答えた書庫を手続きも「無い」と断る: {opened:?}"
    );
    assert_eq!(open_records.len(), 1, "対照: open は失敗を 1 回記録する");

    // 名前の拒否 ⇒ open も同じ理由の Refused。
    let dot_dot = put(&work, "dotdot.nar", &with_dot_dot());
    let (peeked, peek_records) = capture(|| peek_install_txt(&dot_dot));
    let opened = NarArchive::open(&dot_dot).err();
    assert!(
        peeked.as_ref().err().is_some_and(is_unsafe_dot_dot),
        "{peeked:?}"
    );
    assert!(
        opened.as_ref().is_some_and(is_unsafe_dot_dot),
        "手続きも同じ理由で断る: {opened:?}"
    );
    assert!(
        peek_records.is_empty(),
        "問い合わせは記録しない: {peek_records:?}"
    );
}
