//! メニューの項目名と表示可否の照会（areka-P0-popup-menu-minimal）。
//!
//! 枠ごとの SHIORI リソース名の表、1 回の表示で問い合わせる名前の列挙、kanade への
//! 照会の送出、返り値から項目名への写し、`popupmenu.visible` による表示可否の判定を置く。
//!
//! 照会は「送る」と「返事を写す」の 2 つに分かれている。[`send_query`] は送るだけで待たず
//! （待つと右クリックの処理が返らず画面が止まる）、返事を覗くのは毎 tick 動く仕掛け
//! （[`super::trigger`]）で、集まった返り値を [`interpret`] が項目名と表示可否へ写す。
//!
//! 値は持ち越さない。メニューを出すたびに全部引き直す（要件 3.2）。

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Duration;

use areka_actor::{ReplyReceiver, reply_channel};
use areka_kanade::KanadeMsg;
use areka_kanade::resources::ResourceOutcome;

use super::{Frame, ItemBody, MenuItem};

/// 枠と SHIORI リソース名と既定名の対応（要件 3.1）。
///
/// 並びは [`Frame::ORDER`] と同じで、判別値がそのまま添字になる（[`resource_for`]・
/// [`default_label`] はその前提で引く。並びが崩れたら兄弟テストが赤になる）。
/// 既定名は正典が定めていないので本仕様の裁定である。
///
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html
pub(crate) const FRAME_CAPTIONS: &[(&str, Frame, &str)] = &[
    ("ghostrootbutton.caption", Frame::Ghost, "ゴースト"),
    ("shellrootbutton.caption", Frame::Shell, "シェル"),
    ("balloonrootbutton.caption", Frame::Balloon, "バルーン"),
    ("updatebutton.caption", Frame::Update, "ネットワーク更新"),
    (
        "ghostinstallbutton.caption",
        Frame::Install,
        "インストール…",
    ),
    ("readmebutton.caption", Frame::Readme, "説明書"),
    ("closebutton.caption", Frame::Close, "終了"),
];

/// 本体側（スコープ 0）の窓でメニューを出してよいかを尋ねる名前（要件 3.6）。
///
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#sakura.popupmenu.visible:1
pub(crate) const SAKURA_POPUPMENU_VISIBLE: &str = "sakura.popupmenu.visible";

/// 相方側（スコープ 1）の窓でメニューを出してよいかを尋ねる名前（要件 3.6）。
///
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#kero.popupmenu.visible:1
pub(crate) const KERO_POPUPMENU_VISIBLE: &str = "kero.popupmenu.visible";

/// 正典にはあるが本仕様が**問い合わせない**名前（要件 3.6・3.8）。
///
/// `char*.popupmenu.visible` は n≧2 のキャラクター窓が α に無いので尋ね先が無い。
/// `*.popupmenu.type` の 3 名は、値 `1`（省略メニュー）の中身を正典が定めていないため
/// どの値でも同じメニューを出す——値で中身が変わらないなら尋ねる意味が無く、往復も増やさない。
///
/// この表は運行（kanade）の許可表に 1 つも入っていないこと（＝実際に送られないこと）を
/// 兄弟テストが確かめる。
///
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html
pub(crate) const UNQUERIED_POPUPMENU_RESOURCES: &[&str] = &[
    "char*.popupmenu.visible",
    "sakura.popupmenu.type",
    "kero.popupmenu.type",
    "char*.popupmenu.type",
];

/// 返事を待つ上限。超えたら全件を既定名にしてメニューを出す（要件 3.4）。
pub(crate) const QUERY_TIMEOUT: Duration = Duration::from_millis(1000);

/// 枠の項目名に使う SHIORI リソース名。
pub(crate) fn resource_for(frame: Frame) -> &'static str {
    row(frame).0
}

/// 枠の既定名（リソースが引けなかったときに使う）。
pub(crate) fn default_label(frame: Frame) -> &'static str {
    row(frame).2
}

/// 枠の行を [`FRAME_CAPTIONS`] から取る。
fn row(frame: Frame) -> &'static (&'static str, Frame, &'static str) {
    let row = &FRAME_CAPTIONS[frame as usize];
    debug_assert_eq!(row.1, frame, "対応表の並びが枠の並び順と食い違っている");
    row
}

