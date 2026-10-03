//! # region — バルーン座標の画像空間解決と DPI/スケール契約（純粋層）
//!
//! origin／wordwrappoint／validrect の「負値=反対辺基準」解決・宣言 origin の範囲内外の解決・
//! `TextRegion`／`ScaleContract`（画像座標空間と物理座標空間の 2 空間のみ・論理 px 不在）を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//!
//! ## 座標空間は 2 つだけ（R4.6/R10.4・論理 px は存在しない）
//!
//! - **画像座標空間（image px）**: descript_balloon の全座標（origin/wordwrappoint/
//!   validrect）と `font.height` の単位。作者基準 DPI＝`descript_balloon.dpi`
//!   （省略時 96・ukadoc 正典）。レイアウト決定はすべてこの空間で行う。
//! - **物理座標空間（physical px）**: text_slot・swapchain・窓の単位。`物理 = 画像 × k`。
//!
//! k（合成スケール）の共有点＝`TextSlotView.scale`（バルーン surface と同一の
//! 合成スケール・**窓 DPI 由来ゆえ 1.0 とは限らない**）。k の算出は上流
//! （emo-present/placement）責務——本層は消費のみ。
//!
//! ## 負値=反対辺基準（ukadoc 脚注 *1 正典）
//!
//! `resolve(v, extent) = if v >= 0 { v } else { extent + v }`
//! （「マイナス座標はベース画像の右下からの相対」）。
//!
//! ## 描画開始点（spec `areka-P0-balloon-origin-outside-validrect` が正典）
//!
//! 描画開始点＝`resolve(origin)`。ただし解決後の値が validrect の当該軸の範囲（**両端を
//! 含む**）の外にある成分は、**宣言されていないものとして扱い**、書字開始角
//! （horizontal_tb/vertical_lr＝validrect 左上・vertical_rl＝右上）を用いる。範囲内の
//! 宣言は宣言どおりの位置、成分 `None` も書字開始角である。判定は x と y で独立に行い、
//! 無視した宣言の解決値は [`TextRegion::ignored_origin`] が運ぶ（警告は登録口が書く）。
//!
//! **撤去と取り下げの経緯**: かつて areka は「範囲外に解決された origin 成分を書字開始角へ
//! 寄せる」規約を持っていたが、2026-08-27 の開発者裁定で撤去し、宣言は validrect の内外を
//! 問わず宣言どおりとした（完了 spec `areka-P0-balloon-vertical-canon` の要件 3.10）。
//! 2026-09-18 にこの撤去を取り下げ、上の規則へ戻した——撤去の前提だった「範囲外の宣言は
//! ほかのベースウェアでも壊れた定義である」が、範囲外に origin を宣言した第三者のバルーンが
//! SSP では読めるのに areka では行頭が 1 文字欠ける、という実機目視で反証されたためである。
//! アーカイブ済み spec 本体は非改変とし、取り下げの事実は `doc/COMPAT_ARCHITECTURE.md`
//! §8 へ登記する。

use areka_parsers::balloon::BalloonModel;

use crate::writing::WritingMode;

/// 画像座標空間の値（単位を型で固定・論理 px は存在しない・R4.6/R10.4）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagePx(pub f32);

/// 物理座標空間の値（text_slot・swapchain・窓の単位・`物理 = 画像 × k`）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalPx(pub f32);

/// DPI/スケール契約（R4.6/R10.4 の一点定義）。
///
/// 画像座標空間と物理座標空間の 2 空間の変換規約をこの型で確定する:
///
/// | 変換 | 規約 |
/// |---|---|
/// | 画像→物理 | [`to_physical`](Self::to_physical)＝`画像 × k` |
/// | 物理→画像 | [`to_image`](Self::to_image)＝`物理 / k` |
/// | 物理寸（TextSurface/swapchain/Arrangement） | [`physical_extent`](Self::physical_extent)＝`ceil(寸 × k)` |
///
/// **image px 原寸はここでは導出しない**。作者画像空間の原寸は emo-present が native 原寸として
/// 正確に保持しており（`TextSlotView::surface_size`）、`TextSlotBinding::from_view` がそれを
/// そのまま透過する。かつて `image_size = round(物理 / k)` と逆写像で復元していたが、順写像の
/// ±0.5 物理px 誤差が k で割られて ±0.5/k 画像px へ増幅されるため **k<1 で 1px ずれた**
/// （2026-07-30 撤去）。
///
/// k の適用は TextSurface 生成寸と D2D `SetTransform(scale(k))` の一点のみ
/// （k の多重適用・混在を構造排除・design.md 不変条件 (3)）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleContract {
    /// バルーン surface と同一の合成スケール k（`TextSlotView.scale` 由来・現行 1.0）。
    pub scale: f32,
    /// `descript_balloon.dpi`（省略時 96・参考情報として保持。k の算出は上流責務）。
    pub author_dpi: u32,
}

