//! # choice_fixture_test — choice-render 検証の test-local 最小適合 fixture（task 11.1・R7.6）
//!
//! design.md「Testing Strategy > Integration Tests #5（7.6）」の正典述語を張る:
//! **実フォント出力の目視確認（既定フォント盲点の回避）＋ test-local 最小 fixture を tests 配下に
//! 自前で用意**する。fixture は `tests/fixtures/emo2-choice/`:
//!
//! - `descript-cursor.txt` — cursor.* 指定バルーン（SquareFill 実導出＝塗り(105,25,25)＋白文字(255,255,255)）。
//! - `descript-plain.txt`  — cursor.* 未指定バルーン（Invert 縮退＝塗り=既定 font.color・文字=255−c）。
//! - `menu.txt`            — 短メニュー台本（4 項目・注入 cue 列の対応表）。
//!
//! 本テストは 3 本（開始点の [`choice_fixtures_start_at_writing_corner_regardless_of_image_size`] と次の 2 本）:
//! 1. [`fixtures_parse_to_expected_choice_styles`]（純・GPU 不要）: fixture descript を **実 balloon
//!    パーサ**（`areka_parsers::balloon::parse_str`）で parse し、cursor.* 指定→`SquareFill`・
//!    未指定→`Invert` へ解決されることを固定する（fixture が in-code モデルでなく実 parse される
//!    descript であることの証明）。
//! 2. [`real_font_menu_hover_render_dumps_png`]（GPU・headless）: `descript-cursor.txt` を実フォントで
//!    解決し、4 項目メニュー＋ordinal 1（先頭でない行）の hover を通し経路（`register_actor`→`apply_cue`→
//!    `inject_choice_hover`→`present_frame`→`read_back`）で描画する。pixel 檻（hover 行のセグメント
//!    矩形へ塗り色＋白文字画素が載る）に加え、read_back を白背景へ合成して **PNG を既知パスへ保存**し、
//!    実フォント出力の目視確認を pixel 檻へ伴わせる（記憶 emo-text-byte-equiv-default-font-blindspot）。

#[path = "support/mod.rs"]
mod support;

use support::fixture::{read_balloon_file, read_script_file};
use support::{
    Band, assert_hover_band, dump_png_on_white, fill_pixels_in_rect, make_world_with_gpu,
    opaque_count, read_back, spawn_reserved_slot, surface_size, to_canvas_local,
    white_pixels_in_rect,
};

use std::path::PathBuf;

use areka_emo_text::actor::{
    ChoiceHitRow, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use areka_emo_text::choice::ResolvedChoiceStyle;
use areka_emo_text::region::TextRegion;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

// ── fixture ロード（vertical_fixture_test と同一の読み込み規約：実ファイル＋charset デコード） ──

/// fixture 配置（`tests/fixtures/emo2-choice/`）。
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("emo2-choice")
}

/// バルーンの定義 1 枚（面別上書き層なし）を本番と同じ規約（既定 Ansi・`charset,UTF-8` 宣言
/// 優先）で読み、実 balloon パーサで parse した BalloonModel。
fn model_of(name: &str) -> BalloonModel {
    parse_str(&read_balloon_file(&fixture_dir(), name), None)
}

/// cursor.* 指定 fixture を実 balloon パーサで parse した BalloonModel。
fn cursor_model() -> BalloonModel {
    model_of("descript-cursor.txt")
}

/// cursor.* 未指定 fixture を実 balloon パーサで parse した BalloonModel。
fn plain_model() -> BalloonModel {
    model_of("descript-plain.txt")
}

fn font_color_tuple(model: &BalloonModel) -> (u8, u8, u8) {
    let c = model.font().color();
    (c.r().unwrap_or(0), c.g().unwrap_or(0), c.b().unwrap_or(0))
}

// ══ 開始点の檻: origin 未宣言（正典推奨形）でも書字開始角 (5,5) が変わらない（GPU 不要） ══════

