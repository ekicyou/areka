use super::ViewboxExecutor;
use super::test_support::{
    Rig, block_axis_ink_span, build, glyph_items, live_diff_model_font, opaque_count,
};
use crate::canvas::ContentCanvas;
use crate::draw::{DWriteMetrics, DrawExecutor, ResolvedFont};
use crate::layout::{FixedMetrics, LayoutEngine, VisibleWindow, WrapPlan};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{TextItem, TextLayerConfig, TextLayerState};
use crate::surface::TextSurface;
use crate::writing::WritingMode;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

// ════ live-diff pixel 等価主檻（task 10・R4.5/R6.1/R6.2/R6.3/R6.5/R8.1・design Testing
//      Strategy「Integration Tests #1」） ════
//
// 同一プロセス・同一ターゲット型（headless World＋Compositor＋GraphicsCore）で、同一 cue 列・
// 同一注入時刻列を比較専用オラクル（DrawExecutor 全域再描画・#[cfg(test)]）と新実行部
// （ViewboxExecutor ダーティ矩形スクロール）の双方へ流し、read_back を k=1.0 で byte 比較する。
// オラクルは front へ全域再描画し、viewbox は back へダーティ描画→flip する——どちらも read_back
// は front を読むため直接比較できる（surface.rs の front 一本化契約）。Clear は actor.rs::apply_cue
// と同じ写像で両方式の Clear 適用点（oracle=行キャッシュ破棄／viewbox=planner 初期化＋FullClear
// 予約）を経由させる。

/// live-diff 検証リグ: 2 つの独立 TextSurface（同一物理寸・k=1.0・同一 core/compositor）へ
/// オラクルと viewbox を装着し、1 本の TextLayerState を両方式へ同一入力で流す。
struct LiveDiffRig {
    /// オラクル（全域再描画）の供給面。
    oracle_surface: TextSurface,
    /// viewbox（ダーティスクロール）の供給面。
    viewbox_surface: TextSurface,
    /// 比較専用オラクル（全域再描画・front へ焼く）。
    oracle: DrawExecutor,
    /// 新実行部（保持ピクセル面内 blit ＋ ダーティ描画・back へ焼き flip）。
    viewbox: ViewboxExecutor,
    /// 解決済みテキスト領域（両方式共通・image px）。
    region: TextRegion,
    /// 解決済みフォント（既定 ＭＳ ゴシック 10px）。
    font: ResolvedFont,
    /// DPI/スケール契約（k=1.0＝byte 一致の受け入れ基準）。
    contract: ScaleContract,
    /// 書字方向（横書き／vertical_rl）。
    mode: WritingMode,
    /// フォント高さ（純粋レイアウトへ渡す・FixedMetrics と対）。
    font_height: f32,
    /// 対象 actor（単一 actor で十分——byte 等価は actor 非依存）。
    actor: ActorKey,
    /// cue 駆動の純粋状態機械（両方式へ同一入力を供給する単一の正本）。
    state: TextLayerState,
    /// World／Compositor／Core／DispatcherQueue の寿命を束ねる（供給面より後に drop）。
    #[allow(dead_code)]
    rig: Rig,
}

impl LiveDiffRig {
    /// image px 原寸と mode から 2 面（オラクル／viewbox）を装着し、両実行部を生成する
    /// （k=1.0＝byte 一致の受け入れ基準・既定）。
    fn new(mode: WritingMode, image: (u32, u32)) -> LiveDiffRig {
        Self::new_scaled(mode, image, 1.0)
    }

    /// 合成スケール k を明示して装着する（k≠1.0＝byte 完全一致でなく ≤0.5px 許容の受け入れ
    /// 基準・G2）。フォントは既定 ＭＳ ゴシック 10px。
    fn new_scaled(mode: WritingMode, image: (u32, u32), k: f32) -> LiveDiffRig {
        Self::new_full(mode, image, k, None, 10)
    }

