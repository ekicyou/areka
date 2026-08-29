// =============================================================================
// zorder drain 相：台帳から**鎖の計画**への射影と公開
// （task 3.1・要件 1.4／4.1／4.2／4.3／5.3／6.1／7.1／8.4／14.5／15.3／15.4）
//
// 出口が旧来のグループ列から鎖の計画へ替わった。この相が負うのは 4 つである。
//   ⑴ 実在しない要素は飛ばし、残る要素の相対順は**宣言のまま**保つ（1.4／7.2）
//   ⑵ 公開は**内容が前回と異なるときだけ**。窓の出現・破棄はここで自然に検出される
//      （7.1／15.3／14.5）
//   ⑶ グループが 1 つも無ければ受け口そのものを作らない（6.1／6.4）
//   ⑷ 全グループの解除で計画が空へ戻る（後方配置ごと撤去され既定状態へ戻る・4.1／15.4）
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
/// その後で窓が現れれば、鎖はそのぶんだけ伸びる（要件 1.4／7.1）。
///
/// 後半の窓の追加は `GhostWindows` を作り直す形で行う（正本の差し替えが唯一の入口
/// なので、スコープ 0 の entity も新しくなる）。見ているのは「宣言順のどこに
/// 収まるか」であって entity の同一性ではない。
#[test]
fn t_zdp01_absent_scope_still_yields_the_declared_order_among_existing_windows() {
    // scope 1 の窓はまだ無い。`\![set,zorder,1,0]` は b1,s1,b0,s0 を宣言する。
    let (mut world, mut ledger, _tx, rx) = seeded(&[0], &["1", "0"]);

    assert_eq!(
        chain_members(&world),
        Some(vec![balloon_of(&world, 0), char_of(&world, 0)]),
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
        chain_members(&world),
        Some(vec![
            balloon_of(&world, 1),
            char_of(&world, 1),
            balloon_of(&world, 0),
            char_of(&world, 0),
        ]),
        "現れた窓が宣言順の位置へ収まっていない（要件 7.1）"
    );
}

/// 鎖はグループの内側の**手前から奥への順**を宣言のまま保つ。
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
        chain_members(&world),
        Some(expected.clone()),
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

/// 繋ぐのは**スコープをまたぐ連続対だけ**——同一スコープの（バルーン, キャラ窓）対は
/// 既存のペア機構の担当であり、本 spec は 1 本も張らない（要件 6.3・境界）。
///
/// 窓 4 枚の鎖の連続対は 3 つあり、そのうち 2 つが同一スコープの対である。繋ぎが 1 本に
/// 絞られていなければ、同じ窓へ 2 つの機構が所有関係を書き合うことになる。
#[test]
fn t_zdp13_only_the_cross_scope_pair_becomes_an_edge() {
    let (world, _ledger, _tx, _rx) = seeded(&[0, 1], &["1", "0"]);

    assert_eq!(
        chain_edges(&world),
        Some(vec![(char_of(&world, 1), balloon_of(&world, 0))]),
        "繋ぎがスコープをまたぐ 1 本だけになっていない（ペア機構の担当まで張っている）"
    );
}

// ---------------------------------------------------------------------------
// 窓が減っても残りの相対順は宣言のまま（要件 7.2）
// ---------------------------------------------------------------------------

