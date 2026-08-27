//! 検証段・グループごとの頭打ち・印の解除に対する決定論的テスト（要件 8.2／8.3／9.1／9.2）。
//!
//! 実機も実ディスプレイも要らない——窓の実体は空の `World` から採った `Entity` の値、
//! ハンドルは Win32 へ渡さない偽の `HWND`、前面走査は手で組んだ値である。測る道具
//! （[`FakeProbe`]）は兄弟の [`zorder_group_maintain_tests`](super::zorder_group_maintain_tests)
//! と**同一のもの**を借りている——同じ経路を測る道具を 2 つ作ると、片方だけが較正から
//! 外れても誰も気づかない。
//!
//! # 本ファイルが固定するもの
//!
//! - **証跡は次の巡でしか採れない**——是正の記録（`fix`）は発行と同じ巡には出ず、
//!   次の巡の実測で初めて出る（指令の書込は巡の後の flush であり、同巡の実測は必ず
//!   書込前の値だから）。
//! - **測れなかった巡を証跡にしない**——検証の相手が 2 枚未満に減った巡は、成立とも
//!   不成立とも記録せず、理由つきの見送りになる。
//! - **打ち切りを「居なかった」と混ぜない**——検証不一致の行は走査が最前面まで辿れたか
//!   （`scan_complete`）を同じ 1 行に載せる。
//! - **頭打ちはグループごと**——連続失敗が上限に達したグループ**だけ**が維持対象から
//!   外れ、他のグループの是正は流れ続ける。外す事実は warn として残る（黙って諦めない）。
//! - **印の解除**——維持対象の全グループの相対順が成立した時点で降り、1 つでも崩れて
//!   いれば降りない。外したグループは判定に加えない。
//!
//! # 片側だけの主張をしない
//!
//! 「出ない」には必ず**出る**巡を、「降りない」には**降りる**巡を、「数えない」には
//! **数える**側を、同じテストの中に併置してある（本 spec の先行タスクは、入力が片側に
//! 偏った檻が変異体を素通りさせる事故を繰り返している）。

use super::run_group_maintenance_pass;
use super::zorder_group_maintain_tests::{
    FakeProbe, clear_queue, entities, fake_hwnd, groups_with, issued_targets,
};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{ZOrderGroupSpec, ZOrderGroups, drain_window_pos_commands};

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::HWND;

/// 実機サインオフが用いる `RUST_LOG` 相当（グループ系の出力先を点灯させる指定）。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_group=debug";

/// 既定水準（診断手順を有効化していない通常運転）。
const DEFAULT_DIRECTIVES: &str = "info";

/// 捕捉した出力から、指定タグを含む行をちょうど 1 本取り出す。
fn only_line_with<'a>(out: &'a str, tag: &str) -> &'a str {
    let found: Vec<&str> = out.lines().filter(|l| l.contains(tag)).collect();
    assert_eq!(found.len(), 1, "`{tag}` の行がちょうど 1 本ではない: {out}");
    found[0]
}

/// 指定タグを含む行の本数。
fn count_lines_with(out: &str, tag: &str) -> usize {
    out.lines().filter(|l| l.contains(tag)).count()
}

/// 2 枚組のグループを 1 本ぶん組む道具（実体・ハンドル・受け口）。
struct Pair {
    members: Vec<Entity>,
    hwnds: [HWND; 2],
}

/// 2 枚組を `count` 本ぶん組む（実体は**ひとつの** `World` からまとめて採る）。
///
/// まとめて採るのが要点である——空の `World` を組ごとに作ると `Entity` の値が組をまたいで
/// 衝突し（どの World も同じ番号から配る）、ハンドルの対応表が後の組で上書きされる。
/// テストが測っているつもりの窓と、実測の口が実際に引く窓が静かにずれる形なので、
/// 組の作り方そのものでこれを塞ぐ。`base` は記録に現れる 16 進の頭であり、組ごとに
/// 0x100 ずつずらして重ならないようにしてある。
fn pairs(count: usize, base: usize) -> Vec<Pair> {
    let members = entities(2 * count);
    (0..count)
        .map(|i| Pair {
            members: members[2 * i..2 * i + 2].to_vec(),
            hwnds: [
                fake_hwnd(base + 0x100 * i + 1),
                fake_hwnd(base + 0x100 * i + 2),
            ],
        })
        .collect()
}

