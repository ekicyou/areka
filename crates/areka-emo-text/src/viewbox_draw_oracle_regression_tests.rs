use super::ViewboxExecutor;
use super::test_support::{Rig, geo_model};
use crate::canvas::ContentCanvas;
use crate::draw::{DWriteMetrics, DrawExecutor, ResolvedFont};
use crate::layout::{LayoutEngine, WrapPlan};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{TextItem, TextLayerConfig, TextLayerState};
use crate::writing::WritingMode;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

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
    let mut oracle_surface = rig.attach(image, 1.0);
    let mut viewbox_surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let model = geo_model(Some(28));
    let font = ResolvedFont::resolve(&model);
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let config = TextLayerConfig::default();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let metrics = DWriteMetrics::new(&factory, &font, mode, &config).expect("DWriteMetrics");
    let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
    let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");

    // 要件 7.4「参照描画との画素等価比較が両側とも同じ寸法で動く」の確認点はここ。
    // 参照描画（oracle）と実描画（viewbox）は同一寸の面へ描き、read_back を byte 比較する。
    // 面寸 320×122 は本体側バルーンの描画範囲そのもの（`balloons0s.txt` の validrect
    // (36,46)-(356,168)）で、行送り 30px では行 i の下端 = 30i + 28 ゆえ
    // 4 行目が 90..118 ≤ 122 まで収まり、5 行目 120..148 が 122 を超えてあふれる
    // （旧 35px では 3 行目 70..98 までで 4 行目 105..133 があふれていた）。
    assert_eq!(
        oracle_surface.size(),
        viewbox_surface.size(),
        "参照描画と実描画は同じ寸法の面で比較する（要件 7.4）"
    );
    assert_eq!(
        oracle_surface.size(),
        image,
        "面寸は本体側バルーンの描画範囲 320×122 と同値"
    );

    let actor = ActorKey::from("0");
    let mut state = TextLayerState::default();

    // reveal は配送 duration 由来（interval = duration / N）。全 cue at=0.0 ゆえ、
    // Text へ N×REVEAL_INTERVAL を焼き込むと reveal が連続 typewriter 進行する
    // （旧 char_wait=0.05 と機能等価・注入時刻はこの間隔で刻む）。
    const REVEAL_INTERVAL: f64 = 0.05;
    let mk = |cmd: CueCommand| TalkCue {
        at: 0.0,
        actor: actor.clone(),
        duration: match &cmd {
            CueCommand::Text(t) => t.chars().count() as f64 * REVEAL_INTERVAL,
            _ => 0.0,
        },
        command: cmd,
    };
    // example の共有 fixture フル シナリオ（行1/2/3＋あふれ誘発の短行 9 本でスクロール発火）。
    state.apply_cue(&mk(CueCommand::Text("おっはよー！".into())));
    state.apply_cue(&mk(CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&mk(CueCommand::Text("めっちゃええ朝やん！".into())));
    state.apply_cue(&mk(CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&mk(CueCommand::Text("今日もいくでー！".into())));
    for _ in 0..9 {
        state.apply_cue(&mk(CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&mk(CueCommand::Text("ほな".into())));
    }

    let total_glyphs = 6 + 10 + 8 + 9 * 2;
    for step in 0..=(total_glyphs + 2) {
        let t = step as f64 * REVEAL_INTERVAL + 0.001;
        let visible = state.visible_glyphs(&actor, t);
        let items: Vec<TextItem> = state
            .actor_state(&actor)
            .map(|s| s.items().to_vec())
            .unwrap_or_default();
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &region,
            mode,
            font.height,
            &metrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &region, mode);
        let canvas = ContentCanvas::from_layout(&lines, &region, mode);
        oracle
            .render(
                &canvas,
                &window,
                &font,
                mode,
                &contract,
                &mut oracle_surface,
            )
            .expect("oracle render");
        viewbox
            .render(
                &canvas,
                &window,
                &font,
                mode,
                &contract,
                &mut viewbox_surface,
            )
            .expect("viewbox render");
        let ob = oracle_surface.read_back().expect("oracle read_back");
        let vb = viewbox_surface.read_back().expect("viewbox read_back");
        if ob != vb {
            let (w, h) = oracle_surface.size();
            let mut diff_rows: Vec<u32> = Vec::new();
            for y in 0..h {
                let row = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                if ob[row.clone()] != vb[row] {
                    diff_rows.push(y);
                }
            }
            panic!(
                "viewbox が oracle と diverge（viewbox 固有欠陥）: step={step} t={t:.3} visible={visible} \
                 相違行 y={diff_rows:?}（行1セル 0..28・行間 28..30・行2セル 30..58）"
            );
        }
    }
    // 前方進行では全フレーム byte 一致。次に**後方時刻ジャンプ**（example の C8 検分が
    // present_at(earlier t_mid) で行うパターン）を検証する: 大 t（スクロール済み）から
    // 小 t（未スクロール）へ戻したとき、viewbox が un-scroll を正しく扱い oracle と一致するか。
    // viewbox は前方スクロール前提ゆえ、後方でスクロールアウト行の再露出を取りこぼすと diverge。
    let big_t = (total_glyphs as f64 + 5.0) * REVEAL_INTERVAL;
    for &back_t in &[big_t, 0.05, 0.3, 0.1, 0.5] {
        let visible = state.visible_glyphs(&actor, back_t);
        let items: Vec<TextItem> = state
            .actor_state(&actor)
            .map(|s| s.items().to_vec())
            .unwrap_or_default();
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &region,
            mode,
            font.height,
            &metrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &region, mode);
        let canvas = ContentCanvas::from_layout(&lines, &region, mode);
        oracle
            .render(
                &canvas,
                &window,
                &font,
                mode,
                &contract,
                &mut oracle_surface,
            )
            .expect("oracle render(後方)");
        viewbox
            .render(
                &canvas,
                &window,
                &font,
                mode,
                &contract,
                &mut viewbox_surface,
            )
            .expect("viewbox render(後方)");
        let ob = oracle_surface.read_back().expect("oracle read_back(後方)");
        let vb = viewbox_surface
            .read_back()
            .expect("viewbox read_back(後方)");
        if ob != vb {
            let (w, h) = oracle_surface.size();
            let mut diff_rows: Vec<u32> = Vec::new();
            for y in 0..h {
                let row = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                if ob[row.clone()] != vb[row] {
                    diff_rows.push(y);
                }
            }
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
    let mut oracle_surface = rig.attach(image, 1.0);
    let mut viewbox_surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let model = geo_model(Some(28));
    let font = ResolvedFont::resolve(&model);
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let config = TextLayerConfig::default();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let metrics = DWriteMetrics::new(&factory, &font, mode, &config).expect("DWriteMetrics");
    let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
    let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");
    // 単一行・改行なし。visible を 6→2 と縮める 2 フレーム（後方時刻ジャンプの un-reveal 相当）。
    let items: Vec<TextItem> = "おっはよー！"
        .chars()
        .map(|ch| TextItem::Glyph { ch })
        .collect();
    for &visible in &[6usize, 2usize] {
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &region,
            mode,
            font.height,
            &metrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &region, mode);
        let canvas = ContentCanvas::from_layout(&lines, &region, mode);
        oracle
            .render(
                &canvas,
                &window,
                &font,
                mode,
                &contract,
                &mut oracle_surface,
            )
            .expect("oracle render");
        viewbox
            .render(
                &canvas,
                &window,
                &font,
                mode,
                &contract,
                &mut viewbox_surface,
            )
            .expect("viewbox render");
        let ob = oracle_surface.read_back().expect("oracle read_back");
        let vb = viewbox_surface.read_back().expect("viewbox read_back");
        assert_eq!(
            ob, vb,
            "行内縮小 visible={visible} で viewbox が oracle と byte 一致（DD-9・退避インク一掃）"
        );
    }
}
