// =============================================================================
// zorder drain 相：台帳から実在する窓の列への射影（task 3.3・要件 1.4／6.1／7.1／7.2／8.4）
//
// 射影が守るのは 4 つである。
//   ⑴ 実在しない要素は飛ばし、残る要素の相対順は**宣言のまま**保つ（1.4／7.2）
//   ⑵ 実在が 2 枚未満のグループは射影から外すが**台帳には残る**（7.1）
//   ⑶ グループが 1 つも無ければ受け口そのものを作らない（6.1）
//   ⑷ 何も動いていない巡では印を立てない
//
// どれも「起きない」側の主張を含むので、檻は必ず対になる「起きる」側を隣へ置く。
// 片側だけだと、経路ごと死んでいる実装が緑のまま通る。
// =============================================================================

use super::test_support::*;
use super::*;

/// 台帳へ 1 本のグループを載せた World と受信端を用意する（下ごしらえ）。
///
/// 指令の経路をそのまま通す——射影だけを別経路で作ると、本番が通らない道を檻に
/// 入れることになる。
fn seeded(
    scopes: &[usize],
    tokens: &[&str],
) -> (
    World,
    ZOrderGroupLedger,
    std::sync::mpsc::Sender<ZOrderDirective>,
    Receiver<ZOrderDirective>,
) {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(scopes);
    tx.send(set_directive(tokens)).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    (world, ledger, tx, rx)
}

// ---------------------------------------------------------------------------
// 完了状態: まだ現れていないスコープを含んでも、在る窓だけで指定順が成立する
// ---------------------------------------------------------------------------

/// 窓がまだ無いスコープを含むグループでも、**在る窓だけ**で宣言どおりの相対順が立つ。
/// その後で窓が現れれば、射影はそのぶんだけ伸びる（要件 1.4／7.1）。
///
/// 後半の窓の追加は `GhostWindows` を作り直す形で行う（正本の差し替えが唯一の入口
/// なので、スコープ 0 の entity も新しくなる）。見ているのは「宣言順のどこに
/// 収まるか」であって entity の同一性ではない。
#[test]
fn t_zdp01_absent_scope_still_yields_the_declared_order_among_existing_windows() {
    // scope 1 の窓はまだ無い。`\![set,zorder,1,0]` は b1,s1,b0,s0 を宣言する。
    let (mut world, mut ledger, _tx, rx) = seeded(&[0], &["1", "0"]);

    assert_eq!(
        projected(&world),
        Some(vec![(0, vec![balloon_of(&world, 0), char_of(&world, 0)])]),
        "在る窓だけの列になっていない（宣言順のまま詰めるはず）"
    );
    assert_eq!(
        ledger.groups()[0].members.len(),
        4,
        "台帳から未出現スコープが取り除かれている（要件 1.4）"
    );

    // scope 1 の窓が現れた。
    spawn_scopes(&mut world, &[0, 1]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        projected(&world),
        Some(vec![(
            0,
            vec![
                balloon_of(&world, 1),
                char_of(&world, 1),
                balloon_of(&world, 0),
                char_of(&world, 0),
            ]
        )]),
        "現れた窓が宣言順の位置へ収まっていない（要件 7.1）"
    );
}

/// 射影はグループの内側の**手前から奥への順**を宣言のまま保つ。
///
/// 窓は scope 0 → scope 1 の順に生まれるが、宣言は `1,0`＝scope 1 が手前である。
/// entity の生成順や識別子の大小で並べ替える実装は、この檻で必ず赤くなる。
#[test]
fn t_zdp02_projection_preserves_the_declared_front_to_back_order() {
    let (world, _ledger, _tx, _rx) = seeded(&[0, 1], &["1", "0"]);

    let expected = vec![
        balloon_of(&world, 1),
        char_of(&world, 1),
        balloon_of(&world, 0),
        char_of(&world, 0),
    ];
    assert_eq!(
        projected(&world),
        Some(vec![(0, expected.clone())]),
        "宣言順（手前から奥）が保たれていない"
    );
    // 対照: 生成順（scope 0 が先）とは実際に異なる並びを見ている。
    assert_ne!(
        expected,
        vec![
            balloon_of(&world, 0),
            char_of(&world, 0),
            balloon_of(&world, 1),
            char_of(&world, 1),
        ],
        "檻の入力が生成順と同じで、並べ替えの有無を判別できていない"
    );
}

