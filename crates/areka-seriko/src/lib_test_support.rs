//! 層をまたぐテストの道具の受け口（spec: audit-seriko 指摘 6）。`lib.rs` がテストのときだけ
//! `crate::test_support` として繋ぐ。表・役者・状態・部品・時刻の各テストが同じ定義を使う。

use areka_emo_compose::{
    Cell, ComposeMethod, EmoWorld, FilmId, PartKey, PatternFrame, PatternState,
};
use areka_sakura::{ActorKey, CueCommand, TalkCue};
use log_capture_kit::{LineFormat, capture_lines};

use crate::parts::PartAnim;
use crate::table::{AnimationTable, FrameAct, LoopFrame};

/// `check_calls` の 6 種の記録の目印（seriko-pattern-calls の要件の番号）。
pub(crate) const KINDS: [&str; 6] = [
    "要件 2.5",
    "要件 5.1",
    "要件 5.3",
    "要件 6.6",
    "要件 6.5",
    "要件 6.4",
];

/// 動く絵と手書きの `always` を置いた検体のシェル（中身は同じフォルダの README）。
pub(crate) const ANIMATED_PLAYBACK_SHELL: &str =
    include_str!("../../areka-emo-compose/tests/fixtures/animated-playback/shell/surfaces.txt");

/// 本文を本番と同じ順（読み手 → 畳み込み → 表）で表にする。
pub(crate) fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

/// 本文を読み手 → 畳み込みで組み、`films` のパスの element を、分解と同じに動く絵の子を置く element へ
/// 替えた世界（表は `AnimationTable::from_world_and_films` に絵の列と一緒に渡す）。
pub(crate) fn film_world(text: &str, films: &[(&str, FilmId)]) -> EmoWorld {
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(text));
    let w = world.world_mut();
    let mut q = w.query::<&mut areka_emo_compose::SurfaceMaster>();
    for mut master in q.iter_mut(w) {
        for e in &mut master.elements {
            if let Some(&(_, film)) = films.iter().find(|(path, _)| e.path.as_str() == *path) {
                e.kind = areka_emo_compose::ElementKind::Film(film);
            }
        }
    }
    world
}

/// 動く絵の子の定義（分解が作る値と同じ形）。コマの絵の番号は `id` から 1 ずつ、元の大きさは 2×2。
pub(crate) fn film_sheet(
    id: FilmId,
    path: &str,
    delays_ms: &[u32],
    laps: Option<u32>,
) -> areka_emo_compose::FilmSheet {
    areka_emo_compose::FilmSheet {
        id,
        path: path.to_string(),
        frames: (0..delays_ms.len() as u32).map(|i| id.0 + i).collect(),
        delays_ms: delays_ms.to_vec(),
        laps: laps.and_then(std::num::NonZeroU32::new),
        original: (2, 2),
        rest: 0,
    }
}

/// 着せ替えの無い空のスコープ状態。
pub(crate) fn empty_states() -> crate::state::ScopeStates {
    crate::state::ScopeStates::new(crate::DefaultBinds::uniform(
        areka_emo_compose::BindSet::default(),
    ))
}

/// 静的な着せ替え {1100, 1207} を持つ空のスコープ状態（役者と状態のテストが使う）。
pub(crate) fn fresh_states() -> crate::state::ScopeStates {
    crate::state::ScopeStates::new(crate::DefaultBinds::uniform(
        areka_emo_compose::BindSet::from_ids([1100, 1207]),
    ))
}

/// 既定のスコープ `\0`。
pub(crate) fn scope() -> ActorKey {
    ActorKey::from("0")
}

