//! resolve_shell — 展開済みゴーストパッケージの 2 点マウントの解決。
//!
//! ゴーストのルートから `ghost/master/descript.txt` を起点に SHIORI / shell の
//! 2 点マウントを解決する（パス合成 → 読取 → decode → kv → names → shiori →
//! シェル名 → shell 存在確認 → bindgroup → 構築）。charset 判定・KV 分割は foundation へ
//! 委ね再実装しない（Req 1.5）。ゴーストの descript で読む鍵は名前（`name`・
//! `sakura.name`・`sakura.name2`・`kero.name`）・`shiori`・`shiori.encoding`・
//! `shiori.forceencoding`・`readme`・`seriko.defaultsurfacedirectoryname`。
//!
//! シェル名は呼び手が `Some(名)` で選ぶか、`None` で今日の規則
//! （`seriko.defaultsurfacedirectoryname`、無ければ `master`）に任せる。
//! SHIORI の置き場の綴りは [`shiori_dir`] の 1 か所だけが持つ。

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::charset::{DefaultEncoding, decode};
use crate::kv::parse_kv_lowercased;

use super::model::{GhostNames, MountError, MountModel, ShellMount, ShioriMount};
use super::resolve_bind::read_bindgroup_defaults;

/// 起点定義ファイル名（Req 1.1）。
const DESCRIPT_FILE: &str = "descript.txt";
/// shell ルート（Req 3.1/3.2）。
const SHELL_ROOT: &str = "shell";
/// shell 既定ディレクトリ名（ukadoc 正典・Req 3.1）。
const DEFAULT_SHELL_DIR: &str = "master";

/// SHIORI マウント先（= 起点 descript.txt の親・Req 2.1）。`ghost` と `master` を
/// ネイティブの区切りで継ぐ（`"ghost/master"` の 1 成分にしない・指摘 10）。
pub fn shiori_dir(ghost_root: &Path) -> PathBuf {
    ghost_root.join("ghost").join("master")
}

/// ゴーストの `ghost/master/descript.txt` だけを読み、名前情報を返す（シェルは見ない）。
///
/// 失敗は `resolve_with_shell` と同じ `StartPointMissing`（不在）・`StartPointUnreadable`
/// （その他の I/O 失敗）。`name` 等が無ければ該当欄は None（失敗にしない）。
pub fn read_ghost_names(
    ghost_root: &Path,
    default_encoding: DefaultEncoding,
) -> Result<GhostNames, MountError> {
    let map = read_ghost_descript(&shiori_dir(ghost_root), default_encoding)?;
    Ok(names_from(&map))
}

/// `shiori_dir/descript.txt` を読み、decode して KV に分ける。
fn read_ghost_descript(
    shiori_dir: &Path,
    default_encoding: DefaultEncoding,
) -> Result<BTreeMap<String, String>, MountError> {
    let descript = shiori_dir.join(DESCRIPT_FILE);

    // 起点 descript.txt を読む。不在は StartPointMissing、その他 I/O 失敗は
    // StartPointUnreadable（黙って空を返さない・Req 1.6/5.1）。
    let bytes = match std::fs::read(&descript) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(MountError::StartPointMissing { expected: descript });
        }
        Err(err) => {
            return Err(MountError::StartPointUnreadable {
                path: descript,
                kind: err.kind(),
            });
        }
    };

    // charset 判定 + デコード + KV 分割は foundation へ委譲（Req 1.5）。鍵は大文字と小文字を
    // 区別せず、空の値は「無し」と同じに扱う（指摘 11）。
    Ok(parse_kv_lowercased(&decode(&bytes, default_encoding)))
}

/// 成分がちょうど 1 つの普通のフォルダ名のときだけ返す（`..`・絶対パス・`.`・`a/b` は断る）。
fn folder_name(value: &str) -> Option<&OsStr> {
    let mut components = Path::new(value).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(name)), None) => Some(name),
        _ => None,
    }
}

/// 基のフォルダから外へ出うる相対パスか（`..`・根・ドライブ／UNC の頭を含む）。
fn points_outside(value: &str) -> bool {
    Path::new(value).components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

/// `seriko.defaultsurfacedirectoryname` の値を確かめて既定のシェル名にする。
///
/// 無いか空なら黙って `master`。空でなく 1 つのフォルダ名でない値は、`shell/` の外を
/// 指しうるので使わず、値とゴーストの根を `warn!` に残して `master`（指摘 1・2）。
fn default_shell_name<'a>(map: &'a BTreeMap<String, String>, ghost_root: &Path) -> &'a OsStr {
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#seriko.defaultsurfacedirectoryname_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1
    let Some(value) = map.get("seriko.defaultsurfacedirectoryname") else {
        return OsStr::new(DEFAULT_SHELL_DIR);
    };
    folder_name(value).unwrap_or_else(|| {
        tracing::warn!(
            value = %value,
            ghost_root = %ghost_root.display(),
            "既定のシェル名が 1 つのフォルダ名でない: master を使う"
        );
        OsStr::new(DEFAULT_SHELL_DIR)
    })
}

