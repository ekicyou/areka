//! 鎖の適用系のうち、**後押しと印の持ち越し**を受け持つ檻（要件 1.1／1.2／8.2／9.2／
//! 9.3／11.1／14.2）。
//!
//! 兄弟の [`zorder_chain_apply_tests`](super) と足場（偽ハンドル・台本つきの替え玉・記録の
//! 捕捉）を丸ごと共有するため、**あちらの子モジュール**として置いてある——1 ファイル
//! 1,000 行未満という本 spec の共通制約に従って分けただけであり、主題の切れ目は
//! 「⑸⑹ 後押しとその直後の実測」「⑺ 印の始末」である。
//!
//! # ここが受け持つ 2 つの主題
//!
//! 1. **後押しの形**——動かすのは鎖の**根**であり、挿入位置は錨（根の 1 つ手前の窓）か、
//!    それが空振りになる巡だけ先頭へ切り替わる（`research.md` §13）。切り替えの両枝を
//!    対にして固定するので、切り替えを経路から外す変異はどちらかで必ず赤になる
//! 2. **印の持ち越し**——1 本も書けなかった巡（窓ハンドル未取得）では印を落とさない。
//!    計画は HWND 生成より前に公開されうるので、落とすと鎖が永久に書かれない（§13.5）。
//!    持ち越しが「毎巡の観測と是正」（要件 14.2 が退役させた形）へ化けていないことは、
//!    ⑴ ハンドルが付いた次の巡で解けること ⑵ 食い違いだけの巡では持ち越さないこと
//!    の 2 本で挟む

use super::*;

// ===========================================================================
// 1 本も書けなかった巡は印を持ち越す（計画が HWND より先に公開されうる経路）
// ===========================================================================

/// 印の現況を読む。
fn dirty_now(world: &World) -> bool {
    world
        .get_resource::<ZOrderChainPlan>()
        .expect("受け口が無い")
        .dirty
}

/// **1 つの schedule を使い回して** `passes` 巡ぶん回し、捕捉した記録と呼び出しを返す。
///
/// 使い回すのが要点である——持ち越し中の記録の抑止は system ごとの局所状態
/// （[`Local`](bevy_ecs::system::Local)）で持つので、巡ごとに新しい schedule を作ると
/// 状態が毎回初期化され、「2 巡目以降は出さない」を測れない。本番は 1 つの schedule を
/// アプリの生涯にわたって回すので、こちらが本番に忠実な形である。
fn run_passes(world: &mut World, script: Script, passes: usize) -> (String, Script) {
    let mut schedule = chain_schedule();
    run_on(&mut schedule, world, script, |schedule, world| {
        for _ in 0..passes {
            schedule.run(world);
        }
    })
}

/// **1 つの schedule を使い回したまま**、`body` が組む巡の並びを 1 つの捕捉窓で回す。
///
/// 待ち → 書けた → 再び待つ、のような**巡回**を踏むための道具である。[`run_passes`] は
/// 「同じ状態で N 巡」しか組めないので、巡と巡の間で World を触る必要がある檻はこちらを使う。
/// 記録は**全部の巡ぶんが 1 つの文字列に積まれる**ので、累計の本数で主張できる。
fn run_on(
    schedule: &mut Schedule,
    world: &mut World,
    script: Script,
    body: impl FnOnce(&mut Schedule, &mut World),
) -> (String, Script) {
    with_script(script, || {
        capture_under_filter(SIGNOFF_DIRECTIVES, || {
            body(schedule, world);
        })
    })
}

/// **窓ハンドルが現れないまま何巡回しても、実行環境を 1 度も呼ばず記録も増えない。**
///
/// 持ち越しは「解けるまで毎巡試す」形になってはならない（要件 14.2 が退役させた反復是正）。
/// 実際に走るのは差分の純判断だけで、実行環境の窓口は 1 度も叩かず、見送りの記録も
/// **待ち始めの 1 回**しか出ない——`zorder_drain.rs` の `report_absent_elements` が採る
/// 「前回と内容が違うときだけ出す」と同じ姿勢である。
///
/// 本数を **1 本ちょうど**で固定するのは両側から挟むためである——`5` なら記録の氾濫
/// （実機のログ判定が埋まる）、`0` なら黙って諦めた形（要件 8.3 違反）になる。
#[test]
fn waiting_for_window_handles_never_touches_the_runtime_and_records_only_once() {
    const PASSES: usize = 5;

    let mut world = World::new();
    // ハンドルの無い entity（HWND 生成前の姿）。
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let (out, used) = run_passes(&mut world, Script::default(), PASSES);

    assert!(
        !touched_runtime(&used.calls),
        "ハンドルが無いのに実行環境を呼んでいる（{PASSES} 巡）: {:?}",
        used.calls
    );
    assert_line_count(
        &out,
        SKIPPED,
        1,
        "窓ハンドル待ちの持ち越し 5 巡ぶん（待ち始めの 1 回だけ出す）",
    );
    assert!(
        dirty_now(&world),
        "{PASSES} 巡回したあとに印が落ちている（あとからハンドルが現れても二度と立たない）"
    );
}

