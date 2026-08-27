//! グループ単位の重なりの維持系——印を消費し、既存のペア機構と調停し、連鎖で是正を出す。
//!
//! design.md「wintf 層 > group 維持系」の 1 巡の手順——**①次巡の実測照合と頭打ち・
//! ②印の門・③調停・④連鎖の発行・⑤印の解除・⑥起床の印**——をここで実装する。
//!
//! ```text
//! 前の巡の発行があれば実測して照合（一致→是正の記録／不一致→検証不一致の記録＋連続失敗）
//!   → 連続失敗が上限に達したグループだけを維持対象から外す（理由と観測値を warn で記録）
//! 印（pending）が立っていない → ここで終わり（観測すらしない）
//!   → 同じ巡にペア機構の是正が在る → 理由つきの見送りを記録して終わり（印は保持）
//!       → 維持対象の各グループを観測 → 是正が要る最初の 1 グループへ連鎖を積む
//!                            → 検証待ちを預ける（照合は次の巡）
//!       → 維持対象が全て成立していれば印を降ろす（外したグループは数えない）
//! ここまでの結果、印か検証待ちが残っていれば → 次の画面更新を促す印を立てる
//! ```
//!
//! # なぜ「印が立っていない巡は観測すらしない」のか（要件 6.1／6.4）
//!
//! グループが 1 本も宣言されていない状態で本系統が毎巡 Win32 の走査を回すと、既定状態
//! （＝非強制）の挙動が本機能の導入前と同じである保証が「結果として指令 0 本だった」に
//! 後退する。門を先頭に置き、受け口
//! （[`plan_group_fixes`]）が空の列を舐めるだけで実測の口を一度も呼ばない形と併せて、
//! **判断の機会がそもそも作られない**ようにしてある。
//!
//! # なぜ 1 巡に 1 グループなのか
//!
//! 実測は巡の中で採るが、指令の実際の書込はその巡の後の flush である。同じ巡に 2 グループ
//! ぶんの連鎖を積むと、後から適用される連鎖の挿入位置は先の適用で既に古い——既存ペア機構が
//! 「1 巡で指令を出すのは高々 1 ペア」に落ち着いたのと同じ理由である
//! （[`zorder_pair_maintain`](super::zorder_pair_maintain) の module doc）。見送られた
//! グループの要求は印として残り、次の巡の新しい実測から計算し直される。
//!
//! グループの**内側**の連鎖は自己参照（`w[i]` を `w[i-1]` の直後へ）なので、この陳腐化とは
//! 無縁である——挿入先は同じ連鎖で今まさに位置が決まる窓であり、外の窓の実測に依存しない。
//! よって連鎖そのものは一括で積んでよい（`DeferWindowPos` の一括投入は積んだ順を保存する
//! ——実窓での実証は `command_batch_tests.rs:633`）。
//!
//! # なぜ「同じ巡にペア機構が是正を出していたら見送る」のか（調停）
//!
//! 上と同じ陳腐化が**系統をまたいで**起きるからである。ペア機構が積んだ 1 本もこの巡の
//! flush で書かれるので、こちらが同じ巡に採った実測は適用の時点では古い。既存機構が自分の
//! 内側で敷いていた「1 巡に窓を動かすのは 1 つ」という規律を、系統間へ広げた形である。
//! 見送っても印は落とさない——次の巡でやり直すためであり、落とせば要件 8.3 が禁じる
//! 「黙って諦める」になる。
//!
//! # なぜ最後に「次も回してほしい」と言うのか（要件 7.4）
//!
//! 表示に変化が無い巡は省略され得る（`tick_wake` の旗と `tick_gate` の判断）。是正の指令
//! が実際に書かれるのは巡の**後**の flush であり、効いたかどうかの照合は**次の巡**でしか
//! 採れない。よって、印か検証待ちが残っている間は毎巡ひとつ旗を立てておかないと、要求が
//! 省略された画面更新の向こうで足踏みする。逆に、済んだ後も立て続ければ省略の仕組みが
//! 実質無効になる——ゆえに旗は⑤の後の状態で判断する（[`wants_wake`]）。
//!
//! # 記録はこのモジュールから出さない
//!
//! `tracing` の出力先は呼び出し元の module path が既定であり、ここでマクロを呼ぶと
//! サインオフの grep 対象（`wintf::ecs::window::zorder_group`）が 2 本に割れる。よって
//! 記録は兄弟の [`zorder_group`](super::zorder_group) が持つ唯一の入口
//! （`record_*`）を呼ぶだけにし、本ファイルにはマクロを 1 つも置かない。この不在は
//! 兄弟テストが本文の走査で毎回確かめている。

