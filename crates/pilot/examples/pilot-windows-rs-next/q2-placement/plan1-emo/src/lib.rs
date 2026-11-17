// 問い 2 の案 1 の emo-text 役: 自分の束縛（emo-text 役のフィルターだけ）の型で描く。
// 入口で wintf 役の型の文脈を受け取り、自分の型へ cast してから描く本体へ渡す。
#[allow(warnings)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;
use windows_core::*;

#[path = "../../shared/draw_body.rs"]
mod draw_body;
pub use bindings::E_FAIL;
pub use draw_body::{TEXT, count_ink};

/// emo-text 役の入口: 型の重なりの受け渡しの手当てはこの cast の 1 行
pub fn draw_line(context: &q2_plan1_wintf::ID2D1DeviceContext, text: &str) -> Result<()> {
    draw_body::draw_line(&context.cast::<ID2D1DeviceContext>()?, text)
}
