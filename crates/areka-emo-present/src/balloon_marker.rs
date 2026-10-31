//! 印の族（スクロールの矢印・クリック待ち）の表と名前の連鎖、番号ごとの絵の選び（balloon_marker.rs）。
//!
//! 族の表は吹き出しの族と同じ [`SeriesFamily`] で持ち、連鎖は [`prefix_chain`] がそのまま作る
//! （旧名 `arrow`／`clickwait` が最後）。descript の名前の鍵（`arrow.filename` など）で書かれた
//! 名前は連鎖の先頭に置く。番号 `0`（上・縦書きでは右）と `1`（下・縦書きでは左）は、それぞれ
//! 別に連鎖の先頭から辿る（番号ごと・要件 2.1）。名前の判定はバルーンの面と同じ
//! [`face_digits_of`]（大小無視・`.png`・全数字）を共有する（要件 2.6）。

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use areka_emo_atlas::{AlphaParams, ElementDecoder, Normalizer};

use crate::balloon::{SeriesFamily, SeriesPrefix, face_digits_of, prefix_chain};

/// スクロールの矢印の族（`arrowp{n}def`・`arrows`／`arrowk`・旧名 `arrow` が最後）。
pub const ARROW_FAMILY: SeriesFamily = SeriesFamily {
    base: "arrow",
    scope0_legacy: &["arrows", "arrow"],
    scope1_legacy: &["arrowk"],
};

/// クリック待ちの印の族（本 spec では表と決定論のテストだけ・絵は読まない・要件 2.7）。
pub const CLICKWAIT_FAMILY: SeriesFamily = SeriesFamily {
    base: "clickwait",
    scope0_legacy: &["clickwaits", "clickwait"],
    scope1_legacy: &["clickwaitk"],
};

/// 印の名前の連鎖（書かれた名前を先頭に、族の連鎖を後に辿る）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerChain {
    /// 名前の鍵で書かれた名前（無ければ `None`・全スコープ共通・要件 2.2）。
    pub explicit: Option<String>,
    /// 族の連鎖（[`prefix_chain`] の答え）。
    pub chain: Vec<SeriesPrefix>,
}

/// スコープの印の名前の連鎖を作る（純関数）。空の名前は書かれていないものとして扱う。
pub fn marker_chain(family: &SeriesFamily, scope: u32, explicit: Option<&str>) -> MarkerChain {
    MarkerChain {
        explicit: explicit.filter(|s| !s.is_empty()).map(str::to_string),
        chain: prefix_chain(family, scope),
    }
}

/// 番号 1 つの採用（実ファイル名は原形のまま）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerName {
    /// 実ファイル名（大小は資産のまま）。
    pub file_name: String,
    /// 採った名前（書かれた名前か、連鎖の接頭辞）。
    pub prefix: String,
    /// 書かれた名前で採ったか。
    pub explicit: bool,
}

/// 番号 0（上）と 1（下）の採用。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkerNames {
    pub up: Option<MarkerName>,
    pub down: Option<MarkerName>,
}

/// ファイル名の列から番号 0・1 の採用ファイル名を決める純核（fs 非依存）。
///
/// 番号ごとに、書かれた名前 → 連鎖の先頭から順に照らし、最初に絵がある名前を採る。番号 0・1 以外は
/// 見ない。同じ名前・番号に大小違いのファイルが並ぶときは辞書順最小を採る（走査順に依らない・
/// [`crate::balloon::select_faces`] と同じ決め方）。
pub fn select_marker_names<S: AsRef<str>>(names: &[S], chain: &MarkerChain) -> MarkerNames {
    let candidates: Vec<(&str, bool)> = chain
        .explicit
        .as_deref()
        .map(|e| (e, true))
        .into_iter()
        .chain(chain.chain.iter().map(|p| (p.prefix.as_str(), false)))
        .collect();

    // (番号, 候補の位置) → 実ファイル名。キー順が「候補の先頭優先」と一致する。
    let mut hits: BTreeMap<(u32, usize), &str> = BTreeMap::new();
    for name in names {
        let name = name.as_ref();
        for (index, (prefix, _)) in candidates.iter().enumerate() {
            let Some(number) = face_digits_of(prefix, name).and_then(|d| d.parse::<u32>().ok())
            else {
                continue;
            };
            if number > 1 {
                continue;
            }
            hits.entry((number, index))
                .and_modify(|adopted| {
                    if name < *adopted {
                        *adopted = name;
                    }
                })
                .or_insert(name);
        }
    }

    let pick = |number: u32| {
        hits.range((number, 0)..(number + 1, 0))
            .next()
            .map(|(&(_, index), file_name)| MarkerName {
                file_name: (*file_name).to_string(),
                prefix: candidates[index].0.to_string(),
                explicit: candidates[index].1,
            })
    };
    MarkerNames {
        up: pick(0),
        down: pick(1),
    }
}

/// 読んだ印の絵（原寸・乗算済み BGRA・バルーンの面と同じ透過の扱い）。
#[derive(Debug, PartialEq, Eq)]
pub struct MarkerImage {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

/// 番号 0（上）と 1（下）の印の絵。絵が無い番号は `None`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkerPair {
    pub up: Option<Arc<MarkerImage>>,
    pub down: Option<Arc<MarkerImage>>,
}

