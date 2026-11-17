//! `nar_writer` の兄弟テスト。
//!
//! 自作の書き手を自作の読み手で確かめるだけでは恒真になるので、**書き手と無関係な
//! 出どころ**を噛ませる——⑴ deflate の戻しは `miniz_oxide::inflate`（圧縮側とは別の
//! 実装）、⑵ CRC の刻印は外部の較正値で検算済みの `areka_nar::crc32`、⑶ 容器の構造は
//! `areka-nar` の読取層が毎回読み直す——`areka-nar/src/container_tests.rs` の
//! `writer_bytes_survive_both_methods_unchanged` が、ここで組んだ無圧縮と deflate の
//! 書庫を本番の読み手に通し、原文と 1 バイトも違わずに戻ることを常設で確かめる
//! （読み手は書き手とは別に書き起こされたもので、写しではない）。

use super::*;
use crate::WorkDir;

// ---- テスト内に別途書き起こした最小の読み手（書き手の写しではない） ----

/// 中央ディレクトリ 1 件分の読み取り結果。
struct Parsed {
    name: Vec<u8>,
    flags: u16,
    method: u16,
    crc: u32,
    comp_size: u32,
    uncomp_size: u32,
    external_attrs: u32,
    /// ローカルヘッダの直後から `comp_size` ぶん切り出した生の中身。
    payload: Vec<u8>,
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// EOCD を末尾（書庫コメント無し）に見て、中央ディレクトリを辿る。
fn parse(bytes: &[u8]) -> Vec<Parsed> {
    let eocd = bytes.len() - 22;
    assert_eq!(&bytes[eocd..eocd + 4], b"PK\x05\x06", "EOCD の署名");
    let count = u16_at(bytes, eocd + 10) as usize;
    let mut at = u32_at(bytes, eocd + 16) as usize;
    let mut out = Vec::new();
    for _ in 0..count {
        assert_eq!(&bytes[at..at + 4], b"PK\x01\x02", "中央ディレクトリの署名");
        let name_len = u16_at(bytes, at + 28) as usize;
        let comp_size = u32_at(bytes, at + 20);
        let local = u32_at(bytes, at + 42) as usize;
        assert_eq!(
            &bytes[local..local + 4],
            b"PK\x03\x04",
            "ローカルヘッダの署名"
        );
        let data_at =
            local + 30 + u16_at(bytes, local + 26) as usize + u16_at(bytes, local + 28) as usize;
        out.push(Parsed {
            name: bytes[at + 46..at + 46 + name_len].to_vec(),
            flags: u16_at(bytes, at + 8),
            method: u16_at(bytes, at + 10),
            crc: u32_at(bytes, at + 16),
            comp_size,
            uncomp_size: u32_at(bytes, at + 24),
            external_attrs: u32_at(bytes, at + 38),
            payload: bytes[data_at..data_at + comp_size as usize].to_vec(),
        });
        at += 46 + name_len + u16_at(bytes, at + 30) as usize + u16_at(bytes, at + 32) as usize;
    }
    out
}

// ---- 往復 ----

#[test]
fn stored_entry_round_trips_byte_for_byte() {
    let body = b"\x82\xa0\x82\xa2\r\n\x00\xff tail";
    let nar = NarBuilder::new().file("ghost/master/dic.txt", body).done();
    let parsed = parse(&nar.bytes());
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].method, 0);
    assert_eq!(parsed[0].payload, body, "無圧縮は中身がそのまま");
    assert_eq!(parsed[0].uncomp_size as usize, body.len());
    assert_eq!(
        parsed[0].crc,
        areka_nar::crc32(body),
        "刻印は較正済みの CRC と一致"
    );
}

#[test]
fn deflate_entry_round_trips_through_an_independent_inflater() {
    let body = b"aaaaaaaaaabbbbbbbbbb\r\n\x82\xa0\x82\xa2 the same words the same words".repeat(4);
    let nar = NarBuilder::new()
        .file("ghost/master/dic.txt", &body)
        .deflate()
        .done();
    let parsed = parse(&nar.bytes());
    assert_eq!(parsed[0].method, 8);
    assert!(parsed[0].comp_size < body.len() as u32, "圧縮されている");
    let back = miniz_oxide::inflate::decompress_to_vec_with_limit(&parsed[0].payload, body.len())
        .expect("別実装の伸長器で戻せる");
    assert_eq!(back, body, "deflate の往復でバイト列が戻る");
    assert_eq!(parsed[0].uncomp_size as usize, body.len());
    assert_eq!(parsed[0].crc, areka_nar::crc32(&body));
}

#[test]
fn building_the_same_archive_twice_yields_identical_bytes() {
    let build = || {
        NarBuilder::new()
            .dir("ghost")
            .file("ghost/master/descript.txt", b"charset,Shift_JIS")
            .done()
            .file("install.txt", &install_txt(&["type,ghost", "name,emo2"]))
            .deflate()
            .done()
            .bytes()
    };
    assert_eq!(build(), build(), "時刻や並びの揺れが混ざっていない");
}