use bevy_ecs::prelude::*;
use bevy_ecs::system::NonSendMarker;
use windows::Win32::Foundation::HWND;

use super::SetWindowPosCommand;
use super::zorder_group::{
    GroupFixDecision, GroupProbe, GroupSkipReason, GroupVerify, GroupVerifyOutcome,
    VERIFY_FAIL_CAP, ZOrderGroupSpec, ZOrderGroups, observe_group, plan_group_fixes,
    record_group_decision, record_group_give_up, record_group_skip, record_group_verification,
};
use super::zorder_pair::InsertSpec;
use super::zorder_pair_maintain::{HandleQuery, IssuedPairFix, pair_fix_command};

// ============================================================================
// WorldGroupProbe - 本番の実測の口
// ============================================================================

/// 本番の実測の口——`Entity` から `HWND` を引くところだけを World に依存させる。
///
/// **前面走査は書かない**。[`GroupProbe::scan_in_front`] の既定実装が既存のペア機構の
/// 走査（`measure_windows_in_front`）をそのまま呼ぶので、こちらは
/// [`GroupProbe::resolve`] だけを実装する——「Windows 上で非表示の窓を読み飛ばし、
/// 最も近い可視の隣で測る」という測り方（要件 9.3）をこの型が二重に持たないための形で
/// ある。override して自前の走査を書けば、測り方が 2 通りに分かれた瞬間に隣接の意味が
/// 静かにずれる。**override していないこと**は兄弟テストが本型そのものへ主張している。
pub(crate) struct WorldGroupProbe<'a, 'w, 's> {
    handles: &'a HandleQuery<'w, 's>,
}

impl<'a, 'w, 's> WorldGroupProbe<'a, 'w, 's> {
    /// ハンドルを引くクエリを借りて実測の口を組む。
    pub(crate) fn new(handles: &'a HandleQuery<'w, 's>) -> Self {
        Self { handles }
    }
}

impl GroupProbe for WorldGroupProbe<'_, '_, '_> {
    /// 実体が消えている（`Err`）のと、実体はあるが窓がまだ無い（`Ok(None)`）のを、どちらも
    /// 「まだ現れていない」として `None` に畳む。観測はこの 2 つを区別せず数だけを残し
    /// （要件 8.4 の記録材料）、区別が要る破棄経路は既存ペア機構が持っている。
    fn resolve(&self, entity: Entity) -> Option<HWND> {
        self.handles.get(entity).ok().flatten().map(|h| h.hwnd)
    }
}

// ============================================================================
// 連鎖の発行——位置と寸法を持てない経路で組む
// ============================================================================

/// 連鎖を指令として積む（`chain[i]` を直前の要素の直後へ・先頭は動かさない）。
///
/// 1 本ずつの指令は既存ペア機構の [`pair_fix_command`] で組む。あちらは位置も寸法も
/// 持たない [`WindowPos`](super::WindowPos) から `SetWindowPos` の引数を導くので、
/// `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE` は**組み立て方から従う**——「フラグを
/// 立て忘れない」ではなく「位置と寸法を運ぶ欄がそもそも無い」形であり、これが要件 11.1
/// の構造的な根拠である。指令を自前で組み直せばその保証は消えるので、ここでは組まない。
///
/// 軸は段ごとに進む（`chain[0]` は `head` の直後、`chain[1]` は `chain[0]` の直後）。
/// 挿入先が必ず**同じ連鎖の直前の窓**であることが、構成外の窓を指す余地を無くしている
/// （要件 2.5）——動かす窓も挿入先も、観測が集めた構成窓の列からしか採らない。
fn enqueue_group_chain(head: HWND, chain: &[HWND]) {
    let mut anchor = head;
    for moved in chain {
        SetWindowPosCommand::enqueue(pair_fix_command(*moved, InsertSpec::After(anchor)));
        anchor = *moved;
    }
}

// ============================================================================
// run_group_maintenance_pass - 1 巡の維持（World を持たない中核）
// ============================================================================