/// 印の絵を順の付いたフォルダの列から解いて読む（fs と記録の入口・読み込みの時だけ呼ぶ）。
///
/// 番号ごとに、列の先頭のフォルダで連鎖を全部辿り、無ければ次のフォルダへ進む（要件 7.3）。
/// 書かれた名前の絵が無ければ既定の連鎖へ落ちて番号ごとに `warn!`（要件 2.3）、片方だけなら
/// 無い番号を `warn!`（要件 2.5）、1 枚も無ければ両方 `None`（要件 2.4）。絵はバルーンの面と同じ
/// [`Normalizer`] と `alpha` で乗算済み BGRA にし、読めない絵は場所と理由を `warn!` してその番号
/// だけ `None` で続ける（要件 9.4）。解決 1 回につき `debug!` を 1 行残す（要件 9.1）。
pub fn resolve_marker_pair(
    dirs: &[&Path],
    family: &SeriesFamily,
    scope: u32,
    explicit: Option<&str>,
    decoder: &impl ElementDecoder,
    alpha: AlphaParams,
) -> MarkerPair {
    let chain = marker_chain(family, scope, explicit);
    let per_dir: Vec<(&Path, MarkerNames)> = dirs
        .iter()
        .map(|dir| (*dir, select_marker_names(&list_file_names(dir), &chain)))
        .collect();
    let find = |pick: fn(&MarkerNames) -> &Option<MarkerName>| {
        per_dir
            .iter()
            .find_map(|(dir, names)| pick(names).as_ref().map(|n| (*dir, n)))
    };
    let up = find(|n| &n.up);
    let down = find(|n| &n.down);

    let sides = [("0", up), ("1", down)];
    if let Some(name) = &chain.explicit {
        for (number, found) in sides {
            if !found.is_some_and(|(_, n)| n.explicit) {
                tracing::warn!(
                    scope,
                    name = %name,
                    number,
                    "balloon_marker: 書かれた名前の絵が無いので既定の名前の順で探し直す"
                );
            }
        }
    }
    match (up.is_some(), down.is_some()) {
        (true, false) | (false, true) => {
            let missing = if up.is_none() { "0" } else { "1" };
            tracing::warn!(
                scope,
                base = family.base,
                missing,
                "balloon_marker: 印の絵が片方しか無い（無い方は出さない）"
            );
        }
        _ => {}
    }

    let load = |found: Option<(&Path, &MarkerName)>| {
        let (dir, name) = found?;
        let path = dir.join(&name.file_name);
        match decoder.decode(&path) {
            Ok(img) => Some(Arc::new(to_marker_image(Normalizer.normalize(img, alpha)))),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "balloon_marker: 印の絵が読めない（その印は出さない）"
                );
                None
            }
        }
    };
    let pair = MarkerPair {
        up: load(up),
        down: load(down),
    };

    let chain_text: Vec<&str> = chain
        .explicit
        .iter()
        .map(String::as_str)
        .chain(chain.chain.iter().map(|p| p.prefix.as_str()))
        .collect();
    let describe = |found: Option<(&Path, &MarkerName)>, image: &Option<Arc<MarkerImage>>| match (
        found, image,
    ) {
        (Some((dir, n)), Some(img)) => format!(
            "{} {}x{}",
            dir.join(&n.file_name).display(),
            img.width,
            img.height
        ),
        (Some((dir, n)), None) => format!("{} unreadable", dir.join(&n.file_name).display()),
        (None, _) => "none".to_string(),
    };
    tracing::debug!(
        scope,
        base = family.base,
        chain = %chain_text.join(">"),
        up = %describe(up, &pair.up),
        down = %describe(down, &pair.down),
        "balloon_marker: 印の絵を解いた"
    );
    pair
}

/// フォルダ直下のファイル名を集める（読めないフォルダは `warn!` して空・印は無くても失敗にしない）。
fn list_file_names(dir: &Path) -> Vec<String> {
    match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|entry| {
                entry
                    .inspect_err(|e| {
                        tracing::warn!(
                            dir = %dir.display(),
                            error = %e,
                            "balloon_marker: ディレクトリエントリの取得に失敗（スキップ）"
                        );
                    })
                    .ok()
            })
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .collect(),
        Err(e) => {
            tracing::warn!(
                dir = %dir.display(),
                error = %e,
                "balloon_marker: フォルダが読めない（印の絵は無いものとする）"
            );
            Vec::new()
        }
    }
}

/// 行の詰め物（stride の余り）を除いて原寸の BGRA にする。
fn to_marker_image(img: areka_emo_atlas::NormalizedImage) -> MarkerImage {
    let row = img.width as usize * 4;
    let bgra = if img.stride as usize == row {
        img.pbgra
    } else {
        img.pbgra
            .chunks(img.stride as usize)
            .flat_map(|r| &r[..row])
            .copied()
            .collect()
    };
    MarkerImage {
        width: img.width,
        height: img.height,
        bgra,
    }
}

#[cfg(test)]
#[path = "balloon_marker_load_tests.rs"]
mod load_tests;
#[cfg(test)]
#[path = "balloon_marker_tests.rs"]
mod tests;
