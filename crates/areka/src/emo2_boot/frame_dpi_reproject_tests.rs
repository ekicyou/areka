use areka_emo_compose::ScaleRatio;
use bevy_ecs::prelude::Entity;
use crate::placement::follow::MonitorSnapshot;
use crate::placement::resolver::RectPx;
use crate::placement::test_support::capture_logs as capture_diag_logs;
use wintf::ecs::{Point, SizeI, WindowPos};
use wintf::ecs::DPI;

use super::*;
use super::test_support::{
    FakeReports,
    WRITER_WITNESS,
    arrangement_offset_of,
    assert_no_write,
    dpi_world,
    pos_of,
    reset_write_witness,
    s2_assert_work_area_bottom_moves,
    s2_ground_point,
    s2_snapshot,
    s2_work_area_for_dpi,
    size_of,
    window_move_lines,
    window_move_routes_of,
};

// ── task 4.4: S2 の赤証跡＝DPI 相の位置再射影檻（Req 5.4・診断レポート §1.2／§3.2）──
//
// **S2（診断レポート §1.2 の確定・是正前の構造）**: [`dpi_phase_with`] は
// `source.refresh_scale_report(world, target)` が `Some(new_size)` を返したときだけ
// [`reconcile_window_size`] を呼んでいた。位置の再射影（射影 T の再適用）は
// [`resize_window_to`] の**内部**＝そのゲートの下流にしか無いため、報告が `None` の経路では
// **寸を触らないだけでなく、位置の再射影ごと欠落していた**。
// `EmoPresenter::refresh_scale`（`areka-emo-present`）が `None` を返す経路は 5 つあり、
// うち「不可視」「未表示」は Req 4.6 が名指しで扱う状況、「k は変わったが丸め後の物理寸が
// 同じ」は正常系で日常的に起こる。いずれの場合も窓の DPI は変わっている＝
// **接地点を保つべき work area が変わっている**のに、位置は一切再射影されなかった。
//
// **タスク 5.2（是正）着地後の現在の位置づけ**: 赤 4 件は無視属性を撤去して**常時走る回帰檻**
// へ昇格済み。**dpi96 の 1 件も残してある**——「96 では緑」は是正後も成立する性質であり、
// 4 件が揃って初めて「96 通過／120・192 は是正前なら失敗」という非対称が回帰檻として保存
// される（Req 5.1／5.4）。是正の実体は [`reproject_char_window_at_current_size`]。
//
// 常時走る随伴 2 件（`s2_control_*`／`s2_dpi_phase_writes_nothing_*`）は是正の前後どちらでも
// 緑であり、5.2 が分離を**誤って**実装した場合に赤へ落ちる前方ガードである（実測: 5.2 が当てた
// 変異 M3＝`Some` 経路を据置きへ流す／M6＝現寸でなく別寸で射影する、でそれぞれ赤化した）。

use crate::placement::follow::{
    BalloonFollow, WorkAreaResolution, work_area_for_window_with_origin,
};

/// 当該窓が**今いるモニタ**の work area（射影 T が Y に用いるのと同一の解決規則）。
///
/// 最近傍フォールバックで解決していたら合成レイアウトが退化している（窓がどのモニタにも
/// 属していない）ため、その場で檻を落とす——S3 の「フォールバックが異常を無観測で吸収する」
/// 性質をこの檻の内部に持ち込まないための自己検査である。
fn s2_resolved_work_area(world: &World, e: Entity) -> RectPx {
    let pos = pos_of(world, e).expect("WindowPos.position がある");
    let size = size_of(world, e).expect("WindowPos.size がある");
    let rect = RectPx {
        left: pos.x,
        top: pos.y,
        right: pos.x + size.width,
        bottom: pos.y + size.height,
    };
    let snapshot = world
        .get_resource::<MonitorSnapshot>()
        .expect("MonitorSnapshot がある");
    let (wa, origin) =
        work_area_for_window_with_origin(snapshot, rect).expect("空 snapshot ではない");
    assert_eq!(
        origin,
        WorkAreaResolution::Contains,
        "探針の退化: 窓中心がどのモニタにも属さず最近傍フォールバックで解決された（合成レイアウトが壊れている）"
    );
    wa
}

