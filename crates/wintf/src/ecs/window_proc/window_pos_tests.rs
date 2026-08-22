use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::DPI;
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;
use bevy_ecs::prelude::Entity;
use std::cell::RefCell;
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::WM_DPICHANGED;

/// 診断手順書が指定する `RUST_LOG`（design.md「診断手順書」・要件 1.4/1.5）。
/// 観測点がこの directive で点灯することを機械的に固定するため、リテラルを共有する。
const PROCEDURE_DIRECTIVES: &str =
    "info,wintf::ecs::window_proc=debug,wintf::ecs::drag=debug,areka::placement::diag=debug";

/// 位置書込の共通経路（`guarded_set_window_pos`）まで開ける directive。
/// `wintf::ecs::window` は前方一致ゆえ `wintf::ecs::window_proc` も併せて開く。
const WRITE_PATH_DIRECTIVES: &str = "info,wintf::ecs::window=debug";

/// 既定水準（`RUST_LOG` 未設定時のフォールバック＝areka `main.rs` の `EnvFilter::try_from_default_env()`）。
const DEFAULT_DIRECTIVES: &str = "info";

/// `WM_DPICHANGED` メッセージを組む（LOWORD=X DPI / HIWORD=Y DPI・LPARAM=提案矩形）。
///
/// `suggested` は呼出側が生存させる（LPARAM は生ポインタ）。
fn dpichanged_message(new_dpi: u16, suggested: &RECT) -> WindowMessage {
    WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg: WM_DPICHANGED,
        wparam: WPARAM(((new_dpi as usize) << 16) | new_dpi as usize),
        lparam: LPARAM(suggested as *const RECT as isize),
    }
}

/// ヘッドレスに `WM_DPICHANGED` を 1 回配送する（実 HWND・メッセージループ不要）。
///
/// `hwnd` は null ゆえ `SetWindowPos` は失敗するが、観測点はいずれも呼び出し前後に
/// 置かれており本檻の対象（水準とフィールド）には影響しない。
fn dispatch_dpichanged(new_dpi: u16, suggested: RECT) -> Entity {
    // 政策未宣言の窓では `guarded_set_window_pos` が走り、**プロセス共有**の
    // `SELF_INITIATED_DEPTH` を一時的に持ち上げる。並列に走る他テストの
    // `is_self_initiated()`／`in_swp` 検査が偽の失敗を起こさないよう、配送の区間を
    // 直列化する（`crate::ecs::window::lock_self_initiated_for_test` の doc・要件 7.7）。
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world
        .borrow_mut()
        .world_mut()
        .spawn(DPI::from_dpi(96, 96))
        .id();

    let m = dpichanged_message(new_dpi, &suggested);
    let _ = crate::ecs::dispatch_window_message(&world, entity, &m);

    // TLS に残る DpiChangeContext を回収し、同スレッドの後続テストへ漏らさない。
    let _ = crate::ecs::window::DpiChangeContext::take();
    entity
}

fn suggested_rect() -> RECT {
    RECT {
        left: 3210,
        top: 140,
        right: 3810,
        bottom: 620,
    }
}

/// 要件 1.3: 「提案位置に基づく位置変更を実際に行ったか否か」は、診断手順書の
/// directive で**必ず**点灯する水準に置かれている（旧 `trace!` は点灯せず、
/// 2026-07-18 の偽陰性＝「発生 0 回」の誤結論を生んだ）。
#[test]
fn suggested_position_decision_is_visible_under_procedure_directive() {
    let out = capture_under_filter(PROCEDURE_DIRECTIVES, || {
        dispatch_dpichanged(192, suggested_rect());
    });

    assert!(
        out.contains("[WM_DPICHANGED] suggested position write decision"),
        "提案位置の実施可否が診断手順の水準で観測できない（要件 1.3/1.5）: {out}"
    );
}