    /// フォント名・高さ・k を明示して装着する（G4——非 default フォント/大サイズで AA こぼれ
    /// ガードの実効性を byte 等価検証）。両面とも物理寸＝ceil(image × k)・region は image px。
    /// レイアウトは FixedMetrics（font 非依存の決定論位置）ゆえ font 名は AA ラスタライズ
    /// （描画）にのみ効く——両方式が同一 font ゆえガードが十分なら byte 等価が保たれる。
    fn new_full(
        mode: WritingMode,
        image: (u32, u32),
        k: f32,
        font_name: Option<&str>,
        font_height: u32,
    ) -> LiveDiffRig {
        let mut rig = Rig::new();
        let oracle_surface = rig.attach(image, k);
        let viewbox_surface = rig.attach(image, k);
        let model = live_diff_model_font(font_name, Some(font_height));
        let font = ResolvedFont::resolve(&model);
        let region = TextRegion::resolve(&model, image, mode);
        let contract = ScaleContract::new(k, None);
        let oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");
        let viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
        LiveDiffRig {
            oracle_surface,
            viewbox_surface,
            oracle,
            viewbox,
            region,
            font,
            contract,
            mode,
            font_height: font_height as f32,
            actor: ActorKey::from("0"),
            state: TextLayerState::default(),
            rig,
        }
    }

    /// cue を純粋状態へ適用し、Clear は両方式の Clear 適用点も呼ぶ（actor.rs::apply_cue と同一写像）。
    fn apply(&mut self, cue: &TalkCue) {
        if matches!(cue.command, CueCommand::Clear) {
            self.oracle.clear_cache();
            self.viewbox.request_clear();
        }
        self.state.apply_cue(cue);
    }

    /// Text cue（追記）を適用する。
    fn apply_text(&mut self, at: f64, s: &str) {
        let cue = TalkCue {
            at,
            actor: self.actor.clone(),
            command: CueCommand::Text(s.to_owned()),
            duration: 0.0,
        };
        self.apply(&cue);
    }

    /// NewLine cue（改行マーカー・ratio 1.0）を適用する。
    fn apply_newline(&mut self, at: f64) {
        let cue = TalkCue {
            at,
            actor: self.actor.clone(),
            command: CueCommand::NewLine { ratio: 1.0 },
            duration: 0.0,
        };
        self.apply(&cue);
    }

    /// Clear cue（全消去）を適用する。
    fn apply_clear(&mut self, at: f64) {
        let cue = TalkCue {
            at,
            actor: self.actor.clone(),
            command: CueCommand::Clear,
            duration: 0.0,
        };
        self.apply(&cue);
    }

    /// 注入時刻 `t` で state から可視 prefix→純粋レイアウト→canvas/window を導き、オラクルと
    /// viewbox を**同一入力**で描いて read_back を byte 比較する。`expect_opaque` で content
    /// フレーム（非透明 > 0）／Clear フレーム（全域透明）を区別し、空面同士の vacuous な一致を排除する。
    fn checkpoint(&mut self, label: &str, t: f64, expect_opaque: bool) -> VisibleWindow {
        // state → 可視 prefix → 純粋レイアウト（FixedMetrics で決定論・両方式は同一 canvas を受ける）。
        let items: Vec<TextItem> = self
            .state
            .actor_state(&self.actor)
            .map(|s| s.items().to_vec())
            .unwrap_or_default();
        let visible = self.state.visible_glyphs(&self.actor, t);
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &self.region,
            self.mode,
            self.font_height,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &self.region, self.mode);
        let canvas = ContentCanvas::from_layout(&lines, &self.region, self.mode);