impl ScaleContract {
    /// 合成スケール k と `descript_balloon.dpi`（キー欠落は `None`→96 既定）から構築する。
    ///
    /// 不正な k（0 以下・非有限）は `warn!`＋1.0（現行契約の物理 1:1）へ縮退する
    /// （log-first・panic 禁止）。
    pub fn new(scale: f32, author_dpi: Option<u32>) -> Self {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            tracing::warn!(
                scale,
                "不正な合成スケールのため 1.0（物理 1:1）へ縮退する（正: 有限かつ正の値）"
            );
            1.0
        };
        ScaleContract {
            scale,
            author_dpi: author_dpi.unwrap_or(96),
        }
    }

    /// 画像座標→物理座標（`物理 = 画像 × k`）。
    pub fn to_physical(&self, v: ImagePx) -> PhysicalPx {
        PhysicalPx(v.0 * self.scale)
    }

    /// 物理座標→画像座標（`画像 = 物理 / k`）。
    pub fn to_image(&self, v: PhysicalPx) -> ImagePx {
        ImagePx(v.0 / self.scale)
    }

    /// image px の寸から物理寸を導出する: `ceil(寸 × k)`
    ///
    /// # 裁定済み・許容（2026-08-14 開発者裁定・spec `areka-P0-scale-exact-rational`）
    ///
    /// `k` は `ScaleRatio::as_f32` 由来の f32 ゆえ非二進の比では真値とわずかにずれ、積が整数に
    /// なるはずの場合に `ceil` が **+1 されることがある**。影響は**文字供給面が 1px 大きくなる**
    /// ことのみ（レイアウトは image 空間で決まり、窓寸は丸め権威 `ScaleRatio::scaled_extent` が
    /// 別途決めるため、どちらも汚染しない）。
    ///
    /// この誤差を**厳密化せず f32 のまま引き回すこと**を **2026-08-14** に開発者が裁定した
    /// （`ScaleRatio` の num/den を emo-present 経由で文字層まで配管する厳密化案は却下）。
    ///
    /// ## 裁定の 4 根拠
    ///
    /// 1. **誤差の向きは常に +1 側のみで、−1 は起こらない**。振れるのは真の積が整数のときだけで、
    ///    真の積が整数でないときは整数までの距離が最低 `1/den` あり、約分後の分母は小さいため
    ///    f32 の相対誤差（~1e-7）では跨げない。ゆえに**文字が切れる方向には構造的に転ばない**。
    /// 2. **可視の不具合ではない**。レイアウトは image 空間で決まり、窓寸は丸め権威
    ///    `ScaleRatio::scaled_extent` が別途決める（emo-present `presenter/read.rs`）。供給面の生成は
    ///    初回解決時の 1 回きりで、フレーム毎の負荷にもならない。
    /// 3. **救える範囲が極小**。到達 23 比の総当たり（下記実測）で誤りが出るのは 6/5 と 12/5 の
    ///    各 81 件のみ。12/5 は 6/5 の 2 倍尺で f32 仮数が同一ゆえ、正体は**「1.2 の f32 表現」という
    ///    一点**に帰着する。残り 21 比は 0 件。
    /// 4. **費用が見合わない**。厳密化には拡大契約の構築口の署名変更（`TextSlotBinding::new` の
    ///    引数追加と `ScaleContract` の二重コンストラクタ化）と、それに伴う **112 箇所の呼び出し追随**
    ///    （本番 3・テスト 109・20 ファイル超。**2026-08-14 裁定時点の計測**——以後の再計測は本仕様の
    ///    テスト追加分だけ増える）が要り、追随の変換ミスが緑のまま通る危険もある。
    ///    不可視の 1px に対する対価として過大である。
    ///
    /// ## 実測
    ///
    /// **2026-08-14（到達 23 比の総当たり）**: 作者 DPI {72, 96, 120, 144} × モニタ DPI
    /// {96, 120, 144, 168, 192, 216, 240, 288} を約分・重複排除した **23 比** × 寸 1..=1200 ＝
    /// **27,600 組**を有理数の厳密 `div_ceil` と突き合わせた。差は常に **0 か 1**（**−1 は 1 件も
    /// 出ない**）。差 1 は **162 件**＝ **6/5 で 81 件・12/5 で 81 件**で、残る **21 比は 0 件**。
    /// 代表例は 6/5 の 寸 25 → **31**（真値 30）・12/5 の 寸 25 → **61**（真値 60）。
    ///
    /// **2026-07-30（先行実測・1..1200 の全 v）**——裁定の根拠として保持する:
    ///
    /// | k | f32 実値 | 誤り件数 / 1200 |
    /// |---|---|---|
    /// | 6/5（作者 120・窓 144＝150%） | 1.2000000477 | **81**（例: v=25 → 31・正 30） |
    /// | 4/3・8/5・4/5・2/3 | — | 0 |
    ///
    /// ## 出典
    ///
    /// 裁定の出典は spec **`areka-P0-scale-exact-rational`**（完了後は `.kiro/specs/completed/` 配下へ
    /// 移るため、パスではなく spec 名で辿る）。裁定の前提（差は 0 か 1・−1 は起きない・件数
    /// 81/81/0×21）は決定論テスト `tests/physical_extent_arbitration_test.rs` が固定しており、前提が
    /// 崩れれば赤になる（[[deferral-requires-verified-owner]]: 担当 spec は本仕様として実在し、裁定は
    /// 2026-08-14 に下りている——黙って先送りにはしていない）。
    ///
    /// ## 転記用の正典文面（他所へはこの一文をそのまま写す）
    ///
    /// > 唯一の既知の例外は emo-text `ScaleContract::physical_extent`（文字供給面の確保寸）であり、
    /// > 2026-08-14 の裁定（spec `areka-P0-scale-exact-rational`）に基づく。誤差は +1 側のみで不可視。
    /// > **この例外を他の用途へ拡大してはならない**。
    ///
    /// （TextSurface/swapchain/Arrangement の単位・物理 px 直接・論理 px 不在）。
    pub fn physical_extent(&self, v: ImagePx) -> u32 {
        (v.0 * self.scale).ceil() as u32
    }
}

