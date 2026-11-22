//! 本体だけのモジュールの系の登録と、集めた層から本体へ向かう口の束。

use areka_ghost::GhostRuntime;
use areka_ghost::sink::{BootCueSink, DiscardSink};
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

/// 本体だけのモジュールの系の登録と、集めた層から本体へ向かう口（本体の `BODY_SYSTEMS` が中身を持つ）。
#[derive(Resource, Clone, Copy)]
pub struct BodySystems {
    /// 入力の取り出しの系の間（`user_break` の後・`balloon` の前）で呼ぶ。
    pub menu_poll: fn(&mut World),
    /// 入力の系の後・`FrameFinalize` への足し込みの前で呼ぶ。
    pub after_input: fn(&mut World),
    /// 窓を開いた直後、キャラ窓の入力の差し込みの後で呼ぶ（メニューの解放の受け手）。
    pub after_char_pointer: fn(&mut World),
    /// 窓を開いた直後、バルーン窓の入力の差し込みの後で呼ぶ（投げ込み → 位置初期化の近道の受け手）。
    pub after_balloon_pointer: fn(&mut World),
    /// 入力の結線の後・起動成功の記憶の前で呼ぶ（メニューの結線・枠の登記 8 本・置換語の載せ直し）。
    pub after_runtime_wired: fn(&mut World, &GhostRuntime),
    /// 切替を終えたとき（真は切替先が定常に入った）に更新の窓口へ知らせる。
    pub on_switch_end: fn(&mut World, bool),
    /// 定常に届いたとき、インストール → 更新の順に窓口へ知らせる。
    pub on_steady: fn(&mut World),
    /// 降ろして全窓を閉じた直後・起こす前に、起動中のゴーストへ入れる一周を回す。
    pub overwrite_between: fn(&mut World),
    /// 更新の窓口が忙しいか。
    pub update_busy: fn(&World) -> bool,
    /// インストールの窓口が忙しいか。
    pub install_busy: fn(&World) -> bool,
    /// 受け口の列の 9・10 番目（インストール・更新の順）。
    pub cue_sinks: fn(&World) -> [Box<dyn BootCueSink>; 2],
}

impl BodySystems {
    /// 据えられた束。本番で無ければ `error!(body_systems_missing)` を出して [`NO_BODY`] を返す。
    pub(crate) fn of(world: &World) -> Self {
        if let Some(body) = world.get_resource::<BodySystems>() {
            return *body;
        }
        #[cfg(not(test))]
        tracing::error!(
            event = "body_systems_missing",
            "本体の口の束が据わっていない（結線の誤り）——本体のモジュールが無いものとして続ける"
        );
        NO_BODY
    }
}

/// 本体が無いときの答え: 何もしない・偽・捨てる受け口 2 つ（窓口が無いときの答えと同じ）。
pub(crate) const NO_BODY: BodySystems = BodySystems {
    menu_poll: |_| {},
    after_input: |_| {},
    after_char_pointer: |_| {},
    after_balloon_pointer: |_| {},
    after_runtime_wired: |_, _| {},
    on_switch_end: |_, _| {},
    on_steady: |_| {},
    overwrite_between: |_| {},
    update_busy: |_| false,
    install_busy: |_| false,
    cue_sinks: |_| [Box::new(DiscardSink::new()), Box::new(DiscardSink::new())],
};

#[cfg(test)]
#[path = "body_systems_tests.rs"]
mod tests;