        // オラクル（全域再描画・front）と viewbox（ダーティ描画・back→flip）を同一入力で描く。
        self.oracle
            .render(
                &canvas,
                &window,
                &self.font,
                self.mode,
                &self.contract,
                &mut self.oracle_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: オラクル render 失敗: {e:?}"));
        self.viewbox
            .render(
                &canvas,
                &window,
                &self.font,
                self.mode,
                &self.contract,
                &mut self.viewbox_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: viewbox render 失敗: {e:?}"));

        let ob = self
            .oracle_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: オラクル read_back 失敗: {e:?}"));
        let vb = self
            .viewbox_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: viewbox read_back 失敗: {e:?}"));

        // 非退化担保: content フレームはオラクル面に非透明ピクセルがある（空面比較=vacuous を排除）。
        if expect_opaque {
            assert!(
                opaque_count(&ob) > 0,
                "{label}: content フレームはオラクル面に非透明ピクセルを持つ（vacuous な空面一致を排除）"
            );
        } else {
            assert_eq!(
                opaque_count(&ob),
                0,
                "{label}: Clear 直後はオラクル面が全域透明（全 α=0）"
            );
        }

        // 主檻: k=1.0 で byte 完全一致（受け入れ基準——甘くしない）。
        assert_eq!(
            ob, vb,
            "{label}: k=1.0 で オラクル（全域再描画）と viewbox（ダーティスクロール）の read_back が byte 完全一致する"
        );
        window
    }

    /// k≠1.0 用の許容差チェックポイント（G2・R6.4）: byte 完全一致でなく、オラクル（真位置で
    /// 全域再描画）と viewbox（確定 content は whole-pixel blit で量子化位置・ダーティは真位置で
    /// 再描画）の read_back の**ブロック軸インク範囲**の差が `tol` 物理 px 以内であることを檻化する
    /// （小数アキュムレータで |committed − pos| ≤ 0.5 ⇒ 確定 content 位置差 ≤ ceil(0.5×k)）。
    /// k≠1.0 の実 GPU 描画経路（小数アキュムレータ→whole-pixel blit→実描画→readback）を実際に
    /// 走らせて検証する（従来 k=1.0 でしか走っていなかった経路）。返り値は VisibleWindow。
    fn checkpoint_block_tol(&mut self, label: &str, t: f64, tol: u32) -> VisibleWindow {
        let items: Vec<TextItem> = self
            .state
            .actor_state(&self.actor)
            .map(|s| s.items().to_vec())
            .unwrap_or_default();
        let visible = self.state.visible_glyphs(&self.actor, t);
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &self.region,
            self.mode,
            self.font_height,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &self.region, self.mode);
        let canvas = ContentCanvas::from_layout(&lines, &self.region, self.mode);

        self.oracle
            .render(
                &canvas,
                &window,
                &self.font,
                self.mode,
                &self.contract,
                &mut self.oracle_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: オラクル render 失敗: {e:?}"));
        self.viewbox
            .render(
                &canvas,
                &window,
                &self.font,
                self.mode,
                &self.contract,
                &mut self.viewbox_surface,
            )
            .unwrap_or_else(|e| panic!("{label}: viewbox render 失敗: {e:?}"));

        let ob = self
            .oracle_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: オラクル read_back 失敗: {e:?}"));
        let vb = self
            .viewbox_surface
            .read_back()
            .unwrap_or_else(|e| panic!("{label}: viewbox read_back 失敗: {e:?}"));

        let (w, h) = self.oracle_surface.size();
        // 非退化担保（空面同士の vacuous な一致を排除）。
        assert!(
            opaque_count(&ob) > 0,
            "{label}: オラクル面に非透明ピクセルがある"
        );
        assert!(
            opaque_count(&vb) > 0,
            "{label}: viewbox 面に非透明ピクセルがある"
        );

        let os = block_axis_ink_span(&ob, w, h, self.mode).expect("オラクル面のインク範囲");
        let vs = block_axis_ink_span(&vb, w, h, self.mode).expect("viewbox 面のインク範囲");
        let d0 = os.0.abs_diff(vs.0);
        let d1 = os.1.abs_diff(vs.1);
        assert!(
            d0 <= tol && d1 <= tol,
            "{label}: k≠1.0 でブロック軸インク範囲の差が ≤{tol}px（確定 content の量子化位置差 ≤0.5px）\
             ——oracle {os:?} vs viewbox {vs:?}（差 near={d0} far={d1}）"
        );
        window
    }
}

/// live-diff シナリオ（mode パラメタライズ）: あふれ前→スクロール発火直後→連続スクロール→
/// Clear 直後→Clear 後再追記 の 5 チェックポイントで、オラクルと viewbox の read_back が
/// 常に byte 完全一致することを檻化する（NewLine 区切りの単一グリフ行であふれを決定論制御）。
fn run_live_diff_scenario(mode: WritingMode, image: (u32, u32)) {
    let mut ld = LiveDiffRig::new(mode, image);
    run_live_diff_scenario_on(&mut ld);
}

/// 構築済みリグに対し 5 チェックポイント（あふれ前→スクロール発火→連続→Clear→再追記）を
/// byte 完全一致で檻化する（font/サイズを変えたリグで再利用——G4 の AA ガード実効性検証）。
///
/// 【newline-defer】本シナリオは全 cue を `at=0.0` で発行し、各 checkpoint 時点までに追記済み
/// content が完全リビールされる（＝末尾/部分の保留改行が残らない）。ゆえ各改行は次のグリフを
/// 伴って実体化し、あふれ発火のタイミングは遅延化の影響を受けない（幽霊空行由来の発火が無く
/// checkpoint 前提は不変）。発火時刻の後退は部分リビール（at 分散）を伴う診断ダンプ側の論点。
fn run_live_diff_scenario_on(ld: &mut LiveDiffRig) {
    // ① あふれ前（3 行・可視窓は不動）。
    ld.apply_text(0.0, "あ");
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "い");
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "う");
    let w = ld.checkpoint("あふれ前", 10.0, true);
    assert_eq!(
        (w.first_visible_line, w.block_offset),
        (0, 0.0),
        "あふれ前は可視窓が動かない（先頭可視行 0・オフセット 0）: {w:?}"
    );

    // ② スクロール発火直後（4 行目であふれ・可視窓が初めて移動）。
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "え");
    let w = ld.checkpoint("スクロール発火直後", 20.0, true);
    assert!(
        w.first_visible_line >= 1,
        "あふれ発火で可視窓が移動する（先頭可視行 ≥ 1）: {w:?}"
    );
    assert_ne!(
        w.block_offset, 0.0,
        "発火後はブロックオフセットが非零: {w:?}"
    );

    // ③ 連続スクロール（さらに 2 行追記・複数回スクロール）。
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "お");
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "か");
    let w = ld.checkpoint("連続スクロール", 30.0, true);
    assert!(
        w.first_visible_line >= 2,
        "連続スクロールで可視窓が複数行進む（先頭可視行 ≥ 2）: {w:?}"
    );

    // ④ Clear 直後（両方式の Clear 適用点を経由・全域透明）。
    ld.apply_clear(0.0);
    let w = ld.checkpoint("Clear 直後", 40.0, false);
    assert_eq!(
        w.first_visible_line, 0,
        "Clear 後は既定窓（先頭可視行 0）: {w:?}"
    );

    // ⑤ Clear 後再追記（新規 content が全域ダーティで復帰）。
    ld.apply_text(0.0, "ら");
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "り");
    let w = ld.checkpoint("Clear 後再追記", 50.0, true);
    assert_eq!(
        w.first_visible_line, 0,
        "短い再追記はあふれない（先頭可視行 0）: {w:?}"
    );
}

