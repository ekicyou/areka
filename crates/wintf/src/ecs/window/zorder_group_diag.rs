//! グループ系の診断記録の**行を組む純関数**だけを集めた層（要件 9.1／9.2／9.5）。
//!
//! design.md「group diag（`zorder_group_diag.rs`）」が正本であり、本モジュールはその
//! レコード表をそのまま写した書式を 1 箇所に閉じ込める。実機サインオフの grep 判定語と
//! 出力書式は 1:1 で対応するため、書式が意図せず変われば手順が静かに嘘になる——組立を
//! 純関数へ切り出してテストで語彙を固定し、記録を出す側はその戻り値をそのまま本文にする
//! （組立を二重に持たない）。既存ペア機構の [`zorder_pair_diag`](super::zorder_pair_diag)
//! と同じ分割であり、規律もそちらの module doc が正本である。
//!
//! # ここに置いてよいもの・置いてはならないもの
//!
//! 置いてよいのは `tracing` のマクロを 1 つも含まない**純粋な文字列組立**だけである。
//! `log_*`／`record_*`（マクロ呼出を含む）は兄弟の [`zorder_group`](super::zorder_group) に
//! 残す——`tracing` の出力先は呼び出し元の module path が既定であり、こちらへ移すと
//! サインオフの grep 対象（`wintf::ecs::window::zorder_group`）が分裂する。
//! この不在は兄弟テストが本文の走査で毎回確かめている（不在は書き足された瞬間に静かに
//! 崩れるので、目視やレビューでは守れない）。
//!
//! # 既存ペア機構の 6 タグとは独立の新設である（要件 9.5）
//!
//! グループ系のタグは `[zorder-group]` を冠に持ち、`[zorder-pair]` の 6 タグとは一語も
//! 重ならない。あちらの 5 ファイルは**無編集**であり、語彙・フィールド名・出力先は
//! いずれも本モジュールの新設によって動かない。隣接する仕様があちらを読み続けられる形を
//! 構造で保つのが要件 9.5 の実質であり、こちらが名乗りを横取りしないことがその半分である。
//!
//! # フィールドは落とさず番兵で埋める
//!
//! 値が取れなかったときに欄ごと落とすと、「記録が出ていない」のと「その経路にはその値が
//! 無い」の区別が事後に付かなくなる。よってすべての欄は必ず `field=value` の形で現れ、
//! 値が無いときは [`UNKNOWN`] になる（既存ペア機構と同じ字面）。

use windows::Win32::Foundation::HWND;

use super::zorder_group::{GroupObservation, GroupSkipReason, GroupVerify};
use super::zorder_pair_diag::hwnd_field;

/// 値が取得できなかったフィールドの番兵（既存ペア機構と同じ字面）。
pub(crate) const UNKNOWN: &str = "-";

/// 受理の記録タグ（台帳が指定を受け入れた事実・debug 水準）。
const APPLIED_TAG: &str = "[zorder-group] applied";
/// 是正の記録タグ（指令と実測を同一行に載せる・debug 水準・検証段でのみ発行・要件 9.1／9.2）。
const FIX_TAG: &str = "[zorder-group] fix";
/// 見送りの記録タグ（理由必須・debug 水準・要件 8.3）。
const SKIP_TAG: &str = "[zorder-group] skip";
/// 検証不一致の記録タグ（error 水準・要件 8.2）。
const VERIFY_FAILED_TAG: &str = "[zorder-group] verify-failed";
/// 拒否の記録タグ（warn 水準・要件 8.1／8.3）。
///
/// 見送り（[`SKIP_TAG`]）とは別の概念である——あちらは観測の結果「今は動かさない」と
/// 決めた [`GroupSkipReason`] を伴う判断であり、こちらは作者の書いた指定そのものを
/// **受け付けなかった**という入力側の事実である。1 つのタグへ潰すと、記録を読む者が
/// 「エンジンが様子を見た」と「指定が捨てられた」を区別できなくなる。
const REJECTED_TAG: &str = "[zorder-group] rejected";

