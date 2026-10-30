//! `blit_rect.rs` の兄弟のテスト: 矩形の重ね直しが `blit::execute`（いちから）とバイト単位で
//! 同じこと、矩形の求め方、いちからへ倒れる 3 条件、長い連なり、hot path の静的検査。
//!
//! 見本の絵は半透明を含む決定的な模様（premultiplied・`c <= a`）で、SourceOver の丸めまで突く。

use std::sync::Arc;

use areka_emo_atlas::{
    AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, Placement, Point, Rect, SetId, Size,
};

use super::{Recompose, execute_reusing};
use crate::blit;
use crate::composed::ComposedSurface;
use crate::method::ComposeMethod;
use crate::normalized::Transform;
use crate::plan::{BlitOp, Extent};

/// 見本の絵 1 枚（大きさ・切り詰めのずれ・模様の種）。`None` は全透明（placement 無し）。
type Part = Option<(u32, u32, (i32, i32), u32)>;

/// 半透明を含む決定的な premultiplied BGRA の頁（α は 0 と 255 も出る）。
fn pattern_page(w: u32, h: u32, seed: u32) -> AtlasPage {
    let mut bytes = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let a = (x * 37 + y * 91 + seed * 53) % 256;
            let a = if (x + y + seed).is_multiple_of(7) {
                255
            } else {
                a
            };
            for k in 0..3 {
                bytes.push((a * ((seed * 29 + k * 71 + x * 13) % 256) / 255) as u8);
            }
            bytes.push(a as u8);
        }
    }
    AtlasPage {
        width: w,
        height: h,
        stride: w * 4,
        bytes: Arc::from(bytes),
    }
}

/// 見本の絵ごとに 1 頁の `AtlasTable`（`ElementId(i)` が `parts[i]`）。
fn atlas(parts: &[Part]) -> AtlasTable {
    let mut keys = Vec::new();
    let mut entries = Vec::new();
    let mut pages = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        keys.push(AtlasKey {
            set: SetId(0),
            rel_path: format!("e{i}.png"),
        });
        let placement = part.map(|(w, h, (tx, ty), _)| Placement {
            page: i as u32,
            uv_rect: Rect { x: 0, y: 0, w, h },
            trim_offset: Point { x: tx, y: ty },
        });
        entries.push(AtlasEntry {
            original: Size { w: 1, h: 1 },
            placement,
        });
        let (w, h, seed) = part.map_or((1, 1, 0), |(w, h, _, s)| (w, h, s));
        pages.push(pattern_page(w, h, seed));
    }
    AtlasTable::new(keys, entries, pages)
}

fn op(e: u32, x: i64, y: i64) -> BlitOp {
    BlitOp {
        element: ElementId(e),
        transform: Transform::translate(x, y),
        method: ComposeMethod::Overlay,
    }
}

fn fresh(extent: Extent, ops: &[BlitOp], atlas: &AtlasTable) -> ComposedSurface {
    let mut out = ComposedSurface::new(0, 0);
    blit::execute(&mut out, extent, ops, atlas);
    out
}

/// 前の命令列 `prev`（その絵はいちから）から今 `cur` を下敷き形で重ね、いちからと同じことを確かめて報告を返す。
fn reuse_once(extent: Extent, prev: &[BlitOp], cur: &[BlitOp], atlas: &AtlasTable) -> Recompose {
    let base = fresh(extent, prev, atlas);
    let base_before = base.bytes().to_vec();
    let mut out = ComposedSurface::new(0, 0);
    let r = execute_reusing(&mut out, extent, cur, Some((&base, prev)), atlas);
    assert_eq!(out.width(), extent.w);
    assert_eq!(out.height(), extent.h);
    assert!(
        out.bytes() == fresh(extent, cur, atlas).bytes(),
        "下敷き形の絵がいちからの合成とバイト単位で同じ（{r:?}）"
    );
    assert_eq!(base.bytes(), &base_before[..], "下敷きは読むだけ");
    r
}

