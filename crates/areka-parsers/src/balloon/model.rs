//! balloon モデル型（I/O 契約の正本）。
//!
//! 集約ルート `BalloonModel` と幾何・フォント sub-struct
//! （`WindowPosition`/`Origin`/`WordWrapPoint`/`ValidRect`/`Font`/`FontColor`）を定義する。
//! これがクロスエンジン I/O 契約の正本であり、本パーサ（`balloon::parse`）が生成者、
//! 下流 `emo-text-layer`（バルーンテキスト）/`emo`（render）が消費者となる。
//!
//! 設計規律（design.md「Data Models」）:
//! - 各モデル化スカラを `Option<T>` 直持ちとし「未指定（`None`）」を型で表す。
//!   座標成分（x/y・t/b/l/r）と色成分（r/g/b）を**個別に** `Option` 化し、
//!   部分欠落を欠落なく表現する（要件 2.6/3.4）。`None` は `Some(0)` と判別される。
//! - 内部数値型は座標＝`i32`（符号付き・負値＝反対辺オフセットを保持・要件 4.1/4.2/4.3/4.5）、
//!   `font.height`＝`u32`（非負）、色成分＝`u8`（0–255）。
//! - フィールドは非公開とし read-only accessor のみ公開する（NewType/opaque 流儀・要件 2.8）。
//! - 全公開 struct に `#[non_exhaustive]` を付し、将来のキー追加を後方互換にする（要件 2.8）。
//! - 派生は最小: 整数のみの型は `Clone, Copy, Debug, PartialEq, Eq`。
//!   `Font` は `Option<String>` を含むため `Copy` 不可（`Clone, Debug, PartialEq, Eq` のみ）。
//! - モデル化 subset は emo2 が使う幾何＋フォントに限定する。choice/link/scroll 系キーは
//!   モデル化しない（要件 2.7・過剰実装抑止 要件 5.5）。
//! - 例外として `writing_mode`／`budoux_newline`（areka 拡張キー）を additive な生文字列転記
//!   フィールドとして持つ（値の解釈は下流 emo テキスト層・emo-text-layer 要件 5.6／
//!   budoux-newline 要件 1.1）。
//!
//! 構築は同クレートの `balloon::parse`（写像）とテストが公開/クレートパスで行う。
//! `new` コンストラクタ＋read-only accessor という不変値オブジェクト流儀（`sakura::SurfaceArg` 流儀）。

/// バルーンの幾何＋フォント subset モデル（クロスエンジン I/O 契約の正本）。
///
/// 「1 バルーンの幾何＋フォント設定」を表す不変値オブジェクト（集約ルート）。
/// 各 sub-struct は独立に「未指定（`None`）」を取り得る（要件 2.6/3.4）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalloonModel {
    windowposition: WindowPosition,
    origin: Origin,
    wordwrappoint: WordWrapPoint,
    validrect: ValidRect,
    font: Font,
    writing_mode: Option<String>,
    budoux_newline: Option<String>,
    cursor: BalloonCursor,
}

impl BalloonModel {
    /// 各 sub-struct を束ねて集約ルートを構築する（同クレート写像・テスト用）。
    ///
    /// `cursor` は additive 追加フィールドのため、既存呼び出し互換のため本コンストラクタでは
    /// `BalloonCursor::default()`（全キー未指定）で初期化する。cursor.* を持つ写像は
    /// [`BalloonModel::with_cursor`] で相乗り上書きする（`balloon::parse` が使用・要件 4.2）。
    pub fn new(
        windowposition: WindowPosition,
        origin: Origin,
        wordwrappoint: WordWrapPoint,
        validrect: ValidRect,
        font: Font,
        writing_mode: Option<String>,
        budoux_newline: Option<String>,
    ) -> Self {
        BalloonModel {
            windowposition,
            origin,
            wordwrappoint,
            validrect,
            font,
            writing_mode,
            budoux_newline,
            cursor: BalloonCursor::default(),
        }
    }

    /// cursor.* スタイルモデルを差し替えた値を返す additive ビルダ（不変値オブジェクト流儀）。
    ///
    /// `new` で基層を組んだ後、`balloon::parse` の KV 写像が cursor.* キー群を束ねた
    /// [`BalloonCursor`] を相乗りさせるために消費する（既存呼び出し側は `new` のまま不変・要件 4.2）。
    pub fn with_cursor(mut self, cursor: BalloonCursor) -> Self {
        self.cursor = cursor;
        self
    }

