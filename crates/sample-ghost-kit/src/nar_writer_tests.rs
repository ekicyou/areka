//! `nar_writer` の兄弟テスト。
//!
//! 自作の書き手を自作の読み手で確かめるだけでは恒真になるので、**書き手と無関係な
//! 出どころ**を噛ませる——⑴ deflate の戻しは `miniz_oxide::inflate`（圧縮側とは別の
//! 実装）、⑵ CRC の刻印は外部の較正値で検算済みの `areka_nar::crc32`、⑶ 容器の構造は
//! 開発時に Python の `zipfile`（標準ライブラリ＝完全に別実装）で実際に開いて確かめた
//! （記録は検証報告に置く。常設のテストが外部道具に依存しないよう、ここでは走らせない）。

use super::*;
use std::path::PathBuf;

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

/// `target/` の下に本テスト専用の空フォルダを 1 つ作る（OS の一時フォルダは使わない）。
///
/// 段 ③ の展開先の名前空間フォルダは使わない——あの名前は展開先を指す約束で、
/// 見張りが「まだ現れていない」ことを検査している。本テストが要るのは書いたバイト列を
/// 置く場所だけなので、無関係な自前のフォルダを使う。タスク 2.5 で `WorkDir` が
/// 入ったらそちらへ寄せてよい。
fn scratch(tag: &str) -> PathBuf {
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/nar-writer-work"
    ))
    .join(format!("{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("scratch フォルダを作れる");
    path
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

// ---- 木を畳む入口 ----

#[test]
fn folding_a_tree_converts_nothing() {
    let root = scratch("fold");
    std::fs::create_dir_all(root.join("ghost/master")).expect("下位フォルダを作れる");
    // CRLF と Shift_JIS の高位バイトを両方含む——どちらも変換されてはならない。
    let body = b"charset,Shift_JIS\r\nname,\x82\xa0\x82\xa2\n\xff\x00 end";
    std::fs::write(root.join("ghost/master/descript.txt"), body).expect("書ける");
    std::fs::write(root.join("install.txt"), b"type,ghost\r\n").expect("書ける");

    let nar = fold_tree(&root).expect("木を畳める");
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
        fold_tree(&root).expect("2 度目").bytes(),
        nar.bytes(),
        "畳み込みも決定論的"
    );
    std::fs::remove_dir_all(&root).expect("片付けられる");
}

#[test]
fn folding_a_missing_tree_fails_loudly() {
    let root = scratch("missing");
    std::fs::remove_dir_all(&root).expect("消せる");
    let err = fold_tree(&root).expect_err("無いフォルダは黙って空を返さない");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn write_to_puts_the_same_bytes_on_disk() {
    let root = scratch("write");
    let nar = NarBuilder::new().file("a.txt", b"x").done();
    let path = root.join("a.nar");
    nar.write_to(&path).expect("書ける");
    assert_eq!(std::fs::read(&path).expect("読める"), nar.bytes());
    std::fs::remove_dir_all(&root).expect("片付けられる");
}

/// 設計「Testing Strategy」が並べる固定入力を、本 API だけで**全て組めること**を示す。
///
/// 判定は「組めて、中央ディレクトリの件数が期待どおり」までに留める（何が拒否されるかは
/// 読取層＝タスク 3.1 以降の仕事）。ここが赤くなるのは、後続のタスクが必要とする形を
/// 書き手が表せなくなったときだけ。
#[test]
fn every_fixture_the_testing_strategy_lists_can_be_built() {
    // container_tests
    let cases: Vec<(&str, NarBuilder)> = vec![
        ("無圧縮", NarBuilder::new().file("a.txt", b"x").done()),
        (
            "deflate",
            NarBuilder::new().file("a.txt", b"x").deflate().done(),
        ),
        (
            "CRC 不一致",
            NarBuilder::new()
                .file("a.txt", b"x")
                .corrupt(Corrupt::Crc)
                .done(),
        ),
        (
            "長さ不一致",
            NarBuilder::new()
                .file("a.txt", b"x")
                .corrupt(Corrupt::Size)
                .done(),
        ),
        (
            "中身の破損",
            NarBuilder::new()
                .file("a.txt", b"x")
                .corrupt(Corrupt::Data)
                .done(),
        ),
        (
            "暗号化ビット",
            NarBuilder::new()
                .file("a.txt", b"x")
                .encrypted_flag()
                .done(),
        ),
        (
            "方式 99",
            NarBuilder::new().file("a.txt", b"x").method(99).done(),
        ),
        (
            "方式 12",
            NarBuilder::new().file("a.txt", b"x").method(12).done(),
        ),
        // names_tests: 名前の生バイトは何であれそのまま通る。
        (
            "印つき UTF-8",
            NarBuilder::new().file("ゴースト/本体.txt", b"x").done(),
        ),
        (
            "印なし Shift_JIS",
            NarBuilder::new()
                .file(b"\x83S\x81[\x83X\x83g/\x96{\x91\xcc.txt".to_vec(), b"x")
                .utf8_flag(false)
                .done(),
        ),
        (
            "復号できない列",
            NarBuilder::new()
                .file(b"\xff\xfe\x80.txt".to_vec(), b"x")
                .utf8_flag(false)
                .done(),
        ),
        (
            "親へ昇る",
            NarBuilder::new().file("../escape.txt", b"x").done(),
        ),
        (
            "絶対パス",
            NarBuilder::new().file("/etc/passwd", b"x").done(),
        ),
        (
            "ドライブ文字",
            NarBuilder::new().file("C:/windows/x.txt", b"x").done(),
        ),
        (
            "UNC",
            NarBuilder::new().file(r"\host\share\x.txt", b"x").done(),
        ),
        (
            "円記号の区切り",
            NarBuilder::new().file(r"ghost\master\x.txt", b"x").done(),
        ),
        (
            "NUL",
            NarBuilder::new()
                .file(b"bad\x00name.txt".to_vec(), b"x")
                .done(),
        ),
        ("予約名", NarBuilder::new().file("CON.txt", b"x").done()),
        (
            "末尾ドット",
            NarBuilder::new().file("trailing.", b"x").done(),
        ),
        (
            "使えない記号",
            NarBuilder::new().file("a<>b.txt", b"x").done(),
        ),
        (
            "シンボリックリンク",
            NarBuilder::new().file("link", b"../x").symlink().done(),
        ),
        // manifest_tests: 中身は生バイトで渡せるので任意の文字コードを置ける。
        (
            "大文字キーと値の前の空白",
            NarBuilder::new()
                .file(
                    "install.txt",
                    &install_txt(&["Charset,UTF-8", "type, ghost"]),
                )
                .done(),
        ),
        (
            "charset 無しの Shift_JIS な name",
            NarBuilder::new()
                .file("install.txt", b"type,ghost\r\nname,\x82\xa0\x82\xa2\r\n")
                .done(),
        ),
        // plan_tests: フォルダのエントリの有無。
        (
            "フォルダのエントリ無し",
            NarBuilder::new().file("ghost/master/x.txt", b"x").done(),
        ),
    ];
    for (label, nar) in cases {
        assert_eq!(
            parse(&nar.bytes()).len(),
            1,
            "{label}: 1 件の書庫として組める"
        );
    }

    // 大小衝突は 2 件・空フォルダはフォルダのエントリ 1 件。
    let collision = NarBuilder::new()
        .file("A.txt", b"x")
        .done()
        .file("a.txt", b"y")
        .done();
    assert_eq!(parse(&collision.bytes()).len(), 2, "大小衝突の対");
    assert_eq!(
        parse(&NarBuilder::new().dir("empty").bytes()).len(),
        1,
        "空フォルダ"
    );

    // 宣言サイズの総和が 1 GiB の上限を超える（実データは 2 バイトのまま）。
    let over_cap = NarBuilder::new()
        .file("a.bin", b"x")
        .declared_size(768 << 20)
        .done()
        .file("b.bin", b"y")
        .declared_size(768 << 20)
        .done();
    let parsed = parse(&over_cap.bytes());
    assert!(
        parsed.iter().map(|p| p.uncomp_size as u64).sum::<u64>() > 1 << 30,
        "宣言サイズの総和が上限を超える"
    );
    assert_eq!(
        parsed.iter().map(|p| p.comp_size).sum::<u32>(),
        2,
        "実データは 2 バイト"
    );

    // 書庫まるごとの 4 形は、中央ディレクトリの中身を変えずに組める。
    for how in [
        Damage::NoEocd,
        Damage::Zip64Marker,
        Damage::MultiDisk,
        Damage::CentralOffsetOutOfRange,
    ] {
        let bytes = NarBuilder::new()
            .file("a.txt", b"x")
            .done()
            .damage(how)
            .bytes();
        assert!(!bytes.is_empty(), "{how:?} を立てても書庫は組める");
    }
}