// ---------------------------------------------------------------------------
// 実在 2 枚未満は射影から外れる／台帳には残って戻ってくる（要件 7.1）
// ---------------------------------------------------------------------------

/// 在る窓が 2 枚に満たないグループは射影に載らないが、台帳には残り、窓が揃えば戻る。
/// ここは**実在 0 枚**の側から境界を押さえる。
///
/// 相棒窓の畳み込み（要件 2.6）が入って以降、宣言はスコープ単位のブロックにしかならず、
/// 正本（`GhostWindows`）も正常な経路では**同じスコープの窓を片方だけ**持つ状態を作らない。
/// ただし作れないわけではない——掃除の hook が働けない状況では片側だけが落ちる
/// （[`t_zdp08_a_stale_registry_entry_is_skipped_while_its_living_siblings_are_kept`]）。
/// 実在ちょうど 1 枚の側は
/// [`t_zdp12_exactly_one_existing_window_still_leaves_the_projection`] が同じ技法で押さえる。
#[test]
fn t_zdp03_under_length_group_leaves_the_projection_but_returns_from_the_ledger() {
    // b0 と s1 の 2 枚宣言（畳み込みで b0,s0,b1,s1 の 4 枚になる）。
    // World に在るのは scope 9 だけなので、宣言のどの窓も実在しない。
    let (mut world, mut ledger, _tx, rx) = seeded(&[9], &["b0", "s1"]);

    assert_eq!(
        projected(&world),
        None,
        "実在 2 枚未満のグループが受け口へ渡っている（比べる相手が居ない）"
    );
    assert_eq!(
        ledger.groups().len(),
        1,
        "射影から外したグループを台帳からも消している（要件 7.1）"
    );

    // 宣言した窓が現れたら維持対象へ戻る。
    spawn_scopes(&mut world, &[0, 1, 9]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        projected(&world),
        Some(vec![(
            0,
            vec![
                balloon_of(&world, 0),
                char_of(&world, 0),
                balloon_of(&world, 1),
                char_of(&world, 1),
            ]
        )]),
        "窓が揃ったのに射影へ戻っていない（要件 7.1）"
    );
}

/// 宣言 4 枚のうち**実在がちょうど 1 枚**でも射影から外れる（要件 7.1／1.4 の境界）。
///
/// `project_groups` の「実在 2 枚未満は載せない」判断（`>= 2`）は 0 枚と 1 枚の両方を
/// 落とす。0 枚だけを見ていると `>= 1` へ緩めた実装が素通りするので、**1 枚の側を
/// 直接組んで**境界を挟む。
///
/// 畳み込み（要件 2.6）の後は宣言がスコープ単位のブロックになるため、この形は
/// [`t_zdp08_a_stale_registry_entry_is_skipped_while_its_living_siblings_are_kept`] と
/// 同じ技法——掃除の hook が働けない間に窓を破棄し、古い正本を戻す——でしか作れない。
#[test]
fn t_zdp12_exactly_one_existing_window_still_leaves_the_projection() {
    // `b0,s1` は畳み込みで b0,s0,b1,s1 の 4 枚宣言になる。
    let (mut world, mut ledger, _tx, rx) = seeded(&[0, 1], &["b0", "s1"]);
    assert!(
        projected(&world).is_some(),
        "下ごしらえの時点で射影が立っていない（この後の None が何も証明しなくなる）"
    );

    let registry = ghost_windows(&world);
    let doomed = [
        registry
            .balloon_window(0)
            .expect("scope 0 のバルーン窓が無い"),
        registry.char_window(0).expect("scope 0 のキャラ窓が無い"),
        registry
            .balloon_window(1)
            .expect("scope 1 のバルーン窓が無い"),
    ];

    // 掃除の hook が働かない状況を作ってから 3 枚を破棄し、古い正本を戻す。
    // 残るのは s1 の 1 枚ちょうど。
    world.remove_resource::<GhostWindows>();
    for entity in doomed {
        world.despawn(entity);
    }
    world.insert_resource(registry);

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    // 受け口そのものは下ごしらえの巡で既に立っているので、消えるのは**中身**である
    // （`None`＝受け口が無い、との違いは要件 6.1 の判定で意味を持つので潰さない）。
    assert_eq!(
        projected(&world),
        Some(Vec::new()),
        "実在ちょうど 1 枚のグループが受け口へ渡っている（比べる相手が居ない）"
    );
    assert_eq!(
        ledger.groups()[0].members.len(),
        4,
        "射影から外したグループの宣言を台帳からも削っている（要件 1.4）"
    );
}

