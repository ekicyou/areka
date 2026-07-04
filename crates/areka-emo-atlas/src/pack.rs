//! packing 座標算出（`rectangle-pack` 結線・padding ラップ・複数頁・決定性）。
//!
//! 設計決定 **D7**（要件 **R5**）。
//!
//! トリム済み矩形群を `rectangle-pack` へ渡して静的・決定的・複数頁（bin）に配置する。
//! padding=1px は非内蔵ゆえ矩形を全周 +1px 拡張して渡す自前ラップ（UV は非包含）。
//! 頁サイズ 2048（超過矩形は自頁）。golden 入力ソート順＝正規化パス昇順で決定性を固定。
//! 本段は座標を算出するのみで画素は焼かない（焼付は [`crate::bake`]）。
//!
//! Critical Issue 3（packing と焼付の分離）: 本 `Packer` は **座標のみ**（`page`/`uv_rect`）
//! を返し、頁バッファの確保も画素 blit も一切行わない（`Trimmed.pbgra` を読まない）。
//! 焼付は Baker（task 2.6）の責務。

use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, RectToInsert,
    TargetBin,
};
use std::collections::BTreeMap;

use crate::table::{ElementId, Rect, Size};
use crate::trim::Trimmed;

/// packing 設定（頁サイズ・全周 padding）。既定は D7（2048 / 1px）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackConfig {
    /// 正方頁の一辺（px）。既定 2048（D7）。
    pub page_size: u32,
    /// 各矩形の全周に確保する余白（px）。既定 1（D7・bleed 防止）。
    pub padding: u32,
}

impl Default for PackConfig {
    /// D7 既定: 頁サイズ 2048・padding 1px。
    fn default() -> Self {
        Self {
            page_size: 2048,
            padding: 1,
        }
    }
}

/// 1 矩形の配置結果（座標のみ・画素なし）。
///
/// `uv_rect` は **padding 非包含**の実トリム矩形（5.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedEntry {
    /// 対象 element。
    pub id: ElementId,
    /// 配置頁番号（0-based・密・6.1）。
    pub page: u32,
    /// 頁内 UV（padding 非包含の実矩形・5.3）。
    pub uv_rect: Rect,
}

/// packing 出力（頁数＋各矩形座標）。**画素バッファは持たない**（焼付は Baker）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackOutput {
    /// 使用頁数（密・0..page_count）。
    pub page_count: u32,
    /// 各矩形の配置（ElementId 昇順・golden 安定）。
    pub entries: Vec<PackedEntry>,
}

/// packer（静的・決定的・複数頁・padding ラップ）。
pub struct Packer;