/// 解決済みテキスト領域（**全値 image px**・validrect 絶対矩形・描画開始点・折返し閾値・
/// 描画範囲の行内軸の遠辺・バルーン画像原寸）。
///
/// physical への変換は TextSurface 生成寸と D2D SetTransform の一点のみ
/// （[`ScaleContract`] 経由・k の多重適用を構造排除）。折返し閾値の軸解釈は
/// [`WritingMode`] 依存（横書き＝x・縦書き＝y——design.md 軸読み替え正準表）。
///
/// ## 行内軸には意味の違う 2 つの値がある（spec `areka-P0-emo-text-line-height-canon` §4.3）
///
/// [`wrap_threshold`](Self::wrap_threshold)（`wordwrappoint` 由来）は「**ここを超えたら
/// 折り返す**」折返しの基準であり、[`inline_limit`](Self::inline_limit)（`validrect` の
/// 当該遠辺）は「**ここを超えてはならない**」絶対上限である。2 値は独立に保持し、
/// 一方をもう一方へ丸め込まない——丸め込むと絶対上限の意味論も、行末の禁則文字が基準を
/// 超えてぶら下がる余地（折返しの遅延）も表せなくなる（開発者裁定 2026-09-05）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextRegion {
    /// validrect 絶対矩形の左辺（image px）。
    left: f32,
    /// validrect 絶対矩形の上辺（image px）。
    top: f32,
    /// validrect 絶対矩形の右辺（image px）。
    right: f32,
    /// validrect 絶対矩形の下辺（image px）。
    bottom: f32,
    /// 描画開始点（範囲内の origin 宣言は宣言どおり・範囲外の宣言と未宣言成分は
    /// 書字開始角・image px）。
    start: (f32, f32),
    /// 折返し閾値（行内軸・image px。横書き＝x 値・縦書き＝y 値）。
    wrap_threshold: f32,
    /// 描画範囲の行内軸の遠辺（横書き＝`right`・縦書き＝`bottom`・image px）＝絶対上限。
    inline_limit: f32,
    /// バルーン画像の原寸（幅, 高さ・image px）。`resolve` の入口で受け取った値そのもの。
    image_size: (f32, f32),
    /// 範囲外ゆえ無視した origin 宣言の解決値（x, y。範囲内の宣言と未宣言は `None`）。
    /// 作者へ知らせる警告は actor の登録口が書く——本層は値を運ぶだけである。
    ignored_origin: (Option<f32>, Option<f32>),
}