/// 窓ハンドルを持たない entity を 1 つ生やし、既存の鎖の末尾へ繋いだ計画を公開する。
///
/// 「**新しい待ちが始まる**」ことを組むための道具である（既存の繋ぎは望みのまま据え置くので
/// 差分は新しい付与 1 本だけになり、その 1 本が窓ハンドル未取得で見送られる）。
fn publish_with_one_more_handleless_member(
    world: &mut World,
    members: &[Entity],
    cross_edges: &[CrossEdge],
) {
    let newcomer = world.spawn_empty().id();
    let tail = *members.last().expect("鎖に窓が 1 枚も無い");
    let mut members = members.to_vec();
    members.push(newcomer);
    let mut cross_edges = cross_edges.to_vec();
    cross_edges.push(edge(tail, newcomer, ChainSegment::Group(0)));
    publish(world, members, cross_edges);
}

/// **書けて終わった待ちのあと、次の待ちでもう一度記録が出る**（記録の再武装・要件 8.3）。
///
/// 記録の抑止は「待ち始めの 1 回だけ出す」形なので、**待ちが解けたときに武装し直さないと**
/// 以後どの待ちも無記録になる——起動直後の 1 回しか記録が出ない、静かな失敗経路である。
/// 本番は長く走り、スコープは時間差で増減するので、この巡回は必ず踏まれる。
///
/// 3 段を**1 つの schedule を使い回して**踏む（`run_once` では巡ごとに system が作り直され、
/// 局所状態が初期化されて檻にならない）。記録は 1 つの捕捉窓に積むので**累計**で主張できる。
#[test]
fn a_wait_resolved_by_writing_re_arms_the_record_for_the_next_wait() {
    let mut world = World::new();
    // ハンドルの無い entity（HWND 生成前の姿）。
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    let members = vec![a, b];
    let cross_edges = vec![edge(a, b, ChainSegment::Group(0))];
    publish(&mut world, members.clone(), cross_edges.clone());

    let mut schedule = chain_schedule();
    let (out, used) = run_on(
        &mut schedule,
        &mut world,
        Script::default(),
        |schedule, world| {
            // ⑴ ハンドル無しで数巡——待ち始めの 1 回だけ記録され、以後は積まれない。
            for _ in 0..3 {
                schedule.run(world);
            }

            // ⑵ 両端にハンドルが付く（本番では wintf の窓生成が HWND 取得後に付与する）。
            world.entity_mut(a).insert(WindowHandle {
                hwnd: fake_hwnd(0x10),
                instance: HINSTANCE::default(),
            });
            world.entity_mut(b).insert(WindowHandle {
                hwnd: fake_hwnd(0x20),
                instance: HINSTANCE::default(),
            });
            schedule.run(world);

            // ⑶ 新しい窓が宣言され、そのハンドルはまだ無い＝**次の待ちが始まる**。
            publish_with_one_more_handleless_member(world, &members, &cross_edges);
            for _ in 0..3 {
                schedule.run(world);
            }
        },
    );

    // ⑵ が本当に書けたこと（この段が空振りだと ⑶ の主張が空虚になる）。
    assert_line_count(&out, LINKED, 1, "ハンドルが揃った巡で繋ぐ");
    assert!(
        used.calls.contains(&Call::SetOwner(0x10, 0x20)),
        "ハンドルが揃った巡で所有関係を書いていない: {:?}",
        used.calls
    );

    // 本題——待ちは 2 度あり、記録も 2 本ちょうど（各待ちにつき 1 本）。
    assert_line_count(
        &out,
        SKIPPED,
        2,
        "待ちが 2 度あるのに記録が 2 本でない（1 本なら再武装が壊れて 2 度目の待ちが無記録・         4 本以上なら抑止が壊れて記録が氾濫）",
    );
    assert!(
        dirty_now(&world),
        "2 度目の待ちで印が落ちている（あとからハンドルが現れても二度と立たない）"
    );
}

