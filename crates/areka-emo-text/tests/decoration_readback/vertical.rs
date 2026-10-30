//! # decoration_readback/vertical.rs — 縦書き 2 方向の読み戻し（task 8.2）
//!
//! 出典 spec: `areka-P0-text-decoration-canon`（要件 **12.1**／**12.2**／**12.3**／**12.4**／
//! **15.4**・design.md「Testing Strategy > 読み戻しテスト」）。
//!
//! ## このモジュールが固定するもの
//!
//! 入口（横書き）と**同じ表・同じ述語**を `vertical_rl` と `vertical_lr` の 2 方向で回し、
//! さらに縦書きにしか無い問い——**線が列のどちら側に出るか**——を実測で固定する。
//!
//! 1. **表示に効く 7 項目**（[`super::VISIBLE_KEYS`]）は、2 方向とも有効にした列と素の列で
//!    画素が違う（要件 12.1／15.4）。
//! 2. **下線と打ち消し線**は、インクの増えた列が連続した 1 本の帯になり、その**側**が
//!    2 方向で同じで、今日の DirectWrite の既定と同じである（要件 12.2／12.3／12.4）。
//! 3. 帯の位置（面の x）も 2 方向それぞれ今日の値で固定する（値が動けば赤）。
//!
//! ## 数える軸が横書きと違う
//!
//! 縦書きでは行が縦に並ぶので、線は**縦のインク**として現れる。行で数えると字のある行すべてに
//! 散らばって帯にならない。ゆえに [`super::added_ink_cols`]（列で数える）と
//! [`super::ink_col_range`]（素の側の字の列の左右端＝側を言うための基準）を使う。
//!
//! ## 裁定との照合は述語にしない（要件 12.4）
//!
//! 完了仕様 `areka-P0-balloon-vertical-canon` の裁定は「下線は列の右側」だが、本仕様は縦書きの
//! 線の位置を **DirectWrite の既定に委ねる**（開発者裁定 2026-09-11）。下線は spec
//! `areka-P0-balloon-text-bugs`（件 3）から areka が塗るが、縦書きは DirectWrite が教える位置と
//! 太さのまま D2D の既定の描き方と同じ向きに回して塗る（D2D 自身の下線との一致は
//! `src/viewbox_draw_underline_tests.rs` の実 GPU の檻が固定・要件 3.4）ので、側と位置は変わらない。
//! ゆえにここは「今日の既定はこの側だ」という**実測の固定**に留め、裁定との一致は述語にしない
//! ——照合結果は `doc/COMPAT_ARCHITECTURE.md` §8 への登記（task 9.1）が書く。
//!
//! ## 装着 → 台本の順（task 8.1 からの申し送り）
//!
//! [`super::run`] の契約どおり、装着を先に、台本の投入を後にする。逆順は本番に実在する窓
//! （装着が次フレームへ委ねられる間に `\f[…]` が届くと既定へ追随しない）を踏み、
//! 文字が素の既定 12 px で描かれる。

use super::{
    SAMPLE, Shot, VISIBLE_KEYS, added_ink_cols, assert_real_font_present, differs, font_cue,
    font_is_installed, ink_col_range, model_with, run, run_with, shoot_pair, single_band, text_cue,
    unbaked_cue,
};

// ══ 縦書きの 2 方向 ════════════════════════════════════════════════════════════════════

/// 列送りの向きが違うだけの 2 方向（要件 12.3）。
///
/// 母数を [`the_vertical_modes_are_the_two_column_directions`] が固定する——表が空になると
/// 以下のループが恒真で緑になる。
const VERTICAL_MODES: &[&str] = &["vertical_rl", "vertical_lr"];

// ══ 線の側 ═════════════════════════════════════════════════════════════════════════════

/// 字の列に対して線が出た側。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Center,
    Right,
}