/// 1 巡ぶんの維持を回す（World も Win32 も、実測の口の向こう側にしか無い）。
///
/// system 本体（[`apply_zorder_group_maintenance`]）から World 依存を剥がしてあるのは、
/// 照合・門・調停・連鎖の組立・印の解除という**判断の側**を実機も実ディスプレイも無しに固定するためで
/// ある（要件 10.1）。既存ペア機構が観測を構造体へ写して純関数へ渡しているのと同じ分割
/// であり、こちらは列の長さが可変ゆえ「値の写し」ではなく「引く口」を渡す形にしてある。
///
/// `pair_fix_this_pass` は「この巡に既存のペア機構が是正の指令を出したか」である。
/// 真偽値 1 つで受け取るのは、判断がペア機構の内部（どのペアが・どの窓を）を覗かないため
/// ——覗ける形にすると、ペアの中身を見てグループの是正を変える規則を書く場所ができる。
///
/// # ⑥ の旗をこの外側で立てる理由
///
/// ①〜⑤は②の門と③の調停で `return` を使って抜ける。旗をその内側へ置くと、**見送った
/// 巡だけが促さない**——印は残っているのに次の画面更新が省略され、要求が足踏みする。
/// よって①〜⑤を [`run_group_maintenance_steps`] へ畳み、旗はその呼出の後、脱出路の無い
/// ここで 1 度だけ立てる。既存ペア機構が同じ旗を自分の system の中で立てているのと同じく
/// （`zorder_pair_maintain`）、**自分の系統で立てる**——他系統の旗に相乗りすると、
/// 相手が要求を持たない巡に本系統だけが取り残される。
pub(crate) fn run_group_maintenance_pass<P: GroupProbe + ?Sized>(
    groups: &mut ZOrderGroups,
    pair_fix_this_pass: bool,
    probe: &P,
) {
    run_group_maintenance_steps(groups, pair_fix_this_pass, probe);

    // ⑥ 起床の印。是正が終わっていなければ、次の画面更新も回してほしいと言い残す。
    if wants_wake(groups.pending, groups.has_verify()) {
        crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::ZORDER);
    }
}

/// ⑥ この巡の終わりに、次の画面更新を促すか（World も Win32 も触らない純判断）。
///
/// 真になるのは**是正がまだ終わっていない**間である。終わっていないの中身は 2 つあり、
/// どちらも「次の巡に仕事が残っている」ことを意味する。
///
/// - `pending`——是正が要るかもしれないという印が降りていない（②の門が開いたままである）。
/// - `has_verify`——出した連鎖の照合がまだ済んでいない（照合は①＝次の巡でしか採れない）。
///
/// # なぜ 2 つの真偽値の or に名前を付けるのか
///
/// 項を落とした変異（`pending` だけを見る形）は、本番の巡を回すだけの検査では見えない。
/// 名前の付いた判断にしておけば、真理値表そのものを兄弟テストが固定できる。
///
/// # 検証待ちの項は今日の解除条件の下では結果を変えない
///
/// ⑤が印を降ろすのは維持対象の全グループの相対順が成立した巡だけであり、連鎖を出した
/// グループは相対順が成立していない。よって現状「検証待ちが在る ⇒ 印も立っている」が
/// 成り立ち、この項が単独で真になる巡は無い。それでも design の条件どおり両方を見るのは、
/// ⑤の解除条件を変える改修がこの含意を静かに破ったとき、**照合を待っている最中に画面
/// 更新が省略される**という形（要件 7.4 が禁じているもの）で表に出るからである。
fn wants_wake(pending: bool, has_verify: bool) -> bool {
    pending || has_verify
}