/// **書けずに終わった待ちのあとも、次の待ちで記録が出る**（2 つ目の再武装の枝）。
///
/// 待ちが終わる経路は **3 つ**ある——⑴ 1 本でも書けた ⑵ そもそも持ち越さない巡（食い違い
/// だけ等）が挟まった ⑶ **出す操作が無い巡**（`NoChange`）が挟まった。上の 1 本が ⑴ を、
/// こちらが ⑵ を、次の 1 本が ⑶ を固定する。**どれ 1 つでも武装し直さなければ、それ以降の
/// 待ちが無記録になる**（要件 8.3 への無言の失敗経路）。
#[test]
fn a_wait_ended_by_a_non_carrying_pass_also_re_arms_the_record() {
    let mut world = World::new();
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let mut script = Script::default();
    // 現況の所有者が帳簿の控えと違う＝実行環境を呼ばずに帳簿だけ落とす経路（`Diverged`）。
    script.owner_of.insert(0x10, 0x99);

    let mut schedule = chain_schedule();
    let (out, used) = run_on(&mut schedule, &mut world, script, |schedule, world| {
        // ⑴ ハンドル無しで数巡——待ち始めの 1 回だけ記録。
        for _ in 0..2 {
            schedule.run(world);
        }

        // ⑵ ハンドルは付くが、帳簿の控えと現況が食い違う繋ぎを撤去する巡を作る。
        //    実行環境は 1 度も呼ばれず（`acted` は偽）、ハンドル未取得でもない
        //    （`handle_missing` は偽）＝**持ち越さない巡**である。
        world.entity_mut(a).insert(WindowHandle {
            hwnd: fake_hwnd(0x10),
            instance: HINSTANCE::default(),
        });
        world.entity_mut(b).insert(WindowHandle {
            hwnd: fake_hwnd(0x20),
            instance: HINSTANCE::default(),
        });
        record_link(world, a, b, fake_hwnd(0x10), fake_hwnd(0x20));
        world.insert_resource(ZOrderChainPlan {
            chain: None,
            dirty: true,
        });
        schedule.run(world);

        // ⑶ 再びハンドルの無い窓が宣言される＝**次の待ちが始まる**。
        let c = world.spawn_empty().id();
        publish(world, vec![a, c], vec![edge(a, c, ChainSegment::Group(0))]);
        for _ in 0..2 {
            schedule.run(world);
        }
    });

    // ⑵ が本当に「食い違いだけの巡」だったこと（この段が別物だと ⑶ の主張が空虚になる）。
    assert!(
        lines_with(&out, UNLINKED)
            .iter()
            .any(|line| field(line, "reason") == "Diverged"),
        "食い違いの巡が組めていない: {out}"
    );
    assert!(
        !used.calls.contains(&Call::ClearOwner(0x10)),
        "食い違いなのに実行環境を呼んでいる: {:?}",
        used.calls
    );

    // 本題——待ちは 2 度あり、記録も 2 本ちょうど。
    assert_line_count(
        &out,
        SKIPPED,
        2,
        "持ち越さない巡を挟んだあとの待ちが無記録になっている（再武装が壊れている）",
    );
    assert!(dirty_now(&world), "2 度目の待ちで印が落ちている");
}

