//! 1 字ごとの間隔の式（文字の層と seriko が呼ぶ 1 本の関数）

use dola::{TimeDelta, TimeSpan};

/// 1 塊の字の現れる時刻（塊の頭からの隔たり）を、字の順に返す。
///
/// `prev_since_chunk_start` は、前の字の現れる時刻 − この塊の頭（前の字が無ければ `None`）。
/// 字の時刻は `max(前の字 + duration ÷ count, 塊の頭)` を、字ごとに 1 回だけ丸めた値である。
pub fn chunk_reveal_offsets(
    prev_since_chunk_start: Option<TimeDelta>,
    duration: TimeSpan,
    count: usize,
) -> impl Iterator<Item = TimeSpan> {
    let count = count as u64;
    // 前の字 + 間隔 ≥ 塊の頭 を、実数の間隔を作らずに p × count + duration ≥ 0 で比べる。
    let continuing = prev_since_chunk_start.filter(|p| {
        i128::from(p.as_ticks()) * i128::from(count) + i128::from(duration.as_ticks()) >= 0
    });
    (0..count).map(move |i| match continuing {
        None => duration.mul_div(i, count),
        // 最後の字を p + duration ちょうどに置き、終わりの側から測る（塊を続けても差が積もらない）。
        Some(p) => (p + TimeDelta::from(duration)
            - TimeDelta::from(duration.mul_div(count - 1 - i, count)))
        .to_span()
        .expect("p × count + duration ≥ 0 なので隔たりは 0 以上"),
    })
}

#[cfg(test)]
#[path = "reveal_tests.rs"]
mod tests;
