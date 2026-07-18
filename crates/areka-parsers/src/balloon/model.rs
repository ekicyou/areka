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
}

impl BalloonModel {
    /// 各 sub-struct を束ねて集約ルートを構築する（同クレート写像・テスト用）。
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
        }
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
