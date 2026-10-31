// =============================================================================
// shell 設定（`seriko.zorder`）由来の基底の起動時適用（task 6.3・要件 5.1〜5.4）の
// 決定論テスト
//
// この層が負うのは 3 つだけである——「タグと**同一の**解釈を通すこと」「1 つの設定を
// 1 つの基底として据えること」「解釈できないときは 1 本も載せず、受け取った値と理由を
// 残して起動を続けること」。
//
// 檻は毎回**両側から**挟む。「据わる」の隣に「据わらない」を、「記録が出る」の隣に
// 「記録が出ない」を置く（1.2 の差し戻しの教訓）。片側だけの主張は、経路そのものが
// 死んでいても緑のままになる。
//
// 「同一の解釈」は**逐語一致**で押さえる（要件 5.2）。似た結果を並べるだけでは、
// こちらに 2 つ目の解釈器が生えた日に気づけない——タグが通す純関数の戻り値そのものと
// 突き合わせる。
// =============================================================================

use super::*;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::channel;

use wintf::ecs::window::ZOrderChainPlan;

use crate::emo2_boot::frame::run_zorder_drain_phase;
use crate::emo2_boot::frame::test_support::{headless_wiring_with, zero_clock};
use crate::emo2_boot::zorder_cue::ZOrderDirective;
use crate::placement::follow::OffsetBase;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};
use crate::placement::zorder_group_ledger::{GroupElement, GroupWindowKind, ZOrderGroup};

// ---------------------------------------------------------------- 道具立て

/// 受理・拒否の記録の出力先（実機サインオフの grep 対象と同じ 1 本）。
///
/// 要件 9.5 の保全対象である `[zorder-group] applied`／`rejected` は、退役する
/// `zorder_group` 系から `zorder_chain_diag` へ移設された。**タグの字面は 1 字も
/// 変わっておらず**、変わったのは `tracing` の出力先（module path 既定）だけである。
/// サインオフの `RUST_LOG` は `wintf::ecs::window::zorder_chain=debug` を含み、
/// 指定は前方一致なのでこの出力先を点灯させる（判定に影響しない）。
const GROUP_TARGET: &str = "target=wintf::ecs::window::zorder_chain_diag";

/// クロージャ実行中に**現在のスレッド**で発火した記録を 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち、指定した字面を含むものだけを返す。
fn lines_with<'a>(logs: &'a [String], needle: &str) -> Vec<&'a str> {
    logs.iter()
        .filter(|line| line.contains(needle))
        .map(String::as_str)
        .collect()
}