/// 2 枚組をちょうど 1 本だけ組む。
fn one_pair(base: usize) -> Pair {
    pairs(1, base).remove(0)
}

impl Pair {
    /// この組の宣言（手前から順）。
    fn spec(&self, id: u32) -> ZOrderGroupSpec {
        ZOrderGroupSpec {
            id,
            members: self.members.clone(),
        }
    }

    /// 実測の口へこの組を仕込む（`ordered` が真なら宣言どおりの重なり）。
    fn teach(&self, probe: FakeProbe, ordered: bool, reached_top: bool) -> FakeProbe {
        let front: &[HWND] = if ordered { &self.hwnds[..1] } else { &[] };
        probe
            .with_handles(&self.members, &self.hwnds)
            .with_front(self.hwnds[1], front, reached_top)
    }
}

/// 1 巡を回して、記録と積まれた指令を返す。
fn pass(groups: &mut ZOrderGroups, probe: &FakeProbe) -> (String, Vec<HWND>) {
    clear_queue();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        run_group_maintenance_pass(groups, false, probe);
    });
    (out, issued_targets(&drain_window_pos_commands()))
}

// ===========================================================================
// 検証は次の巡（要件 9.1／9.2）
// ===========================================================================

/// 是正の記録は**発行の巡には出ず、次の巡の実測で出る**。
///
/// 指令の書込は巡の後の flush なので、発行と同じ巡に採った実測は必ず書込前の値であり
/// 証跡にならない。片側（次巡で出る）だけを見ると「同巡でも出す」実装が緑のまま通るので、
/// 発行の巡で**出ていない**ことを対にして主張する。
#[test]
fn the_issuing_pass_records_no_fix_and_the_next_pass_does() {
    let pair = one_pair(0x4100);
    let mut groups = groups_with(11, &pair.members);

    let broken = pair.teach(FakeProbe::new(), false, true);
    let (issuing, issued) = pass(&mut groups, &broken);

    assert_eq!(
        issued,
        vec![pair.hwnds[1]],
        "崩れたグループへ連鎖が出ていない（発行そのものが死んでいる疑い）"
    );
    assert!(
        !issuing.contains("[zorder-group] fix"),
        "発行と同じ巡の実測が是正の証跡として記録された（書込前の値である）: {issuing}"
    );
    assert!(
        groups.has_verify(),
        "発行した巡に検証待ちが預けられていない"
    );
    assert!(groups.pending, "崩れたままの巡で印が降りている");

    // 次の巡: 指令が効いた
    let settled = pair.teach(FakeProbe::new(), true, true);
    let (verifying, after) = pass(&mut groups, &settled);

    let line = only_line_with(&verifying, "[zorder-group] fix");
    assert!(line.contains("group_id=11"), "{line}");
    assert!(line.contains("head=0x4101"), "{line}");
    assert!(line.contains("moves=0x4102@0x4101"), "{line}");
    assert!(
        line.contains("measured=0x4101,0x4102"),
        "実測の並びが載っていない: {line}"
    );
    assert!(
        after.is_empty(),
        "相対順が成立した巡に指令が積まれた（同値ガードが効いていない）"
    );
    assert!(
        !groups.has_verify(),
        "検証を終えた預かりが残っている（次巡も同じ指令を検証してしまう）"
    );
    assert!(
        !groups.pending,
        "維持対象の全グループが成立したのに印が降りていない"
    );
}

