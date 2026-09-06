//! # region — バルーン座標の画像空間解決と DPI/スケール契約（純粋層）
//!
//! origin／wordwrappoint／validrect の「負値=反対辺基準」解決・宣言 origin の字義解決・
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
//! ## 描画開始点は宣言どおり（spec `areka-P0-balloon-vertical-canon` が正典）
//!
//! 描画開始点＝`resolve(origin)`。宣言された成分は validrect の内外を問わず**宣言どおりの
//! 位置**を用いる（同 spec の要件 3.10）——validrect の外にある宣言は `debug!` で記録する
//! だけで、位置は動かさない。成分 `None` のときだけ書字開始角
//! （horizontal_tb/vertical_lr＝validrect 左上・vertical_rl＝右上）へ縮退する（同 3.11）。
//!
//! **撤去された規約**: かつては areka 独自の「origin クランプ正準」
//! （`clamp(resolve(origin), validrect)`）を採っており、完了 spec
//! `areka-P0-emo-text-layer` の design.md（`:464` と `:716`）がそれを正典と称していた。
//! 2026-08-27 の開発者裁定で撤去——正典 ukadoc は `origin.x` について
//! 「通常は指定せず validrect の定義に任せる」と述べるだけで、範囲外宣言を寄せることを
//! 求めていないためである。アーカイブ済み spec 本体は非改変とし、上書きの事実は
//! `areka-P0-balloon-vertical-canon` が `doc/COMPAT_ARCHITECTURE.md` §8 へ登記する。

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
    /// 描画開始点（宣言された origin は字義・未宣言成分は書字開始角・image px）。
    start: (f32, f32),
    /// 折返し閾値（行内軸・image px。横書き＝x 値・縦書き＝y 値）。
    wrap_threshold: f32,
    /// 描画範囲の行内軸の遠辺（横書き＝`right`・縦書き＝`bottom`・image px）＝絶対上限。
    inline_limit: f32,
    /// バルーン画像の原寸（幅, 高さ・image px）。`resolve` の入口で受け取った値そのもの。
    image_size: (f32, f32),
}