/// スコープ n の窓が使う表示可否のリソース名（要件 3.6）。
///
/// α のキャラクター窓はスコープ 0 と 1 だけ（`char_scope` と同じ前提）なので n≧2 は来ない。
/// 万一 release ビルドで n≧2 が来たら相方側の名前を返す（新しい問い合わせ先は作らない）。
/// 多キャラクターの窓を作る spec は、`char*.popupmenu.visible` の扱いと合わせてここを見直すこと。
pub(crate) fn visible_resource_for(scope: u32) -> &'static str {
    debug_assert!(scope <= 1, "α のキャラクター窓はスコープ 0 と 1 だけ");
    if scope == 0 {
        SAKURA_POPUPMENU_VISIBLE
    } else {
        KERO_POPUPMENU_VISIBLE
    }
}

/// 1 回の表示で問い合わせる名前を並べる（要件 3.2・3.6）。
///
/// 先頭が表示可否の名前、続いて写しに現れた `caption_resource` を出た順に並べる
/// （サブメニューの子も辿る）。同じ名前は 1 つに潰す——往復を無駄に増やさないためで、
/// 返り値は名前で引くので 1 件あれば足りる。
pub(crate) fn query_ids(snapshot: &[(Frame, MenuItem)], scope: u32) -> Vec<&'static str> {
    let mut ids = vec![visible_resource_for(scope)];
    for (_, item) in snapshot {
        collect_caption_resources(item, &mut ids);
    }
    ids
}

/// 項目とその子が使うリソース名を `ids` へ足す（既にあるものは足さない）。
fn collect_caption_resources(item: &MenuItem, ids: &mut Vec<&'static str>) {
    if let Some(id) = item.caption_resource
        && !ids.contains(&id)
    {
        ids.push(id);
    }
    if let ItemBody::Submenu(children) = &item.body {
        for child in children {
            collect_caption_resources(child, ids);
        }
    }
}

/// 照会で決まった項目名の表。非空の文言だけを持つ（空・値なし・失敗は既定名に落ちるので
/// この表に入らない＝引けなかった名前はそのまま「既定名を使う」を意味する）。
#[derive(Default)]
pub(crate) struct CaptionMap(HashMap<&'static str, String>);

impl CaptionMap {
    /// リソース名に対応する文言を足す。空文字列は入れない（要件 3.3）。
    pub(crate) fn insert(&mut self, id: &'static str, caption: String) {
        if !caption.is_empty() {
            self.0.insert(id, caption);
        }
    }

    /// リソース名に対応する文言。無ければ `None`（呼び手は既定名を使う）。
    pub(crate) fn get(&self, id: &str) -> Option<&str> {
        self.0.get(id).map(String::as_str)
    }
}

/// メニューを出すかどうか（要件 3.7）。
#[derive(Debug)]
pub(crate) enum Visibility {
    /// 出す。
    Show,
    /// 出さない（`popupmenu.visible` が `0` だった）。
    Suppress,
}

/// 照会の返り値を写した結果。
pub(crate) struct Interpreted {
    /// 引けた項目名（引けなかった名前は入らない＝既定名）。
    pub captions: CaptionMap,
    /// メニューを出すかどうか。
    pub visibility: Visibility,
}

/// 返事そのものが得られなかったときの理由。
#[derive(Debug)]
pub(crate) enum QueryFailure {
    /// 運行へ照会を送れなかった（受け手が既に居ない）。
    SendFailed,
    /// 上限時間（[`QUERY_TIMEOUT`]）までに返事が来なかった。
    Timeout,
    /// 返事を待っている間に返信端が捨てられた（運行が止まった）。
    Dropped,
}

impl QueryFailure {
    /// 記録に載せる理由の綴り。
    fn reason(&self) -> &'static str {
        match self {
            QueryFailure::SendFailed => "send_failed",
            QueryFailure::Timeout => "timeout",
            QueryFailure::Dropped => "dropped",
        }
    }
}

