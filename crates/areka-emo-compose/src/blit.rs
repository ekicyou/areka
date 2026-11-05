//! `BlitExecutor`: 命令列を CPU 整数演算で転写する execute 段。
//!
//! premultiplied SourceOver（`dst_c' = src_c + div255(dst_c × (255 − src_a))`・
//! `div255(v) = (v + 127) / 255`）を u8/u32 整数で実装し、浮動小数を経路に持ち込まない。
//! 式と「1 命令を矩形で切って重ねる」処理は [`overlay_op`] の 1 か所だけにあり、いちからの合成
//! （[`execute`]）と矩形の重ね直し（[`crate::blit_rect`]）の両方がそれを呼ぶ。
//! 転写先座標は「配置座標＋`trim_offset`」で算出しトリムが見た目を変えないことを保証する。
//! `placement` が None（全透明）のエントリは転写をスキップする。合成先バッファを再利用し
//! アトラス転写を O(elements) で行い、途中アロケーションを発生させない。

use areka_emo_atlas::{AtlasPage, AtlasTable, Rect};

use crate::composed::ComposedSurface;
use crate::plan::{BlitOp, Extent};

/// premultiplied SourceOver の整数丸め除算（決定6・`div255(v) = (v + 127) / 255`）。
///
/// 浮動小数を用いず u32 整数演算で `v / 255` を四捨五入する。`v` は最大
/// `255 * 255 = 65025`（`dst_c * (255 - src_a)`）ゆえ u32 で桁溢れしない。
#[inline]
fn div255(v: u32) -> u32 {
    (v + 127) / 255
}

/// premultiplied SourceOver 1 チャンネル（決定6・`dst_c' = src_c + div255(dst_c × (255 − src_a))`）。
///
/// `src_c`/`dst_c` は premultiplied 済みチャンネル値、`inv_src_a = 255 - src_a`。α にも同一式を
/// 適用する（`out_a = src_a + div255(dst_a × (255 − src_a))`）。結果は常に 0..=255 に収まる
/// （src_c ≤ src_a・div255(dst_c×inv) ≤ dst_c ≤ 255 − src_a が premultiplied 不変条件から従う）。
/// 桁溢れ防止に u8→u32 で計算し末尾で u8 化する（飽和不要だが安全側に `min(255)`）。
#[inline]
fn source_over_channel(src_c: u8, dst_c: u8, inv_src_a: u32) -> u8 {
    let out = src_c as u32 + div255(dst_c as u32 * inv_src_a);
    out.min(255) as u8
}

/// 半開の矩形 `[x0, x1) × [y0, y1)`（i64・空は `x0 >= x1 || y0 >= y1`）。
#[derive(Clone, Copy)]
pub(crate) struct Area {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Area {
    pub(crate) const EMPTY: Area = Area {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
    };

