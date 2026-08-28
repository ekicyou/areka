//! バルーンの再表示への追随（要件 7.3）。
//!
//! 「再表示」とはここでは**合成層の shown エッジ**を指す（裁定済みの定義）。Windows 上の
//! 窓は出しっぱなしのまま、中身の絵だけを消して描き直す——だから窓の表示状態は動かず、
//! 表示状態の変化を待つ仕組みは 1 度も呼ばれない。描き直した窓が自分から他の窓の手前や
//! 背面へ移ることは無いが、隠れていた間に外から重なりが崩されている可能性があるので、
//! 描き直した直後に「是正が要るかもしれない」という印を立てて次の巡を促す。
//!
//! # 何を固定するか
//!
//! 1. **判断**——引くか引かないかを純関数
//!    [`wants_group_follow_on_show`](super::wants_group_follow_on_show) として切り出し、
//!    真理値表そのものを固定する（項を 1 つ落とす変異がここで赤くなる）。
//! 2. **状態**——印は立つ側にしか動かない。受け口が無い巡は何もしない（作りもしない）。
//! 3. **結線**——その判断が**本番の発行経路に結びついている**ことを本文の字面で固定する
//!    （呼出はちょうど 1 つ・表示を発行した後・成否を渡している・非表示の経路には無い）。
//!
//! 判断だけを試験しても「呼ばれていること」は誰も見ていない（task 4.2 の教訓＝単体の檻は
//! 結線の檻の代わりにならない）。位置と引数まで押さえるのは、印を立てる場所や渡す値が
//! ずれても**振る舞いだけを読む檻は 1 本も赤くならない**からである（task 3.1／5.1 の教訓）。
//!
//! # なぜ起床の旗そのものを読まないのか
//!
//! 旗（`tick_wake`）はプロセスに 1 組しかなく、検査は並列に走る。しかも `ZORDER` を立てる
//! のはここだけではない（ペア機構・グループ維持系・窓位置の受理・Z 指令の受け口が同じ
//! ビットを立て、それらの検査は共有の錠を取らない）。ゆえに共有の旗の上では「立っていない」
//! も「立っている」も証拠にならない——どちらも走行のたびに結論が変わりうる
//! （`tick_wake_tests.rs:15-17`・task 4.3 が敷いた作法）。よって旗は字面で押さえる。

use bevy_ecs::prelude::{Entity, World};
use wintf::ecs::window::{ZOrderGroupSpec, ZOrderGroups};

use super::{note_balloon_shown, wants_group_follow_on_show};

// ===========================================================================
// ⑴ 判断——真理値表
// ===========================================================================

/// トリガを引くのは「実際に可視になった」かつ「グループが 1 本でもある」ときだけである。
///
/// 4 通りすべてを置くのは、項を 1 つ落とす変異（成否を見ない／宣言の有無を見ない）が
/// 片側の入力しか置かない檻では素通りするからである（task 1.2 の教訓）。
#[test]
fn the_reshow_trigger_fires_only_when_a_balloon_became_visible_while_a_group_exists() {
    assert!(
        wants_group_follow_on_show(true, true),
        "再表示したのに追随しない（隠れている間に崩された重なりが戻らない・要件 7.3）"
    );
    assert!(
        !wants_group_follow_on_show(false, true),
        "表示が実らなかったのに追随している（描き直していない巡で維持系が起きる）"
    );
    assert!(
        !wants_group_follow_on_show(true, false),
        "グループが 1 本も無いのに追随している（既定状態＝非強制の巡で維持系が起きる・要件 6.4）"
    );
    assert!(
        !wants_group_follow_on_show(false, false),
        "表示もしていないし宣言も無いのに追随している"
    );
}

// ===========================================================================
// ⑵ 状態——印は立つ側にしか動かない
// ===========================================================================

