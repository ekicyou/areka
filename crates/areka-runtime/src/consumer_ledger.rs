//! 消費者台帳（コマンド名→担当消費者の宣言表＋一意性檻・design.md「dola — 名前写像 API の
//! 退役（＋areka 消費者台帳）」・R2.2/R2.6・D10）。
//!
//! dola（配送層）はコマンド名の語彙も名前写像 API も持たず（`command_target_of` 退役・D10）、
//! 汎用コマンド cue を完全に不透明な荷物として broadcast する。各消費者は自らのコマンド名
//! リテラル（`MoveCueSink`＝`"move"`・seriko＝`"bind"`）で cue を**名前自己選別**する
//! （`move_cue.rs` の `MoveCueSink::emit`）。
//!
//! 本台帳はその自己選別モデルの一意性不変条件を、結線層（areka app 層）で宣言・検証する単一
//! 権威表である。dola の権威表ではない（配送層は語彙フリー）。以後のコマンド追加は
//! 「消費者＋本表 1 行」のみで **dola 無改変**（R2.6 の構造的保証）。
//!
//! # 登記の粒度＝「名前＋選別子」（要件 11.2/11.3・task 3.2）
//!
//! 消費の粒度はコマンド名だけとは限らない。`\![move]` は名前だけで担当が決まる（`MoveCueSink`
//! は第 1 引数を見ずに `name != "move"` で選別する）が、`\![set,zorder,…]` は**第 1 引数まで
//! 見て**初めて担当が決まる（`ZOrderCueSink` は `("set","zorder")`／`("reset","zorder")` の組
//! だけを受理し、`\![set,他]` は担当外として読み飛ばす）。
//!
//! そこで登記のキーを **(コマンド名, 選別子)** の組とし、選別子＝**第 1 引数**を
//! `Option<String>` で持つ。2 つの形の意味は次のとおり。
//!
//! - **選別子なし**（`(name, None)`）＝「この名前の出現は、第 1 引数が何であれ全てこの担当」。
//!   `("move", None)`・`("bind", None)` がこれ。
//! - **選別子つき**（`(name, Some(sel))`）＝「この名前の出現のうち、第 1 引数がちょうど `sel`
//!   のものだけがこの担当」。`("set", Some("zorder"))`・`("reset", Some("zorder"))` がこれ。
//!   名簿に無い第 1 引数（例: `\![set,nosuch]`）は**担当なし**であり、将来 別の担当を
//!   足せる余地として空いている（要件 11.3）。
//!
//! # 一意性檻＝「1 コマンド出現に高々 1 担当」（要件 11.3）
//!
//! 不変条件は名前ではなく**実際の消費の粒度**（名前＋第 1 引数）で述べる。台帳はこれを 2 つの
//! 規則で保つ。
//!
//! 1. **同じ組の二重登録を拒む**——[`ConsumerLedger::try_register`] は既登記の組へ再登記すると
//!    [`LedgerError::Duplicate`] を返す（多重結線の検出を観測可能化する）。
//! 2. **同一名で選別子の有無を共存させない**——`("set", None)` と `("set", Some("zorder"))` が
//!    同居すると `\![set,zorder,…]` の 1 出現に 2 つの担当が作用してしまうため、
//!    [`LedgerError::SelectorConflict`] で拒む。順序はどちらでも同じく拒む。
//!
//! 正準台帳 [`ConsumerLedger::canonical`] は表 `CANONICAL_ROWS`（行の正本）を 1 行ずつ try_register し、
//! 違反があれば構築時に panic する（正準表は一意ゆえ実際には発火しない・回帰檻）。
//!
//! # 宣言する表であって、選別する機構ではない
//!
//! 本表は結線時の宣言・検証のためだけにあり、実行時に各消費者がここを引くわけではない
//! （消費者は自らの名前と選別子で自己選別する）。両者は一致していなければならないが、
//! 依存はしない——`ZOrderCueSink` が受理する組と、正準台帳に載る zorder の 2 行
//! （`("set", Some("zorder"))`・`("reset", Some("zorder"))`）が同じであることは、本モジュールの
//! テストがその組を名指しで固定する。`runtime` の 11 の受け口については、表の全行と、表に
//! 無い組の掛け合わせを実際に配って一致を固定する（`consumer_ledger_agreement_tests.rs`・
//! areka-P0-mcp-author-tools 要件 3.11）。

