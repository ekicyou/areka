//! resolve — ツリー解決 + I/O（parser ファミリ内で唯一の I/O 保有点）。
//!
//! 展開済みゴーストパッケージのルートから `ghost/master/descript.txt` を起点に
//! SHIORI / shell の 2 点マウントを解決する。制御フローは design.md「System Flows」
//! の mermaid に忠実（パス合成 → 存在 → 読取 → decode → kv → names → shiori →
//! shelldir → shell 存在確認 → 構築）。charset 判定・KV 分割は foundation へ委譲し
//! 再実装しない（Req 1.5）。参照キーは name / sakura.name / kero.name / shiori /
//! seriko.defaultsurfacedirectoryname のみ（install.txt / balloon 系 / NAR には触れない）。

use std::io::ErrorKind;
use std::path::Path;

use crate::charset::{decode, DefaultEncoding};
use crate::kv::parse_kv;

use super::model::{
    BindGroupDefaults, GhostNames, MountError, MountModel, ShellMount, ShioriMount,
};

/// SHIORI マウント先（= 起点 descript.txt の親・Req 2.1）。
const GHOST_MASTER: &str = "ghost/master";
/// 起点定義ファイル名（Req 1.1）。
const DESCRIPT_FILE: &str = "descript.txt";
/// shell ルート（Req 3.1/3.2）。
const SHELL_ROOT: &str = "shell";
/// shell 既定ディレクトリ名（ukadoc 正典・Req 3.1）。
const DEFAULT_SHELL_DIR: &str = "master";

/// 展開済みゴーストパッケージのルートから、descript.txt 起点で
/// SHIORI/shell の 2 点マウントを解決する。
///
/// - `ghost_root`: 展開済みゴーストパッケージのルート（`ghost/` `shell/` を含む階層）。
/// - `default_encoding`: descript.txt に `charset` 宣言が無い場合に用いる既定エンコード。
///   本 module は既定をハードコードせず（固定 Utf8 はレガシー ANSI ゴーストを誤読する）、
///   呼び出し側が指定する（SSP 準拠の既定は ANSI）。非 UTF-8 拒否のエンフォースは
///   下流の SHIORI 層（設計ディスカッション #1）。
///
/// 成功時 `MountModel`、致命的欠落（起点不在・起点読取不能・shell dir 不在）時
/// `MountError` を返す。
pub fn resolve(
    ghost_root: &Path,
    default_encoding: DefaultEncoding,
) -> Result<MountModel, MountError> {
    // ghost/master（SHIORI マウント先）と起点 descript.txt のパスを合成。
    let shiori_dir = ghost_root.join(GHOST_MASTER);
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

    // charset 判定 + デコード + KV 分割は foundation へ委譲（Req 1.5）。
    let text = decode(&bytes, default_encoding);
    let map = parse_kv(&text);

    // 名前情報（欠落は None・推測しない・Req 1.4）。
    let names = GhostNames {
        name: map.get("name").cloned(),
        sakura_name: map.get("sakura.name").cloned(),
        kero_name: map.get("kero.name").cloned(),
    };

    // SHIORI マウント: dir は起点の親（存在確定）、file は未指定なら None（推測禁止・Req 2.3）。
    let shiori = ShioriMount {
        dir: shiori_dir,
        file: map.get("shiori").cloned(),
    };

    // shell マウント: 指定名 or 既定 master（Req 3.1/3.2）→ 物理存在確認（Req 3.3）。
    let shell_name = map
        .get("seriko.defaultsurfacedirectoryname")
        .map(String::as_str)
        .unwrap_or(DEFAULT_SHELL_DIR);
    let shell_dir = ghost_root.join(SHELL_ROOT).join(shell_name);
    if !shell_dir.is_dir() {
        return Err(MountError::ShellDirMissing { expected: shell_dir });
    }
    // shell descript.txt から bindgroup default を転記（Req 4.5）。bindgroup default
    // （`sakura.bindgroup*.default,数値`／`kero.*`）は ukadoc カテゴリ `descript_shell`
    // に属し、起点 ghost/master/descript.txt ではなく **shell の descript.txt** に定義
    // される。shell descript は存在確定していない（shell dir の存在のみ Req 3.3 で確定）
    // ため、読取不能・不在は致命ではなく空の bindgroup として扱う（既存 name 系経路や
    // マウント成立を壊さない・転記のみ・展開しない）。
    let bindgroups = read_bindgroup_defaults(&shell_dir, default_encoding);

    let shell = ShellMount { dir: shell_dir };

    Ok(MountModel {
        names,
        shiori,
        shell,
        bindgroups,
    })
}

/// bindgroup 番号を含むキーが `default,1` のときに番号を取り出す接頭辞集合。
const SAKURA_BINDGROUP_PREFIX: &str = "sakura.bindgroup";
const KERO_BINDGROUP_PREFIX: &str = "kero.bindgroup";
/// bindgroup 番号キーの接尾辞（`sakura.bindgroupNNNN.default` の末尾）。
const BINDGROUP_DEFAULT_SUFFIX: &str = ".default";
/// shell 定義ファイル名（ghost/master と同名だが所在が異なる）。
const SHELL_DESCRIPT_FILE: &str = "descript.txt";

/// 解決済み shell ディレクトリの descript.txt から bindgroup default を転記する。
///
/// `sakura.bindgroupNNNN.default` / `kero.bindgroupNNNN.default` のうち **値が `1`**
/// （= 起動時オン）のものについて、`NNNN` を u32 として本体／相方スコープ別に収集する。
/// **転記のみ・展開しない**（範囲展開・surface 解決はしない・parsers 転写層原則）。
/// 保持順は `parse_kv` の `BTreeMap` 反復順（キー昇順）で決定的だが、下流は集合として
/// 扱うため順序に依存しない。shell descript が不在・読取不能なら空を返す（致命でない）。
fn read_bindgroup_defaults(shell_dir: &Path, default_encoding: DefaultEncoding) -> BindGroupDefaults {
    let descript = shell_dir.join(SHELL_DESCRIPT_FILE);
    // shell descript は存在確定していない。読めなければ空（bindgroup 定義なしと同義）。
    let Ok(bytes) = std::fs::read(&descript) else {
        return BindGroupDefaults::default();
    };

    let text = decode(&bytes, default_encoding);
    let map = parse_kv(&text);

    let mut defaults = BindGroupDefaults::default();
    for (key, value) in &map {
        // 値が "1"（起動時オン）のもののみ転記。"0" や欠落は非オン。
        if value != "1" {
            continue;
        }
        if let Some(id) = parse_bindgroup_id(key, SAKURA_BINDGROUP_PREFIX) {
            defaults.sakura_default_on.push(id);
        } else if let Some(id) = parse_bindgroup_id(key, KERO_BINDGROUP_PREFIX) {
            defaults.kero_default_on.push(id);
        }
    }
    defaults
}

/// `<prefix>NNNN.default` 形のキーから bindgroup 番号 `NNNN`（u32）を取り出す。
/// 形が一致しない／番号が u32 でパースできないキーは `None`（転記対象外）。
fn parse_bindgroup_id(key: &str, prefix: &str) -> Option<u32> {
    let rest = key.strip_prefix(prefix)?;
    let number = rest.strip_suffix(BINDGROUP_DEFAULT_SUFFIX)?;
    number.parse::<u32>().ok()
}