/// [`WindowPos.position`] の Y を直接ずらす（単一ライターを経由しない＝書込 witness を汚さない）。
fn s2_shift_y(world: &mut World, e: Entity, dy: i32) {
    if dy == 0 {
        return;
    }
    let mut wp = world.get_mut::<WindowPos>(e).expect("WindowPos がある");
    let pos = wp.position.expect("position がある");
    wp.position = Some(Point {
        x: pos.x,
        y: pos.y + dy,
    });
}

/// S2 探針の 1 窓ぶんの観測（接地点と、そのとき解決された work area 下端）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct S2Row {
    scope: usize,
    /// 接地点（下端中央）＝`(x + w/2, y + h)`。
    ground: (i32, i32),
    /// 当該窓が今いるモニタの work area 下端（接地規約が要求する Y 成分）。
    wa_bottom: i32,
}

/// S2 探針の観測結果（DPI 変化の前後）。
struct S2Probe {
    from_dpi: u16,
    to_dpi: u16,
    before: Vec<S2Row>,
    after: Vec<S2Row>,
    /// 変化後の DPI 相が実際に報告源を引いた target 群（非空虚性の検査用）。
    refresh_targets: Vec<u32>,
}

/// 全 char 窓の観測行を scope 昇順で取る。
fn s2_rows(world: &World, gw: &GhostWindows, scopes: &[usize]) -> Vec<S2Row> {
    scopes
        .iter()
        .map(|&scope| {
            let e = gw.char_window(scope).expect("char 窓がある");
            S2Row {
                scope,
                ground: s2_ground_point(world, e),
                wa_bottom: s2_resolved_work_area(world, e).bottom,
            }
        })
        .collect()
}

/// **S2 探針**: `from_dpi` の work area へ接地した合成マルチモニタ World に対し、
/// OS 側の拡大率変更（`to_dpi`）を注入して DPI 相を 1 回回す。
///
/// 偽ウィンドウハンドルのヘッドレス World（[`dpi_world`]＝2 scope・偽 HWND・書込 witness）・
/// 合成マルチモニタ（[`s2_snapshot`]）・**「再導出結果なし」固定の偽寸法報告源**
/// （[`FakeReports`] の空マップ＝`refresh_scale_report` が常に `None`）で組む。実 GPU・実高 DPI
/// モニタを要さず決定論（Req 5.2）。
///
/// 手順:
/// 1. `from_dpi` の snapshot を挿し、全 char 窓を当該 work area 下端へ接地させる
///    （随伴バルーンも同量ずらして追従 offset を保つ）。
/// 2. `from_dpi` で DPI 相を 1 回回して `SystemState::new` の初回全窓マッチを消費する
///    （既に接地済みゆえ**是正後もべき等 skip で書込ゼロ**＝この run は探針を汚さない）。
/// 3. snapshot を `to_dpi` のものへ差し替え、全窓の `DPI` を `to_dpi` へ更新する
///    （`Changed<DPI>` エッジ＝OS の拡大率変更に対応する）。
/// 4. DPI 相をもう 1 回回し、変化の前後の接地点を突き合わせる。
fn run_s2_probe(from_dpi: u16, to_dpi: u16) -> S2Probe {
    let (mut world, gw) = dpi_world();
    let scopes: Vec<usize> = gw.scopes().collect();

    // --- 1. 変化前: from_dpi の work area と、そこへ接地した char 窓 ---
    world.insert_resource(s2_snapshot(from_dpi));
    let from_bottom = s2_work_area_for_dpi(from_dpi).bottom;
    for &scope in &scopes {
        let char_e = gw.char_window(scope).expect("char 窓がある");
        let balloon_e = gw.balloon_window(scope).expect("balloon 窓がある");
        let h = size_of(&world, char_e).expect("char 寸がある").height;
        let y = pos_of(&world, char_e).expect("char 位置がある").y;
        let dy = (from_bottom - h) - y;
        s2_shift_y(&mut world, char_e, dy);
        s2_shift_y(&mut world, balloon_e, dy);
        for e in [char_e, balloon_e] {
            world.entity_mut(e).insert(DPI::from_dpi(from_dpi, from_dpi));
        }
    }

    // --- 2. 「再導出結果なし」固定の報告源で初回 run を消費する ---
    let mut source = FakeReports::default();
    let mut state = None;
    dpi_phase_with(&mut source, &mut state, &mut world);
    let before = s2_rows(&world, &gw, &scopes);

    // --- 3. 変化: OS 側の拡大率変更（work area 下端が動く）＋窓 DPI の更新 ---
    world.insert_resource(s2_snapshot(to_dpi));
    for &scope in &scopes {
        for e in [
            gw.char_window(scope).expect("char 窓がある"),
            gw.balloon_window(scope).expect("balloon 窓がある"),
        ] {
            world.entity_mut(e).insert(DPI::from_dpi(to_dpi, to_dpi));
        }
    }
    reset_write_witness(&mut world, &gw);
    source.calls.clear();

    // --- 4. DPI 相をもう 1 回 ---
    dpi_phase_with(&mut source, &mut state, &mut world);
    let after = s2_rows(&world, &gw, &scopes);

    S2Probe {
        from_dpi,
        to_dpi,
        before,
        after,
        refresh_targets: source.calls_of("refresh"),
    }
}

