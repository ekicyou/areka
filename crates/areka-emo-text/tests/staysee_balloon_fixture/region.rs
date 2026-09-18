//! 文字の領域解決と、既定書体の受け入れ口を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **1.3**／**3.3**／**3.5**・設計 **C2** の
//! E 行と F 行・**DD4**）。
//!
//! ## ここで固定するもの
//!
//! 本番の描画入口（`present_frame` が通るのと同じ [`ResolvedBalloonText::resolve`]）で
//! 検体の文字の領域を解き、両 scope について
//!
//! - 描画範囲の 4 辺・描画開始点・折返し基準・遠辺・画像の原寸、
//! - 解決された書体の名前・高さ、
//! - 実測の文字の寸法（半角と全角の送り幅・行ボックス・行送り）
//!
//! を逐語で固定する。検体は `font.name` も `wordwrappoint` も宣言していないので、前者は
//! ukadoc の既定書体へ、後者は描画範囲の遠辺へ縮退する（要件 3.3）。
//!
//! ## 「縮退している」を空振りで主張しないための対照
//!
//! 「未宣言だから既定へ落ちた」「未宣言だから遠辺と同じ値になった」は、読み口が**常に**
//! 既定しか返さない実装でも真になってしまう。そこで、同じ読み口へ**宣言を持つ別の
//! バルーン定義**（`examples/fixtures/emo2-vertical-canon`——`font.name,Yu Gothic UI` と
//! `wordwrappoint` を宣言している）を通し、
//!
//! - 書体の名前が既定とは**別の名前**で返ること、
//! - 折返し基準と遠辺が**別々の値**に分かれること
//!
//! を対照として並べる（[`declared_balloon_returns_a_different_font_and_a_split_wrap_threshold`]）。
//! 対照は検体と**同じ原寸**を渡して解くので、答えの違いはバルーン定義の違いだけから来る。
//!
//! ## 既定書体が無い環境では赤で止める
//!
//! 文字の寸法の期待値は `ＭＳ ゴシック` の実測（半角 0.5em・全角 1em・行ボックス比 1.0）で
//! あり、当該書体が無い環境では DirectWrite が別の書体へ落ちて前提が崩れる。落ちたことを
//! 緑のまま通さないよう、寸法そのものを門として先に確かめる（設計 Technology Stack・DD4 ⑶。
//! 既存の `Yu Gothic UI` を前提にしたテストと同じ扱い）。
//!
//! ## 決定論
//!
//! ファイル読み込み・純粋層の解決・DirectWrite の計測のみ。実 GPU・実窓・実ゴーストを
//! 要さず、同一入力に対して常に同一の結果を返す。

use std::path::PathBuf;

use areka_emo_text::actor::ResolvedBalloonText;
use areka_emo_text::draw::{DEFAULT_FONT_NAME, DWriteMetrics};
use areka_emo_text::layout::GlyphMetrics;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};

use super::test_support::{dwrite_factory, expected_frame_size, staysee_model};

// ── 領域解決の期待値（設計 C2 の E 行）──────────────────────────────────────────
//
// 4 辺は宣言からの計算結果であって実測から採った値ではない: `validrect.left,22`／`top,20`
// （非負は素通し）・`right,-26`／`bottom,-47`（負値は反対辺基準）を、面 0 の原寸
// （本体側 335×205・相方側 335×135）に当てはめて解いたもの。したがって値が食い違ったら
// 期待値ではなく解決の側を疑う。

/// 各 scope の面 0 として焼き込まれる枠画像の名前（原寸の引き先）。
///
/// 原寸そのものは `test_support` の表（実 PNG の IHDR と突合済み）から引くので、ここで
/// 二重に綴らない。
const SCOPE_FACE0_FILES: [(u32, &str); 2] = [(0, "balloons0.png"), (1, "balloonk0.png")];

/// 描画範囲の左辺（`validrect.left,22` の素通し・image px）。
const EXPECTED_LEFT: f32 = 22.0;
/// 描画範囲の上辺（`validrect.top,20` の素通し・image px）。
const EXPECTED_TOP: f32 = 20.0;
/// 描画範囲の右辺（`validrect.right,-26` → 幅 335 − 26・image px）。両 scope とも同値。
const EXPECTED_RIGHT: f32 = 309.0;
/// 描画開始点（`origin` 未宣言 → 横書きの書字開始角＝描画範囲の左上・image px）。
const EXPECTED_START: (f32, f32) = (EXPECTED_LEFT, EXPECTED_TOP);