/// グループ系の記録タグ 5 種（サインオフの grep 判定語の一覧）。
///
/// 定数を個別に公開せず一覧の形で返すのは、「5 種であること」「冠を共有すること」
/// 「互いに異なること」をテストが 1 か所で主張できるようにするためである。
///
/// 本番の記録行は上の定数を直接使うので、この一覧は**テストの覗き窓**にすぎない
/// （`#[cfg(test)]` にしてあるのはそのため——本番に未使用の関数を残さない）。
/// 定数そのものを指しているので、タグを書き換えれば必ずここも動く＝二重帳簿にならない。
#[cfg(test)]
pub(crate) fn group_record_tags() -> [&'static str; 5] {
    [
        APPLIED_TAG,
        FIX_TAG,
        SKIP_TAG,
        VERIFY_FAILED_TAG,
        REJECTED_TAG,
    ]
}

/// 真偽値をログ用表現へ（判定そのものが取れなかった場合は番兵）。
///
/// 「取れなかった」を `false` へ潰さない——潰すと「測ったが偽だった」という**観測**と
/// 「測っていない」という**観測の欠落**が同じ字面になる。既存ペア機構の
/// `tristate_field`（`zorder_pair_diag.rs`）と同じ規律であり、あちらの 5 ファイルは
/// 無編集ゆえ関数そのものは共有できないので、字面だけを合わせてこちらにも置く。
pub(crate) fn tristate_field(value: Option<bool>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => UNKNOWN.to_string(),
    }
}

/// 窓の列をログ用の 1 フィールドへ（空列は番兵）。
fn hwnd_list_field(hwnds: &[HWND]) -> String {
    if hwnds.is_empty() {
        return UNKNOWN.to_string();
    }
    hwnds
        .iter()
        .map(|h| hwnd_field(Some(*h)))
        .collect::<Vec<_>>()
        .join(",")
}

/// 連鎖の各段を「動かした窓@挿入先」の列へ（空列は番兵）。
///
/// 段ごとに軸が進む——`chain[0]` は `head` の直後、`chain[1]` は `chain[0]` の直後、と
/// 続く。指令の意味そのものを行に書き出すので、記録を読むだけで「どの窓をどの窓のすぐ
/// 手前へ移したか」が復元できる（要件 9.1）。
fn moves_field(head: HWND, chain: &[HWND]) -> String {
    if chain.is_empty() {
        return UNKNOWN.to_string();
    }
    let mut anchor = head;
    let mut parts = Vec::with_capacity(chain.len());
    for moved in chain {
        parts.push(format!(
            "{}@{}",
            hwnd_field(Some(*moved)),
            hwnd_field(Some(anchor))
        ));
        anchor = *moved;
    }
    parts.join(",")
}

/// 呼び出し側から受け取った自由文を 1 フィールドへ畳む（空は番兵）。
///
/// 空白を `_` へ潰すのは、`field=value` の 1 行から**機械的に切り出せる**状態を呼び出し側の
/// 行儀に依存させないためである。この関数を通る 2 つの欄（拒否理由・受け取ったトークン列）は
/// areka 側で組んだ文字列であり、`wintf → areka` の import が禁止されている以上こちらは
/// 中身の型を知り得ない——素通しにすると、向こうの文字列に空白が 1 つ混じった日に
/// サインオフの切り出しが静かに壊れる。
///
/// 畳み込みは**意図的に不可逆**である（`"a b"` と `"a_b"` は記録の上で区別が付かなくなる）。
/// 記録から元の字面を復元できることより、1 行から機械的に切り出せることを優先した判断で
/// ある——この 2 欄の読み手はサインオフの grep であり、区別が要るのは値そのものではなく
/// 「どの理由でどのトークン列が落ちたか」だからである。
fn text_field(value: &str) -> String {
    let folded = value.split_whitespace().collect::<Vec<_>>().join("_");
    if folded.is_empty() {
        return UNKNOWN.to_string();
    }
    folded
}

