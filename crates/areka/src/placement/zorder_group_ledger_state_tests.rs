//! [`ZOrderGroupLedger`] の状態遷移の決定論檻——グループの追加・グループをまたぐ
//! 再指定の拒否・解除と descript 基底への復帰（要件 1.4／1.5／3.1／3.2／3.3／3.7／
//! 4.1／4.2／4.3／5.3／5.5・design「Testing Strategy / Unit」3・design「Data Models /
//! Domain Model」の不変条件⑴⑷）。
//!
//! トークン解釈そのものの檻は兄弟ファイル `zorder_group_ledger_tests.rs`（`t_zgp*`）
//! にある。本ファイルは台帳の状態遷移だけを見るので、接頭辞を `t_zgl*` で分ける。
//!
//! 実機・実ディスプレイ・World を一切必要としない（要件 10.1）。台帳は各テストが
//! 自前で作る値であり、テストどうしが状態を共有しない（要件 10.3）。
//!
//! # 拒否の檻が示すこと
//!
//! 拒否分岐の檻はすべて [`assert_ledger_unchanged`] を通す。この道具は台帳**全体**
//! （グループ列・各グループの ID・要素列・出所・次に配る ID・版）を丸ごと比較する
//! ので、task 1.3 の完了状態「拒否が返ったケースで台帳の内容が呼び出し前と完全に
//! 一致する」を欄ごとの抜き取りではなく構造で押さえる（要件 3.2／8.1）。
//!
//! # 「動かない」主張は両側から挟む
//!
//! 版（[`ZOrderGroupLedger::version`]）は「増えない」側だけを見ると、そもそも一度も
//! 増えない実装が素通りする。[`t_zgl15_version_moves_only_when_the_contents_move`] は
//! 増える 3 経路と増えない 3 経路を同じ 1 本で挟む（task 1.2 の差し戻しの教訓）。

use super::*;

// ---------------------------------------------------------------------------
// 檻の道具
// ---------------------------------------------------------------------------

/// バルーン窓の要素（`balloonN`／`bN`）。
fn b(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Balloon,
    }
}

/// キャラ窓の要素（`surfaceN`／`sN`）。
fn s(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Char,
    }
}

/// トークン列を解釈して要素列を得る。
///
/// 台帳の入口へ渡すのは**必ず解釈を通った要素列**である（呼び出し規約）。檻も同じ
/// 経路を通すことで、要件 3.3「明示モードの `sN`／`bN` と数値モードの `N` を同一
/// スコープを指すものとして扱う」が入口から出口まで通しで成立することを見る。
fn accept(tokens: &[&str]) -> Vec<GroupElement> {
    match parse_zorder_tokens(tokens) {
        Ok((elements, _)) => elements,
        Err(reject) => panic!("受理されるべき指定が解釈で拒否された: {tokens:?} reject={reject:?}"),
    }
}

/// タグ由来のグループとして追加を試みる。
fn add_tag(ledger: &mut ZOrderGroupLedger, tokens: &[&str]) -> Result<u32, ZOrderReject> {
    ledger.try_add_tag_group(accept(tokens))
}

/// 追加の受理を期待し、割り当てられたグループ ID を取り出す。
fn expect_added(ledger: &mut ZOrderGroupLedger, tokens: &[&str]) -> u32 {
    match add_tag(ledger, tokens) {
        Ok(id) => id,
        Err(reject) => panic!("受理されるべきタグが拒否された: {tokens:?} reject={reject:?}"),
    }
}

/// 再指定としての拒否を期待し、伴うスコープ列を取り出す。
///
/// 台帳が呼び出し前と完全に一致することも同時に確かめる（task 1.3 の完了状態）。
fn expect_cross_reject(ledger: &mut ZOrderGroupLedger, tokens: &[&str]) -> Vec<u32> {
    let before = ledger.clone();
    let reject = match add_tag(ledger, tokens) {
        Ok(id) => panic!("拒否されるべきタグが受理された: {tokens:?} id={id}"),
        Err(reject) => reject,
    };
    assert_ledger_unchanged(&before, ledger, &format!("tokens={tokens:?}"));

    match reject {
        ZOrderReject::CrossGroupRedesignation { scopes } => scopes,
        other => panic!("再指定以外の理由で拒否された: {tokens:?} reject={other:?}"),
    }
}

