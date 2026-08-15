//! # region — バルーン座標の画像空間解決と DPI/スケール契約（純粋層）
//!
//! origin／wordwrappoint／validrect の「負値=反対辺基準」解決・origin クランプ正準・
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
//! ## origin クランプ正準（areka 独自・design.md が正典）
//!
//! 描画開始点＝`clamp(resolve(origin), validrect)`。成分 `None` または validrect 外は
//! 書字開始角（horizontal_tb/vertical_lr＝validrect 左上・vertical_rl＝右上）へ寄せる。

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

/// 解決済みテキスト領域（**全値 image px**・validrect 絶対矩形・描画開始点・折返し閾値）。
///
/// physical への変換は TextSurface 生成寸と D2D SetTransform の一点のみ
/// （[`ScaleContract`] 経由・k の多重適用を構造排除）。折返し閾値の軸解釈は
/// [`WritingMode`] 依存（横書き＝x・縦書き＝y——design.md 軸読み替え正準表）。
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
    /// 描画開始点（origin クランプ正準適用済み・image px）。
    start: (f32, f32),
    /// 折返し閾値（行内軸・image px。横書き＝x 値・縦書き＝y 値）。
    wrap_threshold: f32,
}

impl TextRegion {
    /// `BalloonModel`＋バルーン画像原寸（**image px**・`ScaleContract::image_size` の
    /// 一点導出値）＋`WritingMode` から解決する。物理 px を渡すのはレビューエラー。
    ///
    /// - validrect: 負値=反対辺基準で絶対値化。成分 `None` は画像全域の辺へ縮退
    ///   （`debug!` 記録）。退化矩形（幅/高さ ≤ 0）は `warn!`＋そのまま返す（縮退継続）。
    /// - 描画開始点: origin クランプ正準（`None`/範囲外成分は書字開始角へ・`debug!` 記録）。
    /// - 折返し閾値: 横書き＝`wordwrappoint.x`（負値=右辺基準）・縦書き＝`wordwrappoint.y`
    ///   （負値=下辺基準）。`None` は行内軸の validrect 遠辺へ縮退（領域端での自然折返し）。
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

        // ── 描画開始点: origin クランプ正準（書字開始角は正準表参照） ──
        let start_corner = match mode {
            WritingMode::HorizontalTb | WritingMode::VerticalLr => (left, top),
            WritingMode::VerticalRl => (right, top),
        };
        let start_x = clamp_origin_component(
            model.origin().x(),
            width,
            (left, right),
            start_corner.0,
            "origin.x",
        );
        let start_y = clamp_origin_component(
            model.origin().y(),
            height,
            (top, bottom),
            start_corner.1,
            "origin.y",
        );

        // ── 折返し閾値: 行内軸は WritingMode 依存（正準表）・None は遠辺へ ──
        let wrap_threshold = match mode {
            WritingMode::HorizontalTb => {
                resolve_or(model.wordwrappoint().x(), width, right, "wordwrappoint.x")
            }
            WritingMode::VerticalRl | WritingMode::VerticalLr => {
                resolve_or(model.wordwrappoint().y(), height, bottom, "wordwrappoint.y")
            }
        };

        TextRegion {
            left,
            top,
            right,
            bottom,
            start: (start_x, start_y),
            wrap_threshold,
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

    /// 描画開始点（origin クランプ正準適用済み・image px）。
    pub fn start(&self) -> (f32, f32) {
        self.start
    }

    /// 折返し閾値（行内軸・image px。横書き＝x 値・縦書き＝y 値）。
    pub fn wrap_threshold(&self) -> f32 {
        self.wrap_threshold
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

/// origin 成分のクランプ正準: `None`／解決後に validrect 範囲外は書字開始角成分へ寄せる
/// （範囲は両端含む・`debug!` 記録——fixture の origin 0,0 も通る正常系挙動）。
fn clamp_origin_component(
    v: Option<i32>,
    extent: f32,
    range: (f32, f32),
    corner: f32,
    key: &'static str,
) -> f32 {
    match v {
        Some(v) => {
            let resolved = resolve_coord(v, extent);
            if range.0 <= resolved && resolved <= range.1 {
                resolved
            } else {
                tracing::debug!(
                    key,
                    resolved,
                    range_min = range.0,
                    range_max = range.1,
                    corner,
                    "validrect 外の origin 成分を書字開始角へ寄せる（クランプ正準）"
                );
                corner
            }
        }
        None => {
            tracing::debug!(key, corner, "未指定の origin 成分を書字開始角へ寄せる");
            corner
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

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
    /// origin (0,0)・wordwrappoint.x,-49（balloons0s.txt 上書き）・
    /// validrect top,46／bottom,-56／left,36／right,-44。
    fn fixture_model() -> BalloonModel {
        model(
            (Some(0), Some(0)),
            (Some(-49), Some(0)),
            (Some(46), Some(-56), Some(36), Some(-44)),
        )
    }

    /// WARN イベント数を数える最小 Subscriber（writing.rs の檻パターン踏襲）。
    struct WarnCounter {
        warns: Arc<AtomicUsize>,
    }

    impl tracing::Subscriber for WarnCounter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.warns.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    /// クロージャを WARN 檻の中で実行し（戻り値, WARN 件数）を返す。
    fn count_warns<T>(f: impl FnOnce() -> T) -> (T, usize) {
        let warns = Arc::new(AtomicUsize::new(0));
        let subscriber = WarnCounter {
            warns: Arc::clone(&warns),
        };
        let value = tracing::subscriber::with_default(subscriber, f);
        (value, warns.load(Ordering::SeqCst))
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
        let descript: BTreeMap<String, String> = [
            ("origin.x", "0"),
            ("origin.y", "0"),
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

    // ── origin クランプ正準（areka 独自・design.md が正典） ──

    /// fixture origin (0,0) は validrect 外＝書字開始角 (left,top)=(36,46) へ寄る
    /// （SSP 表示実態と整合する期待座標）。
    #[test]
    fn fixture_origin_clamps_to_start_corner() {
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

    /// 範囲外成分だけが開始角へ寄り、範囲内成分は保持される（成分独立クランプ）。
    #[test]
    fn out_of_range_component_clamps_independently() {
        let m = model(
            (Some(100), Some(0)), // x は範囲内・y(0) は top(46) より上＝範囲外
            (None, None),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
        assert_eq!(region.start(), (100.0, 46.0));
    }

    /// origin の負値も反対辺基準で解決してからクランプ判定する。
    #[test]
    fn negative_origin_resolves_from_opposite_edge_before_clamp() {
        // origin.x,-100 → 400-100=300（validrect 36..356 内）・origin.y,-100 → 124（46..168 内）。
        let m = model(
            (Some(-100), Some(-100)),
            (None, None),
            (Some(46), Some(-56), Some(36), Some(-44)),
        );
        let region = TextRegion::resolve(&m, FIXTURE_IMAGE_SIZE, WritingMode::HorizontalTb);
        assert_eq!(region.start(), (300.0, 124.0));
    }

    /// vertical_rl の書字開始角は validrect 右上＝範囲外 origin の x は右端側へ寄る。
    #[test]
    fn vertical_rl_start_corner_is_top_right() {
        let region = TextRegion::resolve(
            &fixture_model(),
            FIXTURE_IMAGE_SIZE,
            WritingMode::VerticalRl,
        );
        assert_eq!(region.start(), (356.0, 46.0));
    }

    /// vertical_lr の書字開始角は validrect 左上（horizontal_tb と同じ角）。
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
}
