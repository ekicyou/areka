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
#[path = "source_tests.rs"]
mod tests;