/// scope ごとの描画範囲の下辺（`validrect.bottom,-47` → 高さ − 47・image px）。
///
/// 本体側は 205 − 47 ＝ 158・相方側は 135 − 47 ＝ 88。下辺だけが scope で違う。
const EXPECTED_BOTTOM: [(u32, f32); 2] = [(0, 158.0), (1, 88.0)];

// ── 既定書体と文字の寸法の期待値（設計 C2 の F 行・DD4）──────────────────────────
//
// 書体の名前のリテラルは ukadoc の既定そのもので、`DEFAULT_FONT_NAME` とは別に綴る
// ——両辺を同じ定数から引くと、定数が別の書体へ書き換わったときに気付けない。

/// ukadoc が定める既定書体の名前（リテラル。areka 側の定数とは独立に綴る）。
const CANON_DEFAULT_FONT_NAME: &str = "ＭＳ ゴシック";
/// 解決される文字の高さ（`font.height,12` の素通し・image px）。
const EXPECTED_FONT_HEIGHT: f32 = 12.0;
/// 半角 1 文字の送り幅（`ＭＳ ゴシック` は半角 0.5em ＝ 12 × 0.5・image px）。
const EXPECTED_ADVANCE_HALF: f32 = 6.0;
/// 全角 1 文字の送り幅（`ＭＳ ゴシック` は全角 1em ＝ 12 × 1.0・image px）。
const EXPECTED_ADVANCE_FULL: f32 = 12.0;
/// 行ボックスの丈（`ＭＳ ゴシック` は `ascent + descent` がちょうど 1em・image px）。
const EXPECTED_LINE_BOX: f32 = 12.0;
/// 行送り（正典式 `font.height + 行間 2` ＝ 12 + 2・image px）。
const EXPECTED_LINE_PITCH: f32 = 14.0;

// ── 対照に使う「宣言を持つ」バルーン定義 ─────────────────────────────────────
//
// 検体ではなく**読み口が既定以外も返せること**を示すための対照である。検体の所在は親
// ファイルの定数 1 か所だけが持つ決まり（要件 3.2）なので、対照の所在はそれとは別に、
// 対照であることが分かる名前でここに置く。

/// 対照のバルーン定義（`font.name` と `wordwrappoint` を宣言している既存の検体）。
const DECLARED_BALLOON_DIR: &str = "examples/fixtures/emo2-vertical-canon";
/// 対照の面別上書き層（本番と同じ 2 層マージで読む）。
const DECLARED_BALLOON_OVERLAY: &str = "balloons0s.txt";
/// 対照が宣言する書体の名前（`descript.txt` の `font.name,Yu Gothic UI`）。
const DECLARED_FONT_NAME: &str = "Yu Gothic UI";
/// 対照の折返し基準（面別上書き層の `wordwrappoint.y,-60` → 高さ 205 − 60・image px）。
const DECLARED_WRAP_THRESHOLD: f32 = 145.0;
/// 対照の遠辺（面別上書き層の `validrect.bottom,-56` → 高さ 205 − 56・image px）。
const DECLARED_INLINE_LIMIT: f32 = 149.0;

// ── 読み込み ─────────────────────────────────────────────────────────────────

/// scope の面 0 の原寸（image px）。`test_support` の表から引く。
fn scope_image_size(scope: u32) -> (u32, u32) {
    let (_, file) = SCOPE_FACE0_FILES
        .iter()
        .find(|(s, _)| *s == scope)
        .unwrap_or_else(|| panic!("scope {scope} の面 0 の枠画像が表に無い"));
    expected_frame_size(file)
}

/// scope ごとの描画範囲の下辺を引く。
fn expected_bottom(scope: u32) -> f32 {
    EXPECTED_BOTTOM
        .iter()
        .find(|(s, _)| *s == scope)
        .map(|(_, b)| *b)
        .unwrap_or_else(|| panic!("scope {scope} の下辺の期待値が表に無い"))
}