/// 受理の記録行（純関数）——台帳が組んだ本文へ、こちらはタグだけを貼る。
///
/// 台帳の内容そのものは areka の型であり、`wintf → areka` の import は禁止ゆえここでは
/// 受け取れない。よって組み上がった本文を受け取り、タグと（呼び出し側の module path 既定
/// による）出力先だけをこちらが与える——サインオフの grep 対象を 1 本に保つための形である。
pub(crate) fn applied_line(detail: &str) -> String {
    format!("{APPLIED_TAG} {detail}")
}

/// 是正の記録行（純関数）——**出した指令と、その後の実測を同じ 1 行に載せる**。
///
/// 行を「指令」と「実測」に分けないのは design.md の裁定である（既存ペア機構
/// [`zorder_pair_diag::fix_line`](super::zorder_pair_diag::fix_line) と同じ規律）。
/// 分けると「指令は出したが効かなかった」の判定が 2 行の突合になり、過去に同型の誤診を
/// 生んでいる。
///
/// 載る 4 要素は要件 9.1／9.2 が求めるものそのままである。
///
/// - `group_id`: どのグループの是正か（記録の結合キー）
/// - `head`: 動かさなかった軸の窓（連鎖の起点）
/// - `moves`: 動かした窓と、その挿入先（段ごとに「窓@挿入先」）
/// - `measured`: **検証巡の前面走査が実際に出会った**構成窓の列（手前から順）
///
/// `measured` の出所は [`GroupObservation::measured_front`]——すなわち Win32 の走査が
/// 実際に辿った並びであり、宣言された並び（[`GroupObservation::hwnds`]）ではない。
/// 宣言を載せると、実際の重なりがどう違っていても同じ字面が出てしまい、
/// 「どの窓がどの窓のすぐ手前に着いたか」に答えられない行になる（要件 9.1／9.2）。
/// 範としている既存ペア機構の `measured_next_after_fix`（`zorder_pair_diag::fix_line`）も
/// 本物の実測ハンドルである。
///
/// この行が**検証段でしか出ない**ことも要件 9.2 の一部である——指令の書込は巡後の flush で
/// 起きるため、発行と同巡の実測は必ず書込前の値になり証跡に使えない。相対順が実測で
/// 成立しなかった場合はこの行ではなく [`verify_failed_line`] が出る。
///
/// 宣言列を併記しないのは、この行が出る条件（`order_ok`）の下では実測列が宣言列と
/// **必ず一致する**からである（走査が宣言どおりの順で全メンバーに出会えたときだけ成立する）。
/// 食い違いが有り得るのは不一致の側だけなので、対比は [`verify_failed_line`] が持つ。
pub(crate) fn fix_line(verify: &GroupVerify, observed: &GroupObservation) -> String {
    format!(
        "{FIX_TAG} group_id={id} head={head} moves={moves} measured={measured}",
        id = verify.id,
        head = hwnd_field(Some(verify.head)),
        moves = moves_field(verify.head, &verify.chain),
        measured = hwnd_list_field(&observed.measured_front),
    )
}

