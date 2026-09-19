//! `container` の兄弟テスト（要件 2.1・2.5・2.6・5.9）。
//!
//! 固定入力はすべて `sample_ghost_kit::NarBuilder` が組む。無傷の書庫と、印を
//! 1 つだけ立てた／1 点だけ壊した同じ書庫との対を作れるので、どの検査が働いて
//! 拒否になったのかを取り違えずに判定できる。
//!
//! 期待値の置き方は 2 つ。⑴ 受理の側は**書き手へ渡した原文のバイト列**と読み手が
//! 返したバイト列を突き合わせる（読み手が自分で作った値を期待値にしない）。
//! ⑵ 拒否の側は理由の**変種**まで見る。`CorruptArchive` のように 1 つの変種が
//! 複数の壊れ方を受け持つところは、説明文の語まで見て取り違えを防ぐ。

use super::*;
use crate::error::{Integrity, Unsupported};
use sample_ghost_kit::{Corrupt, Damage, NarBuilder, WorkDir, install_txt};

/// エントリ名の生バイトと、伸長・突合を終えた中身の対。
type NameAndBody = (Vec<u8>, Vec<u8>);

/// 読み取りの全段（中央ディレクトリ → 伸長と突合）を通し、エントリ名と中身の対を返す。
fn read_all(bytes: &[u8]) -> Result<Vec<NameAndBody>, RefuseReason> {
    let entries = read_central_directory(bytes)?;
    entries
        .iter()
        .map(|entry| Ok((entry.name_raw.clone(), inflate_entry(bytes, entry)?)))
        .collect()
}

/// 拒否だけを取り出す。受理されたら「拒否されるはず」の主張ごと落とす。
fn refusal(bytes: &[u8]) -> RefuseReason {
    match read_all(bytes) {
        Ok(entries) => panic!("拒否されるはずの書庫が受理された: {} 件", entries.len()),
        Err(reason) => reason,
    }
}

/// 日本語・改行・NUL を含むバイト列。復号も改行変換もされていないことを見る材料。
const BINARY_BODY: &[u8] = b"charset,Shift_JIS\r\n\x82\xa0\x82\xa2\x00\x01\xff\n\n";

// ---- 受理 ----

/// 無圧縮（方式 0）のエントリを受理し、原文がそのまま戻る。
#[test]
fn stored_entry_is_accepted_and_round_trips() {
    let nar = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&["type,ghost", "directory,emo2"]),
        )
        .done()
        .bytes();

    let read = read_all(&nar).expect("無傷の書庫は受理される");
    assert_eq!(
        read,
        vec![(
            b"install.txt".to_vec(),
            install_txt(&["type,ghost", "directory,emo2"])
        )]
    );
}

/// deflate（方式 8）のエントリを受理し、原文がそのまま戻る。
#[test]
fn deflate_entry_is_accepted_and_round_trips() {
    let nar = NarBuilder::new()
        .file("ghost/master/descript.txt", BINARY_BODY)
        .deflate()
        .done()
        .bytes();

    let read = read_all(&nar).expect("無傷の書庫は受理される");
    assert_eq!(
        read,
        vec![(b"ghost/master/descript.txt".to_vec(), BINARY_BODY.to_vec())]
    );
}

/// 書き手が組んだ無圧縮と deflate が、読取層で**同じバイト列に戻る**（要件 5.9）。
///
/// タスク 2.4 の完了条件がここで初めて確かめられる（書いた時点では読み手が無かった）。
/// 同じ原文を方式だけ変えて 2 度入れ、どちらも 1 バイトの違いも無く戻ることを見る。
/// 改行（`\r\n` と `\n`）・NUL・Shift_JIS のバイト・0xFF を含むので、改行変換や
/// 文字コード変換が挟まれば必ず食い違う。
#[test]
fn writer_bytes_survive_both_methods_unchanged() {
    let bodies: Vec<Vec<u8>> = vec![
        BINARY_BODY.to_vec(),
        Vec::new(),
        (0u8..=255).collect(),
        b"a".repeat(10_000),
    ];

    let mut builder = NarBuilder::new();
    for (index, body) in bodies.iter().enumerate() {
        builder = builder
            .file(format!("stored/{index}").into_bytes(), body)
            .done();
        builder = builder
            .file(format!("deflated/{index}").into_bytes(), body)
            .deflate()
            .done();
    }
    let nar = builder.bytes();

    let read = read_all(&nar).expect("無傷の書庫は受理される");
    let mut expected: Vec<NameAndBody> = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        expected.push((format!("stored/{index}").into_bytes(), body.clone()));
        expected.push((format!("deflated/{index}").into_bytes(), body.clone()));
    }
    assert_eq!(read, expected);
}

