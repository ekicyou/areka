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
use tracing::{debug, error, warn};

use super::PlacementError;

/// descript 定義ファイル名（ghost/master と shell/<dir> の双方で同名）。
const DESCRIPT_FILE: &str = "descript.txt";

/// 窓タイトルの既定値（欠落スコープ・design「placement::source」）。
const DEFAULT_TITLE: &str = "areka";

/// scope n≥2 のタイトルキー接頭辞／接尾辞（ghost descript `char{n}.name`）。
const CHAR_PREFIX: &str = "char";
const NAME_SUFFIX: &str = ".name";

/// 作者基準 DPI の既定値（ukadoc 正典・areka-P0-emo-dpi-scaling design D1）。
///
/// ukadoc「seriko.dpi,推奨DPI」／「dpi,推奨DPI」がともに「何も指定しなければ Windows 標準の
/// 96 固定」と定める（SSP 2.7.21+）。無宣言・不正・0 はすべてこの値へ縮退する。
///
/// `areka_emo_present::scale::DEFAULT_AUTHOR_DPI` と同値だが、本モジュールの依存規約
/// （冒頭 doc: areka-parsers＋std＋tracing のみ・emo 系へは依存しない）を守るため
/// 定数のためだけの依存辺は張らず、ここへ局所定義する。
const DEFAULT_AUTHOR_DPI: u16 = 96;

/// shell descript の作者基準 DPI キー（ukadoc `seriko.dpi`・SSP 2.7.21+）。
const SHELL_DPI_KEY: &str = "seriko.dpi";

/// balloon descript の作者基準 DPI キー（ukadoc `dpi`・SSP 2.7.21+）。
const BALLOON_DPI_KEY: &str = "dpi";

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