use std::collections::BTreeMap;

/// 台帳のキー＝（コマンド名, 選別子＝第 1 引数）。選別子なしは「名前まるごと」を意味する。
///
/// 決定論のため `BTreeMap` のキーとして使う。`Option` の順序（`None` < `Some(_)`）により、
/// 同一名の登記は「選別子なし → 選別子つき（辞書順）」の順で並ぶ。
type LedgerKey = (String, Option<String>);

/// コマンド名を消費する担当消費者の識別子（軽量・design File Structure「move→MoveCueSink・
/// bind→seriko」）。
///
/// dola はコマンド名語彙を持たないため、本 enum は**結線層が誰にコマンド名を割り当てたか**を
/// 宣言するためだけの識別子である（実際の消費は各消費者が自らの名前で自己選別する）。
///
/// - [`MoveSink`](CommandConsumer::MoveSink): 窓の位置の指令（`\![move]` と
///   `\![execute,resetwindowpos]`）を消費する [`MoveCueSink`](super::move_cue::MoveCueSink)
///   （talk スレッド側名前選別 sink）。正準台帳が `move` と `(execute, resetwindowpos)` の 2 行を
///   登記する（areka-P0-move-canon 要件 8.7）。吸着・窓の組・バルーン位置初期化の 5 行
///   （`(set, alignmenttodesktop|alignmentondesktop|sticky-window)`・`(reset, sticky-window)`・
///   `(execute, resetballoonpos)`）も同じ受け口に登記する（areka-P0-window-link-directives）。
///   バルーンの動かせない指定の 2 行（`(lock, balloonmove)`・`(unlock, balloonmove)`）も同じ
///   受け口に登記する（descript-window-keys 要件 5.7）。
/// - [`Seriko`](CommandConsumer::Seriko): `\![bind]`（着せ替え）を消費する表示系 seriko。
///   正準台帳（[`ConsumerLedger::canonical`]）が `bind` → `Seriko` を登記する（task 7.2）。
/// - [`ZOrderSink`](CommandConsumer::ZOrderSink): 重なり指定・重なり解除を消費する
///   [`ZOrderCueSink`](super::zorder_cue::ZOrderCueSink)。正準台帳が `(set, zorder)`・
///   `(reset, zorder)` の 2 組を登記する（要件 11.2）。
/// - [`ReadmeSink`](CommandConsumer::ReadmeSink): 開く系の受け口（説明書を含む）
///   本体の `readme::readme_cue::ReadmeCueSink`。正準台帳が `(open, readme|file|browser|
///   explorer|editor|mailer)` の 6 組と運搬名 `\j`（選別子なし）を登記する（要件 4.5・
///   areka-P0-open-external-tags 要件 9.2）。
/// - [`UserBreakSink`](CommandConsumer::UserBreakSink): 中断の無効化の区間の出入りを消費する
///   [`NoUserBreakCueSink`](super::user_break_cue::NoUserBreakCueSink)。正準台帳が
///   `(enter, nouserbreakmode)`・`(leave, nouserbreakmode)` の 2 組を登記する
///   （areka-P0-balloon-break 要件 5.1／5.2）。
/// - [`TextLayer`](CommandConsumer::TextLayer): 文字装飾 `\f[...]` を消費する文字レンダリング層
///   （`areka_emo_text` の `state_decoration.rs` が同じ運搬名で自己選別する）。正準台帳が
///   運搬名 [`FONT_TAG_CARRIER`](areka_sakura::contract::FONT_TAG_CARRIER) →
///   `TextLayer` を登記する（areka-P0-text-decoration-canon 要件 2.4）。
/// - [`ChangeSink`](CommandConsumer::ChangeSink): `\![change,ghost,…]` を消費する
///   [`ChangeCueSink`](super::change_cue::ChangeCueSink)。正準台帳が `(change, ghost)` の 1 組を
///   登記する（areka-P0-ghost-shell-balloon-switch 要件 1.10）。
/// - [`SwitchSink`](CommandConsumer::SwitchSink): `\![change,shell,…]`・`\![change,balloon,…]` を
///   消費する [`SwitchCueSink`](super::switch_cue::SwitchCueSink)。正準台帳が `(change, shell)`・
///   `(change, balloon)` の 2 組を登記する（areka-P0-shell-balloon-switch 要件 1.15）。
/// - [`InstallSink`](CommandConsumer::InstallSink): `\![execute,install,…]` を消費する
///   本体の `install::install_cue::InstallCueSink`。正準台帳が `(execute, install)` の
///   1 組だけを登記する（areka-P0-ghost-install 要件 1.5〜1.7）。
/// - [`UpdateSink`](CommandConsumer::UpdateSink): `\![updatebymyself]`・`\![update,…]`・
///   `\![updateother,…]` を消費する 本体の `update::update_cue::UpdateCueSink`。正準台帳が
///   3 つの名前を選別子なしで登記する（areka-P0-network-update 要件 1.5〜1.8）。
/// - [`ScriptCompile`](CommandConsumer::ScriptCompile): `\![set,choicetimeout,…]` を読む台本の
///   組み立て（`areka_sakura::compile`）。正準台帳が `(set, choicetimeout)` の 1 組を登記する
///   （areka-P0-mcp-author-tools 要件 3.2）。
/// - [`PropertyWrite`](CommandConsumer::PropertyWrite): プロパティの書き込みの cue を消費する
///   `areka_ghost` の `PropSetCueSink`。正準台帳が運搬名
///   [`PROP_SET_CUE_NAME`](areka_ghost::prop_sink::PROP_SET_CUE_NAME) を選別子なしで登記する
///   （areka-P0-mcp-author-tools 要件 3.2）。
/// - [`PropertyRead`](CommandConsumer::PropertyRead): プロパティの読みの cue を消費する
///   `areka_ghost` の `PropGetCueSink`。正準台帳が `(get, property)` の 1 組を登記する
///   （shell-scale-linkage 要件 1.8）。
/// - [`ReloadSink`](CommandConsumer::ReloadSink): `\![reload,…]`・`\![reloadsurface]` を消費する
///   [`ReloadCueSink`](super::reload_cue::ReloadCueSink)。正準台帳が `(reload, ghost|shiori|shell|
///   balloon|descript)` の 5 組と `reloadsurface`（選別子なし）を登記する（areka-P0-mcp-reload
///   要件 11.1）。
/// - [`WindowStateSink`](CommandConsumer::WindowStateSink): 常に手前の指令を消費する
///   [`WindowStateCueSink`](super::window_state_cue::WindowStateCueSink)。正準台帳が
///   `(set, windowstate)` と運搬名 [`RAISE_TAG_CARRIER`](areka_parsers::sakura::RAISE_TAG_CARRIER)
///   （選別子なし）の 2 行を登記する（stay-on-top-directives 要件 1.8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandConsumer {
    /// 窓の位置の指令（`\![move]`・`\![execute,resetwindowpos]`・`\![set,alignmenttodesktop,方向]`
    /// （`alignmentondesktop` も同じ）・`\![set,sticky-window,…]`・`\![reset,sticky-window]`・
    /// `\![execute,resetballoonpos]`・`\![lock,balloonmove]`・`\![unlock,balloonmove]`）の担当消費者（[`MoveCueSink`](super::move_cue::MoveCueSink)）。
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2cresetwindowpos_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2calignmenttodesktop_2c_65b9_5411_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2calignmenttodesktop_2cfree_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2calignmentondesktop_2cbottom_307e_305f_306ftop_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2csticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breset_2csticky-window_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2cresetballoonpos_5d:1
    MoveSink,
    /// `\![bind]`・`\![bind-noevent]`（着せ替え）の担当消費者（表示系 seriko）。正準台帳が登記する
    /// （task 7.2・script-small-tags 要件 3.7・6.4）。
    Seriko,
    /// 重なり指定・重なり解除の担当消費者
    /// （[`ZOrderCueSink`](super::zorder_cue::ZOrderCueSink)）。名前だけでは決まらず、
    /// 第 1 引数が `zorder` の出現だけを担当する（要件 11.2）。
    ZOrderSink,
    /// 開く系の受け口（説明書を含む）の担当消費者
    /// （本体の `readme::readme_cue::ReadmeCueSink`）。`open` は名前だけでは決まらず、
    /// 第 1 引数が `readme`・`file`・`browser`・`explorer`・`editor`・`mailer` の出現だけを担当する
    /// （作り付けの窓の `help` などは将来の担当の余地・要件 4.5）。運搬名 `\j` は選別子なしで
    /// 担当する（areka-P0-open-external-tags 要件 1.5・9.2）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bopen_2creadme_5d:1
    ReadmeSink,
    /// 中断の無効化の区間の出入りの担当消費者
    /// （[`NoUserBreakCueSink`](super::user_break_cue::NoUserBreakCueSink)）。名前だけでは
    /// 決まらず、第 1 引数が `nouserbreakmode` の出現だけを担当する（`enter`／`leave` の
    /// 別の第 1 引数——`onlinemode` など——は将来の担当の余地・areka-P0-balloon-break 要件 8.2）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5benter_2cnouserbreakmode_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bleave_2cnouserbreakmode_5d:1
    UserBreakSink,
    /// 文字装飾 `\f[...]` の担当消費者（文字レンダリング層——`areka_emo_text` の
    /// `state_decoration.rs` が運搬名で自己選別して適用する）。キーは第 1 引数
    /// （`bold`／`color`／`height` …）で分かれないので、登記は選別子なしの 1 行である
    /// （areka-P0-text-decoration-canon 要件 2.4）。印の合図 `*`（`\![*]`・`%*`）も選別子なしの
    /// 1 行で担当する（script-small-tags 要件 4.1・6.5）。
    TextLayer,
    /// ゴーストの切替のタグの担当消費者
    /// （[`ChangeCueSink`](super::change_cue::ChangeCueSink)）。名前だけでは決まらず、
    /// 第 1 引数が `ghost` の出現だけを担当する（`shell`／`balloon` は
    /// [`SwitchSink`](Self::SwitchSink) の持ち場・areka-P0-ghost-shell-balloon-switch 要件 1.10）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1
    ChangeSink,
    /// シェル・バルーンの切替のタグの担当消費者
    /// （[`SwitchCueSink`](super::switch_cue::SwitchCueSink)）。名前だけでは決まらず、
    /// 第 1 引数が `shell`／`balloon` の出現だけを担当する（`ghost` は
    /// [`ChangeSink`](Self::ChangeSink) の持ち場・areka-P0-shell-balloon-switch 要件 1.15）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1
    SwitchSink,
    /// 書庫を入れるタグの担当消費者
    /// （本体の `install::install_cue::InstallCueSink`）。名前だけでは決まらず、
    /// 第 1 引数が `install` の出現だけを担当する（`execute` の別の第 1 引数は担当外・
    /// areka-P0-ghost-install 要件 1.5〜1.7）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1
    InstallSink,
    /// ネットワーク更新のタグの担当消費者
    /// （本体の `update::update_cue::UpdateCueSink`）。`updatebymyself`・`update`・
    /// `updateother` の 3 つの名前を選別子なしで担当する（第 1 引数は更新対象・指定であって担当を
    /// 分けない・areka-P0-network-update 要件 1.5〜1.8）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdate_2c_66f4_65b0_5bfe_8c61_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdateother_2c_66f4_65b0_5bfe_8c61_2f_30aa_30d7_30b7_30e7_30f3_7fa4_2c..._5d:1
    UpdateSink,
    /// 選択肢の時間切れの指定 `\![set,choicetimeout,時間]` の担当（台本の組み立て
    /// `areka_sakura::compile` が `set,choicetimeout` だけを読み、選択待ちの区切りへ秒を書く）。
    /// 第 1 引数が `choicetimeout` の出現だけを担当する（`set` の他の第 1 引数は別の担当か担当なし）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1
    ScriptCompile,
    /// プロパティの書き込みの cue の担当（`areka_ghost` の `PropSetCueSink` が運搬名
    /// [`PROP_SET_CUE_NAME`](areka_ghost::prop_sink::PROP_SET_CUE_NAME) で自己選別する）。
    /// 第 1 引数は書き込む key であって担当を分けないので、登記は選別子なしの 1 行である。
    PropertyWrite,
    /// プロパティの読み `\![get,property,イベント名,名前…]` の担当（`areka_ghost` の
    /// `PropGetCueSink` が名前 `get` と第 1 引数 `property` で自己選別する）。runtime の外の担当である
    /// （shell-scale-linkage 要件 1.8）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1
    PropertyRead,
    /// バルーンの寿命の受け口の担当消費者
    /// （[`BalloonLifecycleSink`](super::talk_lifecycle::BalloonLifecycleSink)）。名前だけでは
    /// 決まらず、第 1 引数が `balloontimeout` の `set` の出現だけを担当する（時間切れまでの待ち時間の
    /// 差し替え・areka-P0-balloon-lifecycle-events 要件 8.1・9.10）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1
    LifecycleSink,
    /// 読み直しのタグの担当消費者（[`ReloadCueSink`](super::reload_cue::ReloadCueSink)）。`reload` は
    /// 名前だけでは決まらず、第 1 引数が `ghost`・`shiori`・`shell`・`balloon`・`descript` の出現だけを
    /// 担当する（`makoto` は makoto-reload-directives が後から足す余地・`aigraph` は担当なし・
    /// areka-P0-mcp-reload 要件 11.2）。旧い `reloadsurface` は選別子なしで担当する。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breload_2cghost_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breload_2cshiori_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breload_2cshell_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breload_2cballoon_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breload_2cdescript_2c_30d1_30e9_30e1_30fc_30bf_5d:1
    ReloadSink,
    /// 常に手前の指令の担当消費者（[`WindowStateCueSink`](super::window_state_cue::WindowStateCueSink)）。
    /// `set` は第 1 引数が `windowstate` の出現だけを、第 2 引数（`stayontop`・`!stayontop`・
    /// `minimize` …）に関わらず担当する。運搬名の `\v` は選別子なしで担当する（stay-on-top-directives
    /// 要件 1.8）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cwindowstate_2cstayontop_5d:1
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1
    WindowStateSink,
}