/// descript 由来の基底を据える。
fn set_base(ledger: &mut ZOrderGroupLedger, tokens: &[&str]) {
    ledger.set_descript_base(accept(tokens));
}

/// 台帳が呼び出し前と**完全に**一致することを確かめる。
///
/// 欄を抜き取らず値そのものを比べる。グループ列・ID・要素列・出所に加えて、外から
/// は見えない「次に配る ID」と版も含まれるので、「拒否したが ID だけ 1 つ消費した」
/// 「拒否したが版だけ進んだ」という形の部分適用も赤になる（要件 8.1）。
fn assert_ledger_unchanged(before: &ZOrderGroupLedger, after: &ZOrderGroupLedger, ctx: &str) {
    assert_eq!(
        before, after,
        "拒否された呼び出しが台帳を変えている: {ctx}\nbefore={before:?}\nafter={after:?}"
    );
}

/// グループの要素列だけを取り出す（ID を伏せた比較のため）。
fn members_of(ledger: &ZOrderGroupLedger) -> Vec<Vec<GroupElement>> {
    ledger
        .groups()
        .iter()
        .map(|group| group.members.clone())
        .collect()
}

/// 行コメントを落とした本文（説明文の中の字面を数えないため）。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// 初期状態
// ---------------------------------------------------------------------------

/// 何も指定されていない台帳は空である（要件 6.1 の既定状態＝グループ 0 本の側）。
#[test]
fn t_zgl1_fresh_ledger_is_empty() {
    let ledger = ZOrderGroupLedger::default();

    assert!(
        ledger.groups().is_empty(),
        "既定状態にグループは 1 本も無い"
    );
    assert_eq!(ledger.version(), 0, "版は 0 から始まる");
}

// ---------------------------------------------------------------------------
// 追加（要件 3.1）
// ---------------------------------------------------------------------------

/// 既存のどのグループとも重複しないスコープだけを含むタグは、既存グループを保った
/// まま新しいグループとして加わる（要件 3.1）。
#[test]
fn t_zgl2_non_overlapping_tags_accumulate_as_separate_groups() {
    let mut ledger = ZOrderGroupLedger::default();

    let first = expect_added(&mut ledger, &["1", "0"]);
    let second = expect_added(&mut ledger, &["b3", "s3", "b2", "s2"]);

    assert_ne!(first, second, "グループ ID は別々に配られる");
    assert_eq!(ledger.groups().len(), 2, "2 本が併存する");

    assert_eq!(ledger.groups()[0].id, first);
    assert_eq!(
        ledger.groups()[0].members,
        vec![b(1), s(1), b(0), s(0)],
        "先に加えたグループの要素列は 2 本目の追加で動かない"
    );
    assert_eq!(ledger.groups()[0].source, GroupSource::Tag);

    assert_eq!(ledger.groups()[1].id, second);
    assert_eq!(ledger.groups()[1].members, vec![b(3), s(3), b(2), s(2)]);
    assert_eq!(ledger.groups()[1].source, GroupSource::Tag);
}

/// 台帳は解釈が返した要素列をそのまま持つ。同一スコープの 2 窓が
/// `[Balloon, Char]` の隣接ブロックであるという不変条件⑶は、解釈の側で既に
/// 成立している（task 1.2）ものを台帳が壊さない、という形で保たれる。
#[test]
fn t_zgl3_the_ledger_stores_the_normalized_element_list_verbatim() {
    let mut ledger = ZOrderGroupLedger::default();

    // 反転（`s1,b1`）と非隣接（間に b2）を含む指定。解釈の側で寄せられている。
    expect_added(&mut ledger, &["s1", "b2", "b1"]);

    assert_eq!(
        ledger.groups()[0].members,
        accept(&["s1", "b2", "b1"]),
        "台帳は解釈の結果を組み替えない"
    );
    assert_eq!(
        ledger.groups()[0].members,
        vec![b(1), s(1), b(2)],
        "正規化済みの並びがそのまま載る"
    );
}

// ---------------------------------------------------------------------------
// グループをまたぐ再指定の拒否（要件 3.2／3.3）
// ---------------------------------------------------------------------------

