use super::{
    ActorKey, CueCommand, LINE1, LINE2, LINE3, TalkCue, TextLayerRuntime, text_playback_duration,
};

// ---------------------------------------------------------------------------
// シナリオ定義（boot.pasta 起動朝 挨拶由来のハードコード cue 列・注入時刻付き）
// ---------------------------------------------------------------------------
//
// 台詞は fixture スクリプト boot.pasta「起動朝」シーン（むらさき=\0／エモ=\1）由来。
// 共有 fixture の幾何に合わせて刻んである。
//
// 幾何（`balloons0s.txt` の validrect top 46／bottom −56／left 36／right −44・枠画像 400×224）:
// 描画範囲は絶対 (36,46)-(356,168) ＝ 320×122 image px。font 28px ゆえ行送りは
// 字の丈 28 ＋ 行間 2 ＝ **30px**。
// - 横書きの行容量: 行 i の下端 = 46 + 30i + 28 ≤ 168 → i = 0..3 の **4 行**
//   （4 行目 164 ≤ 168・5 行目 194 > 168）。
// - 縦書き（vertical 変種）の列容量: 列 i の左端 = 356 − 28 − 30i ≥ 36 →
//   i = 0..9 の **10 列**（= `floor((320 − 28) / 30) + 1`）。

/// シナリオ全体の watchdog（talk 起点相対秒・超過は FAIL）。
pub(super) const WATCHDOG_SECS: f64 = 30.0;

/// \1（kero）の台詞（7 グリフ・複数 actor 振り分けの観測対象）。
const KERO_LINE: &str = "朝から元気だね";
/// あふれ→スクロール誘発用の短行（2 グリフ×`OVERFLOW_LINES` 行）。
///
/// 2 文字のままにする: 1 文字だと「行が完成する」時刻と「次の行が開く」時刻が同じになり、
/// プラトーが 1 つに潰れて drive.rs の完成プラトーの 2 通りの選び方が同じ点を選んでしまう。
const SHORT_LINE: &str = "ほな";
/// あふれ誘発の短行数。
///
/// drive.rs の C8 は「送り出される行がどちらも短行」である 2 段を比べる（統制された比較）。
/// 縦書きでその 2 段を採るには、先頭可視列 9 の完成プラトー（短行 12 本目が完成し 13 本目が開く点）
/// まで要るので、短行は **13 本**。
/// - 横書き: 既存 3 行 ＋ 13 行 = **16 行** > 容量 4。
/// - 縦書き: 既存 7 列 ＋ 13 列 = **20 列** > 容量 10。
///   長い 3 行が縦書きで占める列数 7 は、drive.rs が短行の流入前（`t = 1.95`）の配置列数として
///   実測で求める（この定数からは導かない）。
const OVERFLOW_LINES: usize = 13;

/// 各ステージのチェックポイント注入時刻（talk 起点相対秒・リビール時刻＋丸め余裕）。
///
/// stage 4 以降: 短行は `at = 2.0` から 1 行 0.1 秒（2 文字 × `CHAR_NOMINAL_MS` 50ms）で順にリビールされ、
/// 13 本のリビールは 3.3 秒に終わる。以降は 0.1 秒刻みで、C5 のチェック 3.4 → \0 の Clear 注入 3.5 →
/// C6 のチェック 3.6 → \1 の Clear 注入 3.7 → C7 のチェック 3.8。
pub(super) const T_CHECK: [f64; 7] = [0.12, 0.35, 1.1, 1.8, 3.4, 3.6, 3.8];

/// DrawStats 檻: 1 行/列スクロールフレームの `DrawTextLayout` 増分の tight bound。
///
/// 描画側（`viewbox_draw_render.rs`）はダーティ矩形ごとに、その矩形と行送り軸で交差する行だけを描く。
/// ゆえに 1 フレームの増分は矩形ごとの交差行数の和 `Σ|d.lines|`。
///
/// この fixture での内訳（validrect 320×122・Yu Gothic UI 28px・行送り 30px・ダーティ矩形はガード 1px を
/// 四辺へ加えた実測値）:
///
/// 横書き（可視 4 行・2 段とも同じ形）＝ **2 + 1 + 2 = 5**
/// - 露出帯（面下端の 30px ＋ ガード＝`y 91..122`）… 交差 2 行（流入した最終行と、下端はみ出し 3px ＋
///   ガードで帯へ 1px 食い込む直前行）
/// - スクロールで可視窓の外へ出た行が面上端へ残す残滓（`y 0..3`）… 交差 1 行（先頭可視行）
/// - 指紋が変わった行＝流入した最終行の矩形（`y 89..122`）… 交差 2 行（自身と直前行）
///
/// 縦書き（可視 10 列・2 段とも同じ形）＝ **1 + 2 = 3**
/// - 露出帯（面左端の 30px ＋ ガード＝`x 0..31`）… 交差 1 列（流入した最終列のみ）
/// - 流入した最終列の矩形（`x 20..51`）… 交差 2 列（自身と直前列）
/// - 残滓の矩形は立たない。送り出される列が短行「ほな」の列でブロック軸（左右）のはみ出しが 0 だから
///   （漢字を含む列は 1px 超はみ出す。だから比べる 2 段は送り出される行がどちらも短行のものに限る）
///
/// 残滓のダーティが要るのは、行間 2px に対し Yu Gothic UI 28px の下端はみ出しが実測 3px で、
/// スクロールアウトした行の下端が面の上端に残るため。
///
/// 数値の出所はこの example の実走（k=1.0 に固定）: 横書き `draw1=5 draw2=5`・縦書き `draw1=3 draw2=3`。
/// 上限は両モードの最大値。k=1.0 への固定は本 example のヘッダ doc を参照（高 DPI 機ではプロセスを
/// DPI 非対応にしないと k=2.0 になり前提が崩れる）。
///
/// talk 全体の行数（あふれで横 16 行・縦 20 列）に伸びない小定数であることが要点。
/// `増分 < 可視行数` の厳密な檻は `tests/viewbox_scroll_test.rs`（可視 8 行の別の検体）が持つ。
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

/// \0 の gate（ステージ順）。items 数: L1=6 → +1+10=17 → +1+8=26 → 26 ＋ `OVERFLOW_LINES` × 3 = 65 → Clear
/// （短行 1 本＝改行マーカー 1 ＋ グリフ 2）。
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
/// `duration` は cue を喋り終えるのにかかる時間で、リビール間隔は状態機械側で
/// `duration / グリフ数` と導かれる（`state.rs` の `RevealSchedule`）。`duration = 0` は「全グリフが `at` で同時に可視」なので、
/// typewriter の進行を観測するここでは本番と同じ `text_playback_duration`（1 文字あたり
/// `CHAR_NOMINAL_MS` = 50ms）を使う。テキスト以外の cue（改行・Clear）は瞬時ゆえ 0。
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