/// 素の字の列を左・中央・右の 3 等分に割り、帯の中心がどこに落ちるかで側を決める。
///
/// 基準（`column`）は**素の側**で測った字の列（[`ink_col_range`]）——装飾で足された線を
/// 基準に混ぜると、線が自分自身を基準に判定されて向きの意味が消える。
///
/// 「列の外か中か」では決められない: 実測では下線も列の**内側**（字の左端に 1 画素だけ食い込む
/// 位置）に落ち、打ち消し線と同じ「中」になってしまう。逆に中心どうしの大小だけで決めると、
/// 字を貫く打ち消し線が「左右のどちらか」を名乗ってしまう——列の中央を貫く線に左右は無い。
///
/// 3 等分が買うのは**ラベルの意味**であって余裕ではない。実測した裏返りまでの距離:
///
/// - 下線は `Left`。`Right` へは裏返らず、`Center` へ移るまで **+8 画素**（列の最左端に
///   1 画素食い込む位置で、2 方向とも同じオフセット）。ここは境目から遠い。
/// - 打ち消し線は `Center`。`Right` へ **+3 画素**・`Left` へ **−8 画素**で裏返る。
///   列 28 画素に対して 3 画素しかなく、**境目から遠くはない**。棄却した「中心どうしの
///   大小」でも同じ 3 画素（帯の中点 290 に対し列の中心 287.5）で、最悪の余裕は
///   1 画素も改善していない。
///
/// この 3 画素が黙って見過ごされることは無い——側が変わるほど線が動けば、同じ検査が
/// 突き合わせている帯と列の定数が先に赤になる。
fn side_of(band: (u32, u32), column: (u32, u32)) -> Side {
    // 半画素を避けるため全て 2 倍の目盛りで測る（帯の中心＝両端の和）。
    let center = i64::from(band.0) + i64::from(band.1);
    let (lo, hi) = (2 * i64::from(column.0), 2 * i64::from(column.1));
    let (offset, width) = (center - lo, hi - lo);
    assert!(width > 0, "字の列 {column:?} に幅が無い");
    if 3 * offset < width {
        Side::Left
    } else if 3 * offset > 2 * width {
        Side::Right
    } else {
        Side::Center
    }
}

/// 素の列と装飾した列を撮り、インクの増えた列の帯・素の字の列・側を返す。
fn measure(mode: &str, tokens: &[&str], what: &str) -> ((u32, u32), (u32, u32), Side) {
    let (plain, styled) = shoot_pair(Some(mode), tokens);
    let band = single_band(&added_ink_cols(&plain, &styled), what);
    let column = ink_col_range(&plain);
    (band, column, side_of(band, column))
}

// ══ 今日の値（実測で固定する定数） ═════════════════════════════════════════════════════
//
// いずれも `Yu Gothic UI`・`font.height,28`・面 320x200・validrect 5 画素内側での実測。
// 位置も側も DirectWrite の既定に委ねてある（要件 5.7／12.2）。下線は areka が塗るが、D2D の
// 既定の描き方と同じ写し方なので値は変わらない（spec `areka-P0-balloon-text-bugs` 要件 3.4）。
// 値が動いたら描画基盤側の変化を疑い、定数を書き換える前に裁定を仰ぐこと。

/// 下線が出る側（2 方向とも同じ・要件 12.2／12.3）——列の**左**（実測では字の列の左端に
/// 1〜2 画素だけ食い込む位置で、列の外へは出ない）。
///
/// 完了仕様 `areka-P0-balloon-vertical-canon` の裁定は「列の右側」で、今日の DirectWrite の
/// 既定はその**反対側**である。本仕様は縦書きの線の位置を DirectWrite に委ねる裁定なので、
/// ここは実測を固定するに留める（照合の登記は task 9.1）。
const UNDERLINE_SIDE: Side = Side::Left;

/// 打ち消し線が出る側（2 方向とも同じ）——字を貫くので列の中央 1/3。
const STRIKE_SIDE: Side = Side::Center;

/// `vertical_rl` の素の字の列（供給面 local の x の左右端）。
const COLUMN_RL: (u32, u32) = (274, 301);

/// `vertical_lr` の素の字の列。
const COLUMN_LR: (u32, u32) = (2, 30);

/// `vertical_rl` で `\f[underline,1]` がインクを増やす列の帯。
const UNDERLINE_BAND_RL: (u32, u32) = (275, 276);