/// **本檻の判定＝接地点の不変条件**（Req 5.6: 絶対 px の固定値ではなく不変条件で表現する）。
///
/// - (1) 探針の前提: 変化**前**の接地点 Y がそのときの work area 下端と一致している。
/// - (2) 接地点の **X 成分**（下端中央の x）が変化の前後で保存される。
/// - (3) 接地点の **Y 成分**が「今いるモニタの work area 下端」＝接地規約の値であり続ける。
///
/// 「接地点を保つ」（Req 4.1）とは絶対座標の凍結ではない——足元の中心 x を保ったまま、
/// 足元が**その時点の work area 下端に接し続ける**ことである（design D7:「`project_anchor`
/// が Y を新モニタ work area 下端へ再導出」）。work area が動いた走行で旧 Y が据え置かれる
/// のは「保った」のではなく、タスクバーの下へ潜り込んだ状態である。
fn s2_assert_ground_point_invariant(probe: &S2Probe) {
    assert!(
        !probe.refresh_targets.is_empty(),
        "非空虚性: 変化後の DPI 相が報告源を一度も引いていない（Changed<DPI> が発火していない＝探針の組み違い）"
    );
    assert_eq!(
        probe.before.len(),
        probe.after.len(),
        "前後で観測窓数が違う（探針の組み違い）"
    );
    for (b, a) in probe.before.iter().zip(&probe.after) {
        assert_eq!(
            b.ground.1, b.wa_bottom,
            "探針の前提: 変化前の char 窓 scope={} は work area 下端へ接地しているはず（before={b:?}）",
            b.scope
        );
        assert_eq!(
            a.ground.0, b.ground.0,
            "接地点の X 成分（下端中央）が dpi {}→{} で保存されていない: scope={} before={b:?} after={a:?}",
            probe.from_dpi, probe.to_dpi, a.scope
        );
        assert_eq!(
            a.ground.1, a.wa_bottom,
            "dpi {}→{}: 接地点 Y が変化後の work area 下端から外れている（work area が動いたのに位置が再射影されていない＝S2・Req 4.1/4.2/4.6）: scope={} before={b:?} after={a:?}",
            probe.from_dpi, probe.to_dpi, a.scope
        );
    }
}