/// 検証が不一致なら、是正ではなく検証不一致が error で記録され、走査の完否も同じ行に載る。
///
/// `scan_complete` は「測ったら居なかった」と「そこまで測れなかった」を分ける欄である
/// （task 2.2 からの申し送り）。真偽の**両方**を回すのは、常に真（または常に偽）を書く
/// 実装を赤にするためである。
#[test]
fn a_mismatch_carries_whether_the_scan_could_finish() {
    for reached_top in [true, false] {
        let pair = one_pair(0x4200);
        let mut groups = groups_with(12, &pair.members);

        // 発行の巡（走査は最前面まで辿れている＝ここは両回で同じ）
        let broken = pair.teach(FakeProbe::new(), false, true);
        let (_, issued) = pass(&mut groups, &broken);
        assert_eq!(issued.len(), 1, "発行そのものが起きていない");

        // 次の巡: まだ崩れている
        let still_broken = pair.teach(FakeProbe::new(), false, reached_top);
        let (out, _) = pass(&mut groups, &still_broken);

        let line = only_line_with(&out, "[zorder-group] verify-failed");
        assert!(
            line.contains("ERROR"),
            "検証不一致が error 水準でない: {line}"
        );
        assert!(line.contains("group_id=12"), "{line}");
        assert!(
            line.contains(&format!("scan_complete={reached_top}")),
            "走査の完否が行に載っていない（`{reached_top}` を期待）: {line}"
        );
        assert!(
            !out.contains("[zorder-group] fix"),
            "不一致なのに是正が記録された: {out}"
        );
        assert!(groups.pending, "崩れたままの巡で印が降りている");
    }
}

/// 検証の相手が 2 枚未満に減った巡は、**成立とも不成立とも記録しない**。
///
/// 実測していない巡に `fix` を出すと、サインオフの grep が「指定が成立した」と読む行を
/// 何も測らずに作れてしまう（task 2.2 で差し戻された「証跡のふりをした非証跡」と同型）。
/// 対照として、同じ発行から**窓が揃っていれば** `fix` が出ることを併置してある。
#[test]
fn a_verification_that_could_not_measure_never_claims_a_fix() {
    for windows_survive in [true, false] {
        let pair = one_pair(0x4300);
        let mut groups = groups_with(13, &pair.members);

        let broken = pair.teach(FakeProbe::new(), false, true);
        let (_, issued) = pass(&mut groups, &broken);
        assert_eq!(issued.len(), 1, "発行そのものが起きていない");

        // 次の巡: 片方の窓が消えると解決できるのは 1 枚だけ＝走査を行う理由が無い
        let probe = if windows_survive {
            pair.teach(FakeProbe::new(), true, true)
        } else {
            FakeProbe::new().with_handles(&pair.members[..1], &pair.hwnds[..1])
        };
        let (out, _) = pass(&mut groups, &probe);

        if windows_survive {
            assert!(
                out.contains("[zorder-group] fix"),
                "窓が揃っているのに是正が記録されない（対照が死んでいる）: {out}"
            );
        } else {
            assert!(
                !out.contains("[zorder-group] fix"),
                "何も測っていない巡が是正として記録された: {out}"
            );
            assert!(
                !out.contains("[zorder-group] verify-failed"),
                "何も測っていない巡が検証不一致として記録された: {out}"
            );
            assert!(
                out.contains("reason=TooFewResolved") && out.contains("group_id=13"),
                "測れなかった巡の見送りが理由つきで記録されていない（黙って諦めている）: {out}"
            );
        }
    }
}

// ===========================================================================
// グループごとの頭打ち（要件 8.2／8.3）
// ===========================================================================