/// 選別子を記録本文へ書くときの見え方（「無い」側も読める形にする——片側だけの本文では
/// 衝突が読めないため・要件 8.3 の規律）。
fn selector_label(selector: &Option<String>) -> String {
    match selector {
        Some(selector) => format!("'{selector}'"),
        None => "なし".to_string(),
    }
}

/// 消費者台帳の構築時失敗（一意性違反の観測可能化・log-first ではなく Result で呼び手へ返す）。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LedgerError {
    /// 同じ（コマンド名, 選別子）の組が二重登録された（多重結線＝要件 11.3 の禁止事項）。
    #[error(
        "コマンド名 '{name}'（選別子: {}）が消費者台帳へ二重登録されました（1 出現=高々 1 担当・要件 11.3）",
        selector_label(.selector)
    )]
    Duplicate {
        /// 二重登録が検出されたコマンド名。
        name: String,
        /// 二重登録が検出された選別子（第 1 引数。名前まるごとの登記なら `None`）。
        selector: Option<String>,
    },
    /// 同一コマンド名について、選別子を伴う登記と伴わない登記を同居させようとした。
    ///
    /// 同居すると、選別子つきの組に当たる 1 出現へ 2 つの担当が作用してしまい
    /// 「1 コマンド出現に高々 1 担当」（要件 11.3）が破れる。順序はどちらでも拒む。
    #[error(
        "コマンド名 '{name}' は選別子 {} で登記済みのため、選別子 {} を追加できません\
         （同一名で選別子の有無は共存できない・1 出現=高々 1 担当・要件 11.3）",
        selector_label(.existing),
        selector_label(.incoming)
    )]
    SelectorConflict {
        /// 衝突が起きたコマンド名。
        name: String,
        /// 既に登記されていた側の選別子。
        existing: Option<String>,
        /// 追加しようとした側の選別子。
        incoming: Option<String>,
    },
}

