//! 決定論テストの固定入力になる最小の `.nar`（zip）書き手（spec: `areka-P0-nar-install`
//! 要件 9.3・9.4）。
//!
//! # なぜ本 crate に住むか
//!
//! `areka-nar` の `#[cfg(test)]` に置くとテストバイナリの内側にしか存在せず、
//! `ghost-install` の `[dev-dependencies]` から届かない。ここに置けば `areka-nar` 自身の
//! テストも `ghost-install` のテストも `[dev-dependencies] sample-ghost-kit` の 1 行で
//! 同じ入力を得る。
//!
//! # 書けるもの
//!
//! 無圧縮（方式 0）と deflate（方式 8）・名前の生バイト・「名前は UTF-8」の印
//! （汎用目的ビット 11）の有無・外部属性（シンボリックリンク）・暗号化の印
//! （ビット 0）・対応外の圧縮方式・宣言サイズの上書き。加えて **CRC・宣言サイズ・
//! 中身**のいずれか 1 つだけを外科的に壊す口（[`Corrupt`]）と、**書庫そのもの**を
//! 壊す口（[`Damage`]）を持つ。無傷の入力と、印を 1 つだけ立てた同じ入力との対を
//! 作れるので、読取層のどの検査が働いたのかを取り違えずに判定できる。
//!
//! # 決定論
//!
//! DOS 日時は 1980-01-01 00:00 に固定し、時計も乱数も連想配列の反復順も使わない。
//! 同じ呼び方は必ず同じバイト列になる（[`fold_tree`] の走査も名前順に固定する）。
//!
//! ```rust
//! use sample_ghost_kit::{install_txt, NarBuilder};
//!
//! let nar = NarBuilder::new()
//!     .file("install.txt", &install_txt(&["type,ghost", "directory,emo2"]))
//!     .done()
//!     .file("ghost/master/descript.txt", b"charset,Shift_JIS")
//!     .deflate()
//!     .done();
//! assert_eq!(nar.bytes(), nar.bytes());
//! ```

use std::path::Path;

/// DOS 時刻 00:00:00。時計を読まないので固定値。
const DOS_TIME: u16 = 0;
/// DOS 日付 1980-01-01（zip の日付欄で表せる最小の日）。
const DOS_DATE: u16 = 0x0021;
/// 「Unix の 3.0 で作った」——外部属性の上位 16 bit を Unix の種別・権限として読ませる。
const VERSION_MADE_BY: u16 = 0x031E;
/// 展開に要る最低の版（2.0＝deflate まで）。
const VERSION_NEEDED: u16 = 20;
/// Windows のフォルダ属性（`FILE_ATTRIBUTE_DIRECTORY`）。
const ATTR_DIRECTORY: u32 = 0x10;
/// Unix の `S_IFLNK | 0777` を外部属性の上位 16 bit に置いた値。
const ATTR_SYMLINK: u32 = 0xA1FF_0000;

/// エントリ 1 件を意図的に壊す口。**壊すのは常に 1 つだけ**で、残りは無傷のまま。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corrupt {
    /// 刻まれた CRC だけを違う値にする（宣言サイズと中身は正しいまま）。
    Crc,
    /// 宣言した伸長後サイズだけを 1 大きくする（CRC と中身は正しいまま）。
    Size,
    /// 中身のバイトだけを書き換える（CRC も宣言サイズも長さも正しいまま）。
    Data,
}

/// 書庫そのものを意図的に壊す口。中央ディレクトリの各エントリは無傷のまま。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Damage {
    /// EOCD（`PK\x05\x06`）を丸ごと書かない。
    NoEocd,
    /// EOCD の件数欄を `0xFFFF` にする（zip64 の印）。
    Zip64Marker,
    /// ディスク番号を 1 にする（分割書庫の印）。
    MultiDisk,
    /// 中央ディレクトリの位置をファイルの外に置く（`0xFFFF_FFFF` は zip64 の印と
    /// 取り違えられるので使わない）。
    CentralOffsetOutOfRange,
}

/// 組み立て中のエントリ 1 件。
#[derive(Clone, Debug)]
struct Entry {
    name_raw: Vec<u8>,
    data: Vec<u8>,
    utf8_flag: bool,
    encrypted: bool,
    deflate: bool,
    /// 明示した圧縮方式（対応外の方式を書くとき）。`None` なら `deflate` から決める。
    method: Option<u16>,
    external_attrs: u32,
    /// 明示した伸長後サイズ。`None` なら実データの長さ。
    declared_size: Option<u64>,
    corrupt: Option<Corrupt>,
}

