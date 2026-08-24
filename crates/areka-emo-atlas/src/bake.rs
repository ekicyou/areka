//! Baker: 頁バッファ確保・stride 決定・トリム矩形 blit（`bake_pages`）。
//!
//! 要件 **R4.3 / R6.3**。
//!
//! packing で算出した配置座標（[`crate::pack::PackOutput`]）に従い、premultiplied
//! BGRA の頁バッファ（[`crate::table::AtlasPage`]）を確保し stride を決定して、
//! トリム済み矩形（[`crate::trim::Trimmed`]）を各頁へ blit する。空エントリ
//! （全透明）は上流（Trimmer）が既に `pack.entries` から除外しているため、Baker は
//! `pack.entries` に載る矩形のみを blit する（空エントリ統合は `bake` 入口＝task 3.1）。
//!
//! **設計決定 D8**: blit は premultiplied を素通しし、画素変換を一切行わない。
//! **Critical Issue 3**（packing と焼付の分離）: Packer は座標のみ（`page`/`uv_rect`）を
//! 算出し、Baker が頁バッファを確保して画素を blit する。Baker は Packer 出力の座標を
//! 改変しない。padding（UV 非包含）に相当する余白画素は初期値 0（完全透明 premultiplied）
//! のまま保たれ、隣接画像との滲みが生じない。

use std::collections::HashMap;
use std::sync::Arc;

use crate::pack::PackOutput;
use crate::table::{AtlasPage, ElementId, Placement};
use crate::trim::Trimmed;

/// 焼付器（頁確保・blit・座標不変）。
pub struct Baker;

