//! # decoration_readback_test — 文字装飾の横書き読み戻し（task 8.1）
//!
//! 出典 spec: `areka-P0-text-decoration-canon`（要件 **15.4**／**15.7**／**15.8**／
//! **5.1**／**5.9**／**6.1**・design.md「Testing Strategy > 読み戻しテスト」）。
//!
//! ## このテストが固定するもの
//!
//! `\f[…]` の指定が**実際に画面へ出る**こと——机上の状態遷移ではなく、実フォントを
//! 実 GPU（WARP 可）で描いた画素の読み戻しで確かめる。述語は 3 種:
//!
//! 1. **表示に効く 7 項目**（`name`・`height`・`color`・`bold`・`italic`・`underline`・
//!    `strike`）は、有効にした行と素の行で**画素が違う**（要件 5.1）。
//! 2. **下線と打ち消し線**は、インクの増えた行が連続した 1 本の帯になり、その位置が
//!    今日の DirectWrite の既定と同じである（要件 5.1・位置は areka で描き分けない）。
//! 3. **語彙のみの 3 項目**（`sub`・`sup`・`outline`）は、有効にしても**画素が同じ**
//!    （要件 5.9／6.1——DirectWrite の範囲指定で表せないので表示は変えない）。
//!
//! ## 置き方（1 バイナリ）
//!
//! `tests/` の直下に置いた `.rs` は 1 本ずつ独立した実行ファイルになる。ゆえに共有ヘルパと
//! 縦書きの分（task 8.2）は**入口から取り込むサブディレクトリ**へ置き、本ファイルだけを
//! 実行ファイルにする（`crates/dola/tests/compile.rs`＋`tests/compile/` と同じ作法）。
//! src 側の `<stem>_<モジュール名>.rs` の規則は `tests/` には適用しない。
//!
//! ## 較正（要件 15.7）
//!
//! 「画素が違う」を述べる述語は、比べる 2 枚がそもそも別物なら**何をしても緑**になる。
//! ゆえに 2 本の対照を置く:
//!
//! - [`the_same_script_paints_the_same_pixels_twice`]——同じ台本を 2 度描くと 1 バイトも
//!   違わない（読み戻しに実行ごとのばらつきが無い）。
//! - [`tokens_that_never_reach_the_decoration_path_paint_the_plain_pixels`]——同じトークン列を
//!   装飾を焼かない運搬名で流すと差が消える（差は装飾を焼いたことに由来する）。
//!
//! 「画素が同じ」を述べる語彙のみ 3 項目の述語も、同じ窓・同じ本文で `bold` を有効にすると
//! 画素が変わることを**同じ検査の中で**示す（そちらが赤にならない限り「同じ」は意味を持たない）。
//!
//! ## 常時実行（要件 15.8）
//!
//! GPU は headless の実資源（`GraphicsCore::new()`・WARP 可）、時刻は注入、外部の機材も
//! 32 ビットの実行も要らない。

#[path = "decoration_readback/mod.rs"]
mod support;

use support::{
    ALT_FONT, FONT_HEIGHT, MISSING_FONT, REAL_FONT, SAMPLE, VISIBLE_KEYS, added_ink_rows,
    assert_real_font_present, differs, font_cue, font_is_installed, run, shoot_pair, single_band,
    text_cue, unbaked_cue,
};

// ══ 今日の値（実測で固定する定数） ═════════════════════════════════════════════════════

/// `\f[underline,1]` でインクが増える行の帯（供給面 local の y・`Yu Gothic UI` の
/// `font.height,28`・横書き）。
///
/// 位置は DirectWrite の既定に委ねてあり、areka 側で線を描き分けない（要件 5.1・
/// `doc/COMPAT_ARCHITECTURE.md` §8 の登記対象）。ここは**今日の値**を実測で固定するだけで、
/// 値が動いたら赤にして裁定を仰ぐ。
const UNDERLINE_BAND: (u32, u32) = (33, 33);