/// **S2 赤証跡（96 水準・是正前でも通過する）**: 拡大率が 96 のままなら work area 下端が
/// 動かず、旧 Y と「新 work area 下端 − h」が自己整合する——ゆえに**再射影の欠落が観測
/// されない**。本件は是正の前後いずれでも緑であり、下の 120／192 の 3 件との**非対称**
/// そのものが「96 の自己整合が欠陥を隠す」性質の記録である（Req 5.1／5.4）。
///
/// 5.2 はこの 1 件も無視属性を外して常時走らせている——4 件が揃って初めて非対称が回帰檻
/// として保存される。
#[test]
fn s2_red_ground_point_preserved_at_dpi96() {
    let probe = run_s2_probe(96, 96);
    assert_eq!(
        probe.before[0].wa_bottom, probe.after[0].wa_bottom,
        "96 水準では work area 下端が動かない（＝本件が是正前でも通過する理由そのもの）"
    );
    s2_assert_ground_point_invariant(&probe);
}

/// **S2 赤証跡（96→120）**: work area 下端が動いたのに位置が再射影されず、接地点 Y が
/// 旧下端に据え置かれる（＝タスクバーの下へ潜り込む）。
#[test]
fn s2_red_ground_point_preserved_from_dpi96_to_dpi120() {
    s2_assert_work_area_bottom_moves(96, 120);
    s2_assert_ground_point_invariant(&run_s2_probe(96, 120));
}

/// **S2 赤証跡（96→192）**: 同上（k=2/1 相当・下端の変位が最大の水準）。
#[test]
fn s2_red_ground_point_preserved_from_dpi96_to_dpi192() {
    s2_assert_work_area_bottom_moves(96, 192);
    s2_assert_ground_point_invariant(&run_s2_probe(96, 192));
}

/// **S2 赤証跡（120→192）**: 起点が 96 でない遷移でも同じ欠落が起きる（96 が特別なのは
/// 「自己整合して隠す」からであって、96 起点だけの問題ではない）。
#[test]
fn s2_red_ground_point_preserved_from_dpi120_to_dpi192() {
    s2_assert_work_area_bottom_moves(120, 192);
    s2_assert_ground_point_invariant(&run_s2_probe(120, 192));
}

