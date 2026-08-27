use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc;

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::HitRectPx;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::world::World;
use log_capture_kit::{LineFormat, capture_lines};
use wintf::ecs::Window;
use wintf::ecs::pointer::PointerLeave;

use crate::emo2_boot::assets::{BalloonScopeAssets, BootAssets, LoopTables, ScopeAssets};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::move_cue::MoveDirective;
use crate::emo2_boot::talk_clock::TalkClock;
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::placement::spawn::BalloonWindowMarker;

/// 窓物理 px の行矩形を持つ `ChoiceHitRow` を組む（ordinal は入力順昇順を模す）。
/// rect 以外のフィールドは 3.1 の判定に無関係——不透明転写の placeholder。
pub(super) fn row(ordinal: usize, left: f32, top: f32, right: f32, bottom: f32) -> ChoiceHitRow {
    ChoiceHitRow {
        ordinal,
        id: format!("q{ordinal}"),
        label: format!("label{ordinal}"),
        references: Vec::new(),
        rect: HitRectPx {
            left,
            top,
            right,
            bottom,
        },
    }
}

/// 空 `EmoWorld`（空 shell から build・COM/GPU 不要の寛容契約・frame.rs 檻同型）。
fn empty_world() -> EmoWorld {
    EmoWorld::build(&areka_parsers::shell::parse(""))
}

/// 空アトラス（headless 構築・frame.rs 檻同型）。
fn empty_atlas() -> AtlasTable {
    AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
}

/// 合成 `BootAssets`（scope0 の最小形・COM/GPU/fixture 不要の純合成・frame.rs synth_assets 同型）。
fn synth_boot_assets() -> BootAssets {
    BootAssets {
        shells: vec![ScopeAssets {
            scope: 0,
            emo_world: empty_world(),
            atlas: empty_atlas(),
            initial_surface_id: 0,
        }],
        balloons: vec![BalloonScopeAssets {
            scope: 0,
            emo_world: empty_world(),
            atlas: empty_atlas(),
            model: areka_parsers::balloon::parse_str("", None),
        }],
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds: BindSet::default(),
        bind_resolver: BindResolver::empty(),
        shell_author_dpi: 96,
        balloon_author_dpi: 96,
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: BTreeMap::new(),
        },
    }
}

/// headless な `Emo2Wiring`（実 `EmoPresenter`／与えた runtime／合成 `BootAssets`・COM/GPU 不要）。
///
/// ハンドラが `Emo2Wiring::runtime()` から借りる runtime を、テスト側が事前に populate した実体で
/// 差し込むための最小結線（frame.rs の headless_wiring_with と同型）。
pub(super) fn headless_emo2_wiring(runtime: Rc<RefCell<TextLayerRuntime>>) -> Emo2Wiring {
    Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<MoveDirective>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        runtime,
        TalkClock::new(Arc::new(|| 0.0)),
        synth_boot_assets(),
    )
}

/// 選択肢スパンを 1 つ載せて `choice_active(actor)==true` にした実 runtime を組む（GPU 不要）。
/// `choice_hit_rows` は `present_frame`（GPU）未実行ゆえ空のまま（headless の既知制約）。
pub(super) fn runtime_with_active_choice(actor: &str) -> Rc<RefCell<TextLayerRuntime>> {
    let rt = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    rt.borrow_mut().apply_cue(&TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::Choice {
            id: "OnYes".into(),
            text: "はい".into(),
            references: Vec::new(),
        },
        duration: 0.0,
    });
    rt
}

// ── ログレベルを決定論的に観測するための捕捉（捕捉層は共有機構へ委譲）───────────────
//
// 捕捉層そのものはここに持たず、硬化機構の唯一の定義元 `log-capture-kit` へ委譲する。
// 行の形（1 イベント 1 行・`level=…` に続けてフィールドを訪問順で ` name=value`）も
// 呼出側の判定内容も、移行前と 1 バイト変わらない。
//
// 「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」は**誤り**である。差し替わる
// のはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
// interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点を最初に踏んだスレッドの
// 判定が焼き付く（先着が勝つ）。捕捉窓を持たないスレッドの既定は `NoSubscriber` で判定は
// 「不要」ゆえ、先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても
// 取りこぼす。共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 捕捉窓の内側での interest 再計算
// ⑶ 番兵イベントによる空振り検出 の 3 点でこれを塞ぐ（機序の逐条解説は `log_capture_kit` の
// crate doc と同 crate の `src/probe.rs`）。

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelFields, f);
    lines
}

/// バルーン窓（`BalloonWindowMarker`＋`Window`）を組み、その子 entity へ `PointerLeave` を載せる。
/// 子の親チェーンは window へ届く（`find_owner_window` 相当）。
pub(super) fn spawn_balloon_leave_child(world: &mut World, scope: usize) -> Entity {
    let win = world
        .spawn((BalloonWindowMarker { scope }, Window::default()))
        .id();
    world.spawn((PointerLeave, ChildOf(win))).id()
}
