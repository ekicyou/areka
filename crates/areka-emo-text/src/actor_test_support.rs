// task 7.2 の檻: UI スレッド常駐ランタイム（TextLayerRuntime／spawn_emo_text／
// present_frame）。実 pump 上の終了指示・全送信元切断・個別失敗継続の 3 経路
// （R1.2/R1.3/R1.4）と、注入時刻フレーム駆動での可視グリフ進行・未解決 actor の
// 蓄積＋スキップ＋再試行・装着済み actor の Present 完結（R9.3）を檻化する。

use areka_parsers::balloon::{
    BalloonCursor, BalloonModel, CursorColor, Font, FontColor, Origin, ValidRect, WindowPosition,
    WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

// ── テスト土台 ──

/// reveal 間隔（秒/グリフ）。reveal は配送 duration 由来（`interval = duration / N`）ゆえ、
/// Text cue へ `N × REVEAL_INTERVAL` の duration を焼き込むことで interval=0.05 の進行を得る
/// （旧 char_wait=0.05 既定と機能等価・進行観測は安全マージン付き時刻 0.06/0.11 で行う）。
const REVEAL_INTERVAL: f64 = 0.05;

/// テスト用 cue。Text cue には配送 duration = `N × REVEAL_INTERVAL` を焼き込む
/// （reveal interval=0.05）。他コマンドは瞬時（duration=0）。
pub(super) fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => t.chars().count() as f64 * REVEAL_INTERVAL,
        _ => 0.0,
    };
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration,
    }
}

/// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
/// （sink.rs テストの決定論パターン——WM_QUIT は posted メッセージが尽きた後にのみ配送）。
pub(super) fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけの
    // 無害な Win32 呼び出し（テストスレッド＝pump スレッド）。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// テスト用 BalloonModel（幾何のみ・font 未指定＝既定 ＭＳ ゴシック 12px・
/// validrect 未指定＝画像全域・wordwrap 未指定＝右辺折返し）。
pub(super) fn geo_model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// cursor.\* を持つ BalloonModel（fixture 実導出＝square 塗り(105,25,25)＋白文字(255,255,255)）。
/// `geo_model` の幾何既定へ `with_cursor` で SquareFill 導出の cursor を相乗せする
/// （`ResolvedChoiceStyle::resolve` が `SquareFill { fill:(105,25,25), text:(255,255,255) }` を束ねる）。
pub(super) fn cursor_model() -> BalloonModel {
    geo_model().with_cursor(BalloonCursor::new(
        Some("square".to_string()),
        CursorColor::new(Some(105), Some(25), Some(25)), // brush.color＝矩形塗り色
        CursorColor::new(None, None, None),              // pen.color（M1 非参照）
        CursorColor::new(Some(255), Some(255), Some(255)), // font.color＝hover 白文字
        None,                                            // blendmethod（既定 none）
    ))
}

/// emo-present `VisualMount` と同型の予約スロット（surface.rs テストと同型）。
/// 返り値は (window, slot)。
pub(super) fn spawn_reserved_slot(
    world: &mut World,
) -> (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-layer-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();
    (window, slot)
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0・draw.rs 檻と同じ述語）。
pub(super) fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

// ══ task 8.1: 選択肢契約 API（inject_choice_hover／choice_hit_rows／choice_active・純粋・COM 不要） ══

/// 選択肢テキストを載せる Choice cue（duration は文字数×REVEAL_INTERVAL・runtime_tests の cue 流儀）。
pub(super) fn choice_cue(actor: &str, at: f64, id: &str, text: &str, refs: &[&str]) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command: CueCommand::Choice {
            id: id.into(),
            text: text.into(),
            references: refs.iter().map(|s| s.to_string()).collect(),
        },
        duration: text.chars().count() as f64 * REVEAL_INTERVAL,
    }
}

// ══ task 8.2: 提示パイプラインとヒット行スナップショット配線（COM・headless・R3.1/3.2/3.3/5.2/6.3） ══

/// 3 資源（GraphicsCore/WucGraphicsResource/予約スロット）を積んだ World を組む土台。
/// 返り値は (world, window, slot)。COM は本番 UI スレッド（MTA）を再現する。
pub(super) fn com_world() -> (World, bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    world.insert_resource(core);
    world.insert_resource(wuc);
    (world, window, slot)
}