/// 折返し基準が描画範囲の外に解決されたときの警告で、バルーン名の欄に載せる代替値。
///
/// 警告そのものを書くのは actor の登録口（`actor.rs` の `TextLayerRuntime::register_actor`）
/// である。本層は値を提供するだけなので、欄の代替値は本 const を共有する
/// （2 か所に別々の文字列を置くと、片方だけが直って静かに食い違う）。
///
/// `BalloonModel`（`areka-parsers` の balloon 集約ルート）は `descript.txt` の `name,` キーを
/// **写像していない**——写像対象キーを列挙しているのは同 crate の balloon parse の
/// `map_merged` であり、そこに `name` は無い（あるのは `font.name` で、これはフォント名で
/// あってバルーン名ではない）。名前を読めるようになるまでは欄をこの値で埋める。欄ごと
/// 落とさないのは、記録の無い経路を作らないためである（`.kiro/steering/logging.md`）。
pub(crate) const BALLOON_NAME_PLACEHOLDER: &str = "(名前なし)";

/// 行内軸の名前（横書き＝`"x"`・縦書き 2 方向＝`"y"`）——折返し警告の `axis` 欄の値。
///
/// 軸の割り当ては [`TextRegion::resolve`] の折返し基準・遠辺の解決（正準表）と同じであり、
/// **両者は必ず一致していなければならない**（片方だけを直すと、警告の軸欄が実際に解決した
/// 軸と食い違う）。檻は `actor_region_warn_tests.rs` の横書き・縦書き 2 本が持つ。
pub(crate) fn inline_axis_name(mode: WritingMode) -> &'static str {
    match mode {
        WritingMode::HorizontalTb => "x",
        WritingMode::VerticalRl | WritingMode::VerticalLr => "y",
    }
}