/// 検証不一致の記録行（純関数・error 水準で出す・要件 8.2）。
///
/// 是正の行の 4 欄に加えて、**宣言された並び**（`members`）と未解決の枚数を載せる。
///
/// - `members` と `measured` を並べるのは、不一致の行こそ「期待した並び」と「実際の並び」の
///   対比が要るからである（既存ペア機構の `verify_failed_line` が `expected_*` と `measured`
///   を同一行に並べているのと同じ形）。名前を分けてあるのは、宣言列が実測を騙らないためで
///   ある——`measured` に載るのは走査が実際に出会った並びだけである。
/// - `missing` を載せるのは、不一致の原因が「指令が効かなかった」のか「そもそも窓が
///   揃っていなかった」のかを 1 行で切り分けられるようにするためである。
/// - `scan_complete` を載せるのは、`measured` に現れないメンバーが「測ったら別の場所に
///   居た」のか「そこまで測れなかった」のかを**この 1 行で**切り分けるためである。
///   前面走査は上限（512 枚）で打ち切られることがあり、打ち切られた巡は
///   [`FrontScan::reached_top`](super::zorder_pair::FrontScan) が偽になる。打ち切り
///   そのものは走査層が warn に残すが、それは**別の出力先の 2 行目**であり、突き合わせて
///   初めて解ける形は本行の設計理由（1 行で読める）が戒めているものそのものである。
///   走査を行わなかった巡は番兵（`-`）——「測っていない」を `false` へ潰さない。
pub(crate) fn verify_failed_line(verify: &GroupVerify, observed: &GroupObservation) -> String {
    format!(
        "{VERIFY_FAILED_TAG} group_id={id} head={head} moves={moves} \
         members={members} measured={measured} missing={missing} \
         scan_complete={scan_complete}",
        id = verify.id,
        head = hwnd_field(Some(verify.head)),
        moves = moves_field(verify.head, &verify.chain),
        members = hwnd_list_field(&observed.hwnds),
        measured = hwnd_list_field(&observed.measured_front),
        missing = observed.missing,
        scan_complete = tristate_field(observed.scan_complete),
    )
}

/// 見送りの記録行（純関数・理由必須・要件 8.3）。
///
/// `group_id` が `None` なのは巡そのものの見送り（既存ペア機構との調停）であり、
/// `observed` が `None` なのは観測より前に見送った場合である——どちらも欄は落とさず
/// 番兵にする。
///
/// 理由語は [`GroupSkipReason`] の `Debug` 表現をそのまま用いる。4 種が互いに異なる語で
/// あることがそのまま「理由を伴う見送り」の実質になる——1 語へ潰れると記録があっても
/// 理由が読めない。語そのものは兄弟テストが逐語で固定しているので、列挙子の改名は
/// 記録の書式の変更として赤くなる。
pub(crate) fn skip_line(
    group_id: Option<u32>,
    reason: GroupSkipReason,
    observed: Option<&GroupObservation>,
) -> String {
    let group_id = match group_id {
        Some(id) => id.to_string(),
        None => UNKNOWN.to_string(),
    };
    let (resolved, missing, order_ok) = match observed {
        Some(obs) => (
            obs.hwnds.len().to_string(),
            obs.missing.to_string(),
            obs.order_ok.to_string(),
        ),
        None => (
            UNKNOWN.to_string(),
            UNKNOWN.to_string(),
            UNKNOWN.to_string(),
        ),
    };
    format!(
        "{SKIP_TAG} group_id={group_id} reason={reason:?} \
         resolved={resolved} missing={missing} order_ok={order_ok}"
    )
}

/// 拒否の記録行（純関数・warn 水準で出す・要件 8.1／8.3）。
///
/// 載るのは**拒否理由**と**受け取ったトークン列**の 2 欄である。トークン列を載せるのは、
/// 作者が何を書いたのかが記録から復元できなければ書き間違いを直せないからであり
/// （要件 8.1 は「そのタグによる変更を一切行わず、拒否理由を記録する」）、理由を載せる
/// のは「黙って無視された」を禁じる要件 8.3 の実質そのものである。
///
/// どちらも**組み上がった文字列**で受け取る。拒否理由の型（areka の `ZOrderReject`）は
/// areka 側にあり、`wintf → areka` の import は禁止だからである（[`applied_line`] と
/// 同じ形）。素通しにせず [`text_field`] を通すので、向こうの文字列に空白が混じっても
/// 1 行からの切り出しは壊れない。
pub(crate) fn rejected_line(reason: &str, tokens: &str) -> String {
    format!(
        "{REJECTED_TAG} reason={reason} tokens={tokens}",
        reason = text_field(reason),
        tokens = text_field(tokens),
    )
}

#[cfg(test)]
#[path = "zorder_group_diag_tests.rs"]
mod zorder_group_diag_tests;