/// グループを 1 本だけ持つ受け口を載せた world を組む（印は降りた状態から始める）。
fn world_with_group(id: u32) -> World {
    let mut world = World::new();
    let members: Vec<Entity> = (0..2).map(|_| world.spawn_empty().id()).collect();
    let mut groups = ZOrderGroups::default();
    groups.groups.push(ZOrderGroupSpec { id, members });
    world.insert_resource(groups);
    world
}

/// 受け口だけを載せた world を組む（グループの宣言は 1 本も無い＝既定状態）。
fn world_without_group() -> World {
    let mut world = World::new();
    world.insert_resource(ZOrderGroups::default());
    world
}

/// 印の現在値を読む（受け口が無ければ `None`）。
fn pending_of(world: &World) -> Option<bool> {
    world.get_resource::<ZOrderGroups>().map(|g| g.pending)
}

/// 印を立てた状態にする（檻の前提づくり）。
fn raise_mark(world: &mut World) {
    world.resource_mut::<ZOrderGroups>().pending = true;
}

/// 実った再表示は印を立て、実らなかった表示と既定状態は立てない。
#[test]
fn a_reshown_balloon_raises_the_mark_and_a_failed_show_does_not() {
    // 実った再表示——立つ。
    let mut shown = world_with_group(71);
    assert_eq!(
        pending_of(&shown),
        Some(false),
        "檻の前提が崩れている（受け口は印の降りた状態から始まる）"
    );
    note_balloon_shown(&mut shown, true);
    assert_eq!(
        pending_of(&shown),
        Some(true),
        "再表示で印が立たない（隠れている間に崩れた重なりが戻らない・要件 7.3）"
    );

    // 実らなかった表示——立たない。
    let mut failed = world_with_group(72);
    note_balloon_shown(&mut failed, false);
    assert_eq!(
        pending_of(&failed),
        Some(false),
        "表示が実らなかった巡に印が立っている（描き直していないのに維持系が起きる）"
    );

    // 既定状態（宣言が 1 本も無い）——立たない。
    let mut bare = world_without_group();
    note_balloon_shown(&mut bare, true);
    assert_eq!(
        pending_of(&bare),
        Some(false),
        "宣言が 1 本も無いのに印が立っている（既定状態の挙動が導入前と変わる・要件 6.4）"
    );
}

/// トリガは印を降ろさない——引かなかった巡も、立っている印はそのまま残る。
///
/// 安全側の不変条件は「検証待ちがある ⇒ 印が立っている」であり、印を降ろす口は
/// 維持系の⑤（維持対象の全グループの相対順が成立した巡）ただ 1 つである。トリガが
/// `pending = false` を書くと、未処理の是正要求が**誰にも記録されずに**消える。
#[test]
fn the_reshow_trigger_never_lowers_a_raised_mark() {
    // 実らなかった表示——引かないが、既に立っている印は落とさない。
    let mut failed = world_with_group(81);
    raise_mark(&mut failed);
    note_balloon_shown(&mut failed, false);
    assert_eq!(
        pending_of(&failed),
        Some(true),
        "実らなかった巡に印が落ちている（未処理の是正要求が記録も無く消える）"
    );

    // 宣言の無い巡も同じ（印を降ろすのは維持系の⑤だけ）。
    let mut bare = world_without_group();
    raise_mark(&mut bare);
    note_balloon_shown(&mut bare, true);
    assert_eq!(
        pending_of(&bare),
        Some(true),
        "宣言の無い巡にトリガが印を落としている（印を降ろす口はここではない）"
    );
}

/// 受け口がまだ挿さっていない巡は何もしない——落ちないし、受け口を勝手に作りもしない。
///
/// グループ機構を使わない構成（Z 順のタグも descript 由来の基底も無いゴースト）では
/// `ZOrderGroups` が world に載らない。ここで受け口を作ると、宣言が 1 本も無いのに
/// 受け口だけが生え、以後の「受け口の有無」を見る判断の意味が変わる。
#[test]
fn a_world_without_the_group_sink_is_left_untouched() {
    let mut world = World::new();
    note_balloon_shown(&mut world, true);
    assert!(
        world.get_resource::<ZOrderGroups>().is_none(),
        "受け口の無い world にトリガが受け口を作っている（宣言が無いのに機構が起きる）"
    );
}