const EXT: Extent = Extent { w: 64, h: 64 };

/// 背景 0・小物 1〜2・動く絵 3〜12（12×12・切り詰めのずれがコマごとに違う）・全透明 13。
fn sample_atlas() -> AtlasTable {
    let mut parts: Vec<Part> = vec![
        Some((64, 64, (0, 0), 1)),
        Some((20, 16, (2, 1), 2)),
        Some((18, 22, (0, 3), 3)),
    ];
    for i in 0..10u32 {
        parts.push(Some((12, 12, ((i % 4) as i32, (i % 3) as i32), 10 + i)));
    }
    parts.push(None);
    atlas(&parts)
}

/// 下敷きなしはいちから（`Full`）で、`blit::execute` とバイト単位で同じ。
#[test]
fn no_underlay_is_full_and_byte_identical() {
    let a = sample_atlas();
    let ops = [op(0, 0, 0), op(1, 5, 7), op(3, 30, 30)];
    let mut out = ComposedSurface::new(3, 3);
    assert_eq!(
        execute_reusing(&mut out, EXT, &ops, None, &a),
        Recompose::Full
    );
    assert_eq!(out.bytes(), fresh(EXT, &ops, &a).bytes());
}

/// 矩形が全面になる道（前は空の命令列＝全透明の絵）でも、重ね方の式がいちからとバイト単位で同じ。
#[test]
fn full_rect_path_is_byte_identical() {
    let a = sample_atlas();
    let ops = [
        op(0, 0, 0),
        op(1, 5, 7),
        op(2, 40, 30),
        op(3, 10, 10),
        op(4, 11, 12),
        op(5, 58, 58),
    ];
    let r = reuse_once(EXT, &[], &ops, &a);
    assert_eq!(
        r,
        Recompose::Partial {
            x: 0,
            y: 0,
            w: 64,
            h: 64
        }
    );
}

/// 動く層 1 枚の位置替えでは、矩形が前と今の置き場の和になる。
#[test]
fn one_moving_layer_rect_is_union_of_old_and_new() {
    let a = sample_atlas();
    // 動く絵 3 は 12×12・切り詰めのずれ (0,0)。
    let prev = [op(0, 0, 0), op(1, 2, 2), op(3, 5, 6)];
    let cur = [op(0, 0, 0), op(1, 2, 2), op(3, 20, 10)];
    assert_eq!(
        reuse_once(EXT, &prev, &cur, &a),
        Recompose::Partial {
            x: 5,
            y: 6,
            w: 27,
            h: 16
        }
    );
}

/// 命令列が前と同じなら幅 0 の矩形（写しただけ）。
#[test]
fn identical_ops_report_zero_rect() {
    let a = sample_atlas();
    let ops = [op(0, 0, 0), op(3, 20, 10)];
    assert_eq!(
        reuse_once(EXT, &ops, &ops, &a),
        Recompose::Partial {
            x: 0,
            y: 0,
            w: 0,
            h: 0
        }
    );
}

/// 着せ替えの重ねが足される・外れる（命令の数が替わる）と、その置き場だけが矩形になる。
#[test]
fn dressup_added_and_removed() {
    let a = sample_atlas();
    let without = [op(0, 0, 0), op(2, 40, 30)];
    let with = [op(0, 0, 0), op(1, 10, 12), op(2, 40, 30)];
    // 絵 1 は 20×16・切り詰めのずれ (2,1) → (12,13) から 20×16。
    let want = Recompose::Partial {
        x: 12,
        y: 13,
        w: 20,
        h: 16,
    };
    assert_eq!(reuse_once(EXT, &without, &with, &a), want);
    assert_eq!(reuse_once(EXT, &with, &without, &a), want);
    // 末尾に足す・末尾から外す。
    let top = [op(0, 0, 0), op(2, 40, 30), op(1, 10, 12)];
    assert_eq!(reuse_once(EXT, &without, &top, &a), want);
    assert_eq!(reuse_once(EXT, &top, &without, &a), want);
}

