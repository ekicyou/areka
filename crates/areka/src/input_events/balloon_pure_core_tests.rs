use super::*;
use super::test_support::row;

// -------------------------------------------------------------------------
// hit_choice_row 純関数檻（task 3.1・design「純関数判定核」／Testing Strategy item 1/4）
//
// 上流実型 `ChoiceHitRow`／`HitRectPx`（全 pub フィールド）で fixture を組み、
// 包含／境界（半開）／行外／空／複数行／病的重なりを GPU・実窓不要で決定的に判定する
// （R1.1/1.5/2.3/5.2）。座標を**無変換**で照合する不変条件（矩形側が実適用 k で ×k 済み
// ゆえ正しい・÷k 追加は二重縮約）は直後の DD-11 檻（R3.7）が k=2.0 で固定する。
// -------------------------------------------------------------------------

use areka_emo_text::actor::HitRectPx;

/// 内側包含（R1.1）: 矩形の内部点はその行 index を返す。
#[test]
fn hit_inside_rect_returns_index() {
    let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];
    assert_eq!(
        hit_choice_row(&rows, 30.0, 30.0),
        Some(0),
        "矩形内部の点はヒット index を返す"
    );
}

/// 空 rows（R2.3）: 判定対象が無ければ常に None。
#[test]
fn hit_empty_rows_returns_none() {
    let rows: [ChoiceHitRow; 0] = [];
    assert_eq!(hit_choice_row(&rows, 30.0, 30.0), None, "空 rows は None");
}

/// 行外（R2.3・非ヒット→None）: 矩形外の点は None。四方向すべて外側を確認。
#[test]
fn hit_outside_rect_returns_none() {
    let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];
    assert_eq!(hit_choice_row(&rows, 5.0, 30.0), None, "左外");
    assert_eq!(hit_choice_row(&rows, 60.0, 30.0), None, "右外");
    assert_eq!(hit_choice_row(&rows, 30.0, 10.0), None, "上外");
    assert_eq!(hit_choice_row(&rows, 30.0, 50.0), None, "下外");
}

/// 半開区間の境界（design Testing Strategy item 1）: `left`/`top` 辺は包含・
/// `right`/`bottom` 辺は非包含（whole-pixel 行矩形と整合）。
#[test]
fn hit_half_open_boundary_edges() {
    let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];

    // left/top 辺は包含。
    assert_eq!(hit_choice_row(&rows, 10.0, 30.0), Some(0), "left 辺は包含");
    assert_eq!(hit_choice_row(&rows, 30.0, 20.0), Some(0), "top 辺は包含");
    assert_eq!(hit_choice_row(&rows, 10.0, 20.0), Some(0), "左上角は包含");

    // right/bottom 辺は非包含。
    assert_eq!(hit_choice_row(&rows, 50.0, 30.0), None, "right 辺は非包含");
    assert_eq!(hit_choice_row(&rows, 30.0, 40.0), None, "bottom 辺は非包含");
    assert_eq!(hit_choice_row(&rows, 50.0, 40.0), None, "右下角は非包含");
}

/// 複数の非重複行のうち正しい行（design Testing Strategy item 1）: 点を含む行の
/// index が返る。各行が独立に判定される。
#[test]
fn hit_multiple_non_overlapping_rows_returns_correct_index() {
    let rows = [
        row(0, 0.0, 0.0, 100.0, 20.0),   // index 0
        row(1, 0.0, 20.0, 100.0, 40.0),  // index 1
        row(2, 0.0, 40.0, 100.0, 60.0),  // index 2
    ];
    assert_eq!(hit_choice_row(&rows, 50.0, 10.0), Some(0), "1 行目内");
    assert_eq!(hit_choice_row(&rows, 50.0, 30.0), Some(1), "2 行目内");
    assert_eq!(hit_choice_row(&rows, 50.0, 50.0), Some(2), "3 行目内");
    assert_eq!(hit_choice_row(&rows, 50.0, 70.0), None, "全行外");
}