    pub(crate) fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    pub(crate) fn intersect(self, o: Area) -> Area {
        Area {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    pub(crate) fn union(self, o: Area) -> Area {
        if self.is_empty() {
            return o;
        }
        if o.is_empty() {
            return self;
        }
        Area {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }
}

/// 命令の置き場（左上が配置＋`trim_offset`・大きさが `uv_rect`・座標は i64）。
/// `placement` が無い命令は空。
pub(crate) fn op_area(op: &BlitOp, atlas: &AtlasTable) -> Area {
    let Some(p) = atlas.entry(op.element).placement.as_ref() else {
        return Area::EMPTY;
    };
    let (ox, oy) = op.transform.offset();
    let x0 = ox + p.trim_offset.x as i64;
    let y0 = oy + p.trim_offset.y as i64;
    Area {
        x0,
        y0,
        x1: x0 + p.uv_rect.w as i64,
        y1: y0 + p.uv_rect.h as i64,
    }
}

/// `uv` が頁 `page` に収まり、頁の行の幅とバイト列の長さも足りるか（掛け算と足し算は `u64`）。
/// 表（`AtlasTable::new`）は uv と頁を照らさないので、転写の手前で信じずに確かめる。
fn uv_fits(uv: &Rect, page: &AtlasPage) -> bool {
    let (w, h, stride) = (page.width as u64, page.height as u64, page.stride as u64);
    uv.x as u64 + uv.w as u64 <= w
        && uv.y as u64 + uv.h as u64 <= h
        && w * 4 <= stride
        && stride * h <= page.bytes.len() as u64
}

/// 命令 `op` を `clip`（合成先の中の矩形）で切って `dst`（行の幅 `stride`）へ SourceOver で重ねる。
///
/// 置き場が無い（全透明）なら何もしない。頁が無い・uv が頁に収まらないなら `warn!` を出して
/// その命令だけ飛ばす（非パニック・破損アトラス防御）。`clip` は合成先の中にあること（呼び手の責任）。
pub(crate) fn overlay_op(
    dst: &mut [u8],
    stride: usize,
    clip: Area,
    op: &BlitOp,
    atlas: &AtlasTable,
) {
    let Some(placement) = atlas.entry(op.element).placement.as_ref() else {
        return;
    };
    let Some(page) = atlas.page(placement.page) else {
        tracing::warn!(
            target: "areka_emo_compose",
            element = op.element.0,
            page = placement.page,
            "転写元アトラス頁が存在しない: 命令を skip"
        );
        return;
    };
    let uv = &placement.uv_rect;
    if !uv_fits(uv, page) {
        tracing::warn!(
            target: "areka_emo_compose",
            element = op.element.0,
            page = placement.page,
            uv = ?uv,
            "転写元の uv が頁に収まらない: 命令を skip"
        );
        return;
    }
    let placed = op_area(op, atlas);
    let area = placed.intersect(clip);
    if area.is_empty() {
        return;
    }
    // 転写先座標 − 置き場の左上 ＝ 転写元の uv 内座標。
    let src_x = (uv.x as i64 + area.x0 - placed.x0) as usize * 4;
    let src_y = (uv.y as i64 + area.y0 - placed.y0) as usize;
    let n = (area.x1 - area.x0) as usize * 4;
    let src_stride = page.stride as usize;
    for (i, dy) in (area.y0..area.y1).enumerate() {
        let s = (src_y + i) * src_stride + src_x;
        let d = dy as usize * stride + area.x0 as usize * 4;
        // premultiplied BGRA: [B, G, R, A] へ決定6の式を各 ch＋α に同式適用。
        let (src_px, _) = page.bytes[s..s + n].as_chunks::<4>();
        let (dst_px, _) = dst[d..d + n].as_chunks_mut::<4>();
        for (s, d) in src_px.iter().zip(dst_px) {
            let inv_src_a = 255u32 - s[3] as u32;
            d[0] = source_over_channel(s[0], d[0], inv_src_a);
            d[1] = source_over_channel(s[1], d[1], inv_src_a);
            d[2] = source_over_channel(s[2], d[2], inv_src_a);
            d[3] = source_over_channel(s[3], d[3], inv_src_a);
        }
    }
}

/// 命令列をアトラス頁から合成先バッファ `out` へ premultiplied SourceOver 整数演算で転写する。
///
/// design「BlitExecutor（blit.rs）」の契約に厳密に従う:
///
/// 1. `out` を `extent`（`w×h`・`stride = w*4`）へ [`ComposedSurface::resize_and_clear`] で
///    再確保し、全画素 0（全透明）へクリアする。既存 `Vec<u8>` 容量を再利用し、必要バイト数が
///    現容量以下なら再割り当てしない（要件 10.3・毎フレーム経路のゼロアロケーション）。
/// 2. `ops` を順に、合成先の全体を矩形として [`overlay_op`] で重ねる。置き場を先に合成先で
///    切るので、画面の外の画素は回らない（負座標・はみ出しのクリップ・要件 6.2）。
///
/// 各命令は合成前に [`crate::method::ComposeMethod::is_implemented`] で dispatch する。M1 の実装対象は
/// [`crate::method::ComposeMethod::Overlay`]（＝SourceOver）のみゆえ、`is_implemented() == false` の
/// 未実装メソッド命令はパニックせず `debug!`（メソッド名・element id 付き）を発してその命令のみ
/// skip し、処理を継続する（要件 8.4・design「Method Registry」Postcondition／Error Handling 表）。
///
/// # 整数専用（要件 10.2）
///
/// 座標は i64、画素演算は u8/u32 のみ。浮動小数（f32/f64）を経路に一切持ち込まない（決定性）。
///
/// # 非パニック（要件 1.4）
///
/// placement None・頁欠落・頁をはみ出す uv・境界外座標はすべて skip／クリップで吸収し、`panic!` しない。
///
/// # 結線（task 7）
///
/// 本関数は task 7 の Composer ファサード（[`crate::Composer::compose_into`]）から
/// [`crate::plan::build_plan`] が返す `(Extent, ops)` を受けて結線・消費される。
pub(crate) fn execute(
    out: &mut ComposedSurface,
    extent: Extent,
    ops: &[BlitOp],
    atlas: &AtlasTable,
) {
    // (1) 合成先を extent へ再確保＋全透明クリア（容量再利用・要件 10.3）。
    out.resize_and_clear(extent.w, extent.h);

    let canvas = Area {
        x0: 0,
        y0: 0,
        x1: extent.w as i64,
        y1: extent.h as i64,
    };
    let stride = out.stride() as usize;
    let dst = out.bytes_mut();

    // (2) 命令を順に転写（画家のアルゴリズム・下層から上層）。
    for op in ops {
        // 未実装メソッド命令はパニックせず debug＋skip して処理を継続する（要件 8.4。組むたびの
        // 知らせなので debug・areka-P0-offline-surface-dump design ShapeIssuesOnce）。M1 の実装対象は
        // `Overlay`（SourceOver）のみ。`BlitOp` は surface id を保持しない（design Contracts）ため、
        // 命令化されている行動可能フィールド＝method（＋element id）をログに載せる。
        if !op.method.is_implemented() {
            tracing::debug!(
                target: "areka_emo_compose",
                method = ?op.method,
                element = op.element.0,
                "未実装の合成メソッド命令をスキップ"
            );
            continue;
        }
        overlay_op(dst, stride, canvas, op, atlas);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::method::{BlendKind, BlendMode, ComposeMethod};
    use crate::normalized::Transform;
    use crate::plan_test_support::{page_1px, px, single_entry_table};
    use areka_emo_atlas::{
        AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, Placement, Point, Rect, SetId, Size,
    };
    use std::sync::Arc;

    /// `ElementId(0)` を (x,y) へ overlay 転写する命令 1 本。
    fn op_at(x: i64, y: i64) -> BlitOp {
        BlitOp {
            element: ElementId(0),
            transform: Transform::translate(x, y),
            method: ComposeMethod::Overlay,
        }
    }

    /// div255 の丸めが四捨五入であること（決定6）。
    #[test]
    fn div255_rounds_to_nearest() {
        assert_eq!(div255(0), 0);
        assert_eq!(div255(255), 1);
        assert_eq!(div255(127), 0); // 127/255 = 0.498 → 0
        assert_eq!(div255(128), 1); // 128/255 = 0.502 → 1
        assert_eq!(div255(255 * 255), 255);
    }

    /// テスト①（受入基準・要件 6.4/10.2）: 既知画素ペアの SourceOver がバイト等価。
    ///
    /// 部分透明 src（α=128）を不透明 dst の上へ overlay 転写し、`dst_c' = src_c + div255(dst_c×127)`
    /// を手計算した期待値とバイト一致することを固定する（div255 の丸めを突く）。
    ///
    /// 手計算（inv = 255 − 128 = 127・div255(v) = (v+127)/255）:
    ///   src = [B20, G40, R60, A128] / dst = [B200, G100, R50, A255]
    ///   out_B = 20 + div255(200×127=25400) = 20 + (25527/255=100) = 120
    ///   out_G = 40 + div255(100×127=12700) = 40 + (12827/255=50)  = 90
    ///   out_R = 60 + div255( 50×127= 6350) = 60 + ( 6477/255=25)  = 85
    ///   out_A = 128 + div255(255×127=32385) = 128 + (32512/255=127) = 255
    #[test]
    fn known_pixel_pair_source_over_byte_exact() {
        // dst 背景を先に置くため、下層＝不透明 dst / 上層＝部分透明 src の 2 命令で組む。
        // 下層 dst 用エントリ（ElementId(0)）・上層 src 用エントリ（ElementId(1)）を持つ table。
        let keys = vec![
            AtlasKey {
                set: SetId(0),
                rel_path: "dst.png".into(),
            },
            AtlasKey {
                set: SetId(0),
                rel_path: "src.png".into(),
            },
        ];
        let entries = vec![
            AtlasEntry {
                original: Size { w: 1, h: 1 },
                placement: Some(Placement {
                    page: 0,
                    uv_rect: Rect {
                        x: 0,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                    trim_offset: Point { x: 0, y: 0 },
                }),
            },
            AtlasEntry {
                original: Size { w: 1, h: 1 },
                placement: Some(Placement {
                    page: 1,
                    uv_rect: Rect {
                        x: 0,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                    trim_offset: Point { x: 0, y: 0 },
                }),
            },
        ];
        let dst_page = page_1px(200, 100, 50, 255);
        let src_page = page_1px(20, 40, 60, 128);
        let atlas = AtlasTable::new(keys, entries, vec![dst_page, src_page]);

        let ops = vec![
            BlitOp {
                element: ElementId(0),
                transform: Transform::translate(0, 0),
                method: ComposeMethod::Overlay,
            },
            BlitOp {
                element: ElementId(1),
                transform: Transform::translate(0, 0),
                method: ComposeMethod::Overlay,
            },
        ];

        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 1, h: 1 }, &ops, &atlas);

        assert_eq!(
            px(&out, 0, 0),
            [120, 90, 85, 255],
            "手計算した SourceOver 期待値とバイト一致"
        );
    }

    /// テスト②（要件 6.4）: 完全不透明 src（α=255）は dst を置換する（255−255=0 → out == src）。
    #[test]
    fn opaque_src_replaces_dst() {
        // 下層に別色不透明 dst、上層に不透明 src を重ねる。
        let keys = vec![
            AtlasKey {
                set: SetId(0),
                rel_path: "dst.png".into(),
            },
            AtlasKey {
                set: SetId(0),
                rel_path: "src.png".into(),
            },
        ];
        let entries = vec![
            AtlasEntry {
                original: Size { w: 1, h: 1 },
                placement: Some(Placement {
                    page: 0,
                    uv_rect: Rect {
                        x: 0,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                    trim_offset: Point { x: 0, y: 0 },
                }),
            },
            AtlasEntry {
                original: Size { w: 1, h: 1 },
                placement: Some(Placement {
                    page: 1,
                    uv_rect: Rect {
                        x: 0,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                    trim_offset: Point { x: 0, y: 0 },
                }),
            },
        ];
        let dst_page = page_1px(11, 22, 33, 255);
        let src_page = page_1px(200, 150, 100, 255);
        let atlas = AtlasTable::new(keys, entries, vec![dst_page, src_page]);

        let ops = vec![
            BlitOp {
                element: ElementId(0),
                transform: Transform::translate(0, 0),
                method: ComposeMethod::Overlay,
            },
            BlitOp {
                element: ElementId(1),
                transform: Transform::translate(0, 0),
                method: ComposeMethod::Overlay,
            },
        ];
        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 1, h: 1 }, &ops, &atlas);
        assert_eq!(
            px(&out, 0, 0),
            [200, 150, 100, 255],
            "不透明 src は dst を完全置換"
        );
    }

    /// テスト③（要件 6.4）: 完全透明 src（α=0・premultiplied ゆえ色も 0）は dst を残す。
    #[test]
    fn transparent_src_leaves_dst() {
        let keys = vec![
            AtlasKey {
                set: SetId(0),
                rel_path: "dst.png".into(),
            },
            AtlasKey {
                set: SetId(0),
                rel_path: "src.png".into(),
            },
        ];
        let entries = vec![
            AtlasEntry {
                original: Size { w: 1, h: 1 },
                placement: Some(Placement {
                    page: 0,
                    uv_rect: Rect {
                        x: 0,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                    trim_offset: Point { x: 0, y: 0 },
                }),
            },
            AtlasEntry {
                original: Size { w: 1, h: 1 },
                placement: Some(Placement {
                    page: 1,
                    uv_rect: Rect {
                        x: 0,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                    trim_offset: Point { x: 0, y: 0 },
                }),
            },
        ];
        let dst_page = page_1px(70, 80, 90, 255);
        let src_page = page_1px(0, 0, 0, 0); // premultiplied 全透明。
        let atlas = AtlasTable::new(keys, entries, vec![dst_page, src_page]);

        let ops = vec![
            BlitOp {
                element: ElementId(0),
                transform: Transform::translate(0, 0),
                method: ComposeMethod::Overlay,
            },
            BlitOp {
                element: ElementId(1),
                transform: Transform::translate(0, 0),
                method: ComposeMethod::Overlay,
            },
        ];
        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 1, h: 1 }, &ops, &atlas);
        // inv = 255 → out_c = 0 + div255(dst_c×255) = dst_c（div255(c×255)=c）。dst そのまま。
        assert_eq!(
            px(&out, 0, 0),
            [70, 80, 90, 255],
            "全透明 src は dst を変えない"
        );
    }

    /// テスト④（要件 6.2）: trim_offset が転写先へ加算され、素の配置位置には現れない。
    ///
    /// 配置 (2,1)＋trim (1,2)＝(3,3) に不透明画素が着弾し、(2,1) は全透明のまま。
    #[test]
    fn trim_offset_shifts_destination() {
        let page = page_1px(10, 20, 30, 255);
        let atlas = single_entry_table(
            page,
            Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            Point { x: 1, y: 2 }, // trim_offset。
        );
        let ops = vec![op_at(2, 1)]; // 配置 (2,1)。着弾は (3,3)。

        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 6, h: 6 }, &ops, &atlas);

        assert_eq!(
            px(&out, 3, 3),
            [10, 20, 30, 255],
            "配置+trim の (3,3) に着弾"
        );
        assert_eq!(px(&out, 2, 1), [0, 0, 0, 0], "素の配置 (2,1) には現れない");
    }

    /// テスト⑤（要件 6.2）: 負座標・はみ出しはクリップされ、範囲内のみ転写・非パニック。
    ///
    /// 2×2 の不透明頁を (−1,−1) へ配置。範囲内に落ちるのは 1 画素 (0,0) のみ（左上角）。
    #[test]
    fn negative_and_overflow_coords_are_clipped() {
        // 2×2 premultiplied 頁（4 画素それぞれ別色・不透明）。
        let bytes = vec![
            1, 2, 3, 255, // (0,0)
            4, 5, 6, 255, // (1,0)
            7, 8, 9, 255, // (0,1)
            10, 11, 12, 255, // (1,1)
        ];
        let page = AtlasPage {
            width: 2,
            height: 2,
            stride: 8,
            bytes: Arc::from(bytes),
        };
        let atlas = single_entry_table(
            page,
            Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            },
            Point { x: 0, y: 0 },
        );
        // (−1,−1) 配置 → 頁 (1,1)=[10,11,12] が out (0,0) に落ち、他 3 画素はクリップ。
        let ops = vec![op_at(-1, -1)];

        let mut out = ComposedSurface::new(0, 0);
        // extent 1×1 なので右下方向へのはみ出しも同時に突く。
        execute(&mut out, Extent { w: 1, h: 1 }, &ops, &atlas);

        assert_eq!(
            px(&out, 0, 0),
            [10, 11, 12, 255],
            "頁の右下画素のみ範囲内・他はクリップ"
        );
    }

    /// テスト⑥（要件 10.3）: 同一 out・同一 extent で 2 回 execute → 容量不変・内容は上書き。
    #[test]
    fn buffer_is_reused_and_cleared_between_calls() {
        let page = page_1px(100, 110, 120, 255);
        let atlas = single_entry_table(
            page,
            Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            Point { x: 0, y: 0 },
        );
        let ops = vec![op_at(0, 0)];

        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 2, h: 2 }, &ops, &atlas);
        let len1 = out.bytes().len();
        let cap1 = out.bytes().as_ptr(); // 再割り当てされなければ同一先頭ポインタ。
        assert_eq!(px(&out, 0, 0), [100, 110, 120, 255]);
        assert_eq!(px(&out, 1, 1), [0, 0, 0, 0], "他画素は全透明");