    /// バルーン配置調整値 `windowposition`（x, y）を読み取る（要件 2.1）。
    pub fn windowposition(&self) -> WindowPosition {
        self.windowposition
    }

    /// 文字描画原点 `origin`（x, y）を読み取る（要件 2.2）。
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// 自動折返し点 `wordwrappoint`（x, 存在すれば y）を読み取る（要件 2.3）。
    pub fn wordwrappoint(&self) -> WordWrapPoint {
        self.wordwrappoint
    }

    /// テキスト描画有効矩形 `validrect`（top/bottom/left/right）を読み取る（要件 2.4）。
    pub fn validrect(&self) -> ValidRect {
        self.validrect
    }

    /// フォント設定 `font`（name/height/color）を参照で読み取る（`String` を含むため参照返し・要件 2.5）。
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// 縦書き/横書き宣言 `writing_mode` の生文字列を読み取る（emo-text-layer 要件 5.6）。
    ///
    /// 2 層マージ済みの生値をそのまま転記したもの（値の解釈・語彙判定・fallback は
    /// 下流 emo テキスト層の責務・parser は転記に徹する）。未指定は `None`。
    pub fn writing_mode(&self) -> Option<&str> {
        self.writing_mode.as_deref()
    }

    /// 分かち書きワードラップ opt-in 宣言 `budoux_newline` の生文字列を読み取る（budoux-newline 要件 1.1）。
    ///
    /// 2 層マージ済みの生値をそのまま転記したもの（値の解釈・語彙判定・fallback は
    /// 下流 emo テキスト層の責務・parser は転記に徹する）。未指定は `None`。
    pub fn budoux_newline(&self) -> Option<&str> {
        self.budoux_newline.as_deref()
    }

    /// cursor.* 選択肢マーカースタイルモデル `cursor` を参照で読み取る（要件 4.2/6.2）。
    ///
    /// `Option<String>` を含むため参照返し（`font()` 流儀）。cursor.* 未記載バルーンでは
    /// 全フィールドが未指定（`BalloonCursor::default()`）となり、下流 `ResolvedChoiceStyle::resolve`
    /// が「未指定バルーン＝反転縮退」判定に用いる（要件 4.3/6.1）。
    pub fn cursor(&self) -> &BalloonCursor {
        &self.cursor
    }
}

/// `windowposition`（x: シェル側+/離れる側-、y: 下+/上-）。未指定は `None`（要件 2.1/2.6/4.2/4.3）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPosition {
    x: Option<i32>,
    y: Option<i32>,
}

impl WindowPosition {
    /// x/y を個別に `Option` で保持して構築する（部分欠落を欠落なく表現・要件 2.6）。
    pub fn new(x: Option<i32>, y: Option<i32>) -> Self {
        WindowPosition { x, y }
    }

    /// x 成分（未指定は `None`・`Some(0)` と判別・要件 2.6/4.2）。
    pub fn x(&self) -> Option<i32> {
        self.x
    }

    /// y 成分（未指定は `None`・`Some(0)` と判別・要件 2.6/4.3）。
    pub fn y(&self) -> Option<i32> {
        self.y
    }
}

/// `origin`（文字描画原点）。未指定は `None`（要件 2.2/2.6）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Origin {
    x: Option<i32>,
    y: Option<i32>,
}

impl Origin {
    /// x/y を個別に `Option` で保持して構築する（要件 2.6）。
    pub fn new(x: Option<i32>, y: Option<i32>) -> Self {
        Origin { x, y }
    }

    /// x 成分（未指定は `None`・要件 2.2/2.6）。
    pub fn x(&self) -> Option<i32> {
        self.x
    }

    /// y 成分（未指定は `None`・要件 2.2/2.6）。
    pub fn y(&self) -> Option<i32> {
        self.y
    }
}

/// `wordwrappoint`（x、y は存在すれば）。負値＝反対辺基準（要件 2.3/4.1/2.6）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordWrapPoint {
    x: Option<i32>,
    y: Option<i32>,
}