/// 中央ディレクトリの欄が `RawEntry` にそのまま写る（名前の生バイト・印・種別・外部属性・順番）。
#[test]
fn central_directory_fields_are_carried_verbatim() {
    let shift_jis_name = b"\x83S\x81[\x83X\x83g/\x82\xa0.txt".to_vec();
    let nar = NarBuilder::new()
        .dir("ghost")
        .file(shift_jis_name.clone(), b"x")
        .utf8_flag(false)
        .done()
        .file("link", b"../elsewhere")
        .symlink()
        .done()
        .bytes();

    let entries = read_central_directory(&nar).expect("無傷の書庫は受理される");
    assert_eq!(entries.len(), 3);

    assert_eq!(entries[0].index, 0);
    assert_eq!(entries[0].name_raw, b"ghost/");
    assert!(entries[0].is_dir, "名前が / で終わればフォルダ");
    assert_eq!(entries[0].method, 0);

    assert_eq!(entries[1].index, 1);
    assert_eq!(entries[1].name_raw, shift_jis_name, "名前は生バイトのまま");
    assert!(!entries[1].utf8_flag, "印の無い名前は印無しのまま渡る");
    assert!(!entries[1].is_dir);

    assert_eq!(entries[2].index, 2);
    assert!(entries[2].utf8_flag, "書き手の既定は印あり");
    assert_eq!(
        entries[2].external_attrs >> 16 & 0o170_000,
        0o120_000,
        "S_IFLNK が外部属性から読める（判定は names の担当）"
    );
}

/// 拡張子が `.nar` でも `.zip` でも同じ手順で読む（要件 2.1）。
///
/// 読み手はバイト列しか受け取らないので拡張子を見る余地が無い——それを、同じ
/// バイト列を両方の名前で**実際にファイルへ置いて読み直し**、結果が完全に一致する
/// ことで示す。片方だけを通す分岐が入れば落ちる。
#[test]
fn nar_and_zip_extensions_read_identically() {
    let nar = NarBuilder::new()
        .file("install.txt", &install_txt(&["type,balloon"]))
        .done()
        .file("balloon/descript.txt", BINARY_BODY)
        .deflate()
        .done();

    let work = WorkDir::new().expect("空の根を借りる");
    let as_nar = work.path().join("sample.nar");
    let as_zip = work.path().join("sample.zip");
    nar.write_to(&as_nar).expect("書ける");
    nar.write_to(&as_zip).expect("書ける");

    let from_nar = read_all(&std::fs::read(&as_nar).expect("読める")).expect("受理される");
    let from_zip = read_all(&std::fs::read(&as_zip).expect("読める")).expect("受理される");
    assert_eq!(from_nar, from_zip);
    assert_eq!(from_nar.len(), 2);
}

// ---- 整合性の拒否（要件 2.6） ----

/// 刻まれた CRC だけを壊すと CRC 不一致で拒否する。
#[test]
fn corrupted_crc_is_refused_as_crc_mismatch() {
    let nar = NarBuilder::new()
        .file("install.txt", b"type,ghost")
        .corrupt(Corrupt::Crc)
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::IntegrityMismatch {
        index, ref what, ..
    } = reason
    else {
        panic!("整合性の不一致で拒否されるはず: {reason}");
    };
    assert_eq!(index, 0);
    assert!(
        matches!(what, Integrity::Crc { expected, actual } if expected != actual),
        "期待と実際の両方を持つ: {what}"
    );
}

/// 宣言した伸長後サイズだけを壊すと長さ不一致で拒否する（無圧縮）。
#[test]
fn corrupted_stored_size_is_refused_as_size_mismatch() {
    let body = b"type,ghost";
    let nar = NarBuilder::new()
        .file("install.txt", body)
        .corrupt(Corrupt::Size)
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::IntegrityMismatch { ref what, .. } = reason else {
        panic!("整合性の不一致で拒否されるはず: {reason}");
    };
    assert_eq!(
        *what,
        Integrity::Size {
            expected: body.len() as u64 + 1,
            actual: body.len() as u64,
        }
    );
}