/// 窓が破棄されたスコープは鎖から外れ、**他スコープは巻き込まれない**（要件 7.2）。
///
/// 窓の破棄は片割れ 1 個で scope エントリごと正本から落ちる（`spawn.rs` の
/// `on_ghost_window_marker_remove`）。よって破棄後の鎖に残るのは無事なスコープの
/// 2 枚であり、その相対順は宣言のままである。
#[test]
fn t_zdp04_destroyed_scope_leaves_the_chain_without_dragging_others_with_it() {
    let (mut world, mut ledger, _tx, rx) = seeded(&[0, 1], &["1", "0"]);
    let survivors = vec![balloon_of(&world, 0), char_of(&world, 0)];
    let gone = balloon_of(&world, 1);
    world.despawn(gone);

    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        chain_members(&world),
        Some(survivors),
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
/// 適用系がそのハンドルを解決できずに毎巡失敗を数え続ける。
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
        chain_members(&world),
        Some(expected),
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
    assert!(
        !receiver_exists(&world),
        "既定状態で受け口が作られた（適用系が仕事を得てしまう・要件 6.1）"
    );
    assert_eq!(dirty(&world), None, "既定状態で印が立った");

    // グループ 1 本。
    tx.send(set_directive(&["1", "0"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        chain_members(&world).map(|m| m.len()),
        Some(4),
        "グループが在るのに鎖が受け口へ渡っていない"
    );
    assert_eq!(
        dirty(&world),
        Some(true),
        "計画が動いたのに印が立っていない"
    );
}

/// 何も動いていない巡では公開しない。内容が動いた巡では公開する（要件 14.5）。
///
/// 「動いた」の契機は台帳だけではない——**窓の出現**もここで自然に検出される。
#[test]
fn t_zdp06_publishing_happens_only_when_the_content_actually_differs() {
    let (mut world, mut ledger, _tx, rx) = seeded(&[0], &["1", "0"]);
    assert_eq!(dirty(&world), Some(true), "下ごしらえで印が立っていない");

    // 適用系が印を倒した後の状態を作る。
    clear_dirty(&mut world);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        dirty(&world),
        Some(false),
        "同じ内容の巡に公開し直された（適用系が毎巡空振りする）"
    );

    // 窓が現れた＝内容が動いた巡。
    spawn_scopes(&mut world, &[0, 1]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        dirty(&world),
        Some(true),
        "窓が現れた巡に公開されていない（鎖が組み替わらない）"
    );
}

/// **どのグループにも属さない**スコープの窓の出現・破棄も、同じ 1 つの門で検出される
/// （要件 15.3）。
///
/// 未指定スコープは後方へ、スコープ ID の昇順で参加する（要件 15.1／15.2）。指定した
/// スコープだけを見る実装はここで赤くなる——印が立たず、鎖の長さも伸びない。
#[test]
fn t_zdp14_an_unnamed_scope_appearing_and_leaving_is_detected_too() {
    let (mut world, mut ledger, _tx, rx) = seeded(&[0, 1], &["1", "0"]);
    assert_eq!(
        chain_members(&world).map(|m| m.len()),
        Some(4),
        "下ごしらえの鎖が 4 枚になっていない"
    );
    clear_dirty(&mut world);

    // 誰も指定していない scope 2 の窓が現れた。
    spawn_scopes(&mut world, &[0, 1, 2]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        dirty(&world),
        Some(true),
        "未指定スコープの出現が検出されていない（要件 15.3）"
    );
    assert_eq!(
        chain_members(&world),
        Some(vec![
            balloon_of(&world, 1),
            char_of(&world, 1),
            balloon_of(&world, 0),
            char_of(&world, 0),
            balloon_of(&world, 2),
            char_of(&world, 2),
        ]),
        "未指定スコープの窓が全グループの後ろのブロックとして参加していない（要件 15.1）"
    );

    // 去るときも同じ門で検出される。
    clear_dirty(&mut world);
    spawn_scopes(&mut world, &[0, 1]);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        dirty(&world),
        Some(true),
        "未指定スコープの破棄が検出されていない（要件 15.3）"
    );
    assert_eq!(
        chain_members(&world).map(|m| m.len()),
        Some(4),
        "後方配置が撤去されていない"
    );
}

// ---------------------------------------------------------------------------
// 解除で計画が空へ戻る（要件 4.1／4.2／15.4）
// ---------------------------------------------------------------------------

/// 全グループの解除で、鎖は**後方配置ごと**撤去され既定状態へ戻る（要件 4.2／15.4）。
///
/// 受け口そのものは残る（一度出来たものを消すと、適用系が撤去の指示を受け取れない）。
/// 消えるのは**中身**であり、`None`＝既定状態が公開されて印が立つ。
#[test]
fn t_zdp15_resetting_every_group_returns_the_plan_to_the_default_state() {
    let (mut world, mut ledger, tx, rx) = seeded(&[0, 1, 2], &["1", "0"]);
    assert_eq!(
        chain_members(&world).map(|m| m.len()),
        Some(6),
        "下ごしらえの鎖に未指定スコープの後方配置が入っていない"
    );
    clear_dirty(&mut world);

    tx.send(ZOrderDirective::Reset).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert!(
        ledger.groups().is_empty(),
        "解除で台帳が空になっていない（下ごしらえの前提が崩れている）"
    );
    assert_eq!(
        chain(&world),
        None,
        "解除で計画が空へ戻っていない（後方配置が残ると既定状態にならない・要件 15.4）"
    );
    assert!(
        receiver_exists(&world),
        "受け口ごと消えている（適用系が撤去の指示を受け取れない）"
    );
    assert_eq!(
        dirty(&world),
        Some(true),
        "解除が公開されていない（張った繋ぎが外れないまま残る・要件 4.1）"
    );

    // 解除後も相は静かなまま——同じ既定状態を毎巡公開し直さない。
    clear_dirty(&mut world);
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    assert_eq!(
        dirty(&world),
        Some(false),
        "既定状態が毎巡公開し直されている"
    );
}