/// **本タスクの完了条件**——実現できないグループが 1 つ混ざっていても、他のグループの
/// 是正は止まらず、全て成立した時点で印が降りる。
///
/// 実現できない側（グループ 1）は何度差し込んでも相対順が成立しない窓である。連続失敗が
/// 上限（3）に達した巡に warn が残って維持対象から外れ、**その同じ巡で**グループ 2 の
/// 是正が流れる。**外れる前に印が降りない**ことも併せて見る。
#[test]
fn one_unachievable_group_neither_stops_the_others_nor_drops_the_mark_early() {
    let mut built = pairs(2, 0x5100);
    let curable = built.remove(1);
    let doomed = built.remove(0);
    let mut groups = ZOrderGroups::default();
    groups.groups.push(doomed.spec(1));
    groups.groups.push(curable.spec(2));
    groups.pending = true;

    // 巡 1〜3: 実現できない側だけが発行される（検証の失敗は巡 2 と巡 3 で 2 回）。
    for round in 1..=3 {
        let probe = curable.teach(doomed.teach(FakeProbe::new(), false, true), false, true);
        let (out, issued) = pass(&mut groups, &probe);

        assert_eq!(
            issued,
            vec![doomed.hwnds[1]],
            "巡 {round}: 実現できない側への是正が出ていない"
        );
        assert!(
            groups.pending,
            "巡 {round}: 崩れたグループが残っているのに印が降りた"
        );
        assert!(
            !out.contains("reason=GaveUpAfterFailures"),
            "巡 {round}: 上限に達する前に諦めている: {out}"
        );
    }

    // 巡 4: 3 度目の検証失敗で外れる。外れたのは実現できない側**だけ**なので、
    // 治せる側の是正が同じ巡で流れる。
    let probe = curable.teach(doomed.teach(FakeProbe::new(), false, true), false, true);
    let (out, issued) = pass(&mut groups, &probe);

    let line = only_line_with(&out, "reason=GaveUpAfterFailures");
    assert!(
        line.contains("WARN"),
        "諦めの記録が warn 水準でない: {line}"
    );
    assert!(
        line.contains("group_id=1"),
        "諦めたグループが読めない: {line}"
    );
    assert!(line.contains("streak=3"), "連続失敗の数が読めない: {line}");
    assert_eq!(
        issued,
        vec![curable.hwnds[1]],
        "頭打ちが他のグループの是正まで止めている"
    );
    assert!(groups.pending, "治せる側がまだ崩れているのに印が降りた");

    // 巡 5: 治せる側が成立した。維持対象は全て成立＝印が降りる（外した側は数えない）。
    let probe = curable.teach(doomed.teach(FakeProbe::new(), false, true), true, true);
    let (out, issued) = pass(&mut groups, &probe);
    let fixed = only_line_with(&out, "[zorder-group] fix");
    assert!(
        fixed.contains("group_id=2"),
        "治った側の是正が記録されていない: {fixed}"
    );
    assert!(issued.is_empty(), "成立した巡に指令が積まれた");
    assert!(
        !groups.pending,
        "維持対象が全て成立したのに印が降りない（外したグループを判定に数えている）"
    );
}

/// 連続失敗は**グループごと**に数える——先に諦めた分が、次のグループへ持ち越されない。
///
/// 全体で 1 つの数として数える実装だと、2 本目のグループは 1 回目の失敗でいきなり上限に
/// 達し、是正されないまま印が降りる。ここでは 2 本とも実現できない窓にしてあるので、
/// 「1 本目を諦めた次の巡」で 2 本目が**まだ発行される**ことがそのまま per-group の証拠に
/// なる。
#[test]
fn each_group_carries_its_own_failure_count() {
    let mut built = pairs(2, 0x6100);
    let second = built.remove(1);
    let first = built.remove(0);
    let mut groups = ZOrderGroups::default();
    groups.groups.push(first.spec(1));
    groups.groups.push(second.spec(2));
    groups.pending = true;

    let broken = || second.teach(first.teach(FakeProbe::new(), false, true), false, true);

    // 巡 1〜3: 1 本目が発行され続ける（検証の失敗は巡 2・巡 3 の 2 回）。
    for _ in 0..3 {
        pass(&mut groups, &broken());
    }
    assert_eq!(
        groups.fail_streak(1),
        2,
        "1 本目の連続失敗が数えられていない"
    );
    assert_eq!(
        groups.fail_streak(2),
        0,
        "発行もされていない 2 本目に失敗が数えられている"
    );

    // 巡 4: 1 本目が上限に達して外れ、2 本目が初めて発行される。
    let (_, issued) = pass(&mut groups, &broken());
    assert_eq!(groups.fail_streak(1), 3, "1 本目が上限まで数えられていない");
    assert_eq!(issued, vec![second.hwnds[1]], "2 本目が発行されていない");

    // 巡 5: 2 本目の**1 回目**の失敗。上限には遠いので、まだ発行が続く。
    let (out, issued) = pass(&mut groups, &broken());
    assert_eq!(
        groups.fail_streak(2),
        1,
        "2 本目の連続失敗が 1 回目で 1 になっていない（全体で数えている疑い）"
    );
    assert!(
        !out.contains("reason=GaveUpAfterFailures"),
        "2 本目が 1 回の失敗で諦められた（連続失敗を全体で数えている）: {out}"
    );
    assert_eq!(
        issued,
        vec![second.hwnds[1]],
        "2 本目の是正が 1 回の失敗で止まった"
    );
    assert!(groups.pending, "崩れたグループが残っているのに印が降りた");
}