/// 宣言した伸長後サイズだけを壊すと長さ不一致で拒否する（deflate）。
///
/// 伸長の上限は宣言サイズなので、宣言が 1 大きい側は伸長そのものは通り、
/// 長さの突合で落ちる。CRC の突合まで届かないことも同時に見る。
#[test]
fn corrupted_deflate_size_is_refused_as_size_mismatch() {
    let nar = NarBuilder::new()
        .file("body", BINARY_BODY)
        .deflate()
        .corrupt(Corrupt::Size)
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::IntegrityMismatch { ref what, .. } = reason else {
        panic!("整合性の不一致で拒否されるはず: {reason}");
    };
    assert_eq!(
        *what,
        Integrity::Size {
            expected: BINARY_BODY.len() as u64 + 1,
            actual: BINARY_BODY.len() as u64,
        }
    );
}

/// 無圧縮の中身だけを書き換えると CRC 不一致で拒否する（長さは変わらない）。
#[test]
fn corrupted_stored_data_is_refused_as_crc_mismatch() {
    let nar = NarBuilder::new()
        .file("install.txt", b"type,ghost")
        .corrupt(Corrupt::Data)
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::IntegrityMismatch { ref what, .. } = reason else {
        panic!("整合性の不一致で拒否されるはず: {reason}");
    };
    assert!(
        matches!(what, Integrity::Crc { .. }),
        "長さは合うので CRC で落ちる: {what}"
    );
}

/// deflate の中身を書き換えると**圧縮された流れ**が壊れるので、伸長の失敗で拒否する。
///
/// `Corrupt::Data` が触るのは圧縮後のバイト列なので、CRC まで届かずここで落ちるのが
/// 正しい。書き手を直して「CRC で落ちる」形にはしない（壊す位置が変わると
/// どの検査が働いたのかが分からなくなる）。
#[test]
fn corrupted_deflate_stream_is_refused_as_inflate_failure() {
    let nar = NarBuilder::new()
        .file("body", BINARY_BODY)
        .deflate()
        .corrupt(Corrupt::Data)
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::IntegrityMismatch { ref what, .. } = reason else {
        panic!("整合性の不一致で拒否されるはず: {reason}");
    };
    assert_eq!(*what, Integrity::Inflate);
}

// ---- 書庫の構造の拒否 ----

/// 終端記録（EOCD）が無ければ、構造が読めないとして拒否する。
#[test]
fn missing_end_of_central_directory_is_refused() {
    let nar = NarBuilder::new()
        .file("install.txt", b"type,ghost")
        .done()
        .damage(Damage::NoEocd)
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::CorruptArchive { ref detail } = reason else {
        panic!("構造が読めないとして拒否されるはず: {reason}");
    };
    assert!(
        detail.contains("終端記録"),
        "どこが読めないかを言う: {detail}"
    );
}

/// 中央ディレクトリの位置が書庫の外を指していれば拒否する。
///
/// 終端記録は在るので、上のテストとは別の説明文になることまで見る（同じ変種が
/// 受け持つ 2 つの壊れ方を取り違えない）。
#[test]
fn central_directory_offset_out_of_range_is_refused() {
    let nar = NarBuilder::new()
        .file("install.txt", b"type,ghost")
        .done()
        .damage(Damage::CentralOffsetOutOfRange)
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::CorruptArchive { ref detail } = reason else {
        panic!("構造が読めないとして拒否されるはず: {reason}");
    };
    assert!(
        detail.contains("中央ディレクトリ") && detail.contains("範囲外"),
        "位置が範囲外だと言う: {detail}"
    );
    assert!(!detail.contains("終端記録"), "終端記録は在った: {detail}");
}

/// zip64 の印があれば拒否する。
#[test]
fn zip64_marker_is_refused() {
    let nar = NarBuilder::new()
        .file("install.txt", b"type,ghost")
        .done()
        .damage(Damage::Zip64Marker)
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::UnsupportedEntry { ref what, .. } = reason else {
        panic!("対応外として拒否されるはず: {reason}");
    };
    assert_eq!(*what, Unsupported::Zip64);
}

/// 複数ディスクに分かれていれば拒否する。
#[test]
fn multi_disk_archive_is_refused() {
    let nar = NarBuilder::new()
        .file("install.txt", b"type,ghost")
        .done()
        .damage(Damage::MultiDisk)
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::UnsupportedEntry { ref what, .. } = reason else {
        panic!("対応外として拒否されるはず: {reason}");
    };
    assert_eq!(*what, Unsupported::MultiDisk);
}

// ---- エントリの形式の拒否（要件 2.5） ----