/// **常時走る随伴 (1)・非退行の対照**: 寸の再導出結果が**得られる**（`Some`）経路は S2 の
/// 対象外であり、タスク 5.2 の是正後も**一切変わってはならない**。DPI 相は従来どおり
/// [`reconcile_window_size`] を通り、char 窓は新寸で接地規約へ再射影され、随伴バルーンは
/// 追従恒等式（`balloon 位置 − char 位置 ≡ BalloonFollow.offset`）を保ち（Req 4.4）、
/// 経路語は `DpiReproject` のままである（D13）。
///
/// 5.2 が `Some` 経路まで作り替えると本件が赤になる。
#[test]
fn s2_control_some_report_path_reprojects_and_keeps_balloon_offset() {
    let (mut world, gw) = dpi_world();
    world.insert_resource(s2_snapshot(96));
    let char0 = gw.char_window(0).expect("char 窓がある");
    let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");
    let native = size_of(&world, char0).expect("char 寸がある");
    let ground_before = s2_ground_point(&world, char0);
    assert_eq!(
        ground_before.1,
        s2_work_area_for_dpi(96).bottom,
        "前提: 変化前は 96 の work area 下端へ接地している"
    );

    // 96→120（k=5/4）: work area 下端が動き、報告源も新物理寸を返す。
    s2_assert_work_area_bottom_moves(96, 120);
    world.insert_resource(s2_snapshot(120));
    for e in [char0, balloon0] {
        world.entity_mut(e).insert(DPI::from_dpi(120, 120));
    }
    let scaled = ScaleRatio::new(120, 96)
        .expect("非ゼロ比")
        .scaled_extent(native.width as u32, native.height as u32);
    let mut source = FakeReports::default();
    source.refresh.insert(shell_target(0).0, scaled);
    let mut state = None;
    let (_, events) =
        capture_diag_logs(|| dpi_phase_with(&mut source, &mut state, &mut world));

    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(scaled.0 as i32, scaled.1 as i32)),
        "Some 経路: 報告された新物理寸へ reconcile される"
    );
    assert_eq!(
        s2_ground_point(&world, char0),
        (ground_before.0, s2_work_area_for_dpi(120).bottom),
        "Some 経路: 接地点の X が保存され Y が変化後の work area 下端へ再射影される"
    );
    // 随伴恒等式（Req 4.4）: offset は寸法変動に伴い付け替えられるが、恒等式自体は保たれる。
    let offset = world
        .get::<BalloonFollow>(char0)
        .expect("char 窓は BalloonFollow を持つ")
        .offset;
    let cp = pos_of(&world, char0).expect("char 位置がある");
    let bp = pos_of(&world, balloon0).expect("balloon 位置がある");
    assert_eq!(
        (bp.x - cp.x, bp.y - cp.y),
        (offset.x, offset.y),
        "随伴恒等式 balloon − char ≡ BalloonFollow.offset が崩れている（Req 4.4）"
    );
    assert_eq!(
        window_move_routes_of(&events, char0),
        vec!["DpiReproject"],
        "Some 経路の route は DpiReproject のまま: {:?}",
        window_move_lines(&events)
    );
}

// ── task 7.2: 再導出結果が得られた経路の非退行を混在 DPI 全水準へ拡充 ──
//
// 上の `s2_control_*` は `Some` 経路の非退行を **96→120・scope 0** の 1 点でしか
// 見ておらず、しかも随伴の主張が「**書込後**に world から読んだ `offset`」に対する
// `balloon − char ≡ offset` である。この式は [`follow_balloon`] が
// 「バルーン位置 ← キャラ位置 ＋ offset」と書いていることの**恒真の言い換え**であり、
// `offset` が書込の途中で付け替えられても成立し続ける——[[5.2 の教訓＝空虚性
// 6 例目「不動点」型]]／[[7.2 の空虚性 8 例目＝「恒等式を、それを作った当人に問う」型]]
// の配置である。
//
// 本檻は同じ恒等式を**空虚でない形**へ書き直す。すなわち `offset` を書込の**前**に
// 読み、書込の**後**に
//   (a) `BalloonFollow.offset` の値自体が 1 bit も変わっていないこと
//   (b) `balloon − char` が**その前読み値**と一致すること
// を主張する。offset を付け替える実装（かつて Bottom だけに存在し、2026-07-31 の
// 実機 SSP 裁定で欠陥と確定して撤去された「原点＝下端中央基準への付替え」）は
// (a) と (b) の両方を落とす。正典は窓（char 左上）相対＝
// `balloon_pos − char_pos ≡ offset` が**全アンカーで不変**であること
// （`.kiro/steering/roadmap.md`「DPI 追従が基本設計」・
//   檻 `resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset`）。
//
// 併せて 96/120/192 の 3 遷移 × 全 scope へ拡充する。判定は絶対 px ではなく
// 「変化の前後で保存される差分ベクトル」＝不変条件である（Req 5.6）。
//
// 非空虚性の自己検査を 3 段で持つ:
//   (1) work area 下端が実際に動く（[`s2_assert_work_area_bottom_moves`]）
//   (2) 報告寸が現寸と**異なる**＝`Some` 経路が実際に reconcile を走らせる
//   (3) バルーンの**絶対位置は動く**＝「相対不変」が「何も起きなかった」の
//       言い換えに退化していない