/// 病的重なり→最終一致（R1.5・DD-CI-5）: 点を含む行が複数あっても、逆順走査の最初の
/// 一致＝スライス**最終** index を決定的に返す（後定義が手前・画家のアルゴリズム）。
/// 期待 index を各行で一意にするため、重なり方を非対称にして「最初一致」なら別 index に
/// なる配置を用いる（最初一致=1／最終一致=2 を弁別）。
#[test]
fn hit_pathological_overlap_returns_last_match_deterministically() {
    // 3 行が (50, 30) を共通に含む。スライス順（index 昇順）で最後の index 2 が返るべき。
    let rows = [
        row(0, 0.0, 0.0, 100.0, 100.0),  // index 0: 点を含む
        row(1, 40.0, 20.0, 60.0, 40.0),  // index 1: 点を含む
        row(2, 45.0, 25.0, 80.0, 60.0),  // index 2: 点を含む（最後定義＝手前）
    ];
    assert_eq!(
        hit_choice_row(&rows, 50.0, 30.0),
        Some(2),
        "重なり時はスライス最終一致 index（逆順走査の最初一致）"
    );

    // 2 行重なりでも最終一致（index 1）が返る——最初一致 index 0 と弁別。
    let two = [
        row(0, 0.0, 0.0, 100.0, 100.0), // index 0: 点を含む
        row(1, 10.0, 10.0, 90.0, 90.0), // index 1: 点を含む（最後定義）
    ];
    assert_eq!(
        hit_choice_row(&two, 50.0, 50.0),
        Some(1),
        "2 行重なりでも最終一致 index を返す（最初一致 0 ではない）"
    );
}

// -------------------------------------------------------------------------
// バルーン逆向き整合の不変条件檻（DD-11・R3.7／R5.6/5.7/6.4）
//
// 旧「座標素通し（k=1.0）」fixture の**強化版**（新規並置ではなく理由の是正＋条件の拡張）。
// 旧版は「k=1.0 だから素通し」を理由とし、行矩形を実 k で持ち上げた条件を持たなかった。
// 本檻は上流の実写像 `to_window_physical` に **k=2.0** を実供給して行矩形を窓物理 px へ
// 持ち上げ、そこへ (a) **無変換の** client 物理 px 点が正しくヒットすること、(b) 同じ点を
// ÷k してしまうと外れること（＝二重縮約の退行検出）を `click_selection` で固定する。
//
// 成立根拠は「矩形側が既に ×k 済み」——k=1.0 であることではない。シェル窓は逆向き（矩形は
// 作者定義サーフェス px のまま・点を ÷k）であり、両者は等価に正しい。バルーン経路へ ÷k を
// 足すと矩形 ×k と点 ÷k の二重縮約になり正常動作を壊す（R6.4 が明文で禁じる）。
//
// f32 誤差との無関係化（design Risks）: `to_window_physical` は f32 積を含み 1px の余地が
// あるため、檻の点は矩形境界から **8px 以上**（要件の 2px 以上を満たす）内側／外側に置く。
// -------------------------------------------------------------------------

use areka_emo_text::choice::{CanvasHitRow, to_window_physical};
use areka_emo_text::layout::LineRect;
use areka_emo_text::region::{ScaleContract, TextRegion};
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

/// 檻に用いる実適用スケール（k≠1.0——DPI追従下で実供給される値の代表）。
const CAGE_K: f32 = 2.0;

/// validrect 原点 (36, 46) の `TextRegion`（choice.rs 檻と同型の最小構築・画像原寸 400×224）。
fn cage_region() -> TextRegion {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(46), Some(168), Some(36), Some(356)),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    TextRegion::resolve(&model, (400, 224), WritingMode::HorizontalTb)
}

/// canvas-local 行矩形を **上流の実写像 `to_window_physical`** で k=2.0 の窓物理 px へ
/// 持ち上げた 2 行（ordinal 0＝上帯・1＝下帯・committed=0）。
///
/// 期待値（横書き・`行内 x=(36+inline)×2`／`ブロック y=(46+block)×2+0`）:
/// - 行 0: canvas (10, 0, 30, 10) → 窓物理 `[92, 132) × [92, 112)`
/// - 行 1: canvas (10, 10, 30, 20) → 窓物理 `[92, 132) × [112, 132)`
fn rows_lifted_by_real_k() -> Vec<ChoiceHitRow> {
    let region = cage_region();
    let contract = ScaleContract::new(CAGE_K, None);
    [(0usize, 0.0_f32, 10.0_f32), (1, 10.0, 20.0)]
        .into_iter()
        .map(|(ordinal, top, bottom)| {
            let canvas = CanvasHitRow {
                ordinal,
                rect: LineRect {
                    left: 10.0,
                    top,
                    right: 30.0,
                    bottom,
                },
            };
            ChoiceHitRow {
                ordinal,
                id: format!("q{ordinal}"),
                label: format!("label{ordinal}"),
                references: Vec::new(),
                rect: to_window_physical(
                    &canvas,
                    &region,
                    WritingMode::HorizontalTb,
                    0,
                    &contract,
                ),
            }
        })
        .collect()
}