/// ①〜⑤（照合・門・調停・連鎖の発行・印の解除）を 1 巡ぶん回す。
///
/// 早期に抜けるのは②の門と③の調停であり、いずれも「この巡は指令を出さない」であって
/// 「もう促さなくてよい」ではない。旗の判断を呼び出し側
/// （[`run_group_maintenance_pass`]）に置いてあるのはそのためである。
fn run_group_maintenance_steps<P: GroupProbe + ?Sized>(
    groups: &mut ZOrderGroups,
    pair_fix_this_pass: bool,
    probe: &P,
) {
    // ① 前の巡に出した連鎖の照合。印の門より前に置く（設計の手順どおり）。
    //
    // 門の前で構わないのは、**検証待ちが預けられている巡は必ず印も立っている**からで
    // ある——検証待ちが付くのは連鎖を出した巡だけであり、その巡のグループは相対順が
    // 成立していないので⑤の解除条件を満たさない。よってここが「印の立っていない巡に
    // 観測する」経路になることはなく、要件 6.1／6.4 の構造的保証（グループが 1 本も
    // 宣言されていなければ実測の口を一度も呼ばない）はそのまま残る。
    verify_previous_issue(groups, probe);

    // ② 印の門。立っていなければ観測も判断も指令も無い（要件 6.1／6.4）。
    // 記録も出さない——「是正が要るかもしれない」と誰も言っていない巡であり、
    // 見送るべき判断がそもそも無いからである（要件 8.3 の対象は判断した上での沈黙）。
    if !groups.pending {
        return;
    }

    // ③ 調停。この巡はペア機構が既に窓を動かす指令を積んでいるので、こちらは出さない。
    // 印は落とさない——次の巡の新しい実測でやり直す（見送りであって断念ではない）。
    // グループを名指しできないのは巡そのものの見送りだからであり、記録の group_id は番兵になる。
    if pair_fix_this_pass {
        record_group_skip(None, GroupSkipReason::PairFixThisPass, None);
        return;
    }

    // ④ 観測して、是正が要る**最初の 1 グループ**へ連鎖を積む。
    //
    // 観測は先に全グループぶんを組む（[`plan_group_fixes`] がグループ間の順序を一切見ずに
    // 1 本ずつ独立に回す）。発行を 1 本に絞るのは観測ではなく**指令の側**であり、
    // 2 本目以降の是正は記録も要求の取り下げも伴わずに次巡へ持ち越される
    // ——印が残っている以上、記録の無い握り潰しにはならない。
    //
    // 観測にかけるのは**維持対象のグループだけ**である。頭打ちで外したグループを列から
    // 落としてから渡すので、外れたグループは実測もされない——「外した」を、指令を出さない
    // 判断ではなく判断の機会そのものの不在として作る（②の門と同じ形）。
    let maintained: Vec<ZOrderGroupSpec> = groups
        .groups
        .iter()
        .filter(|spec| groups.is_maintained(spec.id))
        .cloned()
        .collect();
    let plans = plan_group_fixes(&maintained, probe);
    let mut issued: Option<GroupVerify> = None;
    // ⑤の判定材料。維持対象のうち 1 本でも相対順が成立していなければ印は残る。
    let mut all_ordered = true;

    for plan in plans {
        if !plan.observation.order_ok {
            all_ordered = false;
        }
        // 判断結果は必ず記録の入口を通す——見送りはそこで理由つきの記録になり、
        // 返ってくるのは適用すべき是正だけである（要件 8.3 の規約）。発行済みの巡でも
        // この呼出は飛ばさない。飛ばすと、後ろに居るグループの見送りが**発行したという
        // 理由だけで**記録から消える。
        let Some(decision) = record_group_decision(&plan.observation, plan.decision) else {
            continue;
        };
        let GroupFixDecision::Chain { head, chain } = decision else {
            // 記録の入口は見送りを返さないのでここへは届かない。届いても panic は作らず、
            // 指令を出さずに読み飛ばす（記録は入口が済ませている）。
            continue;
        };
        if issued.is_some() {
            // この巡は既に別のグループの連鎖を積んだ。いま手にしている実測は適用の時点では
            // 古いので、印を残したまま次巡でやり直す。
            continue;
        }
        enqueue_group_chain(head, &chain);
        issued = Some(GroupVerify {
            id: plan.observation.id,
            head,
            chain,
        });
    }

    // 出した連鎖を次巡の照合（①）のために預ける。
    if let Some(verify) = issued {
        groups.arm_verify(verify);
    }

    // ⑤ 印の解除。**維持対象**の全グループの相対順が成立した時点で降ろす。
    //
    // 頭打ちで外したグループを数えないのは、数えると 1 本の不成立が他のグループの静穏
    // まで止め、tick の門が永久に開いたままになるからである（要件 7.4 の起床旗は印が
    // 立つ間ずっと立つ）。外した事実は打ち切りの巡に warn として残っているので、
    // 「黙って諦めた」にはならない（要件 8.3）。
    //
    // 是正を出した巡はここへ来ても降りない——出した相手は相対順が成立していない
    // グループであり、`all_ordered` が偽になっているからである。
    if all_ordered {
        groups.pending = false;
        // 打ち切りの記憶もここで捨てる。印が降りている間は維持系が何もしないので、
        // 外したグループが実際に戻るのは次の追随トリガが印を立て直した巡である。
        groups.clear_all_fail_streaks();
    }
}

// ============================================================================
// verify_previous_issue - 前の巡の発行に対する照合と、グループごとの頭打ち
// ============================================================================

