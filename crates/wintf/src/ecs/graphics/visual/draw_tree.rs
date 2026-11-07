use super::draw::command::DrawCommand;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::*;

/// 描画面の持ち主のうち、節の木を持つ根の印と、平らにした列の置き場。
///
/// 入れる・外すのは描画の木の係だけ（根に節が 1 つでも在れば入れ、無くなれば外す）。
/// `Changed<DrawTree>` が「木を描き直す」合図。
#[derive(Component, Default, Debug)]
pub struct DrawTree {
    /// 平らにした列（画家の順）。
    pub commands: Vec<DrawCommand>,
    /// (0,0)-(1,1) の矩形の図形。回転した矩形の切り抜きを図形へ上げるとき共有する。
    /// 装置の失いで空に戻り、次の組み直しで工場から作り直す。
    pub(crate) unit_rect: Option<ID2D1Geometry>,
}

// SAFETY 条件: windows-rs の COM スマートポインタは自動では Send/Sync にならない。
// 中の COM は 2 種類で、どちらも D2D1_FACTORY_TYPE_MULTI_THREADED の工場（GraphicsCore::new →
// create_d2d_factory）の系列に属する: `ID2D1Geometry`（unit_rect・切り抜きの図形）は工場の、
// `ID2D1CommandList`（描く 1 件の複製）はその工場の CreateDevice で作った装置の持ち物。同じ工場の
// 系列への COM 呼び出しは D2D が内部で直列化するため、跨スレッド移動・参照共有とも安全
// （ClipShape・VisualDrawContent と同じ根拠）。
unsafe impl Send for DrawTree {}
unsafe impl Sync for DrawTree {}