impl Baker {
    /// 座標（`PackOutput`）＋トリム画素から頁バッファ群を確保・blit する。
    ///
    /// 返り値は `AtlasPage` 群と、各キーの `Placement`（`page`/`uv_rect`/`trim_offset`）。
    ///
    /// # `page_size` 引数について（承認済みシグネチャ調整）
    /// 設計の Baker Service Interface は `bake_pages(&self, items, pack)` で `page_size` を
    /// 持たないが、Baker は `page_size × page_size` の正方頁を確保する必要があり
    /// （設計 Responsibilities: 「`page_size` 正方・`stride = page_size*4`」）、`PackOutput` は
    /// `page_size` を運ばない。よって `page_size: u32` 引数を追加する（`bake` 入口＝task 3.1
    /// が `PackConfig.page_size` から供給）。`PackOutput` を改変せず変更を bake.rs 内に閉じる。
    ///
    /// # Preconditions
    /// - `pack.entries` の各 `id` は `items` に存在する。
    /// - 各 `uv_rect` は `page_size` 内に収まる（`x+w <= page_size`・`y+h <= page_size`）。
    /// - 各 `Trimmed.pbgra` は tightly packed（`stride == size.w*4`・premultiplied BGRA）。
    ///
    /// # Postconditions
    /// - 各 `AtlasPage.bytes.len() == stride * height`（`stride == page_size*4`）。
    /// - blit は premultiplied を保存（無変換・D8）。
    ///
    /// # Invariants
    /// - Packer 出力の座標（`page`/`uv_rect`）を改変しない（Critical Issue 3）。
    /// - `uv_rect` 外の画素は初期値 0（透明）のまま（bleed 防止・5.2/5.3）。
    pub fn bake_pages(
        &self,
        items: &[(ElementId, Trimmed)],
        pack: &PackOutput,
        page_size: u32,
    ) -> (Vec<AtlasPage>, Vec<(ElementId, Placement)>) {
        let stride = page_size * 4;
        let page_len = (stride * page_size) as usize;

        // 1. 頁数分の可変バッファを確保（初期値 0＝完全透明 premultiplied・6.3）。
        //    blit 後に Arc<[u8]> へ変換する。
        let mut page_bufs: Vec<Vec<u8>> =
            (0..pack.page_count).map(|_| vec![0u8; page_len]).collect();

        // 2. id→Trimmed 逆引き（blit と trim_offset 参照用）。
        let lookup: HashMap<ElementId, &Trimmed> = items.iter().map(|(id, t)| (*id, t)).collect();

        // 3. 各 PackedEntry を対応頁へ blit（トリム後矩形のみ・premultiplied 素通し・4.3/D8）。
        let mut placements: Vec<(ElementId, Placement)> = Vec::with_capacity(pack.entries.len());
        for entry in &pack.entries {
            let trimmed = match lookup.get(&entry.id) {
                Some(t) => *t,
                None => {
                    // Precondition 違反（pack.entries のキーは items に存在するはず）。
                    // 「ログ無し失敗経路の禁止」に従い error! でログし、当該矩形をスキップ。
                    tracing::error!(
                        element_id = entry.id.0,
                        page = entry.page,
                        "bake: pack entry id not found in items; skipping blit (precondition violated)"
                    );
                    continue;
                }
            };

            let uv = entry.uv_rect;
            let page_buf = match page_bufs.get_mut(entry.page as usize) {
                Some(buf) => buf,
                None => {
                    tracing::error!(
                        element_id = entry.id.0,
                        page = entry.page,
                        page_count = pack.page_count,
                        "bake: pack entry page out of range; skipping blit (precondition violated)"
                    );
                    continue;
                }
            };

            // 行ごとに tightly-packed 元 stride → 頁 stride へ verbatim コピー（無変換・D8）。
            // 元: src_offset = row * trimmed.stride（tightly packed・== uv.w*4）。
            // 先: dest_offset = (uv.y + row) * stride + uv.x * 4（頁 stride）。
            let src_stride = trimmed.stride as usize;
            let dest_row_bytes = (uv.w * 4) as usize;
            for row in 0..uv.h {
                let src_off = (row as usize) * src_stride;
                let dest_off = ((uv.y + row) * stride + uv.x * 4) as usize;
                // in-range スライスへ copy_from_slice（premultiplied バイトを素通し）。
                page_buf[dest_off..dest_off + dest_row_bytes]
                    .copy_from_slice(&trimmed.pbgra[src_off..src_off + dest_row_bytes]);
            }

            // 4. Placement を収集（page/uv_rect は Packer 由来・trim_offset は Trimmed 由来）。
            placements.push((
                entry.id,
                Placement {
                    page: entry.page,
                    uv_rect: uv,
                    trim_offset: trimmed.trim_offset,
                },
            ));
        }

        // 5. 可変バッファを AtlasPage（Arc 共有・stride 明示）へ確定。
        let pages: Vec<AtlasPage> = page_bufs
            .into_iter()
            .map(|buf| AtlasPage {
                width: page_size,
                height: page_size,
                stride,
                bytes: Arc::from(buf),
            })
            .collect();

        (pages, placements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{PackConfig, PackOutput, PackedEntry, Packer};
    use crate::table::{Point, Rect, Size};

    /// 既知の premultiplied バイトで塗った tightly-packed Trimmed を手組みする。
    /// 全画素を単一の BGRA 値で塗るので、blit 後の照合が単純になる。
    fn trimmed_filled(w: u32, h: u32, offset: Point, bgra: [u8; 4]) -> Trimmed {
        let stride = w * 4;
        let mut pbgra = Vec::with_capacity((stride * h) as usize);
        for _ in 0..(w * h) {
            pbgra.extend_from_slice(&bgra);
        }
        Trimmed {
            trim_offset: offset,
            size: Size { w, h },
            pbgra,
            stride,
        }
    }

    /// 行・列で値が変わる（ずれ検出用の）premultiplied パターンで Trimmed を手組みする。
    /// 各画素 = [x as u8, y as u8, tag, 255]（α=255 で premultiplied 前提と矛盾しない）。
    fn trimmed_pattern(w: u32, h: u32, offset: Point, tag: u8) -> Trimmed {
        let stride = w * 4;
        let mut pbgra = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                pbgra.extend_from_slice(&[x as u8, y as u8, tag, 255]);
            }
        }
        Trimmed {
            trim_offset: offset,
            size: Size { w, h },
            pbgra,
            stride,
        }
    }

    /// 頁バッファの (px_x, px_y) の 4byte を読む。
    fn read_px(page: &AtlasPage, x: u32, y: u32) -> [u8; 4] {
        let off = (y * page.stride + x * 4) as usize;
        [
            page.bytes[off],
            page.bytes[off + 1],
            page.bytes[off + 2],
            page.bytes[off + 3],
        ]
    }

    /// 頁の uv_rect 領域が元 Trimmed.pbgra と行ごとに一致することを検証。
    fn assert_region_matches(page: &AtlasPage, uv: &Rect, t: &Trimmed) {
        let row_bytes = (uv.w * 4) as usize;
        for row in 0..uv.h {
            let src_off = (row * t.stride) as usize;
            let dest_off = ((uv.y + row) * page.stride + uv.x * 4) as usize;
            assert_eq!(
                &page.bytes[dest_off..dest_off + row_bytes],
                &t.pbgra[src_off..src_off + row_bytes],
                "row {} mismatch at uv {:?}",
                row,
                uv
            );
        }
    }