/// 他のグループの陰で**発行を見送られただけ**のグループは、連続失敗を 1 つも数えられない。
///
/// 1 巡に発行するのは 1 グループだけなので、2 本目以降は記録を伴わずに次巡へ持ち越される
/// （task 4.1 の形）。これが要件 8.3 に触れないのは「そのグループが成立するまで印が降り
/// ない」からであり、頭打ちがこの持ち越し組を維持対象から外すと、**一度も発行されないまま
/// 印が降りる**経路ができてしまう。連続失敗を数えるのは**検証**だけ＝発行していない
/// グループは構造上そこへ到達しない、というのが本実装の答えである。
#[test]
fn a_group_only_ever_deferred_is_never_given_up_on() {
    let mut built = pairs(2, 0x7100);
    let deferred = built.remove(1);
    let issuing = built.remove(0);
    let mut groups = ZOrderGroups::default();
    groups.groups.push(issuing.spec(1));
    groups.groups.push(deferred.spec(2));
    groups.pending = true;

    for round in 1..=3 {
        let probe = deferred.teach(issuing.teach(FakeProbe::new(), false, true), false, true);
        let (out, issued) = pass(&mut groups, &probe);

        assert_eq!(
            issued,
            vec![issuing.hwnds[1]],
            "巡 {round}: 発行が 1 本目に閉じていない"
        );
        assert_eq!(
            groups.fail_streak(2),
            0,
            "巡 {round}: 見送られただけのグループに連続失敗が数えられている"
        );
        assert!(
            !out.contains("group_id=2 reason=GaveUpAfterFailures"),
            "巡 {round}: 見送られただけのグループが諦められた: {out}"
        );
        assert!(
            groups.pending,
            "巡 {round}: 一度も発行していないグループが崩れたまま印が降りた"
        );
    }

    // 対照: 1 本目が上限に達して外れた巡に、持ち越されていた 2 本目が**初めて発行される**
    // ——見送りは握り潰しではなく持ち越しであり、頭打ちはそれを飛ばさない。
    let probe = deferred.teach(issuing.teach(FakeProbe::new(), false, true), false, true);
    let (_, issued) = pass(&mut groups, &probe);
    assert_eq!(
        issued,
        vec![deferred.hwnds[1]],
        "見送られ続けたグループが、頭打ちの後も発行されない"
    );
    assert!(
        groups.pending,
        "一度も成立していないグループが残っているのに印が降りた"
    );
}