/// 窓が破棄されたスコープは射影から外れ、**他スコープは巻き込まれない**（要件 7.2）。
///
/// 窓の破棄は片割れ 1 個で scope エントリごと正本から落ちる（`spawn.rs` の
/// `on_ghost_window_marker_remove`）。よって破棄後の射影に残るのは無事なスコープの
/// 2 枚であり、その相対順は宣言のままである。
#[test]
fn t_zdp04_destroyed_scope_leaves_the_projection_without_dragging_others_with_it() {
    let (mut world, mut ledger, _tx, rx) = seeded(&[0, 1], &["1", "0"]);
    let survivors = vec![balloon_of(&world, 0), char_of(&world, 0)];
    let gone = balloon_of(&world, 1);
    world.despawn(gone);

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        projected(&world),
        Some(vec![(0, survivors)]),
        "破棄されたスコープを外して残りを宣言順で保てていない（要件 7.2）"
    );
    assert_eq!(
        ledger.groups()[0].members.len(),
        4,
        "破棄に巻き込まれて台帳の宣言が削られている（要件 1.4）"
    );
}

/// 正本が**既に居ない entity** を指していても、その要素だけを飛ばして残りは活かす。
///
/// 正常な破棄では正本の scope エントリごと落ちるので、この形は掃除の hook が働けない
/// 状況（正本が一時的に居ない間の破棄）でしか生じない。それでも射影が entity の生存を
/// 自分で確かめるのは、受け口へ死んだハンドルを渡さないためである——確かめずに渡すと、
/// 維持系がそのハンドルを解決できずに毎巡失敗を数え続ける。
#[test]
fn t_zdp08_a_stale_registry_entry_is_skipped_while_its_living_siblings_are_kept() {
    let (mut world, mut ledger, _tx, rx) = seeded(&[0, 1], &["1", "0"]);
    let registry = ghost_windows(&world);
    let dead = registry
        .balloon_window(1)
        .expect("scope 1 のバルーン窓が無い");
    let expected = vec![
        registry.char_window(1).expect("scope 1 のキャラ窓が無い"),
        registry
            .balloon_window(0)
            .expect("scope 0 のバルーン窓が無い"),
        registry.char_window(0).expect("scope 0 のキャラ窓が無い"),
    ];

    // 掃除の hook が働かない状況を作ってから破棄し、古い正本を戻す。
    world.remove_resource::<GhostWindows>();
    world.despawn(dead);
    world.insert_resource(registry);

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        projected(&world),
        Some(vec![(0, expected)]),
        "居ない entity を飛ばして残りを宣言順で保てていない（要件 7.2）"
    );
}

// ---------------------------------------------------------------------------
// 既定状態＝受け口そのものが無い（要件 6.1／6.4）
// ---------------------------------------------------------------------------

/// グループが 1 つも無ければ受け口を作らない。1 つでも在れば作って印を立てる。
#[test]
fn t_zdp05_no_group_means_no_receiver_while_one_group_creates_it() {
    // グループ 0 本（指令も無い）。
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        projected(&world),
        None,
        "既定状態で受け口が作られた（維持系が観測する対象を得てしまう・要件 6.1）"
    );
    assert_eq!(pending(&world), None, "既定状態で印が立った");

    // グループ 1 本。
    tx.send(set_directive(&["1", "0"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        projected(&world).map(|g| g.len()),
        Some(1),
        "グループが在るのに受け口へ渡っていない"
    );
    assert_eq!(
        pending(&world),
        Some(true),
        "射影が動いたのに印が立っていない"
    );
}

/// 何も動いていない巡では印を立て直さない。動いた巡では立てる。
#[test]
fn t_zdp06_pending_is_raised_only_when_the_projection_actually_moves() {
    let (mut world, mut ledger, _tx, rx) = seeded(&[0], &["1", "0"]);
    assert_eq!(pending(&world), Some(true), "下ごしらえで印が立っていない");

    // 維持系が印を倒した後の状態を作る。
    clear_pending(&mut world);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        pending(&world),
        Some(false),
        "何も動いていない巡に印が立て直された（維持系が毎巡空振りする）"
    );

    // 窓が現れた＝射影が動いた巡。
    spawn_scopes(&mut world, &[0, 1]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        pending(&world),
        Some(true),
        "射影が動いた巡に印が立っていない（是正が始まらない）"
    );
}