/// 要件 1.3: 実施可否の行は「書いたか否か」と提案 left/top・entity を伴う。
#[test]
fn suggested_position_decision_carries_applied_flag_and_suggested_origin() {
    let out = capture_under_filter(PROCEDURE_DIRECTIVES, || {
        dispatch_dpichanged(192, suggested_rect());
    });

    let line = out
        .lines()
        .find(|l| l.contains("[WM_DPICHANGED] suggested position write decision"))
        .unwrap_or_else(|| panic!("実施可否行が無い: {out}"));

    assert!(
        line.contains("applied="),
        "実施可否フィールドが無い: {line}"
    );
    assert!(
        line.contains("suggested_left=3210") && line.contains("suggested_top=140"),
        "提案 left/top が復元できない: {line}"
    );
    assert!(
        line.contains("entity="),
        "表示基盤ログと areka 側レコードの結合キー `entity` が無い: {line}"
    );
}

/// 要件 1.3: 新旧 DPI も同じ directive で点灯する（受理回数の 2 段 grep 計数の前提）。
#[test]
fn dpi_acceptance_line_reports_old_and_new_dpi_under_procedure_directive() {
    let out = capture_under_filter(PROCEDURE_DIRECTIVES, || {
        dispatch_dpichanged(192, suggested_rect());
    });

    let line = out
        .lines()
        .find(|l| l.contains("[WM_DPICHANGED] DPI component directly updated"))
        .unwrap_or_else(|| panic!("DPI 受理行が無い: {out}"));

    assert!(
        line.contains("old_dpi_x=96") && line.contains("new_dpi_x=192"),
        "新旧 DPI が同一行から復元できない（方向の機械判定が不能）: {line}"
    );
}

/// 要件 1.5: 既定水準（`info`）では実施可否の行は出ない。
/// ＝観測点が「手順で有効化される水準」に置かれていることの対偶側の固定。
#[test]
fn suggested_position_decision_is_silent_under_default_info_filter() {
    let out = capture_under_filter(DEFAULT_DIRECTIVES, || {
        dispatch_dpichanged(192, suggested_rect());
    });

    assert!(
        !out.contains("[WM_DPICHANGED] suggested position write decision"),
        "既定 `info` で診断専用の実施可否行が漏れている: {out}"
    );
}

/// 要件 1.3: 実際の窓位置書込を行う共通経路（`guarded_set_window_pos`）の実施ログも
/// 診断手順が有効化できる水準にある（旧 `trace!` からの是正）。
#[test]
fn window_pos_write_path_is_visible_at_debug() {
    let out = capture_under_filter(WRITE_PATH_DIRECTIVES, || {
        dispatch_dpichanged(192, suggested_rect());
    });

    assert!(
        out.contains("[guarded_set_window_pos] Calling SetWindowPos"),
        "窓位置書込の共通経路が debug 水準で観測できない（要件 1.3）: {out}"
    );
}

/// 要件 1.5: 書込経路の実施ログも既定 `info` では出ない（診断専用のまま）。
#[test]
fn window_pos_write_path_is_silent_under_default_info_filter() {
    let out = capture_under_filter(DEFAULT_DIRECTIVES, || {
        dispatch_dpichanged(192, suggested_rect());
    });

    assert!(
        !out.contains("[guarded_set_window_pos] Calling SetWindowPos"),
        "既定 `info` で書込経路ログが漏れている: {out}"
    );
}