/// 外したグループは、印が降りたあとの追随トリガで維持対象へ戻る。
///
/// 対照は 3 つある——⑴ 外れる前の巡では指令が出る ⑵ 外れた巡では出ず印が降りる
/// ⑶ 印を立て直せば**また**出る。⑶ が無いと「外したきり永久に捨てた」実装が緑になる。
#[test]
fn a_given_up_group_returns_at_the_next_follow_trigger() {
    let doomed = one_pair(0x8100);
    let mut groups = groups_with(21, &doomed.members);
    let broken = || doomed.teach(FakeProbe::new(), false, true);

    // 巡 1〜3: 発行され続ける（外れる前）
    for round in 1..=3 {
        let (_, issued) = pass(&mut groups, &broken());
        assert_eq!(
            issued,
            vec![doomed.hwnds[1]],
            "巡 {round}: 上限前なのに是正が出ていない"
        );
        assert!(groups.pending, "巡 {round}: 崩れたままなのに印が降りた");
    }

    // 巡 4: 上限に達して外れる。維持対象が空になるので印も降りる（tick を静かにする）。
    let (out, issued) = pass(&mut groups, &broken());
    assert!(
        out.contains("reason=GaveUpAfterFailures") && out.contains("group_id=21"),
        "諦めが記録されていない: {out}"
    );
    assert!(
        issued.is_empty(),
        "維持対象から外したグループへ是正が出ている"
    );
    assert!(
        !groups.pending,
        "維持対象が空になったのに印が降りない（tick が永久に起き続ける）"
    );

    // 巡 5: 印が降りている間は何もしない
    let (_, issued) = pass(&mut groups, &broken());
    assert!(issued.is_empty(), "印が降りた巡に指令が積まれた");

    // 巡 6: 追随トリガ（印を立てる）で維持対象へ戻る
    groups.pending = true;
    let (_, issued) = pass(&mut groups, &broken());
    assert_eq!(
        issued,
        vec![doomed.hwnds[1]],
        "追随トリガの後も外したままになっている（永久に捨てている）"
    );
}

/// 連続失敗は**連続**である——成功した巡に 0 へ戻るので、失敗と成功を交互に歩くグループは
/// 諦められない。
///
/// design の手順①は「成功→当該グループの連続失敗をリセット」と定めている。このリセットが
/// 死ぬと連続失敗が**累積**失敗に化け、3 連敗していないグループが `GaveUpAfterFailures` で
/// 維持対象から外れる。踏む経路はごく普通の並び（失敗 → 成功 → 失敗）である。
///
/// 崩れ続ける 2 本目を添えてあるのは、印が途中で降りると⑤が連続失敗を丸ごと捨ててしまい、
/// **リセットの有無がそもそも見えなくなる**からである（片側だけの檻にしないための足場）。
#[test]
fn a_successful_verification_resets_the_streak_so_alternating_failures_never_give_up() {
    let mut built = pairs(2, 0xB100);
    let keeper = built.remove(1);
    let target = built.remove(0);
    let mut groups = ZOrderGroups::default();
    groups.groups.push(target.spec(1));
    groups.groups.push(keeper.spec(2));
    groups.pending = true;

    let probe = |target_ordered: bool| {
        keeper.teach(
            target.teach(FakeProbe::new(), target_ordered, true),
            false,
            true,
        )
    };

    // 巡 1: 発行。巡 2: 1 回目の検証失敗。
    pass(&mut groups, &probe(false));
    pass(&mut groups, &probe(false));
    assert_eq!(groups.fail_streak(1), 1, "検証失敗が数えられていない");

    // 巡 3: 指令が効いた＝検証成功。連続失敗はここで 0 へ戻る。
    let (out, _) = pass(&mut groups, &probe(true));
    let line = only_line_with(&out, "[zorder-group] fix");
    assert!(
        line.contains("group_id=1"),
        "成立したのが対象のグループでない: {line}"
    );
    assert_eq!(
        groups.fail_streak(1),
        0,
        "成功した巡に連続失敗が 0 へ戻っていない（連続ではなく累積になっている）"
    );
    assert!(
        groups.pending,
        "崩れた 2 本目が残っているのに印が降りた（⑤が連続失敗を消して檻の足場が崩れる）"
    );

    // 巡 4〜6: 再び崩れる。この 3 巡で対象が失敗するのは 2 回だけなので、まだ諦めない
    // （累積なら成功前の 1 回が残っており、1 + 2 で上限に達してここで warn が出る）。
    let mut seen = String::new();
    for _ in 0..3 {
        let (out, _) = pass(&mut groups, &probe(false));
        seen.push_str(&out);
    }
    assert_eq!(
        groups.fail_streak(1),
        2,
        "連続失敗の数が「連続」になっていない（成功した巡を跨いで数え続けている）"
    );
    assert_eq!(
        count_lines_with(&seen, "reason=GaveUpAfterFailures"),
        0,
        "3 連敗していないグループを諦めた: {seen}"
    );
}