/// 既にいずれかのグループに属しているスコープを含むタグは、タグ全体を採用せず、
/// 既存グループを一切変更しない（要件 3.2）。
#[test]
fn t_zgl4_redesignating_a_scope_rejects_the_whole_tag() {
    let mut ledger = ZOrderGroupLedger::default();
    expect_added(&mut ledger, &["1", "0"]);

    let scopes = expect_cross_reject(&mut ledger, &["0", "5"]);

    assert_eq!(scopes, vec![0], "拒否は該当スコープを伴う");
    assert_eq!(ledger.groups().len(), 1, "グループは増えない");
}

/// 再指定の判定は**スコープ**で行う。明示モードの `sN`／`bN` と数値モードの `N` は
/// 同じスコープを指すものとして突き合わせる（要件 3.3）。
///
/// 解釈（`parse_zorder_tokens`）から台帳まで通しで見る。モードの区別は解釈の時点で
/// 要素へ畳まれているので、両方向とも同じ結論に落ちなければならない。
#[test]
fn t_zgl5_numeric_and_explicit_modes_denote_the_same_scope() {
    // 数値モードで押さえたスコープを、明示モードで取りに行く。
    let mut numeric_first = ZOrderGroupLedger::default();
    expect_added(&mut numeric_first, &["1", "0"]);
    assert_eq!(
        expect_cross_reject(&mut numeric_first, &["b1", "s5"]),
        vec![1],
        "`b1` は数値モードのスコープ 1 と衝突する"
    );
    assert_eq!(
        expect_cross_reject(&mut numeric_first, &["s1", "b5"]),
        vec![1],
        "`s1` も同じスコープ 1 を指す"
    );

    // 明示モードで押さえたスコープを、数値モードで取りに行く。
    let mut explicit_first = ZOrderGroupLedger::default();
    expect_added(&mut explicit_first, &["b1", "s1", "b0", "s0"]);
    assert_eq!(
        expect_cross_reject(&mut explicit_first, &["1", "4"]),
        vec![1],
        "数値モードの `1` は明示モードのスコープ 1 と衝突する"
    );

    // 窓の片方だけを押さえている場合も、判定はスコープ単位である。
    let mut half = ZOrderGroupLedger::default();
    expect_added(&mut half, &["b1", "s0"]);
    assert_eq!(
        expect_cross_reject(&mut half, &["s1", "b6"]),
        vec![1],
        "`b1` しか載っていなくてもスコープ 1 は塞がっている"
    );
}

/// 拒否は衝突した**全ての**スコープを、要素列に現れた順で 1 度ずつ伴う（要件 3.2）。
#[test]
fn t_zgl6_rejection_names_every_colliding_scope_once_in_order() {
    let mut ledger = ZOrderGroupLedger::default();
    expect_added(&mut ledger, &["1", "0"]);
    expect_added(&mut ledger, &["b2", "s2"]);

    // 要素列は正規化後 [b2, b1, s1, s0]＝スコープの初出順は 2 → 1 → 0。
    let scopes = expect_cross_reject(&mut ledger, &["b2", "s1", "b1", "s0"]);

    assert_eq!(
        scopes,
        vec![2, 1, 0],
        "衝突したスコープが初出順に並び、スコープ 1 は 2 窓あっても 1 度だけ載る"
    );
}

/// 一部だけが衝突するタグも全体を採用しない（要件 3.2 の「タグ全体」）。
/// 衝突していなかった側のスコープは台帳に載らず、**空いたまま**である。
#[test]
fn t_zgl7_a_partially_overlapping_tag_is_rejected_in_full() {
    let mut ledger = ZOrderGroupLedger::default();
    expect_added(&mut ledger, &["1", "0"]);

    expect_cross_reject(&mut ledger, &["1", "5"]);

    // 巻き添えを食ったスコープ 5 は塞がれていない＝部分適用が起きていない。
    expect_added(&mut ledger, &["5", "6"]);
    assert_eq!(
        members_of(&ledger),
        vec![vec![b(1), s(1), b(0), s(0)], vec![b(5), s(5), b(6), s(6)],],
        "拒否されたタグの要素は 1 つも台帳へ入っていない"
    );
}

// ---------------------------------------------------------------------------
// descript 基底（要件 5.3／5.5）
// ---------------------------------------------------------------------------