// ================================================================
// S1 の赤証跡＝表示基盤ディスパッチ檻（タスク 4.3・Req 5.4／4.3／4.2）
//
// design.md「Testing Strategy > Integration Tests 5」が S1 の赤→緑の
// **正証跡**と定める檻。是正前の欠陥は wndproc の無条件書込＝実配線に
// 在るため、`dpi_helpers.rs` の純関数檻（分岐網羅の補助）では
// 「是正前のコードに対して失敗する」の証明力が足りない。
//
// ## 何を主張しているか（是正後の契約＝D3）
// - `DpiSuggestedRectPolicy::ExternalAuthority` を宣言した窓:
//   `DPI` component は更新される／`DpiChangeContext` は **確立されない**／
//   位置書込も **起きない**（＝最終位置は現接地点のまま残る）
// - 宣言の無い窓（既定 `ApplyPosition` 相当）: 従来どおり確立され書かれる
//   （非ゴースト窓の後方互換・design.md「Compatibility」）
//
// ## 是正投入（タスク 5.1）で赤 4 件は緑へ反転し、常時走る回帰檻へ昇格した
// 4.3 採取時点の `WM_DPICHANGED` は `DpiChangeContext::set` も
// `guarded_set_window_pos` も **窓ごとの分岐なしに** 実行し、`let applied = true;`
// の定数がその事実の表示だった（診断レポート §1.1・§3.1 に赤の実行出力を保存）。
// タスク 5.1 が `dpi_suggested_position_decision`（`window_proc/dpi_helpers.rs:31`）を
// ハンドラへ配線し、`None` 判定で②③をまとめて飛ばす形（D3）へ組み替えた。
//
// ## 無視属性（ignore）のゲートは 5.1 で全 4 件とも撤去済み（dpi96 を含む）
// 赤の採取中だけは常時失敗する檻を通常実行から外していた（`cargo test` を門として
// 無価値にしないため・`areka-emo-atlas/src/emo2_golden.rs:228` の先例）。是正後は
// 4 件とも常時走る。**本ファイルに無視属性が 1 件も残っていないこと**が完了条件で
// あり、属性名の grep がゼロ件になることで機械的に確認できる（この注記自体が当たると
// 検査が壊れるため、ここでは属性を字面で書かない）。**dpi96 の 1 件も外してある**——
// 「96 では緑」は是正後も成立する性質であり、外して初めて 96/120/192 の非対称が
// 回帰檻として保存される。名前の `s1_red_` 接頭辞は赤証跡としての出自を示す履歴で
// あって、現在の色ではない。
//
// ## dpi 水準の非対称（Req 5.1／5.4 の「96 が欠陥を隠す」）
// 96 では OS 提案原点が現位置と一致するため、提案位置を書いても書かなくても
// 最終位置が変わらず **政策分岐が観測できない＝赤にならない**。120／192 では
// 提案原点が現位置から離れるため、無条件書込が接地点を破壊して赤になる。
// 水準ごとに独立した檻へ分けてあるのは、この非対称が 1 回の実行出力から
// そのまま読み取れるようにするためである。
// ================================================================

use crate::ecs::window::DpiSuggestedRectPolicy;

/// 「areka が直前に確定した接地点」の代役となる現位置（物理 px）。
///
/// 具体値そのものに意味は無い——判定は下の `suggested_rect_for` が組む
/// **DPI 水準に対する比**と「現位置が保存されるか」の不変条件で表現する（Req 5.6）。
const CURRENT_ORIGIN: (i32, i32) = (1200, 400);

/// 提案矩形の寸（`SWP_NOSIZE` ゆえ判断には使われない・原点だけが効く）。
const SUGGESTED_EXTENT: (i32, i32) = (400, 300);

/// モニタ跨ぎ相当の OS 提案矩形を **比** で組む（絶対 px の直書きを避ける・Req 5.6）。
///
/// `dpi=96` では比が 1.0 ゆえ提案原点＝現位置になる（＝欠陥が隠れる水準）。
fn suggested_rect_for(dpi: u16, current: (i32, i32)) -> RECT {
    let ratio = dpi as f32 / 96.0;
    let left = (current.0 as f32 * ratio).round() as i32;
    let top = (current.1 as f32 * ratio).round() as i32;
    RECT {
        left,
        top,
        right: left + SUGGESTED_EXTENT.0,
        bottom: top + SUGGESTED_EXTENT.1,
    }
}

/// ディスパッチの外から観測できる 3 事実。
#[derive(Debug)]
struct DpiChangedOutcome {
    /// dispatch 後の `DPI` component の X（是正の有無によらず更新されるべき値）。
    dpi_x_after: u16,
    /// `DpiChangeContext` が確立されたか（＝提案位置の書込コンテキスト）。
    context_established: bool,
    /// 実窓へ書かれた原点（`guarded_set_window_pos` の実施ログから復元）。
    /// `None` = 書込が 1 度も起きていない。
    written_origin: Option<(i32, i32)>,
    /// ハンドラの戻り値（`LRESULT` の生値）。**源断ちの最外殻**——
    /// `None`（＝未処理）を返すと `DefWindowProcW` が既定の提案矩形適用を行い、
    /// その中から `SetWindowPos` が同期的に呼ばれる（`window/components.rs:29-31`
    /// が明記）。ゆえに `guarded_set_window_pos` を飛ばしただけでは源断ちは
    /// 完成せず、`Some(0)`（＝処理済み）を返し切って初めて成立する。
    handler_result: Option<isize>,
}