/// 持ち上げ結果の固定（トートロジー回避の土台）: `to_window_physical` が k=2.0 で実際に
/// 生む窓物理 px 矩形を**ハードコード期待値**で押さえる。以降 2 本の檻の座標判断は、
/// この確定した矩形に対して行われる。
#[test]
fn cage_rows_are_lifted_to_window_physical_px_by_real_k() {
    let rows = rows_lifted_by_real_k();
    assert_eq!(
        rows[0].rect,
        HitRectPx {
            left: 92.0,
            top: 92.0,
            right: 132.0,
            bottom: 112.0
        },
        "行 0 は (36+10..30)×2 / (46+0..10)×2 の窓物理 px へ持ち上げられる"
    );
    assert_eq!(
        rows[1].rect,
        HitRectPx {
            left: 92.0,
            top: 112.0,
            right: 132.0,
            bottom: 132.0
        },
        "行 1 は (46+10..20)×2 のブロック帯へ持ち上げられる"
    );
}

/// 檻 (a)（R3.7）: k=2.0 で持ち上げた行矩形に対し、**無変換の** client 物理 px 点
/// (110, 120) が正しく行 1 へヒットし `ChoiceSelection` を構成する。
///
/// 点は行 1 の矩形 `[92, 132) × [112, 132)` の内側で、最寄り境界（top=112）から 8px 離れて
/// おり f32 誤差 1px と無関係に成立する。**成立根拠は矩形が ×k 済みであること**——k=1.0
/// ではないことを直上の持ち上げ檻が示している。本経路へ ÷k を追加すれば本檻は落ちる。
#[test]
fn click_untransformed_point_hits_row_lifted_by_real_k() {
    let rows = rows_lifted_by_real_k();
    // 無変換の client 物理 px 点（÷k も ×k もしない）。
    let (x, y) = (110.0_f32, 120.0_f32);

    let sel = click_selection(true, &rows, x, y, 7)
        .expect("×k 済み行矩形へ無変換の client 物理 px 点は正しくヒットする（R3.7）");
    assert_eq!(
        sel.id, "q1",
        "無変換点 (110,120) は k=2.0 で持ち上げた行 1（y∈[112,132)）へ当たる"
    );
    assert_eq!(sel.scope, 7, "scope は引数由来（判定空間とは独立）");
}

/// 檻 (b)（R3.7・二重縮約の退行検出）: 檻 (a) と**同一の行矩形・同一の点**に対し、点を
/// ÷k してしまうと `(55, 60)` となり全行の外（最寄り境界 left=92 から 37px・top=92 から
/// 32px 外）＝ヒットしない。
///
/// (a) と (b) は行矩形も k も同一で、**÷k の有無だけ**で結果が割れる。ゆえに「バルーン経路
/// へ ÷k を足す」改変（＝矩形 ×k と点 ÷k の二重縮約）は (a) を落として必ず検出される。
#[test]
fn click_double_reduced_point_misses_row_lifted_by_real_k() {
    let rows = rows_lifted_by_real_k();
    let (x, y) = (110.0_f32, 120.0_f32);

    // 誤って ÷k した座標（シェル経路の規約をバルーンへ一般化した場合の値）。
    let (rx, ry) = (x / CAGE_K, y / CAGE_K);
    assert_eq!((rx, ry), (55.0, 60.0), "÷k 後の座標（期待値ハードコード）");
    assert_eq!(
        click_selection(true, &rows, rx, ry, 7),
        None,
        "÷k した点は ×k 済み行矩形の外＝二重縮約は必ず外れる（R3.7 退行検出）"
    );

    // 同じ入力で無変換なら当たる——÷k の有無だけで結果が割れることをこの檻の中でも示す。
    assert!(
        click_selection(true, &rows, x, y, 7).is_some(),
        "無変換なら当たる（÷k の有無だけが結果を分ける・非空虚性）"
    );
}