/// 検体の当該 scope を本番の描画入口と同じ関数で解く。
fn resolve_staysee(scope: u32) -> ResolvedBalloonText {
    ResolvedBalloonText::resolve(&staysee_model(), scope_image_size(scope))
}

/// 対照のバルーン定義を本番と同じ 2 層マージで組む。
///
/// 読み込み失敗と空は明示的に落とす（読めなかったときに「対象 0 件だから緑」になる形を
/// 作らない）。
fn declared_model() -> BalloonModel {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DECLARED_BALLOON_DIR);
    let read = |name: &str| {
        let path = dir.join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "対照のバルーン定義 {} の読取に失敗した（対照はこのファイルの実在が前提）: {e}",
                path.display()
            )
        });
        assert!(
            !bytes.is_empty(),
            "対照のバルーン定義 {} が空である（空では対照が何も示さない）",
            path.display()
        );
        decode(&bytes, DefaultEncoding::Ansi)
    };
    parse_str(&read("descript.txt"), Some(&read(DECLARED_BALLOON_OVERLAY)))
}

/// 検体の書体で実測の文字の寸法を組む（GPU 不要——計測に要るのは factory だけ）。
fn staysee_metrics(resolved: &ResolvedBalloonText) -> DWriteMetrics {
    DWriteMetrics::new(
        &dwrite_factory(),
        &resolved.font,
        resolved.mode,
        &TextLayerConfig::default(),
    )
    .expect("既定バルーンの書体で文字の寸法を組める")
}

// ── 領域解決（設計 C2 の E 行・要件 3.3／3.5）────────────────────────────────

/// 本番の描画入口で解いた文字の領域を、両 scope について成分ごとに逐語で固定する。
///
/// 下辺だけが scope で違い、他の成分（左辺・上辺・右辺・開始点・折返し基準・遠辺・原寸の幅）は
/// 両 scope で同値である——面 0 の枠の幅が同じで、`validrect` の宣言が 1 組しかないため。
#[test]
fn staysee_text_region_is_pinned_for_both_scopes() {
    for (scope, _) in SCOPE_FACE0_FILES {
        let image = scope_image_size(scope);
        let resolved = resolve_staysee(scope);
        let region = resolved.region;
        let bottom = expected_bottom(scope);

        assert_eq!(
            resolved.mode,
            WritingMode::HorizontalTb,
            "scope {scope}: `vertical` を宣言していないので横書きへ解決される（実測 {:?}）",
            resolved.mode
        );
        assert_eq!(
            region.left(),
            EXPECTED_LEFT,
            "scope {scope}: 描画範囲の左辺が {} ではなく {}",
            EXPECTED_LEFT,
            region.left()
        );
        assert_eq!(
            region.top(),
            EXPECTED_TOP,
            "scope {scope}: 描画範囲の上辺が {} ではなく {}",
            EXPECTED_TOP,
            region.top()
        );
        assert_eq!(
            region.right(),
            EXPECTED_RIGHT,
            "scope {scope}: 描画範囲の右辺が {} ではなく {}（幅 {} − 26 の解決）",
            EXPECTED_RIGHT,
            region.right(),
            image.0
        );
        assert_eq!(
            region.bottom(),
            bottom,
            "scope {scope}: 描画範囲の下辺が {} ではなく {}（高さ {} − 47 の解決）",
            bottom,
            region.bottom(),
            image.1
        );
        assert_eq!(
            region.start(),
            EXPECTED_START,
            "scope {scope}: 描画開始点が {:?} ではなく {:?}（`origin` 未宣言 → 書字開始角）",
            EXPECTED_START,
            region.start()
        );
        assert_eq!(
            region.image_size(),
            (image.0 as f32, image.1 as f32),
            "scope {scope}: 解決が保持する画像の原寸が {:?} ではなく {:?}",
            (image.0 as f32, image.1 as f32),
            region.image_size()
        );
    }
}