impl DpiChangedOutcome {
    /// 「書かなければ現位置が最終位置として残る」を畳んだ最終位置。
    fn final_position(&self, current: (i32, i32)) -> (i32, i32) {
        self.written_origin.unwrap_or(current)
    }
}

/// `guarded_set_window_pos` の実施ログ 1 行から書込原点を復元する。
///
/// トークン境界でアンカーする——`x=` は `cx=` の接尾辞であり、部分一致では
/// 取り違える（申し送り「`w=-` が `w=-12` の接頭辞」と同型の罠）。
fn parse_write_origin(out: &str) -> Option<(i32, i32)> {
    let line = out
        .lines()
        .find(|l| l.contains("[guarded_set_window_pos] Calling SetWindowPos"))?;
    let field = |name: &str| -> Option<i32> {
        line.split_whitespace()
            .find_map(|tok| tok.strip_prefix(name))
            .and_then(|v| v.parse::<i32>().ok())
    };
    Some((field("x=")?, field("y=")?))
}

/// 政策 component の有無を変えて `WM_DPICHANGED` を 1 回配送し、**診断手順書の
/// directive** で濾過したログ出力をそのまま返す（実施可否行のフィールド検査用）。
fn dispatch_dpichanged_logged(
    new_dpi: u16,
    suggested: RECT,
    policy: Option<DpiSuggestedRectPolicy>,
) -> String {
    // 書込経路を通り得るので直列化する（`dispatch_dpichanged` と同じ理由）。
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = {
        let mut w = world.borrow_mut();
        let mut e = w.world_mut().spawn(DPI::from_dpi(96, 96));
        if let Some(p) = policy {
            e.insert(p);
        }
        e.id()
    };

    let m = dpichanged_message(new_dpi, &suggested);
    let out = capture_under_filter(PROCEDURE_DIRECTIVES, || {
        let _ = crate::ecs::dispatch_window_message(&world, entity, &m);
    });
    let _ = crate::ecs::window::DpiChangeContext::take();
    out
}

/// 実施可否行から 1 行を取り出す（無ければ panic して出力全体を見せる）。
fn decision_line(out: &str) -> String {
    out.lines()
        .find(|l| l.contains("[WM_DPICHANGED] suggested position write decision"))
        .unwrap_or_else(|| panic!("実施可否行が無い: {out}"))
        .to_string()
}

/// 政策 component の有無を変えて `WM_DPICHANGED` を 1 回配送し、外形を観測する。
fn dispatch_dpichanged_observed(
    new_dpi: u16,
    suggested: RECT,
    policy: Option<DpiSuggestedRectPolicy>,
) -> DpiChangedOutcome {
    // 書込経路を通り得るので直列化する（`dispatch_dpichanged` と同じ理由）。
    let _serialized = crate::ecs::window::lock_self_initiated_for_test();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = {
        let mut w = world.borrow_mut();
        let mut e = w.world_mut().spawn(DPI::from_dpi(96, 96));
        if let Some(p) = policy {
            e.insert(p);
        }
        e.id()
    };

    let m = dpichanged_message(new_dpi, &suggested);
    let mut handler_result = None;
    let out = capture_under_filter(WRITE_PATH_DIRECTIVES, || {
        handler_result =
            crate::ecs::dispatch_window_message(&world, entity, &m).map(|lresult| lresult.0);
    });

    // TLS に残る `DpiChangeContext` を回収する。回収は観測そのものであり、
    // 同時に同スレッドの後続テストへの漏洩も防ぐ。
    let context_established = crate::ecs::window::DpiChangeContext::take().is_some();

    let dpi_x_after = {
        let mut w = world.borrow_mut();
        w.world_mut()
            .get::<DPI>(entity)
            .copied()
            .expect("DPI component は spawn 時に付与済み")
            .dpi_x
    };

    DpiChangedOutcome {
        dpi_x_after,
        context_established,
        written_origin: parse_write_origin(&out),
        handler_result,
    }
}

