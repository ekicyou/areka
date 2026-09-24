//! zip コンテナの読みと伸長・整合性検査（要件 2.1・2.5・2.6・5.9）。
//!
//! `.nar` は zip の拡張子を変えただけのものなので、拡張子は一切見ない。入口は
//! バイト列だけを受け取り、末尾の終端記録（EOCD）→ 中央ディレクトリ → ローカル
//! ヘッダの順に辿って [`RawEntry`] の列を作る。
//!
//! # ここが書き込みを持たない理由
//!
//! 伸長と CRC の突合は `open` の中で**全エントリに対して**行い、伸長済みのバイト列は
//! 呼び出し側（`NarArchive`）が保持する。展開の段はそれを書くだけで再伸長しない。
//! この分け方のおかげで「壊れていれば 1 バイトも書かない」（要件 2.6）と「書き込みの
//! 前に検証を終える」（要件 4.1）が同じ場所で成り立つ。本モジュールは宛先のパスを
//! 受け取らず、ファイルシステムを変える呼び出しを 1 つも持たない（兄弟テストが
//! 字面で見張る）。
//!
//! # 名前を復号しない理由
//!
//! 名前は生バイトのまま [`RawEntry::name_raw`] に載せ、復号と安全性の判定は `names`
//! が受け持つ。ここで復号すると、拒否の理由に要る「生バイトの 16 進」（要件 2.4）を
//! 作り直せなくなる。エラーの表示に使う名前だけは、診断のために非可逆な写し
//! （[`String::from_utf8_lossy`]）を作る。

use crate::error::{Integrity, RefuseReason, Unsupported, bounded_value};
use std::ops::Range;

/// 終端記録（End Of Central Directory）の署名。
const EOCD_SIGNATURE: &[u8] = b"PK\x05\x06";
/// 中央ディレクトリの各ヘッダの署名。
const CENTRAL_SIGNATURE: &[u8] = b"PK\x01\x02";
/// ローカルヘッダの署名。
const LOCAL_SIGNATURE: &[u8] = b"PK\x03\x04";
/// zip64 の終端記録ロケータの署名。EOCD の直前に置かれる。
const ZIP64_LOCATOR_SIGNATURE: &[u8] = b"PK\x06\x07";

/// 終端記録の固定部の長さ（書庫コメントを除く）。
const EOCD_LEN: usize = 22;
/// zip64 の終端記録ロケータの長さ。
const ZIP64_LOCATOR_LEN: usize = 20;
/// 中央ディレクトリ 1 件の固定部の長さ（名前・拡張欄・注釈を除く）。
const CENTRAL_HEADER_LEN: usize = 46;
/// ローカルヘッダの固定部の長さ（名前・拡張欄を除く）。
const LOCAL_HEADER_LEN: usize = 30;

/// 無圧縮。
const METHOD_STORE: u16 = 0;
/// deflate。
const METHOD_DEFLATE: u16 = 8;
/// AES。方式の番号を借りているだけで実体は暗号化なので、暗号化として拒否する。
const METHOD_AES: u16 = 99;

/// 汎用目的ビット 0＝暗号化。
const FLAG_ENCRYPTED: u16 = 1;
/// 汎用目的ビット 11＝「名前は UTF-8」。
const FLAG_UTF8_NAME: u16 = 1 << 11;

/// 受け入れる宣言サイズの総和の上限（1 GiB）。
///
/// 伸長済みのバイト列を全エントリぶん保持する設計なので、この値がそのまま
/// メモリの上限になる。ゴーストの実寸は数十 MB なので実害は無く、宣言だけを
/// 大きくした書庫（伸長爆弾）は 1 バイトも伸長せずに撥ねられる。
pub(crate) const MAX_TOTAL_DECLARED_SIZE: u64 = 1 << 30;

/// 書庫そのものに対する拒否で、エントリ名の代わりに置く語。
///
/// 語彙 `UnsupportedEntry` はエントリ番号と名前を持つが、終端記録の段階では
/// まだ 1 件も読めていない。番号 0 とこの語で「個別のエントリではない」と示す。
/// 表示側（`error.rs`）が名前を `（…）` で包むので、この語自身は括弧を持たない。
const WHOLE_ARCHIVE: &str = "書庫全体";