/// テスト用の `.nar` を組む本体。
#[derive(Clone, Debug, Default)]
pub struct NarBuilder {
    entries: Vec<Entry>,
    damage: Option<Damage>,
}

/// [`NarBuilder::file`] が返す、1 件ぶんの設定を受ける形。[`EntryBuilder::done`] で戻る。
#[derive(Clone, Debug)]
pub struct EntryBuilder {
    builder: NarBuilder,
    entry: Entry,
}

impl NarBuilder {
    /// 空の書庫から始める。
    pub fn new() -> Self {
        Self::default()
    }

    /// ファイルのエントリを 1 件足す。既定は **UTF-8 の印あり・無圧縮**。
    pub fn file(self, name: impl Into<Vec<u8>>, data: &[u8]) -> EntryBuilder {
        EntryBuilder {
            builder: self,
            entry: Entry {
                name_raw: name.into(),
                data: data.to_vec(),
                utf8_flag: true,
                encrypted: false,
                deflate: false,
                method: None,
                external_attrs: 0,
                declared_size: None,
                corrupt: None,
            },
        }
    }

    /// フォルダのエントリを 1 件足す（名前が `/` で終わっていなければ足す）。
    pub fn dir(self, name: impl Into<Vec<u8>>) -> Self {
        let mut name_raw = name.into();
        if !name_raw.ends_with(b"/") {
            name_raw.push(b'/');
        }
        self.file(name_raw, b"")
            .external_attrs(ATTR_DIRECTORY)
            .done()
    }

    /// 書庫そのものを壊す口を立てる。
    pub fn damage(mut self, how: Damage) -> Self {
        self.damage = Some(how);
        self
    }

    /// 組み上がったバイト列を返す。同じ呼び方は必ず同じバイト列になる。
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for entry in &self.entries {
            let offset = u32::try_from(out.len()).expect("テスト用の書庫は 4 GiB を超えない");
            let (auto_method, mut payload) = if entry.deflate {
                (8u16, miniz_oxide::deflate::compress_to_vec(&entry.data, 6))
            } else {
                (0u16, entry.data.clone())
            };
            let method = entry.method.unwrap_or(auto_method);
            let mut crc = areka_nar::crc32(&entry.data);
            let mut declared = entry.declared_size.unwrap_or(entry.data.len() as u64);
            match entry.corrupt {
                Some(Corrupt::Crc) => crc ^= 1,
                Some(Corrupt::Size) => declared += 1,
                Some(Corrupt::Data) => {
                    assert!(
                        !payload.is_empty(),
                        "Corrupt::Data は中身のあるエントリにしか立てられない（長さを変えずに\
                         書き換える口なので、空のエントリでは壊す先が無い）"
                    );
                    payload[0] ^= 0xFF;
                }
                None => {}
            }
            let declared = u32::try_from(declared).expect("テスト用のエントリは 4 GiB を超えない");
            let comp_size =
                u32::try_from(payload.len()).expect("テスト用のエントリは 4 GiB を超えない");
            let flags = if entry.utf8_flag { 1u16 << 11 } else { 0 }
                | if entry.encrypted { 1u16 } else { 0 };
            let name_len =
                u16::try_from(entry.name_raw.len()).expect("テスト用の名前は 64 KiB を超えない");

            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&VERSION_NEEDED.to_le_bytes());
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&DOS_TIME.to_le_bytes());
            out.extend_from_slice(&DOS_DATE.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&comp_size.to_le_bytes());
            out.extend_from_slice(&declared.to_le_bytes());
            out.extend_from_slice(&name_len.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra
            out.extend_from_slice(&entry.name_raw);
            out.extend_from_slice(&payload);

            central.extend_from_slice(b"PK\x01\x02");
            central.extend_from_slice(&VERSION_MADE_BY.to_le_bytes());
            central.extend_from_slice(&VERSION_NEEDED.to_le_bytes());
            central.extend_from_slice(&flags.to_le_bytes());
            central.extend_from_slice(&method.to_le_bytes());
            central.extend_from_slice(&DOS_TIME.to_le_bytes());
            central.extend_from_slice(&DOS_DATE.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&comp_size.to_le_bytes());
            central.extend_from_slice(&declared.to_le_bytes());
            central.extend_from_slice(&name_len.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // extra
            central.extend_from_slice(&0u16.to_le_bytes()); // comment
            central.extend_from_slice(&0u16.to_le_bytes()); // disk start
            central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            central.extend_from_slice(&entry.external_attrs.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(&entry.name_raw);
        }

        let mut cd_offset = u32::try_from(out.len()).expect("テスト用の書庫は 4 GiB を超えない");
        let cd_size = u32::try_from(central.len()).expect("テスト用の書庫は 4 GiB を超えない");
        let mut count =
            u16::try_from(self.entries.len()).expect("テスト用の書庫は 64 Ki 件を超えない");
        let mut disk = 0u16;
        out.extend_from_slice(&central);

        match self.damage {
            Some(Damage::NoEocd) => return out,
            Some(Damage::Zip64Marker) => count = 0xFFFF,
            Some(Damage::MultiDisk) => disk = 1,
            Some(Damage::CentralOffsetOutOfRange) => cd_offset = 0xFFFF_FFF0,
            None => {}
        }

        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&disk.to_le_bytes()); // 中央ディレクトリが始まるディスク
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // 書庫コメント
        out
    }

    /// 組み上がったバイト列をファイルに置く。
    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, self.bytes())
    }
}