/// `vertical_lr` で同上。
const UNDERLINE_BAND_LR: (u32, u32) = (3, 4);

/// `vertical_rl` で `\f[strike,1]` がインクを増やす列の帯。
const STRIKE_BAND_RL: (u32, u32) = (289, 291);

/// `vertical_lr` で同上。
const STRIKE_BAND_LR: (u32, u32) = (18, 19);

// ══ 表 の母数 ══════════════════════════════════════════════════════════════════════════

#[test]
fn the_vertical_modes_are_the_two_column_directions() {
    assert_eq!(
        VERTICAL_MODES,
        &["vertical_rl", "vertical_lr"],
        "縦書きは列送りの向きが違う 2 方向（要件 12.3）"
    );
    assert_eq!(
        VISIBLE_KEYS.len(),
        7,
        "縦書きも横書きと同じ 7 項目の表を回す（要件 12.1／15.4）"
    );
}

// ══ 1. 表示に効く 7 項目は 2 方向とも画素を変える（要件 12.1／15.4） ═══════════════════

/// 較正: 装飾入りの入口（`layout_styled`／`render_styled`）を従来の入口へ戻すと、2 方向 ×
/// 7 項目とも画素が素の列と一致して赤になる。
#[test]
fn each_visible_key_changes_the_pixels_in_both_vertical_modes() {
    assert_real_font_present();
    for mode in VERTICAL_MODES {
        for (key, tokens) in VISIBLE_KEYS {
            let (plain, styled) = shoot_pair(Some(mode), tokens);
            assert!(
                differs(&plain, &styled),
                "{mode}: `\\f[{}]` を有効にしても画素が 1 バイトも変わらない（本文 {SAMPLE}）",
                tokens.join(","),
            );
            assert!(
                styled.ink_count() > 0,
                "{mode}: `\\f[{key}]` の列にインクが 1 画素も無い（字が消えたのを「違う」と数えていないか）"
            );
        }
    }
}

// ══ 2. 線が列のどちら側に出るか（要件 12.2／12.3／12.4） ═══════════════════════════════

/// `\f[underline,1]` は 2 方向とも**同じ側**に連続した 1 本の縦の帯を足す。
///
/// 「2 方向で同じ」だけでは、両方が同じように壊れても緑になる。ゆえに側そのものを
/// [`UNDERLINE_SIDE`] で、帯の位置を方向ごとの定数で固定し、片方だけ動いても赤になるようにする。
#[test]
fn underline_lands_on_the_same_side_of_the_column_in_both_vertical_modes() {
    assert_real_font_present();
    let (band_rl, column_rl, side_rl) = measure("vertical_rl", &["underline", "1"], "underline rl");
    let (band_lr, column_lr, side_lr) = measure("vertical_lr", &["underline", "1"], "underline lr");

    assert_eq!(
        (band_rl, column_rl),
        (UNDERLINE_BAND_RL, COLUMN_RL),
        "vertical_rl の下線の帯／字の列が今日の値から動いた"
    );
    assert_eq!(
        (band_lr, column_lr),
        (UNDERLINE_BAND_LR, COLUMN_LR),
        "vertical_lr の下線の帯／字の列が今日の値から動いた"
    );
    assert_eq!(
        side_rl, side_lr,
        "下線の出る側が 2 方向で違う（要件 12.3——列送りの向きが違うだけで列の中での位置は同じ）。\
         rl: 帯 {band_rl:?}／列 {column_rl:?}・lr: 帯 {band_lr:?}／列 {column_lr:?}"
    );
    assert_eq!(
        side_rl, UNDERLINE_SIDE,
        "下線の出る側が今日の DirectWrite の既定から動いた。縦書きの線は D2D の既定の描き方と\
         同じ写し方で塗るので、定数を書き換える前に写し方か描画基盤側の変化を疑い裁定を仰ぐこと\
         （要件 5.7／12.2・spec `areka-P0-balloon-text-bugs` 要件 3.4）"
    );
}

