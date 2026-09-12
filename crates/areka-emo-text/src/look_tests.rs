//! 見た目の値型（[`super::TextLook`]）・2 層（[`super::LookLayers`]）・装飾の表
//! （[`super::StyleTable`]／[`super::StyleId`]／[`super::GlyphStyles`]）の決定論テスト
//! （タスク 3.2・要件 4.1／4.3／4.5／4.7・15.6）。
//!
//! ## 章立て
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | 正典の既定（全 9 項目を明示・COM 層の既定定数との一致） | 4.1 |
//! | §2 | 計測鍵——送り幅に効く 4 項目だけを取り出す | 4.1 |
//! | §3 | 2 層の組み立て（バルーン定義の有無 × 各項目・無効表示は色だけ混色） | 4.1, 4.3, 4.5, 4.7 |
//! | §4 | 装飾の表（既定は番号 0・同値の畳み込み・範囲外は既定・消去） | 4.1 |
//! | §5 | グリフ序数からの読み口（範囲外は既定） | 4.1 |
//! | §6 | 較正——過去に壊れうる形を再現すると赤になる述語 | 15.6 |
//!
//! **期待値は正典の式から書く**——実装が返した値を書き写さない。無効表示の色は
//! 「(背景 + 文字色 × 2) / 3」（要件 4.6・[`crate::color::mix_disabled`] が唯一の実装点）なので、
//! 期待値は黒文字 × 白背景＝`(255 + 0) / 3 = 85`、白文字 × 黒背景＝`(0 + 510) / 3 = 170` の
//! ように手で解いた値を直に書く（`mix_disabled` を呼んで期待値を作ると恒真になる）。
//!
//! **「定義が無い側」も独立に断言する**——バルーン定義からまだ読めていない 8 キー
//! （太字・斜体・下線・打ち消し線・白抜き・上下付き ほか）は「零のまま」であることが
//! 要件（4.3 の口が空いているだけで値は既定）なので、零を明示的に述べる（§3）。

use super::*;

/// バルーン定義を模した 2 層——各項目が既定と異なる値で入っていること自体を試験に使う。
fn balloon_layers() -> LookLayers {
    LookLayers::from_balloon(
        vec!["Meiryo".to_owned(), "Arial".to_owned()],
        20.0,
        (10, 20, 30),
        (200, 210, 220),
        (7, 8, 9),
    )
}

/// 既定と 1 項目だけ異なる見た目（表の畳み込みを試すための素材）。
fn bold_look(default: &TextLook) -> TextLook {
    TextLook {
        bold: true,
        ..default.clone()
    }
}

// ------------------------------------------------------------------ §1 正典の既定

/// 正典の既定は ＭＳ ゴシック・12・黒・装飾なし——9 項目すべてを明示して固定する（R4.1）。
///
/// 装飾 6 項目の「無効」は表に書かなければ述べたことにならないので、真偽値も明示的に断言する。
#[test]
fn ukadoc_default_is_ms_gothic_twelve_black_without_decoration() {
    let look = TextLook::ukadoc_default();
    assert_eq!(look.name, vec![UKADOC_DEFAULT_FONT_NAME.to_owned()]);
    assert_eq!(look.height, 12.0);
    assert_eq!(look.color, (0, 0, 0));
    assert!(!look.bold, "既定は太字でない");
    assert!(!look.italic, "既定は斜体でない");
    assert!(!look.underline, "既定は下線なし");
    assert!(!look.strike, "既定は打ち消し線なし");
    assert!(!look.outline, "既定は白抜きなし");
    assert_eq!(look.script, Script::None, "既定は上下付きでない");
}

/// 正典の既定値は COM 層の既定定数と同じ値でなければならない（2 か所に書かれた同じ正典が
/// 黙って食い違わないための見張り。片方だけ直す変更を赤にする）。
#[test]
fn ukadoc_defaults_agree_with_the_com_layer_constants() {
    assert_eq!(UKADOC_DEFAULT_FONT_NAME, crate::draw::DEFAULT_FONT_NAME);
    assert_eq!(UKADOC_DEFAULT_FONT_HEIGHT, crate::draw::DEFAULT_FONT_HEIGHT);
}

// ------------------------------------------------------------------ §2 計測鍵

