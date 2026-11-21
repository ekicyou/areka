//! `areka_runtime::frame` の代わりの親（子のテストの宣言だけ）。

use areka_emo_present::EmoPresenter;
use areka_emo_present::PresentCommand;
use areka_emo_text::actor::TextLayerRuntime;
use areka_placement::spawn::GhostWindows;
use areka_runtime::assets::BootAssets;
use areka_runtime::frame::*;
use areka_runtime::move_cue::PositionCue;
use areka_runtime::target_map::balloon_target;
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use wintf::ecs::DPI;
use wintf::ecs::FrameTime;
use wintf::ecs::SizeI;
// 中で記録を出す私有の関数は渡し口から同じ名前で引く。
use crate::sample_test_support::{emo2_balloon_root, emo2_root};
use areka_emo_text::state::TextLayerConfig;
use areka_runtime::assets::build_boot_assets;
use areka_runtime::frame::drain_resnap::test_support::{finalize_chain_once_with, resnap_with};
use areka_runtime::frame::test_support::{gpu_attach_world, zero_clock};
use areka_runtime::talk_lifecycle::TalkLifecycleSignal;
use std::sync::mpsc;

mod status_report;
// シェル・バルーンの差し替えの相。
mod switch;
// 常に手前の状態と取り出しの相。
mod window_state;

// move drain の置き場の檻。ハーネスに乗るので **x64 のみ**で接続する（常時テストに x86 を用いない）。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_move_order_tests.rs"]
mod move_order_tests;
// 常に手前の状態と台本の組の寿命（ゴーストの読み直し・切り替え・SHIORI だけの読み直し）。
#[cfg(test)]
#[path = "frame_stay_on_top_lifecycle_tests.rs"]
mod stay_on_top_lifecycle_tests;
// ghost の読み直しで控えた面を装着の相が出し直す檻。
#[path = "frame_attach_carry_tests.rs"]
mod carry_tests;
// 窓を全部閉じたあとの次の窓の一式の確定。
#[cfg(test)]
#[path = "frame_chain_finalize_restart_tests.rs"]
mod chain_finalize_restart_tests;
// 並べ直しから記憶まで。共有の部品と、それを使うテストファイル（1,000 行の上限に合わせて分けてある）。
#[cfg(test)]
#[path = "frame_chain_finalize_persist_nowrite_tests.rs"]
mod chain_finalize_persist_nowrite_tests;
#[cfg(test)]
#[path = "frame_chain_finalize_persist_restart_tests.rs"]
mod chain_finalize_persist_restart_tests;
#[cfg(test)]
#[path = "frame_chain_finalize_persist_test_support.rs"]
mod chain_finalize_persist_test_support;
#[cfg(test)]
#[path = "frame_chain_finalize_persist_tests.rs"]
mod chain_finalize_persist_tests;
// 起動・作り直しの並べ直しが記憶から戻した組を覚えた印で隣り合わせる檻。
// ハーネスに乗るので **x64 のみ**で接続する。
#[cfg(all(test, target_pointer_width = "64"))]
#[path = "frame_chain_finalize_remembered_tests.rs"]
mod chain_finalize_remembered_tests;
// 相順そのものの検査。実 emo2 資産＋実 GPU で
// `frame_system` をそのまま駆動し、相を並べ替えたときにだけ壊れる性質を主張する。
#[cfg(test)]
#[path = "frame_visibility_integration_tests.rs"]
mod visibility_integration_tests;