/// 解除の後に同じスコープを指定し直せる（要件 4.3）。計画も新しい並びで立ち直る。
#[test]
fn t_zdp16_a_group_can_be_declared_again_after_a_reset() {
    let (mut world, mut ledger, tx, rx) = seeded(&[0, 1], &["1", "0"]);
    tx.send(ZOrderDirective::Reset).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    // 解除前と同じスコープを、今度は逆順で。
    tx.send(set_directive(&["0", "1"])).unwrap();
    run_zorder_drain_phase(&rx, &mut ledger, &mut world);

    assert_eq!(
        ledger.groups().len(),
        1,
        "解除前のスコープが再指定の拒否対象として残っている（要件 4.3）"
    );
    assert_eq!(
        chain_members(&world),
        Some(vec![
            balloon_of(&world, 0),
            char_of(&world, 0),
            balloon_of(&world, 1),
            char_of(&world, 1),
        ]),
        "再指定した並びが計画へ反映されていない"
    );
}

// ---------------------------------------------------------------------------
// 不在の記録（要件 8.4）
// ---------------------------------------------------------------------------

/// 宣言された窓が欠けたグループは不在の記録を残す。揃っていれば残さない。
///
/// 欠け方は**二側**ある——一部だけ欠ける（鎖には載る）と、1 枚も解決できない
/// （鎖が空になる）である。要件 8.4 が名指ししているのは後者
/// （「対応する窓が**一度も現れないまま**推移する場合」）なので、前者だけを
/// 踏む檻は肝心の場合を素通りさせる。両方をここで踏む。
#[test]
fn t_zdp07_absent_elements_are_recorded_and_complete_groups_are_not() {
    // 欠けている（scope 1 の窓がまだ無い）。
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0]);
    tx.send(set_directive(&["1", "0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));

    let absent = lines_with(&logs, "[zorder-chain] absent");
    assert_eq!(
        absent.len(),
        2,
        "欠けた要素が黙って落とされている（要件 8.4）: {logs:?}"
    );
    for line in &absent {
        assert!(
            line.contains(CHAIN_TARGET),
            "記録の出力先が 1 本に揃っていない: {line}"
        );
        assert!(
            line.contains("group_id=0"),
            "どのグループの宣言が空振りしたのか読めない: {line}"
        );
    }
    assert!(
        absent[0].contains("element=b1") && absent[1].contains("element=s1"),
        "不在の要素が宣言順の正準表記で読めない: {absent:?}"
    );

    // 1 枚も解決できない（要件 8.4 が名指しする「一度も現れないまま」）。
    // 鎖は空になるが、**記録は出なければならない**。
    let (tx2, rx2) = directive_channel();
    let mut ledger2 = ZOrderGroupLedger::default();
    let mut world2 = world_with_scopes(&[9]);
    tx2.send(set_directive(&["1", "0"])).unwrap();
    let logs2 = capture_logs(|| run_zorder_drain_phase(&rx2, &mut ledger2, &mut world2));

    assert_eq!(
        lines_with(&logs2, "[zorder-chain] absent").len(),
        4,
        "窓が 1 枚も現れていないグループの宣言が黙って落とされている（要件 8.4／8.3）: {logs2:?}"
    );

    // 揃っている（窓が全部在る）。
    let (tx3, rx3) = directive_channel();
    let mut ledger3 = ZOrderGroupLedger::default();
    let mut world3 = world_with_scopes(&[0, 1]);
    tx3.send(set_directive(&["1", "0"])).unwrap();
    let logs3 = capture_logs(|| run_zorder_drain_phase(&rx3, &mut ledger3, &mut world3));
    assert!(
        lines_with(&logs3, "[zorder-chain] absent").is_empty(),
        "全ての窓が在るのに不在が記録された: {logs3:?}"
    );
}

/// 後方参加のスコープの窓は、欠けても不在には数えない（要件 8.4 は**宣言**が対象）。
///
/// 誰も名前を挙げていない窓を「無い」と報せると、記録が本物の書き間違いを埋める。
#[test]
fn t_zdp17_unnamed_scopes_never_appear_in_the_absence_record() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[0, 1, 2]);
    tx.send(set_directive(&["1", "0"])).unwrap();
    let logs = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert!(
        lines_with(&logs, "[zorder-chain] absent").is_empty(),
        "下ごしらえの前提が崩れている（宣言は全て揃っているはず）: {logs:?}"
    );

    // 誰も指定していない scope 2 が去る——後方参加は宣言ではないので不在に数えない。
    let unnamed_gone = capture_logs(|| {
        spawn_scopes(&mut world, &[0, 1]);
        run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    });
    assert!(
        lines_with(&unnamed_gone, "[zorder-chain] absent").is_empty(),
        "宣言していないスコープの窓が不在として報告された: {unnamed_gone:?}"
    );

    // 対照——**宣言した** scope 1 が去れば報される（記録の経路そのものは生きている）。
    let named_gone = capture_logs(|| {
        spawn_scopes(&mut world, &[0]);
        run_zorder_drain_phase(&rx, &mut ledger, &mut world);
    });
    assert_eq!(
        lines_with(&named_gone, "[zorder-chain] absent").len(),
        2,
        "宣言したスコープの不在まで黙っている（上の主張が空虚になる）: {named_gone:?}"
    );
}