/// 照会の返り値（問い合わせた名前と、その結果の組）。
pub(crate) type QueryReply = Vec<(&'static str, ResourceOutcome)>;

/// 運行（kanade）へ照会を送る。**送るだけで返事は待たない**（要件 3.2）。
///
/// 待つと右クリックの処理がその場で止まり、画面が固まる。返事を覗くのは毎 tick 動く
/// [`super::trigger`] 側で、上限は [`QUERY_TIMEOUT`]。
///
/// 送れなかったときの記録は残さない——呼び手はこの失敗をそのまま [`interpret`] へ渡し、
/// そこで「全件既定名」の警告 1 行にまとまる（要件 3.4 の「1 回」）。
pub(crate) fn send_query(
    kanade: &Sender<KanadeMsg>,
    ids: Vec<&'static str>,
) -> Result<ReplyReceiver<QueryReply>, QueryFailure> {
    let (reply, receiver) = reply_channel::<QueryReply>();
    kanade
        .send(KanadeMsg::ResourceQuery { ids, reply })
        .map_err(|_| QueryFailure::SendFailed)?;
    Ok(receiver)
}

/// 照会の返り値を項目名と表示可否へ写す（要件 3.3・3.4・3.7）。
///
/// 1 件ずつの写し方は ⑴ 非空の値＝その文言をそのまま使う、⑵ 空文字列または値なし＝既定名に
/// 落とし `debug!` に名前を載せる、⑶ 失敗＝既定名に落とし `warn!` に名前と理由を載せる。
/// ⑵⑶ の記録はどちらも **1 回の表示につき 1 行**にまとめる（要件 3.4）。返事そのものが
/// 得られなかったとき（`Err`）は全件が ⑶ と同じ扱いで、警告 1 行に理由を載せる。
///
/// 表示可否（`visible_id`）は値が **ちょうど `0`** のときだけ「出さない」。前後に空白のある
/// ` 0 ` や `00` は別の値なので出す——正典が書いているのは `0` という値であり、綴りを
/// 揺らして解釈すると作者の意図しない抑止が起きるからである。値なし・空・失敗・返事なしも
/// すべて「出す」（要件 3.7）。この名前は項目名ではないので文言の表には入れない。
pub(crate) fn interpret(
    result: Result<QueryReply, QueryFailure>,
    visible_id: &'static str,
    scope: u32,
) -> Interpreted {
    let reply = match result {
        Ok(reply) => reply,
        Err(failure) => {
            tracing::warn!(
                event = "menu_resource_query_unanswered",
                scope = scope,
                reason = failure.reason(),
                "[menu] resource query got no reply: every item falls back to its default label"
            );
            return Interpreted {
                captions: CaptionMap::default(),
                visibility: Visibility::Show,
            };
        }
    };

    let mut captions = CaptionMap::default();
    let mut visibility = Visibility::Show;
    // 記録は表示 1 回につき 1 行ずつなので、名前をここへ貯めてから最後にまとめて出す。
    let mut empty: Vec<&'static str> = Vec::new();
    let mut failed: Vec<(&'static str, String)> = Vec::new();

    for (id, outcome) in reply {
        if id == visible_id {
            if let ResourceOutcome::Value(value) = &outcome
                && value == "0"
            {
                visibility = Visibility::Suppress;
                tracing::info!(
                    event = "menu_suppressed_by_visible",
                    scope = scope,
                    id = id,
                    "[menu] suppressed by popupmenu.visible"
                );
            }
            if let ResourceOutcome::Failed(reason) = outcome {
                failed.push((id, reason));
            }
            continue;
        }
        match outcome {
            ResourceOutcome::Value(value) if !value.is_empty() => captions.insert(id, value),
            ResourceOutcome::Value(_) | ResourceOutcome::NoContent => empty.push(id),
            ResourceOutcome::Failed(reason) => failed.push((id, reason)),
        }
    }

    if !empty.is_empty() {
        tracing::debug!(
            event = "menu_resource_empty",
            scope = scope,
            count = empty.len(),
            ids = ?empty,
            "[menu] resource answered empty or no content: using the default labels"
        );
    }
    if !failed.is_empty() {
        tracing::warn!(
            event = "menu_resource_failed",
            scope = scope,
            count = failed.len(),
            failed = ?failed,
            "[menu] resource query failed: using the default labels"
        );
    }

    Interpreted {
        captions,
        visibility,
    }
}

#[cfg(test)]
#[path = "captions_tests.rs"]
mod captions_tests;
