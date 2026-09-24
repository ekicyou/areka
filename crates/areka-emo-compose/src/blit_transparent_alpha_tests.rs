//! 転写した画素の α が、全透明にクリアされた合成先へそのまま運ばれることを固定するテスト。
//!
//! `blit.rs` の中のテストと合成の golden 群は、不透明な下地の上に重ねるか全画素不透明の絵しか
//! 使わないので、転写した画素の α を捨てて不透明で書いても赤にならない（spec
//! `areka-P0-keycolor-clickthrough-coverage` の完了記録 3.2）。本テストは α=0・中間の α・不透明
//! の 3 画素を並べた絵を、クリア直後の合成先へ 1 回だけ転写し、合成先の α が元の α のまま
//! （0 は 0・中間は中間・255 は 255）であることを主張する。下地が全透明なので乗算済み
//! SourceOver の結果は元の画素そのもの（`src_c + div255(0 × (255 − src_a)) = src_c`）になる。

use super::*;
use crate::method::ComposeMethod;
use crate::normalized::Transform;
use areka_emo_atlas::{
    AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, Placement, Point, Rect, SetId, Size,
};
use std::sync::Arc;

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