/// （コマンド名, 選別子）→ 担当消費者の宣言台帳（結線層の単一権威表・要件 11.2/11.3）。
///
/// 登記は [`try_register`](Self::try_register)（重複と選別子の排他違反を拒否）で行い、
/// 照会は [`consumer_of`](Self::consumer_of)（未登記＝`None`＝自己選別モデルの良性未処理）で行う。
#[derive(Debug, Default, Clone)]
pub struct ConsumerLedger {
    /// （コマンド名, 選別子）→担当消費者（決定論のため `BTreeMap`）。
    table: BTreeMap<LedgerKey, CommandConsumer>,
}

impl ConsumerLedger {
    /// 空の台帳を作る（登記は [`try_register`](Self::try_register) で追加する）。
    pub fn new() -> Self {
        Self::default()
    }

    /// （コマンド名, 選別子）の組を担当消費者へ登記する。
    ///
    /// `selector` は消費の粒度を決める**第 1 引数**である。`None` は「この名前の出現は第 1 引数に
    /// よらず全てこの担当」を、`Some(sel)` は「第 1 引数がちょうど `sel` の出現だけがこの担当」を
    /// 宣言する（モジュール doc「登記の粒度」）。
    ///
    /// 次の 2 つを拒む（いずれも観測可能な `Err`。黙って上書きも黙って無視もしない）。
    ///
    /// - 同じ組の再登記 → [`LedgerError::Duplicate`]
    /// - 同一名で選別子の有無が食い違う登記 → [`LedgerError::SelectorConflict`]
    ///
    /// 拒否したときは表を一切変更しない（既存の担当は据え置き）。
    pub fn try_register(
        &mut self,
        name: &str,
        selector: Option<&str>,
        consumer: CommandConsumer,
    ) -> Result<(), LedgerError> {
        let key: LedgerKey = (name.to_string(), selector.map(str::to_string));
        if self.table.contains_key(&key) {
            return Err(LedgerError::Duplicate {
                name: key.0,
                selector: key.1,
            });
        }
        // 同一名の既存登記だけを走る（`(name, None)` は同名キーの最小値なので、そこから
        // 名前が変わるまでが同一名の範囲）。選別子の有無が食い違うものが 1 つでもあれば排他違反。
        let same_name_from: LedgerKey = (name.to_string(), None);
        let conflict = self
            .table
            .range(same_name_from..)
            .take_while(|((existing_name, _), _)| existing_name.as_str() == name)
            .find(|((_, existing_selector), _)| existing_selector.is_none() != key.1.is_none());
        if let Some(((_, existing_selector), _)) = conflict {
            return Err(LedgerError::SelectorConflict {
                name: key.0.clone(),
                existing: existing_selector.clone(),
                incoming: key.1,
            });
        }
        self.table.insert(key, consumer);
        Ok(())
    }