/// **出す操作が無い巡（`NoChange`）を挟んだあとも、次の待ちで記録が出る**（3 つ目の再武装の枝）。
///
/// `ops` が空の巡は**印を落として早期に戻る**——つまりそこで待ちは終わっている。にもかかわらず
/// 武装し直さないと、以後どの待ちも無記録になる。発火条件は狭い（待ちの最中に、1 本も書けない
/// まま望みと現況が一致する公開が挟まる＝先に現れたスコープがハンドル取得前に消える等）が、
/// **狭いだけで塞がっていない経路は、静かな失敗経路である**。
///
/// ⑵ の 1 本と同じ 3 段構成で、真ん中の段だけを差し替えてある。刺激が届いたことの自己検査には
/// **`NoChange` の記録の実在**を使う——その巡が本当に「出す操作が無い巡」だったことの証拠である。
#[test]
fn a_wait_ended_by_a_no_change_pass_also_re_arms_the_record() {
    let mut world = World::new();
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let mut schedule = chain_schedule();
    let (out, used) = run_on(
        &mut schedule,
        &mut world,
        Script::default(),
        |schedule, world| {
            // ⑴ ハンドル無しで数巡——待ち始めの 1 回だけ記録。
            for _ in 0..2 {
                schedule.run(world);
            }

            // ⑵ **横断 edge の無い計画**を公開する（望みも帳簿も空＝差分ゼロ）。
            //    印は立つが出す操作が無いので、適用系は `NoChange` を記録して早期に戻る
            //    ——このとき `dirty` は落ちており、待ちはここで終わっている。
            publish(world, vec![a, b], Vec::new());
            schedule.run(world);

            // ⑶ 再びハンドルの無い窓が宣言される＝**次の待ちが始まる**。
            let c = world.spawn_empty().id();
            publish(world, vec![a, c], vec![edge(a, c, ChainSegment::Group(0))]);
            for _ in 0..2 {
                schedule.run(world);
            }
        },
    );

    // ⑵ が本当に「出す操作が無い巡」だったこと（この段が別物だと ⑶ の主張が空虚になる）。
    let reasons: Vec<&str> = lines_with(&out, SKIPPED)
        .iter()
        .map(|line| field(line, "reason"))
        .collect();
    assert!(
        reasons.contains(&"NoChange"),
        "差分ゼロの巡が組めていない（`NoChange` の記録が無い）: {reasons:?}"
    );
    assert!(
        !touched_runtime(&used.calls),
        "どの巡も実行環境を呼ばないはずが呼んでいる: {:?}",
        used.calls
    );

    // 本題——待ちは 2 度あり、`HandleMissing` の記録も 2 本ちょうど。
    let waits = reasons
        .iter()
        .filter(|reason| **reason == "HandleMissing")
        .count();
    assert_eq!(
        waits, 2,
        "差分ゼロの巡を挟んだあとの待ちが無記録になっている（再武装が壊れている）: {reasons:?}"
    );
    assert!(dirty_now(&world), "2 度目の待ちで印が落ちている");
}

/// **窓ハンドルがまだ 1 枚も無い巡は、印を落とさずに持ち越す。**
///
/// 計画の解決（areka の `resolve_member`）は在庫と entity の実在だけを見て HWND を
/// 要求しないので、計画は HWND 生成より前に公開されうる（`WindowHandle` は
/// `placement/spawn.rs` のとおり entity より遅れて付く）。ここで印を落とすと、
/// あとからハンドルが現れても**計画の内容は変わらないので再公開されず、印は二度と
/// 立たない**＝鎖が永久に書かれない。
#[test]
fn a_pass_that_could_not_write_a_single_link_keeps_the_mark_raised() {
    let mut world = World::new();
    // ハンドルの無い entity（`WindowHandle` を付けない＝HWND 生成前の姿）。
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let (out, used) = run_once(&mut world, Script::default());

    assert!(
        !touched_runtime(&used.calls),
        "ハンドルが無いのに実行環境を呼んでいる: {:?}",
        used.calls
    );
    assert_line_count(&out, SKIPPED, 1, "ハンドル未取得");
    assert!(
        dirty_now(&world),
        "1 本も書けなかった巡で印が落ちている（あとからハンドルが現れても二度と立たない）"
    );
}

/// 持ち越した印は、**ハンドルが現れた次の巡で解ける**（無限に持ち越さない）。
///
/// 前の 1 本と対を成す——持ち越しが「毎巡の観測と是正」へ化けていないことの証跡である。
#[test]
fn the_carried_mark_is_consumed_as_soon_as_the_handles_appear() {
    let mut world = World::new();
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let (_out, _used) = run_once(&mut world, Script::default());
    assert!(dirty_now(&world), "1 巡目で印が落ちている");

    // 窓ハンドルが付く（本番では wintf の窓生成が HWND 取得後に付与する）。
    world.entity_mut(a).insert(WindowHandle {
        hwnd: fake_hwnd(0x10),
        instance: HINSTANCE::default(),
    });
    world.entity_mut(b).insert(WindowHandle {
        hwnd: fake_hwnd(0x20),
        instance: HINSTANCE::default(),
    });

    let (out, used) = run_once(&mut world, Script::default());

    assert_line_count(&out, LINKED, 1, "ハンドルが揃えば張る");
    assert!(
        used.calls.contains(&Call::SetOwner(0x10, 0x20)),
        "ハンドルが揃った巡で所有関係を書いていない: {:?}",
        used.calls
    );
    assert!(
        !dirty_now(&world),
        "書けた巡でも印が残っている（毎巡の空振りへ逆戻りしている）"
    );
}

