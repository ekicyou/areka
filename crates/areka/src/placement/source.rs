//! I/O 層: descript 供給源（task 2.2）。
//!
//! `areka_parsers::package::resolve` → ghost/shell descript.txt 読込
//! （`charset::decode` 既定 Ansi → `kv::parse_kv`）→ (ghost_kv, shell_kv, shell_dir, titles)。
//!
//! 依存規約（placement/mod.rs・design「placement::source」）: areka-parsers＋std
//! （＋tracing）のみを import する。wintf/bevy_ecs/emo 系・兄弟モジュール
//! （config/resolver/…）へは依存しない。
//!
//! エンコーディング既定は `DefaultEncoding::Ansi`（SSP 既定・記憶
//! areka-descript-encoding。emo2 は `charset,UTF-8` 宣言を持つため prescan の
//! 宣言優先で UTF-8 デコードされる＝既定 Ansi でも実測どおり読める・DD4）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_parsers::charset::{decode, DefaultEncoding};
use areka_parsers::kv::parse_kv;
use areka_parsers::package::{resolve, GhostNames};
use tracing::{error, warn};

use super::PlacementError;

/// descript 定義ファイル名（ghost/master と shell/<dir> の双方で同名）。
const DESCRIPT_FILE: &str = "descript.txt";

/// 窓タイトルの既定値（欠落スコープ・design「placement::source」）。
const DEFAULT_TITLE: &str = "areka";

/// scope n≥2 のタイトルキー接頭辞／接尾辞（ghost descript `char{n}.name`）。
const CHAR_PREFIX: &str = "char";
const NAME_SUFFIX: &str = ".name";

/// 窓タイトルの正本（Win32 識別／デバッグ観測用）。
///
/// scope0 = `sakura.name`・scope1 = `kero.name`（`MountModel.names` 由来・design
/// 記載どおり）・scope n≥2 = ghost descript KV の `char{n}.name`（あれば）。
/// `GhostNames` は `sakura_name`/`kero_name` のみを運ぶため、design の
/// 「`char{n}.name`（あれば）」は同じ起点 descript を decode した `ghost_kv` から
/// 補完する（正本は同一ファイル＝意味は等価・選択理由を本コメントに記録）。
/// 欠落スコープは既定 [`DEFAULT_TITLE`]（パニックしない・常に文字列を返す）。
#[allow(dead_code)] // scaffold（task 2.2）: spawn（task 5）が消費するまで非テストビルドでは未使用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostTitles {
    /// スコープ番号 → タイトル（非公開・アクセサ `title` 経由）。
    titles: BTreeMap<usize, String>,
}

#[allow(dead_code)] // scaffold（task 2.2）: spawn（task 5）が消費するまで非テストビルドでは未使用
impl GhostTitles {
    /// スコープの窓タイトルを返す。欠落時は既定 `"areka"`（panic しない）。
    pub fn title(&self, scope: usize) -> &str {
        self.titles
            .get(&scope)
            .map(String::as_str)
            .unwrap_or(DEFAULT_TITLE)
    }
}

#[cfg(test)]
impl GhostTitles {
    /// テスト専用コンストラクタ（`titles` は非公開フィールドのため、spawn（task 5.1）
    /// 等の headless テストが fixture I/O（`load_descript_source`）なしで
    /// `GhostTitles` を構築する唯一の経路。`#[cfg(test)]` 限定で本番公開面は不変）。
    pub(crate) fn from_scope_titles<I>(titles: I) -> Self
    where
        I: IntoIterator<Item = (usize, String)>,
    {
        Self {
            titles: titles.into_iter().collect(),
        }
    }
}