/// 要素列を省略記法（`bN`／`sN`）の並びへ（記録の `members` 欄と同じ字面）。
fn members_text(members: &[GroupElement]) -> String {
    members
        .iter()
        .map(|element| {
            let prefix = match element.kind {
                GroupWindowKind::Balloon => 'b',
                GroupWindowKind::Char => 's',
            };
            format!("{prefix}{}", element.scope)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// 台帳に据わっている基底（出所が `Descript` のグループ）を引く。
fn base_of(ledger: &ZOrderGroupLedger) -> Option<&ZOrderGroup> {
    ledger
        .groups()
        .iter()
        .find(|group| group.source == GroupSource::Descript)
}

/// 1 スコープぶんの合成配置（値は散らしただけで意味を持たない。この層は窓を動かさない）。
fn placement(scope: usize) -> ScopePlacement {
    let base = 100 * (scope as i32 + 1);
    ScopePlacement {
        scope,
        char_pos: PointPx { x: base, y: base },
        char_size: SizePx { w: 200, h: 300 },
        balloon_pos: PointPx {
            x: base + 220,
            y: base,
        },
        balloon_size: SizePx { w: 180, h: 120 },
        balloon_offset: PointPx { x: 220, y: 0 },
        balloon_offset_base: OffsetBase::unpinned(PointPx { x: 220, y: 0 }),
        balloon_limit: false,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 指定したスコープの窓だけを持つ World を組む（`GhostWindows` は Resource として載る）。
fn world_with_scopes(scopes: &[usize]) -> World {
    let mut world = World::new();
    let placements: Vec<ScopePlacement> = scopes.iter().map(|s| placement(*s)).collect();
    let titles = GhostTitles::from_scope_titles(
        scopes
            .iter()
            .map(|s| (*s, format!("scope-{s}")))
            .collect::<Vec<_>>(),
    );
    spawn_ghost_windows(&mut world, &placements, &titles);
    world
}

/// スコープのバルーン窓／キャラ窓 entity。
fn window_of(world: &World, scope: usize, kind: GroupWindowKind) -> Entity {
    let gw = world
        .get_resource::<GhostWindows>()
        .expect("GhostWindows が載っていない World で窓を引こうとした");
    match kind {
        GroupWindowKind::Balloon => gw.balloon_window(scope),
        GroupWindowKind::Char => gw.char_window(scope),
    }
    .unwrap_or_else(|| panic!("scope {scope} の窓（{kind:?}）が無い"))
}

// ---------------------------------------------------------------------------
// 据わる側／据わらない側（要件 5.1／5.3・既定＝非強制）
// ---------------------------------------------------------------------------

/// 数値モードの設定は、基底のグループ**ちょうど 1 本**として据わる（要件 5.1／5.3）。
///
/// 記録も併せて確かめる——出所が `Descript` であることが、起動由来とタグ由来を弁別する
/// 唯一の欄だからである。
#[test]
fn t_zdb01_a_numeric_setting_seats_exactly_one_descript_base() {
    let mut ledger = ZOrderGroupLedger::default();

    let logs = capture_logs(|| apply_descript_base(&mut ledger, Some("1,0")));

    assert_eq!(
        ledger.groups().len(),
        1,
        "1 つの設定が 1 つのグループになっていない（要件 5.3）"
    );
    let base = base_of(&ledger).expect("基底（出所 Descript）が据わっていない");
    assert_eq!(
        members_text(&base.members),
        "b1,s1,b0,s0",
        "数値モードの展開がタグと同じ形（各スコープ [バルーン, キャラ] の隣接ブロック）でない"
    );

    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(
        applied.len(),
        1,
        "受理の記録がちょうど 1 本ではない: {logs:?}"
    );
    let line = applied[0];
    assert!(
        line.contains(GROUP_TARGET),
        "記録の出力先が 1 本に揃っていない: {line}"
    );
    assert!(
        line.contains("source=Descript"),
        "起動由来であることが記録から読めない（タグ由来と弁別できない）: {line}"
    );
    assert!(
        line.contains("members=b1,s1,b0,s0"),
        "台帳に載った内容が記録に載っていない: {line}"
    );
}

/// 逆側——設定が無い運転では台帳も記録も 1 ミリも動かない（既定＝非強制・要件 6.1／6.4）。
///
/// 上の檻だけでは「何を渡しても 1 本据わる」形と区別が付かない。
#[test]
fn t_zdb02_no_setting_leaves_the_ledger_and_the_log_untouched() {
    let mut ledger = ZOrderGroupLedger::default();

    let logs = capture_logs(|| apply_descript_base(&mut ledger, None));

    assert!(
        ledger.groups().is_empty(),
        "設定が無いのに台帳が動いた（既定状態が壊れている）"
    );
    assert!(
        logs.is_empty(),
        "設定が無い運転で記録が出た（失敗でも見送りでもないので報せることは無い）: {logs:?}"
    );
}

// ---------------------------------------------------------------------------
// 解釈はタグと同一（要件 5.2）——逐語一致で押さえる
// ---------------------------------------------------------------------------

/// 明示モードも省略記法も、タグが通す純関数の戻り値と**逐語一致**する（要件 5.2）。
///
/// 期待値を手で書き写さずに `parse_zorder_tokens` の戻り値そのものと突き合わせるのは、
/// 「別解釈器を新設していない」を主張するためである。手書きの期待値だと、こちらに
/// 2 つ目の解釈器が生えても両方を同じに直せば緑のまま通ってしまう。
///
/// 入力に**逆転の並び**（`s1,b1,s0,b0`＝同一スコープ内でキャラ窓が手前）を 1 本
/// 混ぜてあるのは、この錨を自足させるためである。隣り合いの並びだけを与えると
/// 「バルーン先頭へ寄せ直す」型の第 2 解釈器が素通りする。
#[test]
fn t_zdb03_the_setting_is_read_by_the_very_function_the_tag_uses() {
    for raw in [
        "1,0",
        "balloon1,surface1,balloon0,surface0",
        "b1,s1,b0,s0",
        "s1,b1,s0,b0",
        "2,0,1",
    ] {
        let mut ledger = ZOrderGroupLedger::default();
        apply_descript_base(&mut ledger, Some(raw));

        let tokens: Vec<&str> = raw.split(',').collect();
        let (expected, _) = parse_zorder_tokens(&tokens)
            .unwrap_or_else(|reject| panic!("檻の前提が崩れた（{raw} が拒否された）: {reject:?}"));

        let base = base_of(&ledger).unwrap_or_else(|| panic!("{raw} で基底が据わっていない"));
        assert_eq!(
            base.members, expected,
            "設定の解釈がタグの解釈と一致しない（別解釈器が生えている）: {raw}"
        );
    }
}

/// 同一スコープ内の逆転も、タグと同じ解釈で書かれた順のまま据わり、逆転が記録に載る
/// （`areka-P0-zorder-scope-inversion` 要件 1.3・1.4 が起動の経路でも成り立つ）。
#[test]
fn t_zdb04_an_inverted_setting_is_seated_in_written_order_and_recorded() {
    let mut ledger = ZOrderGroupLedger::default();

    let logs = capture_logs(|| apply_descript_base(&mut ledger, Some("s1,b1,s0,b0")));

    let base = base_of(&ledger).expect("基底が据わっていない");
    assert_eq!(
        members_text(&base.members),
        "s1,b1,s0,b0",
        "書かれた順のまま据わっていない（バルーン先頭へ寄せ直している）"
    );

    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(applied.len(), 1, "受理の記録が 1 本ではない: {logs:?}");
    assert!(
        applied[0].contains("normalized=1:inverted,0:inverted"),
        "逆転のまま採ったことが記録に載っていない: {}",
        applied[0]
    );
}

// ---------------------------------------------------------------------------
// 解釈できない値（要件 5.4／8.1／8.3）
// ---------------------------------------------------------------------------

/// 解釈できない値は**グループを 1 本も載せず**、受け取った値と理由を warn で残す（要件 5.4）。
#[test]
fn t_zdb05_an_unreadable_setting_seats_nothing_and_records_value_and_reason() {
    let mut ledger = ZOrderGroupLedger::default();

    let logs = capture_logs(|| apply_descript_base(&mut ledger, Some("balloon1,Surface0")));

    assert!(
        ledger.groups().is_empty(),
        "解釈できない設定でグループが載った（部分適用の禁止に反する）"
    );
    assert!(
        lines_with(&logs, "[zorder-group] applied").is_empty(),
        "受け付けていないのに受理の記録が出た: {logs:?}"
    );

    let rejected = lines_with(&logs, "[zorder-group] rejected");
    assert_eq!(
        rejected.len(),
        1,
        "拒否の記録がちょうど 1 本ではない: {logs:?}"
    );
    let line = rejected[0];
    assert!(
        line.contains(GROUP_TARGET),
        "記録の出力先が 1 本に揃っていない: {line}"
    );
    assert!(
        line.contains("level=WARN"),
        "拒否の水準が warn でない（通常運転で読めなければ黙殺に等しい）: {line}"
    );
    assert!(
        line.contains("reason=UnparsableToken(Surface0)"),
        "拒否理由が読めない: {line}"
    );
    assert!(
        line.contains("tokens=balloon1,Surface0"),
        "受け取った値そのものが記録に載っていない（何を書き間違えたか復元できない）: {line}"
    );
}

/// 要素が足りない値も、空の値も、途中に紛れた誤字も、黙って落とさず理由つきで残す
/// （要件 5.4／8.3）。3 種類の落ち方を 1 本に並べるのは、**どの経路でも記録が出る**ことが
/// 主張だからである。
#[test]
fn t_zdb06_short_and_empty_settings_are_rejected_with_a_reason() {
    for (raw, reason) in [
        ("0", "reason=TooFewElements(1)"),
        ("", "reason=UnparsableToken()"),
        ("1,0,x", "reason=UnparsableToken(x)"),
    ] {
        let mut ledger = ZOrderGroupLedger::default();
        let logs = capture_logs(|| apply_descript_base(&mut ledger, Some(raw)));

        assert!(
            ledger.groups().is_empty(),
            "拒否すべき設定 {raw:?} でグループが載った"
        );
        let rejected = lines_with(&logs, "[zorder-group] rejected");
        assert_eq!(
            rejected.len(),
            1,
            "{raw:?} の拒否の記録が 1 本でない: {logs:?}"
        );
        assert!(
            rejected[0].contains(reason),
            "{raw:?} の拒否理由が読めない: {}",
            rejected[0]
        );
    }
}

/// 拒否は**既に据わっている基底を落とさない**（部分適用の禁止・要件 8.1）。
///
/// 上の 2 本は「載らない」を主張するが、それだけでは「据わっていたものを消してから
/// 載せ損ねる」形と区別が付かない。
#[test]
fn t_zdb07_a_rejected_setting_does_not_disturb_a_seated_base() {
    let mut ledger = ZOrderGroupLedger::default();
    apply_descript_base(&mut ledger, Some("1,0"));
    let seated = base_of(&ledger)
        .expect("前提の基底が据わっていない")
        .clone();

    apply_descript_base(&mut ledger, Some("balloon1,Surface0"));

    assert_eq!(
        base_of(&ledger),
        Some(&seated),
        "拒否された設定が、据わっていた基底を巻き添えにした"
    );
}

// ---------------------------------------------------------------------------
// 完了状態——最初の巡から指定どおりの相対順が成立する（要件 5.1）
// ---------------------------------------------------------------------------

/// 設定を据えた台帳は、タグの指令が**1 本も無い**最初の巡で受け口へ指定どおりの鎖を出す。
///
/// これが起動時適用の完了状態である（「タグの実行を待たない」・要件 5.1／5.3）。
/// 1 つの設定は 1 つのグループとして扱われるので、鎖の並びは設定の要素順そのものになる。
#[test]
fn t_zdb08_the_seated_base_projects_on_the_first_pass_without_any_tag() {
    let mut ledger = ZOrderGroupLedger::default();
    apply_descript_base(&mut ledger, Some("1,0"));
    assert_eq!(
        ledger.groups().len(),
        1,
        "1 つの設定が 1 つのグループとして据わっていない（要件 5.3）"
    );

    let mut world = world_with_scopes(&[0, 1]);
    let (_tx, rx) = channel::<ZOrderDirective>();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    let expected = vec![
        window_of(&world, 1, GroupWindowKind::Balloon),
        window_of(&world, 1, GroupWindowKind::Char),
        window_of(&world, 0, GroupWindowKind::Balloon),
        window_of(&world, 0, GroupWindowKind::Char),
    ];
    let receiver = world
        .get_resource::<ZOrderChainPlan>()
        .expect("最初の巡で受け口が出来ていない（基底が最初の巡から効いていない）");
    let chain = receiver
        .chain
        .as_ref()
        .expect("受け口は在るのに鎖が空（基底が射影まで届いていない）");
    assert_eq!(
        chain.members, expected,
        "鎖の窓の並びが設定どおり（手前が scope 1）でない"
    );
    assert!(
        receiver.dirty,
        "最初の巡に印が立っていない（適用系が鎖を書きに行かない）"
    );
}

/// 逆側——設定が無ければ、同じ巡で受け口の Resource すら作られない（要件 6.1）。
#[test]
fn t_zdb09_without_a_setting_the_first_pass_creates_no_receiver() {
    let mut ledger = ZOrderGroupLedger::default();
    apply_descript_base(&mut ledger, None);

    let mut world = world_with_scopes(&[0, 1]);
    let (_tx, rx) = channel::<ZOrderDirective>();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert!(
        world.get_resource::<ZOrderChainPlan>().is_none(),
        "設定が無い運転で受け口が作られた（既定＝非強制が構造で成り立っていない）"
    );
}

/// 据わった基底のスコープは、タグの再指定の拒否判定に**参加する**（要件 5.5）。
///
/// 台帳に本当に載っていることの、射影とは独立した証拠である。
#[test]
fn t_zdb10_the_seated_base_takes_part_in_the_redesignation_refusal() {
    let mut ledger = ZOrderGroupLedger::default();
    apply_descript_base(&mut ledger, Some("1,0"));

    let mut world = world_with_scopes(&[0, 1]);
    let (tx, rx) = channel::<ZOrderDirective>();
    tx.send(ZOrderDirective::Set {
        tokens: vec!["0".to_string(), "1".to_string()],
    })
    .unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert_eq!(
        ledger.groups().len(),
        1,
        "基底が押さえているスコープを指すタグが受理された（要件 5.5）"
    );
    let rejected = lines_with(&logs, "[zorder-group] rejected");
    assert_eq!(
        rejected.len(),
        1,
        "再指定の拒否が記録されていない: {logs:?}"
    );
    assert!(
        rejected[0].contains("reason=CrossGroupRedesignation(0,1)"),
        "拒否理由が「既に塞がっているスコープ」でない: {}",
        rejected[0]
    );
}

// ---------------------------------------------------------------------------
// 本番の口——結線状態の台帳へ届く
// ---------------------------------------------------------------------------

/// `Emo2Wiring::seed_zorder_descript_base` は**結線状態が持つ台帳**へ基底を据える。
///
/// 起動の結線（`wire_emo2_boot`）が触れるのはこの口だけなので、ここが台帳へ届かなければ
/// 設定は本番で一切効かない。逆側（設定なし）も同じ口で確かめる。
#[test]
fn t_zdb11_the_boot_entry_point_seats_the_base_in_the_wired_ledger() {
    let mut wiring = headless_wiring_with(channel().1, zero_clock());
    assert!(
        wiring.zorder_ledger.groups().is_empty(),
        "結線直後の台帳が既定（グループ 0 本）でない"
    );

    wiring.seed_zorder_descript_base(Some("1,0"));

    let base = base_of(&wiring.zorder_ledger)
        .expect("結線状態の台帳へ基底が届いていない（本番で設定が効かない）");
    assert_eq!(
        members_text(&base.members),
        "b1,s1,b0,s0",
        "結線状態の台帳へ届いた内容が設定どおりでない"
    );
}

/// 逆側——設定が無ければ結線状態の台帳は既定のままである。
#[test]
fn t_zdb12_the_boot_entry_point_leaves_the_ledger_default_without_a_setting() {
    let mut wiring = headless_wiring_with(channel().1, zero_clock());

    wiring.seed_zorder_descript_base(None);

    assert!(
        wiring.zorder_ledger.groups().is_empty(),
        "設定が無いのに結線状態の台帳が動いた"
    );
}

// ---------------------------------------------------------------------------
// シェルの差し替え——台本の組を保ち基底だけを取り替える
// （areka-P0-shell-balloon-switch 要件 2.7・stay-on-top-directives 要件 5.1／5.2／5.5／6.5）
// ---------------------------------------------------------------------------

/// 前のシェルの基底（ID 0・scope 0,1）と台本の組（ID 1・scope 2,3）が載った台帳を組む。
fn ledger_with_base_and_tag() -> ZOrderGroupLedger {
    let mut ledger = ZOrderGroupLedger::default();
    apply_descript_base(&mut ledger, Some("1,0"));
    let (tag, _) = parse_zorder_tokens(&["2", "3"]).expect("檻の前提: タグの値が受理される");
    ledger
        .try_add_tag_group(tag)
        .expect("檻の前提: タグ由来のグループが載る");
    assert_eq!(ledger.groups().len(), 2, "檻の前提: 基底とタグの 2 本");
    ledger
}

/// 台本の組だけを要素の字面で引く（追加の順）。
fn tag_members(ledger: &ZOrderGroupLedger) -> Vec<String> {
    ledger
        .groups()
        .iter()
        .filter(|group| group.source == GroupSource::Tag)
        .map(|group| members_text(&group.members))
        .collect()
}

/// 新しいシェルの値は基底として先頭に据わり、台本の組は残る。記録は保った組と旧→新の 1 行。
#[test]
fn t_zdb13_rebase_keeps_the_script_group_and_replaces_the_base() {
    let mut ledger = ledger_with_base_and_tag();

    let logs = capture_logs(|| apply_descript_rebase(&mut ledger, Some("0,1")));

    assert_eq!(ledger.groups().len(), 2, "基底と台本の組の 2 本でない");
    assert_eq!(
        ledger.groups()[0].source,
        GroupSource::Descript,
        "新しい基底が先頭でない"
    );
    let base = base_of(&ledger).expect("新しいシェルの基底が据わっていない");
    assert_eq!(members_text(&base.members), "b0,s0,b1,s1");
    assert_eq!(base.id, 2, "取り替えた基底に新しい ID が配られていない");
    assert_eq!(
        tag_members(&ledger),
        vec!["b2,s2,b3,s3"],
        "台本の組が保たれていない（要件 5.1）"
    );

    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(
        applied.len(),
        1,
        "受理の記録がちょうど 1 本でない: {logs:?}"
    );
    assert!(applied[0].contains(GROUP_TARGET), "{}", applied[0]);
    assert!(
        applied[0].contains("action=rebase kept=[1] base=0→2"),
        "保った組と取り替えた基底が記録に載っていない（要件 6.5）: {}",
        applied[0]
    );
    assert!(
        lines_with(&logs, "[zorder-group] rejected").is_empty(),
        "重なりが無いのに拒否の記録が出た: {logs:?}"
    );
}

/// 新しいシェルに設定が無ければ基底なしで取り替え、台本の組は残る。記録は保った組と旧→`-`。
#[test]
fn t_zdb14_rebase_without_a_setting_drops_only_the_base() {
    let mut ledger = ledger_with_base_and_tag();

    let logs = capture_logs(|| apply_descript_rebase(&mut ledger, None));

    assert!(base_of(&ledger).is_none(), "前のシェルの基底が残った");
    assert_eq!(
        tag_members(&ledger),
        vec!["b2,s2,b3,s3"],
        "台本の組が落ちた"
    );
    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(applied.len(), 1, "{logs:?}");
    assert!(
        applied[0].contains("action=rebase kept=[1] base=0→-"),
        "{}",
        applied[0]
    );
    assert!(
        lines_with(&logs, "[zorder-group] rejected").is_empty(),
        "{logs:?}"
    );
}

/// 逆側——保つものも取り替えるものも無い切り替えでは台帳も記録も動かない。
#[test]
fn t_zdb14b_rebase_with_nothing_to_keep_or_replace_records_nothing() {
    let mut ledger = ZOrderGroupLedger::default();
    let before = ledger.version();

    let logs = capture_logs(|| apply_descript_rebase(&mut ledger, None));

    assert!(ledger.groups().is_empty());
    assert_eq!(ledger.version(), before, "動いていないのに版が進んだ");
    assert!(logs.is_empty(), "報せることが無いのに記録が出た: {logs:?}");
}

/// 解釈できない値も基底なしで取り替え、拒否は起動と同じ記録で 1 本残す。台本の組は残る。
#[test]
fn t_zdb15_rebase_with_an_unreadable_setting_drops_the_base_and_records_the_rejection() {
    let mut ledger = ledger_with_base_and_tag();

    let logs = capture_logs(|| apply_descript_rebase(&mut ledger, Some("1,x")));

    assert!(
        base_of(&ledger).is_none(),
        "解釈できない値なのに前の基底が残った"
    );
    assert_eq!(
        tag_members(&ledger),
        vec!["b2,s2,b3,s3"],
        "台本の組が落ちた"
    );
    let rejected = lines_with(&logs, "[zorder-group] rejected");
    assert_eq!(rejected.len(), 1, "拒否の記録が 1 本でない: {logs:?}");
    assert!(
        rejected[0].contains("reason=UnparsableToken(x)"),
        "{}",
        rejected[0]
    );
    assert!(rejected[0].contains("tokens=1,x"), "{}", rejected[0]);
    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(applied.len(), 1, "{logs:?}");
    assert!(
        applied[0].contains("action=rebase kept=[1] base=0→-"),
        "{}",
        applied[0]
    );
}

/// 台本の組と重なる新しい基底は据えず、重なったスコープと値を拒否として残す（要件 5.2）。
#[test]
fn t_zdb16_rebase_refuses_a_base_overlapping_the_script_group() {
    let mut ledger = ledger_with_base_and_tag();

    let logs = capture_logs(|| apply_descript_rebase(&mut ledger, Some("3,0")));

    assert!(base_of(&ledger).is_none(), "台本の組と重なる基底が据わった");
    assert_eq!(
        tag_members(&ledger),
        vec!["b2,s2,b3,s3"],
        "台本の組が落ちた"
    );
    let rejected = lines_with(&logs, "[zorder-group] rejected");
    assert_eq!(
        rejected.len(),
        1,
        "重なりの拒否の記録が 1 本でない: {logs:?}"
    );
    assert!(rejected[0].contains("level=WARN"), "{}", rejected[0]);
    assert!(
        rejected[0].contains("reason=CrossGroupRedesignation(3)"),
        "理由と重なったスコープが読めない: {}",
        rejected[0]
    );
    assert!(
        rejected[0].contains("tokens=3,0"),
        "値が記録に載っていない: {}",
        rejected[0]
    );
    let applied = lines_with(&logs, "[zorder-group] applied");
    assert_eq!(applied.len(), 1, "{logs:?}");
    assert!(
        applied[0].contains("action=rebase kept=[1] base=0→-"),
        "据えなかった基底が旧→新に載った: {}",
        applied[0]
    );
}

/// 切り替えの後の `\![reset,zorder]` で台本の組が落ち、据え損ねた新しい基底へ戻る（要件 5.5）。
#[test]
fn t_zdb17_reset_after_rebase_returns_to_the_new_shell_base() {
    let mut ledger = ledger_with_base_and_tag();
    apply_descript_rebase(&mut ledger, Some("3,0"));
    assert!(
        base_of(&ledger).is_none(),
        "檻の前提: 重なる基底は据わっていない"
    );

    let mut world = world_with_scopes(&[0, 3]);
    let (tx, rx) = channel::<ZOrderDirective>();
    tx.send(ZOrderDirective::Reset).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        ledger.groups().len(),
        1,
        "reset の後が新しい基底 1 本でない"
    );
    let base = base_of(&ledger).expect("reset で新しいシェルの基底へ戻っていない");
    assert_eq!(members_text(&base.members), "b3,s3,b0,s0");
    let chain = world
        .get_resource::<ZOrderChainPlan>()
        .and_then(|plan| plan.chain.as_ref())
        .expect("新しい基底が射影まで届いていない");
    assert_eq!(
        chain.members,
        vec![
            window_of(&world, 3, GroupWindowKind::Balloon),
            window_of(&world, 3, GroupWindowKind::Char),
            window_of(&world, 0, GroupWindowKind::Balloon),
            window_of(&world, 0, GroupWindowKind::Char),
        ]
    );
}

/// 重なりの無い切り替えの後の reset も、取り替えた新しい基底へ戻る（要件 5.5）。
#[test]
fn t_zdb18_reset_after_a_clean_rebase_keeps_the_new_base() {
    let mut ledger = ledger_with_base_and_tag();
    apply_descript_rebase(&mut ledger, Some("0,1"));

    ledger.reset_to_descript();

    assert_eq!(ledger.groups().len(), 1);
    let base = base_of(&ledger).expect("新しい基底が残っていない");
    assert_eq!(members_text(&base.members), "b0,s0,b1,s1");
}
