//! 拒否語彙の全数対応と、失敗のたびに記録が 1 回出ることの判定（要件 9.1・9.2・9.3）。
//!
//! # なぜ 1 本のテストなのか
//!
//! 13 変種それぞれの固定入力をこの 1 本が自分で組み、得た短い語の集合を
//! [`RefuseReason::ALL_KINDS`] と**完全一致**で突き合わせる。別ファイルの兄弟テストの
//! 結果を集める形にすると、テスト間で状態を共有できないぶん「どれか 1 つが走らなかった」
//! が集合の欠けとして現れず、母数の減った突合が緑のまま通ってしまう（設計
//! 「Testing Strategy」）。集合が空なら当然赤になる——完全一致なので恒真にならない。
//!
//! # `UnsupportedEntry` をエントリの印から起こす理由
//!
//! `container` は zip64 や分割書庫という**書庫全体**の条件も同じ変種で返すが、その形は
//! `UnsupportedEntry { index: 0, name: "書庫全体" }` という番兵で、`index: 0` は
//! エントリ番号ではない。語彙の全数対応をその番兵で満たすと、後から読む人が
//! 「0 番目のエントリが対応外だった」と読み違える。ここでは暗号化の印という
//! **実在のエントリ 1 件**の条件から起こす。

use super::*;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{Corrupt, Damage, NarBuilder, WorkDir, install_txt};
use std::collections::BTreeSet;
use std::fs;

// ---- 固定入力 ----

/// 拒否 1 件ぶんの固定入力。
pub(super) struct Case {
    /// 期待する短い語（[`RefuseReason::kind`]）。書庫のファイル名にも使う。
    pub kind: &'static str,
    /// なぜこの入力がその語になるのか。赤くなったときに読む。
    pub note: &'static str,
    pub nar: NarBuilder,
    /// `None`＝`open` が拒否する。`Some(t)`＝`open` は通り `install` が拒否する
    /// （`t` は呼び出し側が渡す宛先ゴーストの名前）。
    pub install: Option<Option<&'static str>>,
}

/// 受理されるゴーストの `install.txt`。
fn ghost_manifest() -> Vec<u8> {
    install_txt(&["type,ghost", "name,テスト", "directory,tester"])
}

/// 受理される `install.txt` を 1 枚だけ持つ書庫。ここに壊れたエントリを足していく。
fn with_manifest() -> NarBuilder {
    NarBuilder::new()
        .file("install.txt", &ghost_manifest())
        .done()
}

