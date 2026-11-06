// UI スレッド常駐ランタイム（TextLayerRuntime／spawn_emo_text／present_frame）のテスト
// （`actor_*_tests.rs`・`sink` のテスト）が共有する部品（`crate::actor::test_support`）。
// cue の組み立て・予約スロットの World・読み戻し・箱の束・受信口の drain・記録の捕捉を置く。

use std::collections::BTreeMap;
use std::ops::{ControlFlow, RangeInclusive};
use std::sync::{Arc, Mutex};

use areka_emo_compose::{BoxLayout, BoxName, EmoWorld, fold_boxes};
use areka_parsers::balloon::{
    BalloonCursor, BalloonModel, CursorColor, Font, FontColor, Origin, ValidRect, WindowPosition,
    WordWrapPoint,
};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use log_capture_kit::{CapturedEvent, capture};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

use super::{ChoiceHitRow, SurfaceKeyOutcome, SurfaceKeyResolver, TextLayerRuntime};
use crate::place::{PlaceKey, TextPlace};
use crate::sink::{EmoTextSink, TextMsg, handle_text_msg_with};
use crate::state::TextItem;

// ── テスト土台 ──

/// reveal 間隔（秒/グリフ）。reveal は配送 duration 由来（`interval = duration / N`）ゆえ、
/// Text cue へ `N × REVEAL_INTERVAL` の duration を焼き込むことで interval=0.05 の進行を得る
/// （旧 char_wait=0.05 既定と機能等価・進行観測は安全マージン付き時刻 0.06/0.11 で行う）。
pub(crate) const REVEAL_INTERVAL: f64 = 0.05;

/// テスト用 cue。Text cue には配送 duration = `N × REVEAL_INTERVAL` を焼き込む
/// （reveal interval=0.05）。他コマンドは瞬時（duration=0）。
pub(crate) fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => areka_sakura::cluster::cluster_count(t) as f64 * REVEAL_INTERVAL,
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
pub(crate) fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけの
    // 無害な Win32 呼び出し（テストスレッド＝pump スレッド）。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// テスト用 BalloonModel（幾何のみ・font 未指定＝既定 ＭＳ ゴシック 12px・
/// validrect 未指定＝画像全域・wordwrap 未指定＝右辺折返し）。
pub(crate) fn geo_model() -> BalloonModel {
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
pub(crate) fn cursor_model() -> BalloonModel {
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
pub(crate) fn spawn_reserved_slot(world: &mut World) -> (Entity, Entity) {
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
pub(crate) fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

// ══ 選択肢契約 API（inject_choice_hover／choice_hit_rows／choice_active・純粋・COM 不要） ══

/// 選択肢テキストを載せる Choice cue（duration はクラスタ数×REVEAL_INTERVAL・[`cue`] と同じ流儀）。
pub(crate) fn choice_cue(actor: &str, at: f64, id: &str, text: &str, refs: &[&str]) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command: CueCommand::Choice {
            id: id.into(),
            text: text.into(),
            references: refs.iter().map(|s| s.to_string()).collect(),
        },
        duration: areka_sakura::cluster::cluster_count(text) as f64 * REVEAL_INTERVAL,
    }
}

// ══ 提示パイプラインとヒット行スナップショット配線（COM・headless・R3.1/3.2/3.3/5.2/6.3） ══

/// 3 資源（GraphicsCore/WucGraphicsResource/予約スロット）を積んだ World を組む土台。
/// 返り値は (world, window, slot)。COM は本番 UI スレッド（MTA）を再現する。
pub(crate) fn com_world() -> (World, Entity, Entity) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    insert_gpu(&mut world);
    (world, window, slot)
}

/// 新しい GraphicsCore と WucGraphicsResource を作って World へ入れる（既にあれば差し替える）。
pub(crate) fn insert_gpu(world: &mut World) {
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");
    world.insert_resource(core);
    world.insert_resource(wuc);
}

/// 窓の子の列（`Children` の順）。子が無ければ空。
pub(crate) fn window_children(world: &World, window: Entity) -> Vec<Entity> {
    world
        .get::<Children>(window)
        .map(|c| c.iter().copied().collect())
        .unwrap_or_default()
}