/// 帳簿との**食い違いだけ**の巡は、印を持ち越さない（待っても変わらないため）。
///
/// 持ち越しの条件が「ハンドル未取得」に限られていることの対照である。ここまで
/// 一括りにすると、食い違いが解けない限り毎巡走り続ける形＝要件 14.2 が退役させた
/// 反復是正になる。
#[test]
fn a_pass_that_only_found_a_diverged_ledger_entry_does_not_carry_the_mark() {
    let mut world = World::new();
    let front = spawn_window(&mut world, fake_hwnd(0x10));
    let back = spawn_window(&mut world, fake_hwnd(0x20));
    // 帳簿にはあるが、望む鎖からは消えている繋ぎ（＝撤去の対象）。
    record_link(&mut world, front, back, fake_hwnd(0x10), fake_hwnd(0x20));
    publish(&mut world, vec![front, back], Vec::new());

    let mut script = Script::default();
    // 現況の所有者が帳簿の控えと違う＝実行環境を呼ばずに帳簿だけ落とす経路。
    script.owner_of.insert(0x10, 0x99);

    let (out, used) = run_once(&mut world, script);

    assert!(
        !used.calls.contains(&Call::ClearOwner(0x10)),
        "食い違いなのに実行環境を呼んでいる: {:?}",
        used.calls
    );
    assert!(
        lines_with(&out, UNLINKED)
            .iter()
            .any(|line| field(line, "reason") == "Diverged"),
        "食い違いの記録が無い: {out}"
    );
    assert!(
        !dirty_now(&world),
        "食い違いだけの巡で印を持ち越している（毎巡の反復是正へ逆戻りしている）"
    );
}

// ===========================================================================
// 後押し 1 回と、その直後の実測（要件 9.2／9.3／11.1）
// ===========================================================================

/// 操作が走った巡は、鎖全体へ後押しが**ちょうど 1 回**出て、その直後に実測される。
///
/// 宣言と実測は同じ 1 行に載る（分けると「指令は出したが効かなかった」の判定が
/// 2 行の突合になる＝要件 9.2 が同一行を求める理由）。
#[test]
fn the_nudge_runs_once_for_the_whole_chain_and_the_measurement_follows_it_immediately() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    publish(
        &mut world,
        vec![a, b, c],
        vec![
            edge(a, b, ChainSegment::Group(0)),
            edge(b, c, ChainSegment::Group(0)),
        ],
    );

    let mut script = Script::default();
    // 最も奥（0x30）から手前へ辿ると、間に部外者（0x99・不可視の隣は走査が既に
    // 読み飛ばしている）を挟んで 0x20・0x10 が現れる。
    script
        .front_of
        .insert(0x30, vec![0x20usize, 0x99usize, 0x10usize]);

    let (out, used) = run_once(&mut world, script);

    let nudges: Vec<&Call> = used
        .calls
        .iter()
        .filter(|c| matches!(c, Call::Nudge(_, _)))
        .collect();
    assert_eq!(
        nudges.len(),
        1,
        "後押しが鎖全体につき 1 回になっていない: {:?}",
        used.calls
    );
    assert_eq!(
        *nudges[0],
        Call::Nudge(0x30, 0x20),
        "後押しの形が「鎖の根を錨（1 つ手前の窓）の直後へ差し直す」になっていない"
    );

    // 挿入位置の 2 択は**後押しの直前に 1 度だけ**現況を読んで決める。
    let nudge_at = used
        .calls
        .iter()
        .position(|c| matches!(c, Call::Nudge(_, _)))
        .expect("後押しが無い");
    assert_eq!(
        used.calls.get(nudge_at - 1),
        Some(&Call::RawNext(0x20)),
        "後押しの直前に錨の現況を読んでいない: {:?}",
        used.calls
    );
    assert_eq!(
        used.calls
            .iter()
            .filter(|c| matches!(c, Call::RawNext(_)))
            .count(),
        1,
        "現況の読みが 1 巡に 1 度でない（周期的な観測へ逆戻りしている）: {:?}",
        used.calls
    );

    // 実測は**後押しの直後**である（間に他の窓口を挟まない）。
    assert_eq!(
        used.calls.get(nudge_at + 1),
        Some(&Call::MeasureFront(0x30)),
        "後押しの直後に実測していない: {:?}",
        used.calls
    );

    assert_line_count(&out, SETTLED, 1, "鎖全体につき 1 行");
    let settled = lines_with(&out, SETTLED)[0];
    assert_eq!(field(settled, "nudged_hwnd"), "0x30", "{settled}");
    assert_eq!(field(settled, "insert_after"), "0x20", "{settled}");
    assert_eq!(field(settled, "declared"), "0x10,0x20,0x30", "{settled}");
    assert_eq!(field(settled, "measured"), "0x10,0x20,0x30", "{settled}");
    assert_eq!(field(settled, "nudge_ok"), "true", "{settled}");
}

