use super::{Resource, WritingMode, error, info};

// ---------------------------------------------------------------------------
// 判定（Verdict）と readback 述語ヘルパ
// ---------------------------------------------------------------------------

/// 単一 pass/fail の集計（World リソース・main が run() 復帰後に読む）。
#[derive(Resource, Default)]
pub(super) struct Verdict {
    /// 通過した readback/構造 assert の件数。
    pub(super) checks: usize,
    /// 失敗記録（空なら PASS 候補）。
    pub(super) failures: Vec<String>,
    /// シナリオが最後（Clear 検証）まで到達したか。
    pub(super) done: bool,
}

impl Verdict {
    /// 述語を検証し、失敗は log-first で記録する（panic しない・FAIL へ集計）。
    pub(super) fn check(&mut self, cond: bool, label: &str) {
        if cond {
            self.checks += 1;
            info!(check = label, "readback 検証 OK");
        } else {
            error!(check = label, "readback 検証 FAIL");
            self.failures.push(label.to_string());
        }
    }
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0・attach_wiring_test と同じ述語）。
pub(super) fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// ピクセル (x, y) が非透明か。
fn is_opaque(bytes: &[u8], w: u32, x: u32, y: u32) -> bool {
    bytes[((y * w + x) * 4 + 3) as usize] != 0
}

/// 行送り軸（block 軸）方向のインク範囲（validrect-local 物理 px）。
///
/// 軸読み替え正準表: horizontal_tb＝最下インク行（+y）・vertical_rl＝右端から最左インク列
/// までの距離（−x 方向）。改行・折返しで行/列が増えると単調に伸びる。
pub(super) fn block_extent(bytes: &[u8], w: u32, h: u32, mode: WritingMode) -> u32 {
    let mut extent = 0u32;
    for y in 0..h {
        for x in 0..w {
            if is_opaque(bytes, w, x, y) {
                let e = match mode {
                    WritingMode::HorizontalTb => y + 1,
                    WritingMode::VerticalRl => w - x,
                    WritingMode::VerticalLr => x + 1,
                };
                extent = extent.max(e);
            }
        }
    }
    extent
}

/// 先頭バンド（可視窓先頭の 1 行/列分・厚み `pitch`）内の行内軸インク範囲
/// （validrect-local 物理 px）。スクロールで先頭行が消える（短い行に入れ替わる）と縮む。
pub(super) fn inline_extent_first_band(
    bytes: &[u8],
    w: u32,
    h: u32,
    mode: WritingMode,
    pitch: u32,
) -> u32 {
    let mut extent = 0u32;
    match mode {
        WritingMode::HorizontalTb => {
            for y in 0..pitch.min(h) {
                for x in 0..w {
                    if is_opaque(bytes, w, x, y) {
                        extent = extent.max(x + 1);
                    }
                }
            }
        }
        WritingMode::VerticalRl => {
            for x in w.saturating_sub(pitch)..w {
                for y in 0..h {
                    if is_opaque(bytes, w, x, y) {
                        extent = extent.max(y + 1);
                    }
                }
            }
        }
        WritingMode::VerticalLr => {
            for x in 0..pitch.min(w) {
                for y in 0..h {
                    if is_opaque(bytes, w, x, y) {
                        extent = extent.max(y + 1);
                    }
                }
            }
        }
    }
    extent
}

/// 行送り軸バンド `[b0, b1)`（validrect-local）内の非透明ピクセル数。
pub(super) fn band_ink(bytes: &[u8], w: u32, h: u32, mode: WritingMode, b0: f32, b1: f32) -> usize {
    let clamp = |v: f32, max: u32| -> u32 { (v.max(0.0) as u32).min(max) };
    let mut count = 0usize;
    match mode {
        WritingMode::HorizontalTb => {
            for y in clamp(b0, h)..clamp(b1, h) {
                for x in 0..w {
                    if is_opaque(bytes, w, x, y) {
                        count += 1;
                    }
                }
            }
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            for x in clamp(b0, w)..clamp(b1, w) {
                for y in 0..h {
                    if is_opaque(bytes, w, x, y) {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

// ---------------------------------------------------------------------------
// 観測記録（チェックポイント間で持ち回る readback 実測値）
// ---------------------------------------------------------------------------

#[derive(Default)]
pub(super) struct Observations {
    /// C1: typewriter 途中の非透明ピクセル数。
    pub(super) ink_c1: usize,
    /// C2: 改行前の block 軸インク範囲。
    pub(super) block_extent_c2: u32,
    /// C2: 改行前の行数（純粋 layout・折返し観測の基準）。
    pub(super) lines_c2: usize,
    /// C4: スクロール前の先頭バンド行内インク範囲。
    pub(super) inline_extent_c4: u32,
    /// C4: \1（kero）供給面のバイト列スナップショット（独立性検証用）。
    pub(super) kero_bytes_c4: Vec<u8>,
}
