use super::{FixedMetrics, LayoutEngine, LineRect, PositionedLine, WrapPlan};
use crate::region::TextRegion;
use crate::segment::{Segment, SegmentPlan};
use crate::state::TextItem;
use crate::writing::WritingMode;
use super::test_support::{IMAGE, glyphs, inline_positions, model};

// ── Task 4.1: 塊先決による折返し（WrapPlan::Segmented・手組み SegmentPlan 注入） ──
//
// budouy 非依存でゲート③の判断分岐を全網羅するため、plan は手組み（from_segments）で
// 注入する（design「テスト形: 手組み SegmentPlan」）。共通前提: FixedMetrics・font 10 →
// 全角 'あ' の advance 10・pitch 13。閾値は wordwrappoint x（横書き）／y（縦書き）。

/// (start, len) 列から手組み SegmentPlan を作る。
fn plan(segs: &[(usize, usize)]) -> SegmentPlan {
    SegmentPlan::from_segments(
        segs.iter()
            .map(|&(start, len)| Segment { start, len })
            .collect(),
    )
}

/// 各グリフの (行 index, 行内位置, 文字) を配置順に平坦化する（prefix 一致比較用）。
fn flat_glyphs(lines: &[PositionedLine]) -> Vec<(usize, f32, char)> {
    lines
        .iter()
        .enumerate()
        .flat_map(|(li, l)| l.glyphs.iter().map(move |g| (li, g.inline_pos, g.ch)))
        .collect()
}

/// 2.1/2.3: 残り行幅に収まる塊は分割せず現在行へ継続配置する（塊内は追加判定なし）。
/// 行頭の塊 {0,2}（fit）→ 続く塊 {2,2} も残り行幅 30 に収まる → 全 4 グリフが 1 行。
#[test]
fn segmented_fits_places_on_current_line_without_split() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(4);
    let p = plan(&[(0, 2), (2, 2)]);
    let lines = LayoutEngine::layout(
        &items,
        4,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(lines.len(), 1, "両塊とも残り行幅に収まる → 1 行に継続配置");
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0, 30.0]);
}

/// 2.2: 残り行幅に収まらないが行頭幅には収まる塊は、塊の前で行送りして塊全体を次行頭へ
/// 移す（途中分割しない）。塊 {4,2}（seg_sum 20）は残り行幅 10 に入らず行頭幅 50 に入る。
/// CharByChar（5+1 割り）との対比で ON が効いていることを示す。
#[test]
fn segmented_not_fit_breaks_before_whole_segment() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(6);
    let p = plan(&[(0, 4), (4, 2)]);
    let seg_lines = LayoutEngine::layout(
        &items,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(seg_lines.len(), 2);
    assert_eq!(inline_positions(&seg_lines[0]), vec![0.0, 10.0, 20.0, 30.0]);
    assert_eq!(
        inline_positions(&seg_lines[1]),
        vec![0.0, 10.0],
        "塊 {{4,2}} は分割されず塊ごと次行へ"
    );
    // 対比: CharByChar は 5 グリフ目まで行 0（塊を割る割り方）。
    let char_lines = LayoutEngine::layout(
        &items,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(char_lines[0].glyphs.len(), 5);
    assert_ne!(seg_lines, char_lines, "ON（塊維持）が char 割りと異なる");
}

/// 2.2/2.3: 境界値の檻（`<=` vs `<`）。塊の advance 合計がちょうど残り行幅に一致すれば
/// 残留、1 グリフ増えて超えれば塊前で行送り。prefix {0,2}（inline 20・残り行幅 30）に対し、
/// seg_sum 30（={2,3}）は残留・seg_sum 40（={2,4}）は行送り。
#[test]
fn segmented_boundary_exactly_fits_stays_else_breaks() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // ちょうど: seg_sum 30 == cap_rem 30 → 残留（1 行）。
    let items5 = glyphs(5);
    let p_fit = plan(&[(0, 2), (2, 3)]);
    let fit = LayoutEngine::layout(
        &items5,
        5,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p_fit),
    );
    assert_eq!(fit.len(), 1, "seg_sum == cap_rem は残留（<= 判定の境界檻）");
    assert_eq!(inline_positions(&fit[0]), vec![0.0, 10.0, 20.0, 30.0, 40.0]);
    // 1 グリフ増: seg_sum 40 > cap_rem 30 → 塊前で行送り。
    let items6 = glyphs(6);
    let p_over = plan(&[(0, 2), (2, 4)]);
    let over = LayoutEngine::layout(
        &items6,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p_over),
    );
    assert_eq!(over.len(), 2);
    assert_eq!(over[0].glyphs.len(), 2, "行 0 は prefix 塊のみ");
    assert_eq!(over[1].glyphs.len(), 4, "塊 {{2,4}} は分割されず次行へ");
}

