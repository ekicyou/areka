//! 転写した画素の α が、全透明にクリアされた合成先へそのまま運ばれることを固定するテスト。
//!
//! `blit.rs` の中の他のテストと合成の golden 群は、不透明な下地の上に重ねるか全画素不透明の絵しか
//! 使わないので、それらだけでは転写した画素の α を捨てて不透明で書いても赤にならない。本テストは α=0・中間の α・不透明
//! の 3 画素を並べた絵を、クリア直後の合成先へ 1 回だけ転写し、合成先の α が元の α のまま
//! （0 は 0・中間は中間・255 は 255）であることを主張する。下地が全透明なので乗算済み
//! SourceOver の結果は元の画素そのもの（`src_c + div255(0 × (255 − src_a)) = src_c`）になる。
//!
//! あわせて、頁の中の切り出しの位置（uv の x・y が 0 でない）から源の画素を読むことも固定する。

use super::*;
use crate::method::ComposeMethod;
use crate::normalized::Transform;
use areka_emo_atlas::{AtlasPage, ElementId, Point, Rect};
use std::sync::Arc;

use crate::plan_test_support::{px, single_entry_table};

#[test]
fn copied_alpha_is_kept_on_a_cleared_output() {
    // 乗算済み BGRA。左から α=0（抜かれた画素）・α=128（色も α 以下に乗算済み）・α=255。
    let src: [[u8; 4]; 3] = [[0, 0, 0, 0], [20, 40, 60, 128], [10, 20, 30, 255]];
    let page = AtlasPage {
        width: 3,
        height: 1,
        stride: 12,
        bytes: Arc::from(src.concat()),
    };
    let uv = Rect {
        x: 0,
        y: 0,
        w: 3,
        h: 1,
    };
    let atlas = single_entry_table(page, uv, Point { x: 0, y: 0 });
    let ops = [BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    execute(&mut out, Extent { w: 3, h: 1 }, &ops, &atlas);

    let got: Vec<[u8; 4]> = (0..3).map(|x| px(&out, x, 0)).collect();
    assert_eq!(
        got,
        src.to_vec(),
        "全透明の合成先へ転写した画素が元の画素（α を含む）と違う。左から α=0・128・255 のはず"
    );
}

// ── blit uv_rect.x/y != 0（源矩形オフセット・記録の檻 log_firing_tests から移した・指摘 103）──

/// 要件 6.1/6.4: エントリの uv_rect.x・y がどちらも 0 でない（頁の中の切り出し）とき、頁の原点でなく
/// **uv の位置**の源の画素を転写する。3×3 の頁は画素ごとに色が違う（B＝80×x・G＝80×y）ので、x か y の
/// どちらかを読み落とせば (2,2) と違う色になる。
#[test]
fn blit_reads_source_at_nonzero_uv_offset() {
    let bytes: Vec<u8> = (0..3u8)
        .flat_map(|y| (0..3u8).flat_map(move |x| [80 * x, 80 * y, 50, 255]))
        .collect();
    let page = AtlasPage {
        width: 3,
        height: 3,
        stride: 12,
        bytes: Arc::from(bytes),
    };
    // uv (2,2) を 1×1 で切り出す。trim_offset=0 ゆえ着弾は配置 (0,0)。
    let atlas = single_entry_table(
        page,
        Rect {
            x: 2,
            y: 2,
            w: 1,
            h: 1,
        },
        Point { x: 0, y: 0 },
    );
    let ops = [BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    execute(&mut out, Extent { w: 1, h: 1 }, &ops, &atlas);

    assert_eq!(
        px(&out, 0, 0),
        [160, 160, 50, 255],
        "uv (2,2) の画素が転写される（頁の原点や x・y の片方だけの位置でない）",
    );
}