    /// 1（4.3/D8）: 2 矩形を既知 uv へ手組み配置し、頁画素が元 pbgra と byte 単位で一致。
    /// premultiplied を verbatim 転写する（無変換）ことを証明する。
    #[test]
    fn blit_verbatim_correctness() {
        let page_size = 32;
        // 距離検出しやすいよう行列で変化するパターンを 2 枚。
        let t0 = trimmed_pattern(5, 4, Point { x: 3, y: 7 }, 0xA0);
        let t1 = trimmed_pattern(6, 3, Point { x: 1, y: 2 }, 0xB0);
        let items = vec![(ElementId(0), t0.clone()), (ElementId(1), t1.clone())];

        // 手組み PackOutput: 非重複な既知 uv（page 0）。
        let uv0 = Rect {
            x: 2,
            y: 2,
            w: 5,
            h: 4,
        };
        let uv1 = Rect {
            x: 12,
            y: 8,
            w: 6,
            h: 3,
        };
        let pack = PackOutput {
            page_count: 1,
            entries: vec![
                PackedEntry {
                    id: ElementId(0),
                    page: 0,
                    uv_rect: uv0,
                },
                PackedEntry {
                    id: ElementId(1),
                    page: 0,
                    uv_rect: uv1,
                },
            ],
        };

        let (pages, _placements) = Baker.bake_pages(&items, &pack, page_size);
        assert_eq!(pages.len(), 1);
        let page = &pages[0];

        // 各 uv 領域が元 pbgra と byte-for-byte 一致（premultiplied verbatim・行/頁stride補正）。
        assert_region_matches(page, &uv0, &t0);
        assert_region_matches(page, &uv1, &t1);
    }

    /// 2（4.3）: blit した uv_rect の外側画素（padding gap・隅）は 0（完全透明）のまま。
    /// 初期化 0 の保存＝bleed なしを証明する。
    #[test]
    fn padding_and_gaps_stay_transparent() {
        let page_size = 32;
        let t0 = trimmed_filled(4, 4, Point { x: 0, y: 0 }, [10, 20, 30, 255]);
        let t1 = trimmed_filled(4, 4, Point { x: 0, y: 0 }, [40, 50, 60, 255]);
        let items = vec![(ElementId(0), t0), (ElementId(1), t1)];

        // uv0 と uv1 の間に 1px 以上の隙間（gap）を空ける。
        let uv0 = Rect {
            x: 2,
            y: 2,
            w: 4,
            h: 4,
        }; // 覆う x:2..6
        let uv1 = Rect {
            x: 10,
            y: 2,
            w: 4,
            h: 4,
        }; // 覆う x:10..14
        let pack = PackOutput {
            page_count: 1,
            entries: vec![
                PackedEntry {
                    id: ElementId(0),
                    page: 0,
                    uv_rect: uv0,
                },
                PackedEntry {
                    id: ElementId(1),
                    page: 0,
                    uv_rect: uv1,
                },
            ],
        };

        let (pages, _) = Baker.bake_pages(&items, &pack, page_size);
        let page = &pages[0];

        // gap 画素（x=7,y=3）は透明のまま。
        assert_eq!(
            read_px(page, 7, 3),
            [0, 0, 0, 0],
            "gap pixel must stay transparent"
        );
        // 隅（0,0）と（page_size-1, page_size-1）も透明。
        assert_eq!(
            read_px(page, 0, 0),
            [0, 0, 0, 0],
            "top-left corner transparent"
        );
        assert_eq!(
            read_px(page, page_size - 1, page_size - 1),
            [0, 0, 0, 0],
            "bottom-right corner transparent"
        );
        // uv0 の直上（x=2,y=1）＝矩形の 1px 外も透明（bleed なし）。
        assert_eq!(
            read_px(page, 2, 1),
            [0, 0, 0, 0],
            "pixel just above uv0 transparent"
        );
        // uv0 内部は塗られている（対照）。
        assert_eq!(
            read_px(page, 2, 2),
            [10, 20, 30, 255],
            "uv0 interior blitted"
        );
    }

    /// 3（6.3）: page_count==2 で両頁にエントリ。返り Vec 長 2・各頁 width/height/stride/len。
    /// page 1 への blit が page[0] を汚さないことも確認。
    #[test]
    fn multi_page_allocation() {
        let page_size = 16;
        let stride = page_size * 4;
        let t0 = trimmed_filled(3, 3, Point { x: 0, y: 0 }, [11, 22, 33, 255]);
        let t1 = trimmed_filled(3, 3, Point { x: 0, y: 0 }, [99, 88, 77, 255]);
        let items = vec![(ElementId(0), t0), (ElementId(1), t1.clone())];

        let uv0 = Rect {
            x: 1,
            y: 1,
            w: 3,
            h: 3,
        };
        let uv1 = Rect {
            x: 5,
            y: 5,
            w: 3,
            h: 3,
        };
        let pack = PackOutput {
            page_count: 2,
            entries: vec![
                PackedEntry {
                    id: ElementId(0),
                    page: 0,
                    uv_rect: uv0,
                },
                PackedEntry {
                    id: ElementId(1),
                    page: 1,
                    uv_rect: uv1,
                },
            ],
        };

        let (pages, _) = Baker.bake_pages(&items, &pack, page_size);
        assert_eq!(pages.len(), 2, "two pages allocated");
        for p in &pages {
            assert_eq!(p.width, page_size);
            assert_eq!(p.height, page_size);
            assert_eq!(p.stride, stride);
            assert_eq!(p.bytes.len(), (stride * page_size) as usize);
        }

        // page 1 の uv1 に t1 が焼かれている。
        assert_region_matches(&pages[1], &uv1, &t1);
        // page 0 の uv1 位置（5,5）は透明のまま（page1 の blit が page0 に漏れていない）。
        assert_eq!(
            read_px(&pages[0], 5, 5),
            [0, 0, 0, 0],
            "page 0 not touched by page 1 blit"
        );
    }