// -------------------------------------------------------------------------
// hover_action 純関数檻（task 3.2・design「純関数判定核」／Observable 全分岐）
//
// active・hit_ordinal・last_injected の 3 入力から hover 遷移を決める純関数の
// 全分岐（非表示時無処理／消滅時自前整合／同値維持／新規注入／解除注入）を
// World・runtime 不要で決定的に網羅する（R1.2/1.3/1.4/3.4）。
// -------------------------------------------------------------------------

/// 非表示時無処理（R1.4）: `active == false` かつ `last_injected == None` は
/// 何もしない。hit_ordinal（Some/None いずれも）は無視される。
#[test]
fn hover_inactive_no_prior_injection_is_noop() {
    assert_eq!(
        hover_action(false, None, None),
        HoverAction::NoopInactive,
        "非表示・未注入・hit なしは NoopInactive"
    );
    assert_eq!(
        hover_action(false, Some(3), None),
        HoverAction::NoopInactive,
        "非表示・未注入は hit があっても NoopInactive（hit_ordinal 無視・R1.4）"
    );
}

/// 消滅時自前整合（R3.4）: `active == false` かつ `last_injected == Some(k)` は
/// 自前状態を None 整合する `ResetOwnState`（inject はしない＝上流原子性が正本）。
/// hit_ordinal（Some/None いずれも）は無視される。
#[test]
fn hover_inactive_with_prior_injection_resets_own_state() {
    assert_eq!(
        hover_action(false, None, Some(2)),
        HoverAction::ResetOwnState,
        "非表示・注入済・hit なしは ResetOwnState（R3.4）"
    );
    assert_eq!(
        hover_action(false, Some(5), Some(2)),
        HoverAction::ResetOwnState,
        "非表示・注入済は hit があっても ResetOwnState（hit_ordinal 無視・R3.4）"
    );
}

/// 同値維持（遷移なし）: 表示中で hit_ordinal == last_injected は再注入しない Keep。
/// Some==Some と None==None の双方を確認する。
#[test]
fn hover_active_same_value_keeps() {
    assert_eq!(
        hover_action(true, Some(2), Some(2)),
        HoverAction::Keep,
        "表示中・同一 Some は Keep（再注入しない）"
    );
    assert_eq!(
        hover_action(true, None, None),
        HoverAction::Keep,
        "表示中・None 同値は Keep（ハイライト無しのまま遷移なし）"
    );
}

/// 新規注入（R1.2）: 表示中で hover 対象が変化したら新値を Inject する。
/// None→Some の初回注入と Some→Some の遷移の双方を確認する。
#[test]
fn hover_active_new_row_injects() {
    assert_eq!(
        hover_action(true, Some(1), None),
        HoverAction::Inject(Some(1)),
        "表示中・None→Some(1) は Inject(Some(1))（初回ハイライト・R1.2）"
    );
    assert_eq!(
        hover_action(true, Some(3), Some(1)),
        HoverAction::Inject(Some(3)),
        "表示中・Some(1)→Some(3) は Inject(Some(3))（行遷移・R1.2）"
    );
}

/// 解除注入（R1.3）: 表示中で hover が行外へ出たら None を Inject する
/// （Some→None＝ハイライト解除の遷移）。
#[test]
fn hover_active_leaves_row_injects_none() {
    assert_eq!(
        hover_action(true, None, Some(2)),
        HoverAction::Inject(None),
        "表示中・Some(2)→None は Inject(None)（ハイライト解除・R1.3）"
    );
}

// -------------------------------------------------------------------------
// click_selection 純関数檻（task 3.3・design「純関数判定核」／R2.1/2.2/2.3/3.1/3.2）
//
// active・現行 rows・click 座標・scope の入力から確定 ChoiceSelection の構成を
// 決める純関数を、World・runtime・send 不要で決定的に判定する。
// ヒット時のフィールド一致（scope は arg 由来・ordinal 非含有）、非 hit／非表示／
// stale 行での非構成（None）を網羅する（R2.1/2.2/2.3/3.1/3.2/6.2/6.3）。
// -------------------------------------------------------------------------

/// `references` を明示指定できる `ChoiceHitRow` を組む（転写忠実性の検証用）。
fn row_with_refs(
    ordinal: usize,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    references: Vec<String>,
) -> ChoiceHitRow {
    ChoiceHitRow {
        ordinal,
        id: format!("q{ordinal}"),
        label: format!("label{ordinal}"),
        references,
        rect: HitRectPx {
            left,
            top,
            right,
            bottom,
        },
    }
}