/// 観測可能な完了状態（前半・横書き）: 横書きの全シナリオでオラクルと viewbox の read_back が
/// k=1.0 で byte 完全一致する（R6.1/R6.2/R6.5/R4.5/R8.1）。
#[test]
fn live_diff_horizontal_matches_oracle_byte_for_byte() {
    run_live_diff_scenario(WritingMode::HorizontalTb, (80, 40));
}

/// 観測可能な完了状態（前半・縦書き vertical_rl）: 縦書きの全シナリオでも byte 完全一致する
/// （R6.3——横/縦 両 mode パラメタライズ）。
#[test]
fn live_diff_vertical_rl_matches_oracle_byte_for_byte() {
    run_live_diff_scenario(WritingMode::VerticalRl, (40, 80));
}

/// 観測可能な完了状態（前半・縦書き vertical_lr）: 縦書き左送り（列が左→右へ流れ、あふれ時は
/// content が**左へ** blit・露出帯＝**右端**）でも全シナリオで byte 完全一致する（R5.2/R6.3——
/// スクロール軸は横（x）・vertical_rl と同一の x 軸だが符号が逆＝blit 左方向。純粋層の軸写像
/// unit 檻（task 3.2）だけでなく、実 GPU 描画 readback で oracle↔viewbox の byte 等価を証明する）。
#[test]
fn live_diff_vertical_lr_matches_oracle_byte_for_byte() {
    run_live_diff_scenario(WritingMode::VerticalLr, (40, 80));
}