/// `\f[strike,1]` は 2 方向とも同じ側に連続した 1 本の縦の帯を足す。
///
/// 打ち消し線は字を貫くので帯は列の**中央 1/3**、下線は列の**左端**（[`UNDERLINE_SIDE`]）——
/// `SetUnderline` と `SetStrikethrough` を取り違えれば側も帯の位置も入れ替わって両方が赤になる。
#[test]
fn strike_lands_on_the_same_side_of_the_column_in_both_vertical_modes() {
    assert_real_font_present();
    let (band_rl, column_rl, side_rl) = measure("vertical_rl", &["strike", "1"], "strike rl");
    let (band_lr, column_lr, side_lr) = measure("vertical_lr", &["strike", "1"], "strike lr");

    assert_eq!(
        (band_rl, column_rl),
        (STRIKE_BAND_RL, COLUMN_RL),
        "vertical_rl の打ち消し線の帯／字の列が今日の値から動いた"
    );
    assert_eq!(
        (band_lr, column_lr),
        (STRIKE_BAND_LR, COLUMN_LR),
        "vertical_lr の打ち消し線の帯／字の列が今日の値から動いた"
    );
    assert_eq!(
        side_rl, side_lr,
        "打ち消し線の出る側が 2 方向で違う（要件 12.3）。\
         rl: 帯 {band_rl:?}／列 {column_rl:?}・lr: 帯 {band_lr:?}／列 {column_lr:?}"
    );
    assert_eq!(
        side_rl, STRIKE_SIDE,
        "打ち消し線の出る側が今日の DirectWrite の既定から動いた（要件 5.7／12.2）。\
         `Center` は帯の中心が字の列の中央 1/3 に落ちること——下線（左の 1/3）と取り違えれば赤になる"
    );
}

// ══ 較正——差の検出が空振りしていないこと（要件 15.7） ═════════════════════════════════

/// 装飾を焼かない経路で描くと 2 方向とも差が消える。
///
/// 同じトークン列を**装飾ではない運搬名**で流すと `state.rs` の自己選別が読み飛ばして装飾を
/// 1 つも焼かない。cue の本数・並び・再生時間は装飾ありの台本と同一なので、上の「違う」が
/// **cue が 1 本増えたこと**ではなく**装飾を焼いたこと**に由来することを示す。
#[test]
fn tokens_that_never_reach_the_decoration_path_paint_the_plain_pixels_in_both_vertical_modes() {
    assert_real_font_present();
    for mode in VERTICAL_MODES {
        let plain = run(Some(mode), &[text_cue(SAMPLE)]);
        for (key, tokens) in VISIBLE_KEYS {
            let unbaked = run(Some(mode), &[unbaked_cue(tokens), text_cue(SAMPLE)]);
            assert!(
                !differs(&plain, &unbaked),
                "{mode}: `{key}` のトークン列を装飾ではない運搬名で流したのに画素が変わった——\
                 装飾を焼かない経路が装飾を焼いている",
            );
            // 同じトークン列を装飾の運搬名で流せば違う（対照が空振りでないこと）。
            let baked = run(Some(mode), &[font_cue(tokens), text_cue(SAMPLE)]);
            assert!(
                differs(&plain, &baked),
                "{mode}: `{key}` を装飾の運搬名で流しても画素が変わらない（対照の意味が消える）",
            );
        }
    }
}

// ══ 3. 括弧の上下・列の真ん中・括弧と次の字の空き（spec `emo-text-dwrite-fidelity`） ═══
//
// 出典 spec: `emo-text-dwrite-fidelity`（要件 1.1〜1.5・2.3・6.1・design.md「Testing Strategy >
// 実 GPU の読み戻し」の G1〜G3）。バルーン定義はぱすた（`ghost_dev`）と同じ形を基にする。
// 字の枠は行の矩形の中で `PASTA_HEIGHT` ごと——供給面は `validrect` の内側だけを持つので、
// 列の先頭の字の枠の上端は面の y=0。

/// ぱすたのバルーン定義の書体名（候補列のまま・GDI の名前「游ゴシック Medium」が中太 500 で当たる）。
const PASTA_FONT: &str =
    "游ゴシック Medium,Yu Gothic Medium,游ゴシック,Yu Gothic,BIZ UDPゴシック,メイリオ";