impl EntryBuilder {
    /// 「名前は UTF-8」の印（汎用目的ビット 11）の有無。印無し＝Shift_JIS の名前。
    pub fn utf8_flag(mut self, on: bool) -> Self {
        self.entry.utf8_flag = on;
        self
    }

    /// 中身を deflate（方式 8）で圧縮する。
    pub fn deflate(mut self) -> Self {
        self.entry.deflate = true;
        self
    }

    /// 外部属性にシンボリックリンクの種別（`S_IFLNK`）を置く。
    pub fn symlink(self) -> Self {
        self.external_attrs(ATTR_SYMLINK)
    }

    /// 外部属性を直に置く。
    pub fn external_attrs(mut self, attrs: u32) -> Self {
        self.entry.external_attrs = attrs;
        self
    }

    /// 暗号化の印（汎用目的ビット 0）を立てる。
    pub fn encrypted_flag(mut self) -> Self {
        self.entry.encrypted = true;
        self
    }

    /// 圧縮方式を直に置く（対応外の方式を書くとき。中身は無変換のまま）。
    pub fn method(mut self, method: u16) -> Self {
        self.entry.method = Some(method);
        self
    }

    /// 宣言する伸長後サイズを直に置く（実データは変えない）。宣言サイズの総和が
    /// 上限を超える入力を、巨大なデータを持たずに組むための口。
    pub fn declared_size(mut self, size: u64) -> Self {
        self.entry.declared_size = Some(size);
        self
    }

    /// このエントリを 1 点だけ壊す。
    pub fn corrupt(mut self, how: Corrupt) -> Self {
        self.entry.corrupt = Some(how);
        self
    }

    /// エントリを確定して書庫へ戻る。
    pub fn done(mut self) -> NarBuilder {
        self.builder.entries.push(self.entry);
        self.builder
    }
}

/// `install.txt` の中身を組む。行を CRLF で連結し、末尾にも CRLF を置く。
pub fn install_txt(lines: &[&str]) -> Vec<u8> {
    lines
        .iter()
        .flat_map(|line| [line.as_bytes(), b"\r\n"].concat())
        .collect()
}

/// フォルダの木を走査して 1 本の書庫に畳む（段 ③ の検体の畳み込みが使う入口）。
///
/// 区切りは `/` 固定・名前は印つきの UTF-8・**中身は無変換**（改行も文字コードも
/// 触らない）。フォルダも 1 件ずつエントリにする。走査の順は名前順に固定するので、
/// 同じ木からは必ず同じバイト列が出る。
///
/// 名前が UTF-8 で表せない場合は、黙って落とさずに [`std::io::ErrorKind::InvalidData`]
/// を返す。
pub fn fold_tree(root: &Path) -> std::io::Result<NarBuilder> {
    let mut builder = NarBuilder::new();
    fold_into(&mut builder, root, "")?;
    Ok(builder)
}

/// `dir` の直下を名前順に辿り、`prefix` を冠した名前で `builder` へ足す。
fn fold_into(builder: &mut NarBuilder, dir: &Path, prefix: &str) -> std::io::Result<()> {
    let mut children: Vec<_> = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    children.sort_by_key(|child| child.file_name());
    for child in children {
        let name = child.file_name().into_string().map_err(|raw| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("名前が UTF-8 で表せない: {raw:?}（{}）", dir.display()),
            )
        })?;
        let path = child.path();
        let joined = format!("{prefix}{name}");
        if child.file_type()?.is_dir() {
            *builder = std::mem::take(builder).dir(joined.clone());
            fold_into(builder, &path, &format!("{joined}/"))?;
        } else {
            let data = std::fs::read(&path)?;
            *builder = std::mem::take(builder).file(joined, &data).done();
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "nar_writer_tests.rs"]
mod tests;