/// **測れなかった巡は頭打ちに数えない**——環境が是正を拒んだわけではないからである。
///
/// 要件 8.2 が記録を求めているのは「実行環境側の理由で是正が失敗した」場合である。窓が
/// 減って走査そのものを行わなかった巡をそこへ数えると、一度も是正を拒まれていない
/// グループに諦めの warn が立ち、**記録は残るが事実でない断念**になる。
///
/// 対照として、同じ回数を**実測して不一致だった**側で回せば確かに諦めることを併置して
/// ある（片側だけなら「そもそも諦めない実装」でも緑になる）。
#[test]
fn a_verification_that_could_not_measure_is_never_counted_toward_the_cap() {
    let mut built = pairs(4, 0xC100);
    let contrast_keeper = built.remove(3);
    let contrast_target = built.remove(2);
    let keeper = built.remove(1);
    let target = built.remove(0);

    // ⑴ 測れない巡を 3 度通しても諦めない
    let mut groups = ZOrderGroups::default();
    groups.groups.push(target.spec(1));
    groups.groups.push(keeper.spec(2));
    groups.pending = true;

    // 対象の窓が 1 枚しか解決しない巡＝比べる相手が居らず走査を行う理由が無い。
    let unmeasurable = || {
        keeper.teach(
            FakeProbe::new().with_handles(&target.members[..1], &target.hwnds[..1]),
            false,
            true,
        )
    };
    let measurable = || keeper.teach(target.teach(FakeProbe::new(), false, true), false, true);

    let mut seen = String::new();
    for _ in 0..3 {
        // 発行の巡（対象の窓は 2 枚とも在る）→ 検証の巡（1 枚に減っていて測れない）
        let (out, _) = pass(&mut groups, &measurable());
        seen.push_str(&out);
        let (out, _) = pass(&mut groups, &unmeasurable());
        seen.push_str(&out);
    }

    assert_eq!(
        groups.fail_streak(1),
        0,
        "測れなかった巡が連続失敗として数えられている"
    );
    assert_eq!(
        count_lines_with(&seen, "reason=GaveUpAfterFailures"),
        0,
        "一度も是正を拒まれていないグループを諦めた: {seen}"
    );

    // ⑵ 対照——同じ回数を「実測して不一致」で回せば、確かに諦める
    let mut contrast = ZOrderGroups::default();
    contrast.groups.push(contrast_target.spec(1));
    contrast.groups.push(contrast_keeper.spec(2));
    contrast.pending = true;
    let broken = || {
        contrast_keeper.teach(
            contrast_target.teach(FakeProbe::new(), false, true),
            false,
            true,
        )
    };

    let mut contrast_seen = String::new();
    for _ in 0..4 {
        let (out, _) = pass(&mut contrast, &broken());
        contrast_seen.push_str(&out);
    }
    assert_eq!(
        contrast.fail_streak(1),
        3,
        "実測して不一致だった巡が数えられていない（対照が死んでいる）"
    );
    assert_eq!(
        count_lines_with(&contrast_seen, "reason=GaveUpAfterFailures"),
        1,
        "実測して 3 度拒まれたグループを諦めていない（対照が死んでいる）: {contrast_seen}"
    );
}

// ===========================================================================
// 印の解除（設計の手順⑤）
// ===========================================================================