/// k≠1.0（非96DPI）のスクロール実描画を許容差で檻化する（G2・R6.4）。あふれ→スクロールの
/// 各段で、オラクル（真位置・全域再描画）と viewbox（確定 content は whole-pixel blit で量子化・
/// ダーティは真位置）の read_back のブロック軸インク範囲が `tol` 物理 px 以内に収まることを実 GPU
/// 描画で確認する（小数アキュムレータ→whole-pixel blit→実描画→readback の経路を実走）。
/// byte 完全一致は k=1.0 に scope（design）ゆえ k≠1.0 は ≤0.5px 相当（ceil(0.5×k)＋AA 余白）の許容差。
fn run_live_diff_nonunit_scale(mode: WritingMode, image: (u32, u32), k: f32, tol: u32) {
    let mut ld = LiveDiffRig::new_scaled(mode, image, k);

    // ① あふれ前（3 行）。
    ld.apply_text(0.0, "あ");
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "い");
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "う");
    ld.checkpoint_block_tol("k≠1 あふれ前", 10.0, tol);

    // ② スクロール発火直後。
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "え");
    let w = ld.checkpoint_block_tol("k≠1 スクロール発火", 20.0, tol);
    assert!(
        w.first_visible_line >= 1,
        "k≠1 あふれ発火で可視窓が移動: {w:?}"
    );

    // ③ 連続スクロール。
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "お");
    ld.apply_newline(0.0);
    ld.apply_text(0.0, "か");
    let w = ld.checkpoint_block_tol("k≠1 連続スクロール", 30.0, tol);
    assert!(
        w.first_visible_line >= 2,
        "k≠1 連続スクロールで可視窓が複数行進む: {w:?}"
    );
}

/// 横書き k=1.25: あふれ→スクロール実描画で viewbox が oracle とブロック軸 ≤2px（≈ceil(0.5×1.25)
/// ＋AA 余白）に収まる（R6.4——小数アキュムレータの実 GPU 経路検証）。
#[test]
fn live_diff_nonunit_scale_horizontal_within_tolerance() {
    run_live_diff_nonunit_scale(WritingMode::HorizontalTb, (80, 40), 1.25, 2);
}

/// 縦書き vertical_rl k=1.25: 同上（スクロール軸＝横 x）。
#[test]
fn live_diff_nonunit_scale_vertical_rl_within_tolerance() {
    run_live_diff_nonunit_scale(WritingMode::VerticalRl, (40, 80), 1.25, 2);
}

/// 縦書き vertical_lr k=1.25: 同上（スクロール軸＝横 x・左送り）。
#[test]
fn live_diff_nonunit_scale_vertical_lr_within_tolerance() {
    run_live_diff_nonunit_scale(WritingMode::VerticalLr, (40, 80), 1.25, 2);
}

/// G4: 大サイズフォント（20px・既定 ＭＳ ゴシック）でも全シナリオで oracle↔viewbox byte
/// 完全一致する——AA こぼれが `DIRTY_GUARD_IMG_PX`(=1 image px) を超えず、ダーティ矩形が
/// AA を取りこぼさないことを実描画 byte 等価で確認（spike/live-diff は従来 10px のみ）。
/// image 寸は `2P+F ≤ block ≤ 3P+F`（P=20+2=22・F=20 ゆえ 64 ≤ 80 ≤ 86）を満たし
/// ①3行収容／②あふれ。行送りが 25 から 22 へ縮んでも 80 は範囲内ゆえ面寸は据え置き。
#[test]
fn live_diff_larger_font_matches_oracle_byte_for_byte() {
    run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
        WritingMode::HorizontalTb,
        (160, 80),
        1.0,
        None,
        20,
    ));
    run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
        WritingMode::VerticalRl,
        (80, 160),
        1.0,
        None,
        20,
    ));
}