/// 計測鍵は送り幅に効く 4 項目（候補列・大きさ・太字・斜体）だけを持つ（R4.1）。
#[test]
fn font_key_changes_with_each_item_that_moves_the_advance() {
    let base = TextLook::ukadoc_default();
    let key = base.font_key();
    assert_eq!(key.name, base.name);
    assert_eq!(key.height_bits, 12.0f32.to_bits());
    assert!(!key.bold);
    assert!(!key.italic);

    for changed in [
        TextLook {
            name: vec!["Meiryo".to_owned()],
            ..base.clone()
        },
        TextLook {
            height: 20.0,
            ..base.clone()
        },
        TextLook {
            bold: true,
            ..base.clone()
        },
        TextLook {
            italic: true,
            ..base.clone()
        },
    ] {
        assert_ne!(
            changed.font_key(),
            key,
            "送り幅に効く項目を変えたのに計測鍵が変わらない: {changed:?}"
        );
    }
}

/// 送り幅に効かない項目（色・下線・打ち消し線・白抜き・上下付き）は計測鍵を変えない——
/// 変えてしまうと同じ幅の文字が別の run と別の計測へ割れる（R4.1）。
#[test]
fn font_key_ignores_the_items_that_do_not_move_the_advance() {
    let base = TextLook::ukadoc_default();
    let key = base.font_key();
    for changed in [
        TextLook {
            color: (255, 0, 0),
            ..base.clone()
        },
        TextLook {
            underline: true,
            ..base.clone()
        },
        TextLook {
            strike: true,
            ..base.clone()
        },
        TextLook {
            outline: true,
            ..base.clone()
        },
        TextLook {
            script: Script::Sub,
            ..base.clone()
        },
    ] {
        assert_eq!(
            changed.font_key(),
            key,
            "送り幅に効かない項目で計測鍵が変わっている: {changed:?}"
        );
    }
}

// ------------------------------------------------------------------ §3 2 層の組み立て

/// バルーン定義が無い側——2 層は正典の既定・背景は白・選択肢文字色は黒（R4.1）。
///
/// 無効表示の色は黒文字 × 白背景で `(255 + 0 × 2) / 3 = 85`（要件 4.6 の式を手で解いた値）。
#[test]
fn layers_without_a_balloon_definition_are_the_ukadoc_defaults() {
    let layers = LookLayers::default();
    assert_eq!(layers.default, TextLook::ukadoc_default());
    assert_eq!(layers.cursor_text, (0, 0, 0), "選択肢文字色の既定は黒");
    assert_eq!(
        layers.disable.color,
        (85, 85, 85),
        "黒文字 × 白背景の無効表示色は (255 + 0) / 3"
    );
}

/// バルーン定義がある側——読めている 5 キー（名前・大きさ・色 r/g/b）と背景・選択肢文字色が
/// そのまま既定層へ入る（R4.1・R4.3 の「口」が実際に効いていること）。
#[test]
fn a_balloon_definition_fills_each_readable_item() {
    let layers = balloon_layers();
    assert_eq!(
        layers.default.name,
        vec!["Meiryo".to_owned(), "Arial".to_owned()],
        "候補列は記述順のまま"
    );
    assert_eq!(layers.default.height, 20.0);
    assert_eq!(layers.default.color, (10, 20, 30));
    assert_eq!(layers.cursor_text, (7, 8, 9));
}

/// フォント名の定義が無い（候補列が空）ときは正典の既定名へ落ちる（R4.1）。
#[test]
fn an_absent_font_name_falls_back_to_the_ukadoc_default_name() {
    let layers = LookLayers::from_balloon(Vec::new(), 20.0, (10, 20, 30), (0, 0, 0), (0, 0, 0));
    assert_eq!(
        layers.default.name,
        vec![UKADOC_DEFAULT_FONT_NAME.to_owned()]
    );
    assert_eq!(layers.disable.name, layers.default.name);
}

/// 大きさが正の有限値でないときは正典の既定 12 へ落ちる——DirectWrite の em は正値必須で、
/// 2 層は「常に正の大きさ」を下流へ約束する（R4.1）。
#[test]
fn a_non_positive_font_height_falls_back_to_twelve() {
    for height in [0.0, -3.0, f32::NAN, f32::INFINITY] {
        let layers =
            LookLayers::from_balloon(Vec::new(), height, (0, 0, 0), (255, 255, 255), (0, 0, 0));
        assert_eq!(
            layers.default.height, 12.0,
            "大きさ {height} が既定 12 へ落ちていない"
        );
        assert_eq!(layers.disable.height, 12.0);
    }
}