/// KV から名前情報を取る（欠落は None・推測しない・Req 1.4）。
fn names_from(map: &BTreeMap<String, String>) -> GhostNames {
    GhostNames {
        name: map.get("name").cloned(),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#sakura.name_2c_540d_524d:1
        sakura_name: map.get("sakura.name").cloned(),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#sakura.name2_2c_540d_524d:1
        sakura_name2: map.get("sakura.name2").cloned(),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#kero.name_2c_540d_524d:1
        kero_name: map.get("kero.name").cloned(),
    }
}

/// 展開済みゴーストパッケージのルートから、descript.txt 起点で
/// SHIORI/shell の 2 点マウントを解決する（シェル名つき）。
///
/// - `ghost_root`: 展開済みゴーストパッケージのルート（`ghost/` `shell/` を含む階層）。
/// - `default_encoding`: descript.txt に `charset` 宣言が無い場合に用いる既定エンコード。
///   本 module は既定をハードコードせず（固定 Utf8 はレガシー ANSI ゴーストを誤読する）、
///   呼び出し側が指定する（SSP 準拠の既定は ANSI）。非 UTF-8 拒否のエンフォースは
///   下流の SHIORI 層（設計ディスカッション #1）。
/// - `shell`: `Some(名)` なら `ghost_root/shell/<名>` を選ぶ。`None` なら今日の規則
///   （`seriko.defaultsurfacedirectoryname`、無ければ `master`）。名前の先が無ければ
///   既定のシェルへ黙って戻らず、今日と同じ `ShellDirMissing` を返す。
///
/// 成功時 `MountModel`、致命的欠落（起点不在・起点読取不能・shell dir 不在）時
/// `MountError` を返す。bindgroup は選んだシェルの `descript.txt` から読む。
pub fn resolve_with_shell(
    ghost_root: &Path,
    default_encoding: DefaultEncoding,
    shell: Option<&str>,
) -> Result<MountModel, MountError> {
    // ghost/master（SHIORI マウント先）と起点 descript.txt を読む（名前だけの読みと同じ部品）。
    let shiori_dir = shiori_dir(ghost_root);
    let map = read_ghost_descript(&shiori_dir, default_encoding)?;
    let names = names_from(&map);

    // SHIORI マウント: dir は起点の親（存在確定）、file は未指定なら None（推測禁止・Req 2.3）。
    let shiori = ShioriMount {
        dir: shiori_dir,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#shiori_2c_30d5_30a1_30a4_30eb_540d:1
        // 外を指しても記録だけで使う（ゴーストは元々好きな DLL を持ち込める・R2）。
        file: map
            .get("shiori")
            .inspect(|value| {
                if points_outside(value) {
                    tracing::warn!(
                        value = %value,
                        ghost_root = %ghost_root.display(),
                        "shiori の値が ghost/master の外を指す: そのまま使う"
                    );
                }
            })
            .cloned(),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#shiori.encoding_2c_6587_5b57_30b3_30fc_30c9:1
        encoding: map.get("shiori.encoding").cloned(),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#shiori.forceencoding_2c_6587_5b57_30b3_30fc_30c9:1
        force_encoding: map.get("shiori.forceencoding").cloned(),
    };

    // shell マウント: 呼び手の名 → 指定名 → 既定 master（Req 3.1/3.2）→ 物理存在確認（Req 3.3）。
    let shell_name = match shell {
        Some(name) => OsStr::new(name),
        None => default_shell_name(&map, ghost_root),
    };
    let shell_dir = ghost_root.join(SHELL_ROOT).join(shell_name);
    if !shell_dir.is_dir() {
        return Err(MountError::ShellDirMissing {
            expected: shell_dir,
        });
    }
    // shell descript.txt から bindgroup default を転記（bindopt 1.1/1.2）。bindgroup default
    // （`sakura.bindgroup*.default,数値`／`kero.*`）は ukadoc カテゴリ `descript_shell`
    // に属し、起点 ghost/master/descript.txt ではなく **選んだ shell の descript.txt** に
    // 定義される。shell descript は存在確定していない（shell dir の存在のみ Req 3.3 で確定）
    // ため、読取不能・不在は致命ではなく空の bindgroup として扱う（既存 name 系経路や
    // マウント成立を壊さない・転記のみ・展開しない）。
    let bindgroups = read_bindgroup_defaults(&shell_dir, default_encoding);

    let shell = ShellMount { dir: shell_dir };

    Ok(MountModel {
        names,
        shiori,
        shell,
        bindgroups,
        // 外を指す値は無いのと同じ（OS の関連付けでゴーストの外のファイルを開かせない・R2）。
        readme: map
            .get("readme")
            .filter(|value| {
                let outside = points_outside(value);
                if outside {
                    tracing::warn!(
                        value = %value,
                        ghost_root = %ghost_root.display(),
                        "readme の値がゴーストのフォルダの外を指す: 既定の名を使う"
                    );
                }
                !outside
            })
            .cloned(),
    })
}
