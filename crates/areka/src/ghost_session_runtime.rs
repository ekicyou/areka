//! 実行系に結びつく窓側の結線（areka-P0-mcp-reload 13.4）。
//!
//! `boot_wired` の boot 後の塊を純移動した置き場。実行系（[`GhostRuntime`]）の送出端を窓の側の
//! 状態へ差し直す。窓の側だけの結線（`wire_balloon_choice`）は含めない。

use areka_ghost::GhostRuntime;
use bevy_ecs::world::World;

use crate::boot_resolve::{BalloonDecision, GhostDecision};
use crate::input_events;
use crate::menu;

/// 実行系に結びつく窓側の結線をやり直す（`boot_wired` の boot 後の塊の純移動＋`ChoiceForwarder`）。
/// 移した塊の中の呼び順は移動前と同じ。`wire_balloon_choice`（窓の側）は含めない。
pub(crate) fn rewire_runtime_consumers(
    world: &mut World,
    runtime: &GhostRuntime,
    ghost: &GhostDecision,
    balloon: &BalloonDecision,
) {
    // マウス配信資源を World へ結線（task 3.1・design「main.rs＋wire_mouse_input」・
    // DD-IE-9）: kanade Sender クローンで MouseWiring（NonSend・Presenter）を挿入する。
    // 挿入は wire_emo2_boot 成功後＝Emo2Wiring 挿入済みゆえ presenter 経由の region 解決が
    // 成立する（Emo2Wiring 挿入と同位置・同型・self-gating）。窓へのハンドラ登録は task 3.2。
    let sender = runtime.kanade().clone();
    input_events::wire_mouse_input(world, sender);
    // 右クリックメニューの結線（areka-P0-popup-menu-minimal）: 終了の項目が上の入力の結線を使う。
    menu::wire_menu(world, runtime.kanade().clone());
    // 「ゴースト」枠の登記（要件 1.12）: `wire_menu` が登記の口を新品にするので、起こすたびにやり直す。
    menu::ghost_frame::register(world);
    // 「シェル」「バルーン」枠の登記（shell-balloon-switch 要件 1.5・11.9）: 同じ理由で起こすたびにやり直す。
    menu::shell_frame::register(world);
    menu::balloon_frame::register(world);
    // 「インストール」枠の登記（ghost-install 要件 1.8）: 同じ理由で起こすたびにやり直す。
    menu::install_frame::register(world);
    // 「置き場を追加…」「置き場を外す」枠の登記（baseware-root-list 要件 6.1・7.1）: 同じ理由で起こすたびにやり直す。
    menu::roots_frame::register(world);
    // 「ネットワーク更新」枠の登記（network-update 要件 1.2）: 同じ理由で起こすたびにやり直す。
    menu::update_frame::register(world);
    // 「シェル位置初期化」枠の登記（move-canon 要件 8.1）: 同じ理由で起こすたびにやり直す。
    menu::reset_pos_frame::register(world);
    // 置換語 %lastghostname・%lastobjectname の載せ直し（記憶の置き場は起こすたびに新しくなる）。
    crate::install::names::reseed(world, runtime);
    // 位置永続の World 結線（task 6.2・design C4/C5・要件 1.9）: wire_mouse_input とは
    // 別行の additive 挿入。ゴースト窓を保持する同一 World（`wire_mouse_input` と同経路）へ
    // sylphya publisher clone を持つ PersistWiring（NonSend）を差し、DragEnd→persist_entries の
    // write-through 導管を確立する。続けて起動成功時の記憶を書く（baseware-root-layout 要件 3）。
    crate::on_boot_ok(world, runtime, ghost, balloon);
    // 選択確定通知の受信結線（areka-P0-choice-select-events task 5・design C1 ChoiceDrain・
    // Req1.1/1.2/1.5/1.6/3.7）: 呼び手が続けて `wire_balloon_choice` で挿入する `ChoiceSelectionInbox`
    // （precondition・どちらも挿入だけで互いを読まない）を毎フレーム drain し dispatcher の inbox 経由で kanade へ全件転送する系（登録は
    // `register_systems`）の送り口を置く。送り先が dispatcher なのは、押下の選択を再生の
    // `ChoiceWaiting` と同じ inbox に並べて追い越させないため（areka-P0-balloon-text-bugs 件 1・案 (b)）。
    // 位置・様式は `wire_mouse_input`（上方の同 boot スロット）と同型——Sender クローンを持つ NonSend 資源挿入。
    input_events::choice_drain::wire_choice_drain(world, runtime.dispatcher().clone());
}
