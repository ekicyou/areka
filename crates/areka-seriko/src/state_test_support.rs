use super::*;
use log_capture_kit::{LineFormat, capture_lines};

/// テスト用 bind 集合（非空・emo2 実測相当の任意 id）。
pub(super) fn binds_1100_1207() -> BindSet {
    BindSet::from_ids([1100, 1207])
}

pub(super) fn empty_states() -> ScopeStates {
    ScopeStates::new(binds_1100_1207())
}

/// テスト専用 tracing 捕捉ハーネス（actor/table の同名ヘルパと同一＝硬化機構の唯一の定義元
/// `log-capture-kit` へ委譲）。1 イベント 1 行へ level／target／各フィールド（`name=value`）を
/// 整形し、改行連結で返す。
///
/// 「`with_default` はスレッドローカルゆえ並行テスト安全」は誤りである（callsite の interest
/// キャッシュはプロセス全体で 1 つ・先着スレッドの判定が焼き付く）。共有機構が probe 常駐・
/// 窓内の interest 再計算・番兵による空振り検出で塞ぐ。機序は `log_capture_kit` の crate doc
/// と同 crate の `src/probe.rs` を参照。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> String {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines.join("\n")
}