/// `wordwrappoint` を宣言していないので、折返し基準が描画範囲の遠辺へ縮退する。
///
/// 宣言が無いこと（読み口が `None` を返すこと）と、縮退した先が遠辺の実値 309 であることの
/// 両方を固定する。**両者が等しいことだけ**を見ると、片方がもう片方から導かれている実装では
/// 常に真になってしまうので、遠辺そのものの値も併せて押さえる。読み口が既定以外も返せることは
/// [`declared_balloon_returns_a_different_font_and_a_split_wrap_threshold`] が示す。
#[test]
fn undeclared_wrap_threshold_degenerates_to_the_inline_limit() {
    let model = staysee_model();
    assert_eq!(
        model.wordwrappoint().x(),
        None,
        "検体は `wordwrappoint.x` を宣言していないはず（実測 {:?}）",
        model.wordwrappoint().x()
    );
    assert_eq!(
        model.wordwrappoint().y(),
        None,
        "検体は `wordwrappoint.y` を宣言していないはず（実測 {:?}）",
        model.wordwrappoint().y()
    );

    for (scope, _) in SCOPE_FACE0_FILES {
        let region = resolve_staysee(scope).region;
        assert_eq!(
            region.inline_limit(),
            EXPECTED_RIGHT,
            "scope {scope}: 横書きの遠辺は描画範囲の右辺 {} のはずだが {}",
            EXPECTED_RIGHT,
            region.inline_limit()
        );
        assert_eq!(
            region.wrap_threshold(),
            EXPECTED_RIGHT,
            "scope {scope}: 未宣言の折返し基準は遠辺 {} へ縮退するはずだが {}",
            EXPECTED_RIGHT,
            region.wrap_threshold()
        );
        assert_eq!(
            region.wrap_threshold(),
            region.inline_limit(),
            "scope {scope}: 折返し基準 {} と遠辺 {} が分かれている（縮退していない）",
            region.wrap_threshold(),
            region.inline_limit()
        );
    }
}

// ── 既定書体の門（設計 C2 の F 行・DD4・要件 1.3）────────────────────────────

/// `font.name` を宣言していないので、書体が ukadoc の既定へ縮退し、高さは宣言どおりになる。
///
/// 名前は ukadoc の既定のリテラルと areka 側の定数の**両方**と突き合わせる。前者だけだと
/// areka の定数が別の書体へ書き換わっても気付けず、後者だけだと定数の書き換えに期待値が
/// 黙って追従して恒真になる。
#[test]
fn undeclared_font_name_resolves_to_the_canon_default() {
    let model = staysee_model();
    assert_eq!(
        model.font().name(),
        None,
        "検体は `font.name` を宣言していないはず（実測 {:?}）",
        model.font().name()
    );

    for (scope, _) in SCOPE_FACE0_FILES {
        let font = resolve_staysee(scope).font;
        assert_eq!(
            font.name, CANON_DEFAULT_FONT_NAME,
            "scope {scope}: 解決された書体が ukadoc 既定の `{}` ではなく `{}`",
            CANON_DEFAULT_FONT_NAME, font.name
        );
        assert_eq!(
            font.name, DEFAULT_FONT_NAME,
            "scope {scope}: 解決された書体 `{}` が areka 側の既定書体の定数 `{}` と食い違う",
            font.name, DEFAULT_FONT_NAME
        );
        assert!(
            font.fallback_chain.is_empty(),
            "scope {scope}: 宣言が無いので代替の候補列は空のはず（実測 {:?}）",
            font.fallback_chain
        );
        assert_eq!(
            font.height, EXPECTED_FONT_HEIGHT,
            "scope {scope}: 文字の高さが宣言どおりの {} ではなく {}",
            EXPECTED_FONT_HEIGHT, font.height
        );
    }
}

