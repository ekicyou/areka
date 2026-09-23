//! ベースウェアの根の目録（design.md「areka-ghost: catalog」）。
//!
//! 根の直下 `ghost/`・`balloon/` と、ゴーストの `shell/` を**直下 1 段だけ**走査し、
//! ゴースト・シェル・バルーンを素性（要件 2.4 の 7 項目）付きで返す。同梱バルーン名
//! （`install.txt` の `balloon.directory`）と「そのフォルダはゴーストか」の判定も持つ。
//!
//! - descript は `charset::decode`（既定 `Ansi`）＋`kv::parse_kv` で読み、鍵は ASCII
//!   小文字化して引く。`menu`／`type` の**値**は trim＋ASCII 小文字化して比べる（R5）。
//!   素性の値は無加工。
//! - 1 体のゴーストの解決（`package::resolve`）とは別の読み手で、それを呼ばない（要件 2.10）。
//! - 記憶・解決順・既定の定数は知らない（`areka-sylphya`・`areka-nar` に依存しない）。
//! - 失敗は縮退（`warn!`＋除外）で、列挙は `Result` を返さない。格納フォルダが無いのは 0 件。
//! - 並びはフォルダ名のバイト順（`String::cmp`）。`recommended.*` は使わない（要件 2.5）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::kv::parse_kv;

/// descript のファイル名（ゴーストは `ghost/master/` の下・シェルとバルーンは最上位）。
const DESCRIPT_FILE: &str = "descript.txt";
/// 説明書の既定のファイル名（descript に `readme` 鍵が無いとき）。
const DEFAULT_README: &str = "readme.txt";
/// サムネイルのファイル名（有無だけを見る）。
const THUMBNAIL_FILE: &str = "thumbnail.png";

/// ベースウェアの根（直下に ghost/ と balloon/）。値型・パスの解釈だけを持つ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasewareRoot {
    dir: PathBuf,
}

impl BasewareRoot {
    /// 実在検査はしない（検査は bin の `resolve_root_from`）。
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `<根>/ghost`
    pub fn ghost_store(&self) -> PathBuf {
        self.dir.join("ghost")
    }

    /// `<根>/balloon`
    pub fn balloon_store(&self) -> PathBuf {
        self.dir.join("balloon")
    }

    /// `<根>/ghost/<folder>`
    pub fn ghost_dir(&self, folder: &str) -> PathBuf {
        self.ghost_store().join(folder)
    }

    /// `<根>/balloon/<folder>`
    pub fn balloon_dir(&self, folder: &str) -> PathBuf {
        self.balloon_store().join(folder)
    }
}

