use super::{ActorKey, CueCommand, TalkCue, TextLayerRuntime};

// ---------------------------------------------------------------------------
// シナリオ定義（boot.pasta 起動朝 挨拶由来のハードコード cue 列・注入時刻付き）
// ---------------------------------------------------------------------------
//
// 台詞は fixture スクリプト boot.pasta「起動朝」シーン（むらさき=\0／エモ=\1）由来。
// 折返し閾値・行容量の檻が決定論で効くよう、共有 fixture の幾何
// （validrect 320×122 image px・font 28px・line pitch 35px）に合わせて刻んでいる。

/// \0 の 1 行目（6 グリフ・typewriter 進行の観測対象）。
const LINE1: &str = "おっはよー！";
/// \0 の 2 行目（10 グリフ・改行観測。横書き 1 行に収まり、縦書きでは複数列へ折返す）。
const LINE2: &str = "めっちゃええ朝やん！";
/// \0 の 3 行目（8 グリフ・横書き行容量 3 行ちょうどの最終行）。
const LINE3: &str = "今日もいくでー！";
/// \1（kero）の台詞（7 グリフ・複数 actor 振り分けの観測対象）。
const KERO_LINE: &str = "朝から元気だね";
/// あふれ→スクロール誘発用の短行（2 グリフ×9 行・先頭行より行内範囲が確実に短い）。
const SHORT_LINE: &str = "ほな";
/// あふれ誘発の短行数（横書き容量 3 行・縦書き容量 9 列をどちらも確実に超える）。
const OVERFLOW_LINES: usize = 9;

/// 各ステージのチェックポイント注入時刻（talk 起点相対秒・リビール時刻＋丸め余裕）。
pub(super) const T_CHECK: [f64; 7] = [0.12, 0.35, 1.1, 1.8, 3.0, 3.2, 3.4];

/// R10.3 DrawStats 檻: 1 行/列スクロールフレームの `DrawTextLayout` 増分の tight bound。
/// この共有 fixture（validrect 320×122・28px フォント＝可視 3 行）では、1 行スクロールの
/// ダーティ＝露出帯 ∪ 完成した流入行 ∪ 末尾の空行の 3 枚に、実描画対象は流入行 1 本のみ
/// （空行は skip）＝`draw_text_layout_calls` 増分は 3（`dirty_len 3 × draws 1`）に収まる。
/// talk 全体の行数（あふれで 12 行超）に伸びない小定数であることが要点（reference:
/// tests/viewbox_scroll_test.rs は可視 8 行の別 fixture で `増分 < 可視行数` を厳密に檻化）。
pub(super) const EXPOSURE_BAND_DRAW_BOUND: u64 = 3;

/// ステージ gate（cue が UI ドレインを経て状態機械へ適用済みであることの決定論条件）。
#[derive(Clone, Copy, Debug)]
pub(super) enum Gate {
    /// actor 状態の items（グリフ＋改行マーカー）数が一致する。
    Items(usize),
    /// actor 状態が空（Clear 適用済み）。
    Empty,
    /// 条件なし。
    Any,
}

impl Gate {
    pub(super) fn satisfied(self, rt: &TextLayerRuntime, actor: &ActorKey) -> bool {
        match self {
            Gate::Items(n) => rt
                .state()
                .actor_state(actor)
                .map(|s| s.items().len())
                == Some(n),
            Gate::Empty => rt
                .state()
                .actor_state(actor)
                .is_some_and(|s| s.items().is_empty()),
            Gate::Any => true,
        }
    }
}

/// \0 の gate（ステージ順）。items 数: L1=6 → +1+10=17 → +1+8=26 → +9×(1+2)=53 → Clear。
pub(super) const GATE_SAKURA: [Gate; 7] = [
    Gate::Items(6),
    Gate::Items(6),
    Gate::Items(17),
    Gate::Items(26),
    Gate::Items(53),
    Gate::Empty,
    Gate::Empty,
];
/// \1 の gate（ステージ順）。KERO_LINE=7 items → Clear。
pub(super) const GATE_KERO: [Gate; 7] = [
    Gate::Any,
    Gate::Any,
    Gate::Any,
    Gate::Items(7),
    Gate::Items(7),
    Gate::Items(7),
    Gate::Empty,
];

/// cue 生成ヘルパ。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

/// ステージごとに sink へ流す cue 列（`at` は注入時刻＝リビール開始の下限）。
pub(super) fn stage_cues(stage: usize) -> Vec<TalkCue> {
    match stage {
        0 => vec![cue("0", 0.0, CueCommand::Text(LINE1.into()))],
        1 => Vec::new(),
        2 => vec![
            cue("0", 0.5, CueCommand::NewLine { ratio: 1.0 }),
            cue("0", 0.5, CueCommand::Text(LINE2.into())),
        ],
        3 => vec![
            cue("0", 1.2, CueCommand::NewLine { ratio: 1.0 }),
            cue("0", 1.2, CueCommand::Text(LINE3.into())),
            cue("1", 1.2, CueCommand::Text(KERO_LINE.into())),
        ],
        4 => {
            let mut cues = Vec::with_capacity(OVERFLOW_LINES * 2);
            for _ in 0..OVERFLOW_LINES {
                cues.push(cue("0", 2.0, CueCommand::NewLine { ratio: 1.0 }));
                cues.push(cue("0", 2.0, CueCommand::Text(SHORT_LINE.into())));
            }
            cues
        }
        5 => vec![cue("0", 3.1, CueCommand::Clear)],
        6 => vec![cue("1", 3.3, CueCommand::Clear)],
        _ => Vec::new(),
    }
}