        // 2 回目: 同一 extent・同一命令。バッファ長不変・再割り当てなし・内容は上書き（累積しない）。
        execute(&mut out, Extent { w: 2, h: 2 }, &ops, &atlas);
        let len2 = out.bytes().len();
        let cap2 = out.bytes().as_ptr();
        assert_eq!(len1, len2, "同一 extent でバッファ長不変（10.3）");
        assert_eq!(
            cap1, cap2,
            "容量再利用ゆえ先頭ポインタ不変（再割り当てなし）"
        );
        // 上書きゆえ 1 回目と同一結果（累積で 2 倍にならない・SourceOver 冪等ではないが不透明置換で確認）。
        assert_eq!(
            px(&out, 0, 0),
            [100, 110, 120, 255],
            "クリア後に上書き（残像なし）"
        );
        assert_eq!(px(&out, 1, 1), [0, 0, 0, 0]);
    }

    /// テスト⑦（要件 6.3）: stride が width*4 を超える padding 付き頁でも正しい行位置を読む。
    ///
    /// 2×2 論理・stride=12（各行 4 バイト padding）。uv(0,0,2,2) を (0,0) へ転写し、
    /// 行 1 の画素が padding をまたいで正しく読まれることを確認する。
    #[test]
    fn padded_stride_source_rows_read_correctly() {
        // 各行: [px0(4B)][px1(4B)][pad(4B)]。行 0 = A/B、行 1 = C/D。
        let bytes = vec![
            // 行 0
            11, 12, 13, 255, // (0,0)=A
            21, 22, 23, 255, // (1,0)=B
            0, 0, 0, 0, // padding
            // 行 1
            31, 32, 33, 255, // (0,1)=C
            41, 42, 43, 255, // (1,1)=D
            0, 0, 0, 0, // padding
        ];
        let page = AtlasPage {
            width: 2,
            height: 2,
            stride: 12,
            bytes: Arc::from(bytes),
        };
        let atlas = single_entry_table(
            page,
            Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            },
            Point { x: 0, y: 0 },
        );
        let ops = vec![op_at(0, 0)];

        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 2, h: 2 }, &ops, &atlas);

        assert_eq!(px(&out, 0, 0), [11, 12, 13, 255], "行0 px0");
        assert_eq!(px(&out, 1, 0), [21, 22, 23, 255], "行0 px1");
        // 行 1 は stride=12 で正しくオフセットされて読まれる（padding を跨がずに px を拾う）。
        assert_eq!(
            px(&out, 0, 1),
            [31, 32, 33, 255],
            "行1 px0（stride padding 越え）"
        );
        assert_eq!(px(&out, 1, 1), [41, 42, 43, 255], "行1 px1");
    }

    /// placement None エントリは防御的に skip される（要件 6.3・非パニック）。
    #[test]
    fn placement_none_op_is_skipped() {
        let keys = vec![AtlasKey {
            set: SetId(0),
            rel_path: "empty.png".into(),
        }];
        let entries = vec![AtlasEntry {
            original: Size { w: 4, h: 4 },
            placement: None, // 全透明（空エントリ）。
        }];
        let atlas = AtlasTable::new(keys, entries, Vec::new());
        let ops = vec![op_at(0, 0)];

        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 2, h: 2 }, &ops, &atlas);
        // 何も転写されず全透明のまま（パニックしない）。
        assert!(
            out.bytes().iter().all(|&b| b == 0),
            "placement None は転写されない"
        );
    }

    /// 非実装メソッド用の BlitOp を組む（`ElementId(0)` を (x,y) へ・任意 method）。
    fn op_with_method(x: i64, y: i64, method: ComposeMethod) -> BlitOp {
        BlitOp {
            element: ElementId(0),
            transform: Transform::translate(x, y),
            method,
        }
    }

    /// テスト（受入基準・要件 8.4）: 非実装メソッド命令はスキップされ、実装済み overlay 命令のみ合成される。
    ///
    /// overlay 命令 1 本＋非実装 `Replace` 命令 1 本を混在させて execute し、
    /// 「overlay 命令のみを含む ops」の結果とバイト等価になることで skip を証明する（非パニック）。
    #[test]
    fn non_implemented_method_op_is_skipped() {
        let page = page_1px(100, 110, 120, 255);
        let atlas = single_entry_table(
            page,
            Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            Point { x: 0, y: 0 },
        );

        // overlay 命令（実装済み・着弾 (0,0)）＋ Replace 命令（未実装・別画素 (1,0) を狙う）を混在。
        // 着弾位置を分けることで「Replace が合成に寄与しない」を画素で直接検出できる（同一画素だと
        // 不透明 src の SourceOver が冪等で skip の有無を判別できない）。
        let mixed = vec![
            op_at(0, 0),                                  // Overlay（実装済み）→ (0,0) に着弾。
            op_with_method(1, 0, ComposeMethod::Replace), // 未実装 → skip・(1,0) に現れない。
        ];
        let mut out_mixed = ComposedSurface::new(0, 0);
        execute(&mut out_mixed, Extent { w: 2, h: 1 }, &mixed, &atlas);

        // overlay のみの ops を別に execute し、バイト等価であることで「Replace が合成に寄与しない」を証明。
        let only_overlay = vec![op_at(0, 0)];
        let mut out_only = ComposedSurface::new(0, 0);
        execute(&mut out_only, Extent { w: 2, h: 1 }, &only_overlay, &atlas);

        assert_eq!(
            out_mixed.bytes(),
            out_only.bytes(),
            "未実装 Replace は合成に寄与せず overlay のみと同一結果（skip・8.4）"
        );
        // 実装済み overlay の画素は確かに合成されている（skip が overlay まで巻き込まないことの確認）。
        assert_eq!(
            px(&out_mixed, 0, 0),
            [100, 110, 120, 255],
            "overlay 命令は合成される"
        );
        // 未実装 Replace の狙った (1,0) は全透明のまま（skip の直接証拠）。
        assert_eq!(
            px(&out_mixed, 1, 0),
            [0, 0, 0, 0],
            "未実装 Replace は着弾しない（skip・8.4）"
        );
    }

    /// テスト（要件 8.4）: 全命令が非実装メソッドなら全 skip → クリア済み（全 0）バッファ・非パニック。
    #[test]
    fn all_non_implemented_ops_produce_cleared_buffer() {
        let page = page_1px(200, 150, 100, 255);
        let atlas = single_entry_table(
            page,
            Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            Point { x: 0, y: 0 },
        );

        // 非実装メソッドのみ（Replace / Blend / Base）。どれも is_implemented()==false。
        let ops = vec![
            op_with_method(0, 0, ComposeMethod::Replace),
            op_with_method(
                0,
                0,
                ComposeMethod::Blend(BlendMode::new(BlendKind::Multiply)),
            ),
            op_with_method(0, 0, ComposeMethod::Base),
        ];
        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 2, h: 2 }, &ops, &atlas);

        assert!(
            out.bytes().iter().all(|&b| b == 0),
            "全命令が非実装 → 何も合成されずクリア済み全透明（非パニック・8.4）"
        );
    }

    /// テスト（回帰・要件 6.4）: 純 overlay 命令列は 6.1 と同一挙動（ガード追加で挙動不変）。
    #[test]
    fn pure_overlay_ops_still_composite() {
        let page = page_1px(11, 22, 33, 255);
        let atlas = single_entry_table(
            page,
            Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            Point { x: 0, y: 0 },
        );
        let ops = vec![op_at(0, 0)];

        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 1, h: 1 }, &ops, &atlas);
        assert_eq!(
            px(&out, 0, 0),
            [11, 22, 33, 255],
            "overlay 命令は従来どおり合成される"
        );
    }

    /// 頁欠落（placement.page が範囲外）でも warn＋skip・非パニック。
    #[test]
    fn missing_page_is_skipped_without_panic() {
        let keys = vec![AtlasKey {
            set: SetId(0),
            rel_path: "e.png".into(),
        }];
        let entries = vec![AtlasEntry {
            original: Size { w: 1, h: 1 },
            placement: Some(Placement {
                page: 5, // 頁が無い（pages は空）。
                uv_rect: Rect {
                    x: 0,
                    y: 0,
                    w: 1,
                    h: 1,
                },
                trim_offset: Point { x: 0, y: 0 },
            }),
        }];
        let atlas = AtlasTable::new(keys, entries, Vec::new());
        let ops = vec![op_at(0, 0)];

        let mut out = ComposedSurface::new(0, 0);
        execute(&mut out, Extent { w: 1, h: 1 }, &ops, &atlas);
        assert!(
            out.bytes().iter().all(|&b| b == 0),
            "頁欠落は skip（非パニック）"
        );
    }
}

#[cfg(test)]
#[path = "blit_transparent_alpha_tests.rs"]
mod transparent_alpha_tests;