impl TextRegion {
    /// `BalloonModel`＋バルーン画像原寸（**image px**）＋`WritingMode` から解決する。
    /// 物理 px を渡すのはレビューエラー。原寸の出所は emo-present が保持する native 原寸
    /// （`TextSlotView::surface_size` を `TextSlotBinding::from_view` が透過）であって、
    /// [`ScaleContract`] からの逆写像ではない（逆写像は 2026-07-30 に撤去済み）。
    ///
    /// 受け取った原寸はそのまま保持し、[`image_size`](Self::image_size) で返す
    /// （`\_l` の `centerx`／`centery` の基準）。
    ///
    /// - validrect: 負値=反対辺基準で絶対値化。成分 `None` は画像全域の辺へ縮退
    ///   （`debug!` 記録）。退化矩形（幅/高さ ≤ 0）は `warn!`＋そのまま返す（縮退継続）。
    /// - 描画開始点: validrect の範囲内（両端を含む）へ解決された origin 成分は宣言どおり。
    ///   範囲外へ解決された成分と `None` 成分は書字開始角へ縮退する（どちらも `debug!` 記録）。
    ///   無視した宣言の解決値は [`ignored_origin`](Self::ignored_origin) が運ぶ。
    /// - 折返し閾値: 横書き＝`wordwrappoint.x`（負値=右辺基準）・縦書き＝`wordwrappoint.y`
    ///   （負値=下辺基準）。`None` は行内軸の validrect 遠辺へ縮退（領域端での自然折返し）。
    /// - 描画範囲の行内軸の遠辺（[`inline_limit`](Self::inline_limit)）: 横書き＝解決後の
    ///   `right`・縦書き＝解決後の `bottom`。折返し閾値がこの遠辺の**外**に解決されても
    ///   本関数は何も記録しない——粗さを知らせる警告は actor の登録口が書く（下の節）。
    ///
    /// ## 粗いバルーンの警告は本関数では書かない（2026-09-06・spec `areka-P0-emo2-conformance-e2e`）
    ///
    /// 折返し基準が遠辺の外に解決されたことを知らせる `warn!` は、かつて本関数の中にあった。
    /// しかし本関数は**毎フレーム**呼ばれる——再追従シーム（`actor.rs` の
    /// `TextLayerRuntime::refresh_actor_binding`）が churn ガードの判定キーを得るために
    /// 毎フレーム解き直すためで、実機の一周走行では生ログ 30,837 行のうち **27,908 行**が
    /// この 1 種類の警告になった（走行 A の実測・要件 14）。
    ///
    /// 「読み込み（装着）1 回につき 1 件」という意味を持つ層は、解決する側ではなく **actor の
    /// 登録口**（`actor.rs` の `TextLayerRuntime::register_actor`）である。ゆえに本関数から
    /// 粗さの記録を外し、登録口が「解決済み領域の値が新しく決まったとき」だけ 1 件書く。
    /// 文言と 4 つの欄（`balloon`・`axis`・`wrap_threshold`・`inline_limit`）は移動の前後で
    /// 1 文字も変えていない。
    ///
    /// 移したのは**この警告だけ**である——退化した validrect の `warn!` と未指定成分の縮退の
    /// `debug!` は別の症状の記録であり、そのまま残す（記録の無い縮退経路を作らない）。
    pub fn resolve(model: &BalloonModel, image_size: (u32, u32), mode: WritingMode) -> TextRegion {
        let (width, height) = (image_size.0 as f32, image_size.1 as f32);

        // ── validrect: 負値=反対辺基準の絶対値化（None は画像全域の辺へ） ──
        let vr = model.validrect();
        let left = resolve_or(vr.left(), width, 0.0, "validrect.left");
        let top = resolve_or(vr.top(), height, 0.0, "validrect.top");
        let right = resolve_or(vr.right(), width, width, "validrect.right");
        let bottom = resolve_or(vr.bottom(), height, height, "validrect.bottom");
        if right <= left || bottom <= top {
            tracing::warn!(
                left,
                top,
                right,
                bottom,
                "解決後の validrect が退化している（幅/高さ ≤ 0）——描画は空領域へ縮退する"
            );
        }

        // ── 描画開始点: 範囲内の宣言は宣言どおり・範囲外と未宣言は書字開始角へ（正準表参照） ──
        let start_corner = match mode {
            WritingMode::HorizontalTb | WritingMode::VerticalLr => (left, top),
            WritingMode::VerticalRl => (right, top),
        };
        let (start_x, ignored_x) = resolve_origin_component(
            model.origin().x(),
            width,
            (left, right),
            start_corner.0,
            "origin.x",
        );
        let (start_y, ignored_y) = resolve_origin_component(
            model.origin().y(),
            height,
            (top, bottom),
            start_corner.1,
            "origin.y",
        );

        // ── 折返し基準（soft）と描画範囲の遠辺（hard）: 行内軸は WritingMode 依存（正準表） ──
        // 遠辺は上で解決済みの right／bottom をそのまま採る（モデルから引き直さない——
        // 引き直すと未指定成分の縮退や負値解決が 2 か所に増える）。
        // 軸の割り当ては [`inline_axis_name`] と同じ表であり、両者は一致していなければならない。
        // 折返し基準が遠辺の外でも**ここでは記録しない**——毎フレーム呼ばれる本関数に粗さの
        // 記録を持たせず、actor の登録口が装着 1 回につき 1 件書く（要件 14.1）。
        let (wrap_threshold, inline_limit) = match mode {
            WritingMode::HorizontalTb => (
                resolve_or(model.wordwrappoint().x(), width, right, "wordwrappoint.x"),
                right,
            ),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (
                resolve_or(model.wordwrappoint().y(), height, bottom, "wordwrappoint.y"),
                bottom,
            ),
        };

        TextRegion {
            left,
            top,
            right,
            bottom,
            start: (start_x, start_y),
            wrap_threshold,
            inline_limit,
            image_size: (width, height),
            ignored_origin: (ignored_x, ignored_y),
        }
    }

