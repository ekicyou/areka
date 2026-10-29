//! strict の台本の合図の配り口（spec: mcp-strict-errors）。`compile_strict` が残す控え [`StrictTrace`] を置く。

use std::ops::Range;

use crate::strict::DetectionPoint;

/// `compile_strict` の控え: 合図ごとの台本の位置と、合図の順番に結びつけた誤り。
///
/// 順番は sink へ配られる合図（`CuePayload::Command`）だけを数える。先頭の全消しは順番 0、
/// 末尾の選択待ちの区切りと、後で足すエピローグの合図は数えない。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StrictTrace {
    /// 合図の順番 → 台本の読みの範囲（バイト）。None は台本に元の無い合図。
    pub(crate) cue_spans: Vec<Option<Range<usize>>>,
    /// （結びつけた合図の順番, 検出点, 読みの範囲）を位置の順に。順番が合図の数と等しいものは
    /// 最後の合図より後ろの命令の誤り。
    pub(crate) pending: Vec<(usize, DetectionPoint, Range<usize>)>,
}