/// descript 供給源（ghost/shell の生 KV＋shell dir＋窓タイトル）。
///
/// `ghost_kv`/`shell_kv` は `kv::parse_kv` の出力形（`BTreeMap<String, String>`）
/// そのままで、`config::build_placement_config(&ghost_kv, &shell_kv)` へ直接
/// 供給できる（task 2.1 との結線契約）。
#[allow(dead_code)] // scaffold（task 2.2）: 後続タスク（3〜6）が消費するまで非テストビルドでは未使用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptSource {
    /// ghost/master/descript.txt の生 KV（読取失敗時は空・継続）。
    pub ghost_kv: BTreeMap<String, String>,
    /// shell/<dir>/descript.txt の生 KV（読取失敗は致命 `Err`）。
    pub shell_kv: BTreeMap<String, String>,
    /// 解決済み shell ディレクトリ（`MountModel.shell.dir`・measure が消費）。
    pub shell_dir: PathBuf,
    /// 窓タイトルの正本。
    pub titles: GhostTitles,
}

/// `ghost_root` から shell dir を解決し、ghost/shell descript.txt を charset
/// 対応（既定 Ansi）で読み `DescriptSource` を返す（design「placement::source」・DD4）。
///
/// 失敗契約（design「Error Handling」・log-first）:
/// - resolve 失敗 → `error!`＋`Err(PlacementError::Mount)`
/// - shell descript 読取失敗 → `error!`＋`Err(PlacementError::DescriptRead)`
/// - ghost descript 読取失敗 → `warn!`＋空 KV で継続（shell 側だけで emo2 は成立。
///   resolve 成功直後ゆえ通常は読めるが、TOCTOU への防御として寛容経路を維持する）
#[allow(dead_code)] // scaffold（task 2.2）: main.rs シーム（task 6）が結線するまで非テストビルドでは未使用
pub fn load_descript_source(ghost_root: &Path) -> Result<DescriptSource, PlacementError> {
    // マウント解決（shell dir・ghost/master dir・names の正本）。
    let model = resolve(ghost_root, DefaultEncoding::Ansi).map_err(|e| {
        error!(ghost_root = %ghost_root.display(), error = ?e, "ゴーストのマウント解決に失敗");
        PlacementError::Mount(e)
    })?;

    // ghost descript: 読取失敗は警告＋空 KV で継続（寛容経路は read_kv_lenient に局在）。
    let ghost_descript = model.shiori.dir.join(DESCRIPT_FILE);
    let ghost_kv = read_kv_lenient(&ghost_descript);

    // shell descript: 読取失敗は致命（→シームがフォールバック・DD14）。
    let shell_descript = model.shell.dir.join(DESCRIPT_FILE);
    let shell_kv = match std::fs::read(&shell_descript) {
        Ok(bytes) => parse_kv(&decode(&bytes, DefaultEncoding::Ansi)),
        Err(source) => {
            error!(path = %shell_descript.display(), error = %source, "shell descript の読み取りに失敗");
            return Err(PlacementError::DescriptRead {
                path: shell_descript,
                source,
            });
        }
    };

    let titles = build_titles(&model.names, &ghost_kv);

    Ok(DescriptSource {
        ghost_kv,
        shell_kv,
        shell_dir: model.shell.dir,
        titles,
    })
}

/// descript.txt の寛容読取: 読めなければ `warn!`＋空 KV（ghost 側の継続契約）。
/// 読めれば `charset::decode`（既定 Ansi・宣言優先）→ `kv::parse_kv`。
fn read_kv_lenient(path: &Path) -> BTreeMap<String, String> {
    match std::fs::read(path) {
        Ok(bytes) => parse_kv(&decode(&bytes, DefaultEncoding::Ansi)),
        Err(err) => {
            warn!(path = %path.display(), error = %err, "ghost descript の読み取りに失敗（空 KV で継続）");
            BTreeMap::new()
        }
    }
}

/// `MountModel.names`（scope0/1 の正本）＋ghost descript KV（`char{n}.name`・n≥2）
/// から `GhostTitles` を構築する。
fn build_titles(names: &GhostNames, ghost_kv: &BTreeMap<String, String>) -> GhostTitles {
    let mut titles = BTreeMap::new();
    if let Some(name) = &names.sakura_name {
        titles.insert(0usize, name.clone());
    }
    if let Some(name) = &names.kero_name {
        titles.insert(1usize, name.clone());
    }
    for (key, value) in ghost_kv {
        if let Some(n) = char_name_scope_of(key) {
            // scope0/1 の正本は sakura.name/kero.name（char0/char1 別名はタイトルへ
            // 写像しない・config の別名寛容はキー値の受理のみでタイトル正本は names）。
            if n >= 2 {
                titles.insert(n, value.clone());
            }
        }
    }
    GhostTitles { titles }
}