/// 水準を引数に取る S1 赤檻の本体。
///
/// 主張は「`ExternalAuthority` 窓の最終位置＝現接地点」（Req 4.3: OS 推奨位置を
/// 最終位置としてそのまま残さない／Req 4.2: 最終位置が接地点規約に従う）。
fn assert_external_authority_preserves_anchor_at(dpi: u16) {
    let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
    let outcome = dispatch_dpichanged_observed(
        dpi,
        suggested,
        Some(DpiSuggestedRectPolicy::ExternalAuthority),
    );

    // S1 の是正は DPI 受理を止めるものではない（止めたら寸の再導出が死ぬ）。
    assert_eq!(
        outcome.dpi_x_after, dpi,
        "dpi={dpi}: DPI component が更新されていない: {outcome:?}"
    );

    // 探針の自己検査: 96 は提案＝現位置（分岐が観測できない水準）、
    // 96 以外は提案が現位置から離れている（＝不動点でない・記憶〈2.2 の教訓〉）。
    if dpi == 96 {
        assert_eq!(
            (suggested.left, suggested.top),
            CURRENT_ORIGIN,
            "dpi=96 では提案原点が現位置と一致する前提（96 が欠陥を隠す性質）"
        );
    } else {
        assert_ne!(
            suggested.left, CURRENT_ORIGIN.0,
            "dpi={dpi} では提案 X が現位置から動いている前提"
        );
    }

    assert_eq!(
        outcome.final_position(CURRENT_ORIGIN),
        CURRENT_ORIGIN,
        "dpi={dpi}: ExternalAuthority 窓の最終位置は現接地点のままであるべき\
         （OS 提案位置が無条件に採用されている＝S1・Req 4.3/4.2）: {outcome:?}"
    );
}

/// dpi=96: 提案原点＝現位置ゆえ、是正の有無にかかわらず**通過する**。
/// この緑が「96 の自己整合が欠陥を隠す」ことの実行証跡である（Req 5.1／5.4）。
#[test]
fn s1_red_external_authority_preserves_anchor_at_dpi96() {
    assert_external_authority_preserves_anchor_at(96);
}

/// dpi=120: 提案原点が現位置から離れる → 是正未投入では失敗していた水準（4.3 の赤）。
#[test]
fn s1_red_external_authority_preserves_anchor_at_dpi120() {
    assert_external_authority_preserves_anchor_at(120);
}

/// dpi=192: 同上（変位がさらに大きい）。
#[test]
fn s1_red_external_authority_preserves_anchor_at_dpi192() {
    assert_external_authority_preserves_anchor_at(192);
}

/// D3 の構造そのもの: `ExternalAuthority` 窓では
/// **書込コンテキストが確立されず**、**位置書込も起きない**。
///
/// 上の 3 件が「最終位置」という外形で主張するのに対し、こちらは
/// 是正が置かれた 2 箇所（`DpiChangeContext::set` と
/// `guarded_set_window_pos`）を名指しで固定する。是正未投入では両方とも
/// 無条件に走るため 120／192 で失敗していた（4.3 の赤）。
#[test]
fn s1_red_external_authority_establishes_no_write_context() {
    for dpi in [120_u16, 192] {
        let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
        let outcome = dispatch_dpichanged_observed(
            dpi,
            suggested,
            Some(DpiSuggestedRectPolicy::ExternalAuthority),
        );

        assert_eq!(
            outcome.dpi_x_after, dpi,
            "dpi={dpi}: DPI component が更新されていない: {outcome:?}"
        );
        assert!(
            !outcome.context_established,
            "dpi={dpi}: ExternalAuthority 窓で DpiChangeContext が確立されている\
             （残置コンテキストが後続 WM_WINDOWPOSCHANGED を DPI echo と誤認させる・D3）: {outcome:?}"
        );
        assert!(
            outcome.written_origin.is_none(),
            "dpi={dpi}: ExternalAuthority 窓へ OS 提案位置が書き込まれている（S1）: {outcome:?}"
        );
    }
}