// ---------------------------------------------------------------------------
// 不在の記録（要件 8.4）
// ---------------------------------------------------------------------------

/// 宣言された窓が欠けたグループは見送りの記録を残す。揃っていれば残さない。
///
/// 欠け方は**二側**ある——一部だけ欠ける（射影には載る）と、1 枚も解決できない
/// （射影から丸ごと外れる）である。要件 8.4 が名指ししているのは後者
/// （「対応する窓が**一度も現れないまま**推移する場合」）なので、前者だけを
/// 踏む檻は肝心の場合を素通りさせる。両方をここで踏む。
#[test]
fn t_zdp07_missing_members_are_recorded_and_complete_groups_are_not() {
    // 欠けている（scope 1 の窓がまだ無い）。
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0]);
    tx.send(set_directive(&["1", "0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    let skips = lines_with(&logs, "[zorder-group] skip");
    assert_eq!(
        skips.len(),
        1,
        "欠けたメンバーが黙って落とされている（要件 8.4）: {logs:?}"
    );
    let line = skips[0];
    assert!(
        line.contains(GROUP_TARGET),
        "記録の出力先が 1 本に揃っていない: {line}"
    );
    assert!(
        line.contains("group_id=0"),
        "どのグループが欠けたのか読めない: {line}"
    );
    assert!(
        line.contains("reason=MemberMissing"),
        "既存の見送り理由の語で出ていない（読む側の語彙を増やしている）: {line}"
    );
    assert!(
        line.contains("declared=4 existing=2"),
        "宣言の数と実在の数が載っていない（一部だけ現れた形が読めない）: {line}"
    );
    assert!(
        line.contains("resolved=- missing=- order_ok=-"),
        "観測していない 3 欄が番兵になっていない（射影の実数で観測を騙っている）: {line}"
    );

    // 1 枚も解決できない（要件 8.4 が名指しする「一度も現れないまま」）。
    // 射影は空になり受け口すら作られないが、**記録は出なければならない**。
    let (tx2, rx2) = directive_channel();
    let mut ledger2 = ZOrderGroupLedger::default();
    let mut world2 = world_with_scopes(&[9]);
    tx2.send(set_directive(&["1", "0"])).unwrap();
    let logs2 = capture_logs(|| run_zorder_drain_phase(&rx2, &mut ledger2, &mut world2));

    assert_eq!(
        projected(&world2),
        None,
        "1 枚も無いグループが受け口へ渡っている（下ごしらえの前提が崩れている）"
    );
    let skips2 = lines_with(&logs2, "[zorder-group] skip");
    assert_eq!(
        skips2.len(),
        1,
        "窓が 1 枚も現れていないグループが黙って落とされている（要件 8.4／8.3）: {logs2:?}"
    );
    assert!(
        skips2[0].contains("group_id=0") && skips2[0].contains("reason=MemberMissing"),
        "全欠けの見送りが同じ語彙で出ていない: {}",
        skips2[0]
    );
    assert!(
        skips2[0].contains("declared=4 existing=0"),
        "「1 枚も現れていない」と「一部だけ現れた」が記録の上で区別できない: {}",
        skips2[0]
    );

    // 揃っている（窓が全部在る）。
    let (tx3, rx3) = directive_channel();
    let mut ledger3 = ZOrderGroupLedger::default();
    let mut world3 = world_with_scopes(&[0, 1]);
    tx3.send(set_directive(&["1", "0"])).unwrap();
    let logs3 = capture_logs(|| run_zorder_drain_phase(&rx3, &mut ledger3, &mut world3));
    assert!(
        lines_with(&logs3, "[zorder-group] skip").is_empty(),
        "全ての窓が在るのに不在の見送りが記録された: {logs3:?}"
    );
}

/// 射影が 1 ミリも動かない巡でも、新たに不完全になったグループは報告される。
///
/// 既に安定した射影が在るところへ「窓が 1 枚も無いグループ」を足すと、受け口へ渡る列は
/// **前の巡と同一**である。記録を「射影が動いたか」に紐付けると、この形が丸ごと沈黙する
/// ——不在の報告は射影の書込とは別の事実なので、書込の有無に依存させてはならない。
#[test]
fn t_zdp09_a_newly_incomplete_group_is_reported_even_when_the_projection_does_not_move() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1]);

    // 完全なグループ 1 本で射影を安定させる。
    tx.send(set_directive(&["1", "0"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    let stable = projected(&world);
    assert!(stable.is_some(), "下ごしらえの射影が立っていない");

    // 窓が 1 枚も無いスコープだけのグループを足す（射影は動かない）。
    tx.send(set_directive(&["5", "4"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    assert_eq!(
        projected(&world),
        stable,
        "下ごしらえの前提が崩れている（射影が動いてしまっている）"
    );
    let skips = lines_with(&logs, "[zorder-group] skip");
    assert_eq!(
        skips.len(),
        1,
        "射影が動かない巡に不在の報告が消えている（要件 8.4／8.3）: {logs:?}"
    );
    assert!(
        skips[0].contains("group_id=1"),
        "報告されたのが新しく不完全になったグループではない: {}",
        skips[0]
    );
}

/// 同じ不在は繰り返し報せず、事実が動いたときだけ報せ直す。
///
/// 相は毎フレーム走るので、無条件に出すと同じ 1 行が延々と積み上がって本物の変化を
/// 埋める。かといって一度きりにすると、後から欠けが増減した事実が読めなくなる。
/// 両側から挟む——2 巡目は無音、解決できた枚数が変わった巡は再び 1 本。
#[test]
fn t_zdp10_the_same_absence_is_not_repeated_but_a_changed_one_is_reported_again() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[9]);
    tx.send(set_directive(&["1", "0"])).unwrap();

    let first = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert_eq!(
        lines_with(&first, "[zorder-group] skip").len(),
        1,
        "初回の不在が報告されていない: {first:?}"
    );

    let second = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert!(
        lines_with(&second, "[zorder-group] skip").is_empty(),
        "同じ不在が毎巡繰り返し報告されている（本物の変化が埋まる）: {second:?}"
    );

    // scope 0 の窓が現れた＝解決できた枚数が動いた（まだ scope 1 は欠けたまま）。
    spawn_scopes(&mut world, &[9, 0]);
    let third = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert_eq!(
        lines_with(&third, "[zorder-group] skip").len(),
        1,
        "欠けの中身が動いたのに報告し直されていない: {third:?}"
    );
}

/// 一度**完全**へ戻ったグループが再び壊れたら、また報される。
///
/// [`t_zdp10`] の兄弟であり、あちらが歩かない遷移——「不完全 → **完全** → また不完全」
/// ——を歩く。控えを毎巡**まるごと組み直す**のはこの遷移のためである: 完全になった
/// グループの控えを落とすからこそ、後で壊れたときが「新しい事実」として報される。
/// 控えへ足すだけ（累積）にすると古い枚数が残り続け、本物の新しい欠落を黙って飲み込む
/// ——連呼抑止が「黙って諦める」（要件 8.3 が禁じるもの）に堕ちる形である。
///
/// 3 つの巡を数で挟む: 1 本 → 0 本 → 1 本。真ん中の 0 本が無いと「毎巡出す」実装が、
/// 最後の 1 本が無いと「累積する」実装が、それぞれ素通りする。
///
/// [`t_zdp10`]: t_zdp10_the_same_absence_is_not_repeated_but_a_changed_one_is_reported_again
#[test]
fn t_zdp11_a_group_that_became_complete_is_reported_again_when_it_breaks_again() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0]);
    tx.send(set_directive(&["1", "0"])).unwrap();

    // ⑴ 不完全（scope 1 の窓がまだ無い）。
    let broken = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert_eq!(
        lines_with(&broken, "[zorder-group] skip").len(),
        1,
        "初回の不在が報告されていない: {broken:?}"
    );

    // ⑵ 完全（scope 1 の窓が現れた）。控えからこのグループが落ちる巡である。
    spawn_scopes(&mut world, &[0, 1]);
    let complete = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert!(
        lines_with(&complete, "[zorder-group] skip").is_empty(),
        "窓が揃った巡に不在が報告された: {complete:?}"
    );
    assert_eq!(
        projected(&world).map(|groups| groups[0].1.len()),
        Some(4),
        "下ごしらえの前提が崩れている（このグループは完全になっているはず）"
    );

    // ⑶ また不完全（scope 1 の窓が消えた）。控えが組み直されていれば再び報される。
    spawn_scopes(&mut world, &[0]);
    let broken_again = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert_eq!(
        lines_with(&broken_again, "[zorder-group] skip").len(),
        1,
        "一度完全になったグループが再び壊れたのに黙って飲み込まれている（要件 8.3／8.4）: {broken_again:?}"
    );
}