/// 素性（要件 2.4 の 7 項目）。「無し」は None／false。7 項目以外は足さない（要件 2.9）。
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Identity {
    /// フォルダ名（UTF-8。非 UTF-8 名は `warn!` で除外＝R6）。
    pub folder: String,
    pub name: Option<String>,
    pub craftman: Option<String>,
    pub craftmanw: Option<String>,
    pub id: Option<String>,
    /// descript の `readme`（無ければ `readme.txt`）がフォルダ最上位に実在すればそのパス。
    pub readme: Option<PathBuf>,
    /// `<フォルダ>/thumbnail.png` の有無。
    pub has_thumbnail: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GhostEntry {
    pub dir: PathBuf,
    pub identity: Identity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellEntry {
    pub dir: PathBuf,
    pub identity: Identity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalloonEntry {
    pub dir: PathBuf,
    pub identity: Identity,
}

/// `<根>/ghost/` の直下で `ghost/master/descript.txt` を持つフォルダ。素性は
/// `ghost/master/descript.txt` から、readme と thumbnail.png はゴーストのフォルダ最上位で見る。
/// フォルダ名の昇順。
pub fn list_ghosts(root: &BasewareRoot) -> Vec<GhostEntry> {
    subdirs(&root.ghost_store())
        .into_iter()
        .filter_map(|(folder, dir)| {
            let descript = read_descript(&master_descript(&dir))?;
            let identity = identity(folder, &dir, &descript);
            Some(GhostEntry { dir, identity })
        })
        .collect()
}

/// `<ゴースト>/shell/` の直下で descript.txt を持ち、`menu` が `hidden` でないフォルダ。
pub fn list_shells(ghost_dir: &Path) -> Vec<ShellEntry> {
    subdirs(&ghost_dir.join("shell"))
        .into_iter()
        .filter_map(|(folder, dir)| {
            let descript = read_descript(&dir.join(DESCRIPT_FILE))?;
            if folded(&descript, "menu").as_deref() == Some("hidden") {
                return None;
            }
            let identity = identity(folder, &dir, &descript);
            Some(ShellEntry { dir, identity })
        })
        .collect()
}

/// `<根>/balloon/` の直下で descript.txt を持ち、`type` が無いか `balloon` のフォルダ。
/// `type` が他の値なら `warn!`＋除外（要件 2.3）。
pub fn list_balloons(root: &BasewareRoot) -> Vec<BalloonEntry> {
    subdirs(&root.balloon_store())
        .into_iter()
        .filter_map(|(folder, dir)| {
            let descript = read_descript(&dir.join(DESCRIPT_FILE))?;
            if let Some(r#type) = folded(&descript, "type").filter(|t| t != "balloon") {
                tracing::warn!(
                    event = "catalog_type_not_balloon",
                    path = %dir.display(),
                    r#type = %r#type,
                    "type が balloon でないフォルダはバルーンの列挙から除く"
                );
                return None;
            }
            let identity = identity(folder, &dir, &descript);
            Some(BalloonEntry { dir, identity })
        })
        .collect()
}

/// `<ゴースト>/install.txt` の `balloon.directory`（鍵は小文字化・値は trim）。
/// 無い／読めない → None（読めないときは `warn!`）。番号付きの鍵は読まない。
pub fn companion_balloon(ghost_dir: &Path) -> Option<String> {
    let path = ghost_dir.join("install.txt");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            tracing::warn!(
                event = "catalog_install_unreadable",
                path = %path.display(),
                error = %err,
                "install.txt が読めない——同梱バルーンは無しとして扱う"
            );
            return None;
        }
    };
    lowercased(&bytes)
        .remove("balloon.directory")
        .filter(|v| !v.is_empty())
}

/// `<dir>/ghost/master/descript.txt` が実在するか（argv のゴーストの検査＝要件 4.8）。
pub fn is_ghost_dir(dir: &Path) -> bool {
    master_descript(dir).is_file()
}

/// `<ゴースト>/ghost/master/descript.txt`
fn master_descript(ghost_dir: &Path) -> PathBuf {
    ghost_dir.join("ghost").join("master").join(DESCRIPT_FILE)
}

/// 格納フォルダの直下 1 段のフォルダを（フォルダ名, パス）でバイト順に返す。
/// 格納フォルダが無ければ 0 件。非 UTF-8 のフォルダ名は `warn!`＋除外（R6）。
fn subdirs(store: &Path) -> Vec<(String, PathBuf)> {
    let entries = match std::fs::read_dir(store) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            tracing::warn!(
                path = %store.display(),
                error = %err,
                "格納フォルダが読めない——0 件として扱う"
            );
            return Vec::new();
        }
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(err) => {
                tracing::warn!(
                    path = %store.display(),
                    error = %err,
                    "格納フォルダの項目が読めない——その項目を除く"
                );
                None
            }
        })
        .filter(|path| path.is_dir())
        .filter_map(
            |path| match path.file_name().map(|n| n.to_owned().into_string()) {
                Some(Ok(folder)) => Some((folder, path)),
                _ => {
                    tracing::warn!(
                        event = "catalog_non_utf8_name",
                        path = %path.display(),
                        "フォルダ名が UTF-8 でない——列挙から除く"
                    );
                    None
                }
            },
        )
        .collect();
    dirs.sort();
    dirs
}

/// descript を読み、鍵を ASCII 小文字化した表を返す。無ければ黙って None（その項目ではない）、
/// 読めなければ `warn!`＋None（要件 2.7）。
fn read_descript(path: &Path) -> Option<BTreeMap<String, String>> {
    if !path.is_file() {
        return None;
    }
    match std::fs::read(path) {
        Ok(bytes) => Some(lowercased(&bytes)),
        Err(err) => {
            tracing::warn!(
                event = "catalog_descript_unreadable",
                path = %path.display(),
                error = %err,
                "descript.txt が読めない——その項目を列挙から除く"
            );
            None
        }
    }
}

/// 既存の charset 復号（要件 2.8）＋`parse_kv` の上で鍵を ASCII 小文字化する（R5）。
fn lowercased(bytes: &[u8]) -> BTreeMap<String, String> {
    parse_kv(&decode(bytes, DefaultEncoding::Ansi))
        .into_iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v))
        .collect()
}

/// 比べるための値（trim＋ASCII 小文字化・R5）。
fn folded(descript: &BTreeMap<String, String>, key: &str) -> Option<String> {
    descript.get(key).map(|v| v.trim().to_ascii_lowercase())
}

/// 素性 7 項目を組む。readme と thumbnail.png は `top`（フォルダ最上位）で見る。
fn identity(folder: String, top: &Path, descript: &BTreeMap<String, String>) -> Identity {
    let get = |key: &str| descript.get(key).cloned();
    let readme_name = descript
        .get("readme")
        .map(String::as_str)
        .filter(|v| !v.is_empty())
        .unwrap_or(DEFAULT_README);
    // 最上位のファイル名だけを受ける（`sub/x.txt`・`..` は最上位でないので無し）。
    let readme = (Path::new(readme_name).file_name() == Some(readme_name.as_ref()))
        .then(|| top.join(readme_name))
        .filter(|path| path.is_file());
    Identity {
        folder,
        name: get("name"),
        craftman: get("craftman"),
        craftmanw: get("craftmanw"),
        id: get("id"),
        readme,
        has_thumbnail: top.join(THUMBNAIL_FILE).is_file(),
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