/// descript 由来の基底は高々 1 つである（要件 5.3・不変条件⑷）。据え直すと
/// 前の基底は残らない。
#[test]
fn t_zgl8_at_most_one_descript_base_is_kept() {
    let mut ledger = ZOrderGroupLedger::default();

    set_base(&mut ledger, &["1", "0"]);
    let first_id = ledger.groups()[0].id;
    set_base(&mut ledger, &["b3", "s3", "b2", "s2"]);

    assert_eq!(ledger.groups().len(), 1, "基底は 1 本だけ");
    assert_eq!(ledger.groups()[0].source, GroupSource::Descript);
    assert_eq!(
        ledger.groups()[0].members,
        vec![b(3), s(3), b(2), s(2)],
        "後から据えた基底が残る"
    );
    assert_ne!(
        ledger.groups()[0].id,
        first_id,
        "据え直した基底には新しい ID を配る（ID は再利用しない）"
    );
}

/// descript 由来のグループも、タグの再指定の拒否判定に参加する（要件 5.5）。
#[test]
fn t_zgl9_the_descript_base_participates_in_the_redesignation_check() {
    let mut ledger = ZOrderGroupLedger::default();
    set_base(&mut ledger, &["1", "0"]);

    assert_eq!(
        expect_cross_reject(&mut ledger, &["b1", "s4"]),
        vec![1],
        "基底が押さえているスコープはタグから取り直せない"
    );

    // 基底に無いスコープは従来どおり受け付ける（拒否が基底以外へ広がっていない）。
    expect_added(&mut ledger, &["4", "5"]);
    assert_eq!(ledger.groups().len(), 2);
}

/// 基底を据えることは「基底そのものへ戻す」ことである（正典沈黙箇所の裁量）。
///
/// 要件 5.1 は基底の適用を**起動時・タグ実行より前**と定めており、タグ由来の
/// グループと衝突する状況は正典の経路では起きない。design の署名は拒否を返す口を
/// 持たない（`-> ()`）ので、衝突しても黙って落とす／不変条件⑴を破る／のどちらも
/// 採れない。ここでは終状態を [`ZOrderGroupLedger::reset_to_descript`] と一致させる
/// ——「基底へ戻った状態」は要件 4.1 が既に定義している唯一の形だからである。
#[test]
fn t_zgl10_setting_the_base_re_establishes_the_ledger_at_that_base() {
    let mut ledger = ZOrderGroupLedger::default();
    expect_added(&mut ledger, &["1", "0"]);
    expect_added(&mut ledger, &["3", "2"]);

    set_base(&mut ledger, &["b1", "s1", "b9", "s9"]);

    assert_eq!(ledger.groups().len(), 1, "タグ由来のグループは残らない");
    assert_eq!(ledger.groups()[0].source, GroupSource::Descript);
    assert_eq!(ledger.groups()[0].members, vec![b(1), s(1), b(9), s(9)]);

    // 解除しても同じ形＝据え直しの終状態は「基底へ戻った状態」と一致する。
    let after_set = ledger.clone();
    ledger.reset_to_descript();
    assert_eq!(
        ledger.groups(),
        after_set.groups(),
        "基底の据え直しの終状態は reset_to_descript の終状態と同じ"
    );

    // 落ちたスコープは空いている（3.1 の受理側へ戻っている）。
    expect_added(&mut ledger, &["3", "2"]);
}

/// 空の要素列は「基底なし」として扱う。
///
/// `parse_zorder_tokens` が受理する要素列は必ず 2 個以上（要件 1.6）なので、
/// 空列は正典の経路では届かない。届いた場合に 0 要素のグループを台帳へ載せると、
/// 要件 4.2 の「基底が無ければ既定状態へ戻る」が「0 要素の基底へ戻る」に化けて
/// 意味が変わる——載せない側へ倒す。
#[test]
fn t_zgl11_an_empty_base_means_no_base_at_all() {
    let mut ledger = ZOrderGroupLedger::default();
    set_base(&mut ledger, &["1", "0"]);

    ledger.set_descript_base(Vec::new());

    assert!(ledger.groups().is_empty(), "0 要素のグループは載せない");

    // 基底が無い状態なので、解除は既定状態（空）へ戻る（要件 4.2）。
    expect_added(&mut ledger, &["1", "0"]);
    ledger.reset_to_descript();
    assert!(ledger.groups().is_empty());
}