impl Packer {
    /// トリム矩形群を頁へ決定的に配置し、座標（page/uv_rect）のみを返す。
    ///
    /// - `items` はマニフェスト採番順（ElementId 昇順）で渡す想定だが、防御的に内部でも
    ///   ElementId 昇順へソートしてから配置する（決定性・5.5/D7）。
    /// - padding は矩形を全周 +`cfg.padding` 拡張して packer へ登録し、配置後の UV から
    ///   padding を差し引く（UV は非包含＝実トリム矩形・5.2/5.3/D7）。
    /// - 全矩形が単頁に収まらなければ bin（頁）を増やして再試行し複数頁へ分割する（5.4）。
    /// - 各入力 ElementId は正確に 1 エントリを得る（5.6・重複排除は上流マニフェスト）。
    ///
    /// **座標のみ**を返し、頁バッファ確保も画素 blit も行わない（Critical Issue 3）。
    /// `Trimmed.pbgra` は読まない（`size` のみ使用）。
    pub fn pack(&self, items: &[(ElementId, Trimmed)], cfg: PackConfig) -> PackOutput {
        // 空入力 → 頁ゼロ・エントリ空。
        if items.is_empty() {
            return PackOutput {
                page_count: 0,
                entries: Vec::new(),
            };
        }

        // 防御的決定性: ElementId 昇順へソートし、id→実寸（padding 非包含）を控える。
        let mut sorted: Vec<(ElementId, Size)> =
            items.iter().map(|(id, t)| (*id, t.size)).collect();
        sorted.sort_by_key(|(id, _)| id.0);

        let padding = cfg.padding;

        // 過大矩形ガード: padded 寸が page_size を超える矩形は如何なる bin にも収まらない。
        // emo2 の element 矩形はすべて < 2048 ゆえ実運用では発火しないが、無限ループ回避と
        // 「ログ無し失敗経路の禁止」（steering）に従い、error! でログして entries から除外する。
        // （自頁に載せることも不可能ゆえ座標は算出できない。上流スコープ外の劣化ケース。）
        let (placeable, oversized): (Vec<_>, Vec<_>) = sorted.into_iter().partition(|(_, sz)| {
            sz.w.saturating_add(2 * padding) <= cfg.page_size
                && sz.h.saturating_add(2 * padding) <= cfg.page_size
        });
        for (id, sz) in &oversized {
            tracing::error!(
                element_id = id.0,
                w = sz.w,
                h = sz.h,
                page_size = cfg.page_size,
                padding = padding,
                "pack: padded rect exceeds page size; excluding from atlas (out-of-scope degenerate)"
            );
        }

        if placeable.is_empty() {
            return PackOutput {
                page_count: 0,
                entries: Vec::new(),
            };
        }

        // id→実寸マップ（配置後に UV を実寸で復元するため）。
        let size_of: BTreeMap<u32, Size> =
            placeable.iter().map(|(id, sz)| (id.0, *sz)).collect();

        // padding を全周に足した寸で packer へ登録（RectId = ElementId.0・depth=1 の 2D）。
        let mut grouped: GroupedRectsToPlace<u32, u32> = GroupedRectsToPlace::new();
        for (id, sz) in &placeable {
            let pw = sz.w + 2 * padding;
            let ph = sz.h + 2 * padding;
            grouped.push_rect(id.0, None, RectToInsert::new(pw, ph, 1));
        }

        // bin（頁）数を 1 から増やしながら all-or-nothing の pack_rects を再試行（5.4）。
        // cap = placeable.len()（最悪 1 矩形 1 頁）で無限ループを封じる。
        let cap = placeable.len();
        let mut locations = None;
        for num_bins in 1..=cap {
            let mut target_bins: BTreeMap<usize, TargetBin> = BTreeMap::new();
            for bin_id in 0..num_bins {
                target_bins.insert(bin_id, TargetBin::new(cfg.page_size, cfg.page_size, 1));
            }
            match pack_rects(
                &grouped,
                &mut target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(ok) => {
                    // (RectId=ElementId.0) → (BinId, PackedLocation) を自前構造へ写す。
                    let placed: Vec<(u32, usize, u32, u32)> = ok
                        .packed_locations()
                        .iter()
                        .map(|(rect_id, (bin_id, loc))| (*rect_id, *bin_id, loc.x(), loc.y()))
                        .collect();
                    locations = Some(placed);
                    break;
                }
                Err(_) => continue, // NotEnoughBinSpace → bin を 1 つ増やして再試行。
            }
        }

        // cap 回試しても収まらない場合（過大矩形は上で除外済ゆえ通常起きない）。
        let placed = match locations {
            Some(p) => p,
            None => {
                tracing::error!(
                    count = placeable.len(),
                    cap = cap,
                    "pack: failed to place all rects within cap bins; returning empty (unexpected)"
                );
                return PackOutput {
                    page_count: 0,
                    entries: Vec::new(),
                };
            }
        };

        // 実際に使用された BinId を昇順に集めて密な 0-based 頁番号へ remap（空頁ギャップ回避）。
        let mut used_bins: Vec<usize> = placed.iter().map(|(_, bin, _, _)| *bin).collect();
        used_bins.sort_unstable();
        used_bins.dedup();
        let page_of: BTreeMap<usize, u32> = used_bins
            .iter()
            .enumerate()
            .map(|(page, bin)| (*bin, page as u32))
            .collect();
        let page_count = used_bins.len() as u32;

        // UV は padding を差し引いた実矩形で復元（5.3）。
        let mut entries: Vec<PackedEntry> = placed
            .into_iter()
            .map(|(rect_id, bin, lx, ly)| {
                let sz = size_of[&rect_id];
                PackedEntry {
                    id: ElementId(rect_id),
                    page: page_of[&bin],
                    uv_rect: Rect {
                        x: lx + padding,
                        y: ly + padding,
                        w: sz.w,
                        h: sz.h,
                    },
                }
            })
            .collect();

        // 出力は ElementId 昇順（golden 安定・7）。
        entries.sort_by_key(|e| e.id.0);

        PackOutput {
            page_count,
            entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Point;

    /// 指定寸の Trimmed を手組みする（Packer は pbgra を読まないので最小 placeholder）。
    fn trimmed(w: u32, h: u32) -> Trimmed {
        let stride = w * 4;
        Trimmed {
            trim_offset: Point { x: 0, y: 0 },
            size: Size { w, h },
            // Packer は画素を読まない。長さ整合だけ最小限保つ（空でも可だが self-consistent に）。
            pbgra: vec![0u8; (stride * h) as usize],
            stride,
        }
    }

    /// (id, w, h) の並びから items を組む。
    fn items(specs: &[(u32, u32, u32)]) -> Vec<(ElementId, Trimmed)> {
        specs
            .iter()
            .map(|(id, w, h)| (ElementId(*id), trimmed(*w, *h)))
            .collect()
    }

    /// 2 矩形が重なるか（頁は呼び出し側で同一を保証）。
    fn overlaps(a: &Rect, b: &Rect) -> bool {
        let ax2 = a.x + a.w;
        let ay2 = a.y + a.h;
        let bx2 = b.x + b.w;
        let by2 = b.y + b.h;
        a.x < bx2 && b.x < ax2 && a.y < by2 && b.y < ay2
    }

    /// padding を全周に足した矩形（bleed 検証用）。
    fn expand(r: &Rect, pad: u32) -> Rect {
        Rect {
            x: r.x.saturating_sub(pad),
            y: r.y.saturating_sub(pad),
            w: r.w + 2 * pad,
            h: r.h + 2 * pad,
        }
    }

    /// 5.1: 単頁に収まる複数矩形は互いに重ならない。
    #[test]
    fn no_overlap_on_single_page() {
        let cfg = PackConfig {
            page_size: 256,
            padding: 1,
        };
        let out = Packer.pack(&items(&[(0, 30, 20), (1, 40, 40), (2, 10, 60), (3, 25, 25)]), cfg);
        assert_eq!(out.entries.len(), 4);
        // 全て同一頁に収まる想定（256×256 に小矩形 4 つ）。
        for e in &out.entries {
            assert!(e.page < out.page_count);
        }
        // 同一頁内で総当り非重複（5.1）。
        for i in 0..out.entries.len() {
            for j in (i + 1)..out.entries.len() {
                let a = &out.entries[i];
                let b = &out.entries[j];
                if a.page == b.page {
                    assert!(
                        !overlaps(&a.uv_rect, &b.uv_rect),
                        "uv_rects overlap: {:?} vs {:?}",
                        a.uv_rect,
                        b.uv_rect
                    );
                }
            }
        }
    }

    /// 5.2: padding 分離（bleed-free）。UV を padding で全周拡張しても同一頁内で非重複。
    #[test]
    fn padded_boxes_do_not_overlap() {
        let pad = 1u32;
        let cfg = PackConfig {
            page_size: 256,
            padding: pad,
        };
        let out = Packer.pack(
            &items(&[(0, 20, 20), (1, 20, 20), (2, 20, 20), (3, 20, 20), (4, 20, 20)]),
            cfg,
        );
        for i in 0..out.entries.len() {
            for j in (i + 1)..out.entries.len() {
                let a = &out.entries[i];
                let b = &out.entries[j];
                if a.page == b.page {
                    let ea = expand(&a.uv_rect, pad);
                    let eb = expand(&b.uv_rect, pad);
                    assert!(
                        !overlaps(&ea, &eb),
                        "padded boxes overlap (bleed): {:?} vs {:?}",
                        ea,
                        eb
                    );
                }
            }
        }
    }

    /// 5.3: UV は padding を含まず実トリム寸に一致する。
    #[test]
    fn uv_excludes_padding() {
        let cfg = PackConfig {
            page_size: 256,
            padding: 2,
        };
        let specs = [(0u32, 30u32, 20u32), (1, 40, 15), (2, 12, 50)];
        let out = Packer.pack(&items(&specs), cfg);
        for e in &out.entries {
            let (_, w, h) = specs.iter().find(|(id, _, _)| *id == e.id.0).copied().unwrap();
            assert_eq!(e.uv_rect.w, w, "uv width == real trimmed width");
            assert_eq!(e.uv_rect.h, h, "uv height == real trimmed height");
        }
    }

    /// 5.4: 単頁に収まらない多数矩形は複数頁へ分割され、全 id が有効頁を持つ。
    #[test]
    fn multi_page_split() {
        let cfg = PackConfig {
            page_size: 64,
            padding: 1,
        };
        // 40×40（padded 42×42）は 64×64 頁に 1 つしか入らない → 4 個で 4 頁。
        let specs: Vec<(u32, u32, u32)> = (0..4).map(|i| (i, 40, 40)).collect();
        let out = Packer.pack(&items(&specs), cfg);
        assert!(out.page_count > 1, "expected multi-page, got {}", out.page_count);
        assert_eq!(out.entries.len(), 4, "every id has an entry");
        for e in &out.entries {
            assert!(e.page < out.page_count, "page index in range");
        }
        // 各頁は 42×42 を 1 つしか持てない → 4 頁になるはず。
        assert_eq!(out.page_count, 4);
    }

    /// 5.5: 決定性。同一入力二回で同一 PackOutput。
    #[test]
    fn deterministic_same_input() {
        let cfg = PackConfig {
            page_size: 128,
            padding: 1,
        };
        let specs = [(0u32, 30u32, 40u32), (1, 20, 20), (2, 50, 10), (3, 15, 35), (4, 22, 22)];
        let out1 = Packer.pack(&items(&specs), cfg);
        let out2 = Packer.pack(&items(&specs), cfg);
        assert_eq!(out1, out2, "same input → identical output");
    }

    /// 5.5: 入力順が違っても（Packer が内部で ElementId ソート）同一結果。
    #[test]
    fn deterministic_regardless_of_input_order() {
        let cfg = PackConfig {
            page_size: 128,
            padding: 1,
        };
        let sorted = [(0u32, 30u32, 40u32), (1, 20, 20), (2, 50, 10), (3, 15, 35)];
        let shuffled = [(3u32, 15u32, 35u32), (0, 30, 40), (2, 50, 10), (1, 20, 20)];
        let out_sorted = Packer.pack(&items(&sorted), cfg);
        let out_shuffled = Packer.pack(&items(&shuffled), cfg);
        assert_eq!(out_sorted, out_shuffled, "input order must not affect output");
    }

    /// 5.6: 各入力 ElementId は正確に 1 エントリ（重複なし・欠落なし）。
    #[test]
    fn one_entry_per_id() {
        let cfg = PackConfig {
            page_size: 128,
            padding: 1,
        };
        let specs = [(5u32, 10u32, 10u32), (2, 12, 12), (9, 8, 8), (0, 20, 20)];
        let out = Packer.pack(&items(&specs), cfg);
        assert_eq!(out.entries.len(), specs.len());
        let mut ids: Vec<u32> = out.entries.iter().map(|e| e.id.0).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![0, 2, 5, 9], "exactly the input ids, sorted, no dup/missing");
        // 出力は ElementId 昇順（7）。
        let out_ids: Vec<u32> = out.entries.iter().map(|e| e.id.0).collect();
        assert_eq!(out_ids, vec![0, 2, 5, 9], "entries sorted by ElementId ascending");
    }

    /// 7（空入力）: 頁ゼロ・エントリ空。
    #[test]
    fn empty_input() {
        let out = Packer.pack(&[], PackConfig::default());
        assert_eq!(out.page_count, 0);
        assert!(out.entries.is_empty());
    }

    /// 8（既定値）: PackConfig::default() は D7 の 2048 / 1。
    #[test]
    fn default_config_is_d7() {
        let cfg = PackConfig::default();
        assert_eq!(cfg.page_size, 2048);
        assert_eq!(cfg.padding, 1);
    }

    /// padding=0 でも UV は実寸・非重複（境界）。
    #[test]
    fn zero_padding_still_non_overlapping() {
        let cfg = PackConfig {
            page_size: 128,
            padding: 0,
        };
        let out = Packer.pack(&items(&[(0, 30, 30), (1, 30, 30), (2, 30, 30)]), cfg);
        assert_eq!(out.entries.len(), 3);
        for e in &out.entries {
            assert_eq!(e.uv_rect.w, 30);
            assert_eq!(e.uv_rect.h, 30);
        }
        for i in 0..out.entries.len() {
            for j in (i + 1)..out.entries.len() {
                let a = &out.entries[i];
                let b = &out.entries[j];
                if a.page == b.page {
                    assert!(!overlaps(&a.uv_rect, &b.uv_rect));
                }
            }
        }
    }
}