// ===========================================================================
// ⑶ 結線——判断が本番の発行経路に結びついている
// ===========================================================================

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
///
/// **対照は必ずこの側へ当てる**。落とす前の本文に当てると、説明文に綴りがあるだけで
/// 「在る」と読み、コード行を全部消しても緑のまま通る（task 3.1 の教訓）。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 空白の連なりを 1 つに詰める（改行や字下げの入り方で檻が壊れないようにする）。
fn squeeze(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 本文に現れるはずの字面の位置（見つからなければその場で落とす）。
fn index_of(source: &str, needle: &str) -> usize {
    source
        .find(needle)
        .unwrap_or_else(|| panic!("本番の字面 `{needle}` が見つからない（檻の前提が崩れている）"))
}

/// `issue_show` の署名（位置の錨として 2 本のテストが使う）。
const ISSUE_SHOW_SIGNATURE: &str =
    "fn issue_show(presenter: &mut EmoPresenter, world: &mut World, scope: u32) -> bool {";

/// トリガの呼出はちょうど 1 つで、表示を発行した後に、その**成否**を渡して呼ばれている。
///
/// 押さえるのは 6 点である。
///
/// 1. 判断もトリガも素の `fn`——rustc の可視性そのものが、判断を迂回する第 2 の本番経路を
///    禁じる。
/// 2. 呼出はちょうど 1 つ（定義と合わせて 2 回の出現）。
/// 3. 判断と 2 つの作用（印・旗）が 1 つの `if` に閉じている。無条件に立てれば既定状態の
///    静穏が死に、片方だけ立てれば「印はあるのに画面更新が省略される」形になる。
/// 4. 位置——表示の成否を畳み込んだ**後**であり、`issue_show` の本体の中に在る。前へ
///    移すと、まだ描き直していない巡に印が立つ（表示に変化の無い巡を省く門が実質無効に
///    なる。`draw-load-parity` の実測＝門 ON で定常 3.30%・門 OFF で 17.04%）。
/// 5. 引数——渡すのは**実際に可視になったか**を畳んだ `shown` であり、定数ではない。
///    `true` を直に渡す変異は、全透明への退化や発行失敗の巡でも印を立てる。
/// 6. 印を降ろす字面がトリガ側に 1 つも無い。
///
/// **この 6 点はいずれも振る舞いに現れない**。引数を `true` に替えても最終的な重なりは
/// 正しいままだし、呼出を消しても次の是正契機が来れば直る。だから字面で押さえる。
#[test]
fn the_reshow_trigger_is_wired_once_after_the_show_and_receives_the_outcome() {
    let code = code_only(include_str!("balloon_visibility_phase.rs"));

    assert!(
        code.contains("fn wants_group_follow_on_show("),
        "説明文を落とす処理が本文まで落としている"
    );

    // 判断もトリガもモジュール私設である。`pub(crate)` へ広げると、この檻が数える
    // 「呼出はちょうど 1 つ」は本ファイルしか走査していないので、他モジュールに生えた
    // 第 2 の本番経路を 1 本も捕まえられない。可視性そのものを錠に使う。
    assert!(
        code.contains("\nfn wants_group_follow_on_show("),
        "判断がモジュール外から呼べる（判断を迂回する第 2 の生産者を作れてしまう）"
    );
    assert!(
        code.contains("\nfn note_balloon_shown("),
        "トリガがモジュール外から呼べる（要件 7.3 の生産者がちょうど 1 つでなくなる）"
    );
    assert_eq!(
        code.matches("wants_group_follow_on_show(").count(),
        2,
        "判断の定義と呼出が 1 対 1 でない（判断を迂回する第 2 の経路がある疑い）"
    );
    assert_eq!(
        code.matches("note_balloon_shown(").count(),
        2,
        "トリガの定義と呼出が 1 対 1 でない（要件 7.3 の生産者はちょうど 1 つ）"
    );

    // 判断と 2 つの作用が 1 つの `if` に閉じている。
    assert!(
        squeeze(&code).contains(
            "if wants_group_follow_on_show(shown, !groups.groups.is_empty()) { groups.pending = true; tick_wake::mark(tick_wake::ZORDER); }"
        ),
        "印と旗が判断に守られていない（無条件なら既定状態の静穏が死に、片方だけなら要求が足踏みする）"
    );
    assert_eq!(
        code.matches("tick_wake::mark(tick_wake::ZORDER)").count(),
        1,
        "ZORDER の旗を立てる呼出がちょうど 1 つでない"
    );
    assert_eq!(
        code.matches("groups.pending = true;").count(),
        1,
        "印を立てる書込がちょうど 1 つでない"
    );

    // 印を降ろす経路を作らない（安全側の不変条件＝検証待ちがある ⇒ 印が立っている）。
    assert!(
        !code.contains("pending = false"),
        "トリガ側に印を降ろす書込がある（是正の要求が記録も無く消える）"
    );

    // 渡すのは成否であって定数ではない。
    assert!(
        code.contains("note_balloon_shown(world, shown);"),
        "トリガへ渡しているのが表示の成否 `shown` でない（定数を渡すと実らなかった巡にも印が立つ）"
    );
    assert!(
        !code.contains("note_balloon_shown(world, true)"),
        "トリガへ定数 `true` を渡している（全透明への退化・発行失敗の巡でも印が立つ）"
    );

    // 位置——成否を畳み込んだ後、`issue_show` の本体の中、そして戻り値の直前。
    let issue_show_at = index_of(&code, ISSUE_SHOW_SIGNATURE);
    let show_target_at = index_of(&code, "let issued = presenter.show_target(world, target);");
    let outcome_at = index_of(&code, "let shown = match issued {");
    let call_at = index_of(&code, "note_balloon_shown(world, shown);");
    let next_fn_at = index_of(&code, "fn roll_back_show(");
    assert!(
        issue_show_at < show_target_at && show_target_at < outcome_at,
        "檻の前提が崩れている（発行={show_target_at}・成否の畳み込み={outcome_at}）"
    );
    assert!(
        outcome_at < call_at,
        "トリガが表示の成否を畳み込む前に在る（描き直していない巡にも印が立つ・成否={outcome_at}・トリガ={call_at}）"
    );
    assert!(
        call_at < next_fn_at,
        "トリガが `issue_show` の本体の外に在る（トリガ={call_at}・次の関数={next_fn_at}）"
    );
    assert!(
        squeeze(&code).contains("note_balloon_shown(world, shown); shown }"),
        "印を立てた値と `issue_show` が返す値が同じ経路で結ばれていない（片方だけ差し替える変異が通る）"
    );
}

/// トリガは**表示の経路にだけ**在る——非表示の発行では引かない。
///
/// 非表示は既存の漏斗 `PresentCommand::Hide` を通る。そこへ印を立てると、消えた巡ごとに
/// 維持系が起きて、表示に変化の無い巡を省く門が実質無効になる。呼出がちょうど 1 つで
/// あることと合わせて、その 1 つが `Hide` の側に無いことを位置で押さえる。
#[test]
fn the_reshow_trigger_is_absent_from_the_hide_path() {
    let code = code_only(include_str!("balloon_visibility_phase.rs"));
    let hide_at = index_of(&code, "PresentCommand::Hide {");
    let issue_show_at = index_of(&code, ISSUE_SHOW_SIGNATURE);
    let call_at = index_of(&code, "note_balloon_shown(world, shown);");
    assert!(
        hide_at < issue_show_at && issue_show_at < call_at,
        "トリガが非表示の漏斗の側に在る（消えた巡ごとに維持系が起きる・非表示={hide_at}・表示={issue_show_at}・トリガ={call_at}）"
    );
}