// ---------------------------------------------------------------------------
// 解除（要件 4.1／4.2／4.3）
// ---------------------------------------------------------------------------

/// 解除はタグ由来のグループを全て落とし、descript 基底へ戻す（要件 4.1）。
/// 基底は ID ごと生き残る＝落とすのはタグ由来だけである。
#[test]
fn t_zgl12_reset_drops_tag_groups_and_returns_to_the_descript_base() {
    let mut ledger = ZOrderGroupLedger::default();
    set_base(&mut ledger, &["1", "0"]);
    let base_id = ledger.groups()[0].id;
    expect_added(&mut ledger, &["3", "2"]);
    expect_added(&mut ledger, &["5", "4"]);
    assert_eq!(ledger.groups().len(), 3);

    ledger.reset_to_descript();

    assert_eq!(ledger.groups().len(), 1, "残るのは基底 1 本だけ");
    assert_eq!(ledger.groups()[0].id, base_id, "基底は据え直されない");
    assert_eq!(ledger.groups()[0].members, vec![b(1), s(1), b(0), s(0)]);
    assert_eq!(ledger.groups()[0].source, GroupSource::Descript);
}

/// descript の指定が無ければ、解除は既定状態（グループ 0 本）へ戻す（要件 4.2）。
#[test]
fn t_zgl13_reset_without_a_base_returns_to_the_default_state() {
    let mut ledger = ZOrderGroupLedger::default();
    expect_added(&mut ledger, &["1", "0"]);
    expect_added(&mut ledger, &["3", "2"]);

    ledger.reset_to_descript();

    assert!(ledger.groups().is_empty(), "既定状態＝グループ 0 本へ戻る");
}

/// 解除の後は、解除前のグループに属していたスコープを再指定の拒否対象とせず、
/// 新しい組み合わせとして受け付ける（要件 4.3）。基底が押さえたスコープは
/// 解除で解放されない（要件 5.5 は解除後も効き続ける）——両側から挟む。
#[test]
fn t_zgl14_reset_frees_tag_scopes_but_not_base_scopes() {
    let mut ledger = ZOrderGroupLedger::default();
    set_base(&mut ledger, &["7", "6"]);
    expect_added(&mut ledger, &["1", "0"]);
    expect_added(&mut ledger, &["3", "2"]);

    ledger.reset_to_descript();

    // 解放された側: 解除前は別々のグループだった 0 と 3 を 1 本へ組み直せる。
    expect_added(&mut ledger, &["3", "0"]);
    assert_eq!(
        members_of(&ledger),
        vec![vec![b(7), s(7), b(6), s(6)], vec![b(3), s(3), b(0), s(0)],]
    );

    // 解放されない側: 基底のスコープは依然として塞がっている。
    assert_eq!(expect_cross_reject(&mut ledger, &["s6", "b8"]), vec![6]);
}

// ---------------------------------------------------------------------------
// 版と ID
// ---------------------------------------------------------------------------

/// 版は台帳の内容が実際に動いたときだけ進む。
///
/// 「進まない」側だけを見ると一度も進まない実装が素通りするので、進む 3 経路と
/// 進まない 3 経路を同じ 1 本で挟む（task 1.2 の差し戻しの教訓）。
#[test]
fn t_zgl15_version_moves_only_when_the_contents_move() {
    let mut ledger = ZOrderGroupLedger::default();
    let start = ledger.version();

    // 進む①: 受理された追加。
    expect_added(&mut ledger, &["1", "0"]);
    let after_add = ledger.version();
    assert!(after_add > start, "受理された追加は版を進める");

    // 進まない①: 拒否された追加。
    expect_cross_reject(&mut ledger, &["0", "5"]);
    assert_eq!(
        ledger.version(),
        after_add,
        "拒否された追加は版を進めない（射影を組み直す理由が無い）"
    );

    // 進む②: タグ由来を落とす解除。
    ledger.reset_to_descript();
    let after_reset = ledger.version();
    assert!(after_reset > after_add, "落とすものがある解除は版を進める");

    // 進まない②: 落とすものが無い解除。
    ledger.reset_to_descript();
    assert_eq!(
        ledger.version(),
        after_reset,
        "空の台帳をもう一度解除しても内容は動かない"
    );

    // 進まない③: 空の台帳へ空の基底を据える。
    ledger.set_descript_base(Vec::new());
    assert_eq!(
        ledger.version(),
        after_reset,
        "何も載らない据え直しは版を動かさない"
    );

    // 進む③: 中身のある基底を据える。
    set_base(&mut ledger, &["3", "2"]);
    assert!(ledger.version() > after_reset, "基底が載れば版は進む");
}