/// 非退行（**是正の前後いずれでも緑**・ゲート外で常時走る）:
/// 政策未宣言／明示 `ApplyPosition` の窓は従来どおり提案原点を書き、
/// `DpiChangeContext` も確立する。
///
/// これが崩れると examples・将来の通常窓が Per-Monitor v2 の標準応答を失う
/// （design.md「Compatibility」）。
#[test]
fn s1_control_default_policy_windows_apply_suggested_origin() {
    for dpi in [96_u16, 120, 192] {
        for policy in [None, Some(DpiSuggestedRectPolicy::ApplyPosition)] {
            let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
            let outcome = dispatch_dpichanged_observed(dpi, suggested, policy);

            assert_eq!(
                outcome.dpi_x_after, dpi,
                "dpi={dpi} policy={policy:?}: DPI component が更新されていない: {outcome:?}"
            );
            assert!(
                outcome.context_established,
                "dpi={dpi} policy={policy:?}: 既定窓で DpiChangeContext が確立されない: {outcome:?}"
            );
            assert_eq!(
                outcome.written_origin,
                Some((suggested.left, suggested.top)),
                "dpi={dpi} policy={policy:?}: 既定窓へ提案原点が書かれていない: {outcome:?}"
            );
        }
    }
}

/// D3 の帰結（2.1 → 5.1 申し送り）: `DpiChangeContext::set` と
/// `guarded_set_window_pos` は **1 個の `if let Some((x, y))` で束ねて分岐**する。
/// ゆえに「コンテキスト確立 ⇔ 位置書込」が常に成り立たねばならない。
///
/// 是正未投入では両者とも恒真なので緑（＝本檻は赤証跡ではない）。5.1 が
/// 片側だけを分岐させた場合に赤になる**設計上の分割禁止**の固定である。
#[test]
fn s1_write_context_and_position_write_are_branched_together() {
    for dpi in [96_u16, 120, 192] {
        for policy in [
            None,
            Some(DpiSuggestedRectPolicy::ApplyPosition),
            Some(DpiSuggestedRectPolicy::ExternalAuthority),
        ] {
            let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
            let outcome = dispatch_dpichanged_observed(dpi, suggested, policy);

            assert_eq!(
                outcome.context_established,
                outcome.written_origin.is_some(),
                "dpi={dpi} policy={policy:?}: コンテキスト確立と位置書込が別々に分岐している（D3 違反）: {outcome:?}"
            );
        }
    }
}

// ================================================================
// 実施可否行のフィールド（タスク 5.1・Req 1.3・design.md:330）
//
// 1.3 は `applied` を `let applied = true;` の**定数**として置き、`policy`
// フィールドは未出力のまま 5.1 へ申し送っていた。以下 2 件は対で
// 「`applied` が実際に分岐すること」と「どちらの政策でそうなったかが
// 同一行から復元できること」を固定する。片方だけでは定数のままでも
// 通ってしまう（`applied=true` 固定なら②が、`false` 固定なら①が赤になる）。
// ================================================================

/// `ExternalAuthority` 窓の実施可否行は `applied=false` と政策名を載せる。
#[test]
fn s1_decision_line_reports_external_authority_and_applied_false() {
    let suggested = suggested_rect_for(192, CURRENT_ORIGIN);
    let out = dispatch_dpichanged_logged(
        192,
        suggested,
        Some(DpiSuggestedRectPolicy::ExternalAuthority),
    );
    let line = decision_line(&out);

    assert!(
        line.contains("applied=false"),
        "外部権威窓なのに実施可否が false でない（`applied` が定数のまま）: {line}"
    );
    assert!(
        line.contains("policy=ExternalAuthority"),
        "判断の根拠となった政策が同一行から復元できない（design.md:330 のフィールド）: {line}"
    );
    // 提案原点は「書かなかった値」として引き続き載る（何を退けたかが読めること）。
    assert!(
        line.contains(&format!("suggested_left={}", suggested.left)),
        "退けた提案原点が記録されていない: {line}"
    );
}

/// 政策未付与の窓は従来どおり `applied=true`。政策名は「未付与」と読める語で出す。
#[test]
fn s1_decision_line_reports_unset_policy_and_applied_true() {
    let out = dispatch_dpichanged_logged(192, suggested_rect_for(192, CURRENT_ORIGIN), None);
    let line = decision_line(&out);

    assert!(
        line.contains("applied=true"),
        "既定窓の実施可否が true でない（後方互換の非退行が壊れている）: {line}"
    );
    assert!(
        line.contains("policy=unset"),
        "component 未付与が政策フィールドから読み取れない: {line}"
    );
}