/// actor の供給面を読み戻す（BGRA 密配列）。
pub(crate) fn read_back(rt: &TextLayerRuntime, actor: &ActorKey) -> Vec<u8> {
    rt.surface(actor)
        .expect("供給面")
        .read_back()
        .expect("read_back")
}

// ── 画素の数え上げ ──

/// 矩形 `(x0, y0, x1, y1)`（右と下は含まない）の中で `pred` を満たす画素の数。
/// 矩形は面（幅 `width`・丈は `bytes` の長さから）の内側へ切り詰める。
pub(crate) fn count_px(
    bytes: &[u8],
    width: u32,
    (x0, y0, x1, y1): (u32, u32, u32, u32),
    pred: impl Fn(&[u8]) -> bool,
) -> usize {
    let height = (bytes.len() / 4 / width.max(1) as usize) as u32;
    let (x1, y1) = (x1.min(width), y1.min(height));
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if pred(&bytes[i..i + 4]) {
                n += 1;
            }
        }
    }
    n
}

/// 非透明（α ≠ 0）の画素か（draw.rs 檻と同じ述語）。
pub(crate) fn inked(px: &[u8]) -> bool {
    px[3] != 0
}

/// 当たり行の矩形を画素の境（左上は切り捨て・右下は切り上げ・負は 0）へ直す。
pub(crate) fn row_px(row: &ChoiceHitRow) -> (u32, u32, u32, u32) {
    let r = row.rect;
    (
        r.left.floor().max(0.0) as u32,
        r.top.floor().max(0.0) as u32,
        r.right.ceil().max(0.0) as u32,
        r.bottom.ceil().max(0.0) as u32,
    )
}

// ── 箱の表 ──

/// surfaces.txt の文面を `parse_boxes` → `fold_boxes` で畳んだ箱の表（文面は誤りを持たないこと）。
pub(crate) fn box_layout(source: &str) -> BoxLayout {
    let world = EmoWorld::build(&parse(source));
    let (layout, report) = fold_boxes(&parse_boxes(source), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
}

/// サーフェス番号 → element番号の昇順の箱の名前の表。
pub(crate) fn box_index(
    layout: &BoxLayout,
    surfaces: RangeInclusive<u32>,
) -> BTreeMap<u32, Vec<BoxName>> {
    surfaces
        .map(|id| {
            let names = layout
                .placements(id)
                .iter()
                .map(|p| p.name.clone())
                .collect();
            (id, names)
        })
        .collect()
}

/// サーフェス `surface` に置いた箱の名前（名前は公開の構築口を持たないので表から拾う）。
pub(crate) fn box_name(layout: &BoxLayout, surface: u32, name: &str) -> BoxName {
    layout
        .placements(surface)
        .iter()
        .find(|p| p.name.as_str() == name)
        .expect("文面にある箱の名前")
        .name
        .clone()
}

/// 1 度組んだ箱の名前の表から名前を拾う。
pub(crate) fn box_in(index: &BTreeMap<u32, Vec<BoxName>>, name: &str) -> BoxName {
    index
        .values()
        .flatten()
        .find(|n| n.as_str() == name)
        .expect("文面にある箱の名前")
        .clone()
}

/// 偽の閉包: 数字ならその番号を表示、`-1` は非表示、それ以外は解決できない。
pub(crate) fn fake_resolver() -> SurfaceKeyResolver {
    Box::new(|key: &str| match key {
        "-1" => SurfaceKeyOutcome::Hide,
        _ => key
            .parse()
            .map_or(SurfaceKeyOutcome::Unresolved, SurfaceKeyOutcome::Show),
    })
}

pub(crate) fn emote(actor: &str, key: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Emote { key: key.into() })
}

pub(crate) fn text(actor: &str, t: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Text(t.into()))
}

/// actor の場所 `place` に届いた文字の項目（場所が無ければ空）。
pub(crate) fn items_at(rt: &TextLayerRuntime, actor: &str, place: TextPlace) -> Vec<TextItem> {
    rt.state()
        .place_state(&PlaceKey {
            actor: ActorKey::from(actor),
            place,
        })
        .map_or_else(Vec::new, |s| s.items().to_vec())
}

// ── 指令の部品（瞬時の cue・記録のドレイン） ──