impl WordWrapPoint {
    /// x/y を個別に `Option` で保持して構築する（y 不在は `None`・要件 2.3/2.6）。
    pub fn new(x: Option<i32>, y: Option<i32>) -> Self {
        WordWrapPoint { x, y }
    }

    /// x 成分（負値＝反対辺基準を符号付きで保持・要件 4.1）。
    pub fn x(&self) -> Option<i32> {
        self.x
    }

    /// y 成分（存在しなければ `None`・`Some(0)` と判別・要件 2.3/2.6）。
    pub fn y(&self) -> Option<i32> {
        self.y
    }
}

/// `validrect`（top/bottom/left/right）。負値＝反対辺基準（要件 4.1）。各成分独立 `None`（要件 2.4/2.6）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidRect {
    top: Option<i32>,
    bottom: Option<i32>,
    left: Option<i32>,
    right: Option<i32>,
}

impl ValidRect {
    /// 4 辺を個別に `Option` で保持して構築する（部分欠落を欠落なく表現・要件 2.4/2.6/3.4）。
    pub fn new(
        top: Option<i32>,
        bottom: Option<i32>,
        left: Option<i32>,
        right: Option<i32>,
    ) -> Self {
        ValidRect {
            top,
            bottom,
            left,
            right,
        }
    }

    /// top 辺（未指定は `None`・負値＝反対辺基準・要件 2.4/4.1）。
    pub fn top(&self) -> Option<i32> {
        self.top
    }

    /// bottom 辺（未指定は `None`・負値＝反対辺基準・要件 2.4/4.1）。
    pub fn bottom(&self) -> Option<i32> {
        self.bottom
    }

    /// left 辺（未指定は `None`・負値＝反対辺基準・要件 2.4/4.1）。
    pub fn left(&self) -> Option<i32> {
        self.left
    }

    /// right 辺（未指定は `None`・負値＝反対辺基準・要件 2.4/4.1）。
    pub fn right(&self) -> Option<i32> {
        self.right
    }
}

/// `font`（name/height/color）。各成分独立 `None`（要件 2.5/2.6）。`String` を含むため `Copy` 不可。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Font {
    name: Option<String>,
    height: Option<u32>,
    color: FontColor,
}

impl Font {
    /// name/height/color を保持して構築する（name/height は未指定なら `None`・要件 2.5/2.6）。
    pub fn new(name: Option<String>, height: Option<u32>, color: FontColor) -> Self {
        Font {
            name,
            height,
            color,
        }
    }

    /// フォント名（未指定は `None`・借用で読む・要件 2.5/2.6）。
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// フォント高さ（非負・未指定は `None`・`Some(0)` と判別・要件 2.5/2.6）。
    pub fn height(&self) -> Option<u32> {
        self.height
    }

    /// フォント色（r/g/b・値返し・要件 2.5）。
    pub fn color(&self) -> FontColor {
        self.color
    }
}

/// `font.color`（r/g/b それぞれ 0–255）。各成分独立 `None`（要件 2.5/2.6・部分欠落を欠落なく表現）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontColor {
    r: Option<u8>,
    g: Option<u8>,
    b: Option<u8>,
}

impl FontColor {
    /// r/g/b を個別に `Option` で保持して構築する（部分欠落を欠落なく表現・要件 2.5/2.6）。
    pub fn new(r: Option<u8>, g: Option<u8>, b: Option<u8>) -> Self {
        FontColor { r, g, b }
    }

    /// 赤成分（0–255・未指定は `None`・`Some(0)` と判別・要件 2.5/2.6）。
    pub fn r(&self) -> Option<u8> {
        self.r
    }

    /// 緑成分（0–255・未指定は `None`・`Some(0)` と判別・要件 2.5/2.6）。
    pub fn g(&self) -> Option<u8> {
        self.g
    }

    /// 青成分（0–255・未指定は `None`・`Some(0)` と判別・要件 2.5/2.6）。
    pub fn b(&self) -> Option<u8> {
        self.b
    }
}