/// 両 fixture の**描画開始点**が `(5, 5)` であることを固定する
/// （spec `areka-P0-balloon-vertical-canon` 要件 3.11／10.9）。
///
/// 同 spec は「origin クランプ正準」を撤去し（この撤去は spec `areka-P0-balloon-origin-outside-validrect`
/// で取り下げられ、範囲外の宣言は再び書字開始角へ落ちる）、あわせて validrect 外の `origin.x,0`／`origin.y,0`
/// 宣言を持っていた出荷／テスト資産を**正典推奨形**（指定せず validrect の定義に任せる）へ
/// 是正した。本 fixture の 2 本もその対象だが、**着地後も開始点を観測する消費者が 1 本も
/// 無かった**（同 spec タスク 4.1 の意味論棚卸しが着手前に記録した既知の穴）。ここで塞ぐ。
///
/// - **是正前**（宣言 `origin(0,0)`）: `0` は validrect `[5, …]` の外なので書字開始角 `(5,5)` へ寄っていた。
/// - **是正後**（宣言なし）: `None` 腕が書字開始角 `(5,5)` へ縮退する。
///
/// つまり両経路が同じ値を与えるため是正は挙動中立である——その主張を反証可能にするのが本檻。
/// もし `origin(0,0)` 宣言が復活すれば、`0` は validrect `[5, …]` の外ゆえ範囲外の宣言として
/// 書字開始角 `(5,5)` へ落ち、開始点の期待値 `(5,5)` は動かない。ただし本檻は冒頭で
/// 「宣言が無いこと」自体を検査しているので、宣言が戻れば**その検査で赤くなる**
/// （是正が保たれていることの見張り）。
///
/// 開始点が**画像原寸に依存しない**ことも併せて見る（`validrect.left`／`.top` が非負素通しのため）。
/// 本 fixture は画像を持たないので、これは「どの寸法で解決しても同じ」ことの明示でもある。
#[test]
fn choice_fixtures_start_at_writing_corner_regardless_of_image_size() {
    // 画像原寸を 2 通り振る（負値辺 right/bottom は寸法で動くが、開始点は動かない）。
    for image_size in [(200u32, 150u32), (400u32, 224u32)] {
        for (label, model) in [("cursor", cursor_model()), ("plain", plain_model())] {
            // 前提: 正典推奨形＝origin は未宣言のまま（宣言が戻ったら本檻の意味が変わる）。
            assert_eq!(
                (model.origin().x(), model.origin().y()),
                (None, None),
                "{label}: fixture は origin を宣言しない（正典推奨形・要件 10.9）"
            );

            let region = TextRegion::resolve(&model, image_size, WritingMode::HorizontalTb);
            assert_eq!(
                region.start(),
                (5.0, 5.0),
                "{label} / {image_size:?}: 書字開始角＝validrect 左上 (5,5)（要件 3.11）"
            );
            assert_eq!(
                (region.left(), region.top()),
                (5.0, 5.0),
                "{label} / {image_size:?}: validrect の left/top は非負素通し"
            );
        }
    }
}

// ══ テスト 1: fixture descript が実 parse で期待スタイルへ解決される（GPU 不要） ══════════════

/// Observable（R7.6）: test-local fixture の 2 descript を**実 balloon パーサ**で parse すると、
/// cursor.* 指定バルーンは `SquareFill { fill:(105,25,25), text:(255,255,255) }`・cursor.* 未指定
/// バルーンは `Invert` へ解決される。fixture は実フォント（Yu Gothic UI）を font.name に持つ
/// （既定 ＭＳ ゴシック盲点の回避＝実フォント目視確認の前提）。
#[test]
fn fixtures_parse_to_expected_choice_styles() {
    // ── cursor.* 指定 → SquareFill（実導出・9.2/9.3 の cursor_model と同値） ──
    let cursor = cursor_model();
    assert_eq!(
        cursor.font().name(),
        Some("Yu Gothic UI"),
        "cursor fixture は実フォントを指定する（既定 ＭＳ ゴシック盲点の回避・7.6）"
    );
    let cursor_style =
        ResolvedChoiceStyle::resolve(Some(cursor.cursor()), font_color_tuple(&cursor));
    assert_eq!(
        cursor_style,
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        },
        "cursor.* 指定 fixture は SquareFill（塗り(105,25,25)＋白文字(255,255,255)）へ解決される"
    );

    // ── cursor.* 未指定 → Invert（矩形反転縮退・M1 実導出） ──
    let plain = plain_model();
    assert_eq!(
        plain.font().name(),
        Some("Yu Gothic UI"),
        "plain fixture も実フォントを指定する"
    );
    let plain_style = ResolvedChoiceStyle::resolve(Some(plain.cursor()), font_color_tuple(&plain));
    assert_eq!(
        plain_style,
        ResolvedChoiceStyle::Invert,
        "cursor.* 未指定 fixture は Invert（矩形反転縮退）へ解決される"
    );

    // ── ResolvedBalloonText 経由でも同一スタイルを運ぶ（通し経路が読む choice_style） ──
    let image = (200u32, 120u32);
    assert_eq!(
        ResolvedBalloonText::resolve(&cursor, image).choice_style,
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        },
        "ResolvedBalloonText.choice_style も cursor fixture では SquareFill"
    );
    assert_eq!(
        ResolvedBalloonText::resolve(&plain, image).choice_style,
        ResolvedChoiceStyle::Invert,
        "ResolvedBalloonText.choice_style も plain fixture では Invert"
    );

    // ── menu.txt の項目数（\q[…]）が注入 cue 列（4 項目）と一致する（台本と注入の対応検証） ──
    let menu = read_script_file(&fixture_dir(), "menu.txt");
    let q_count = menu.matches("\\q[").count();
    assert_eq!(
        q_count,
        MENU.len(),
        "menu.txt の \\q 項目数が注入 cue 列（{} 項目）と一致する: {q_count}",
        MENU.len()
    );
}