    /// 4（4.2/6.1）: 返る (id, Placement) が page/uv_rect（Packer 由来）・trim_offset
    /// （Trimmed 由来）を正しく運ぶ。
    #[test]
    fn placement_mapping() {
        let page_size = 32;
        let off0 = Point { x: 10, y: 12 };
        let off1 = Point { x: 3, y: 4 };
        let t0 = trimmed_filled(4, 4, off0, [1, 2, 3, 255]);
        let t1 = trimmed_filled(5, 2, off1, [4, 5, 6, 255]);
        let items = vec![(ElementId(7), t0), (ElementId(9), t1)];

        let uv0 = Rect {
            x: 2,
            y: 2,
            w: 4,
            h: 4,
        };
        let uv1 = Rect {
            x: 10,
            y: 10,
            w: 5,
            h: 2,
        };
        let pack = PackOutput {
            page_count: 1,
            entries: vec![
                PackedEntry {
                    id: ElementId(7),
                    page: 0,
                    uv_rect: uv0,
                },
                PackedEntry {
                    id: ElementId(9),
                    page: 0,
                    uv_rect: uv1,
                },
            ],
        };

        let (_pages, placements) = Baker.bake_pages(&items, &pack, page_size);
        assert_eq!(placements.len(), 2);

        let p7 = placements
            .iter()
            .find(|(id, _)| *id == ElementId(7))
            .map(|(_, p)| p)
            .unwrap();
        assert_eq!(p7.page, 0);
        assert_eq!(p7.uv_rect, uv0, "uv_rect from PackOutput");
        assert_eq!(p7.trim_offset, off0, "trim_offset from Trimmed");

        let p9 = placements
            .iter()
            .find(|(id, _)| *id == ElementId(9))
            .map(|(_, p)| p)
            .unwrap();
        assert_eq!(p9.page, 0);
        assert_eq!(p9.uv_rect, uv1);
        assert_eq!(p9.trim_offset, off1);
    }

    /// 5（6.3）: stride == page_size*4 を明示的に検証。
    #[test]
    fn stride_is_page_size_times_four() {
        let page_size = 48;
        let items: Vec<(ElementId, Trimmed)> = vec![];
        let pack = PackOutput {
            page_count: 1,
            entries: vec![],
        };
        let (pages, placements) = Baker.bake_pages(&items, &pack, page_size);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].stride, page_size * 4);
        assert_eq!(pages[0].bytes.len(), (page_size * 4 * page_size) as usize);
        // 空頁は全透明。
        assert!(
            pages[0].bytes.iter().all(|&b| b == 0),
            "empty page fully transparent"
        );
        assert!(placements.is_empty());
    }

    /// 6（統合・end-to-end blit sanity）: Packer::pack → bake_pages。
    /// 各 placed id の頁領域が元 pbgra と一致することを確認。
    #[test]
    fn integration_pack_then_bake() {
        let page_size = 256;
        let cfg = PackConfig {
            page_size,
            padding: 1,
        };
        let t0 = trimmed_pattern(20, 15, Point { x: 1, y: 1 }, 0x10);
        let t1 = trimmed_pattern(30, 10, Point { x: 2, y: 3 }, 0x20);
        let t2 = trimmed_pattern(12, 25, Point { x: 5, y: 6 }, 0x30);
        let items = vec![
            (ElementId(0), t0.clone()),
            (ElementId(1), t1.clone()),
            (ElementId(2), t2.clone()),
        ];

        let pack = Packer.pack(&items, cfg);
        let (pages, placements) = Baker.bake_pages(&items, &pack, page_size);

        assert_eq!(pages.len(), pack.page_count as usize);
        assert_eq!(placements.len(), pack.entries.len());

        // 各 placed id の頁領域が元 pbgra と一致（end-to-end verbatim blit）。
        let by_id: HashMap<ElementId, &Trimmed> = items.iter().map(|(id, t)| (*id, t)).collect();
        for entry in &pack.entries {
            let page = &pages[entry.page as usize];
            let t = by_id[&entry.id];
            assert_region_matches(page, &entry.uv_rect, t);
        }
    }
}
