//! `--dump-surface-list` の読みと、シェルの面の番号への展開（要件 3.1〜3.5・3.7）。

use std::collections::BTreeSet;

/// 指定の 1 要素（surfaces.txt の面の番号の拡張の書き方と同じ意味）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SurfacePick {
    Single(u32),
    Range(u32, u32),
    Exclude(u32),
    ExcludeRange(u32, u32),
}

/// 読めた要素の列（書かれた順）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SurfaceSelection {
    picks: Vec<SurfacePick>,
}

/// 読めない要素は warn! を出して飛ばす。
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.7）
pub(crate) fn read_surface_list(raw: &str) -> SurfaceSelection {
    let mut picks = Vec::new();
    for element in raw.split(',') {
        let element = element.trim();
        match read_pick(element) {
            Some(pick) => picks.push(pick),
            None => tracing::warn!(
                event = "offline_dump_surface_list_unreadable",
                element,
                "[offline_dump] --dump-surface-list の要素を面の番号として読めないので、この要素を飛ばします"
            ),
        }
    }
    SurfaceSelection { picks }
}

/// 頭の `surface`（大文字小文字を問わない）を 1 つ外し、`N`・`a-b`・`!N`・`!a-b` として読む。
fn read_pick(element: &str) -> Option<SurfacePick> {
    const PREFIX: &str = "surface";
    let body = match element.get(..PREFIX.len()) {
        Some(head) if head.eq_ignore_ascii_case(PREFIX) => &element[PREFIX.len()..],
        _ => element,
    };
    let (exclude, body) = match body.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, body),
    };
    let pick = match body.split_once('-') {
        Some((a, b)) => {
            let (a, b) = (a.parse::<u32>().ok()?, b.parse::<u32>().ok()?);
            if a > b {
                return None;
            }
            if exclude {
                SurfacePick::ExcludeRange(a, b)
            } else {
                SurfacePick::Range(a, b)
            }
        }
        None => {
            let n = body.parse::<u32>().ok()?;
            if exclude {
                SurfacePick::Exclude(n)
            } else {
                SurfacePick::Single(n)
            }
        }
    };
    Some(pick)
}

impl SurfaceSelection {
    /// シェルに在る番号へ展開する（昇順・重ねなし）。単独で指した無い面は warn!。
    #[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.7）
    pub(crate) fn resolve(&self, existing: &BTreeSet<u32>) -> Vec<u32> {
        let mut included = BTreeSet::new();
        // 範囲のまま持つ（`!0-4294967295` を番号へ広げない）。
        let mut excluded = Vec::new();
        let mut missing = BTreeSet::new();
        for pick in &self.picks {
            match *pick {
                SurfacePick::Single(n) if existing.contains(&n) => {
                    included.insert(n);
                }
                SurfacePick::Single(n) => {
                    missing.insert(n);
                }
                SurfacePick::Range(a, b) => included.extend(existing.range(a..=b)),
                SurfacePick::Exclude(n) => excluded.push(n..=n),
                SurfacePick::ExcludeRange(a, b) => excluded.push(a..=b),
            }
        }
        for surface in missing {
            tracing::warn!(
                event = "offline_dump_surface_missing",
                surface,
                "[offline_dump] --dump-surface-list で指した面がシェルに無いので飛ばします"
            );
        }
        included
            .into_iter()
            .filter(|n| !excluded.iter().any(|r| r.contains(n)))
            .collect()
    }
}

#[cfg(test)]
#[path = "surface_list_tests.rs"]
mod surface_list_tests;