/// ヒット確定（R2.1/2.2）: 表示中・ヒット座標では現行ヒット行の全フィールドを
/// clone 転写した `ChoiceSelection` を返す。scope は arg 由来・ordinal は非含有。
#[test]
fn click_hit_builds_selection_from_current_row() {
    let rows = [
        row_with_refs(0, 0.0, 0.0, 100.0, 20.0, vec!["a".to_string()]),
        row_with_refs(
            1,
            0.0,
            20.0,
            100.0,
            40.0,
            vec!["r0".to_string(), "r1".to_string()],
        ),
    ];
    // (50, 30) は index 1 の行内。
    let sel = click_selection(true, &rows, 50.0, 30.0, 7)
        .expect("表示中・ヒット座標では ChoiceSelection を構成する");
    assert_eq!(sel.id, "q1", "id は現行ヒット行から転写");
    assert_eq!(sel.label, "label1", "label は現行ヒット行から転写");
    assert_eq!(sel.scope, 7, "scope は引数由来（BalloonWindowMarker.scope）");
    assert_eq!(
        sel.references,
        vec!["r0".to_string(), "r1".to_string()],
        "references は現行ヒット行から忠実転写"
    );
}

/// 非ヒット→非発行（R2.3）: 表示中でも全矩形外の座標なら None。
#[test]
fn click_non_hit_returns_none() {
    let rows = [row_with_refs(0, 10.0, 20.0, 50.0, 40.0, Vec::new())];
    assert_eq!(
        click_selection(true, &rows, 5.0, 5.0, 0),
        None,
        "全矩形外の click は非構成（None・R2.3）"
    );
}

/// 非表示中は非発行（R3.1）: `active == false` は、たとえ矩形内座標でも None。
/// hit 判定より前に短絡する（消滅時の stale／原子性ガード）。
#[test]
fn click_inactive_returns_none_even_inside_rect() {
    let rows = [row_with_refs(0, 10.0, 20.0, 50.0, 40.0, Vec::new())];
    // (30, 30) は矩形内だが active=false ゆえ None。
    assert_eq!(
        click_selection(false, &rows, 30.0, 30.0, 0),
        None,
        "非表示中（active=false）は矩形内でも非構成（None・R3.1）"
    );
}

/// stale 行棄却（R3.2/6.3）: 以前ヒットしていた座標に、現行 rows ではどの行も
/// 存在しない（レイアウト差替後）場合、同座標の click は None。
/// 現行ジオメトリのみを読むことを、キャッシュ座標を覆わない現行 rows で固定する。
#[test]
fn click_stale_coords_not_in_current_rows_returns_none() {
    // hover 時代には座標 (30, 30) に行があったが、現行 rows はその座標を覆わない
    // （行が消滅／別位置へ差し替わった）。
    let current = [row_with_refs(0, 200.0, 200.0, 240.0, 220.0, Vec::new())];
    assert_eq!(
        click_selection(true, &current, 30.0, 30.0, 0),
        None,
        "現行 rows がクリック座標を覆わなければ stale 棄却で None（R3.2）"
    );
}

/// stale 差替（R3.2/2.5）: 同座標に別行が現れた場合、確定は必ず**現行**行から
/// 構成される（キャッシュではなく現行ジオメトリが正本）。
#[test]
fn click_replaced_row_builds_from_current_not_cached() {
    // 現行 rows: 座標 (30, 30) を覆うのは ordinal 9 の別行のみ。
    let current = [row_with_refs(9, 10.0, 20.0, 50.0, 40.0, vec!["z".to_string()])];
    let sel = click_selection(true, &current, 30.0, 30.0, 3)
        .expect("現行行がヒットするので構成される");
    assert_eq!(sel.id, "q9", "確定は現行ヒット行（差替後）から構成される");
    assert_eq!(sel.label, "label9", "label も現行行から");
    assert_eq!(sel.references, vec!["z".to_string()], "references も現行行から");
    assert_eq!(sel.scope, 3, "scope は引数由来");
}

/// 空 references の忠実転写: 参照列が空でも空 Vec として構成される。
#[test]
fn click_empty_references_transcribed_as_empty() {
    let rows = [row_with_refs(0, 0.0, 0.0, 100.0, 20.0, Vec::new())];
    let sel = click_selection(true, &rows, 50.0, 10.0, 0).expect("ヒットするので構成される");
    assert!(sel.references.is_empty(), "空 references は空 Vec として転写");
}
