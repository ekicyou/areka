mod command;
mod components;
mod dpi;
pub mod monitor;
/// DPI／拡大率遷移の観測チャネル（専用 target・既定 OFF・レコード語彙の純関数）。
///
/// 判定側（areka の `transition_judge`）とサインオフ手順書が語彙を直接参照するため、
/// `pub` で crate 外へ開いてある。
pub mod transition_diag;
mod window_handle;
mod window_pos;
pub(crate) mod window_system;
/// グループ単位の重なり（受け口・観測・純判断・記録の唯一の出口）。
mod zorder_group;
mod zorder_pair;
/// 記録の行を組む純関数だけの層（マクロを含まない＝出力先を分裂させない）。
mod zorder_pair_diag;
mod zorder_pair_establish;
mod zorder_pair_maintain;
mod zorder_pair_sink;

pub use command::*;
pub use components::*;
pub use dpi::*;
pub use window_handle::*;
pub use window_pos::*;
pub use zorder_group::*;
pub use zorder_pair::*;
pub use zorder_pair_establish::*;
pub use zorder_pair_maintain::*;
pub use zorder_pair_sink::*;