/// 明示 `ApplyPosition` は `unset` と**同じ判断**だが**別の語**で報告される。
///
/// 「未付与だったのか、既定を明示宣言した窓だったのか」は実機ログの事後突合で
/// 別物として読めなければならない（`diagnosis-procedure.md` §3.4 の値語彙表）。
/// 判断が同じだからといって語を畳むと、政策の付与漏れと明示既定が区別できなくなる。
#[test]
fn s1_decision_line_reports_apply_position_as_its_own_label() {
    let out = dispatch_dpichanged_logged(
        192,
        suggested_rect_for(192, CURRENT_ORIGIN),
        Some(DpiSuggestedRectPolicy::ApplyPosition),
    );
    let line = decision_line(&out);

    assert!(
        line.contains("policy=ApplyPosition"),
        "明示 ApplyPosition が専用の語で報告されていない: {line}"
    );
    assert!(
        line.contains("applied=true"),
        "ApplyPosition は書く判断のはず（純関数の契約と食い違っている）: {line}"
    );
    // 探針の自己検査: 4 語彙は互いに部分文字列でない（`unset` を含む行が
    // `ApplyPosition` の行と取り違えられない＝トークン取り違えの罠を封じる）。
    assert!(
        !line.contains("policy=unset") && !line.contains("policy=unreachable"),
        "政策フィールドに複数の語が同時に現れている（書式の取り違え）: {line}"
    );
}

/// **政策を読めなかった**場合は `unreachable` で報告し、`unset`（宣言が無かった）と
/// 混同させない（Req 1.5）。判断自体は従来挙動へフォールバックする（`applied=true`）。
///
/// 到達不能は 2 経路ある——⑴entity が破棄済み（`get_entity_mut` が `Err`）
/// ⑵World が既に借用されている（再入・`try_borrow_mut` が `Err`）。どちらも
/// 同じ語を出すことを固定する。
///
/// **探針は `ExternalAuthority` を付けた窓で組む**——読めていれば
/// `policy=ExternalAuthority`・`applied=false` になるはずの窓が `unreachable`・
/// `applied=true` を出すことで、「読めなかった」と「宣言が無い」の区別が
/// 実際に効いていることが分かる（不動点に落ちない探針）。
#[test]
fn s1_decision_line_reports_unreachable_when_policy_cannot_be_read() {
    let suggested = suggested_rect_for(192, CURRENT_ORIGIN);

    // ⑴ entity が破棄済み
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let despawned = {
        let mut w = world.borrow_mut();
        let e = w
            .world_mut()
            .spawn((
                DPI::from_dpi(96, 96),
                DpiSuggestedRectPolicy::ExternalAuthority,
            ))
            .id();
        w.world_mut().despawn(e);
        e
    };
    let m = dpichanged_message(192, &suggested);
    // 到達不能時のフォールバックは「書く」ので、この配送も書込経路を通る。錠は**この区間
    // だけ**で持つ——末尾の `dispatch_dpichanged_logged` が自分で取るため、テスト全体で
    // 抱えたままにすると再入で自分自身と競合する（`std::sync::Mutex` は再入不可）。
    let out = {
        let _serialized = crate::ecs::window::lock_self_initiated_for_test();
        capture_under_filter(PROCEDURE_DIRECTIVES, || {
            let _ = crate::ecs::dispatch_window_message(&world, despawned, &m);
        })
    };
    let _ = crate::ecs::window::DpiChangeContext::take();
    let line = decision_line(&out);
    assert!(
        line.contains("policy=unreachable"),
        "破棄済み entity で政策が読めなかったのに `unreachable` を名乗っていない\
         （`unset` と混同すると事後突合が偽の結論を作る・Req 1.5）: {line}"
    );
    assert!(
        line.contains("applied=true"),
        "到達不能時のフォールバックは従来挙動（書く）であるべき: {line}"
    );

    // ⑵ World が既に借用されている（wndproc 再入相当）
    let world2 = Rc::new(RefCell::new(EcsWorld::new()));
    let entity2 = world2
        .borrow_mut()
        .world_mut()
        .spawn((
            DPI::from_dpi(96, 96),
            DpiSuggestedRectPolicy::ExternalAuthority,
        ))
        .id();
    let m2 = dpichanged_message(192, &suggested);
    let out2 = {
        let _serialized = crate::ecs::window::lock_self_initiated_for_test();
        capture_under_filter(PROCEDURE_DIRECTIVES, || {
            let _held = world2.borrow_mut(); // 借用を保持したまま配送する
            let _ = crate::ecs::dispatch_window_message(&world2, entity2, &m2);
        })
    };
    let _ = crate::ecs::window::DpiChangeContext::take();
    let line2 = decision_line(&out2);
    assert!(
        line2.contains("policy=unreachable"),
        "World 借用の再入で政策が読めなかったのに `unreachable` を名乗っていない: {line2}"
    );

    // 探針の自己検査: 読めていれば `ExternalAuthority` になるはずの窓である
    // （＝この檻は「政策が無い窓」を見ているのではない）。
    let readable = dispatch_dpichanged_logged(
        192,
        suggested,
        Some(DpiSuggestedRectPolicy::ExternalAuthority),
    );
    assert!(
        decision_line(&readable).contains("policy=ExternalAuthority"),
        "同一構成の窓が到達可能なら ExternalAuthority を出すはず（探針が退化している）"
    );
}