/// バルーン定義からまだ読めていない 8 キーは、両層とも正典の既定（＝零）のままである
/// （R4.3・R4.7 の「口は空けるが値は既定」）。零は明示しなければ述べたことにならない。
#[test]
fn the_keys_not_yet_read_from_the_balloon_stay_at_the_ukadoc_default_in_both_layers() {
    let layers = balloon_layers();
    for (label, look) in [("既定層", &layers.default), ("無効表示層", &layers.disable)] {
        assert!(!look.bold, "{label}: 太字は既定の無効のまま");
        assert!(!look.italic, "{label}: 斜体は既定の無効のまま");
        assert!(!look.underline, "{label}: 下線は既定の無効のまま");
        assert!(!look.strike, "{label}: 打ち消し線は既定の無効のまま");
        assert!(!look.outline, "{label}: 白抜きは既定の無効のまま");
        assert_eq!(look.script, Script::None, "{label}: 上下付きは既定の無し");
    }
}

/// 無効表示の層は「色だけ混色・他の項目は既定と同じ」（R4.5）。
///
/// 期待値は正典の式を手で解いて書く: 文字色 (10,20,30)・背景 (200,210,220) なら
/// r=(200+20)/3=73・g=(210+40)/3=83・b=(220+60)/3=93（いずれも整数除算）。
#[test]
fn the_disable_layer_differs_from_the_default_layer_only_in_color() {
    let layers = balloon_layers();
    assert_eq!(layers.disable.color, (73, 83, 93));
    assert_ne!(
        layers.disable.color, layers.default.color,
        "無効表示の色が既定と同じでは薄くなっていない"
    );
    // 色以外の 8 項目は既定層と一致する（色を揃えたら完全一致になることで述べる）。
    let disable_with_default_color = TextLook {
        color: layers.default.color,
        ..layers.disable.clone()
    };
    assert_eq!(disable_with_default_color, layers.default);
}

/// 背景だけを変えると、変わるのは無効表示の色だけ——既定層も選択肢文字色も動かない（R4.5）。
#[test]
fn the_background_moves_the_disable_color_and_nothing_else() {
    let on_white = LookLayers::from_balloon(
        Vec::new(),
        12.0,
        (255, 255, 255),
        (255, 255, 255),
        (1, 2, 3),
    );
    let on_black =
        LookLayers::from_balloon(Vec::new(), 12.0, (255, 255, 255), (0, 0, 0), (1, 2, 3));
    assert_eq!(on_white.default, on_black.default);
    assert_eq!(on_white.cursor_text, on_black.cursor_text);
    assert_eq!(
        on_white.disable.color,
        (255, 255, 255),
        "白文字 × 白背景は (255 + 510) / 3 = 255"
    );
    assert_eq!(
        on_black.disable.color,
        (170, 170, 170),
        "白文字 × 黒背景は (0 + 510) / 3 = 170"
    );
}

// ------------------------------------------------------------------ §4 装飾の表

/// 既定と同じ見た目は番号 0 になり、表には積まれない（R4.1・D14 の記号）。
#[test]
fn the_default_look_is_style_zero_and_is_not_stored() {
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    assert_eq!(table.len(), 0);
    assert!(table.is_empty());

    assert_eq!(table.intern(&default, &default), StyleId::DEFAULT);
    assert_eq!(StyleId::DEFAULT, StyleId(0));
    assert_eq!(table.len(), 0, "既定の見た目を表へ積んでいる");
}

/// 異なる見た目は 1 から順に番号が付く（追記専用・既存番号の意味は変わらない）。
#[test]
fn distinct_looks_get_consecutive_numbers() {
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let bold = bold_look(&default);
    let italic = TextLook {
        italic: true,
        ..default.clone()
    };

    assert_eq!(table.intern(&bold, &default), StyleId(1));
    assert_eq!(table.intern(&italic, &default), StyleId(2));
    assert_eq!(table.len(), 2);
    // 先に付いた番号は後の追記で意味が変わらない。
    assert_eq!(table.resolve(StyleId(1), &default), &bold);
    assert_eq!(table.resolve(StyleId(2), &default), &italic);
}

/// 同値の見た目は同じ番号へ畳み込む（別々に組み立てた等値でも 1 つ）。
#[test]
fn equal_looks_fold_into_one_number() {
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let first = table.intern(&bold_look(&default), &default);
    let second = table.intern(&bold_look(&default), &default);
    assert_eq!(first, second);
    assert_eq!(table.len(), 1, "同値の見た目が 2 つ積まれている");
}