/// `char{n}.name` 形のキーからスコープ番号を抽出する。`charset` 等の非該当キー・
/// 数値化不能（空・巨大値含む）は None（panic しない）。
fn char_name_scope_of(key: &str) -> Option<usize> {
    let digits = key.strip_prefix(CHAR_PREFIX)?.strip_suffix(NAME_SUFFIX)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::placement::PlacementError;

    /// emo2 実フィクスチャのルート（`crates/pilot/examples/shiori-host-32/fixtures/emo2/`）。
    ///
    /// `CARGO_MANIFEST_DIR`（= `.../crates/areka`）相対で組み立てる
    /// （areka-parsers validation_tests と同じ規約・絶対パス埋め込みやフィクスチャ複製をしない）。
    fn emo2_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("pilot")
            .join("examples")
            .join("shiori-host-32")
            .join("fixtures")
            .join("emo2")
    }

    /// このテスト専用の一意な一時ディレクトリを返す（areka-parsers resolve_tests と
    /// 同じ規約: 外部クレート tempfile に依存せず `std::env::temp_dir()` 直下へ
    /// テスト名タグでユニーク化）。
    fn unique_temp_dir(tag: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_placement_source_tests_{tag}"));
        dir
    }

    // ------------------------------------------------------------------
    // T-I5: emo2 fixture からの end-to-end 読込（統合）
    // ------------------------------------------------------------------

    /// T-I5: emo2 fixture から `load_descript_source` を呼び、`shell_kv` に
    /// 実測キー（`seriko.alignmenttodesktop=bottom` 等・正典表検証行）が含まれる。
    #[test]
    fn t_i5_emo2_fixture_loads_descript_source() {
        let src = load_descript_source(&emo2_root())
            .expect("emo2 fixture は Ok(DescriptSource) を返す");

        // shell descript 実測キー（正典表「emo2 実測値による検証行」）
        assert_eq!(
            src.shell_kv.get("seriko.alignmenttodesktop").map(String::as_str),
            Some("bottom")
        );
        assert_eq!(src.shell_kv.get("sakura.defaultx").map(String::as_str), Some("0"));
        assert_eq!(src.shell_kv.get("kero.defaultx").map(String::as_str), Some("0"));
        assert_eq!(
            src.shell_kv.get("sakura.balloon.alignment").map(String::as_str),
            Some("left")
        );
        assert_eq!(
            src.shell_kv.get("kero.balloon.alignment").map(String::as_str),
            Some("right")
        );

        // ghost descript 実測キー（scope1 検出シグナル kero.name を含む）
        assert_eq!(src.ghost_kv.get("kero.name").map(String::as_str), Some("エモ"));
        assert_eq!(src.ghost_kv.get("sakura.name").map(String::as_str), Some("むらさき"));

        // shell_dir は resolve の解決値（<root>/shell/master）
        assert_eq!(src.shell_dir, emo2_root().join("shell").join("master"));
    }

    /// T-I5 補: emo2 の `GhostTitles` は `sakura.name`/`kero.name` を写像し、
    /// 未定義スコープは既定 `"areka"` を返す（パニックしない）。
    #[test]
    fn t_i5_emo2_titles_from_names() {
        let src = load_descript_source(&emo2_root())
            .expect("emo2 fixture は Ok(DescriptSource) を返す");

        assert_eq!(src.titles.title(0), "むらさき");
        assert_eq!(src.titles.title(1), "エモ");
        // emo2 に char2 以降は無い → 既定
        assert_eq!(src.titles.title(2), "areka");
        assert_eq!(src.titles.title(99), "areka");
    }

    // ------------------------------------------------------------------
    // GhostTitles 単体（既定値・char{n}.name 由来）
    // ------------------------------------------------------------------

    /// 名前情報が全欠落でも全スコープで既定 `"areka"` を返す（常に文字列・panic しない）。
    #[test]
    fn titles_all_missing_default_to_areka() {
        let titles = build_titles(&areka_parsers::package::GhostNames::default(), &BTreeMap::new());
        assert_eq!(titles.title(0), "areka");
        assert_eq!(titles.title(1), "areka");
        assert_eq!(titles.title(7), "areka");
    }

    /// scope n≥2 のタイトルは ghost descript KV の `char{n}.name` から拾う。
    /// `charset` 等の非該当キー・`char0.name`/`char1.name`（scope0/1 の正本は
    /// `sakura.name`/`kero.name`）は写像しない。
    #[test]
    fn titles_char_n_name_from_ghost_kv() {
        let names = areka_parsers::package::GhostNames::default();
        let ghost_kv: BTreeMap<String, String> = [
            ("char2.name", "三人目"),
            ("char10.name", "十人目"),
            ("char0.name", "偽さくら"),
            ("char1.name", "偽けろ"),
            ("charset", "UTF-8"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let titles = build_titles(&names, &ghost_kv);
        assert_eq!(titles.title(2), "三人目");
        assert_eq!(titles.title(10), "十人目");
        // scope0/1 は names 由来のみ（欠落 → 既定）
        assert_eq!(titles.title(0), "areka");
        assert_eq!(titles.title(1), "areka");
    }

    // ------------------------------------------------------------------
    // 失敗経路（決定論化可能なもの）
    // ------------------------------------------------------------------

    /// resolve 失敗（ghost_root 不在）は `Err(PlacementError::Mount)`。
    #[test]
    fn load_missing_root_returns_mount_err() {
        let root = unique_temp_dir("missing_root_returns_mount_err").join("no_such_ghost");
        let err = load_descript_source(&root).expect_err("不在 root は Err");
        assert!(
            matches!(err, PlacementError::Mount(_)),
            "Mount variant 以外が返った: {err:?}"
        );
    }

    /// shell descript 読取失敗（shell dir は実在・descript.txt 不在）は
    /// `Err(PlacementError::DescriptRead)` で、path が shell 側 descript を指す。
    #[test]
    fn load_missing_shell_descript_returns_descript_read_err() {
        let root = unique_temp_dir("missing_shell_descript");
        let _ = fs::remove_dir_all(&root);
        let ghost_master = root.join("ghost").join("master");
        fs::create_dir_all(&ghost_master).expect("create ghost/master");
        fs::write(
            ghost_master.join("descript.txt"),
            "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
        )
        .expect("write ghost descript");
        // shell/master は dir のみ実在（descript.txt なし）→ resolve は成功する
        let shell_dir = root.join("shell").join("master");
        fs::create_dir_all(&shell_dir).expect("create shell/master");

        let err = load_descript_source(&root).expect_err("shell descript 不在は Err");
        match err {
            PlacementError::DescriptRead { path, .. } => {
                assert_eq!(path, shell_dir.join("descript.txt"));
            }
            other => panic!("DescriptRead variant 以外が返った: {other:?}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    /// ghost descript の寛容読取ヘルパ: 読めなければ警告＋空 KV（継続契約の檻）。
    #[test]
    fn read_kv_lenient_missing_returns_empty() {
        let path = unique_temp_dir("read_kv_lenient_missing").join("descript.txt");
        let kv = read_kv_lenient(&path);
        assert!(kv.is_empty());
    }

    /// 寛容読取ヘルパは読めれば通常どおり decode→parse_kv する（Ansi 既定・宣言優先）。
    #[test]
    fn read_kv_lenient_reads_utf8_declared_file() {
        let dir = unique_temp_dir("read_kv_lenient_reads");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("descript.txt");
        fs::write(&path, "charset,UTF-8\nname,えも？？\n".as_bytes()).expect("write");

        let kv = read_kv_lenient(&path);
        assert_eq!(kv.get("name").map(String::as_str), Some("えも？？"));

        let _ = fs::remove_dir_all(&dir);
    }
}
