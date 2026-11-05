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
use areka_emo_atlas::{
    AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, Placement, Point, Rect, SetId, Size,
};
use std::sync::Arc;

use crate::plan_test_support::single_entry_table;

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
    let atlas = AtlasTable::new(
        vec![AtlasKey {
            set: SetId(0),
            rel_path: "e.png".into(),
        }],
        vec![AtlasEntry {
            original: Size { w: 3, h: 1 },
            placement: Some(Placement {
                page: 0,
                uv_rect: uv,
                trim_offset: Point { x: 0, y: 0 },
            }),
        }],
        vec![page],
    );
    let ops = [BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    execute(&mut out, Extent { w: 3, h: 1 }, &ops, &atlas);

    let got: Vec<[u8; 4]> = out
        .bytes()
        .chunks_exact(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect();
    assert_eq!(
        got,
        src.to_vec(),
        "全透明の合成先へ転写した画素が元の画素（α を含む）と違う。左から α=0・128・255 のはず"
    );
}

// ── blit uv_rect.x/y != 0（源矩形オフセット・記録の檻 log_firing_tests から移した・指摘 103）──

/// 要件 6.1/6.4: エントリの uv_rect.x/y が非ゼロ（頁内サブ矩形オフセット）でも、頁原点でなく
/// **uv オフセット位置**の源画素が転写される。
///
/// 3×1 の頁に [赤, 緑, 青] を横並びに置き、uv_rect=(x:2,y:0,w:1,h:1) で **青のみ**を切り出す
/// エントリを組む。合成結果 (0,0) が青（頁原点の赤でない）であればサンプル位置が uv.x で
/// 正しくオフセットされている証左。手計算バイトで厳密一致を突く。
#[test]
fn blit_reads_source_at_nonzero_uv_offset() {
    // 3×1 premultiplied BGRA 頁: (0,0)=赤 [0,0,255] / (1,0)=緑 [0,255,0] / (2,0)=青 [255,0,0]。
    let bytes = vec![
        0, 0, 255, 255, // (0,0) 赤
        0, 255, 0, 255, // (1,0) 緑
        255, 0, 0, 255, // (2,0) 青
    ];
    let page = AtlasPage {
        width: 3,
        height: 1,
        stride: 12,
        bytes: Arc::from(bytes),
    };
    // uv.x=2（青の列）を 1×1 で切り出す。trim_offset=0 ゆえ着弾は配置 (0,0)。
    let atlas = single_entry_table(
        page,
        Rect {
            x: 2,
            y: 0,
            w: 1,
            h: 1,
        },
        Point { x: 0, y: 0 },
    );
    let ops = vec![BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    crate::blit::execute(&mut out, crate::plan::Extent { w: 1, h: 1 }, &ops, &atlas);

    // 頁原点(赤)でなく uv.x=2 の青が着弾する（源矩形オフセット読み取りの直接証拠）。
    let i = 0usize;
    let b = out.bytes();
    assert_eq!(
        [b[i], b[i + 1], b[i + 2], b[i + 3]],
        [255, 0, 0, 255],
        "uv.x=2 の青が転写される（頁原点の赤でない）",
    );
}

/// uv_rect.y も非ゼロで効くことを縦方向で突く（3 行頁の 3 行目を切り出す）。
#[test]
fn blit_reads_source_at_nonzero_uv_y_offset() {
    // 1×3 premultiplied BGRA 頁（縦並び）: 行0=赤 / 行1=緑 / 行2=青。
    let bytes = vec![
        0, 0, 255, 255, // (0,0) 赤
        0, 255, 0, 255, // (0,1) 緑
        255, 0, 0, 255, // (0,2) 青
    ];
    let page = AtlasPage {
        width: 1,
        height: 3,
        stride: 4,
        bytes: Arc::from(bytes),
    };
    // uv.y=2（青の行）を 1×1 で切り出す。
    let atlas = single_entry_table(
        page,
        Rect {
            x: 0,
            y: 2,
            w: 1,
            h: 1,
        },
        Point { x: 0, y: 0 },
    );
    let ops = vec![BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    crate::blit::execute(&mut out, crate::plan::Extent { w: 1, h: 1 }, &ops, &atlas);

    let b = out.bytes();
    assert_eq!(
        [b[0], b[1], b[2], b[3]],
        [255, 0, 0, 255],
        "uv.y=2 の青が転写される（頁原点の赤でない）",
    );
}
