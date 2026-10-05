//! 安全地帯の判定・置き場所の計算・はみ出す行の折り返し（純粋）。
//!
//! 入力も出力も整数の画面の物理ピクセル。World も OS も知らない。
//! 矩形は Win32 の RECT と同じく、左と上の端を含み、右と下の端を含まない。

/// 画面の物理ピクセルの矩形（右と下の端は含まない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RectPx {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// 画面の物理ピクセルの点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PointPx {
    pub x: i32,
    pub y: i32,
}

/// 物理ピクセルの大きさ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SizePx {
    pub width: i32,
    pub height: i32,
}

impl RectPx {
    fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    fn contains(&self, p: PointPx) -> bool {
        self.left <= p.x && p.x < self.right && self.top <= p.y && p.y < self.bottom
    }
}

/// 点が安全地帯の中か。tip が None なら範囲の矩形だけを見る。
///
/// 安全地帯は「範囲の矩形 ∪ ツールチップの矩形 ∪ その 2 つを包む最小の凸の領域」。
/// 包む領域は、2 つを囲む外枠から、どちらの矩形にも属さない角を斜めの線で切り落とした形になる。
pub(crate) fn in_safe_zone(p: PointPx, range: RectPx, tip: Option<RectPx>) -> bool {
    let Some(tip) = tip else {
        return range.contains(p);
    };
    if range.is_empty() || tip.is_empty() {
        return range.contains(p) || tip.contains(p);
    }
    // 端の画素を含む座標（右と下は 1 引く）にそろえて、外枠と 4 つの角を見る。
    let a = Inclusive::from(range);
    let b = Inclusive::from(tip);
    let (x, y) = (i64::from(p.x), i64::from(p.y));
    let in_bounds =
        a.l.min(b.l) <= x && x <= a.r.max(b.r) && a.t.min(b.t) <= y && y <= a.b.max(b.b);
    // 右と下の角は、左右・上下を裏返して左上の角として見る。
    in_bounds
        && corner_ok((x, y), a, b)
        && corner_ok((-x, y), a.flip_x(), b.flip_x())
        && corner_ok((x, -y), a.flip_y(), b.flip_y())
        && corner_ok((-x, -y), a.flip_x().flip_y(), b.flip_x().flip_y())
}

/// 端の画素を含む矩形（裏返しで負になっても桁あふれしないよう i64）。
#[derive(Clone, Copy)]
struct Inclusive {
    l: i64,
    t: i64,
    r: i64,
    b: i64,
}

impl From<RectPx> for Inclusive {
    fn from(rc: RectPx) -> Self {
        Self {
            l: rc.left.into(),
            t: rc.top.into(),
            r: i64::from(rc.right) - 1,
            b: i64::from(rc.bottom) - 1,
        }
    }
}

impl Inclusive {
    fn flip_x(self) -> Self {
        Self {
            l: -self.r,
            r: -self.l,
            ..self
        }
    }

    fn flip_y(self) -> Self {
        Self {
            t: -self.b,
            b: -self.t,
            ..self
        }
    }
}

/// 外枠の左上の角について、点が切り落とす側にないか。
///
/// 角がどちらかの矩形の角なら切り落としは無い。そうでなければ、いちばん左の矩形の左上と
/// いちばん上の矩形の左上を結ぶ線が包む領域の縁になる（線の上は中）。
fn corner_ok(p: (i64, i64), a: Inclusive, b: Inclusive) -> bool {
    let corner = (a.l.min(b.l), a.t.min(b.t));
    if (a.l, a.t) == corner || (b.l, b.t) == corner {
        return true;
    }
    let (leftmost, topmost) = if a.l < b.l { (a, b) } else { (b, a) };
    let from = (leftmost.l, leftmost.t);
    let to = (topmost.l, topmost.t);
    let side = |q: (i64, i64)| (to.0 - from.0) * (q.1 - from.1) - (to.1 - from.1) * (q.0 - from.0);
    // 角は線から外れているので side(corner) は 0 でない。点が角と同じ側なら外。
    side(p) * side(corner).signum() <= 0
}

/// 置き場所の計算の入力。
pub(crate) struct PlaceInput {
    /// 出すことが決まった時のマウスの位置。
    pub anchor: PointPx,
    /// ツールチップの大きさ。
    pub tip: SizePx,
    /// マウスのある画面の作業領域。
    pub work_area: RectPx,
    /// カーソルから上へ離す量（20 を画面の DPI で換算）。
    pub offset_above: i32,
    /// 下へ返すときに離す量（カーソルの絵の高さ）。
    pub offset_below: i32,
}

/// ツールチップの左上の位置を返す。
///
/// 真上・左右の中央合わせ・カーソルから離す。真上に収まらなければ真下へ返し、どちらにも
/// 収まらなければ広い側に置く。最後に作業領域の中へ寄せる（大きすぎれば左上に合わせる）。
pub(crate) fn place(input: &PlaceInput) -> PointPx {
    let PlaceInput {
        anchor,
        tip,
        work_area: wa,
        offset_above,
        offset_below,
    } = *input;
    let above = anchor.y - offset_above - tip.height;
    let below = anchor.y + offset_below;
    // 収まらない側では負になる「空き」。どちらにも収まらなければ大きい方（広い側）を採る。
    let room_above = above - wa.top;
    let room_below = wa.bottom - (below + tip.height);
    let y = if room_above >= 0 || (room_below < 0 && room_above >= room_below) {
        above
    } else {
        below
    };
    PointPx {
        x: clamp_into(anchor.x - tip.width / 2, tip.width, wa.left, wa.right),
        y: clamp_into(y, tip.height, wa.top, wa.bottom),
    }
}

/// 長さ size のものの始まりを [lo, hi) の中へ寄せる。入りきらなければ lo に合わせる。
fn clamp_into(start: i32, size: i32, lo: i32, hi: i32) -> i32 {
    start.min(hi - size).max(lo)
}

/// 幅を越える行を、入る文字数ごとに改行で割る。fit は「この並びの先頭から何文字入るか」（1 以上）。
///
/// 文字数は Rust の `char` で数えるので、多バイトの文字の途中では切らない。既にある改行は
/// そのまま保ち、足す改行は CR LF。fit が 0 を返しても 1 文字は進める（止まらないため）。
pub(crate) fn force_break(text: &str, fit: &dyn Fn(&str) -> usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let end = rest.find(['\r', '\n']).unwrap_or(rest.len());
        let mut line = &rest[..end];
        while !line.is_empty() {
            let n = fit(line).max(1);
            let cut = line.char_indices().nth(n).map_or(line.len(), |(i, _)| i);
            out.push_str(&line[..cut]);
            line = &line[cut..];
            if !line.is_empty() {
                out.push_str("\r\n");
            }
        }
        let newline = if rest[end..].starts_with("\r\n") {
            2
        } else {
            rest[end..].chars().next().map_or(0, char::len_utf8)
        };
        out.push_str(&rest[end..end + newline]);
        rest = &rest[end + newline..];
    }
    out
}

/// 改行（LF だけ・CR LF・CR だけ）を、標準のツールチップが行を分ける CR LF に揃える（要件 3.3）。
pub(crate) fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n")
}

/// 最大の幅（物理ピクセル）＝ min(320 を dpi で換算した幅, 作業領域の幅)（要件 3.4）。
pub(crate) fn max_tip_width(dpi: u32, work_area: RectPx) -> i32 {
    let scaled = (320 * i64::from(dpi) + 48) / 96;
    let work = i64::from(work_area.right) - i64::from(work_area.left);
    scaled.min(work) as i32
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod geometry_tests;
