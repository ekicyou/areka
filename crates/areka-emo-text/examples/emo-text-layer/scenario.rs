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
/// あふれ→スクロール誘発用の短行（2 グリフ×`OVERFLOW_LINES` 行・先頭行より行内範囲が確実に短い）。
///
/// **2 文字のままにする理由**: 1 文字にすると「その行が完成する」時刻と「次の行が開く」時刻が
/// 同じになり、プラトーが 1 つに潰れる。すると drive.rs の完成プラトーの選び方（改行遅延に
/// 従う新しい選び方と、行を到着即時に開いていた頃の旧い選び方）が同じ点を選んでしまい、
/// 選び直しの是正が緑のまま意味を失う。
const SHORT_LINE: &str = "ほな";
/// あふれ誘発の短行数。
///
/// **導出**（design §13.4 の決定 4）: drive.rs の C8 は「送り出される行がどちらも短行」である
/// 2 段を比べる（統制された比較）。縦書きでその 2 段を採るには、先頭可視列 9 の完成プラトー
/// ——短行 12 本目（0 始まりで 11）が完成し 13 本目が開く点——まで要る。ゆえに短行は **13 本**。
/// - 横書き: 既存 3 行 ＋ 13 行 = **16 行** > 容量 4（先頭可視行は 12 まで進む）。
/// - 縦書き: 既存 7 列 ＋ 13 列 = **20 列** > 容量 10（先頭可視列は 10 まで進む）。
///   長い 3 行が縦書きで占める列数 7 は、短行が流入する前（`t = 1.95`・可視グリフ 24）の
///   配置列数として drive.rs が実測で求める（この定数からは導かない）。
const OVERFLOW_LINES: usize = 13;

/// 各ステージのチェックポイント注入時刻（talk 起点相対秒・リビール時刻＋丸め余裕）。
///
/// **stage 4 以降の導出**: 短行は `at = 2.0` から 1 行 0.1 秒（2 文字 × `CHAR_NOMINAL_MS` 50ms）で
/// 順にリビールされるので、13 本のリビールは `2.0 + 13 × 0.1 = 3.3` 秒に終わる。以降は
/// 0.1 秒刻みで、C5 のチェック 3.4 → \0 の Clear 注入 3.5 → C6 のチェック 3.6 →
/// \1 の Clear 注入 3.7 → C7 のチェック 3.8。
pub(super) const T_CHECK: [f64; 7] = [0.12, 0.35, 1.1, 1.8, 3.4, 3.6, 3.8];

/// R10.3 DrawStats 檻: 1 行/列スクロールフレームの `DrawTextLayout` 増分の tight bound。
///
/// **数え方**: 描画側（`viewbox_draw.rs` の Phase 2）はダーティ矩形ごとに、その矩形と行送り軸で
/// 交差する行だけを描く。ゆえに 1 フレームの増分は **矩形ごとの交差行数の和** `Σ|d.lines|`
/// である。
///
/// **この fixture での内訳**（validrect 320×122・Yu Gothic UI 28px・行送り 30px・ダーティ矩形は
/// ガード 1px を四辺へ加えた実測値・2026-09-06 に両モードで採取）:
///
/// 横書き（可視 4 行・`first_visible_line` 10 → 11 → 12 の 2 段とも同じ形）＝ **2 + 1 + 2 = 5**
/// - 露出帯（面下端の 30px ＋ ガード＝`y 91..122`）… 交差 **2 行**（流入した最終行と、その直前行——
///   直前行は下端はみ出し 3px ＋ ガードで帯へ 1px 食い込む）
/// - スクロールで可視窓の外へ出た行が面上端へ残す残滓（`y 0..3`）… 交差 **1 行**（先頭可視行）
/// - 指紋が変わった行＝流入した最終行の矩形（`y 89..122`）… 交差 **2 行**（自身と直前行）
///
/// 縦書き（可視 10 列・先頭可視列 8 → 9 → 10 の 2 段とも同じ形）＝ **1 + 2 = 3**
/// - 露出帯（面左端の 30px ＋ ガード＝`x 0..31`）… 交差 **1 列**（流入した最終列のみ）
/// - 流入した最終列の矩形（`x 20..51`）… 交差 **2 列**（自身と直前列）
/// - 残滓の矩形は**立たない**——送り出される列が短行「ほな」の列で、ブロック軸（左右）の
///   はみ出しが 0 だから（漢字を含む列は 1px 超のはみ出しを持つ。だから比べる 2 段は
///   「送り出される行がどちらも短行」のものへ限る＝統制された比較・drive.rs を参照）
///
/// 残滓のダーティは、行送りが「字の丈 ＋ 行間 2px」になって行と行の隙間が 2px になった
/// 結果はじめて必要になった（Yu Gothic UI 28px の下端はみ出しは実測 3px ＞ 2px ゆえ、
/// スクロールアウトした行の下端が面の上端に残る）。旧行送り 35px（隙間 7px）では
/// はみ出しが面外へ抜けていたので残滓は無かった。
///
/// **数値の出所＝この example の実走**（2026-09-06・k=1.0 に固定して横書き・縦書きとも実測）:
/// 横書き `draw1=5 draw2=5`・縦書き `draw1=3 draw2=3`（連続する 2 段のスクロールで同値）。
/// 上限は両モードの最大値を採る。実行手順は本 example のヘッダ doc
/// 「k=1.0 に固定して自動判定を走らせる」を参照（高 DPI 機ではプロセスを DPI 非対応に
/// しないと k=2.0 になり、この example の前提が崩れる）。
///
/// **履歴**（現行の数え方とは無関係・当時の記録として残す）: 矩形ごとに描画対象行を全部
/// 描き直していた頃（〜2026-09-06 午前）は増分が枚数と行数の積になり、横書き 16・縦書きは
/// 段によって 9／16 と分かれていた（送り出される行のはみ出しの有無で矩形が 3 枚／4 枚に
/// 分かれたため）。さらに前、タスク 3.4（スクロールアウト残滓の修正）前は 9、旧行送り 35px
/// 時代の記録値は 3 だった。
///
/// talk 全体の行数（あふれで横 16 行・縦 20 列）に伸びない小定数であることが要点（reference:
/// tests/viewbox_scroll_test.rs は可視 8 行の別 fixture で `増分 < 可視行数` を厳密に檻化）。
pub(super) const EXPOSURE_BAND_DRAW_BOUND: u64 = 5;

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

/// \0 の gate（ステージ順）。items 数: L1=6 → +1+10=17 → +1+8=26 → +13×(1+2)=65 → Clear。
/// 末項は「26 ＋ `OVERFLOW_LINES` × 3」（短行 1 本＝改行マーカー 1 ＋ グリフ 2）。
pub(super) const GATE_SAKURA: [Gate; 7] = [
    Gate::Items(6),
    Gate::Items(6),
    Gate::Items(17),
    Gate::Items(26),
    Gate::Items(65),
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
        5 => vec![cue("0", 3.5, CueCommand::Clear)],
        6 => vec![cue("1", 3.7, CueCommand::Clear)],
        _ => Vec::new(),
    }
}