/// 錨の直後が既に根の巡は、挿入位置が**先頭へ切り替わる**（空振りを塞ぐ）。
///
/// 本 task の是正が本番の 1 巡を通っていることの証跡である。上の 1 本と入力が違うのは
/// 台本の `next_of`（錨 0x20 の生の 1 つ奥）だけであり、そこが根（0x30）を指すと
/// 挿入位置が先頭（0x10）へ替わる。
#[test]
fn a_pass_whose_plain_insert_position_is_redundant_nudges_against_the_head_instead() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    publish(
        &mut world,
        vec![a, b, c],
        vec![
            edge(a, b, ChainSegment::Group(0)),
            edge(b, c, ChainSegment::Group(0)),
        ],
    );

    let mut script = Script::default();
    // 錨（0x20）の生の 1 つ奥が根（0x30）＝素直な形が 1 ミリも動かさない巡。
    script.next_of.insert(0x20, 0x30);
    script.front_of.insert(0x30, vec![0x20usize, 0x10usize]);

    let (out, used) = run_once(&mut world, script);

    let nudges: Vec<&Call> = used
        .calls
        .iter()
        .filter(|c| matches!(c, Call::Nudge(_, _)))
        .collect();
    assert_eq!(nudges.len(), 1, "後押しが 1 回でない: {:?}", used.calls);
    assert_eq!(
        *nudges[0],
        Call::Nudge(0x30, 0x10),
        "空振りする巡で挿入位置が先頭へ切り替わっていない"
    );

    let settled = lines_with(&out, SETTLED)[0];
    assert_eq!(field(settled, "nudged_hwnd"), "0x30", "{settled}");
    assert_eq!(field(settled, "insert_after"), "0x10", "{settled}");
}

/// 【対照】同じ台本から `next_of` の 1 行を抜くと、挿入位置は錨へ戻る。
///
/// 上の 1 本が「切り替えの経路を本当に通っている」ことの自己検査である。切り替えを
/// 経路から外す変異を当てると、この 2 本のどちらかが必ず赤になる。
#[test]
fn the_same_pass_without_the_redundant_neighbour_keeps_the_plain_insert_position() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    publish(
        &mut world,
        vec![a, b, c],
        vec![
            edge(a, b, ChainSegment::Group(0)),
            edge(b, c, ChainSegment::Group(0)),
        ],
    );

    let mut script = Script::default();
    // 錨の生の 1 つ奥は根ではない（鎖の外の窓が挟まっている）。
    script.next_of.insert(0x20, 0x99);
    script.front_of.insert(0x30, vec![0x20usize, 0x10usize]);

    let (_out, used) = run_once(&mut world, script);

    assert!(
        used.calls.contains(&Call::Nudge(0x30, 0x20)),
        "錨の直後が根でない巡で素直な形が出ていない: {:?}",
        used.calls
    );
}

/// 後押しが失敗しても記録して続行する（黙って消えない・要件 8.2／8.3）。
#[test]
fn a_failed_nudge_is_recorded_on_the_settled_line_rather_than_swallowed() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let mut script = Script::default();
    script.nudge_fails = true;

    let (out, _used) = run_once(&mut world, script);

    assert_line_count(&out, SETTLED, 1, "後押しの失敗");
    assert_eq!(
        field(lines_with(&out, SETTLED)[0], "nudge_ok"),
        "false",
        "後押しの失敗が行から読めない"
    );
}

/// 実測は宣言に無い窓を拾わない（鎖の外の窓の前後は主張しない・DD-3b）。
#[test]
fn the_measurement_reports_only_the_windows_the_chain_declared() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let mut script = Script::default();
    // 部外者（0xAA・0xBB）が鎖の窓の間にも手前にも居る。
    script
        .front_of
        .insert(0x20, vec![0xAAusize, 0x10usize, 0xBBusize]);

    let (out, _used) = run_once(&mut world, script);

    assert_eq!(
        field(lines_with(&out, SETTLED)[0], "measured"),
        "0x10,0x20",
        "鎖の外の窓を実測へ混ぜている"
    );
}