// ---- 印・属性 ----

#[test]
fn flags_and_attributes_land_in_the_headers() {
    let nar = NarBuilder::new()
        .file(b"\x83S\x81[\x83X\x83g/\x96{\x91\xcc.txt".to_vec(), b"x")
        .utf8_flag(false)
        .done()
        .file("link", b"target")
        .symlink()
        .done()
        .file("secret", b"x")
        .encrypted_flag()
        .done()
        .file("odd", b"x")
        .method(99)
        .done()
        .dir("empty/");
    let parsed = parse(&nar.bytes());
    assert_eq!(
        parsed[0].flags & (1 << 11),
        0,
        "印なし＝Shift_JIS の生バイト"
    );
    assert_eq!(
        parsed[0].name, b"\x83S\x81[\x83X\x83g/\x96{\x91\xcc.txt",
        "名前は無変換"
    );
    assert_eq!(parsed[0].flags & 1, 0, "既定では暗号化の印は立たない");
    assert_eq!(parsed[1].external_attrs >> 16 & 0xF000, 0xA000, "S_IFLNK");
    assert_eq!(parsed[2].flags & 1, 1, "汎用目的ビット 0＝暗号化");
    assert_eq!(parsed[3].method, 99);
    assert_eq!(parsed[4].name, b"empty/", "フォルダは `/` で終わる");

    let utf8 = NarBuilder::new().file("ゴースト/本体.txt", b"x").done();
    assert_eq!(
        parse(&utf8.bytes())[0].flags & (1 << 11),
        1 << 11,
        "既定は印あり"
    );
}

// ---- 壊し口が外科的であること ----

#[test]
fn each_corruption_changes_exactly_one_thing() {
    let build = |how: Option<Corrupt>| {
        let mut entry = NarBuilder::new().file("a.txt", b"abcdefghij");
        if let Some(how) = how {
            entry = entry.corrupt(how);
        }
        parse(&entry.done().bytes()).remove(0)
    };
    let clean = build(None);

    let crc = build(Some(Corrupt::Crc));
    assert_ne!(crc.crc, clean.crc, "CRC だけが違う");
    assert_eq!(crc.uncomp_size, clean.uncomp_size);
    assert_eq!(crc.comp_size, clean.comp_size);
    assert_eq!(crc.payload, clean.payload);

    let size = build(Some(Corrupt::Size));
    assert_ne!(size.uncomp_size, clean.uncomp_size, "宣言サイズだけが違う");
    assert_eq!(size.crc, clean.crc);
    assert_eq!(size.comp_size, clean.comp_size);
    assert_eq!(size.payload, clean.payload);

    let data = build(Some(Corrupt::Data));
    assert_ne!(data.payload, clean.payload, "中身だけが違う");
    assert_eq!(data.crc, clean.crc);
    assert_eq!(data.uncomp_size, clean.uncomp_size);
    assert_eq!(data.comp_size, clean.comp_size, "長さは変えない");
}

#[test]
fn declared_size_can_be_set_far_beyond_the_data() {
    let nar = NarBuilder::new()
        .file("big.txt", b"x")
        .declared_size(1 << 30)
        .done();
    let parsed = parse(&nar.bytes());
    assert_eq!(parsed[0].uncomp_size, 1 << 30);
    assert_eq!(parsed[0].comp_size, 1, "実データは 1 バイトのまま");
}

// ---- 書庫まるごとの壊し口 ----

#[test]
fn archive_level_damage_is_visible_in_the_trailer() {
    let base = NarBuilder::new().file("a.txt", b"x").done();
    let eocd = base.bytes().len() - 22;

    assert_eq!(
        base.clone().damage(Damage::NoEocd).bytes().len(),
        eocd,
        "EOCD が丸ごと無い"
    );

    let zip64 = base.clone().damage(Damage::Zip64Marker).bytes();
    assert_eq!(u16_at(&zip64, eocd + 10), 0xFFFF, "件数欄が zip64 の印");

    let disk = base.clone().damage(Damage::MultiDisk).bytes();
    assert_eq!(u16_at(&disk, eocd + 4), 1, "ディスク番号 ≠ 0");

    let far = base.clone().damage(Damage::CentralOffsetOutOfRange).bytes();
    let offset = u32_at(&far, eocd + 16);
    assert!(
        offset as usize > far.len(),
        "中央ディレクトリの位置が範囲外"
    );
    assert_ne!(offset, u32::MAX, "zip64 の印とは取り違えない値");
}

// ---- install.txt ----

#[test]
fn install_txt_joins_with_crlf() {
    assert_eq!(
        install_txt(&["type,ghost", "  name,emo2"]),
        b"type,ghost\r\n  name,emo2\r\n".to_vec()
    );
}