/// テスト専用 tracing 捕捉（硬化機構の唯一の定義元 `log-capture-kit` へ委譲）。1 イベント 1 行へ
/// level／target／各フィールド（`name=value`）を整形した行の列を返す。
///
/// **「`with_default` はスレッドローカルゆえ並行テスト安全」は誤り**である。差し替わるのは
/// スレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
/// interest キャッシュはプロセス全体で 1 つしかなく、その発行点を最初に踏んだスレッドの
/// 判定が焼き付く。捕捉窓を持たないスレッド（既定は `NoSubscriber`）が先に踏むと `never`
/// が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。共有機構は
/// ⑴ probe 常駐 ⑵ 窓内の interest 再計算 ⑶ 番兵による空振り検出でこれを塞ぐ。機序の
/// 逐条解説は `log_capture_kit` の crate doc および同 crate の `src/probe.rs` にある。
pub(crate) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    capture_lines(LineFormat::LevelTargetFields, f).1
}

/// [`capture_logs`] の行を改行でつないだ文字列。
pub(crate) fn capture_text<F: FnOnce()>(f: F) -> String {
    capture_logs(f).join("\n")
}

/// 本文から表を組み、表を組む走行（`from_world`）の `tracing` 出力を 1 イベント 1 行で添えて返す。
/// 読み手と畳み込みの記録は含まない。
pub(crate) fn table_and_logs(text: &str) -> (AnimationTable, Vec<String>) {
    let world = EmoWorld::build(&areka_parsers::shell::parse(text));
    capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world(&world)
    })
}

/// `needle` を含む行の数。
pub(crate) fn count(lines: &[String], needle: &str) -> usize {
    lines.iter().filter(|l| l.contains(needle)).count()
}

/// `warn!` の行だけ。
pub(crate) fn warns(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("level=WARN")).collect()
}

/// 台本の `at` 秒の、スコープ `scope` への合図（再生時間は `duration` 秒）。
pub(crate) fn cue(at: f64, scope: &str, command: CueCommand, duration: f64) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command,
        duration,
    }
}

/// 台本の `at` 秒に始まる文字の cue（1 字 50 ms）。
pub(crate) fn text(at: f64, scope: &str, s: &str) -> TalkCue {
    let duration = s.chars().count() as f64 * 0.05;
    cue(at, scope, CueCommand::Text(s.into()), duration)
}

/// 台詞の頭に前置される全消去（台本の 0 秒）。
pub(crate) fn clear_all() -> TalkCue {
    cue(0.0, "0", CueCommand::ClearAll, 0.0)
}

/// 時刻 `at` に始まった部品の再生中。
pub(crate) fn part_playing(at: u64) -> Option<PartAnim> {
    Some(PartAnim::Playing {
        started_at_ms: at,
        acts_through: None,
    })
}

/// 面 `surface_id` を指すコマ 1 枚（`Overlay`・オフセットなし）。
pub(crate) fn overlay_frame(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

/// animation `anim_id` に [`overlay_frame`] 1 枚だけを持つ `PatternState`。
pub(crate) fn one_frame_pattern(anim_id: u32, surface_id: u32) -> PatternState {
    let mut p = PatternState::default();
    p.set(anim_id, overlay_frame(surface_id));
    p
}

/// 面 `surface_id` を描く 1 コマ（絵なし・`Overlay`・オフセットなし）。
pub(crate) fn draw_frame(surface_id: i64, wait_ms: u32) -> LoopFrame {
    LoopFrame {
        surface_id,
        picture: None,
        method: ComposeMethod::Overlay,
        act: FrameAct::Draw,
        wait_ms,
        x: 0,
        y: 0,
    }
}

/// 部品 `part` の欄を (animation の番号, コマの番号) の列で読む。
pub(crate) fn part_frames(pattern: &PatternState, part: u32) -> Vec<(u32, u32)> {
    pattern
        .part(part)
        .map(|(id, f)| (id, f.surface_id))
        .collect()
}

/// 子 `film` の欄の絵（`None`＝載っていない＝経過 0）。
pub(crate) fn film_picture(pattern: &PatternState, film: FilmId) -> Option<u32> {
    match pattern.cell(Some(PartKey::Film(film)), 0) {
        Cell::Picture(id) => Some(id),
        Cell::Rest => None,
        other => panic!("子の欄に {other:?}"),
    }
}
