use super::test_support::{
    RenderPair, Rig, apply_fixture_script, build, diff_rows, geo_model, glyph_items, layout_frame,
    opaque_count, state_frame,
};
use crate::draw::{DWriteMetrics, ResolvedFont};
use crate::layout::WrapPlan;
use crate::region::{ScaleContract, TextRegion};
use crate::state::{TextLayerConfig, TextLayerState};
use crate::writing::WritingMode;
use areka_sakura::contract::ActorKey;

/// D1 診断（実機で行間の文字欠けを観測）: example の共有 fixture（font 28px・行送り
/// pitch 30px＝字の丈 28 ＋ 行間 2・validrect 320×122・**実 DWriteMetrics**・
/// 行1「おっはよー！」行2「めっちゃええ朝やん！」）を
/// oracle（全域再描画）と viewbox（ダーティスクロール）の両方で typewriter 進行させ read_back を
/// byte 比較する。diverge すれば **viewbox 固有の描画欠陥**（行間で確定行 ink をクリアして
/// 再描画しない等）、byte 一致すれば **layout 由来**（両方式に同じ＝emo-text-layer 責務）と切り分ける。
/// live-diff（FixedMetrics＋あいうえお）が見逃した「実 fixture の実文字」条件を実測する。
#[test]
fn diag_line_boundary_dropout_vs_oracle() {
    let mut rig = Rig::new();
    let image = (320u32, 122u32);
    let mut pair = RenderPair::attach(&mut rig, image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let model = geo_model(Some(28));
    let font = ResolvedFont::resolve(&model);
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let config = TextLayerConfig::default();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let metrics = DWriteMetrics::new(&factory, &font, mode, &config).expect("DWriteMetrics");

    // 要件 7.4「参照描画との画素等価比較が両側とも同じ寸法で動く」の確認点はここ。
    // 参照描画（oracle）と実描画（viewbox）は同一寸の面へ描き、read_back を byte 比較する。
    // 面寸 320×122 は本体側バルーンの描画範囲そのもの（`balloons0s.txt` の validrect
    // (36,46)-(356,168)）で、行送り 30px では行 i の下端 = 30i + 28 ゆえ
    // 4 行目が 90..118 ≤ 122 まで収まり、5 行目 120..148 が 122 を超えてあふれる
    // （旧 35px では 3 行目 70..98 までで 4 行目 105..133 があふれていた）。
    assert_eq!(
        pair.oracle_surface.size(),
        pair.viewbox_surface.size(),
        "参照描画と実描画は同じ寸法の面で比較する（要件 7.4）"
    );
    assert_eq!(
        pair.oracle_surface.size(),
        image,
        "面寸は本体側バルーンの描画範囲 320×122 と同値"
    );

    let actor = ActorKey::from("0");
    let mut state = TextLayerState::default();

    // reveal は配送 duration 由来（interval = duration / N）。全 cue at=0.0 ゆえ、
    // Text へ N×REVEAL_INTERVAL を焼き込むと reveal が連続 typewriter 進行する
    // （旧 char_wait=0.05 と機能等価・注入時刻はこの間隔で刻む）。
    // example の共有 fixture フル シナリオ（行1/2/3＋あふれ誘発の短行 9 本でスクロール発火）。
    const REVEAL_INTERVAL: f64 = 0.05;
    apply_fixture_script(&mut state, &actor, [0.0; 4], REVEAL_INTERVAL);

    let total_glyphs = 6 + 10 + 8 + 9 * 2;
    let mut scrolled_frames = 0;
    for step in 0..=(total_glyphs + 2) {
        let t = step as f64 * REVEAL_INTERVAL + 0.001;
        let (visible, _, canvas, window) =
            state_frame(&state, &actor, t, &region, mode, font.height, &metrics);
        let (ob, vb) = pair.render("前方", &canvas, &window, &font, mode, &contract);
        // 空の面どうしの一致で緑にしない（字の見えるフレームは墨がある）。
        assert!(
            visible == 0 || opaque_count(&ob) > 0,
            "前方 step={step} visible={visible}: 見本の面に墨が無い"
        );
        if window.first_visible_line > 0 {
            scrolled_frames += 1;
        }
        if ob != vb {
            let (w, h) = pair.oracle_surface.size();
            let diff_rows = diff_rows(&ob, &vb, w, h);
            panic!(
                "viewbox が oracle と diverge（viewbox 固有欠陥）: step={step} t={t:.3} visible={visible} \
                 相違行 y={diff_rows:?}（行1セル 0..28・行間 28..30・行2セル 30..58）"
            );
        }
    }
    assert!(
        scrolled_frames > 0,
        "前方の進行でスクロールが 1 度も起きていない（名前のスクロールを確かめていない）"
    );
    // 前方進行では全フレーム byte 一致。次に**後方時刻ジャンプ**（example の C8 検分が
    // present_at(earlier t_mid) で行うパターン）を検証する: 大 t（スクロール済み）から
    // 小 t（未スクロール）へ戻したとき、viewbox が un-scroll を正しく扱い oracle と一致するか。
    // viewbox は前方スクロール前提ゆえ、後方でスクロールアウト行の再露出を取りこぼすと diverge。
    let big_t = (total_glyphs as f64 + 5.0) * REVEAL_INTERVAL;
    for &back_t in &[big_t, 0.05, 0.3, 0.1, 0.5] {
        let (visible, _, canvas, window) =
            state_frame(&state, &actor, back_t, &region, mode, font.height, &metrics);
        let (ob, vb) = pair.render("後方", &canvas, &window, &font, mode, &contract);
        assert!(
            opaque_count(&ob) > 0,
            "後方 back_t={back_t:.3} visible={visible}: 見本の面に墨が無い"
        );
        // 最初の大きい時刻だけがスクロール済み、戻した小さい時刻は先頭の行から見える。
        assert_eq!(
            window.first_visible_line > 0,
            back_t == big_t,
            "後方 back_t={back_t:.3}: スクロールの有無が想定と違う {window:?}"
        );
        if ob != vb {
            let (w, h) = pair.oracle_surface.size();
            let diff_rows = diff_rows(&ob, &vb, w, h);
            panic!(
                "後方時刻ジャンプで viewbox が oracle と diverge（un-scroll 欠陥・C8 probing が誘発）: \
                 back_t={back_t:.3} visible={visible} 相違行数={} y={:?}...",
                diff_rows.len(),
                &diff_rows.iter().take(12).collect::<Vec<_>>()
            );
        }
    }
}

/// DD-9: 改行を一切含まない行内縮小（visible=6「おっはよー！」→ visible=2「おっ」）で
/// oracle（全域再描画）と viewbox（ダーティスクロール）の read_back が byte 一致する。
/// `diag_line_boundary_dropout_vs_oracle` の後方ジャンプ検分が捕えた退避インク未クリア欠陥が
/// **改行非依存**（本 spec の遅延化と直交・後方時刻ジャンプ/un-reveal で単一行が縮む任意
/// アクセスで再現）であることを恒久固定する。guard 拡張（viewbox.rs DD-9）で緑になる。
#[test]
fn within_line_shrink_no_newline_stays_byte_equal_to_oracle() {
    let mut rig = Rig::new();
    let image = (320u32, 122u32);
    let mut pair = RenderPair::attach(&mut rig, image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let model = geo_model(Some(28));
    let font = ResolvedFont::resolve(&model);
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let config = TextLayerConfig::default();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let metrics = DWriteMetrics::new(&factory, &font, mode, &config).expect("DWriteMetrics");
    // 単一行・改行なし。visible を 6→2 と縮める 2 フレーム（後方時刻ジャンプの un-reveal 相当）。
    let items = glyph_items("おっはよー！");
    let mut inks = Vec::new();
    for &visible in &[6usize, 2usize] {
        let (_, canvas, window) = layout_frame(
            &items,
            visible,
            &region,
            mode,
            font.height,
            &metrics,
            WrapPlan::CharByChar,
        );
        let (ob, vb) = pair.render("行内縮小", &canvas, &window, &font, mode, &contract);
        assert_eq!(
            ob, vb,
            "行内縮小 visible={visible} で viewbox が oracle と byte 一致（DD-9・退避インク一掃）"
        );
        inks.push(opaque_count(&ob));
    }
    // 空の面どうしの一致で緑にしない・縮んだフレームは墨が減る（消えた 4 字の墨が残っていない）。
    assert!(
        inks[1] > 0 && inks[1] < inks[0],
        "墨の画素の数 visible 6 → 2 が {inks:?}（2 字の墨が在り、6 字より少ないはず）"
    );
}

/// 指摘 31: 字送りのフレームごとに既定の文字色のブラシと描き先の bitmap を作らない。
/// ブラシは台詞のあいだ 1 つ（`request_clear` で捨てて次の台詞で 1 つ作る）、描き先は
/// front/back の 2 枚だけ。見本と画素が一致することも毎フレーム確かめる（振る舞いは変えない）。
#[test]
fn typing_frames_reuse_default_brush_and_target_bitmaps() {
    let mut rig = Rig::new();
    let image = (320u32, 122u32);
    let mut pair = RenderPair::attach(&mut rig, image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let model = geo_model(Some(28));
    let font = ResolvedFont::resolve(&model);
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let frame = |pair: &mut RenderPair, text: &str| {
        let (canvas, window) = build(&glyph_items(text), &region, mode, font.height);
        let (ob, vb) = pair.render(text, &canvas, &window, &font, mode, &contract);
        assert!(opaque_count(&ob) > 0, "{text}: 見本の面に墨が無い");
        assert_eq!(ob, vb, "{text}: 見本と viewbox の画素が一致する");
    };
    for text in ["あ", "あい", "あいう", "あいうえ", "あいうえお"] {
        frame(&mut pair, text);
    }
    assert_eq!(
        pair.viewbox.created,
        (1, 2),
        "5 フレームでブラシは 1 つ・描き先は front/back の 2 枚だけ作る"
    );
    pair.viewbox.request_clear();
    for text in ["か", "かき"] {
        frame(&mut pair, text);
    }
    assert_eq!(
        pair.viewbox.created,
        (2, 2),
        "台詞の切れ目でブラシを捨てて 1 つ作り直し、描き先は作り直さない"
    );
}