    /// validrect 絶対矩形の左辺（image px）。
    pub fn left(&self) -> f32 {
        self.left
    }

    /// validrect 絶対矩形の上辺（image px）。
    pub fn top(&self) -> f32 {
        self.top
    }

    /// validrect 絶対矩形の右辺（image px）。
    pub fn right(&self) -> f32 {
        self.right
    }

    /// validrect 絶対矩形の下辺（image px）。
    pub fn bottom(&self) -> f32 {
        self.bottom
    }

    /// 描画開始点（範囲内の origin 宣言は宣言どおり・範囲外の宣言と未宣言成分は
    /// 書字開始角・image px）。
    pub fn start(&self) -> (f32, f32) {
        self.start
    }

    /// 範囲外ゆえ無視した origin 宣言の解決値（x, y）——作者向けの警告を書く登録口専用。
    ///
    /// 成分が `Some` であることは、その成分の [`start`](Self::start) が書字開始角であり
    /// かつ宣言が在ったことの**十分条件**である（範囲内の宣言と未宣言では `None`）。
    /// 逆は成り立たない——たとえば左辺 36 のバルーンに `origin.x,36` と宣言すると、範囲の
    /// 両端は範囲内なので宣言はそのまま用いられて本欄は `None` になるが、その宣言値は
    /// 書字開始角と一致している。開始点だけを見て「無視されたかどうか」を導いてはならない
    /// （辺ちょうどの宣言で必ず誤る）。無視されたかどうかを知る手段は本欄だけである。
    pub(crate) fn ignored_origin(&self) -> (Option<f32>, Option<f32>) {
        self.ignored_origin
    }

    /// 折返し閾値（行内軸・image px。横書き＝x 値・縦書き＝y 値）。
    ///
    /// 意味は「**ここを超えたら折り返す**」折返しの基準であって、超えてはならない上限では
    /// ない。上限は [`inline_limit`](Self::inline_limit) が別に持つ。
    pub fn wrap_threshold(&self) -> f32 {
        self.wrap_threshold
    }

    /// 描画範囲（validrect）の行内軸の遠辺（横書き＝[`right`](Self::right)・
    /// 縦書き＝[`bottom`](Self::bottom)・image px）。
    ///
    /// 意味は「**ここを超えてはならない**」絶対上限である——文字の遠端がこれを超えそうな
    /// ときは、折返し基準（[`wrap_threshold`](Self::wrap_threshold)）に関わらず無条件に
    /// 折り返す。web ページの文字列折返しと同じ二段構えであり、開発者裁定 2026-09-05
    /// （spec `areka-P0-emo-text-line-height-canon` の design §4.3・要件 6.2／6.3）による。
    ///
    /// 2 値は独立に読める。粗いバルーン定義では折返し基準がこの遠辺の外に解決されることが
    /// 実際にあり（出荷 fixture `emo2-kakukaku` の相方側は 254 > 240）、その場合も
    /// [`resolve`](Self::resolve) は両方の値をそのまま保持する（丸め込まない）。粗さを知らせる
    /// `warn!` は actor の登録口（`actor.rs` の `TextLayerRuntime::register_actor`）が装着
    /// 1 回につき 1 件書く。唯一の例外は行頭の 1 グリフで、遠辺より広い 1 文字は無限折返しを
    /// 避けるために置かれる——その判断は配置層（`layout`）の領分であり、本層は値を提供する
    /// だけである。
    pub fn inline_limit(&self) -> f32 {
        self.inline_limit
    }