/// 同じ位置でもコマごとに切り詰めのずれが違えば、両方の置き場が矩形に入る。
#[test]
fn per_frame_trim_offset_is_in_rect() {
    let a = sample_atlas();
    // 絵 3 のずれ (0,0)、絵 6 のずれ (3,0)、絵 8 のずれ (1,2)。
    let prev = [op(0, 0, 0), op(3, 20, 20)];
    let cur = [op(0, 0, 0), op(6, 20, 20)];
    assert_eq!(
        reuse_once(EXT, &prev, &cur, &a),
        Recompose::Partial {
            x: 20,
            y: 20,
            w: 15,
            h: 12
        }
    );
    let cur2 = [op(0, 0, 0), op(8, 20, 20)];
    assert_eq!(
        reuse_once(EXT, &prev, &cur2, &a),
        Recompose::Partial {
            x: 20,
            y: 20,
            w: 13,
            h: 14
        }
    );
}

/// 合成の絵の外へはみ出す位置は絵の範囲で切られ、外に出きった位置どうしは幅 0 になる。
#[test]
fn out_of_bounds_placement_is_clipped() {
    let a = sample_atlas();
    let prev = [op(0, 0, 0), op(3, -5, -7)];
    let cur = [op(0, 0, 0), op(3, 58, 60)];
    assert_eq!(
        reuse_once(EXT, &prev, &cur, &a),
        Recompose::Partial {
            x: 0,
            y: 0,
            w: 64,
            h: 64
        }
    );
    let prev = [op(0, 0, 0), op(3, -30, 5)];
    let cur = [op(0, 0, 0), op(3, 100, 5)];
    assert_eq!(
        reuse_once(EXT, &prev, &cur, &a),
        Recompose::Partial {
            x: 0,
            y: 0,
            w: 0,
            h: 0
        }
    );
    // 全透明（placement 無し）の命令は置き場が空。
    let cur = [op(0, 0, 0), op(13, 10, 10)];
    let prev = [op(0, 0, 0)];
    assert_eq!(
        reuse_once(EXT, &prev, &cur, &a),
        Recompose::Partial {
            x: 0,
            y: 0,
            w: 0,
            h: 0
        }
    );
}

/// 倒れる 3 条件（下敷きなし・外形の違い・`Overlay` でない命令）はどれも `Full` で、絵はいちからと同じ。
#[test]
fn fallbacks_report_full() {
    let a = sample_atlas();
    let prev = [op(0, 0, 0), op(3, 5, 5)];
    let cur = [op(0, 0, 0), op(3, 9, 5)];
    let want = fresh(EXT, &cur, &a);

    let mut out = ComposedSurface::new(0, 0);
    assert_eq!(
        execute_reusing(&mut out, EXT, &cur, None, &a),
        Recompose::Full
    );
    assert_eq!(out.bytes(), want.bytes());

    let small = fresh(Extent { w: 63, h: 64 }, &prev, &a);
    let mut out = ComposedSurface::new(0, 0);
    assert_eq!(
        execute_reusing(&mut out, EXT, &cur, Some((&small, &prev)), &a),
        Recompose::Full
    );
    assert_eq!(out.bytes(), want.bytes());

    let mut odd = prev.clone();
    odd[0].method = ComposeMethod::OverlayFast;
    let odd_img = fresh(EXT, &odd, &a);
    let mut out = ComposedSurface::new(0, 0);
    assert_eq!(
        execute_reusing(&mut out, EXT, &cur, Some((&odd_img, &odd)), &a),
        Recompose::Full,
        "前の命令列に Overlay でない命令"
    );
    assert_eq!(out.bytes(), want.bytes());

    let mut odd_cur = cur.clone();
    odd_cur[1].method = ComposeMethod::OverlayFast;
    let base = fresh(EXT, &prev, &a);
    let mut out = ComposedSurface::new(0, 0);
    assert_eq!(
        execute_reusing(&mut out, EXT, &odd_cur, Some((&base, &prev)), &a),
        Recompose::Full,
        "今の命令列に Overlay でない命令"
    );
    assert_eq!(out.bytes(), fresh(EXT, &odd_cur, &a).bytes());
}