/// グループ ID はセッション内で単調増加し、解除で空いても再利用しない
/// （design「Data Models / Domain Model」）。
#[test]
fn t_zgl16_group_ids_increase_and_are_never_reused() {
    let mut ledger = ZOrderGroupLedger::default();

    let first = expect_added(&mut ledger, &["1", "0"]);
    let second = expect_added(&mut ledger, &["3", "2"]);
    assert!(second > first, "ID は単調増加する");

    ledger.reset_to_descript();
    let third = expect_added(&mut ledger, &["1", "0"]);

    assert!(
        third > second,
        "解除で空いた ID を配り直さない: first={first} second={second} third={third}"
    );
}

// ---------------------------------------------------------------------------
// 上限なし・窓の存在を知らない・保存しない
// ---------------------------------------------------------------------------

/// グループ数にも要素数にも上限検査を設けない（要件 3.7）。
#[test]
fn t_zgl17_no_upper_bound_on_group_or_member_count() {
    let mut ledger = ZOrderGroupLedger::default();

    // グループ数: 重ならないスコープ対を 64 本。
    for pair in 0..64u32 {
        let tokens = [(pair * 2).to_string(), (pair * 2 + 1).to_string()];
        let borrowed: Vec<&str> = tokens.iter().map(String::as_str).collect();
        expect_added(&mut ledger, &borrowed);
    }
    assert_eq!(ledger.groups().len(), 64);

    // 要素数: 1 本のグループへ 128 スコープ＝窓 256 枚。
    let mut wide = ZOrderGroupLedger::default();
    let tokens: Vec<String> = (1000..1128u32).map(|n| n.to_string()).collect();
    let borrowed: Vec<&str> = tokens.iter().map(String::as_str).collect();
    expect_added(&mut wide, &borrowed);
    assert_eq!(wide.groups()[0].members.len(), 256);
}

/// 台帳は窓が存在するかどうかを知らない（要件 1.4）。窓がまだ現れていない
/// スコープもグループに残り続け、取り除かれるのは解除のときだけである。
///
/// 型の形でも押さえる: 本体には `Entity` も `HWND` も現れない——実機・実ディスプレイ
/// 無しで全分岐を検査できる根拠がこれである（要件 10.1）。
#[test]
fn t_zgl18_the_ledger_never_learns_whether_a_window_exists() {
    let mut ledger = ZOrderGroupLedger::default();

    // 窓が 1 枚も無いスコープ番号でも、そのまま載って残る。
    expect_added(&mut ledger, &["4242", "4243"]);
    assert_eq!(
        ledger.groups()[0].members,
        vec![b(4242), s(4242), b(4243), s(4243)],
        "窓の存在を確かめる口が無いので、要素は書かれたまま残る"
    );

    // 取り除く経路は解除だけ。
    ledger.reset_to_descript();
    assert!(ledger.groups().is_empty());

    let code = code_only(include_str!("zorder_group_ledger.rs"));
    assert!(
        !code.contains("Entity"),
        "台帳本体が Entity を知っている（射影は drain 相の担当）"
    );
    assert!(
        !code.contains("HWND"),
        "台帳本体が HWND を知っている（実測は wintf 側の担当）"
    );
}

/// 台帳は保存の仕組みに接続しない（要件 1.5）。ゴーストの終了で消えることは
/// 「何もしない」ことで成立するので、**接続していないこと**を本文の走査で守る。
#[test]
fn t_zgl19_the_ledger_is_not_connected_to_persistence() {
    let code = code_only(include_str!("zorder_group_ledger.rs"));

    assert!(
        !code.contains("persist"),
        "台帳本体が保存層へ接続している（要件 1.5 は接続しないことで成立する）"
    );
    assert!(
        !code.contains("serde") && !code.contains("Serialize") && !code.contains("Deserialize"),
        "台帳本体に直列化が入っている（保存経路の入口になる）"
    );

    // 走査が中身を見ている対照——落とし過ぎていないこと。
    assert!(
        code.contains("pub struct ZOrderGroupLedger"),
        "走査対象がずれている（本文が読めていない）"
    );
}