    /// バルーン画像の原寸（幅, 高さ・image px）＝[`resolve`](Self::resolve) が受け取った
    /// `image_size` を f32 化しただけの値。
    ///
    /// **`\_l` の `centerx`／`centery` の基準はこの値である**——文字描画開始点（[`start`](Self::start)）
    /// でも文字描画範囲（validrect）でもなく、**バルーン画像そのもの**が基準になる。ukadoc 正典が
    /// 「これだけは文字描画開始点ではなくバルーン画像そのものが基準」と定めているためで、
    /// `centerx` は幅の半分・`centery` は高さの半分、書字方向には依らない
    /// （spec `areka-P0-cursor-tag-canon` の要件 4.3／4.4）。
    ///
    /// validrect は画像の部分矩形にすぎないので、**この値を validrect の幅・高さや辺と
    /// 取り違えてはならない**（檻: 本ファイルの
    /// `image_size_is_the_balloon_image_not_the_validrect_or_origin`）。
    pub fn image_size(&self) -> (f32, f32) {
        self.image_size
    }
}

/// 負値=反対辺基準の座標解決（ukadoc 脚注 *1 正典）:
/// `v >= 0` は絶対値素通し・負値は `extent + v`（右下辺からの相対）。
fn resolve_coord(v: i32, extent: f32) -> f32 {
    if v >= 0 { v as f32 } else { extent + v as f32 }
}

/// `Option` 成分の解決: `Some` は負値=反対辺基準で解決・`None` は fallback へ縮退
/// （`debug!` 記録・正常系に近い縮退につき warn にしない）。
fn resolve_or(v: Option<i32>, extent: f32, fallback: f32, key: &'static str) -> f32 {
    match v {
        Some(v) => resolve_coord(v, extent),
        None => {
            tracing::debug!(key, fallback, "未指定座標成分を既定辺へ縮退する");
            fallback
        }
    }
}

/// origin 成分の解決（spec `areka-P0-balloon-origin-outside-validrect` が正典）。
///
/// 返値は「開始点の成分」と「範囲外ゆえ無視した宣言の解決値」の対であり、第 2 要素は
/// **宣言が在り、かつ解決後の値が `range`（validrect の当該軸・両端を含む）の外**に
/// あったときだけ `Some` になる。
///
/// | `v` | 解決後の値 | 返す開始点 | 第 2 要素 | `debug!` |
/// |---|---|---|---|---|
/// | `None` | — | `corner` | `None` | 1 件 |
/// | `Some`・範囲内 | `resolve_coord(v, extent)` | 解決後の値 | `None` | 0 件（何も落としていない） |
/// | `Some`・範囲外 | 同上 | `corner` | `Some(解決後の値)` | 1 件 |
///
/// 負値は先に反対辺基準で絶対値化し（`areka-P0-balloon-vertical-canon` の要件 3.7・不変）、
/// **その解決後の値**で範囲の内外を判定する。判定は x と y で独立であり、片方の成分が
/// 範囲外でも他方は宣言どおりに残る。作者向けの警告（`warn!`）は本関数では書かない——
/// 本関数は毎フレーム呼ばれるので、「装着 1 回につき 1 件」を数えられるのは登録口だけである。
fn resolve_origin_component(
    v: Option<i32>,
    extent: f32,
    range: (f32, f32),
    corner: f32,
    key: &'static str,
) -> (f32, Option<f32>) {
    match v {
        Some(v) => {
            let resolved = resolve_coord(v, extent);
            if resolved < range.0 || range.1 < resolved {
                tracing::debug!(
                    key,
                    resolved,
                    range_min = range.0,
                    range_max = range.1,
                    corner,
                    "宣言された origin 成分が文字を描いてよい範囲の外にある——宣言を使わず書き始めの角を用いる"
                );
                (corner, Some(resolved))
            } else {
                (resolved, None)
            }
        }
        None => {
            tracing::debug!(key, corner, "未指定の origin 成分を書字開始角へ寄せる");
            (corner, None)
        }
    }
}

#[cfg(test)]
#[rustfmt::skip]
#[path = "region_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "region_inline_limit_tests.rs"]
mod inline_limit_tests;
#[cfg(test)]
#[path = "region_vertical_canon_tests.rs"]
mod vertical_canon_tests;