// ================================================================
// 源断ちの最外殻: ハンドラの戻り値（タスク 5.1・Req 4.3）
//
// `guarded_set_window_pos` を飛ばしただけでは源断ちは完成しない。
// `None`（未処理）を返すと `DefWindowProcW` が既定の提案矩形適用を行い、
// その内部から `SetWindowPos` が同期的に呼ばれる（`window/components.rs:29-31`
// が当該同期発火を明記している）。ヘッドレス檻の `guarded_set_window_pos`
// 実施ログは自前の書込しか捉えないため、**戻り値を直接見ないと
// この退化は無検出で通る**。
// ================================================================

/// 「書かない」判定でもメッセージは**処理済み**として返す（`DefWindowProcW` へ
/// 委譲しない）。提案矩形の不採用は Per-Monitor v2 の契約違反ではない
/// （OS 提示は勧告である）。
#[test]
fn s1_external_authority_handles_the_message_instead_of_delegating_to_defwindowproc() {
    for dpi in [96_u16, 120, 192] {
        let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
        let outcome = dispatch_dpichanged_observed(
            dpi,
            suggested,
            Some(DpiSuggestedRectPolicy::ExternalAuthority),
        );

        // 探針の自己検査: この走行は本当に「書かなかった」側である
        // （書いた走行で戻り値だけ見ても源断ちの最外殻を見たことにならない）。
        assert!(
            outcome.written_origin.is_none(),
            "dpi={dpi}: 探針が「書かない」経路を通っていない: {outcome:?}"
        );
        assert_eq!(
            outcome.handler_result,
            Some(0),
            "dpi={dpi}: 書かない判定でハンドラが未処理（None）を返している\
             ——`DefWindowProcW` が提案矩形を適用し、その中の `SetWindowPos` が
                 同期的に窓を動かすため源断ちが無効化する: {outcome:?}"
        );
    }
}

/// 非退行: 書く判定でも従来どおり処理済みを返す（既存の窓応答を変えない）。
#[test]
fn s1_default_policy_windows_also_report_the_message_as_handled() {
    for policy in [None, Some(DpiSuggestedRectPolicy::ApplyPosition)] {
        let suggested = suggested_rect_for(192, CURRENT_ORIGIN);
        let outcome = dispatch_dpichanged_observed(192, suggested, policy);

        assert!(
            outcome.written_origin.is_some(),
            "policy={policy:?}: 探針が「書く」経路を通っていない: {outcome:?}"
        );
        assert_eq!(
            outcome.handler_result,
            Some(0),
            "policy={policy:?}: 既定窓でハンドラの戻り値が変わっている: {outcome:?}"
        );
    }
}