/// 2.1/2.3（counter）: 先決済み塊の内部では追加判定を通さず配置する（残グリフ数カウンタ）。
/// threshold 30 で塊 {2,3}（seg_sum 30）が break-before で行 1 頭へ移り、3 グリフが分割
/// されずに行 1 へ連続配置される。CharByChar は g2 まで行 0（塊を割る）——counter による
/// 塊維持が char 割りと異なることを示す。
#[test]
fn segmented_predecided_segment_is_not_split_inside() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(30), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(5);
    let p = plan(&[(0, 2), (2, 3)]);
    let seg = LayoutEngine::layout(
        &items,
        5,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(seg.len(), 2);
    assert_eq!(seg[0].glyphs.len(), 2);
    assert_eq!(
        inline_positions(&seg[1]),
        vec![0.0, 10.0, 20.0],
        "塊内は追加判定なしで連続配置（3 グリフが行 1 に一体で載る）"
    );
    // 対比: char 規則は g2 まで行 0（3 グリフ）→ 塊が割れる割り方。
    let ch = LayoutEngine::layout(
        &items,
        5,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(ch[0].glyphs.len(), 3);
    assert_ne!(seg, ch, "counter による塊維持が char 割りと異なる");
}

/// 4.2: plan に被覆されないグリフ（不整合・長大塊継続）は既存の文字単位規則で配置される
/// （優しい縮退）。塊 {0,2} のみ被覆・g2..g5 は非被覆 → 出力は純 CharByChar と完全一致。
#[test]
fn plan_non_covered_glyphs_fall_back_to_char_rule() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(6);
    let p = plan(&[(0, 2)]); // glyph 2..5 は非被覆
    let seg = LayoutEngine::layout(
        &items,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    let ch = LayoutEngine::layout(
        &items,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(
        seg, ch,
        "非被覆グリフは既存文字単位規則で配置（plan 不整合の優しい縮退）"
    );
    assert_eq!(seg[0].glyphs.len(), 5, "char 割り（5+1）どおり");
    assert_eq!(seg[1].glyphs.len(), 1);
}

/// 7.1/8.3（INV-1/INV-2）: 塊先決は visible_count に依存しない。同一 items+plan で visible を
/// 段階的に増やすと、各段階の配置は全量出力の先頭 v グリフ（行所属・行内位置とも）に一致する
/// （リフロー跳び不発生）。核心: g4（塊 {4,2} 先頭）は g5 が不可視でも行 1 へ落ちる
/// （seg_sum が全文 items から算出されるため——可視部分列で計算する実装なら行 0 に留まり失敗）。
#[test]
fn predecision_is_independent_of_visible_count() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(6);
    let p = plan(&[(0, 4), (4, 2)]);
    let full = LayoutEngine::layout(
        &items,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    let full_flat = flat_glyphs(&full);
    for v in 0..=6 {
        let partial = LayoutEngine::layout(
            &items,
            v,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(
            flat_glyphs(&partial).as_slice(),
            &full_flat[..v],
            "visible {v}: 配置が全量出力の prefix と不一致（リフロー跳び）"
        );
    }
    // 核心: visible 5（g5 不可視）でも g4 は行 1（塊先決は全文由来・INV-1）。
    let v5 = LayoutEngine::layout(
        &items,
        5,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(v5.len(), 2);
    assert_eq!(v5[1].glyphs.len(), 1, "行 1 は g4 のみ（g5 未リビール）");
    assert_eq!(v5[1].glyphs[0].inline_pos, 0.0);
}

/// 6.1/6.2: 縦書き（vertical_rl/vertical_lr）でも軸読み替えのみで同一の塊先決式が成立する。
/// 同一 items+plan で塊 {4,2} が 2 列目へ移り（4+2）、行内軸の先決（塊割れ点・行内位置）が
/// rl/lr で完全一致する（新規 mode 分岐なし・単一読み替え規則）。
#[test]
fn segmented_same_rule_in_vertical_modes() {
    let m = model((None, None), (None, Some(50)));
    let items = glyphs(6);
    let p = plan(&[(0, 4), (4, 2)]);
    let mut inline_by_mode: Vec<Vec<Vec<f32>>> = Vec::new();
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let region = TextRegion::resolve(&m, IMAGE, mode);
        let lines = LayoutEngine::layout(
            &items,
            6,
            &region,
            mode,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(lines.len(), 2, "{mode:?}: 塊 {{4,2}} が 2 列目へ");
        assert_eq!(lines[0].glyphs.len(), 4);
        assert_eq!(lines[1].glyphs.len(), 2, "{mode:?}: 塊は分割されない");
        inline_by_mode.push(lines.iter().map(inline_positions).collect());
    }
    assert_eq!(
        inline_by_mode[0], inline_by_mode[1],
        "縦書き 2 方向で行内軸の塊先決（割れ点・行内位置）が不一致"
    );
}

/// 4.1/4.3/8.3: OFF 経路（`WrapPlan::CharByChar`）の非回帰アンカー。既定の折返し
/// モード（文字単位）は本機能導入前の layout と byte 等価であり、`SegmentPlan` を
/// 一切参照しない（境界値の算出自体が起きない・R4.2）。閾値 50・font 10 の 6 グリフは
/// char 割り（5+1）で、行内位置・行矩形まで確定値へ pin する（この確定出力は
/// `horizontal_wraps_before_glyph_exceeding_threshold` と一致——OFF 出力が全 layout
/// 呼出で不変であることの明示アンカー）。
#[test]
fn char_by_char_is_off_path_non_regression_anchor() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(6);
    let lines = LayoutEngine::layout(
        &items,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "char 割り（5+1）どおり");
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0, 30.0, 40.0]);
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
    assert_eq!(
        lines[0].rect,
        LineRect {
            left: 0.0,
            top: 0.0,
            right: 50.0,
            bottom: 10.0
        }
    );
    assert_eq!(
        lines[1].rect,
        LineRect {
            left: 0.0,
            top: 13.0,
            right: 10.0,
            bottom: 23.0
        }
    );
}

// ── Task 4.2: 長大塊の文字単位縮退（WrapPlan::Segmented・design System Flows H-No→D→C） ──
//
// 縮退は `seg_sum > cap_full`（行頭からでも 1 行に収まらない塊）に限って発火し、当該塊のみ
// 既存の文字単位規則へ委譲する（design「縮退は塊に閉じる」3.3）。共通前提は 4.1 と同じ
// FixedMetrics・font 10（全角 'あ' advance 10）。

/// 3.1: 縮退は `seg_sum > cap_full` の塊に限る。長大塊 {0,5}（seg_sum 50 > cap_full 30）は
/// 文字単位で複数行へ割られ（塊維持でも欠落でもない）、直後の収まる塊 {5,2} は縮退せず塊ごと
/// 1 行へ載る。threshold 30・font 10。
#[test]
fn segmented_degrades_only_when_exceeding_cap_full() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(30), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(7);
    let p = plan(&[(0, 5), (5, 2)]);
    let lines = LayoutEngine::layout(
        &items,
        7,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    // 長大塊 {0,5} は char 規則で 3+2 に割れ（1 行 3 グリフ＝閾値 30／30）・全 5 グリフ配置。
    assert_eq!(lines.len(), 3);
    assert_eq!(
        inline_positions(&lines[0]),
        vec![0.0, 10.0, 20.0],
        "長大塊は char 粒度で複数行へ（塊ごと 1 行に潰さない）"
    );
    assert_eq!(inline_positions(&lines[1]), vec![0.0, 10.0]);
    // 直後の塊 {5,2}（seg_sum 20 ≤ cap_full 30）は縮退せず塊ごと 1 行へ（通常判定）。
    assert_eq!(
        inline_positions(&lines[2]),
        vec![0.0, 10.0],
        "収まる塊は縮退せず塊単位で維持（縮退は > cap_full に限る）"
    );
    // 全 7 グリフが過不足なく配置される（欠落なし）。
    assert_eq!(flat_glyphs(&lines).len(), 7);
}

/// 3.2: 縮退中も行頭 1 グリフは閾値超過でも配置し（無限折返し回避）、呼出は必ず停止して
/// 全グリフを配置する。極小閾値 3・font 10（'あ' advance 10 > threshold）で 1 グリフ/行。
/// char モードの `single_glyph_exceeding_threshold_is_placed_per_line` を Segmented で鏡写す。
#[test]
fn segmented_degrade_places_line_head_glyph_and_terminates() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(3), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(3);
    // 単一長大塊 {0,3}（seg_sum 30 > cap_full 3）→ 当該塊のみ char 規則へ縮退。
    let p = plan(&[(0, 3)]);
    let lines = LayoutEngine::layout(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(
        lines.len(),
        3,
        "閾値超過グリフは 1 行 1 グリフで前進（無限ループしない・停止する）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
    assert_eq!(inline_positions(&lines[2]), vec![0.0]);
    // 全グリフが配置される（行頭 1 グリフ配置規則で 1 個も落ちない）。
    assert_eq!(flat_glyphs(&lines).len(), 3);
}

/// 3.3: 縮退は当該塊に閉じ、直後の塊で通常の塊単位判定が再開する。長大塊 {0,4}（seg_sum 40 >
/// cap_full 30）は char 割り、続く塊 {4,3}（seg_sum 30 == cap_full 30）は縮退を引き継がず塊前
/// 行送りで塊ごと次行へ載る。char モードなら g4 は行 1 に残る（塊維持でない）——差分が再開の証左。
#[test]
fn segmented_degrade_scoped_to_segment_resumes_next() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(30), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyphs(7);
    let p = plan(&[(0, 4), (4, 3)]);
    let seg = LayoutEngine::layout(
        &items,
        7,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(seg.len(), 3);
    // 長大塊 {0,4} は char 縮退（3+1）。
    assert_eq!(inline_positions(&seg[0]), vec![0.0, 10.0, 20.0]);
    assert_eq!(inline_positions(&seg[1]), vec![0.0], "縮退塊の残り g3");
    // 後続塊 {4,3} は通常判定を再開＝塊ごと行 2 へ（塊単位 break-before・塊維持）。
    assert_eq!(
        inline_positions(&seg[2]),
        vec![0.0, 10.0, 20.0],
        "後続塊は塊単位で維持（縮退が漏れ出していない）"
    );
    // 縮退が漏れれば g4 は行 1（inline 10）に char 継続で載る——そうでないことを対比で固定。
    let ch = LayoutEngine::layout(
        &items,
        7,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(ch[1].glyphs.len(), 3, "char モードは行 1 が [g3,g4,g5]");
    assert_ne!(seg, ch, "後続塊で塊単位判定が再開＝char 全割りと異なる");
}

/// 8.3: 極端に長い塊でも全グリフが配置され表示が破綻しない（panic せず停止し無損失）。
/// 50 グリフ単一塊・threshold 30 → char 縮退で複数行に割れ、全グリフがちょうど一度ずつ現れる。
#[test]
fn segmented_extremely_long_segment_places_all_glyphs() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(30), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let n = 50;
    let items = glyphs(n);
    let p = plan(&[(0, n)]);
    let lines = LayoutEngine::layout(
        &items,
        n,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    // 全 50 グリフがちょうど一度ずつ配置される（欠落・重複なし）。
    assert_eq!(flat_glyphs(&lines).len(), n, "全グリフが配置される（無損失）");
    // 各行は行頭 1 グリフ以外は閾値内（3 グリフ/行＝30／30）——はみ出しが構造的に起きない。
    for line in &lines {
        assert!(
            line.glyphs.len() <= 3,
            "1 行に閾値超のグリフが積まれている（縮退が効いていない）: {}",
            line.glyphs.len()
        );
        assert!(!line.glyphs.is_empty(), "空行が生じている");
    }
    // 全グリフが同一文字 'あ'（内容が壊れていない）。
    assert!(lines.iter().all(|l| l.glyphs.iter().all(|g| g.ch == 'あ')));
}

// ── Task 4.4: 保留改行との整合とリフロー跳び不発生（WrapPlan::Segmented） ──
//
// ゲート順序（①可視打切り→②保留フラッシュ→③折返し判定→④配置）は 4.1 で確立済み。
// ここではその順序契約の帰結を檻化する: 保留改行の実体化直後は行頭（inline_pos ==
// inline_start）ゆえ塊先決が「塊先頭かつ残り行幅最大（cap_rem == cap_full）」で走り
// （design System Flows「保留フラッシュとの順序」5.3）、deferred newline の意味論
// （遅延・累算・蒸発）は ON でも一切変わらず（5.1/5.2）、typewriter リビール進行の
// 全段階で配置済みグリフの行が動かない（INV-2・7.2/7.3）。共通前提は 4.1/4.2 と同じ
// FixedMetrics・font 10（全角 'あ' advance 10・pitch 13）。

/// 5.3: 保留改行の実体化直後の行頭で塊先決が走る。`[塊A, \n, 塊B, 塊C]`（run2 = 塊B+塊C）で、
/// 保留改行が run2 を 2 行目行頭へ送り、そこで塊 C が塊ごと 3 行目へワードラップされる
/// （フラッシュ後の行で塊単位判定が効いている証左）。block 前進は `pitch × Σratio` で OFF と
/// 同一（char 経路の 2 行目 top と一致）。CharByChar は run2 を char 割りして塊 C を割る。
#[test]
fn segmented_predecision_runs_at_line_head_after_pending_flush() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 塊A(2) / \n / 塊B(3) 塊C(3)。glyph 通し番号は LineBreak を数えない → 塊B は 2、塊C は 5。
    let mut items = glyphs(2);
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyphs(6));
    let p = plan(&[(0, 2), (2, 3), (5, 3)]);
    let seg = LayoutEngine::layout(
        &items,
        8,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(seg.len(), 3, "塊A / 塊B / 塊C が 3 行に分かれる");
    // 行 0: 塊A（保留改行の前）。
    assert_eq!(inline_positions(&seg[0]), vec![0.0, 10.0]);
    assert_eq!(seg[0].rect.top, 0.0);
    // 行 1: 塊B が実体化後の行頭に塊ごと載る（塊先決が cap_rem == cap_full で走る）。
    assert_eq!(
        inline_positions(&seg[1]),
        vec![0.0, 10.0, 20.0],
        "塊B は実体化後の行頭に塊ごと配置（塊先決が行頭で先決）"
    );
    assert_eq!(
        seg[1].rect.top, 13.0,
        "block 前進 = pitch(13) × Σratio(1.0)（保留改行の送りは OFF と同一）"
    );
    // 行 2: 塊C は残り行幅（20）に収まらず塊ごと次行へ（フラッシュ後の行で塊単位判定が再開）。
    assert_eq!(
        inline_positions(&seg[2]),
        vec![0.0, 10.0, 20.0],
        "塊C は分割されず塊ごと 3 行目へ（ワードラップが 2 行目以降でも効く）"
    );
    // 対比: CharByChar は同一入力で run2 を char 割り（塊C を割る）＝実体化後の送りは同じでも
    // 折返し粒度が異なる。2 行目 top は両経路で 13（block 前進 = pitch × Σratio は不変）。
    let ch = LayoutEngine::layout(
        &items,
        8,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(
        ch[1].rect.top, 13.0,
        "OFF でも実体化後の block 前進は pitch × Σratio（deferred newline の送りは分岐不変）"
    );
    assert_eq!(ch[1].glyphs.len(), 5, "OFF は run2 を char 割り（5+…）");
    assert_ne!(
        seg, ch,
        "ON はフラッシュ後の行で塊単位ワードラップ＝char 割りと異なる"
    );
}

/// 5.1/5.2: deferred newline の意味論（遅延・累算・蒸発）は ON でも不変。ワードラップが
/// 発火しない（全塊が収まる）入力では Segmented 出力は CharByChar 出力と完全一致する
/// ——segmentation は改行意味論を変えず、行内の折返し粒度だけを担う。
/// (a) 末尾保留改行の蒸発（空行なし）・(b) 連続 `\n\n` の単一累算フラッシュ。
#[test]
fn deferred_newline_semantics_unchanged_under_segmented() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // (a) 末尾保留改行の蒸発: `[塊A(2), \n]` → 1 行・末尾改行は保留のまま蒸発。
    let mut trailing = glyphs(2);
    trailing.push(TextItem::LineBreak { ratio: 1.0 });
    let p_a = plan(&[(0, 2)]);
    let seg_a = LayoutEngine::layout(
        &trailing,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p_a),
    );
    let ch_a = LayoutEngine::layout(
        &trailing,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(seg_a.len(), 1, "ON でも末尾保留改行は蒸発（空行なし）");
    assert_eq!(
        seg_a, ch_a,
        "ワードラップ非発火では ON 出力 = OFF 出力（改行意味論は不変）"
    );

    // (b) 連続 `\n\n(0.5)` の単一累算フラッシュ: `[a, \n, \n(0.5), b, c]` → 2 行・Σratio 1.5。
    // run1="a"(glyph0)・run2="bc"(glyph1,2)。塊は全て収まる → ON = OFF。
    let acc = [
        TextItem::Glyph { ch: 'a' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'b' },
        TextItem::Glyph { ch: 'c' },
    ];
    let p_b = plan(&[(0, 1), (1, 2)]);
    let seg_b = LayoutEngine::layout(
        &acc,
        3,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p_b),
    );
    let ch_b = LayoutEngine::layout(
        &acc,
        3,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(seg_b.len(), 2, "ON でも連続改行は単一累算＝中間空行なし");
    let tops: Vec<f32> = seg_b.iter().map(|l| l.rect.top).collect();
    assert_eq!(tops, vec![0.0, 22.5], "行間 = pitch(15) × Σratio(1.5)（ON でも不変）");
    assert_eq!(
        seg_b, ch_b,
        "ワードラップ非発火では連続改行の累算意味論が ON = OFF"
    );
}

/// 7.2/7.3（INV-2 の核心）: 保留改行 + ワードラップが共存する入力で、可視グリフ数を 0 から
/// 段階的に増やしても、各段階の配置は全量出力の先頭 v グリフ（行所属・行内位置とも）に
/// 常に一致する（配置済みグリフの行が後から動かない＝リフロー跳び不発生）。
/// `[塊A(2), \n, 塊B(3), 塊C(3)]`・threshold 50 で塊C は残り行幅に入らず塊ごと 3 行目へ。
/// 核心: visible 6（塊C 先頭 g5 のみ可視・g6/g7 不可視）でも g5 は 3 行目行頭に居る
/// ——seg_sum が全文 plan から算出されるため。可視部分列で seg_sum を計算する実装なら
/// g5 は 2 行目末（inline 30）に留まり、後で g6/g7 の出現で 3 行目へ跳んで失敗する。
#[test]
fn segmented_prefix_stable_across_pending_flush_and_wrap() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let mut items = glyphs(2);
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyphs(6));
    let p = plan(&[(0, 2), (2, 3), (5, 3)]);
    let full = LayoutEngine::layout(
        &items,
        8,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    let full_flat = flat_glyphs(&full);
    assert_eq!(full_flat.len(), 8);
    for v in 0..=8 {
        let partial = LayoutEngine::layout(
            &items,
            v,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(
            flat_glyphs(&partial).as_slice(),
            &full_flat[..v],
            "visible {v}: 配置が全量出力の prefix と不一致（リフロー跳び発生）"
        );
    }
    // 核心の明示: visible 6（g5 のみ可視）で g5 は 3 行目行頭（塊C 先決は全文由来・INV-1/INV-2）。
    let v6 = LayoutEngine::layout(
        &items,
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    assert_eq!(v6.len(), 3, "行 0=塊A・行 1=塊B・行 2=塊C 先頭");
    assert_eq!(v6[2].glyphs.len(), 1, "行 2 は g5 のみ（g6/g7 未リビール）");
    assert_eq!(
        v6[2].glyphs[0].inline_pos, 0.0,
        "g5 は 3 行目行頭に先決済み（可視非依存＝リフロー跳びなし）"
    );
}

/// 6.1/6.2 × 7.2/7.3: 縦書き（vertical_rl）でも保留改行 + ワードラップ共存下で prefix 安定性が
/// 成立する（軸読み替えのみ・新規 mode 分岐なし）。横書きと同一 items+plan を vertical_rl で
/// 段階リビールし、各段階の配置（列所属・行内軸位置）が全量出力の prefix に一致する。
#[test]
fn segmented_prefix_stable_in_vertical_mode() {
    let region = TextRegion::resolve(
        &model((None, None), (None, Some(50))),
        IMAGE,
        WritingMode::VerticalRl,
    );
    let mut items = glyphs(2);
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyphs(6));
    let p = plan(&[(0, 2), (2, 3), (5, 3)]);
    let full = LayoutEngine::layout(
        &items,
        8,
        &region,
        WritingMode::VerticalRl,
        10.0,
        &FixedMetrics,
        WrapPlan::Segmented(&p),
    );
    let full_flat = flat_glyphs(&full);
    assert_eq!(full_flat.len(), 8);
    assert_eq!(full.len(), 3, "縦書きでも 塊A / 塊B / 塊C の 3 列");
    for v in 0..=8 {
        let partial = LayoutEngine::layout(
            &items,
            v,
            &region,
            WritingMode::VerticalRl,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(
            flat_glyphs(&partial).as_slice(),
            &full_flat[..v],
            "vertical_rl visible {v}: 配置が全量出力の prefix と不一致（リフロー跳び）"
        );
    }
}