/// balloon descript の `cursor.*` スタイルキー群（選択肢マーカー・忠実転写・要件 4.2/6.2）。
///
/// 「`cursor,ファイル名`」（マウスカーソル画像キー）は別キーであり本モデルの対象外。
/// 各成分は `Option`（`None` と `Some(0)` を判別・既存パーサ規約）で保持し、値の解釈・
/// スタイル解決は下流（`ResolvedChoiceStyle::resolve`）に委ねる（parser は転記に徹する）。
/// 全キー未指定（`Default`）＝「未指定バルーン」判定の素材となる（要件 4.3/6.1）。
///
/// `#[derive(Default)]` は「全キー未指定」の素直な表現であり、additive フィールドとして
/// 既存コンストラクタ互換を保つために [`BalloonModel::new`] が用いる。`Eq` は集約ルート
/// `BalloonModel` の `Eq` 派生（全フィールド `Eq` 要求）を満たすため付与する。
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BalloonCursor {
    style: Option<String>,
    brush_color: CursorColor,
    pen_color: CursorColor,
    font_color: CursorColor,
    blendmethod: Option<String>,
}

impl BalloonCursor {
    /// style/brush/pen/font/blendmethod を束ねて構築する（同クレート写像・テスト用）。
    ///
    /// `style`（square/underline/square+underline/none）と `blendmethod`（none/notmaskpen/…）は
    /// 数値化せず生文字列で保持し、語彙判定・fallback は下流（`resolve`）の責務とする。
    pub fn new(
        style: Option<String>,
        brush_color: CursorColor,
        pen_color: CursorColor,
        font_color: CursorColor,
        blendmethod: Option<String>,
    ) -> Self {
        BalloonCursor {
            style,
            brush_color,
            pen_color,
            font_color,
            blendmethod,
        }
    }

    /// 選択肢マーカースタイル `cursor.style`（square / underline / square+underline / none・
    /// 未指定は `None`・生文字列転記・要件 4.2/6.5）。
    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }

    /// 矩形内塗り色 `cursor.brush.color.{r,g,b}`（hover 塗り色・要件 4.2）。
    pub fn brush_color(&self) -> CursorColor {
        self.brush_color
    }

    /// 枠/下線色 `cursor.pen.color.{r,g,b}`（語彙保持・M1 非参照・要件 6.2）。
    pub fn pen_color(&self) -> CursorColor {
        self.pen_color
    }

    /// hover 文字色 `cursor.font.color.{r,g,b}`（要件 4.2）。
    ///
    /// 既存 `font.color.*`（非 hover 文字色）とは別キー系であり、完全一致引きゆえ相互に
    /// 汚染しない（「cursor キーを font へ巻き込まない」既存不変条件の分離側・要件 6.2）。
    pub fn font_color(&self) -> CursorColor {
        self.font_color
    }

    /// ブレンド方式 `cursor.blendmethod`（none / notmaskpen / …・不透明転写・未指定は `None`・要件 6.5）。
    pub fn blendmethod(&self) -> Option<&str> {
        self.blendmethod.as_deref()
    }
}

/// `cursor.*.color`（`brush`/`pen`/`font` の r/g/b それぞれ 0–255）。各成分独立 `None`。
///
/// `FontColor` と同一表現を意図的にミラーする（r/g/b を個別に `Option<u8>` 化し、部分欠落を
/// 欠落なく表現・`None` と `Some(0)` を判別・要件 4.2/2.6）。`FontColor` を再利用せず別 NewType
/// とするのは、設計（BalloonCursorModel）が `CursorColor` を命名した意味論分離に従うため。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorColor {
    r: Option<u8>,
    g: Option<u8>,
    b: Option<u8>,
}

impl CursorColor {
    /// r/g/b を個別に `Option` で保持して構築する（部分欠落を欠落なく表現・要件 4.2/2.6）。
    pub fn new(r: Option<u8>, g: Option<u8>, b: Option<u8>) -> Self {
        CursorColor { r, g, b }
    }

    /// 赤成分（0–255・未指定は `None`・`Some(0)` と判別・要件 4.2/2.6）。
    pub fn r(&self) -> Option<u8> {
        self.r
    }

    /// 緑成分（0–255・未指定は `None`・`Some(0)` と判別・要件 4.2/2.6）。
    pub fn g(&self) -> Option<u8> {
        self.g
    }

    /// 青成分（0–255・未指定は `None`・`Some(0)` と判別・要件 4.2/2.6）。
    pub fn b(&self) -> Option<u8> {
        self.b
    }
}