/// 番号 0 と範囲外の番号は既定へ落ちる（panic しない）。表に無い番号を引いても
/// 別の見た目には決してならない（R4.1）。
#[test]
fn zero_and_out_of_range_numbers_resolve_to_the_default() {
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let bold = bold_look(&default);
    assert_eq!(table.intern(&bold, &default), StyleId(1));

    assert_eq!(table.resolve(StyleId::DEFAULT, &default), &default);
    assert_eq!(table.resolve(StyleId(2), &default), &default);
    assert_eq!(table.resolve(StyleId(u32::MAX), &default), &default);
}

/// 消去すると表は空に戻り、以前の番号も既定へ落ちる。
#[test]
fn clearing_the_table_drops_every_number() {
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    table.intern(&bold_look(&default), &default);
    table.clear();
    assert_eq!(table.len(), 0);
    assert!(table.is_empty());
    assert_eq!(table.resolve(StyleId(1), &default), &default);
}

// ------------------------------------------------------------------ §5 グリフ序数の読み口

/// グリフ序数から番号と見た目を引き、序数が番号列の外なら既定に落ちる（R4.1）。
#[test]
fn glyph_styles_read_by_ordinal_and_fall_back_outside_the_range() {
    let layers = balloon_layers();
    let default = layers.default.clone();
    let mut table = StyleTable::default();
    let bold = bold_look(&default);
    let id = table.intern(&bold, &default);
    let ids = [StyleId::DEFAULT, id];
    let current = TextLook {
        height: 30.0,
        ..default.clone()
    };
    let styles = GlyphStyles {
        table: &table,
        ids: &ids,
        default: &default,
        current: &current,
    };

    assert_eq!(styles.id_of(0), StyleId::DEFAULT);
    assert_eq!(styles.id_of(1), id);
    assert_eq!(styles.id_of(2), StyleId::DEFAULT, "序数が範囲外なら既定");
    assert_eq!(styles.look_of(0), &default);
    assert_eq!(styles.look_of(1), &bold);
    assert_eq!(styles.look_of(9), &default);
}

// ------------------------------------------------------------------ §6 較正

/// 較正: 無効表示の層を「色以外の項目まで既定と変えてしまう」誤り（R4.5・R15.6）。
#[test]
fn calibration_disable_layer_must_not_change_anything_but_color() {
    let layers = balloon_layers();
    assert_ne!(
        layers.disable.name,
        vec![UKADOC_DEFAULT_FONT_NAME.to_owned()],
        "無効表示層のフォント名が既定名へ差し替わっている（既定層の候補列を引き継いでいない）"
    );
    assert_ne!(
        layers.disable.height, 12.0,
        "無効表示層の大きさが正典の既定へ戻っている（既定層の値を引き継いでいない）"
    );
    assert!(
        !layers.disable.bold,
        "無効表示層が太字を勝手に立てている（既定層は太字でない）"
    );
}

/// 較正: 同値の見た目を別番号に分けてしまう誤り（畳み込み漏れ・R15.6）。
/// 表が膨れるだけでなく、番号が違えば描画が別 run に割れる。
#[test]
fn calibration_equal_looks_must_not_take_separate_numbers() {
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let first = table.intern(&bold_look(&default), &default);
    let second = table.intern(&bold_look(&default), &default);
    assert_ne!(
        (first, table.len()),
        (StyleId(1), 2),
        "同値の見た目が畳み込まれず 2 番目の番号が生えている"
    );
    assert_eq!(second, first);
}

/// 較正: 範囲外の番号で別の見た目に落ちる（あるいは panic する）誤り（R15.6）。
/// panic する実装ならこのテスト自体が落ちるので、両方の壊れ方を捕まえる。
#[test]
fn calibration_out_of_range_number_must_not_resolve_to_another_look() {
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let bold = bold_look(&default);
    table.intern(&bold, &default);
    assert_ne!(
        table.resolve(StyleId(2), &default),
        &bold,
        "範囲外の番号が表の末尾（や飽和した番号）の見た目に落ちている"
    );
}

/// 較正: 計測鍵に色を混ぜてしまう誤り（R15.6）——色だけ違う文字が別の計測・別の書式に
/// 割れ、送り幅が同じはずの文字で表とキャッシュが無駄に膨れる。
#[test]
fn calibration_font_key_must_not_include_color() {
    let base = TextLook::ukadoc_default();
    let red = TextLook {
        color: (255, 0, 0),
        ..base.clone()
    };
    assert_eq!(
        red.font_key(),
        base.font_key(),
        "計測鍵が色で変わっている（色は送り幅に効かないので鍵に入れてはならない）"
    );
}