// ══ テスト 2: 実フォントで短メニュー＋hover を描き、PNG を目視確認用に保存する（GPU・headless） ══

/// 注入する短メニュー（menu.txt と対応・4 項目）。
const MENU: &[(&str, &str, &str)] = &[
    ("OnYes", "はい", "r0"),
    ("OnNo", "いいえ", "r1"),
    ("OnMaybe", "どちらでも", "r2"),
    ("OnLater", "あとで", "r3"),
];

fn choice_cue(id: &str, text: &str, references: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Choice {
            id: id.into(),
            text: text.into(),
            references: vec![references.into()],
        },
        duration: text.chars().count() as f64 * 0.05,
    }
}

fn newline_cue() -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::NewLine { ratio: 1.0 },
        duration: 0.0,
    }
}

/// Observable（R7.6）: cursor.* 指定 fixture を**実フォント（Yu Gothic UI）**で解決し、4 項目メニュー
/// ＋ordinal 1 の hover を通し経路で描画すると、(a) hover 行のセグメント矩形へ SquareFill 塗り色
/// (105,25,25)＋白文字画素が載り（pixel 檻）、(b) その read_back を白背景へ合成した PNG が既知パスへ
/// 保存される（実フォント出力の目視確認を pixel 檻へ伴わせる）。
///
/// PNG 出力先は env `AREKA_DIAG_OUT`（無指定は `CARGO_TARGET_TMPDIR`＝crate 専用 tmp）。
#[test]
fn real_font_menu_hover_render_dumps_png() {
    let mut world = make_world_with_gpu();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");

    let model = cursor_model();
    let image = (200u32, 120u32);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&model, image),
    );

    // 4 項目を NewLine 区切りで別行へ（hover 行が 1 行に限定されることを観測可能にする）。
    for (i, (id, text, refs)) in MENU.iter().enumerate() {
        rt.apply_cue(&choice_cue(id, text, refs));
        if i + 1 < MENU.len() {
            rt.apply_cue(&newline_cue());
        }
    }

    // 全リビール済みフレーム（t 大）。hover 前も選択肢テキストが描かれる。
    present_frame(&mut rt, &mut world, 100.0).expect("ベースライン提示");
    let rows: Vec<ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), MENU.len(), "4 選択肢＝4 ヒット行");

    let (w, h) = surface_size(&rt, &actor);
    let base = read_back(&rt, &actor);
    assert!(
        opaque_count(&base) > 0,
        "実フォントで選択肢テキストが描画される（非退化）"
    );

    // ── ordinal 1（「いいえ」）を hover 注入 → 再提示 → read_back ──
    //
    // ホバーするのを**先頭でない行**にするのは、下の縦範囲の検査で「字の上端が帯の内」を
    // 反証可能にするためである。先頭行は帯の上端が面の縁のすぐ下（帯の寄せの分＝y2）にしかなく、
    // 走査窓を 4 画素上へ開けようがないので、上端の判定は窓の取り方に寄りかかってしまう
    // （字がベースラインごと上へずれても窓に切られて緑のまま）。ordinal 1 なら帯の上端は
    // 行送り 1 つぶん下（y24）にあり、窓はその 4 画素上から見られる。
    rt.inject_choice_hover(&actor, Some(1));
    present_frame(&mut rt, &mut world, 100.0).expect("hover 提示");
    let hover = read_back(&rt, &actor);

    // read_back は validrect-local なので、窓物理のヒット矩形から validrect 原点を差し引く。
    let region = TextRegion::resolve(&model, image, WritingMode::HorizontalTb);
    let (ox, oy) = (region.left(), region.top());
    let row0_cl = to_canvas_local(&rows[0], ox, oy);
    let row1_cl = to_canvas_local(&rows[1], ox, oy);
    let row2_cl = to_canvas_local(&rows[2], ox, oy);

    // 画素の検査: hover 行（ordinal 1）へ SquareFill 塗り色＋白文字が載る。
    assert!(
        fill_pixels_in_rect(&hover, w, h, row1_cl) > 0,
        "hover 行に SquareFill 塗り色(105,25,25)画素が載る: canvas-local {row1_cl:?}"
    );
    assert!(
        white_pixels_in_rect(&hover, w, h, row1_cl) > 0,
        "hover 行に白文字(255,255,255)画素が載る: canvas-local {row1_cl:?}"
    );
    // 非 hover 行（ordinal 0＝hover 行の 1 つ上）へは塗り色画素が載らない
    //（ダーティ限定＝hover 行のみハイライト。帯が上の行へ伸びていないことも同時に言える）。
    assert_eq!(
        fill_pixels_in_rect(&hover, w, h, row0_cl),
        0,
        "非 hover 行には SquareFill 塗り色画素が載らない: canvas-local {row0_cl:?}"
    );

    // ── 回帰検査（実機不具合「hover 文字の下が切れる」）: インクがハイライト矩形からほぼはみ出さない ──
    //
    // 帯（ハイライト矩形＝ヒット矩形のブロック軸寸）を em ボックス丈（font.height）で切ると、
    // DirectWrite が ascent+descent で描く実インクの descent 側が塗りの外へ落ちる（実フォント
    // Yu Gothic UI は行ボックス 1.3301em）。白バルーン＋白 hover 文字では「下が消えた」ように見える。
    // 走査 y 窓は「hover 行の block 起点−余白 4 〜 次行の block 起点＋余白 4」＝帯寸に依存しない
    // 独立の窓。窓を帯とぴったり同じにすると、帯からはみ出したインクが窓の外へ落ちて**測れない
    // まま緑になる**（同 spec 要件 7.3）。これは上端にも同じく効く——窓の上端を帯の上端と揃えると
    // 「字の上端は帯の上端以上」が窓の取り方だけで必ず成り立ち、字が上へずれても緑のままになる。
    // ゆえに窓は帯の上下へ 4 画素ずつ開ける。上の行の字は黒（premultiplied の青チャネルは 0）ゆえ
    // 白文字の述語に当たらず、塗り色 (25,25,105,255) でもないので窓へ入っても数えられない。
    // 次行のインクはその行の block 起点から 5〜6 画素下で始まるので、4 画素の余白なら窓へ入らない。
    // 座標は validrect 原点を差し引いた canvas-local。
    //
    // **裁定 2026-09-06（第 2 回・spec `areka-P0-emo2-conformance-e2e` 要件 13）**:
    // 行送りが正典（`font.height + 行間 2`）へ確定した後、実フォントの読み戻しで帯の下端から
    // 文字のインクが下へ出た。同日の第 1 の裁定は「2 画素のはみ出しを許容する」だったが、
    // 開発者は実機の目視でこれを覆した——**帯を行ボックスの中央へ寄せる**のがあるべき姿である。
    // 帯は行矩形のブロック軸近端から `band_offset = round(max(0, 行ボックス丈 − 帯の丈) / 2)`
    // だけ内側へ寄る（この fixture の font 20 では 行ボックス 26.6 − 帯 22 → **2 画素**。
    // 正典の 28 では 37.24 − 30 → 4 画素）。ゆえに下の判定は
    // 「上端は帯の内（上の余白 ≥ 0）・下端のはみ出しは 0」を見る。
    //
    // 寄せる前の実測（ordinal 1）は 帯 y22..43 ＝ 丈 22（font.height 20 ＋ 行間 2）に対し
    // 白文字のインク y28..44 ＝ 上の余白 6・はみ出し 1 画素。寄せた後は 帯 y24..45 に対し
    // 同じインク y28..44 ＝ 上の余白 4・はみ出し **0** になる（丈は変えていない）。
    //
    // 帯を広げる案は採らない——帯を行送りより広げると隣接する行の帯と重なり、どの選択肢を
    // 指しているかの一意性が壊れる（`emo-text-line-height-canon` design §「帯の防御式を保つ」）。
    // **はみ出しが出た場合は許容幅を上げず、画素数を添えて改めて開発者の裁定を仰ぐ（R12.2）。**
    /// 帯を行ボックスの中央へ寄せる量（画素・この fixture の `font.height,20`）。
    /// `round(max(0, 26.6 − 22) / 2) = round(2.3) = 2`（要件 13.1）。
    const BAND_OFFSET_20: u32 = 2;

    assert_hover_band(
        "choice-fixture",
        &hover,
        (w, h),
        row1_cl,
        row2_cl,
        Band {
            // ordinal 1 の行矩形の上端は行送り 1 つぶん下（y22）。offset を 0 へ戻すと帯 y22..43 で赤。
            line_top: 22,
            height: 22,
            offset: BAND_OFFSET_20,
        },
    );

    // ── 目視確認: read_back（premultiplied BGRA）を白背景へ合成して PNG を保存する ──
    let path = dump_png_on_white("choice_menu_hover_realfont.png", &hover, w, h);
    eprintln!(
        "[choice-fixture] 実フォント目視確認 PNG を保存: {path} ({w}x{h}・font=Yu Gothic UI・\
         4 項目メニュー・ordinal 1「いいえ」hover＝SquareFill 塗り(105,25,25)＋白文字)"
    );
}