    /// 1 つのコマンド出現（コマンド名＋第 1 引数）の担当消費者を引く。
    ///
    /// 引き方はモジュール doc「登記の粒度」の 2 つの形をそのまま写したもの。
    ///
    /// 1. 名前まるごとの登記 `(name, None)` があれば、`selector` が何であれそれが担当である
    ///    （`\![move]` も `\![move,--x=10]` も同じ `MoveSink`）。
    /// 2. 無ければ、`selector` とちょうど同じ選別子つきの登記を引く。名簿に無い第 1 引数と、
    ///    第 1 引数の無い出現はどちらも `None`＝担当なしになる。
    ///
    /// 相互排他（[`LedgerError::SelectorConflict`]）により ⑴ と ⑵ は同時に成り立たないので、
    /// 1 出現が引く担当は高々 1 つである（要件 11.3）。未登記名は `None`——自己選別モデルでは
    /// 担当のいないコマンドは良性の未処理であって、異常ではない（要件 11.2）。
    pub fn consumer_of(&self, name: &str, selector: Option<&str>) -> Option<CommandConsumer> {
        if let Some(consumer) = self.table.get(&(name.to_string(), None)) {
            return Some(*consumer);
        }
        let selector = selector?;
        self.table
            .get(&(name.to_string(), Some(selector.to_string())))
            .copied()
    }