/// 13 変種それぞれに 1 つずつ対応する固定入力。宣言順は [`RefuseReason::ALL_KINDS`] と同じ。
pub(super) fn cases() -> Vec<Case> {
    vec![
        Case {
            kind: "CorruptArchive",
            note: "終端記録（EOCD）を書かない書庫は構造として読めない",
            nar: with_manifest().damage(Damage::NoEocd),
            install: None,
        },
        Case {
            kind: "IntegrityMismatch",
            note: "刻まれた CRC だけを違う値にした（宣言サイズも中身も正しいまま）",
            nar: with_manifest()
                .file("ghost/master/dic.txt", b"line\r\n")
                .corrupt(Corrupt::Crc)
                .done(),
            install: None,
        },
        Case {
            kind: "UnsupportedEntry",
            note: "暗号化の印（汎用目的ビット 0）が立ったエントリ 1 件。書庫全体の番兵ではない",
            nar: with_manifest()
                .file("ghost/master/secret.txt", b"locked")
                .encrypted_flag()
                .done(),
            install: None,
        },
        Case {
            kind: "NameUndecodable",
            note: "印なし（＝Shift_JIS）の名前に、後続バイトの範囲外の列を置いた",
            nar: with_manifest()
                .file(b"\x81\x20.txt".to_vec(), b"x")
                .utf8_flag(false)
                .done(),
            install: None,
        },
        Case {
            kind: "SymlinkEntry",
            note: "外部属性の種別が S_IFLNK のエントリ",
            nar: with_manifest()
                .file("ghost/master/link.txt", b"x")
                .symlink()
                .done(),
            install: None,
        },
        Case {
            kind: "UnsafePath",
            note: "親へ遡る要素を含む名前（根の外が宛先になり得る）",
            nar: with_manifest().file("../外.txt", b"x").done(),
            install: None,
        },
        Case {
            kind: "CaseCollision",
            note: "大文字小文字だけが違う 2 つの名前。Windows では後勝ちで一方が消える",
            nar: with_manifest()
                .file("ghost/master/A.txt", b"x")
                .done()
                .file("ghost/master/a.txt", b"y")
                .done(),
            install: None,
        },
        Case {
            kind: "MissingInstallTxt",
            note: "包みフォルダ 1 段の下に install.txt がある（黙って剥がさない）",
            nar: NarBuilder::new()
                .file("wrap/install.txt", &ghost_manifest())
                .done()
                .file("wrap/ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
                .done(),
            install: None,
        },
        Case {
            kind: "UnsupportedType",
            note: "areka が扱わない種別",
            nar: NarBuilder::new()
                .file(
                    "install.txt",
                    &install_txt(&["type,plugin", "name,テスト", "directory,tester"]),
                )
                .done(),
            install: None,
        },
        Case {
            kind: "MissingRequiredKey",
            note: "name が無い",
            nar: NarBuilder::new()
                .file(
                    "install.txt",
                    &install_txt(&["type,ghost", "directory,tester"]),
                )
                .done(),
            install: None,
        },
        Case {
            kind: "InvalidDirectoryName",
            note: "directory の値が 1 階層のフォルダ名として使えない",
            nar: NarBuilder::new()
                .file(
                    "install.txt",
                    &install_txt(&["type,ghost", "name,テスト", "directory,../外"]),
                )
                .done(),
            install: None,
        },
        Case {
            kind: "CompanionSourceMissing",
            note: "同梱バルーンの取り出し元フォルダが書庫に 1 件も無い（計画で分かる）",
            nar: NarBuilder::new()
                .file(
                    "install.txt",
                    &install_txt(&[
                        "type,ghost",
                        "name,テスト",
                        "directory,tester",
                        "balloon.directory,kaku",
                    ]),
                )
                .done()
                .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
                .done(),
            install: Some(None),
        },
        Case {
            kind: "TargetGhostMissing",
            note: "shell の宛先ゴーストが渡されていない（計画で分かる）",
            nar: NarBuilder::new()
                .file(
                    "install.txt",
                    &install_txt(&["type,shell", "name,テスト", "directory,tester"]),
                )
                .done()
                .file("shell/master/surface0.png", b"png")
                .done(),
            install: Some(None),
        },
    ]
}

// ---- 記録を数える ----

/// 重大度が `level` の記録だけを取り出す。
///
/// 捕捉そのものは共有の窓口（`log_capture_kit::capture`）を通す。捕捉先を自前で
/// 差す形はワークスペースの常設検査が禁じており、その窓口は「捕捉が働いていない」
/// ときに自分で落ちるので、ここでの 0 件が空振りの 0 でないことも同時に担保される。
fn at(records: &[CapturedEvent], level: tracing::Level) -> Vec<&CapturedEvent> {
    records
        .iter()
        .filter(|event| event.level == level)
        .collect()
}

// ---- 全数対応（要件 9.2・9.3） ----

/// 13 変種それぞれに固定入力が 1 つ対応し、得た短い語の集合が全数の一覧と完全に一致する。
///
/// 同時に、失敗 1 回につき error の記録がちょうど 1 件出ること（要件 9.1）と、
/// その記録が確定済みの件数と巻き戻せたかを欄として持つこと（要件 6.4）も見る。
/// 失敗のたびに走る道が同じなので、別のテストに分けても同じ入力を組み直すだけになる。
#[test]
fn every_refusal_kind_has_a_fixture_and_is_recorded_once() {
    let work = WorkDir::new().expect("根を借りられる");
    let root = work.path();
    let mut observed: BTreeSet<&'static str> = BTreeSet::new();

    for case in cases() {
        let path = root.join(format!("{}.nar", case.kind));
        fs::write(&path, case.nar.bytes()).expect("固定入力を置ける");

        let (result, records) = capture(|| match case.install {
            None => NarArchive::open(&path).map(|_| ()),
            Some(target_ghost) => NarArchive::open(&path)
                .expect("install で拒否される入力は open を通る")
                .install(&InstallRequest { root, target_ghost })
                .map(|_| ()),
        });

        let error = result.expect_err(case.note);
        let NarError::Refused { archive, reason } = &error else {
            panic!("{} は拒否のはず: {error}（{}）", case.kind, case.note);
        };
        assert_eq!(archive, &path, "理由に載る書庫のパスが違う");
        assert_eq!(
            reason.kind(),
            case.kind,
            "{} を狙った入力が {} で拒否された（{}）",
            case.kind,
            reason.kind(),
            case.note
        );

        let recorded = at(&records, tracing::Level::ERROR);
        assert_eq!(
            recorded.len(),
            1,
            "{} の失敗が記録 {} 件（失敗 1 回につき 1 件）",
            case.kind,
            recorded.len()
        );
        assert_eq!(
            recorded[0].field_names_sorted(),
            [
                "archive",
                "committed",
                "message",
                "reason",
                "rolled_back",
                "work"
            ],
            "{} の記録の欄が足りない（確定済みの件数と巻き戻せたかは要件 6.4）",
            case.kind
        );
        // 13 件はすべて拒否＝作業フォルダを掘る前に止まっている。空でなければ、
        // 書く準備を始めてから拒否した（＝拒否が「触れていない」を満たしていない）。
        assert!(
            recorded[0]
                .field("work")
                .is_some_and(|work| work.trim_matches('"').is_empty()),
            "{} の拒否が作業フォルダの場所を持っている: {:?}",
            case.kind,
            recorded[0].field("work")
        );

        observed.insert(reason.kind());
    }

    assert_eq!(
        observed,
        RefuseReason::ALL_KINDS.iter().copied().collect(),
        "固定入力の集合と拒否語彙の全数が一致しない"
    );
    assert_eq!(observed.len(), 13, "語彙は 13 変種で閉じる（要件 9.2）");
}

/// 上の数え方の較正。受理される一周では error の記録が 1 件も出ない。
///
/// これが無いと、記録を数える仕掛けが「何にでも 1 を返す」形に壊れても気付けない。
#[test]
fn a_successful_install_writes_no_error_record() {
    let work = WorkDir::new().expect("根を借りられる");
    let root = work.path();
    let path = root.join("good.nar");
    fs::write(&path, super::ghost_with_balloon().bytes()).expect("固定入力を置ける");

    let (outcome, records) = capture(|| {
        NarArchive::open(&path)
            .expect("開ける")
            .install(&InstallRequest {
                root,
                target_ghost: None,
            })
            .expect("入る")
    });

    assert_eq!(outcome.installed.len(), 2);
    assert!(
        at(&records, tracing::Level::ERROR).is_empty(),
        "成功の一周で error の記録が出た: {records:?}"
    );
    assert!(
        !at(&records, tracing::Level::INFO).is_empty(),
        "成功の一周で info の記録が 1 件も出ないなら、捕まえる仕掛けのほうが壊れている"
    );
}

/// 読み飛ばしたキーは警告として 1 件ずつ記録に出る（要件 3.9・設計「Monitoring」）。
#[test]
fn skipped_manifest_keys_are_warned_one_by_one() {
    let work = WorkDir::new().expect("根を借りられる");
    let root = work.path();
    let path = root.join("warned.nar");
    let nar = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "type,ghost",
                "name,テスト",
                "directory,tester",
                "bootghost,0",
                "headline.directory,news",
            ]),
        )
        .done()
        .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
        .done();
    fs::write(&path, nar.bytes()).expect("固定入力を置ける");

    let (outcome, records) = capture(|| {
        NarArchive::open(&path)
            .expect("開ける")
            .install(&InstallRequest {
                root,
                target_ghost: None,
            })
            .expect("本体の展開は止まらない")
    });

    assert_eq!(outcome.warnings.len(), 2, "{:?}", outcome.warnings);
    let warned = at(&records, tracing::Level::WARN).len();
    assert_eq!(warned, 2, "警告は 1 件ずつ出す: {records:?}");
}