/// 局所ヘッダの時刻と日付（オフセット 10..14）は時計を読まない固定値（audit-test-kits 指摘 25）。
#[test]
fn every_local_header_carries_the_fixed_dos_time() {
    let nar = NarBuilder::new()
        .dir("ghost")
        .file("a.txt", b"one")
        .done()
        .file("b.txt", b"two")
        .deflate()
        .done()
        .bytes();
    let mut at = 0;
    let mut seen = 0;
    while let Some(found) = nar[at..].windows(4).position(|w| w == b"PK\x03\x04") {
        let local = at + found;
        assert_eq!(u16_at(&nar, local + 10), 0x0000, "DOS 時刻");
        assert_eq!(u16_at(&nar, local + 12), 0x0021, "DOS 日付");
        seen += 1;
        at = local + 4;
    }
    assert!(seen >= 3, "較正: 局所ヘッダを全件見たこと: {seen}");
}

// ---- 木を畳む入口 ----

#[test]
fn folding_a_tree_converts_nothing() {
    let work = WorkDir::new().expect("私有の場所");
    let root = work.path();
    std::fs::create_dir_all(root.join("ghost/master")).expect("下位フォルダを作れる");
    // CRLF と Shift_JIS の高位バイトを両方含む——どちらも変換されてはならない。
    let body = b"charset,Shift_JIS\r\nname,\x82\xa0\x82\xa2\n\xff\x00 end";
    std::fs::write(root.join("ghost/master/descript.txt"), body).expect("書ける");
    std::fs::write(root.join("install.txt"), b"type,ghost\r\n").expect("書ける");

    let nar = fold_tree(root).expect("木を畳める");
    let parsed = parse(&nar.bytes());
    let names: Vec<Vec<u8>> = parsed.iter().map(|p| p.name.clone()).collect();
    assert_eq!(
        names,
        vec![
            b"ghost/".to_vec(),
            b"ghost/master/".to_vec(),
            b"ghost/master/descript.txt".to_vec(),
            b"install.txt".to_vec(),
        ],
        "区切りは `/` 固定・フォルダも 1 件ずつ出る"
    );
    let descript = parsed
        .iter()
        .find(|p| p.name == b"ghost/master/descript.txt")
        .expect("畳まれている");
    assert_eq!(descript.payload, body, "改行も文字コードも無変換");
    assert!(
        parsed.iter().all(|p| p.flags & (1 << 11) != 0),
        "名前は印つきの UTF-8"
    );

    assert_eq!(
        fold_tree(root).expect("2 度目").bytes(),
        nar.bytes(),
        "畳み込みも決定論的"
    );
}

#[test]
fn folding_a_missing_tree_fails_loudly() {
    let work = WorkDir::new().expect("私有の場所");
    let root = work.path().join("missing");
    let err = fold_tree(&root).expect_err("無いフォルダは黙って空を返さない");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    let message = err.to_string();
    assert!(
        message.starts_with("list ") && message.contains("missing"),
        "読めなかったフォルダのパスを文に添える: {message}"
    );
}

#[test]
fn write_to_puts_the_same_bytes_on_disk() {
    let work = WorkDir::new().expect("私有の場所");
    let root = work.path();
    let nar = NarBuilder::new().file("a.txt", b"x").done();
    let path = root.join("a.nar");
    nar.write_to(&path).expect("書ける");
    assert_eq!(std::fs::read(&path).expect("読める"), nar.bytes());
}

/// 畳む順は名前のバイト順（大文字が小文字より先）で、NTFS の `read_dir` の順（大小無視）
/// とは食い違う（audit-test-kits 指摘 25）。
#[test]
fn folding_orders_entries_by_byte_order() {
    let work = WorkDir::new().expect("私有の場所");
    let root = work.path();
    std::fs::write(root.join("a.txt"), b"a").expect("書ける");
    std::fs::write(root.join("B.txt"), b"b").expect("書ける");

    let names: Vec<Vec<u8>> = parse(&fold_tree(root).expect("木を畳める").bytes())
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, vec![b"B.txt".to_vec(), b"a.txt".to_vec()]);
}

/// 名前が UTF-8 にならない（対になっていないサロゲートの）ファイルは、黙って飛ばさずに
/// `InvalidData` で断り、文は ASCII だけで書く（audit-test-kits かけ直し 4 回目 U6）。
#[test]
fn folding_a_name_that_is_not_utf8_fails_with_invalid_data_in_ascii() {
    use std::os::windows::ffi::OsStringExt;
    let work = WorkDir::new().expect("私有の場所");
    let root = work.path().join("ひらがな");
    std::fs::create_dir(&root).expect("作れる");
    let lone = std::ffi::OsString::from_wide(&[0xD800]);
    std::fs::write(root.join(&lone), b"x")
        .expect("NTFS は対になっていないサロゲートの名前を置ける");

    let err = fold_tree(&root).expect_err("UTF-8 にならない名前は断る");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    let message = err.to_string();
    assert!(
        message.starts_with("name is not valid UTF-8: ") && message.is_ascii(),
        "文は ASCII だけ: {message}"
    );
    assert!(
        message.contains("\\u{3072}"),
        "フォルダの日本語は逃がして添える: {message}"
    );
}