/// 暗号化の印（汎用目的ビット 0）が立っていれば拒否し、理由にエントリ名を含める。
#[test]
fn encrypted_entry_is_refused_with_its_name() {
    let nar = NarBuilder::new()
        .file("secret.txt", b"type,ghost")
        .encrypted_flag()
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::UnsupportedEntry {
        index,
        ref name,
        ref what,
    } = reason
    else {
        panic!("対応外として拒否されるはず: {reason}");
    };
    assert_eq!(index, 0);
    assert_eq!(name, "secret.txt");
    assert_eq!(*what, Unsupported::Encrypted);
}

/// 方式 99（AES）は暗号化の別の綴りなので、暗号化として拒否する。
#[test]
fn method_99_is_refused_as_encrypted() {
    let nar = NarBuilder::new()
        .file("secret.txt", b"type,ghost")
        .method(99)
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::UnsupportedEntry { ref what, .. } = reason else {
        panic!("対応外として拒否されるはず: {reason}");
    };
    assert_eq!(*what, Unsupported::Encrypted);
}

/// 対応外の圧縮方式は、方式の番号とエントリ名を添えて拒否する（要件 2.5）。
#[test]
fn unsupported_compression_method_is_refused_with_the_method_number() {
    let nar = NarBuilder::new()
        .file("body.txt", b"type,ghost")
        .method(12)
        .done()
        .bytes();

    let reason = refusal(&nar);
    let RefuseReason::UnsupportedEntry {
        ref name, ref what, ..
    } = reason
    else {
        panic!("対応外として拒否されるはず: {reason}");
    };
    assert_eq!(name, "body.txt");
    assert_eq!(*what, Unsupported::Compression(12));
    assert!(
        reason.to_string().contains("12"),
        "理由に方式が出る: {reason}"
    );
    // 伸長を 1 バイトも試みる前、中央ディレクトリを読んだ時点で撥ねる。
    // これで `RawEntry::method` は 0 か 8 しか取らないと下流が当てにできる。
    assert!(
        read_central_directory(&nar).is_err(),
        "中央ディレクトリの段で既に拒否されている"
    );
}

/// 対応外の方式は、中央ディレクトリの段と伸長の段の**どちらの入口からも**撥ねられる。
///
/// 中央ディレクトリの白名簿だけを潰しても上のテストは緑のままだった＝伸長側の腕が
/// 受け止めていた。逆に伸長側の腕は通常の経路では踏まれないので、`RawEntry` を直に
/// 組んで踏ませる。これで 2 つの門がそれぞれ独立に赤を出せる。
#[test]
fn inflate_entry_refuses_an_unsupported_method_on_its_own() {
    let nar = NarBuilder::new()
        .file("body.txt", b"type,ghost")
        .done()
        .bytes();
    let mut entry = read_central_directory(&nar).expect("無傷の書庫は受理される")[0].clone();
    entry.method = 12;

    let reason = inflate_entry(&nar, &entry).expect_err("対応外の方式は伸長側でも撥ねる");
    let RefuseReason::UnsupportedEntry { ref what, .. } = reason else {
        panic!("対応外として拒否されるはず: {reason}");
    };
    assert_eq!(*what, Unsupported::Compression(12));
}

/// 伸長の上限は宣言サイズそのもの——宣言より大きく膨らむ流れは途中で打ち切られる。
///
/// 伸長爆弾の歯止め（設計「Security Considerations」）。上限を外すと、宣言 1 バイトの
/// エントリが 10 KiB を吐いても素通りしてしまう。
#[test]
fn inflate_stops_at_the_declared_size_so_a_bomb_cannot_expand() {
    let nar = NarBuilder::new()
        .file("bomb", &b"a".repeat(10_000))
        .deflate()
        .declared_size(1)
        .done()
        .bytes();
    assert!(
        nar.len() < 1024,
        "圧縮された固定入力は小さい: {} バイト",
        nar.len()
    );

    let reason = refusal(&nar);
    let RefuseReason::IntegrityMismatch { ref what, .. } = reason else {
        panic!("整合性の不一致で拒否されるはず: {reason}");
    };
    assert_eq!(
        *what,
        Integrity::Inflate,
        "宣言を超えた時点で伸長が止まる（黙って切り詰めも素通りもしない）"
    );
}