/// 追従 offset（`BalloonFollow.offset`）を読む。
///
/// **書込の前**に読んだ値と**後**の状態を突き合わせるために使う——後読み値だけで
/// 恒等式を問うと、恒等式を作った当人に問い返す恒真形になる（上のコメント参照）。
fn s2_follow_offset(world: &World, char_e: Entity) -> (i32, i32) {
    let offset = world
        .get::<BalloonFollow>(char_e)
        .expect("char 窓は BalloonFollow を持つ")
        .offset;
    (offset.x, offset.y)
}

/// **task 7.2**: 寸の再導出結果が**得られる**（`Some`）経路の非退行を、混在 DPI の
/// 3 遷移（96→120・96→192・120→192）× 全 scope で固定する。
///
/// 主張は 4 つ:
/// - 従来経路が走る（報告された新物理寸へ `reconcile_window_size` が反映する）
/// - 接地点の X が保存され、Y は**変化後の** work area 下端へ再射影される（Req 4.1/4.2）
/// - **窓相対の追従 offset が値ごと不変で、バルーンはその前読み値どおりに追従する**
///   （Req 4.4 の非恒真形・2026-07-31 実機 SSP 裁定の契約）
/// - 経路語は `DpiReproject` のまま（D13）
#[test]
fn s2_some_report_path_preserves_the_balloon_ground_anchor_across_mixed_dpi_levels() {
    for (from_dpi, to_dpi) in [(96u16, 120u16), (96, 192), (120, 192)] {
        // (1) 非空虚性: この 2 水準のあいだで work area 下端が実際に動く。
        s2_assert_work_area_bottom_moves(from_dpi, to_dpi);

        let (mut world, gw) = dpi_world();
        let scopes: Vec<usize> = gw.scopes().collect();
        assert!(
            scopes.len() >= 2,
            "探針の退化: 複数 scope でなければ「全 scope で保存」は主張になっていない"
        );

        // --- 変化前: from_dpi の work area へ全 char 窓を接地させる ---
        world.insert_resource(s2_snapshot(from_dpi));
        let from_bottom = s2_work_area_for_dpi(from_dpi).bottom;
        for &scope in &scopes {
            let char_e = gw.char_window(scope).expect("char 窓がある");
            let balloon_e = gw.balloon_window(scope).expect("balloon 窓がある");
            let h = size_of(&world, char_e).expect("char 寸がある").height;
            let y = pos_of(&world, char_e).expect("char 位置がある").y;
            let dy = (from_bottom - h) - y;
            s2_shift_y(&mut world, char_e, dy);
            s2_shift_y(&mut world, balloon_e, dy);
            for e in [char_e, balloon_e] {
                world.entity_mut(e).insert(DPI::from_dpi(from_dpi, from_dpi));
            }
        }

        // `SystemState::new` の初回全窓マッチを「報告なし」で消費する（既に接地済み
        // ＝べき等 skip で書込ゼロ＝以降の観測を汚さない）。
        let mut source = FakeReports::default();
        let mut state = None;
        dpi_phase_with(&mut source, &mut state, &mut world);

        // --- 変化前の観測 ---
        let before: Vec<((i32, i32), (i32, i32), (i32, i32), SizeI)> = scopes
            .iter()
            .map(|&scope| {
                let char_e = gw.char_window(scope).expect("char 窓がある");
                let balloon_e = gw.balloon_window(scope).expect("balloon 窓がある");
                let bp = pos_of(&world, balloon_e).expect("balloon 位置がある");
                (
                    s2_ground_point(&world, char_e),
                    // **書込の前**に読む追従 offset（恒真化の回避・上のコメント参照）。
                    s2_follow_offset(&world, char_e),
                    (bp.x, bp.y),
                    size_of(&world, char_e).expect("char 寸がある"),
                )
            })
            .collect();
        for (i, &scope) in scopes.iter().enumerate() {
            assert_eq!(
                before[i].0.1, from_bottom,
                "探針の前提: scope={scope} は変化前 work area 下端へ接地している"
            );
        }

        // --- 変化: work area・DPI・報告寸をまとめて to_dpi 相当へ ---
        world.insert_resource(s2_snapshot(to_dpi));
        let ratio = ScaleRatio::new(to_dpi.into(), from_dpi.into()).expect("非ゼロ比");
        let mut source = FakeReports::default();
        for (i, &scope) in scopes.iter().enumerate() {
            for e in [
                gw.char_window(scope).expect("char 窓がある"),
                gw.balloon_window(scope).expect("balloon 窓がある"),
            ] {
                world.entity_mut(e).insert(DPI::from_dpi(to_dpi, to_dpi));
            }
            let native = before[i].3;
            let scaled = ratio.scaled_extent(native.width as u32, native.height as u32);
            // (2) 非空虚性: 報告寸が現寸と違う＝`Some` 経路が実際に reconcile を走らせる。
            assert_ne!(
                SizeI::new(scaled.0 as i32, scaled.1 as i32),
                native,
                "探針が不動点: dpi {from_dpi}→{to_dpi} scope={scope} で報告寸が現寸と同じ"
            );
            source.refresh.insert(shell_target(scope as u32).0, scaled);
        }
        reset_write_witness(&mut world, &gw);

        let (_, events) =
            capture_diag_logs(|| dpi_phase_with(&mut source, &mut state, &mut world));

        // --- 判定 ---
        let to_bottom = s2_work_area_for_dpi(to_dpi).bottom;
        for (i, &scope) in scopes.iter().enumerate() {
            let char_e = gw.char_window(scope).expect("char 窓がある");
            let balloon_e = gw.balloon_window(scope).expect("balloon 窓がある");
            let native = before[i].3;
            let scaled = ratio.scaled_extent(native.width as u32, native.height as u32);

            assert_eq!(
                size_of(&world, char_e),
                Some(SizeI::new(scaled.0 as i32, scaled.1 as i32)),
                "dpi {from_dpi}→{to_dpi} scope={scope}: 従来経路（Some）が新物理寸を反映していない"
            );
            let ground_after = s2_ground_point(&world, char_e);
            assert_eq!(
                ground_after.0, before[i].0.0,
                "dpi {from_dpi}→{to_dpi} scope={scope}: 接地点の X が保存されていない"
            );
            assert_eq!(
                ground_after.1, to_bottom,
                "dpi {from_dpi}→{to_dpi} scope={scope}: 接地点 Y が変化後の work area 下端でない"
            );

            // (3) 非空虚性: バルーンの**絶対位置は動く**（相対不変が「無変化」の
            //     言い換えではないことの witness）。
            let bp = pos_of(&world, balloon_e).expect("balloon 位置がある");
            assert_ne!(
                (bp.x, bp.y),
                before[i].2,
                "探針が不動点: dpi {from_dpi}→{to_dpi} scope={scope} でバルーンが 1 bit も動かない\
                 （『相対が保たれた』が『何も起きなかった』と区別できない）"
            );

            // 本題 (a)（Req 4.4）: 追従 offset は**値ごと**不変（窓相対契約）。
            // 寸法・DPI・work area がまとめて変わっても 1 bit も書き換わらない
            // ——Bottom だけを原点（下端中央）基準へ付け替える実装はここで落ちる。
            assert_eq!(
                s2_follow_offset(&world, char_e),
                before[i].1,
                "dpi {from_dpi}→{to_dpi} scope={scope}: BalloonFollow.offset が書き換わった\
                 （窓相対契約＝リサイズで offset を補正しない・2026-07-31 実機 SSP 裁定）"
            );
            // 本題 (b): バルーンは**前読み**の offset どおりに追従している。
            // 比較相手が前読み値ゆえ、付替えが起きればここも同時に落ちる（恒真でない）。
            let cp = pos_of(&world, char_e).expect("char 位置がある");
            assert_eq!(
                (bp.x - cp.x, bp.y - cp.y),
                before[i].1,
                "dpi {from_dpi}→{to_dpi} scope={scope}: 追従恒等式 balloon − char ≡ offset\
                 （書込**前**に読んだ offset）が崩れている・Req 4.4"
            );

            assert_eq!(
                window_move_routes_of(&events, char_e),
                vec!["DpiReproject"],
                "dpi {from_dpi}→{to_dpi} scope={scope}: 経路語が DpiReproject でない: {:?}",
                window_move_lines(&events)
            );
        }
    }
}