/// G4: プロポーショナルフォント（ＭＳ Ｐゴシック・可変幅）でも byte 完全一致する——font 種別
/// を変えても両方式が同一 format ゆえ AA ラスタライズが一致し、ガードが十分なら byte 等価が
/// 保たれることを確認（P=12+2=14・F=12 ゆえ `2P+F ≤ block ≤ 3P+F` は 40 ≤ 50 ≤ 54・
/// 行送りが 15 から 14 へ縮んでも 50 は範囲内ゆえ面寸は据え置き）。
#[test]
fn live_diff_proportional_font_matches_oracle_byte_for_byte() {
    run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
        WritingMode::HorizontalTb,
        (80, 50),
        1.0,
        Some("ＭＳ Ｐゴシック"),
        12,
    ));
    run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
        WritingMode::VerticalRl,
        (50, 80),
        1.0,
        Some("ＭＳ Ｐゴシック"),
        12,
    ));
}

/// 観測可能な完了状態（後半・負のコントロール）: 意図的に不一致を起こす細工（viewbox 側だけ
/// 1 グリフ多い content を描く）を入れると read_back の byte 比較が差を検出する——比較器は
/// ゼロでない差を捕捉できる（＝live-diff はトートロジーでない）。本物のバグ注入ではなく、
/// 比較器の識別能力を示す最小の細工。
#[test]
fn live_diff_detects_injected_divergence() {
    let mut ld = LiveDiffRig::new(WritingMode::HorizontalTb, (60, 40));

    // オラクルは 1 グリフ・viewbox は 2 グリフ（1 グリフぶん確実に異なる pixel）を同一窓で描く。
    let items_oracle = glyph_items("■");
    let (canvas_oracle, window) = build(&items_oracle, &ld.region, ld.mode, ld.font_height);
    let items_viewbox = glyph_items("■■");
    let (canvas_viewbox, _) = build(&items_viewbox, &ld.region, ld.mode, ld.font_height);

    ld.oracle
        .render(
            &canvas_oracle,
            &window,
            &ld.font,
            ld.mode,
            &ld.contract,
            &mut ld.oracle_surface,
        )
        .expect("オラクル render 失敗");
    ld.viewbox
        .render(
            &canvas_viewbox,
            &window,
            &ld.font,
            ld.mode,
            &ld.contract,
            &mut ld.viewbox_surface,
        )
        .expect("viewbox render 失敗");

    let ob = ld
        .oracle_surface
        .read_back()
        .expect("オラクル read_back 失敗");
    let vb = ld
        .viewbox_surface
        .read_back()
        .expect("viewbox read_back 失敗");

    // 双方とも非退化（空面同士の vacuous な不一致でない）。
    assert!(
        opaque_count(&ob) > 0,
        "オラクル面は非透明（content が描かれている）"
    );
    assert!(
        opaque_count(&vb) > 0,
        "viewbox 面は非透明（content が描かれている）"
    );
    // 比較器は差を検出する（＝live-diff の有効性・トートロジーでない）。
    assert_ne!(
        ob, vb,
        "異なる content（1 グリフ差）は byte 比較で差として検出される——live-diff は有効"
    );
}