/// ぱすたのバルーン定義の文字の大きさ（image px）＝字の枠の高さ。
const PASTA_HEIGHT: u32 = 20;

/// 「「」の下端と「繋」の上端の空きの期待値（px）——書体の寸法から決まる値。
///
/// 出どころ（design.md「Testing Strategy」の G3）: 游ゴシックの縦書き用の「「」（字形 7913）の墨の
/// 下端は字の枠の上から 18.9px（research.md §10.1）ゆえ下の空きは 20−18.9＝約 1.1px、「繋」の
/// 上の余白は約 1.0px、合わせて約 2.1px。手本（Chromium の並べ）は
/// 開き括弧の位置をずらさないので、字の枠どおりに並べればこの空きになる。墨の端は画素の行に
/// 丸まるので、「「」の最後の墨の行と「繋」の最初の墨の行の番号の差として数え ±1px で比べる。
const BRACKET_GAP_PX: i64 = 2;

/// 太さの 3 通り（要件 1.5）——名札・書体名・追加の行。
const WEIGHTS: &[(&str, &str, &str)] = &[
    ("Regular", "游ゴシック", ""),
    ("中太", PASTA_FONT, ""),
    ("太字", PASTA_FONT, "font.bold,1\n"),
];

/// 開き括弧類（要件 1.1）——墨が字の枠の下半分に来る。
const OPENING: &[&str] = &["「", "『", "（"];

/// 閉じ括弧（要件 1.2）——墨が字の枠の上半分のまま。
const CLOSING: &[&str] = &["」"];

/// 游ゴシックが無ければ赤で止める（代替の書体のまま緑にしない）。
fn assert_yu_gothic_present() {
    for name in ["游ゴシック", "游ゴシック Medium"] {
        assert!(
            font_is_installed(name),
            "{name} がこの機械に無い。括弧の檻は当該書体の実描画を前提にしている"
        );
    }
}

/// 縦書きの 1 列を、ぱすたの形のバルーン定義で撮る。
fn shoot_column(mode: &str, font: &str, extra: &str, body: &str) -> Shot {
    let model = model_with(font, PASTA_HEIGHT, &format!("writing_mode,{mode}\n{extra}"));
    let shot = run_with(&model, &[text_cue(body)]);
    assert!(
        shot.ink_count() > 0,
        "{mode}: 「{body}」の列にインクが無い（前提）"
    );
    shot
}

/// 墨がある画素か——α が 0 でなければ墨とする。
///
/// 20px の括弧は線が 1px 前後と細く、`Shot::is_ink`（α≥128）では横棒が消えて字の上下端を
/// 取り違える（游ゴシック Regular の「「」は α≥128 の行が 1 本しか残らない）。
fn has_ink(shot: &Shot, x: u32, y: u32) -> bool {
    shot.bytes[((y * shot.width + x) * 4 + 3) as usize] != 0
}

/// 墨のある行（y）を、上から連続した塊に分ける（字ごとの墨の上下端）。
fn ink_row_runs(shot: &Shot) -> Vec<(u32, u32)> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for y in (0..shot.height).filter(|&y| (0..shot.width).any(|x| has_ink(shot, x, y))) {
        match runs.last_mut() {
            Some(last) if last.1 + 1 == y => last.1 = y,
            _ => runs.push((y, y)),
        }
    }
    runs
}

/// 行の範囲 `rows` の中でインクのある列の左右端。
fn ink_cols_in(shot: &Shot, rows: (u32, u32)) -> (u32, u32) {
    let cols: Vec<u32> = (0..shot.width)
        .filter(|&x| (rows.0..=rows.1).any(|y| has_ink(shot, x, y)))
        .collect();
    assert!(!cols.is_empty(), "行 {rows:?} にインクのある列が無い");
    (cols[0], cols[cols.len() - 1])
}

/// 1 字だけの列の墨の上下端（1 つの塊のはず）。
fn single_glyph_rows(shot: &Shot, what: &str) -> (u32, u32) {
    let runs = ink_row_runs(shot);
    assert!(!runs.is_empty(), "{what}: 墨が無い");
    (runs[0].0, runs[runs.len() - 1].1)
}