/// **常時走る随伴 (2)・5.2 の実装違いを捕まえる前方ガード**: DPI 相の書込は
/// **現位置が接地点規約に違反しているときだけ**起きなければならない（design「dpi_phase
/// 位置/寸分離 > Risks / Req 4.5 との整合」）。
///
/// 5.2 が `None` 経路の再射影を「常に書く」形で実装すると、DPI 通知のたびに同値の再配置が
/// 走り Req 4.5（再導出結果が得られないなら現状維持）が壊れる——本件はそれを赤で捕まえる。
///
/// 「書込ゼロ」の主張が空虚にならないよう、**同一ハーネスが書込を検出できること**を先に
/// positive witness で示す（記憶〈3.2 の空虚性・2 例目〉＝witness が壊れていても通って
/// しまう檻にしない）。
#[test]
fn s2_dpi_phase_writes_nothing_when_the_ground_point_already_holds() {
    // --- positive witness: 同一ハーネスは書込を実際に検出できる ---
    {
        let (mut world, gw) = dpi_world();
        world.insert_resource(s2_snapshot(96));
        let char0 = gw.char_window(0).expect("char 窓がある");
        let native = size_of(&world, char0).expect("char 寸がある");
        world.entity_mut(char0).insert(DPI::from_dpi(120, 120));
        let mut source = FakeReports::default();
        source.refresh.insert(
            shell_target(0).0,
            ScaleRatio::new(120, 96)
                .expect("非ゼロ比")
                .scaled_extent(native.width as u32, native.height as u32),
        );
        let mut state = None;
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert_ne!(
            arrangement_offset_of(&world, char0),
            WRITER_WITNESS,
            "positive witness: 異寸報告のある DPI 相は実際に窓へ書く（書込 witness が生きている証拠）"
        );
    }

    // --- 本題: work area が動かず既に接地している走行では書込ゼロ（Req 4.5 現状維持）---
    let (mut world, gw) = dpi_world();
    world.insert_resource(s2_snapshot(96));
    let char0 = gw.char_window(0).expect("char 窓がある");
    let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");
    assert_eq!(
        s2_ground_point(&world, char0).1,
        s2_work_area_for_dpi(96).bottom,
        "前提: 既に 96 の work area 下端へ接地している"
    );

    let mut source = FakeReports::default(); // 「再導出結果なし」固定
    let mut state = None;
    dpi_phase_with(&mut source, &mut state, &mut world); // 初回 run（全窓マッチ）を消費
    reset_write_witness(&mut world, &gw);
    source.calls.clear();

    // work area は不変のまま `Changed<DPI>` だけを立てる（同一水準の DPI 通知）。
    world.entity_mut(char0).insert(DPI::from_dpi(96, 96));
    dpi_phase_with(&mut source, &mut state, &mut world);

    assert!(
        source.calls_of("refresh").contains(&shell_target(0).0),
        "非空虚性: DPI 相は当該窓を実際に訪れている（訪れずに書かなかったのでは檻が空虚）: {:?}",
        source.calls
    );
    assert_no_write(
        &world,
        char0,
        "接地済み・work area 不変の DPI 相（Req 4.5 現状維持）",
    );
    assert_no_write(
        &world,
        balloon0,
        "接地済み・work area 不変の DPI 相（バルーンは位置据置き）",
    );
}