/// 中央ディレクトリから読んだエントリ 1 件。名前は復号しない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawEntry {
    /// 中央ディレクトリに並ぶ順。0 始まり。
    pub index: usize,
    /// エントリ名の生バイト。
    pub name_raw: Vec<u8>,
    /// 汎用目的ビット 11（「名前は UTF-8」）。
    pub utf8_flag: bool,
    /// 名前が `/` で終わる＝フォルダのエントリ。
    pub is_dir: bool,
    /// 外部属性。上位 16 bit の種別が `S_IFLNK` ならシンボリックリンク（判定は `names`）。
    pub external_attrs: u32,
    /// 中央ディレクトリに刻まれた CRC-32。
    pub crc32: u32,
    /// 中央ディレクトリが宣言する伸長後の長さ。
    pub uncompressed_size: u64,
    /// 圧縮方式。ここまで来たものは 0 か 8 のどちらか。
    pub method: u16,
    /// 書庫のバイト列の中で、このエントリの圧縮データが占める範囲。
    pub data: Range<usize>,
}

/// 書庫のバイト列から中央ディレクトリを読み、エントリ列を中央ディレクトリの順で返す。
///
/// 各エントリの内容はローカルヘッダを飛ばした位置から**中央ディレクトリの**圧縮
/// サイズぶんを切り出す。ローカルヘッダ側の長さと CRC は使わないので、データ記述子
/// （汎用目的ビット 3）が立っていても読める。
pub(crate) fn read_central_directory(bytes: &[u8]) -> Result<Vec<RawEntry>, RefuseReason> {
    let eocd = find_eocd(bytes)?;
    let (count, cd_offset, cd_end) = read_eocd(bytes, eocd)?;

    let mut entries = Vec::with_capacity(count);
    let mut declared_total: u64 = 0;
    let mut at = cd_offset;
    for index in 0..count {
        let (entry, next) = read_central_header(bytes, at, cd_end, index)?;
        declared_total = declared_total.saturating_add(entry.uncompressed_size);
        if declared_total > MAX_TOTAL_DECLARED_SIZE {
            return Err(unsupported(&entry.name_raw, index, Unsupported::TooLarge));
        }
        entries.push(entry);
        at = next;
    }
    Ok(entries)
}

/// エントリ 1 件を伸長し、長さと CRC-32 を中央ディレクトリの宣言と突き合わせる。
///
/// 突合に通ったバイト列だけを返す。改行も文字コードも触らない（要件 5.9）。
pub(crate) fn inflate_entry(bytes: &[u8], entry: &RawEntry) -> Result<Vec<u8>, RefuseReason> {
    let data = bytes.get(entry.data.clone()).ok_or_else(|| {
        corrupt(format!(
            "エントリ {} の内容が書庫の外を指している（{}..{}・書庫 {} バイト）",
            entry.index,
            entry.data.start,
            entry.data.end,
            bytes.len()
        ))
    })?;
    // 伸長の上限は宣言サイズそのもの。総和の上限を通っているので usize に収まるが、
    // 収まらない環境でも黙って切り詰めず「大きすぎる」として撥ねる。
    let limit = usize::try_from(entry.uncompressed_size)
        .map_err(|_| unsupported(&entry.name_raw, entry.index, Unsupported::TooLarge))?;

    let out = match entry.method {
        METHOD_STORE => data.to_vec(),
        METHOD_DEFLATE => miniz_oxide::inflate::decompress_to_vec_with_limit(data, limit)
            .map_err(|_| integrity(entry, Integrity::Inflate))?,
        other => {
            return Err(unsupported(
                &entry.name_raw,
                entry.index,
                Unsupported::Compression(other),
            ));
        }
    };

    let actual_size = out.len() as u64;
    if actual_size != entry.uncompressed_size {
        return Err(integrity(
            entry,
            Integrity::Size {
                expected: entry.uncompressed_size,
                actual: actual_size,
            },
        ));
    }
    let actual_crc = crate::crc32::crc32(&out);
    if actual_crc != entry.crc32 {
        return Err(integrity(
            entry,
            Integrity::Crc {
                expected: entry.crc32,
                actual: actual_crc,
            },
        ));
    }
    Ok(out)
}

// ---- 終端記録 ----