/// 同じ不在は繰り返し報せず、内容が動いたときだけ報せ直す。
///
/// 相は毎フレーム走るので、無条件に出すと同じ 1 行が延々と積み上がって本物の変化を
/// 埋める。かといって一度きりにすると、後から欠けが増減した事実が読めなくなる。
/// 両側から挟む——2 巡目は無音、解決できた枚数が変わった巡は再び出る。
#[test]
fn t_zdp10_the_same_absence_is_not_repeated_but_a_changed_one_is_reported_again() {
    let (tx, rx) = directive_channel();
    let mut ledger = ZOrderGroupLedger::default();
    let mut world = world_with_scopes(&[9]);
    tx.send(set_directive(&["1", "0"])).unwrap();

    let first = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert_eq!(
        lines_with(&first, "[zorder-chain] absent").len(),
        4,
        "初回の不在が報告されていない: {first:?}"
    );

    let second = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert!(
        lines_with(&second, "[zorder-chain] absent").is_empty(),
        "同じ不在が毎巡繰り返し報告されている（本物の変化が埋まる）: {second:?}"
    );

    // scope 0 の窓が現れた＝不在の中身が動いた（まだ scope 1 は欠けたまま）。
    spawn_scopes(&mut world, &[9, 0]);
    let third = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert_eq!(
        lines_with(&third, "[zorder-chain] absent").len(),
        2,
        "欠けの中身が動いたのに報告し直されていない: {third:?}"
    );
}

/// 一度**完全**へ戻ったグループが再び壊れたら、また報される。
///
/// [`t_zdp10`] の兄弟であり、あちらが歩かない遷移——「不完全 → **完全** → また不完全」
/// ——を歩く。控えを毎巡**まるごと組み直す**のはこの遷移のためである: 揃った時点で控えが
/// 空になるからこそ、後で壊れたときが「新しい事実」として報される。控えへ足すだけ
/// （累積）にすると古い記憶が残り続け、本物の新しい欠落を黙って飲み込む
/// ——連呼抑止が「黙って諦める」（要件 8.3 が禁じるもの）に堕ちる形である。
///
/// 3 つの巡を数で挟む: 2 本 → 0 本 → 2 本。真ん中の 0 本が無いと「毎巡出す」実装が、
/// 最後の 2 本が無いと「累積する」実装が、それぞれ素通りする。
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
        lines_with(&broken, "[zorder-chain] absent").len(),
        2,
        "初回の不在が報告されていない: {broken:?}"
    );

    // ⑵ 完全（scope 1 の窓が現れた）。控えが空になる巡である。
    spawn_scopes(&mut world, &[0, 1]);
    let complete = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert!(
        lines_with(&complete, "[zorder-chain] absent").is_empty(),
        "窓が揃った巡に不在が報告された: {complete:?}"
    );
    assert_eq!(
        chain_members(&world).map(|m| m.len()),
        Some(4),
        "下ごしらえの前提が崩れている（このグループは完全になっているはず）"
    );

    // ⑶ また不完全（scope 1 の窓が消えた）。控えが組み直されていれば再び報される。
    spawn_scopes(&mut world, &[0]);
    let broken_again = capture_logs(|| run_zorder_drain_phase(&rx, &mut ledger, &mut world));
    assert_eq!(
        lines_with(&broken_again, "[zorder-chain] absent").len(),
        2,
        "一度完全になったグループが再び壊れたのに黙って飲み込まれている（要件 8.3／8.4）: {broken_again:?}"
    );
}