// ---------------------------------------------------------------------------
// 完了状態: 拒否の全経路で台帳が完全に一致する
// ---------------------------------------------------------------------------

/// task 1.3 の完了状態の総取り。台帳がどんな状態にあっても、拒否された呼び出しの
/// 前後で台帳**全体**が完全に一致する（要件 3.2／8.1 の部分適用禁止）。
///
/// 前状態を 4 通り（空／タグ 1 本／基底のみ／基底＋タグ 2 本）用意し、それぞれで
/// 再指定の拒否を起こす。`assert_ledger_unchanged` はグループ列だけでなく
/// 「次に配る ID」と版も比べるので、拒否の途中で ID を 1 つ消費する形も赤になる。
#[test]
fn t_zgl20_every_rejected_call_leaves_the_ledger_completely_identical() {
    // 前状態①: タグ 1 本。
    let mut only_tag = ZOrderGroupLedger::default();
    expect_added(&mut only_tag, &["1", "0"]);

    // 前状態②: 基底のみ。
    let mut only_base = ZOrderGroupLedger::default();
    set_base(&mut only_base, &["1", "0"]);

    // 前状態③: 基底＋タグ 2 本。
    let mut mixed = ZOrderGroupLedger::default();
    set_base(&mut mixed, &["1", "0"]);
    expect_added(&mut mixed, &["3", "2"]);
    expect_added(&mut mixed, &["5", "4"]);

    for (label, ledger) in [
        ("タグ 1 本", &mut only_tag),
        ("基底のみ", &mut only_base),
        ("基底＋タグ 2 本", &mut mixed),
    ] {
        for tokens in [
            ["1", "0"].as_slice(),         // 丸ごと同じ指定
            ["b1", "s1"].as_slice(),       // 明示モードで同じスコープ
            ["0", "9"].as_slice(),         // 片方だけ衝突
            ["s0", "b9", "s9"].as_slice(), // 明示モードで片方だけ衝突
        ] {
            let before = ledger.clone();
            let reject = add_tag(ledger, tokens).expect_err(&format!(
                "拒否されるべきタグが受理された: 前状態={label} tokens={tokens:?}"
            ));
            assert!(
                matches!(reject, ZOrderReject::CrossGroupRedesignation { .. }),
                "再指定以外の理由で拒否された: 前状態={label} tokens={tokens:?} reject={reject:?}"
            );
            assert_ledger_unchanged(
                &before,
                ledger,
                &format!("前状態={label} tokens={tokens:?}"),
            );
        }
    }
}

/// グループ列の並びは「基底が先頭・以降はタグの追加順」で決まる（決定論のため）。
///
/// これは**読み出しの順序**であって前後関係の規則ではない。異なるグループどうしの
/// 相対的な前後関係を固定の規則で決めないこと（要件 3.6）は維持系の担当であり、
/// 台帳の並びはそこへ何も含意しない。
#[test]
fn t_zgl21_groups_are_listed_base_first_then_in_addition_order() {
    let mut ledger = ZOrderGroupLedger::default();

    // 先にタグ、後から基底——それでも読み出しは基底が先頭に来る。
    expect_added(&mut ledger, &["3", "2"]);
    set_base(&mut ledger, &["1", "0"]);
    expect_added(&mut ledger, &["5", "4"]);
    expect_added(&mut ledger, &["7", "6"]);

    let sources: Vec<GroupSource> = ledger.groups().iter().map(|g| g.source).collect();
    assert_eq!(
        sources,
        vec![GroupSource::Descript, GroupSource::Tag, GroupSource::Tag],
        "基底が先頭・タグ由来が追加順で続く"
    );
    assert_eq!(
        members_of(&ledger),
        vec![
            vec![b(1), s(1), b(0), s(0)],
            vec![b(5), s(5), b(4), s(4)],
            vec![b(7), s(7), b(6), s(6)],
        ]
    );
}