/// 末尾から終端記録を探し、その開始位置を返す。
///
/// 書庫コメントは 64 KiB まで置けるので、末尾から `EOCD_LEN + 65535` バイトだけを
/// 後ろから走査する。署名の並びは中身のバイト列にも偶然現れ得るので、コメント長の
/// 欄が「そこから書庫の終わりまで」と辻褄の合う位置だけを採る。
fn find_eocd(bytes: &[u8]) -> Result<usize, RefuseReason> {
    let last = bytes.len().checked_sub(EOCD_LEN).ok_or_else(|| {
        corrupt(format!(
            "終端記録が入る長さが無い（{} バイト）",
            bytes.len()
        ))
    })?;
    let first = last.saturating_sub(u16::MAX as usize);
    for at in (first..=last).rev() {
        if &bytes[at..at + 4] != EOCD_SIGNATURE {
            continue;
        }
        let comment_len = u16_at(bytes, at + 20)
            .ok_or_else(|| corrupt("終端記録が途中で切れている（書庫コメントの長さが読めない）"))?
            as usize;
        if at + EOCD_LEN + comment_len == bytes.len() {
            return Ok(at);
        }
    }
    Err(corrupt(format!(
        "終端記録（PK\\x05\\x06）が末尾に見つからない（書庫 {} バイト）",
        bytes.len()
    )))
}

/// 終端記録を読み、(エントリ数, 中央ディレクトリの開始, 中央ディレクトリの終わり) を返す。
fn read_eocd(bytes: &[u8], eocd: usize) -> Result<(usize, usize, usize), RefuseReason> {
    let cut = || corrupt("終端記録が途中で切れている");
    let disk = u16_at(bytes, eocd + 4).ok_or_else(cut)?;
    let cd_disk = u16_at(bytes, eocd + 6).ok_or_else(cut)?;
    let count_here = u16_at(bytes, eocd + 8).ok_or_else(cut)?;
    let count_total = u16_at(bytes, eocd + 10).ok_or_else(cut)?;
    let cd_size = u32_at(bytes, eocd + 12).ok_or_else(cut)?;
    let cd_offset = u32_at(bytes, eocd + 16).ok_or_else(cut)?;

    // zip64 は、欄が飽和した値になるか、終端記録の直前にロケータが置かれるかで分かる。
    let saturated = disk == u16::MAX
        || cd_disk == u16::MAX
        || count_here == u16::MAX
        || count_total == u16::MAX
        || cd_size == u32::MAX
        || cd_offset == u32::MAX;
    let has_locator = eocd >= ZIP64_LOCATOR_LEN
        && &bytes[eocd - ZIP64_LOCATOR_LEN..eocd - ZIP64_LOCATOR_LEN + 4]
            == ZIP64_LOCATOR_SIGNATURE;
    if saturated || has_locator {
        return Err(unsupported(b"", 0, Unsupported::Zip64));
    }
    if disk != 0 || cd_disk != 0 || count_here != count_total {
        return Err(unsupported(b"", 0, Unsupported::MultiDisk));
    }

    let cd_offset = cd_offset as usize;
    let cd_size = cd_size as usize;
    let cd_end = cd_offset
        .checked_add(cd_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| {
            corrupt(format!(
                "中央ディレクトリの位置が範囲外（位置 {cd_offset}・長さ {cd_size}・書庫 {} バイト）",
                bytes.len()
            ))
        })?;
    Ok((count_total as usize, cd_offset, cd_end))
}

// ---- 中央ディレクトリ ----