/// 同上・`\f[strike,1]`（打ち消し線は字の中ほどを通る）。
///
/// 2 本の帯が別の位置にあることは、この 2 つの定数がそれぞれ**実測と突き合わされている**
/// ことで担保される——`SetUnderline` と `SetStrikethrough` を取り違えれば両方の検査が赤になる。
const STRIKE_BAND: (u32, u32) = (18, 19);

/// 語彙として受理するが表示は変えない 3 項目（要件 5.9／6.1）。
const VOCABULARY_ONLY_KEYS: &[(&str, &[&str])] = &[
    ("sub", &["sub", "1"]),
    ("sup", &["sup", "1"]),
    ("outline", &["outline", "1"]),
];

// ══ 実フォントの門 ═════════════════════════════════════════════════════════════════════

/// 門そのものの較正: 実在しない名前では門が**閉じる**。
///
/// 門は本番の判断関数 `FontCatalog::family_for` で判定するので、門が「常に開く」実装
/// （たとえば候補列をそのまま返す縮退）に変わればここが赤になる。
#[test]
fn the_font_gate_closes_on_a_missing_family() {
    assert!(
        font_is_installed(REAL_FONT),
        "実フォント {REAL_FONT} がこの機械に無い"
    );
    assert!(
        !font_is_installed(MISSING_FONT),
        "実在しないフォント名 {MISSING_FONT} で門が開いた——門は働いていない"
    );
    assert!(
        font_is_installed(ALT_FONT),
        "差し替え先の {ALT_FONT} がこの機械に無い（`\\f[name,…]` の検査が縮退する）"
    );
}

// ══ 表 の母数 ══════════════════════════════════════════════════════════════════════════

#[test]
fn the_visible_keys_are_the_seven_that_directwrite_can_range() {
    assert_eq!(
        VISIBLE_KEYS.len(),
        7,
        "表示に効くのは DirectWrite の範囲指定で素直に表せる 7 項目"
    );
    assert_eq!(
        VOCABULARY_ONLY_KEYS.len(),
        3,
        "語彙のみは sub／sup／outline の 3 項目"
    );
}

// ══ 1. 表示に効く 7 項目は画素を変える（要件 5.1） ═════════════════════════════════════

/// 較正: 装飾入りの入口（`layout_styled`／`render_styled`）を従来の入口へ戻すと、7 項目とも
/// 画素が素の行と一致して赤になる。
#[test]
fn each_visible_key_changes_the_pixels_in_horizontal_tb() {
    assert_real_font_present();
    for (key, tokens) in VISIBLE_KEYS {
        let (plain, styled) = shoot_pair(None, tokens);
        assert!(
            differs(&plain, &styled),
            "`\\f[{}]` を有効にしても画素が 1 バイトも変わらない（横書き・本文 {SAMPLE}）",
            tokens.join(","),
        );
        assert!(
            styled.ink_count() > 0,
            "`\\f[{key}]` の行にインクが 1 画素も無い（字が消えたのを「違う」と数えていないか）"
        );
    }
}

// ══ 2. 下線・打ち消し線のインクの位置（要件 5.1） ══════════════════════════════════════

/// `\f[underline,1]` は字より下に連続した 1 本の帯を足す。
///
/// 位置は DirectWrite の既定（areka は描き分けない）。[`UNDERLINE_BAND`] は今日の実測値で、
/// 動いたら赤にして裁定を仰ぐ。
#[test]
fn underline_adds_one_band_of_ink_below_the_glyphs() {
    assert_real_font_present();
    let (plain, styled) = shoot_pair(None, &["underline", "1"]);
    let rows = added_ink_rows(&plain, &styled);
    let band = single_band(&rows, "underline");
    assert_eq!(
        band, UNDERLINE_BAND,
        "下線のインクの帯が今日の値から動いた（`Yu Gothic UI` font.height={FONT_HEIGHT}・横書き）。\
         位置は DirectWrite の既定に委ねてあるので、値が動いたら描画基盤側の変化を疑い、\
         定数を書き換える前に裁定を仰ぐこと"
    );
}