/// 動く絵 `k` 本目のコマ `f` の命令（隣り合う 3 本が近くを回る・ずれの違う絵を順に使う）。
fn mover(k: u32, f: u32) -> BlitOp {
    let e = 3 + (f + k * 3) % 10;
    let x = 14 + (f * 3 % 17) as i64 + k as i64 * 4;
    let y = 16 + (f * 5 % 13) as i64 - k as i64 * 3;
    op(e, x, y)
}

/// 下敷きを渡し続けて `n` コマ重ね、毎回いちからと同じこと・2 コマ目から矩形の報告・
/// 確保が増えないこと（席を入れ替えて使い回す）を確かめる。
fn run_frames(a: &AtlasTable, n: u32, frame: impl Fn(u32) -> Vec<BlitOp>) {
    let mut prev_img = ComposedSurface::new(0, 0);
    let mut prev_ops = frame(0);
    assert_eq!(
        execute_reusing(&mut prev_img, EXT, &prev_ops, None, a),
        Recompose::Full
    );
    let mut out = ComposedSurface::new(EXT.w, EXT.h);
    let cap = out.bytes_capacity();
    for t in 1..n {
        let ops = frame(t);
        let r = execute_reusing(&mut out, EXT, &ops, Some((&prev_img, &prev_ops)), a);
        let Recompose::Partial { w, h, .. } = r else {
            panic!("コマ {t}: 下敷きがあるのに {r:?}");
        };
        assert!(
            w * h < EXT.w * EXT.h / 2,
            "コマ {t}: 矩形が動く絵の近くに収まる（{r:?}）"
        );
        assert!(
            out.bytes() == fresh(EXT, &ops, a).bytes(),
            "コマ {t}: いちからの合成とバイト単位で同じ"
        );
        std::mem::swap(&mut out, &mut prev_img);
        prev_ops = ops;
    }
    assert_eq!(out.bytes_capacity(), cap, "外形が同じあいだ確保しない");
    assert_eq!(prev_img.bytes_capacity(), cap, "外形が同じあいだ確保しない");
}

/// 面 1002 と同じ並び（変わらない 3 枚の上に隣り合う動く 3 枚・40/48/40 コマ）を 240 コマ。
#[test]
fn surface1002_order_240_frames() {
    let a = sample_atlas();
    run_frames(&a, 240, |t| {
        vec![
            op(0, 0, 0),
            op(1, 3, 4),
            op(2, 40, 36),
            mover(0, t % 40),
            mover(1, t % 48),
            mover(2, t % 40),
        ]
    });
}

/// あいだに変わらない層を挟み、上にも変わらない層が乗る見本を 240 コマ。
#[test]
fn unchanged_layer_between_movers_240_frames() {
    let a = sample_atlas();
    run_frames(&a, 240, |t| {
        vec![
            op(0, 0, 0),
            mover(0, t % 40),
            op(1, 18, 20),
            mover(1, t % 48),
            op(2, 20, 15),
        ]
    });
}

/// 非テストの領域（`#[cfg(test)]` より前）に名前の引き直し（`.resolve(`）が無い。
#[test]
fn hot_path_has_no_name_relookup() {
    let src = include_str!("blit_rect.rs");
    let non_test = src.split("#[cfg(test)]").next().unwrap_or(src);
    assert!(non_test.contains("fn execute_reusing"), "検査の的が在る");
    assert!(
        !non_test.contains(".resolve("),
        "blit_rect.rs の hot path は atlas.resolve を呼ばない（要件 4.3）"
    );
}