    /// 正準台帳を構築する。行の一覧は [`CANONICAL_ROWS`] が正本で、ここは 1 行ずつ
    /// [`try_register`](Self::try_register) するだけ（一覧を説明に写さない）。
    ///
    /// zorder の 2 行は `ZOrderCueSink` が自己選別する組とちょうど同じである（表は宣言し、受け口は
    /// 自ら選別する——実行時に受け口が本表を引くわけではない）。中断の無効化の 2 行と
    /// `NoUserBreakCueSink` の関係も同じである。
    ///
    /// 以後のコマンド追加は「消費者＋本表 1 行」のみで **dola 無改変**（R2.6）。正準表は一意ゆえ
    /// 登記は Ok を返す——万一将来の編集で重複や選別子の排他違反を作れば、名前と選別子を添えて
    /// 構築時に panic して回帰を止める（一意性の内部整合檻）。
    pub fn canonical() -> Self {
        let mut ledger = Self::new();
        for &(name, selector, consumer) in CANONICAL_ROWS {
            if let Err(e) = ledger.try_register(name, selector, consumer) {
                panic!(
                    "正準台帳: ({name:?}, {selector:?}) の登記に失敗（重複・排他違反は編集ミス）: {e:?}"
                );
            }
        }
        ledger
    }
}