/// 回帰檻（実機の行下端欠け・D2）: **実 fixture の実フォント Yu Gothic UI** を使い、typewriter
/// 前進＋あふれスクロールの全フレームで oracle（全域再描画）と viewbox（ダーティスクロール）の
/// read_back が byte 完全一致することを檻化する。
///
/// このバグ（各行 em ボックス下端の 2〜3px はみ出しインクをダーティが切り落とす）は既定 ＭＳ
/// ゴシック（はみ出さない字）では出ず、`diag_line_boundary_dropout_vs_oracle`（ＭＳゴシック）が
/// 見逃していた。Yu Gothic UI は descent 側へインクがこぼれるため、[`crate::viewbox::BLOCK_INK_BLEED_FRACTION`]
/// による行スロット拡張が無いと diverge する（＝この檻が修正前に落ちる）。fixture が読めない環境
/// （font 不在等）でも oracle↔viewbox は font に依らず一致するため頑健。実 example の cue `at`
/// スケジュール（LINE1 0.0／LINE2 0.5／LINE3 1.2／あふれ 2.0）を忠実再現する。
#[test]
fn yugothic_real_fixture_matches_oracle_byte_for_byte() {
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku");
    let read_dec = |name: &str| -> String {
        let bytes = std::fs::read(fixture_dir.join(name))
            .unwrap_or_else(|e| panic!("fixture {name} 読取失敗: {e}"));
        areka_parsers::charset::decode(&bytes, areka_parsers::charset::DefaultEncoding::Utf8)
    };
    let model = areka_parsers::balloon::parse_str(
        &read_dec("descript.txt"),
        Some(&read_dec("balloons0s.txt")),
    );
    let balloon_image = (400u32, 224u32);
    let resolved = crate::actor::ResolvedBalloonText::resolve(&model, balloon_image);
    let mode = resolved.mode;
    let font = &resolved.font;
    let region = &resolved.region;
    assert_eq!(
        mode,
        WritingMode::HorizontalTb,
        "fixture は横書き（D2 は横書き不具合）"
    );

    let image = (
        (region.right() - region.left()).ceil() as u32,
        (region.bottom() - region.top()).ceil() as u32,
    );
    let mut rig = Rig::new();
    let mut oracle_surface = rig.attach(image, 1.0);
    let mut viewbox_surface = rig.attach(image, 1.0);
    let contract = ScaleContract::new(1.0, None);
    let config = TextLayerConfig::default();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let metrics = DWriteMetrics::new(&factory, font, mode, &config).expect("DWriteMetrics");
    let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
    let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");
    let actor = ActorKey::from("0");
    let mut state = TextLayerState::default();
    // reveal は配送 duration 由来（interval = duration / N）。Text へ N×0.05 を焼き込むと
    // 各 chunk 内が旧 char_wait=0.05 と同一ペースで per-glyph 進行する（chunk 境界は at で gate）。
    let cue_at = |at: f64, cmd: CueCommand| TalkCue {
        at,
        actor: actor.clone(),
        duration: match &cmd {
            CueCommand::Text(t) => t.chars().count() as f64 * 0.05,
            _ => 0.0,
        },
        command: cmd,
    };
    state.apply_cue(&cue_at(0.0, CueCommand::Text("おっはよー！".into())));
    state.apply_cue(&cue_at(0.5, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue_at(
        0.5,
        CueCommand::Text("めっちゃええ朝やん！".into()),
    ));
    state.apply_cue(&cue_at(1.2, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue_at(1.2, CueCommand::Text("今日もいくでー！".into())));
    for _ in 0..9 {
        state.apply_cue(&cue_at(2.0, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue_at(2.0, CueCommand::Text("ほな".into())));
    }

    // 密な時間格子で前進提示（viewbox は状態依存ゆえ昇順に全フレーム）。各フレームで byte 比較。
    let mut checked_opaque = false;
    let mut t = 0.0f64;
    while t <= 3.05 {
        let visible = state.visible_glyphs(&actor, t);
        let items: Vec<TextItem> = state
            .actor_state(&actor)
            .map(|s| s.items().to_vec())
            .unwrap_or_default();
        let lines = LayoutEngine::layout(
            &items,
            visible,
            region,
            mode,
            font.height,
            &metrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, region, mode);
        let canvas = ContentCanvas::from_layout(&lines, region, mode);
        oracle
            .render(&canvas, &window, font, mode, &contract, &mut oracle_surface)
            .expect("oracle render");
        viewbox
            .render(
                &canvas,
                &window,
                font,
                mode,
                &contract,
                &mut viewbox_surface,
            )
            .expect("viewbox render");
        let ob = oracle_surface.read_back().expect("oracle read_back");
        let vb = viewbox_surface.read_back().expect("viewbox read_back");
        if ob != vb {
            let (w, h) = oracle_surface.size();
            let diff_rows: Vec<u32> = (0..h)
                .filter(|&y| {
                    let r = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                    ob[r.clone()] != vb[r]
                })
                .collect();
            panic!(
                "Yu Gothic UI 実 fixture で viewbox が oracle と diverge（行下端はみ出しインクの切り落とし・D2）: \
                 t={t:.2} visible={visible} 相違行 y={diff_rows:?}"
            );
        }
        if visible > 0 {
            checked_opaque |= opaque_count(&vb) > 0;
        }
        t += 0.02;
    }
    assert!(
        checked_opaque,
        "非退化: content フレームで非透明ピクセルを観測している（vacuous でない）"
    );
}