/// 「「繋」の列を撮り、「「」と「繋」の墨の上下端を返す（上から 1 つ目の塊が「「」）。
fn bracket_then_kanji(mode: &str) -> (Shot, (u32, u32), (u32, u32)) {
    let shot = shoot_column(mode, PASTA_FONT, "", "「繋");
    let runs = ink_row_runs(&shot);
    assert!(
        runs.len() >= 2,
        "{mode}: 「「」と「繋」の墨が空いた行で分かれていない（空き 0）: {runs:?}"
    );
    let bracket = runs[0];
    let kanji = (runs[1].0, runs[runs.len() - 1].1);
    (shot, bracket, kanji)
}

/// G1（要件 1.1・1.2・1.5・6.1）: 縦書き 2 方向・3 通りの太さで、開き括弧類の墨は字の枠の
/// 下半分、閉じ括弧の墨は上半分にある。
///
/// 較正: 列のレシピ（`VERTICAL_FEATURES`）を外して既定の機能の組へ戻すと、開き括弧の墨が
/// 半 em 上へずれて（research.md §10.1・上から 3.0〜8.9px）赤になる。
#[test]
fn opening_brackets_sit_in_the_lower_half_and_closing_in_the_upper_half_of_the_glyph_frame() {
    assert_yu_gothic_present();
    let half = PASTA_HEIGHT / 2;
    for mode in VERTICAL_MODES {
        for (weight, font, extra) in WEIGHTS {
            for ch in OPENING {
                let shot = shoot_column(mode, font, extra, ch);
                let rows = single_glyph_rows(&shot, ch);
                assert!(
                    rows.0 >= half,
                    "{mode}・{weight}: 「{ch}」の墨が字の枠の下半分に無い（墨の行 {rows:?}・\
                     下半分は y≥{half}）——開き括弧が上へずれている（要件 1.1・1.5）"
                );
            }
            for ch in CLOSING {
                let shot = shoot_column(mode, font, extra, ch);
                let rows = single_glyph_rows(&shot, ch);
                assert!(
                    rows.1 < half,
                    "{mode}・{weight}: 「{ch}」の墨が字の枠の上半分に無い（墨の行 {rows:?}・\
                     上半分は y<{half}）——閉じ括弧が動いた（要件 1.2）"
                );
            }
        }
    }
}

/// G2（要件 1.3）: 「「」の墨の左右の中心は「繋」の墨の左右の中心と ±1px（列の真ん中）。
#[test]
fn opening_bracket_is_centred_in_the_column_like_the_next_kanji() {
    assert_yu_gothic_present();
    for mode in VERTICAL_MODES {
        let (shot, bracket, kanji) = bracket_then_kanji(mode);
        let (b, k) = (ink_cols_in(&shot, bracket), ink_cols_in(&shot, kanji));
        // 半画素を避けるため 2 倍の目盛りで比べる（中心＝両端の和）。
        let diff = (i64::from(b.0) + i64::from(b.1)) - (i64::from(k.0) + i64::from(k.1));
        assert!(
            diff.abs() <= 2,
            "{mode}: 「「」の墨の列 {b:?} と「繋」の墨の列 {k:?} の中心の差が ±1px を超えた\
             （差 {}px・要件 1.3）",
            diff as f64 / 2.0
        );
    }
}

/// G3（要件 1.4）: 「「」の墨の下端と「繋」の墨の上端の空きが [`BRACKET_GAP_PX`] と ±1px。
#[test]
fn gap_between_opening_bracket_and_next_kanji_follows_the_font_metrics() {
    assert_yu_gothic_present();
    for mode in VERTICAL_MODES {
        let (_, bracket, kanji) = bracket_then_kanji(mode);
        let gap = i64::from(kanji.0) - i64::from(bracket.1);
        assert!(
            (gap - BRACKET_GAP_PX).abs() <= 1,
            "{mode}: 「「」の墨 {bracket:?} と「繋」の墨 {kanji:?} の空きが {gap}px——\
             書体の寸法から決まる約 2.1px と ±1px で合わない（要件 1.4）"
        );
    }
}
