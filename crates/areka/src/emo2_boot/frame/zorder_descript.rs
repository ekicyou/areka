//! shell 設定（`seriko.zorder`）由来の**基底**グループを起動の段で据える層
//! （design「descript 起動時適用（main.rs シーム）」・要件 5.1／5.2／5.3／5.4）。
//!
//! 台本のタグ（`\![set,zorder,…]`）は会話の途中でいつでも届くが、shell の設定は
//! ゴーストが起こされた瞬間に一度だけ読まれる。届く時刻が違うだけで**意味は同じ**なので、
//! ここは解釈器を持たない——タグと同一の純関数 [`parse_zorder_tokens`] を通し、受理した
//! 要素列を [`ZOrderGroupLedger::set_descript_base`] へ渡すだけである（要件 5.2）。
//! 別の解釈器を新設すると、同じ字面が入口によって違う意味を持つ日が来る。
//!
//! # 1 つの設定は 1 つのグループ（要件 5.3）
//!
//! 値はカンマで区切って要素へ落とす。`seriko.zorder,1,0` の kv 解析は既に
//! 「キー＝`seriko.zorder`・値＝`1,0`」まで済ませているので、ここが受け取るのは
//! 要素だけを並べた文字列である。区切りの結果が何個であっても載るグループは高々 1 本で、
//! 基底が高々 1 つであることは台帳の側が保つ。
//!
//! # 解釈できないときは 1 本も載せず、値と理由を残して起動を続ける（要件 5.4／8.3）
//!
//! 拒否は [`log_group_rejected`]（warn 水準＝`logging.md` の「無効なパラメーター」区分）
//! で残す。載せるのは**受け取った値そのもの**と拒否理由の両方である——どちらか一方では、
//! 作者は何を書き間違えたのかを記録から復元できない。台帳は 1 バイトも動かさず、呼び手は
//! そのまま起動を続ける（この関数は失敗を返す口を持たない＝起動を止める手段が無い）。
//!
//! # 設定が無い運転では 1 行も出さない
//!
//! `seriko.zorder` を書いていないゴーストは既定状態（グループ 0 本）で動く。これは失敗でも
//! 見送りでもなく「そもそも指定が無い」なので、記録も残さない（要件 6.1／6.4 の既定＝
//! 非強制は、判断ではなくこの**不在**で成り立つ）。
//!
//! # 記録の出力先は wintf の唯一の入口
//!
//! 受理も拒否も [`log_group_applied`]／[`log_group_rejected`] を通す。こちらで `tracing` の
//! マクロを呼ぶと出力先（module path）が割れ、実機サインオフの grep 対象が 2 本になる
//! （兄弟の [`zorder_drain`](super::zorder_drain) と同じ規律）。

use wintf::ecs::window::{log_group_applied, log_group_rejected};

use crate::placement::zorder_group_ledger::{GroupSource, ZOrderGroupLedger, parse_zorder_tokens};

use super::zorder_drain::{reject_reason_text, set_applied_detail};

/// 設定の値を要素へ割る区切り（タグの引数分割と同じ粒度）。
const SEPARATOR: char = ',';

/// shell 設定由来の基底を台帳へ据える（要件 5.1／5.2／5.3／5.4）。
///
/// `raw` は `seriko.zorder` の値そのもの（`placement::config` の `zorder_raw`）。`None` は
/// 「設定が無い」であり、台帳にも記録にも一切触れない。
///
/// 呼ぶのは起動の段でちょうど 1 回である（`wire_emo2_boot` →
/// [`Emo2Wiring::seed_zorder_descript_base`](super::Emo2Wiring::seed_zorder_descript_base)）。
/// タグの取り出しの相はまだ 1 度も走っていないので、ここで据えた基底は**最初の維持の巡**から
/// 効く（要件 5.1「タグの実行を待たずに」）。
///
/// # 受理の記録が `action=set` を名乗る理由
///
/// 本文は兄弟の [`set_applied_detail`] をそのまま使う。行の欄を二重に持つと、片方だけを
/// 直した日に記録の書式が静かに割れるからである。起動由来かタグ由来かは `source` 欄が
/// 一意に弁別する——タグ経由の追加は必ず [`GroupSource::Tag`] を付けるので、
/// `action=set source=Descript` はこの関数からしか出ない。
pub(super) fn apply_descript_base(ledger: &mut ZOrderGroupLedger, raw: Option<&str>) {
    let Some(raw) = raw else {
        // 設定が無い＝既定状態。失敗でも見送りでもないので記録も残さない。
        return;
    };

    let tokens: Vec<&str> = raw.split(SEPARATOR).collect();
    let (members, normalizations) = match parse_zorder_tokens(&tokens) {
        Ok(parsed) => parsed,
        Err(reject) => {
            // 受け取った値と理由の両方を残し、グループは 1 本も載せずに起動を続ける（要件 5.4）。
            log_group_rejected(&reject_reason_text(&reject), raw);
            return;
        }
    };

    ledger.set_descript_base(members);
    // 受理の記録は**台帳に載った後の内容**から組む（兄弟の相と同じ流儀）。基底は高々 1 つ
    // なので、出所で引けば一意に定まる。
    let detail = ledger
        .groups()
        .iter()
        .find(|group| group.source == GroupSource::Descript)
        .map(|group| set_applied_detail(group, &normalizations));
    if let Some(detail) = detail {
        log_group_applied(&detail);
    }
}

#[cfg(test)]
#[path = "zorder_descript_tests.rs"]
mod tests;