#[allow(dead_code)] // scaffold（areka-P0-emo-dpi-scaling task 2.1）: main.rs/measure 結線（task 4）まで非テストビルドでは未使用
impl DescriptSource {
    /// shell descript の作者基準 DPI（ukadoc `seriko.dpi`・SSP 2.7.21+・design D1）。
    ///
    /// 既に読み込み済みの生 KV（[`DescriptSource::shell_kv`]）から読むだけで、
    /// パーサ改造も再 I/O も行わない（D1「既存生 KV から読む」）。
    /// 無宣言＝96（`debug!`）・不正／0＝96（`warn!`）——[`parse_author_dpi`] の縮退梯子に従う。
    /// panic しない（常に有効な非ゼロ DPI を返す）。
    pub fn shell_author_dpi(&self) -> u16 {
        parse_author_dpi(
            self.shell_kv.get(SHELL_DPI_KEY).map(String::as_str),
            SHELL_DPI_KEY,
        )
    }
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
///
/// 本ヘルパは ghost 専用ではなく [`load_balloon_author_dpi`] も通る**共有の読取器**ゆえ、
/// 失敗ログの文言は**帰属中立**にする（実帰属は `path` フィールドが運ぶ）。
/// ghost 固定文言にすると、バルーン descript 不在時の warn が ghost 起因に見え、
/// R6.3 の `RUST_LOG` grep 判定を誤らせる。
fn read_kv_lenient(path: &Path) -> BTreeMap<String, String> {
    match std::fs::read(path) {
        Ok(bytes) => parse_kv(&decode(&bytes, DefaultEncoding::Ansi)),
        Err(err) => {
            warn!(path = %path.display(), error = %err, "descript の読み取りに失敗（空 KV で継続）");
            BTreeMap::new()
        }
    }
}

/// balloon descript.txt（`balloon_root/descript.txt`）の作者基準 DPI
/// （ukadoc `dpi`・SSP 2.7.21+・design D1）。
///
/// balloon 側は shell と別パッケージ（`DescriptSource` の対象外）ゆえ、ここで
/// 寛容読取（[`read_kv_lenient`]・失敗は `warn!`＋空 KV）してから同じ縮退梯子
/// [`parse_author_dpi`] を通す。**読取器は 1 本のまま**（第 2 のリーダを発明しない）で、
/// balloon か shell かの帰属はログの `source` フィールドで区別できる。
///
/// 縮退（design「Error Handling」・すべて観測可能・panic しない）:
/// - ファイル不在・読取失敗 → `warn!`（[`read_kv_lenient`]・パス付き）＋無宣言扱い＝96
/// - 無宣言 → `debug!`＋96 ／ 不正・0 → `warn!`＋96
#[allow(dead_code)] // scaffold（areka-P0-emo-dpi-scaling task 2.1）: main.rs 結線（task 4）まで非テストビルドでは未使用
pub fn load_balloon_author_dpi(balloon_root: &Path) -> u16 {
    let path = balloon_root.join(DESCRIPT_FILE);
    let kv = read_kv_lenient(&path);
    parse_author_dpi(kv.get(BALLOON_DPI_KEY).map(String::as_str), BALLOON_DPI_KEY)
}

/// 作者基準 DPI の生値を解釈する単一権威（design D1・「Error Handling」の
/// 「author_dpi 不正・0・無宣言」行）。`source` は帰属識別用のキー名（ログ専用）。
///
/// 縮退梯子（無言経路なし・log-first）:
/// - `None`（無宣言） → `debug!`＋[`DEFAULT_AUTHOR_DPI`]（正典の既定＝異常ではない）
/// - 数値化不能（非数字・負値・u16 溢れ・空文字） → `warn!`＋[`DEFAULT_AUTHOR_DPI`]
/// - `0`（k の分母に使えない） → `warn!`＋[`DEFAULT_AUTHOR_DPI`]
/// - それ以外 → 宣言値そのまま（ukadoc の対照表 96/120/144/168/192 に限定せず受理する。
///   正典は「推奨 DPI 値」であって列挙ではない）
///
/// 返り値は常に非ゼロ（下流 `ScaleRatio` の分母としてそのまま使える）。panic しない。
fn parse_author_dpi(raw: Option<&str>, source: &str) -> u16 {
    let Some(raw) = raw else {
        debug!(
            source = %source,
            default_dpi = DEFAULT_AUTHOR_DPI,
            "作者基準 DPI の宣言なし: 正典の既定値を採用"
        );
        return DEFAULT_AUTHOR_DPI;
    };

    match raw.parse::<u16>() {
        Ok(0) => {
            warn!(
                source = %source,
                raw = %raw,
                default_dpi = DEFAULT_AUTHOR_DPI,
                "作者基準 DPI が 0（表示スケールの分母に使えない）: 既定値へ縮退"
            );
            DEFAULT_AUTHOR_DPI
        }
        Ok(dpi) => dpi,
        Err(err) => {
            warn!(
                source = %source,
                raw = %raw,
                error = %err,
                default_dpi = DEFAULT_AUTHOR_DPI,
                "作者基準 DPI を数値として解釈できない: 既定値へ縮退"
            );
            DEFAULT_AUTHOR_DPI
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
    use crate::placement::test_support::{capture_logs, expect_one};

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

    // ------------------------------------------------------------------
    // author_dpi 読取（areka-P0-emo-dpi-scaling task 2.1・要件 1.1・design D1）
    // 無宣言=96 / 宣言あり=その値 / 不正=warn+96 / 0=warn+96 の全パターン
    // ------------------------------------------------------------------

    /// shell_kv だけを差し替えた `DescriptSource`（author_dpi 読取の純関数檻用・I/O なし）。
    fn shell_source_with(kv: &[(&str, &str)]) -> DescriptSource {
        DescriptSource {
            ghost_kv: BTreeMap::new(),
            shell_kv: kv
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            shell_dir: PathBuf::from("shell"),
            titles: GhostTitles::from_scope_titles([]),
        }
    }

    /// balloon descript.txt を持つ一時 balloon ルートを作る（内容は呼び手指定）。
    fn balloon_root_with(tag: &str, descript: &str) -> PathBuf {
        let dir = unique_temp_dir(tag);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create balloon root");
        fs::write(dir.join("descript.txt"), descript.as_bytes()).expect("write balloon descript");
        dir
    }

    /// 無宣言（キー不在）は既定 96（ukadoc: 何も指定しなければ Windows 標準の 96 固定）。
    #[test]
    fn parse_author_dpi_absent_is_default_96() {
        assert_eq!(parse_author_dpi(None, "test"), 96);
    }

    /// 宣言ありは宣言値そのまま（ukadoc 対照表 96/120/144/168/192）。
    #[test]
    fn parse_author_dpi_declared_values_pass_through() {
        for raw in ["96", "120", "144", "168", "192"] {
            let expected: u16 = raw.parse().expect("テスト入力は u16");
            assert_eq!(parse_author_dpi(Some(raw), "test"), expected, "raw={raw}");
        }
        // 対照表外の任意値も切り捨てない（正典は「推奨 DPI 値」であり列挙ではない）
        assert_eq!(parse_author_dpi(Some("110"), "test"), 110);
    }

    /// 不正（数値化不能・負・u16 溢れ・空）は既定 96 へ縮退（warn・panic しない）。
    #[test]
    fn parse_author_dpi_invalid_is_default_96() {
        for raw in [
            "abc", "1x2", "", " ", "-96", "999999", "96.0", "0x60", "96 120",
        ] {
            assert_eq!(parse_author_dpi(Some(raw), "test"), 96, "raw={raw:?}");
        }
    }

    /// 0 は k の分母に使えない → 既定 96 へ縮退（warn）。
    #[test]
    fn parse_author_dpi_zero_is_default_96() {
        assert_eq!(parse_author_dpi(Some("0"), "test"), 96);
        assert_eq!(parse_author_dpi(Some("00"), "test"), 96);
    }

    /// `shell_author_dpi` は shell descript の `seriko.dpi` を読む（無宣言/宣言/不正/0）。
    #[test]
    fn shell_author_dpi_reads_seriko_dpi_all_patterns() {
        assert_eq!(shell_source_with(&[]).shell_author_dpi(), 96);
        assert_eq!(
            shell_source_with(&[("seriko.dpi", "120")]).shell_author_dpi(),
            120
        );
        assert_eq!(
            shell_source_with(&[("seriko.dpi", "abc")]).shell_author_dpi(),
            96
        );
        assert_eq!(
            shell_source_with(&[("seriko.dpi", "0")]).shell_author_dpi(),
            96
        );
        // balloon 側キー `dpi` は shell 側の正本ではない（キー取り違えの檻）
        assert_eq!(shell_source_with(&[("dpi", "192")]).shell_author_dpi(), 96);
    }

    /// emo2 実フィクスチャの shell descript は `seriko.dpi` 無宣言 → 96（既存期待値不変）。
    #[test]
    fn shell_author_dpi_emo2_fixture_is_default_96() {
        let src =
            load_descript_source(&emo2_root()).expect("emo2 fixture は Ok(DescriptSource) を返す");
        assert_eq!(src.shell_author_dpi(), 96);
    }

    /// balloon descript 不在（ファイル不在）は既定 96（lenient・panic しない）。
    #[test]
    fn load_balloon_author_dpi_missing_file_is_default_96() {
        let root = unique_temp_dir("balloon_dpi_missing_file").join("no_such_balloon");
        assert_eq!(load_balloon_author_dpi(&root), 96);
    }

    /// balloon descript の `dpi` を読む（無宣言/宣言/不正/0）。
    #[test]
    fn load_balloon_author_dpi_reads_dpi_all_patterns() {
        let absent = balloon_root_with("balloon_dpi_absent", "charset,UTF-8\nname,かくかく\n");
        assert_eq!(load_balloon_author_dpi(&absent), 96);
        let _ = fs::remove_dir_all(&absent);

        let declared = balloon_root_with("balloon_dpi_declared", "charset,UTF-8\ndpi,144\n");
        assert_eq!(load_balloon_author_dpi(&declared), 144);
        let _ = fs::remove_dir_all(&declared);

        let invalid = balloon_root_with("balloon_dpi_invalid", "charset,UTF-8\ndpi,abc\n");
        assert_eq!(load_balloon_author_dpi(&invalid), 96);
        let _ = fs::remove_dir_all(&invalid);

        let zero = balloon_root_with("balloon_dpi_zero", "charset,UTF-8\ndpi,0\n");
        assert_eq!(load_balloon_author_dpi(&zero), 96);
        let _ = fs::remove_dir_all(&zero);

        // shell 側キー `seriko.dpi` は balloon 側の正本ではない（キー取り違えの檻）
        let wrong_key =
            balloon_root_with("balloon_dpi_wrong_key", "charset,UTF-8\nseriko.dpi,192\n");
        assert_eq!(load_balloon_author_dpi(&wrong_key), 96);
        let _ = fs::remove_dir_all(&wrong_key);
    }

    /// emo2 実フィクスチャの balloon（emo2-kakukaku）は `dpi` 無宣言 → 96（既存期待値不変）。
    #[test]
    fn load_balloon_author_dpi_emo2_fixture_is_default_96() {
        let balloon_root = emo2_root().join("emo2-kakukaku");
        assert!(
            balloon_root.join("descript.txt").is_file(),
            "emo2 balloon fixture が見つからない: {}",
            balloon_root.display()
        );
        assert_eq!(load_balloon_author_dpi(&balloon_root), 96);
    }

    // ------------------------------------------------------------------
    // author_dpi 縮退ログの発火（task 6.2・steering `logging.md` の
    // 「ログ無し失敗経路の禁止」）
    //
    // task 2.1 の檻は**戻り値だけ**を見ており、無宣言=debug・不正/0=warn という
    // 縮退梯子のレベル分離と `source` フィールドによる shell/balloon 帰属
    // （[`load_balloon_author_dpi`] の doc が明示的に主張している契約）は無検査だった。
    //
    // 捕捉は共有ハーネス [`crate::placement::test_support`]（`#[cfg(test)]` 限定）を使う。
    // **素朴な `with_default` 捕捉は非決定的に取りこぼす**——`tracing` の callsite interest
    // キャッシュはプロセス大域かつ「最初に踏んだスレッドが勝つ」ため、subscriber を持たない
    // 他テスト（`read_kv_lenient_missing_returns_empty` 等）が同じ callsite を先に踏むと
    // `Interest::never()` が焼き付き、捕捉窓の内側でもイベントが捨てられる。
    // 機構と対策（probe dispatcher 常駐による `has_just_one` 恒久偽化）は
    // `test_support` のモジュール doc を参照。
    // ------------------------------------------------------------------

    /// 無宣言は **`debug!`**（正典の既定＝異常ではない）で、`source`／`default_dpi` を残す。
    ///
    /// 無宣言を `warn!` へ格上げする実装（emo2 を含む正典既定のゴーストが毎回警告を吐く）と、
    /// 無言で 96 を返す実装（縮退が観測できない）の双方をここで落とす。
    #[test]
    fn parse_author_dpi_absent_logs_debug_with_source() {
        let (dpi, events) = capture_logs(|| parse_author_dpi(None, SHELL_DPI_KEY));
        assert_eq!(dpi, DEFAULT_AUTHOR_DPI);

        let ev = expect_one(&events, "宣言なし");
        assert_eq!(
            ev.level,
            tracing::Level::DEBUG,
            "無宣言は正典の既定＝異常ではない（warn 格上げ禁止）: {ev:?}"
        );
        assert_eq!(ev.field("source"), SHELL_DPI_KEY);
        assert_eq!(ev.field("default_dpi"), "96");
        assert_eq!(events.len(), 1, "1 分岐 1 ログ: {events:?}");
    }

    /// 正常宣言は**完全に無言**（無宣言/不正のレベル主張の非空虚性を担保する陰性対照）。
    #[test]
    fn parse_author_dpi_declared_is_silent() {
        for raw in ["96", "120", "144", "168", "192", "110", "65535"] {
            let expected: u16 = raw.parse().expect("テスト入力は u16");
            let (dpi, events) = capture_logs(|| parse_author_dpi(Some(raw), SHELL_DPI_KEY));
            assert_eq!(dpi, expected, "raw={raw}");
            assert!(events.is_empty(), "正常宣言は無言（raw={raw}）: {events:?}");
        }
    }

    /// 数値化不能は **`warn!`**＋`source`／`raw`／`error`（parse エラー）／`default_dpi`。
    ///
    /// `raw` が載ることで「どの生値が捨てられたか」が実機ログから判る（無言縮退の禁止）。
    #[test]
    fn parse_author_dpi_invalid_logs_warn_with_source_and_raw() {
        for raw in ["abc", "", " 120 ", "-96", "65536", "96.0", "0x60"] {
            let (dpi, events) = capture_logs(|| parse_author_dpi(Some(raw), BALLOON_DPI_KEY));
            assert_eq!(dpi, DEFAULT_AUTHOR_DPI, "raw={raw:?}");

            let ev = expect_one(&events, "数値として解釈できない");
            assert_eq!(ev.level, tracing::Level::WARN, "raw={raw:?}: {ev:?}");
            assert_eq!(ev.field("source"), BALLOON_DPI_KEY);
            assert_eq!(ev.field("raw"), raw, "捨てた生値をそのまま残す");
            assert_eq!(ev.field("default_dpi"), "96");
            assert!(
                ev.fields.contains_key("error"),
                "parse エラーを載せる（raw={raw:?}）: {ev:?}"
            );
            assert_eq!(events.len(), 1, "raw={raw:?}: {events:?}");
        }
    }

    /// `0` は **`warn!`** だが「数値化不能」とは**別メッセージ**（0 は解釈可能値であり、
    /// 分母に使えないことが理由——実機ログで両者を取り違えないための識別子）。
    #[test]
    fn parse_author_dpi_zero_logs_warn_distinct_from_invalid() {
        for raw in ["0", "00"] {
            let (dpi, events) = capture_logs(|| parse_author_dpi(Some(raw), SHELL_DPI_KEY));
            assert_eq!(dpi, DEFAULT_AUTHOR_DPI, "raw={raw}");

            let ev = expect_one(&events, "表示スケールの分母に使えない");
            assert_eq!(ev.level, tracing::Level::WARN, "raw={raw}: {ev:?}");
            assert_eq!(ev.field("source"), SHELL_DPI_KEY);
            assert_eq!(ev.field("raw"), raw);
            assert_eq!(ev.field("default_dpi"), "96");
            assert!(
                !ev.message().contains("数値として解釈できない"),
                "0 は解釈可能値ゆえ不正値と同じ文言にしない: {ev:?}"
            );
            assert_eq!(events.len(), 1, "raw={raw}: {events:?}");
        }
    }

    /// u16 境界の厳密確認（1 の差で受理／縮退が入れ替わる）。
    ///
    /// 「極端に大きい値」を u16 上限で切るのか別閾値で切るのかは実装の契約であり、
    /// 65535 受理・65536 縮退の対で固定する。
    #[test]
    fn parse_author_dpi_u16_boundary_is_exact() {
        assert_eq!(
            parse_author_dpi(Some("65535"), "test"),
            65535,
            "u16 上限ちょうどは素通し（正典は列挙ではなく推奨値）"
        );
        assert_eq!(
            parse_author_dpi(Some("65536"), "test"),
            96,
            "u16 溢れは既定へ縮退"
        );
        assert_eq!(parse_author_dpi(Some("1"), "test"), 1, "非ゼロ最小値は受理");
        // Rust の u16 パーサ準拠の受理形（現状挙動の固定・ukadoc は書式を規定しない）。
        assert_eq!(parse_author_dpi(Some("+120"), "test"), 120, "符号付き正値");
        assert_eq!(parse_author_dpi(Some("096"), "test"), 96, "前置ゼロ");
        // 前後空白は trim せず数値化不能扱い＝**縮退**（返り値 96 は正常宣言の 96 と
        // 数値的に区別できないため、縮退経路であることは
        // [`parse_author_dpi_invalid_logs_warn_with_source_and_raw`] の warn 検査が弁別する）。
        // なお本番経路ではここへ空白付きの値は届かない——`areka_parsers::kv::parse`
        // （`kv/parse.rs:31`/`:39`）がキー・値とも trim 済みで渡す。挙動の固定のみが目的。
        assert_eq!(parse_author_dpi(Some(" 120 "), "test"), 96);
    }

    /// [`load_balloon_author_dpi`] の doc が主張する契約:
    /// **読取器は 1 本のまま**で、shell か balloon かの帰属は `source` フィールドで区別できる。
    ///
    /// shell 経路は `seriko.dpi`・balloon 経路は `dpi` を `source` に載せる。
    /// `source` を定数直書きにしたり、両アクセサのキーを取り違える変異はここで落ちる。
    #[test]
    fn author_dpi_log_source_field_attributes_shell_vs_balloon() {
        let (shell_dpi, shell_events) =
            capture_logs(|| shell_source_with(&[("seriko.dpi", "abc")]).shell_author_dpi());
        assert_eq!(shell_dpi, DEFAULT_AUTHOR_DPI);
        assert_eq!(
            expect_one(&shell_events, "数値として解釈できない").field("source"),
            "seriko.dpi",
            "shell 起因のログは seriko.dpi に帰属する"
        );

        let zero = balloon_root_with("balloon_dpi_zero_log", "charset,UTF-8\ndpi,0\n");
        let (balloon_dpi, balloon_events) = capture_logs(|| load_balloon_author_dpi(&zero));
        assert_eq!(balloon_dpi, DEFAULT_AUTHOR_DPI);
        assert_eq!(
            expect_one(&balloon_events, "表示スケールの分母に使えない").field("source"),
            "dpi",
            "balloon 起因のログは dpi に帰属する"
        );
        let _ = fs::remove_dir_all(&zero);
    }

    /// balloon descript 不在は **2 本**のログ（読取失敗 `warn!`＋パス、無宣言 `debug!`）を残し、
    /// 読取失敗の文言は**帰属中立**である（[`read_kv_lenient`] は ghost/balloon 共有の読取器）。
    ///
    /// ファイルが無かったのか宣言が無かったのかを実機ログで弁別できることが縮退の観測条件。
    /// 加えて、共有読取器の文言を ghost 固定に戻す変異（バルーン起因を ghost 起因に見せる
    /// 帰属誤り・R6.3 の `RUST_LOG` grep を誤らせる）を「ghost を含まない」主張で殺す。
    #[test]
    fn load_balloon_author_dpi_missing_file_logs_read_warn_and_absent_debug() {
        let root = unique_temp_dir("balloon_dpi_missing_file_log").join("no_such_balloon");
        let (dpi, events) = capture_logs(|| load_balloon_author_dpi(&root));
        assert_eq!(dpi, DEFAULT_AUTHOR_DPI);

        let read_fail = expect_one(&events, "読み取りに失敗");
        assert_eq!(read_fail.level, tracing::Level::WARN, "{read_fail:?}");
        assert!(
            read_fail.field("path").contains("no_such_balloon"),
            "失敗したパスを残す: {read_fail:?}"
        );
        assert!(
            !read_fail.message().contains("ghost"),
            "共有読取器の失敗文言は帰属中立（balloon 経路で ghost 起因に見えてはならない）: {read_fail:?}"
        );

        let absent = expect_one(&events, "宣言なし");
        assert_eq!(absent.level, tracing::Level::DEBUG, "{absent:?}");
        assert_eq!(absent.field("source"), BALLOON_DPI_KEY);

        assert_eq!(events.len(), 2, "読取失敗と無宣言の 2 本: {events:?}");
    }

    /// balloon descript が読めて宣言もあれば**無言**で宣言値を返す
    /// （実ファイル経路でも正常系にログを撒かない）。
    #[test]
    fn load_balloon_author_dpi_declared_file_is_silent() {
        let declared = balloon_root_with("balloon_dpi_declared_log", "charset,UTF-8\ndpi,192\n");
        let (dpi, events) = capture_logs(|| load_balloon_author_dpi(&declared));
        assert_eq!(dpi, 192);
        assert!(events.is_empty(), "正常経路は無言: {events:?}");
        let _ = fs::remove_dir_all(&declared);
    }

    /// `shell_author_dpi` は**実ファイル**（descript.txt → decode → parse_kv）経由でも
    /// 宣言値を運ぶ（`shell_source_with` の in-memory 檻が通す経路の end-to-end 確認）。
    ///
    /// `seriko.dpi` が KV パーサで落ちる／キー名がずれる変異を、実 I/O 込みで落とす。
    #[test]
    fn shell_author_dpi_reads_declared_value_from_real_descript_file() {
        let root = unique_temp_dir("shell_author_dpi_declared_file");
        let _ = fs::remove_dir_all(&root);
        let ghost_master = root.join("ghost").join("master");
        fs::create_dir_all(&ghost_master).expect("create ghost/master");
        fs::write(
            ghost_master.join("descript.txt"),
            "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
        )
        .expect("write ghost descript");
        let shell_dir = root.join("shell").join("master");
        fs::create_dir_all(&shell_dir).expect("create shell/master");
        fs::write(
            shell_dir.join("descript.txt"),
            "charset,UTF-8\nseriko.dpi,144\n".as_bytes(),
        )
        .expect("write shell descript");

        let src = load_descript_source(&root).expect("shell descript があれば Ok");
        assert_eq!(
            src.shell_author_dpi(),
            144,
            "descript の実宣言値（既定 96 の素通しではない）"
        );

        let _ = fs::remove_dir_all(&root);
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