/// 折返し基準が描画範囲の外に解決されたときの警告で、バルーン名の欄に載せる代替値。
///
/// `BalloonModel`（`areka-parsers` の balloon 集約ルート）は `descript.txt` の `name,` キーを
/// **写像していない**——写像対象キーを列挙しているのは同 crate の balloon parse の
/// `map_merged` であり、そこに `name` は無い（あるのは `font.name` で、これはフォント名で
/// あってバルーン名ではない）。名前を読めるようになるまでは欄をこの値で埋める。欄ごと
/// 落とさないのは、記録の無い経路を作らないためである（`.kiro/steering/logging.md`）。
const BALLOON_NAME_PLACEHOLDER: &str = "(名前なし)";

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
    /// - 描画開始点: 宣言された origin 成分は字義どおり（validrect 外なら `debug!` 記録・
    ///   位置は動かさない）。`None` 成分のみ書字開始角へ縮退（`debug!` 記録）。
    /// - 折返し閾値: 横書き＝`wordwrappoint.x`（負値=右辺基準）・縦書き＝`wordwrappoint.y`
    ///   （負値=下辺基準）。`None` は行内軸の validrect 遠辺へ縮退（領域端での自然折返し）。
    /// - 描画範囲の行内軸の遠辺（[`inline_limit`](Self::inline_limit)）: 横書き＝解決後の
    ///   `right`・縦書き＝解決後の `bottom`。折返し閾値がこの遠辺の**外**に解決された場合は
    ///   `warn!` を 1 件記録する（バルーン名・軸・両方の値）。
    ///
    /// ## 警告が「読み込み 1 回につき 1 回」になる理屈（持続 guard を持たない）
    ///
    /// 本関数はバルーンの装着（actor 登録）と合成スケール k の再追従でしか呼ばれず、
    /// フレームごとには呼ばれない。したがって静的な一回化の仕掛けを持たなくても
    /// 「読み込み 1 回につき 1 回」が構造で成り立つ。DPI 変化による k の再追従では
    /// 再解決＝再読込として改めて 1 件記録する。
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

        // ── 描画開始点: 宣言は字義・未宣言のみ書字開始角へ（書字開始角は正準表参照） ──
        let start_corner = match mode {
            WritingMode::HorizontalTb | WritingMode::VerticalLr => (left, top),
            WritingMode::VerticalRl => (right, top),
        };
        let start_x = resolve_origin_component(
            model.origin().x(),
            width,
            (left, right),
            start_corner.0,
            "origin.x",
        );
        let start_y = resolve_origin_component(
            model.origin().y(),
            height,
            (top, bottom),
            start_corner.1,
            "origin.y",
        );

        // ── 折返し基準（soft）と描画範囲の遠辺（hard）: 行内軸は WritingMode 依存（正準表） ──
        // 遠辺は上で解決済みの right／bottom をそのまま採る（モデルから引き直さない——
        // 引き直すと未指定成分の縮退や負値解決が 2 か所に増える）。
        let (wrap_threshold, inline_limit, axis) = match mode {
            WritingMode::HorizontalTb => (
                resolve_or(model.wordwrappoint().x(), width, right, "wordwrappoint.x"),
                right,
                "x",
            ),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (
                resolve_or(model.wordwrappoint().y(), height, bottom, "wordwrappoint.y"),
                bottom,
                "y",
            ),
        };
        if wrap_threshold > inline_limit {
            tracing::warn!(
                balloon = BALLOON_NAME_PLACEHOLDER,
                axis,
                wrap_threshold,
                inline_limit,
                "折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ）"
            );
        }

        TextRegion {
            left,
            top,
            right,
            bottom,
            start: (start_x, start_y),
            wrap_threshold,
            inline_limit,
            image_size: (width, height),
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

    /// 描画開始点（宣言された origin は字義・未宣言成分は書字開始角・image px）。
    pub fn start(&self) -> (f32, f32) {
        self.start
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
    /// 実際にあり（出荷 fixture `emo2-kakukaku` の相方側は 254 > 240）、その場合は
    /// [`resolve`](Self::resolve) が `warn!` を 1 件記録したうえで、両方の値をそのまま保持する
    /// （丸め込まない）。唯一の例外は行頭の 1 グリフで、遠辺より広い 1 文字は無限折返しを
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

/// origin 成分の解決（spec `areka-P0-balloon-vertical-canon` の要件 3.10／3.11）:
///
/// - `Some`: 負値=反対辺基準で絶対値化し、**宣言どおりの位置をそのまま返す**。
///   解決後の値が validrect の当該軸（`range`・両端含む）の外にあるときは `debug!` を
///   1 件記録するだけで、位置は動かさない。
/// - `None`: 書字開始角へ縮退する（`debug!` 記録・要件 3.11・撤去前と完全に同一）。
///
/// **不変条件**: `range` は**返す値に影響しない**（記録の判定にのみ用いる）。これが
/// 撤去された「origin クランプ正準」がもう残っていないことの、読み手向けの証拠である
/// （檻: `region_vertical_canon_tests.rs` の
/// `declared_origin_resolution_is_independent_of_validrect`）。
fn resolve_origin_component(
    v: Option<i32>,
    extent: f32,
    range: (f32, f32),
    corner: f32,
    key: &'static str,
) -> f32 {
    match v {
        Some(v) => {
            let resolved = resolve_coord(v, extent);
            if resolved < range.0 || range.1 < resolved {
                tracing::debug!(
                    key,
                    resolved,
                    range_min = range.0,
                    range_max = range.1,
                    "宣言された origin 成分が validrect の外にある——宣言どおりの位置を用いる"
                );
            }
            resolved
        }
        None => {
            tracing::debug!(key, corner, "未指定の origin 成分を書字開始角へ寄せる");
            corner
        }
    }
}

#[cfg(test)]
mod tests {

    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };
    use log_capture_kit::count_levels;

    use super::{ImagePx, PhysicalPx, ScaleContract, TextRegion};
    use crate::writing::WritingMode;

    /// fixture 実測のバルーン画像原寸（balloons0.png・image px）。
    const FIXTURE_IMAGE_SIZE: (u32, u32) = (400, 224);

    /// テスト用 BalloonModel 生成ヘルパ（幾何成分だけ指定・font/windowposition は未指定）。
    fn model(
        origin: (Option<i32>, Option<i32>),
        wordwrap: (Option<i32>, Option<i32>),
        validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    ) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(origin.0, origin.1),
            WordWrapPoint::new(wordwrap.0, wordwrap.1),
            ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
            None,
        )
    }

    /// fixture 実測値（2層マージ後）の BalloonModel:
    /// **origin は宣言しない**・wordwrappoint.x,-49（balloons0s.txt 上書き）・
    /// validrect top,46／bottom,-56／left,36／right,-44。
    ///
    /// origin の宣言が無いのは実フィクスチャに追随した結果である——`emo2-vertical`／
    /// `emo2-kakukaku` の `descript.txt` はかつて validrect 外の `origin.x,0`／`origin.y,0`
    /// を宣言していたが、spec `areka-P0-balloon-vertical-canon` の要件 10.9 で正典推奨形
    /// （「通常は指定せず validrect の定義に任せる」）へ是正され、宣言そのものが消えた。
    /// 未宣言時の書字開始角への縮退（要件 3.11）は不変なので、本ヘルパを使う檻の
    /// 開始点期待値は是正の前後で変わらない。
    fn fixture_model() -> BalloonModel {
        model(
            (None, None),
            (Some(-49), Some(0)),
            (Some(46), Some(-56), Some(36), Some(-44)),
        )
    }

    /// クロージャを共有のログ捕捉窓の中で実行し（戻り値, WARN 件数）を返す。
    ///
    /// 件数の集計は硬化機構の唯一の定義元 `log-capture-kit` の [`count_levels`] に委ねる。
    /// 戻り値の組は移行前と同一で、呼出側の判定内容は変わらない。
    fn count_warns<T>(f: impl FnOnce() -> T) -> (T, usize) {
        let (value, counts) = count_levels(f);
        (value, counts.warn)
    }

    // ── R4.3/R4.4: 負値=反対辺基準の解決（fixture 実測値で非退化矩形） ──

    /// fixture 実測 validrect（top46/bottom-56/left36/right-44・画像 400×224）が
    /// 画像座標空間の絶対矩形 (36,46)-(356,168) へ解決され、非退化である。
    #[test]
    fn fixture_validrect_resolves_to_nondegenerate_absolute_rect() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::HorizontalTb,
        );
        assert_eq!(region.left(), 36.0);
        assert_eq!(region.top(), 46.0);
        assert_eq!(region.right(), 356.0); // 400 + (-44)
        assert_eq!(region.bottom(), 168.0); // 224 + (-56)
        assert!(region.right() > region.left(), "非退化（幅 > 0）");
        assert!(region.bottom() > region.top(), "非退化（高さ > 0）");
    }

    /// 非負値は絶対座標として素通し（resolve(v, extent) = v for v >= 0）。
    #[test]
    fn nonnegative_validrect_passes_through_as_absolute() {
        let region = TextRegion::resolve(
            &model(
                (None, None),
                (None, None),
                (Some(10), Some(200), Some(20), Some(300)),
            ),
            FIXTURE_IMAGE_SIZE,
            WritingMode::HorizontalTb,
        );
        assert_eq!(region.left(), 20.0);
        assert_eq!(region.top(), 10.0);
        assert_eq!(region.right(), 300.0);
        assert_eq!(region.bottom(), 200.0);
    }

    /// validrect 成分 None は画像全域の辺へ縮退（left/top→0・right→幅・bottom→高さ）。
    #[test]
    fn missing_validrect_components_fall_back_to_image_edges() {
        let region = TextRegion::resolve(
            &model((None, None), (None, None), (None, None, None, None)),
            FIXTURE_IMAGE_SIZE,
            WritingMode::HorizontalTb,
        );
        assert_eq!(region.left(), 0.0);
        assert_eq!(region.top(), 0.0);
        assert_eq!(region.right(), 400.0);
        assert_eq!(region.bottom(), 224.0);
    }

    /// fixture の descript 基層のみ（validrect 全 0）は退化矩形＝warn を記録しつつ返す
    /// （log-first・縮退継続）。2層マージ後のみ非退化になる fixture 実態の再現。
    #[test]
    fn base_layer_only_validrect_is_degenerate_and_warns() {
        // descript.txt 基層実測: origin 0,0・wordwrappoint.x,-34・validrect 全 0。
        let base = model(
            (Some(0), Some(0)),
            (Some(-34), Some(0)),
            (Some(0), Some(0), Some(0), Some(0)),
        );
        let (region, warns) = count_warns(|| {
            TextRegion::resolve(&base, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb)
        });
        assert_eq!(region.left(), 0.0);
        assert_eq!(region.right(), 0.0);
        assert!(warns >= 1, "退化矩形は warn を記録する");
    }

    /// 2層マージ（balloon-parse 実機構）を通した fixture 再現:
    /// 基層（退化）＋画像別上書きの合成後のみ非退化になる。
    #[test]
    fn two_layer_merged_fixture_yields_nondegenerate_region() {
        use areka_parsers::balloon::parse;
        use std::collections::BTreeMap;

        // fixture 実測の関連キー subset（descript.txt 基層／balloons0s.txt 上書き層）。
        // origin は実フィクスチャと同じく宣言しない（要件 10.9 の是正後の姿）——本檻の
        // 関心は「2 層マージ後にのみ非退化領域が成立する」ことであり、開始点は
        // 未宣言→書字開始角の縮退（要件 3.11）で (36,46) になる。
        let descript: BTreeMap<String, String> = [
            ("wordwrappoint.x", "-34"),
            ("wordwrappoint.y", "0"),
            ("validrect.top", "0"),
            ("validrect.bottom", "0"),
            ("validrect.left", "0"),
            ("validrect.right", "0"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        let image: BTreeMap<String, String> = [
            ("wordwrappoint.x", "-49"),
            ("validrect.top", "46"),
            ("validrect.bottom", "-56"),
            ("validrect.left", "36"),
            ("validrect.right", "-44"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        let merged = parse(&descript, Some(&image));
        let (region, warns) = count_warns(|| {
            TextRegion::resolve(&merged, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb)
        });
        assert_eq!(
            (region.left(), region.top(), region.right(), region.bottom()),
            (36.0, 46.0, 356.0, 168.0)
        );
        assert_eq!(warns, 0, "非退化矩形は warn を記録しない");
        assert_eq!(region.start(), (36.0, 46.0));
        assert_eq!(region.wrap_threshold(), 351.0); // 400 + (-49)
    }

    // ── 描画開始点の解決（宣言は字義・未宣言は書字開始角） ──

    /// origin を宣言しない fixture は書字開始角 (left,top)=(36,46) から書き始める
    /// （要件 3.11 の縮退・SSP 表示実態と整合する期待座標）。
    #[test]
    fn fixture_without_origin_declaration_starts_at_writing_corner() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::HorizontalTb,
        );
        assert_eq!(region.start(), (36.0, 46.0));
    }

    /// validrect 内の origin はそのまま描画開始点になる。
    #[test]
    fn in_range_origin_is_kept_as_start_point() {
        let m = model(
            (Some(100), Some(50)),
            (None, None),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
        assert_eq!(region.start(), (100.0, 50.0));
    }

    /// origin 成分 None は書字開始角へ寄る（成分独立）。
    #[test]
    fn missing_origin_components_fall_back_to_start_corner() {
        let m = model(
            (None, None),
            (None, None),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
        assert_eq!(region.start(), (36.0, 46.0));
    }

    /// 宣言された成分は x・y それぞれ独立に**字義どおり**返る——範囲内でも範囲外でも
    /// 値は同じ扱いで、validrect は寄せ先として使われない（要件 3.10・クランプ撤去後の姿）。
    ///
    /// かつてこの檻は「範囲外成分だけが開始角へ寄る（成分独立クランプ）」を見ていた。
    /// 撤去でその規約自体が無くなったため、**成分独立性**という残る関心だけを引き継ぎ、
    /// 見るものを「独立に字義位置が返る」へ差し替えてある。
    #[test]
    fn origin_components_resolve_literally_and_independently() {
        let m = model(
            (Some(100), Some(0)), // x は範囲内・y(0) は top(46) より上＝範囲外
            (None, None),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
        // y は 46 へ寄らない（寄っていたら旧クランプが残っている）。
        assert_eq!(region.start(), (100.0, 0.0));
    }

    /// origin の負値も反対辺基準で解決してから字義どおり用いる（要件 3.7）。
    #[test]
    fn negative_origin_resolves_from_opposite_edge() {
        // origin.x,-100 → 400-100=300・origin.y,-100 → 224-100=124。
        let m = model(
            (Some(-100), Some(-100)),
            (None, None),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
        assert_eq!(region.start(), (300.0, 124.0));
    }

    /// vertical_rl の書字開始角は validrect 右上＝origin 未宣言なら x は右端側になる。
    #[test]
    fn vertical_rl_start_corner_is_top_right() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::VerticalRl,
        );
        assert_eq!(region.start(), (356.0, 46.0));
    }

    /// vertical_lr の書字開始角は validrect 左上（horizontal_tb と同じ角・origin 未宣言時）。
    #[test]
    fn vertical_lr_start_corner_is_top_left() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::VerticalLr,
        );
        assert_eq!(region.start(), (36.0, 46.0));
    }

    // ── 折返し閾値（軸解釈は WritingMode 依存・負値=反対辺基準） ──

    /// 横書き: wordwrappoint.x（fixture -49・負値=右辺基準）→ 400-49=351。
    #[test]
    fn horizontal_wrap_threshold_resolves_wordwrappoint_x() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::HorizontalTb,
        );
        assert_eq!(region.wrap_threshold(), 351.0);
    }

    /// 縦書き（vertical_rl／vertical_lr）: wordwrappoint.y（負値=下辺基準）。
    #[test]
    fn vertical_wrap_threshold_resolves_wordwrappoint_y() {
        let m = model(
            (None, None),
            (Some(-49), Some(-30)),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, mode);
            assert_eq!(region.wrap_threshold(), 194.0, "224 + (-30) for {mode:?}");
        }
    }

    /// fixture 実測 wordwrappoint.y,0 は 0 のまま忠実に解決される（縦書き折返しの
    /// 退化は design 織り込み済み・退化補正はレイアウト層の領分でなく本層は転記解決に徹する）。
    #[test]
    fn degenerate_wordwrappoint_y_zero_is_resolved_faithfully() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::VerticalRl,
        );
        assert_eq!(region.wrap_threshold(), 0.0);
    }

    /// 折返し点の成分 None は行内軸の validrect 遠辺へ縮退
    /// （横書き→right・縦書き→bottom＝領域端での自然折返し）。
    #[test]
    fn missing_wordwrappoint_falls_back_to_validrect_far_edge() {
        let m = model(
            (None, None),
            (None, None),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        let horizontal = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
        assert_eq!(horizontal.wrap_threshold(), 356.0);
        let vertical = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::VerticalRl);
        assert_eq!(vertical.wrap_threshold(), 168.0);
    }

    // ── R4.6/R10.4: DPI/スケール契約（画像座標空間と物理座標空間の 2 空間のみ） ──

    /// author_dpi 未指定（fixture 実態: dpi キー無し）は 96 へ既定化する（ukadoc 正典）。
    #[test]
    fn author_dpi_defaults_to_96() {
        assert_eq!(ScaleContract::new(1.0, None).author_dpi, 96);
        assert_eq!(ScaleContract::new(1.0, Some(144)).author_dpi, 144);
    }

    /// 現行契約 k=1.0（物理 1:1 表示）: 変換は恒等。
    #[test]
    fn scale_one_maps_identically() {
        let contract = ScaleContract::new(1.0, None);
        assert_eq!(contract.to_physical(ImagePx(36.0)), PhysicalPx(36.0));
        assert_eq!(contract.to_image(PhysicalPx(168.0)), ImagePx(168.0));
        assert_eq!(contract.physical_extent(ImagePx(320.0)), 320);
    }

    /// k=1.25／2.0 の写像: 物理=画像×k・画像=物理/k・物理寸=ceil(寸×k)。
    ///
    /// image px 原寸の導出はここには無い（2026-07-30 撤去）——原寸は presenter の native を
    /// `TextSlotBinding::from_view` が透過するのみで、この契約型は逆写像を持たない。
    #[test]
    fn nonunit_scale_maps_between_image_and_physical() {
        let contract = ScaleContract::new(1.25, None);
        assert_eq!(contract.to_physical(ImagePx(320.0)), PhysicalPx(400.0));
        assert_eq!(contract.to_image(PhysicalPx(400.0)), ImagePx(320.0));
        // 物理寸 = ceil(image 寸 × k)（validrect 幅 320 → 400・端数は切上げ）。
        assert_eq!(contract.physical_extent(ImagePx(320.0)), 400);
        assert_eq!(contract.physical_extent(ImagePx(321.0)), 402); // 401.25 → 402

        let doubled = ScaleContract::new(2.0, None);
        assert_eq!(doubled.to_physical(ImagePx(36.0)), PhysicalPx(72.0));
        assert_eq!(doubled.physical_extent(ImagePx(399.0)), 798);
    }

    /// 画像→物理→画像の往復が原値へ戻る（k≠1 含む）。
    #[test]
    fn image_physical_roundtrip_returns_original() {
        for k in [1.0f32, 1.25, 2.0] {
            let contract = ScaleContract::new(k, None);
            for v in [0.0f32, 36.0, 168.0, 351.0] {
                let roundtrip = contract.to_image(contract.to_physical(ImagePx(v)));
                assert!(
                    (roundtrip.0 - v).abs() < 1e-4,
                    "k={k}: {v} → 往復 {} は原値へ戻る",
                    roundtrip.0
                );
            }
        }
    }

    /// 不正スケール（0 以下・非有限）は warn を記録して 1.0 へ縮退する（log-first）。
    #[test]
    fn invalid_scale_falls_back_to_one_with_warn() {
        for bad in [0.0f32, -2.0, f32::NAN, f32::INFINITY] {
            let (contract, warns) = count_warns(|| ScaleContract::new(bad, None));
            assert_eq!(contract.scale, 1.0, "scale {bad} は 1.0 へ縮退する");
            assert_eq!(warns, 1, "scale {bad} はちょうど 1 回 warn を記録する");
        }
    }

    /// TextRegion の全値は image px（k 非依存）——resolve は ScaleContract を受けない
    /// シグネチャで構造的に担保されるが、値レベルでも fixture 座標がスケール概念と
    /// 無関係に一致することを固定する（レイアウト決定のスケール非依存・R4.6 前半）。
    #[test]
    fn text_region_values_are_image_px_independent_of_scale() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::HorizontalTb,
        );
        // image_size が同じである限り、どの k を仮定しても TextRegion は同一。
        // （物理への写像は ScaleContract 経由の一点のみ）
        assert_eq!(
            (region.left(), region.top(), region.right(), region.bottom()),
            (36.0, 46.0, 356.0, 168.0)
        );
        let contract = ScaleContract::new(2.0, None);
        // 物理寸への写像は消費側の一点適用（例: validrect 幅 320 → 物理 640）。
        assert_eq!(
            contract.physical_extent(ImagePx(region.right() - region.left())),
            640
        );
    }

    // ── areka-P0-cursor-tag-canon R4.3: バルーン画像原寸の保持（`\_l` の centerx／centery の基準） ──

    /// 檻の独立入力に使う原寸。`FIXTURE_IMAGE_SIZE` とは**別の値**であり、幅 ≠ 高さで、
    /// 下の各檻が宣言する validrect の 4 辺・幅・高さ・`start` のいずれとも一致しない。
    /// 「実装の値を読み戻すだけ」にならないよう、期待値はこの定数から直に書く。
    const ALT_IMAGE_SIZE: (u32, u32) = (531, 289);

    /// `image_size()` は `resolve` に渡した原寸をそのまま f32 で返し、3 書字方向で同一である
    /// （`centerx`／`centery` は書字方向に依らない——要件 4.4 の前提になる値）。
    #[test]
    fn image_size_returns_resolve_input_verbatim_in_every_writing_mode() {
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let alt = TextRegion::resolve(&fixture_model(), ALT_IMAGE_SIZE, mode);
            assert_eq!(alt.image_size(), (531.0, 289.0), "{mode:?}");
            let fixture = TextRegion::resolve(&fixture_model(), FIXTURE_IMAGE_SIZE, mode);
            assert_eq!(fixture.image_size(), (400.0, 224.0), "{mode:?}");
        }
    }

    /// 基準は**バルーン画像そのもの**であって validrect でも描画開始点でもない（要件 4.3）。
    /// validrect と origin を明示宣言し、それらが実際に効いていること（対照）を見たうえで、
    /// `image_size()` が validrect の 4 辺・幅高さ・`start` のどれとも一致しないことを固定する。
    #[test]
    fn image_size_is_the_balloon_image_not_the_validrect_or_origin() {
        // validrect (30,50)-(330,200)＝幅 300×高さ 150・origin (120,70)。
        // いずれの数も ALT_IMAGE_SIZE の 531／289 とは重ならない。
        let m = model(
            (Some(120), Some(70)),
            (None, None),
            (Some(50), Some(200), Some(30), Some(330)),
        );
        let region = TextRegion::resolve(&m, ALT_IMAGE_SIZE, WritingMode::HorizontalTb);
        // 対照: 宣言は確かに効いている（この檻は恒真ではない）。
        assert_eq!(
            (region.left(), region.top(), region.right(), region.bottom()),
            (30.0, 50.0, 330.0, 200.0)
        );
        assert_eq!(region.start(), (120.0, 70.0));
        // 本題: 画像原寸は渡された値のまま。
        assert_eq!(region.image_size(), (531.0, 289.0));
        assert_ne!(
            region.image_size(),
            (
                region.right() - region.left(),
                region.bottom() - region.top()
            ),
            "validrect の幅・高さとの取り違え"
        );
        assert_ne!(
            region.image_size(),
            (region.right(), region.bottom()),
            "validrect の右辺・下辺との取り違え"
        );
        assert_ne!(region.image_size(), region.start(), "start との取り違え");
    }

    /// validrect／origin／wordwrappoint を宣言してもしなくても `image_size()` は変わらない
    /// （画像原寸は宣言から導かれる値ではない）。
    #[test]
    fn image_size_is_unchanged_by_validrect_and_origin_declarations() {
        let declared = model(
            (Some(120), Some(70)),
            (Some(-10), Some(-10)),
            (Some(50), Some(200), Some(30), Some(330)),
        );
        let bare = model((None, None), (None, None), (None, None, None, None));
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let declared = TextRegion::resolve(&declared, ALT_IMAGE_SIZE, mode);
            let bare = TextRegion::resolve(&bare, ALT_IMAGE_SIZE, mode);
            assert_eq!(declared.image_size(), (531.0, 289.0), "{mode:?}");
            assert_eq!(bare.image_size(), declared.image_size(), "{mode:?}");
        }
    }
}

#[cfg(test)]
#[path = "region_inline_limit_tests.rs"]
mod inline_limit_tests;
#[cfg(test)]
#[path = "region_vertical_canon_tests.rs"]
mod vertical_canon_tests;
