use super::{ActorKey, CueCommand, TalkCue, TextLayerRuntime, text_playback_duration};

// ---------------------------------------------------------------------------
// シナリオ定義（boot.pasta 起動朝 挨拶由来のハードコード cue 列・注入時刻付き）
// ---------------------------------------------------------------------------
//
// 台詞は fixture スクリプト boot.pasta「起動朝」シーン（むらさき=\0／エモ=\1）由来。
// 折返し閾値・行容量の檻が決定論で効くよう、共有 fixture の幾何に合わせて刻んでいる。
//
// 幾何（`balloons0s.txt` の validrect top 46／bottom −56／left 36／right −44・枠画像 400×224）:
// 描画範囲は絶対 (36,46)-(356,168) ＝ 320×122 image px。font 28px ゆえ行送りは
// 字の丈 28 ＋ 行間 2 ＝ **30px**（旧 35px）。
// - 横書きの行容量: 行 i の下端 = 46 + 30i + 28 ≤ 168 → i = 0..3 の **4 行**
//   （4 行目 164 ≤ 168・5 行目 194 > 168）。旧 35px では 3 行（4 行目 179 > 168）。
// - 縦書き（vertical 変種）の列容量: 列 i の左端 = 356 − 28 − 30i ≥ 36 →
//   i = 0..9 の **10 列**（= `floor((320 − 28) / 30) + 1`）。旧 35px では 9 列。

/// \0 の 1 行目（6 グリフ・typewriter 進行の観測対象）。
const LINE1: &str = "おっはよー！";
/// \0 の 2 行目（10 グリフ・改行観測。横書き 1 行に収まり、縦書きでは複数列へ折返す）。
const LINE2: &str = "めっちゃええ朝やん！";
/// \0 の 3 行目（8 グリフ・横書き行容量 4 行のうちの 3 行目——旧行送りでは最終行だった）。
const LINE3: &str = "今日もいくでー！";
/// \1（kero）の台詞（7 グリフ・複数 actor 振り分けの観測対象）。
const KERO_LINE: &str = "朝から元気だね";
/// あふれ→スクロール誘発用の短行（2 グリフ×9 行・先頭行より行内範囲が確実に短い）。
const SHORT_LINE: &str = "ほな";
/// あふれ誘発の短行数（横書き容量 4 行・縦書き容量 10 列をどちらも確実に超える。
/// 横書きは既存 3 行 ＋ 9 行 = 12 行 > 4・縦書きは 13 列以上 > 10）。
const OVERFLOW_LINES: usize = 9;

/// 各ステージのチェックポイント注入時刻（talk 起点相対秒・リビール時刻＋丸め余裕）。
pub(super) const T_CHECK: [f64; 7] = [0.12, 0.35, 1.1, 1.8, 3.0, 3.2, 3.4];

/// R10.3 DrawStats 檻: 1 行/列スクロールフレームの `DrawTextLayout` 増分の tight bound。
///
/// **数え方**: 描画側（`viewbox_draw.rs` の Phase 2）はダーティ矩形ごとに描画対象行を
/// すべて描き直す二重ループゆえ、1 フレームの増分は **`ダーティ枚数 × 描画対象行数`**
/// （和ではなく積）である。
///
/// **この fixture での内訳**（validrect 320×122・Yu Gothic UI 28px・行送り 30px＝可視 4 行）:
/// 実測でダーティ **4 枚**・描画対象 **4 行**＝増分 `4 × 4 = 16`。4 枚の内訳は
/// (1) 露出帯 1 枚 ＋ (2) 指紋が変わった行の矩形 2 枚 ＋ (3) スクロールで可視窓の外へ出た行が
/// 残す下端はみ出しインク 1 枚。描画対象 4 行は**可視窓の全行**だった（`first_visible_line`
/// が 7 のフレームで `draw_lines = [7, 8, 9, 10]`）。
///
/// (3) のダーティは、行送りが「字の丈 ＋ 行間 2px」になって行と行の隙間が 2px になった
/// 結果はじめて必要になった（Yu Gothic UI 28px の下端はみ出しは実測 3px ＞ 2px ゆえ、
/// スクロールアウトした行の下端が面の上端に 1px 残る）。旧行送り 35px（隙間 7px）では
/// はみ出しが面外へ抜けていたので (3) は無かった。
///
/// **数値の出所＝この example の実走**（2026-09-06・横書き・k=1.0 に固定して実測）:
/// `draw1=16 draw2=16`（連続する 2 段のスクロールで同値）。実行手順は本 example の
/// ヘッダ doc「k=1.0 に固定して自動判定を走らせる」を参照（高 DPI 機ではプロセスを
/// DPI 非対応にしないと k=2.0 になり、この example の前提が崩れる）。
/// 参考: 同手順の実測で、本 spec のタスク 3.4（スクロールアウト残滓の修正）前は
/// `9 = 3 × 3`、旧行送り 35px 時代の記録値は 3 だった。
///
/// talk 全体の行数（あふれで 12 行超）に伸びない小定数であることが要点（reference:
/// tests/viewbox_scroll_test.rs は可視 8 行の別 fixture で `増分 < 可視行数` を厳密に檻化）。
///
/// **既知の未決**（縦書き）: 縦書き変種の実走では `draw1=9 draw2=16` と段によって変わる。
/// (3) のダーティは「その行に実際にはみ出しがあるとき」だけ立つのに対し、縦書きの
/// ブロック軸（左右）のはみ出しは行ごとに有無が分かれるためで、スクロール深さによる
/// 蓄積ではない。ただし drive.rs の `draw1 == draw2` はこの二つを区別できないので
/// 縦書きではその検査が落ちる。derivation-ledger.md「3.5.2」の「残る 3 件」に登記した。
pub(super) const EXPOSURE_BAND_DRAW_BOUND: u64 = 16;

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
            Gate::Items(n) => rt.state().actor_state(actor).map(|s| s.items().len()) == Some(n),
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
///
/// `duration` は「この cue を喋り終えるのにかかる時間」で、リビール間隔は状態機械側で
/// `interval = duration / グリフ数` として導かれる（`state.rs` の `RevealSchedule`）。
/// **`duration = 0` は「全グリフが `at` で同時に可視」を意味する**ので、typewriter の
/// 進行を観測するこの example では本番と同じ `text_playback_duration`（1 文字あたり
/// `CHAR_NOMINAL_MS` = 50ms・areka-sakura が唯一の定義箇所）を使う。テキスト以外の
/// cue（改行・Clear）は瞬時ゆえ 0。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(text) => text_playback_duration(text),
        _ => 0.0,
    };
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration,
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