/// 中央ディレクトリの 1 件を `at` から読み、エントリと次の件の位置を返す。
fn read_central_header(
    bytes: &[u8],
    at: usize,
    cd_end: usize,
    index: usize,
) -> Result<(RawEntry, usize), RefuseReason> {
    if bytes.get(at..at + 4) != Some(CENTRAL_SIGNATURE) {
        return Err(corrupt(format!(
            "中央ディレクトリ {index} 件目の署名が合わない（位置 {at}）"
        )));
    }
    let cut = || corrupt(format!("中央ディレクトリ {index} 件目が途中で切れている"));
    let flags = u16_at(bytes, at + 8).ok_or_else(cut)?;
    let method = u16_at(bytes, at + 10).ok_or_else(cut)?;
    let crc32 = u32_at(bytes, at + 16).ok_or_else(cut)?;
    let comp_size = u32_at(bytes, at + 20).ok_or_else(cut)?;
    let uncomp_size = u32_at(bytes, at + 24).ok_or_else(cut)?;
    let name_len = u16_at(bytes, at + 28).ok_or_else(cut)? as usize;
    let extra_len = u16_at(bytes, at + 30).ok_or_else(cut)? as usize;
    let comment_len = u16_at(bytes, at + 32).ok_or_else(cut)? as usize;
    let disk_start = u16_at(bytes, at + 34).ok_or_else(cut)?;
    let external_attrs = u32_at(bytes, at + 38).ok_or_else(cut)?;
    let local_offset = u32_at(bytes, at + 42).ok_or_else(cut)?;

    let name_at = at + CENTRAL_HEADER_LEN;
    let next = name_at + name_len + extra_len + comment_len;
    if next > cd_end {
        return Err(corrupt(format!(
            "中央ディレクトリ {index} 件目が中央ディレクトリの外へはみ出す（終わり {cd_end}）"
        )));
    }
    let name_raw = bytes
        .get(name_at..name_at + name_len)
        .ok_or_else(cut)?
        .to_vec();

    if comp_size == u32::MAX || uncomp_size == u32::MAX || local_offset == u32::MAX {
        return Err(unsupported(&name_raw, index, Unsupported::Zip64));
    }
    if disk_start != 0 {
        return Err(unsupported(&name_raw, index, Unsupported::MultiDisk));
    }
    // 方式 99（AES）は暗号化の別の綴りなので、暗号化ビットと同じ扱いにする。
    if flags & FLAG_ENCRYPTED != 0 || method == METHOD_AES {
        return Err(unsupported(&name_raw, index, Unsupported::Encrypted));
    }
    if method != METHOD_STORE && method != METHOD_DEFLATE {
        return Err(unsupported(
            &name_raw,
            index,
            Unsupported::Compression(method),
        ));
    }

    let data = locate_entry_data(bytes, local_offset as usize, comp_size as usize, index)?;
    Ok((
        RawEntry {
            index,
            is_dir: name_raw.ends_with(b"/"),
            utf8_flag: flags & FLAG_UTF8_NAME != 0,
            name_raw,
            external_attrs,
            crc32,
            uncompressed_size: uncomp_size as u64,
            method,
            data,
        },
        next,
    ))
}

/// ローカルヘッダを飛ばし、圧縮データが占める範囲を返す。
fn locate_entry_data(
    bytes: &[u8],
    local: usize,
    comp_size: usize,
    index: usize,
) -> Result<Range<usize>, RefuseReason> {
    if bytes.get(local..local + 4) != Some(LOCAL_SIGNATURE) {
        return Err(corrupt(format!(
            "エントリ {index} のローカルヘッダの署名が合わない（位置 {local}）"
        )));
    }
    // ローカルヘッダ側の長さと CRC は読まない（データ記述子が立っていると 0 になる）。
    let cut = || {
        corrupt(format!(
            "エントリ {index} のローカルヘッダが途中で切れている"
        ))
    };
    let name_len = u16_at(bytes, local + 26).ok_or_else(cut)? as usize;
    let extra_len = u16_at(bytes, local + 28).ok_or_else(cut)? as usize;
    let start = local + LOCAL_HEADER_LEN + name_len + extra_len;
    let end = start
        .checked_add(comp_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| {
            corrupt(format!(
                "エントリ {index} の内容が範囲外（{start} から {comp_size} バイト・書庫 {} バイト）",
                bytes.len()
            ))
        })?;
    Ok(start..end)
}

// ---- 拒否の組み立てと数の読み ----

/// 構造そのものが読めないときの拒否。説明文が壊れ方を言い分ける。
fn corrupt(detail: impl Into<String>) -> RefuseReason {
    RefuseReason::CorruptArchive {
        detail: detail.into(),
    }
}

/// 対応していない形式の拒否。名前は診断用の非可逆な写し（正しい復号は `names`）。
fn unsupported(name_raw: &[u8], index: usize, what: Unsupported) -> RefuseReason {
    let name = if name_raw.is_empty() {
        WHOLE_ARCHIVE.to_string()
    } else {
        // 名前の長さの検査より前に起きる拒否なので、ここで縛る。
        bounded_value(&String::from_utf8_lossy(name_raw))
    };
    RefuseReason::UnsupportedEntry { index, name, what }
}

/// 整合性の不一致の拒否。
fn integrity(entry: &RawEntry, what: Integrity) -> RefuseReason {
    RefuseReason::IntegrityMismatch {
        index: entry.index,
        name: String::from_utf8_lossy(&entry.name_raw).into_owned(),
        what,
    }
}

/// `at` から 2 バイトを little endian で読む。範囲外なら `None`。
fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let field: [u8; 2] = bytes.get(at..)?.get(..2)?.try_into().ok()?;
    Some(u16::from_le_bytes(field))
}

/// `at` から 4 バイトを little endian で読む。範囲外なら `None`。
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let field: [u8; 4] = bytes.get(at..)?.get(..4)?.try_into().ok()?;
    Some(u32::from_le_bytes(field))
}

#[cfg(test)]
#[path = "container_tests.rs"]
mod tests;