/// 既定書体で実際に測った文字の寸法を固定する（既定書体が無い環境ではここで赤く止まる）。
///
/// `ＭＳ ゴシック` は等幅で、半角が 0.5em・全角が 1em・行ボックスがちょうど 1em になる。
/// 別の書体へ落ちるとこの比が崩れるので、寸法そのものが「既定書体が居る」ことの門を兼ねる。
#[test]
fn default_font_metrics_are_pinned_at_height_twelve() {
    let resolved = resolve_staysee(0);
    let metrics = staysee_metrics(&resolved);
    let h = resolved.font.height;
    assert_eq!(
        h, EXPECTED_FONT_HEIGHT,
        "文字の高さが {EXPECTED_FONT_HEIGHT} ではなく {h}（以下の寸法はこの高さが前提）"
    );

    let half = metrics.advance('a', h);
    assert_eq!(
        half, EXPECTED_ADVANCE_HALF,
        "半角 `a` の送り幅が {EXPECTED_ADVANCE_HALF} ではなく {half}。\
         既定書体 `{CANON_DEFAULT_FONT_NAME}` がこの環境に無く別の書体へ落ちている疑いがある\
         （この値は当該書体の半角 0.5em の実測であり、代替書体のまま緑にしない）"
    );
    let full = metrics.advance('あ', h);
    assert_eq!(
        full, EXPECTED_ADVANCE_FULL,
        "全角 `あ` の送り幅が {EXPECTED_ADVANCE_FULL} ではなく {full}。\
         既定書体 `{CANON_DEFAULT_FONT_NAME}` がこの環境に無く別の書体へ落ちている疑いがある"
    );
    let box_height = metrics.line_box_height(h);
    assert_eq!(
        box_height, EXPECTED_LINE_BOX,
        "行ボックスの丈が {EXPECTED_LINE_BOX} ではなく {box_height}。\
         既定書体は `ascent + descent` がちょうど 1em なので、崩れていたら別の書体である"
    );
    let pitch = metrics.line_pitch(h);
    assert_eq!(
        pitch, EXPECTED_LINE_PITCH,
        "行送りが {EXPECTED_LINE_PITCH} ではなく {pitch}（正典式は `文字の高さ + 行間 2`）"
    );
}

// ── 対照: 宣言を持つ定義は別の答えを返す ──────────────────────────────────────

/// 同じ読み口へ**宣言を持つ**バルーン定義を通すと、書体は別の名前で返り、折返し基準と遠辺は
/// 別々の値に分かれる。
///
/// これが緑であることは、検体側で見た「既定書体へ落ちた」「折返し基準が遠辺へ縮退した」が
/// 読み口の空振りではないことの裏取りである。原寸は検体の本体側と同じ 335×205 を渡すので、
/// 答えの違いはバルーン定義の違いだけから来る。
#[test]
fn declared_balloon_returns_a_different_font_and_a_split_wrap_threshold() {
    let model = declared_model();
    assert_eq!(
        model.font().name(),
        Some(DECLARED_FONT_NAME),
        "対照は `font.name,{DECLARED_FONT_NAME}` を宣言しているはず（実測 {:?}）",
        model.font().name()
    );
    assert!(
        model.wordwrappoint().y().is_some(),
        "対照は `wordwrappoint.y` を宣言しているはず（実測 {:?}）",
        model.wordwrappoint().y()
    );

    let resolved = ResolvedBalloonText::resolve(&model, scope_image_size(0));

    assert_eq!(
        resolved.font.name, DECLARED_FONT_NAME,
        "対照の書体は宣言どおり `{}` で返るはずだが `{}`",
        DECLARED_FONT_NAME, resolved.font.name
    );
    assert_ne!(
        resolved.font.name, CANON_DEFAULT_FONT_NAME,
        "対照の書体が ukadoc 既定 `{CANON_DEFAULT_FONT_NAME}` で返った\
         ——読み口が宣言を無視して常に既定を返している（検体側の主張が空振りになる）"
    );

    let region = resolved.region;
    assert_eq!(
        region.wrap_threshold(),
        DECLARED_WRAP_THRESHOLD,
        "対照の折返し基準が {} ではなく {}",
        DECLARED_WRAP_THRESHOLD,
        region.wrap_threshold()
    );
    assert_eq!(
        region.inline_limit(),
        DECLARED_INLINE_LIMIT,
        "対照の遠辺が {} ではなく {}",
        DECLARED_INLINE_LIMIT,
        region.inline_limit()
    );
    assert_ne!(
        region.wrap_threshold(),
        region.inline_limit(),
        "対照でも折返し基準と遠辺が同じ値 {} になった\
         ——解決が折返し基準を常に遠辺から導いている（検体側の縮退の主張が空振りになる）",
        region.wrap_threshold()
    );
}