/// 宣言サイズの総和が上限を超えれば、1 GiB のデータを持たずに拒否できる。
///
/// `declared_size` は実データを変えずに宣言だけを大きくするので、固定入力は
/// 数十バイトのままで上限の分岐を踏める。
#[test]
fn declared_size_beyond_the_cap_is_refused_without_a_huge_fixture() {
    let half = MAX_TOTAL_DECLARED_SIZE / 2 + 1;
    let nar = NarBuilder::new()
        .file("a", b"x")
        .declared_size(half)
        .done()
        .file("b", b"y")
        .declared_size(half)
        .done()
        .bytes();
    assert!(
        nar.len() < 1024,
        "固定入力は小さいまま: {} バイト",
        nar.len()
    );

    let reason = refusal(&nar);
    let RefuseReason::UnsupportedEntry { ref what, .. } = reason else {
        panic!("対応外として拒否されるはず: {reason}");
    };
    assert_eq!(*what, Unsupported::TooLarge);
}

/// 上限ちょうどまでは通る（総和の判定が 1 件目だけを見ていないことの対照）。
#[test]
fn declared_size_at_the_cap_still_parses() {
    let nar = NarBuilder::new()
        .file("a", b"x")
        .declared_size(MAX_TOTAL_DECLARED_SIZE - 1)
        .done()
        .bytes();

    let entries = read_central_directory(&nar).expect("上限までは中央ディレクトリを読める");
    assert_eq!(entries[0].uncompressed_size, MAX_TOTAL_DECLARED_SIZE - 1);
}

// ---- 「1 バイトも書かない」（要件 2.6・5.9） ----

/// `container` の本体に決して現れてはならない綴り。
///
/// 書き込みの呼び出しを 1 つずつ数え上げる形は採らない。`use std::fs::*;` の後の
/// 素の `write(p, d)`・`File::options().write(true).create(true).open(p)`・
/// `hard_link`・`soft_link`・`symlink_file`・`Command::new(..).status()` のように、
/// 数え上げの網は必ず外側から回り込まれる。代わりに**本モジュールが正直に約束できる
/// 上位の性質**——ファイルシステムにも別プロセスにも触らない——を 3 語で見張る。
/// 読み取りの `fs::read` すら持たないので、`fs::` は丸ごと禁じてよい。
const FORBIDDEN_SPELLINGS: &[&str] = &["fs::", "Path", "process::"];

/// `source` に現れる禁じ手の綴りを拾う。
fn forbidden_spellings_in(source: &str) -> Vec<&'static str> {
    FORBIDDEN_SPELLINGS
        .iter()
        .copied()
        .filter(|needle| source.contains(needle))
        .collect()
}

/// `container` の本体が、ファイルシステムにも別プロセスにも触る綴りを 1 つも持たない。
///
/// 「観測しなかった」ではなく「触る手段を持たない」を主張する。読み手の署名は
/// `&[u8]` と `&RawEntry` しか受け取らず宛先のパスを知らないので、書くとすれば
/// `fs::`・`Path`／`PathBuf`・`process::` のいずれかが本体に要る。兄弟テスト
/// （このファイル）は `WorkDir` へ実際に書くので、走査の対象は本体だけに絞る。
#[test]
fn container_source_contains_no_filesystem_mutation() {
    let found = forbidden_spellings_in(include_str!("container.rs"));
    assert!(
        found.is_empty(),
        "container が禁じ手の綴りを持っている: {found:?}"
    );
}

/// 上の走査の較正。走査語が空でないこと、各語を足した写しが必ず赤になること、
/// 無関係な行では赤にならないことを、この 1 本で確かめる。
///
/// **最初の主張が要る**——走査語が空だと、上のテストは `found.is_empty()` で緑、
/// 下の繰り返しは 0 周で緑になり、二本とも恒真のまま「見張っている」ように見える。
#[test]
fn the_mutation_scan_catches_every_spelling_it_claims_to_watch() {
    assert_eq!(
        FORBIDDEN_SPELLINGS.len(),
        3,
        "走査語が空なら二本とも恒真で緑になる"
    );
    for needle in FORBIDDEN_SPELLINGS {
        let synthetic = format!("fn f() {{ let _ = {needle}; }}");
        assert_eq!(
            forbidden_spellings_in(&synthetic),
            vec![*needle],
            "綴り {needle} を走査が拾えていない"
        );
    }
    assert!(
        forbidden_spellings_in("fn f() { let _ = u32::from_le_bytes(x); }").is_empty(),
        "バイト列を読むだけの行は拾わない（走査が何にでも当たるのでは意味が無い）"
    );
}