/// 正準台帳の行（名前, 選別子, 担当）。表が正本——行の増減はここだけを直す（見本の表
/// `consumer_ledger_test_support.rs` の `samples` は、組の一致を `samples_cover_exactly_the_ledger_rows`
/// が判定する）。
///
/// `(reload, makoto)` は makoto-reload-directives が後から足す（要件 11.2）。
const CANONICAL_ROWS: &[(&str, Option<&str>, CommandConsumer)] = {
    use CommandConsumer::*;
    &[
        ("move", None, MoveSink),
        (areka_seriko::BIND_COMMAND, None, Seriko),
        ("set", Some("zorder"), ZOrderSink),
        ("reset", Some("zorder"), ZOrderSink),
        ("open", Some("readme"), ReadmeSink),
        ("open", Some("file"), ReadmeSink),
        ("open", Some("browser"), ReadmeSink),
        ("open", Some("explorer"), ReadmeSink),
        ("open", Some("editor"), ReadmeSink),
        ("open", Some("mailer"), ReadmeSink),
        (areka_parsers::sakura::JUMP_TAG_CARRIER, None, ReadmeSink),
        ("enter", Some("nouserbreakmode"), UserBreakSink),
        ("leave", Some("nouserbreakmode"), UserBreakSink),
        (areka_sakura::contract::FONT_TAG_CARRIER, None, TextLayer),
        ("change", Some("ghost"), ChangeSink),
        ("change", Some("shell"), SwitchSink),
        ("change", Some("balloon"), SwitchSink),
        ("execute", Some("install"), InstallSink),
        ("execute", Some("resetwindowpos"), MoveSink),
        ("updatebymyself", None, UpdateSink),
        ("update", None, UpdateSink),
        ("updateother", None, UpdateSink),
        ("set", Some("choicetimeout"), ScriptCompile),
        (
            areka_ghost::prop_sink::PROP_SET_CUE_NAME,
            None,
            PropertyWrite,
        ),
        ("set", Some("balloontimeout"), LifecycleSink),
        ("reload", Some("ghost"), ReloadSink),
        ("reload", Some("shiori"), ReloadSink),
        ("reload", Some("shell"), ReloadSink),
        ("reload", Some("balloon"), ReloadSink),
        ("reload", Some("descript"), ReloadSink),
        ("reloadsurface", None, ReloadSink),
        // 吸着・窓の組・バルーン位置初期化（areka-P0-window-link-directives 要件 1・3・7）。
        ("set", Some("alignmenttodesktop"), MoveSink),
        ("set", Some("alignmentondesktop"), MoveSink),
        ("set", Some("sticky-window"), MoveSink),
        ("reset", Some("sticky-window"), MoveSink),
        ("execute", Some("resetballoonpos"), MoveSink),
        // バルーンの動かせない指定（descript-window-keys 要件 5.7）。
        ("lock", Some("balloonmove"), MoveSink),
        ("unlock", Some("balloonmove"), MoveSink),
        // 常に手前の指令（stay-on-top-directives 要件 1.8）。第 2 引数の読み分けは受け口が持つ。
        ("set", Some("windowstate"), WindowStateSink),
        (
            areka_parsers::sakura::RAISE_TAG_CARRIER,
            None,
            WindowStateSink,
        ),
        (areka_seriko::BIND_NOEVENT_COMMAND, None, Seriko),
        (areka_parsers::sakura::MARKER_COMMAND, None, TextLayer),
        // プロパティの読み（shell-scale-linkage 要件 1.8）。受け口は runtime の外（areka_ghost）。
        ("get", Some("property"), PropertyRead),
    ]
};

#[cfg(test)]
#[path = "consumer_ledger_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "consumer_ledger_agreement_tests.rs"]
mod agreement_tests;

// areka-runtime のテストと本体のテストの両方が使う補助（spec: runtime-crate-split 設計「実装の中の討議の結果」1）。
#[cfg(any(test, feature = "test-support"))]
#[path = "consumer_ledger_test_support.rs"]
pub mod test_support;