/// 印は**維持対象の全グループ**が成立した時点で降り、1 つでも崩れていれば降りない。
///
/// 2 本のグループで、片方だけ崩れている巡と両方成立した巡を続けて回す。
#[test]
fn the_mark_comes_down_only_when_every_maintained_group_holds() {
    let mut built = pairs(2, 0x9100);
    let right = built.remove(1);
    let left = built.remove(0);
    let mut groups = ZOrderGroups::default();
    groups.groups.push(left.spec(1));
    groups.groups.push(right.spec(2));
    groups.pending = true;

    // 片方が崩れている: 印は残る
    let half = right.teach(left.teach(FakeProbe::new(), true, true), false, true);
    let (_, issued) = pass(&mut groups, &half);
    assert_eq!(issued, vec![right.hwnds[1]], "崩れた側へ是正が出ていない");
    assert!(groups.pending, "崩れたグループが 1 本あるのに印が降りた");

    // 両方成立: 印が降りる
    let whole = right.teach(left.teach(FakeProbe::new(), true, true), true, true);
    let (_, issued) = pass(&mut groups, &whole);
    assert!(issued.is_empty(), "成立した巡に指令が積まれた");
    assert!(
        !groups.pending,
        "維持対象の全グループが成立したのに印が降りない"
    );
}

// ===========================================================================
// 記録の水準（諦めは既定運転でも読める）
// ===========================================================================

/// 諦めた事実は既定運転でも読める warn として残り、**最後の実測の走査の完否**も同じ行に
/// 載る（要件 8.2／8.3）。
///
/// 併置してある見送り（debug）は二重の対照である——⑴ サインオフ水準では出ることで
/// 捕捉窓そのものが生きていることを示し、⑵ 既定水準では黙ることで「諦めが残る」が
/// 水準の区別を本当に見ていることを示す。
///
/// `scan_complete` を**両方の値**で回すのは、この欄が「窓を動かせない末の断念」と
/// 「そもそも測り切れていない末の断念」を切り分けるために在るからである。片側だけだと
/// 欄を落とす変異も、常に同じ値を書く変異も緑のまま通る（記録層の 3 値の檻と同じ規律）。
#[test]
fn the_give_up_is_readable_without_the_diagnostic_directive() {
    for reached_top in [true, false] {
        let mut built = pairs(2, 0xA100);
        let settled = built.remove(1);
        let doomed = built.remove(0);
        let mut groups = ZOrderGroups::default();
        groups.groups.push(doomed.spec(31));
        groups.groups.push(settled.spec(32));
        groups.pending = true;
        let probe = || {
            settled.teach(
                doomed.teach(FakeProbe::new(), false, reached_top),
                true,
                true,
            )
        };

        // 巡 1〜2 は捨て、巡 3 で捕捉窓が debug の見送りを拾えることを確かめる。
        for _ in 0..2 {
            pass(&mut groups, &probe());
        }
        let (signoff, _) = pass(&mut groups, &probe());
        assert!(
            signoff.contains("reason=AlreadyOrdered"),
            "サインオフ水準で診断専用の見送りが拾えない（捕捉窓が死んでいる疑い）: {signoff}"
        );
        assert!(
            !signoff.contains("reason=GaveUpAfterFailures"),
            "上限に達する前に諦めている: {signoff}"
        );

        // 巡 4: 3 度目の検証失敗で諦める。既定水準でもこの 1 行だけは読める。
        clear_queue();
        let default = capture_under_filter(DEFAULT_DIRECTIVES, || {
            run_group_maintenance_pass(&mut groups, false, &probe());
        });
        let _ = drain_window_pos_commands();

        assert_eq!(
            count_lines_with(&default, "reason=GaveUpAfterFailures"),
            1,
            "既定運転で諦めが読めない（黙って捨てられている）: {default}"
        );
        let line = only_line_with(&default, "reason=GaveUpAfterFailures");
        assert!(
            line.contains("WARN"),
            "諦めの記録が warn 水準でない: {line}"
        );
        assert!(
            line.contains("group_id=31"),
            "諦めたグループが読めない: {line}"
        );
        assert!(line.contains("streak=3"), "連続失敗の数が読めない: {line}");
        assert!(
            line.contains(&format!("scan_complete={reached_top}")),
            "諦めた巡の走査の完否が行から読めない（`{reached_top}` を期待）: {line}"
        );
        assert_eq!(
            count_lines_with(&default, "reason=AlreadyOrdered"),
            0,
            "診断専用の見送りが既定水準へ漏れている（水準の区別を見ていない）: {default}"
        );
    }
}