/// 前の巡に出した連鎖を実測と突き合わせ、連続失敗が上限に達したグループを維持対象から外す。
///
/// # なぜ発行の巡ではなくここで照合するのか（要件 9.1／9.2）
///
/// 指令の実際の書込は巡の後の flush であり、発行と同じ巡に採った実測は必ず書込前の値に
/// なる。よって是正の記録（`fix` 行）は**この段でしか出さない**——既存ペア機構の
/// `record_verification`（`zorder_pair.rs`）が同じ理由で 2 段階になっている。出した指令
/// （預かった [`GroupVerify`]）とこの巡の実測が同じ 1 行に載ることが要件 9.1／9.2 の実質で
/// ある。
///
/// # 検証の相手が名簿から消えていたら
///
/// 空のメンバー列として観測する。解決できる窓が 2 枚未満なので走査は行われず、記録は
/// [`GroupVerifyOutcome::NotMeasured`] の見送りになる——`fix` も `verify-failed` も出さず、
/// 連続失敗にも数えない。何も測っていない巡を証跡にしないためであり、環境が是正を
/// 拒んだわけでもないからである。
///
/// # 頭打ちはグループごと（要件 8.2／8.3）
///
/// 連続失敗を数えるのは**検証に失敗したグループ**だけである。1 巡に発行するのは 1 本
/// なので、他のグループの陰で持ち越されているグループはそもそもこの段へ到達せず、
/// 数を持てない——「一度も発行していないグループが頭打ちで外れ、印が降りて、二度と
/// 発行されない」という経路が構造として作れない形にしてある（task 4.1 からの申し送り）。
fn verify_previous_issue<P: GroupProbe + ?Sized>(groups: &mut ZOrderGroups, probe: &P) {
    let Some(verify) = groups.take_verify() else {
        return;
    };
    let spec = groups
        .groups
        .iter()
        .find(|spec| spec.id == verify.id)
        .cloned()
        .unwrap_or(ZOrderGroupSpec {
            id: verify.id,
            members: Vec::new(),
        });
    let observed = observe_group(&spec, probe);

    match record_group_verification(&verify, &observed) {
        GroupVerifyOutcome::Matched => groups.clear_fail_streak(verify.id),
        GroupVerifyOutcome::Mismatched => {
            let streak = groups.note_verify_failure(verify.id);
            if streak >= VERIFY_FAIL_CAP {
                record_group_give_up(verify.id, streak, &observed);
            }
        }
        // 測っていないので、成立とも失敗とも数えない（記録は入口が済ませている）。
        GroupVerifyOutcome::NotMeasured => {}
    }
}

// ============================================================================
// apply_zorder_group_maintenance - 維持系 system
// ============================================================================

/// 維持系: 前の巡の発行を照合し、印を消費し、ペア機構と調停し、連鎖で是正を出す。
///
/// # 受け口が挿さっていない巡
///
/// 結線前の状態である。既存ペア機構がストラテジ未挿入を既定値で受け流しているのと同じく、
/// ここも何もせずに戻る——グループが宣言されていない状態と結果は同じ（指令 0 本）であり、
/// 異常終了させる理由が無い。
///
/// # UI スレッド固定
///
/// [`NonSendMarker`] を取るのは、実測の口の既定実装が Win32（`GetWindow` 走査）を呼び、
/// 積んだ指令のキューがスレッドローカルで UI スレッドを前提にするためである。この印が
/// 付いた system をスケジュール実行器はメインスレッド以外で走らせない。
///
/// # スケジュールへの結線
///
/// 本 system をどの並びに挿すかは後続タスクが決める。設計上の位置は既存ペア機構の
/// **直後**であり、[`IssuedPairFix`] がこの巡に付いたかどうかを見て調停するには、
/// ペア機構が先に走っている必要がある。
pub fn apply_zorder_group_maintenance(
    _ui_thread: NonSendMarker,
    groups: Option<ResMut<ZOrderGroups>>,
    // この巡にペア機構が是正を出した窓（`Added` なので前の巡に付いたものは映らない）。
    pair_fixes: Query<(), Added<IssuedPairFix>>,
    // すべての実体に当たるクエリ（実測の口がハンドルを引くために借りる）。
    handles: HandleQuery,
) {
    let Some(mut groups) = groups else {
        return;
    };
    let probe = WorldGroupProbe::new(&handles);
    run_group_maintenance_pass(&mut groups, !pair_fixes.is_empty(), &probe);
}

#[cfg(test)]
#[path = "zorder_group_maintain_tests.rs"]
mod zorder_group_maintain_tests;

#[cfg(test)]
#[path = "zorder_group_verify_tests.rs"]
mod zorder_group_verify_tests;

#[cfg(test)]
#[path = "zorder_group_wake_tests.rs"]
mod zorder_group_wake_tests;
