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
//! 線の位置を **DirectWrite の既定に委ねる**（開発者裁定 2026-09-11・areka は線を描き分けない）。
//! ゆえにここは「今日の既定はこの側だ」という**実測の固定**に留め、裁定との一致は述語にしない
//! ——照合結果は `doc/COMPAT_ARCHITECTURE.md` §8 への登記（task 9.1）が書く。
//!
//! ## 装着 → 台本の順（task 8.1 からの申し送り）
//!
//! [`super::run`] の契約どおり、装着を先に、台本の投入を後にする。逆順は本番に実在する窓
//! （装着が次フレームへ委ねられる間に `\f[…]` が届くと既定へ追随しない）を踏み、
//! 文字が素の既定 12 px で描かれる。

use super::{
    SAMPLE, VISIBLE_KEYS, added_ink_cols, assert_real_font_present, differs, font_cue,
    ink_col_range, run, shoot_pair, single_band, text_cue, unbaked_cue,
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
// 位置も側も DirectWrite の既定に委ねてあり areka 側で描き分けない（要件 5.7／12.2）。
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
        "下線の出る側が今日の DirectWrite の既定から動いた。areka は線を描き分けないので、\
         定数を書き換える前に描画基盤側の変化を疑い裁定を仰ぐこと（要件 5.7／12.2）"
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
