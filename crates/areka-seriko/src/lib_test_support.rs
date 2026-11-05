//! 層をまたぐテストの道具の受け口（spec: audit-seriko 指摘 6）。`lib.rs` がテストのときだけ
//! `crate::test_support` として繋ぐ。表・役者・状態・部品・時刻の各テストが同じ定義を使う。

use areka_emo_compose::{ComposeMethod, EmoWorld, PatternFrame, PatternState};
use areka_sakura::ActorKey;
use log_capture_kit::{LineFormat, capture_lines};

use crate::parts::PartAnim;
use crate::table::{AnimationTable, FrameAct, LoopFrame};

/// `check_calls` の 6 種の記録の目印（要件の番号）。
pub(crate) const KINDS: [&str; 6] = [
    "要件 2.5",
    "要件 5.1",
    "要件 5.3",
    "要件 6.6",
    "要件 6.5",
    "要件 6.4",
];

/// 本文を本番と同じ順（読み手 → 畳み込み → 表）で表にする。
pub(crate) fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
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

/// `warn!` の行だけ。
pub(crate) fn warns(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("level=WARN")).collect()
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