/// 時刻 0・配送 duration 0 の cue（reveal を待たずに全部見える）。
pub(crate) fn instant(actor: &str, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

/// 本文が id と同じ、参照なしの瞬時の選択肢。
pub(crate) fn choice(actor: &str, id: &str) -> TalkCue {
    instant(
        actor,
        CueCommand::Choice {
            id: id.to_owned(),
            text: id.to_owned(),
            references: Vec::new(),
        },
    )
}

pub(crate) fn balloon(actor: &str) -> PlaceKey {
    PlaceKey::balloon(&ActorKey::from(actor))
}

/// 4 つの受け口の形で届いた便りを全部記録するドレイン（本テストスレッド＝pump スレッド）。
pub(crate) fn spawn_message_drain(name: &str) -> (EmoTextSink, Arc<Mutex<Vec<TextMsg>>>) {
    let records = Arc::new(Mutex::new(Vec::new()));
    let (on_cue, on_upcoming, on_mark) = (
        Arc::clone(&records),
        Arc::clone(&records),
        Arc::clone(&records),
    );
    let (tx, _detached_drain) = areka_actor::spawn_ui(
        name,
        move |msg: TextMsg| -> Result<ControlFlow<()>, String> {
            handle_text_msg_with(
                msg,
                |c, _| {
                    on_cue.lock().expect("records").push(TextMsg::Cue(c));
                    Ok(())
                },
                |u| {
                    on_upcoming
                        .lock()
                        .expect("records")
                        .push(TextMsg::Upcoming(u));
                    Ok(())
                },
                |m| {
                    on_mark.lock().expect("records").push(m);
                    Ok(())
                },
            )
        },
    )
    .expect("spawn_ui on the test (pump) thread must succeed");
    (EmoTextSink::new(tx), records)
}

// ── 記録の捕まえ方 ──

/// 捕捉窓の中で `f` を走らせ、`(戻り値, 全部の記録)` を返す。窓の内側で対照の `error!` を 1 件
/// 出し、ERROR がちょうど 1 件であることをここで確かめる（「WARN が 0 件」の主張が捕捉窓の死で
/// 恒真になるのを防ぐ・`f` が ERROR を出していないことも同時に確かめる）。
pub(crate) fn capture_checked<R>(f: impl FnOnce() -> R) -> (R, Vec<CapturedEvent>) {
    let (value, events) = capture(|| {
        tracing::error!("捕捉窓が生きていることの対照イベント");
        f()
    });
    assert_eq!(
        events
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR)
            .count(),
        1,
        "対照の 1 件のほかに ERROR が出たか、対照が数えられていない: {events:?}"
    );
    (value, events)
}

/// 記録の欄を f32 として読む（`{:?}` 表現の細部に依存しないよう、解析してから比べる）。
pub(crate) fn number_field(event: &CapturedEvent, name: &str) -> f32 {
    let raw = event
        .field(name)
        .unwrap_or_else(|| panic!("欄 {name} が記録に載っていない"));
    raw.parse()
        .unwrap_or_else(|_| panic!("欄 {name} の値 {raw} を数値として読めない"))
}

// ── 警告の文（発行点と 1 文字も違わないこと） ──

/// 折返し基準の警告（手順書 §5.7 の grep 語・変えると実機走行の判定が静かに壊れる・要件 14.3）。
pub(crate) const COARSE_WRAP_MESSAGE: &str = "折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ）";

/// 無視した `origin` 宣言の警告（`actor_decoration.rs` の発行点）。
pub(crate) const IGNORED_ORIGIN_MESSAGE: &str = "origin に指定した文字の書き始めの位置が、文字を描いてよい範囲（validrect）の外にある——指定は使わず、範囲の書き始めの角から書いた";

/// 書体の高さ 0 の警告（`draw.rs`・解き直しの中の作者向けの `warn!`）。
pub(crate) const FONT_HEIGHT_ZERO: &str =
    "font.height が 0（DirectWrite fontsize は正値必須）のため既定値 12 へフォールバックする";

/// ROP 系の飾りの警告。
pub(crate) const ROP_FALLBACK_MESSAGE: &str =
    "cursor.blendmethod ROP 系は M1 未対応: none 扱い（色ベース描画）へ縮退";