/// `\f[strike,1]` は字の中ほどに連続した 1 本の帯を足す。
#[test]
fn strike_adds_one_band_of_ink_through_the_glyphs() {
    assert_real_font_present();
    let (plain, styled) = shoot_pair(None, &["strike", "1"]);
    let rows = added_ink_rows(&plain, &styled);
    let band = single_band(&rows, "strike");
    assert_eq!(
        band, STRIKE_BAND,
        "打ち消し線のインクの帯が今日の値から動いた（`Yu Gothic UI` font.height={FONT_HEIGHT}・横書き）"
    );
}

// ══ 3. 語彙のみの 3 項目は画素を変えない（要件 5.9／6.1） ══════════════════════════════

/// 対照込み: 同じ窓・同じ本文で `bold` を有効にすると画素が変わることを**同じ検査の中で**
/// 示す。これが無いと「画素が同じ」は、描画が丸ごと壊れていても緑になる。
#[test]
fn vocabulary_only_keys_leave_the_pixels_untouched() {
    assert_real_font_present();

    // 対照（陽性側）——同じ窓で bold は画素を変える。
    let (plain, bold) = shoot_pair(None, &["bold", "1"]);
    assert!(
        differs(&plain, &bold),
        "対照が働いていない: 同じ窓で `\\f[bold,1]` すら画素を変えていない。\
         以下の「画素が同じ」は恒真なので意味を持たない"
    );

    for (key, tokens) in VOCABULARY_ONLY_KEYS {
        let (_, styled) = shoot_pair(None, tokens);
        assert!(
            !differs(&plain, &styled),
            "`\\f[{}]` は語彙として受理するだけで表示は変えない（要件 5.9／6.1）のに画素が変わった",
            tokens.join(","),
        );
        assert!(
            styled.ink_count() > 0,
            "`\\f[{key}]` の行にインクが 1 画素も無い（両方とも空で「同じ」になっていないか）"
        );
    }
}

// ══ 較正——差の検出が空振りしていないこと（要件 15.7） ═════════════════════════════════

/// 同じ台本を 2 度描くと 1 バイトも違わない（読み戻しに実行ごとのばらつきが無い）。
///
/// これが崩れると「画素が違う」の述語は装飾と無関係に緑になる。
#[test]
fn the_same_script_paints_the_same_pixels_twice() {
    assert_real_font_present();
    let a = run(None, &[text_cue(SAMPLE)]);
    let b = run(None, &[text_cue(SAMPLE)]);
    assert!(a.ink_count() > 0, "素の台本にインクがある（前提）");
    assert!(
        !differs(&a, &b),
        "同じ台本の 2 度の読み戻しが違う——画素の差は装飾の証拠にならない"
    );
}

/// 装飾を焼かない経路で描くと差が消える。
///
/// 同じトークン列を**装飾ではない運搬名**で流すと、`state.rs` の自己選別が読み飛ばして装飾を
/// 1 つも焼かない（番号列は全部 0・装飾の表は空＝`render_styled` は既定の呼出列に落ちる）。
/// cue の本数・並び・再生時間は装飾ありの台本と同一なので、[`each_visible_key_changes_the_pixels_in_horizontal_tb`]
/// の「違う」が **cue が 1 本増えたこと**ではなく**装飾を焼いたこと**に由来することを示す。
#[test]
fn tokens_that_never_reach_the_decoration_path_paint_the_plain_pixels() {
    assert_real_font_present();
    let plain = run(None, &[text_cue(SAMPLE)]);
    for (key, tokens) in VISIBLE_KEYS {
        let unbaked = run(None, &[unbaked_cue(tokens), text_cue(SAMPLE)]);
        assert!(
            !differs(&plain, &unbaked),
            "`{key}` のトークン列を装飾ではない運搬名で流したのに画素が変わった——\
             装飾を焼かない経路が装飾を焼いている",
        );
        // 同じトークン列を装飾の運搬名で流せば違う（対照が空振りでないこと）。
        let baked = run(None, &[font_cue(tokens), text_cue(SAMPLE)]);
        assert!(
            differs(&plain, &baked),
            "`{key}` を装飾の運搬名で流しても画素が変わらない（対照の意味が消える）",
        );
    }
}
