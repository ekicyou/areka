# ギャップ分析: areka-P0-mcp-kanade-tools

> 実測は **2026-10-18・本ブランチ**（`claude/areka-p0-mcp-kanade-tools-759974`・main `2d86dbff` の上に spec の初期化 1 コミット）。読んだのは確定済みの requirements.md・brief.md（「2026-10-18 棚卸㉗の再測定」を含む）・steering（product・tech・structure・logging・roadmap の「ウェーブ編成」C9 の行）とコード。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指す。
> 本書は情報と選択肢を並べるもので、決定はしない（決めるのは設計の段と設計の討議）。SSP には何も送っていない・実機も動かしていない（読むだけの分析）。

---

## 1. 分析の要約

- **MCP の側は薄い。** `crates/areka/src/mcp/{sakurascript,raise_event}.rs` の `handle` を、手本 `get_status.rs` の `handle`（置き場 `GhostSlot` から kanade の送り口を引く → 返信端つきの知らせを送る → `super::later` で毎フレーム覗く → 返信端が落ちたら宛先の解決の失敗と同じ文言）の形で書き換えれば足りる。偽の kanade を置くテストの足場（`GhostSession::for_test`）も在る。`crates/areka-mcp/**` と `crates/areka/src/mcp/mod.rs` は触らずに済む。
- **重いのは kanade の側で、足りないものは 5 つ。** ⑴ 外から台本を流す入力が無い（`start_talk` は在る）⑵ 任意の名前のイベントを運ぶ型が無い（`EventId` は `Static`・`Choice` の 2 種・送出の檻 `round_trip_raw` も 2 種だけを通す）⑶ 返った台本の文字列を返す結果の型が無い（`RaiseOutcome` は `Copy` の 5 値で、しかも触れない `src/change.rs` に在る）⑷ 起動・終了・切替の途中で断る判定が無い（今の `on_raise_event` は `Steady` かどうかだけを見るので、終了や切替が保留されて今のトークの終わりを待つ間も送ってしまう）⑸ 翻訳の Reference1 が欠番に固定（`events::on_translate`）。
- **script 種別のログは「どこで出すか」が分かれ目。** 棚卸㉗の brief は殻（`actor.rs` の `execute_batch` の `Action::StartTalk` の腕）を推すが、そこでは元のイベント名が `&'static str` の `origin` に潰れ（任意の名前は持てない）、翻訳を通った台本では最後の往復が `OnTranslate` になる。運行表の出口（`step` の最後・`balloon_events::settle` と同じ所）なら、元のイベント（`State::reply_source`・`TranslateWait::source`）と最終の台本の両方が手元にあり、テストも呼んだスレッドで捕まえられる（記録の捕捉は呼んだスレッドだけ＝`schedule/log_capture.rs` の説明）。ゴーストの名前は `KanadeConfig` に足し、`crates/areka-ghost/src/config.rs` の `resolve_kanade_config` の中で `mount.names.name` から入れられる＝**`runtime.rs` に触らずに済む**。
- **ウェーブの約束（C9）に当たる所は 3 つで、どれも回避の形がある。** `strict` を再生の側（`StartTalk`）まで運ぶと `schedule/boot.rs` の構造体の字面 2 か所が壊れる（要件 5.1・5.3）。既存の `KanadeMsg::RaiseEvent` の欄を足すと `update/desk.rs`（④ の持ち物）が壊れる（要件 2.1）。名前を `runtime.rs` で入れると ④ と重なる（要件 4.1）。それぞれ「新しい変種・新しいファイル・`config.rs` の中で入れる」で避けられる（下の 4 節）。
- **規模**: M〜L（14〜19 タスク・brief の見立てのまま）。リスクは中（新しい仕組みは無いが、運行表の 1 か所の出口に全部のトークの記録を足す・送出の檻に第 3 の出どころを足すので、既存の決定論テストの網が広く掛かる）。

---

## 2. 今あるもの（調べた資産）

### 2.1 MCP の側（`crates/areka/src/mcp/`・`crates/areka-mcp/`）

| 資産 | 場所 | 使い道 |
|---|---|---|
| ダミーの `handle` 2 つ | `sakurascript.rs`・`raise_event.rs`（どちらも `outcome::ng("not implemented yet")`） | 書き換える（要件 1.6・2.9） |
| ダミーを固定するテスト | `sakurascript_tests.rs`・`raise_event_tests.rs` | 書き換える（要件 7.4） |
| 引数の型 | `crates/areka-mcp/src/tools/sakurascript.rs` の `Args`（`script`・`ghost_name`・`strict: Option<bool>`）／`raise_event.rs` の `Args`（`event`・`references: Vec<String>`・`ghost_name`・`strict`） | そのまま使う。空文字は検査を通って `handle` まで届く（`required_string` は空を `""` で渡す）＝空の判定は `handle` の仕事 |
| 結果を作る関数 | `crates/areka-mcp/src/tools/outcome.rs` の `ok("")`＝本文 `OK`／`ok(note)`＝`OK:<note>`／`ng(reason)`／`value(text)` | `OK`・`OK:the ghost returned no script.` はそのまま作れる。2 行の答え（`OK:…` と `Script: …`）は `value(format!(…))` で 1 本の本文に組む（区切りの字は要件 2.3 の「設計の段に確かめる」） |
| 後から答える口 | `crates/areka/src/mcp/mod.rs` の `later`・`poll_later`（`McpLater`） | そのまま使う（mod.rs は触らない） |
| 手本 | `get_status.rs` の `handle`（`resolve::resolve(None, …)` で「降りた」の文言を作る・`ReplyError::Dropped` を「降りた」に倒す・送り口が無いときは `warn!` と独自の `NG:`） | 同じ形にそろえる |
| 宛先の解決 | `mod.rs` の `dispatch`（`resolved!` マクロ）・`resolve.rs` の `resolve`・`listed_value` | 切替の途中で置き場が空なら解決の段で `NG:` になる＝要件 3.5 の前半は既に満たす |
| テストの足場 | `get_status_tests.rs` の `rig`（`GhostSession::for_test(Some(tx), root)` で偽の kanade の送り口を置く）・`log_capture_kit::capture`・`count_levels` | 同じ形で書ける（要件 7.2） |
| 案内文 | `crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS`（「未実装は `NG:not implemented yet`」の 1 文） | 触らない（`reload` がまだ未実装・消すのは `mcp-strict-errors`） |

### 2.2 kanade（`crates/areka-kanade/src/`）

| 資産 | 場所 | 状態 |
|---|---|---|
| inbox の型 | `msg.rs` の `KanadeMsg`（930 行・うち 510〜927 行目がファイルの中のテストの塊） | 変種を足す。上限 1,000 まで残り 70 行＝先にテストを兄弟へ出す（brief どおり） |
| 汎用の通知の入口 | `KanadeMsg::RaiseEvent { id, references, method, reply: Option<ReplySender<RaiseOutcome>> }` | 許可の表に無い名前は `raise_event_not_allowed`（warn）で捨てる・`Steady` 以外は `raise_event_not_steady`（warn）で捨てる。呼び手は areka の 7 ファイル（下の 4.2） |
| 結果の型 | `src/change.rs` の `RaiseOutcome`（`NotAllowed`・`NotSteady`・`Script`・`NoReply`・`Failed`・`Copy`） | 台本の文字列を持たない。**`src/change.rs` は ④ の持ち物で触れない** |
| 殻の振り分け | `actor.rs` の `spawn_kanade_translating` の `match msg`（`StatusQuery`・`ResourceQuery` は状態機械を経ずその場で答える） | 901 行。新しい腕と返事の組み立ては新しいファイルへ出す前提 |
| 返事の材料 | `actor.rs` の `drive_translating` の 2 つ目の返り値（最初の一括の往復の結果を `raise_outcome_of` で写したもの） | 文字列は捨てている。`ShioriOutcome` は複製できない（`String` は複製できる） |
| 後で取り出す結果の手本 | `schedule::talk_gap::take_outcome(&mut state)`（遷移の後に殻が取り出して返信端へ送る） | 「受け付けた／断った」を運行表から殻へ返す形として使い回せる |
| 送出の檻 | `actor.rs` の `round_trip_raw`（`EventId::Static`＝許可の表 ∨ リソースの表・`EventId::Choice`＝`On` 始まり・それ以外は `event_id_not_allowed`（error）で送らず `Failed(Internal)`） | 第 3 の出どころを足すなら受理の規則を 1 つ足す。今のままだと任意の名前は `Failed` → 運行表が `Unloading{Fault}` へ倒す＝**ゴーストが止まる**（要件 2.1 に反する） |
| エラー応答の写し | `actor.rs` の `round_trip_request`（SHIORI の 400／500 を GET は `NoContent` に写し `shiori_error_response`（**warn**）を 1 件） | 要件 2.4（エラー応答は `OK:the ghost returned no script.`）と要件 6.5（成功の答えで warn 以上を出さない）が食い違う（下の 6 節の議題） |
| 欠番の印 | `shiori3::ABSENT_REFERENCE`（`"\u{0}"`）・`actor.rs` の `replace_absent_markers`（`OnTranslate` の添字 1 だけ許す） | 要件 4.4 の「Reference2・3 は欠番」は、末尾の欠番なら Reference の列を 2 つで止めれば足り、印は要らない |
| 運行表の入力 | `schedule/mod.rs` の `Input`（969 行）・`route` | 変種を足す。上限まで残り 31 行＝先に型の塊を兄弟へ出し、`mod.rs` から同じ名前で見せる（`schedule/boot.rs` の `use super::{Action, ActiveTalk, Input, Phase, State, events, resources}` が変わらない） |
| 台本 1 つを新しいトークとして始める | `schedule/steady_choice_script.rs` の `start_talk(state, origin, script)`（`pub(in crate::schedule)`・採番と `Phase::Steady{Some}` への差し替え） | 使い回せる。選択肢の帳簿の掃除（`clear_choice_ledger`・`choice_prev_talk = None`）は呼び手の仕事 |
| 再生中の置き換え | `schedule/steady.rs` の `on_reply` の `Phase::Steady{talk: Some}`＋`Value` の腕（`events::value_replaces_active_talk(origin)` が真なら採番・帳簿の掃除・枠の差し替え。旧トークの後始末は dispatcher の既存の「閉じてから起こす」） | `raise_event` の返った台本は**この腕にそのまま乗る**（`origin` が `OnSecondChange` でなければ置き換え）。選択肢の帳簿は黙って消え、選択や時間切れのイベントは送られない＝要件 3.3 と同じ振る舞いが既に在る。中断の無効化モードも見ていない＝要件 3.4 も既に満たす |
| 翻訳の運行表の側 | `schedule/translate.rs` の `capture`（入力が SHIORI の応答で台詞のときだけ一括を預ける）・`on_done`・`TranslateWait{ talk_id, source: EventId, deferred }`・`TranslateRequest{ script, source: SourceEvent, status }` | `sakurascript` の台本は SHIORI の応答ではないので `capture` に乗らない＝外から台本を預ける口が要る。`source` は `EventId` なので「SSTP の出どころ」を表せない |
| `OnTranslate` の組み立て | `schedule/events.rs` の `on_translate`（Ref0＝展開済み・**Ref1＝`ABSENT_REFERENCE` 固定**・Ref2＝元のイベントの ID・Ref3＝元の Reference を `\u{1}` で連結） | Ref1 を出どころで変える口が要る（要件 4.4）。`raise_event` の台本は今の形で要件 4.5 を満たす |
| MAKOTO の口 | `translate.rs` の `MakotoChain`（台詞, 元のイベントの ID） | `sakurascript` の台本のとき 2 つ目に何を渡すかを決める（本番は素通し＝`runtime.rs` の `TranslateSeams::passthrough().makoto`） |
| 出口の控え | `schedule/mod.rs` の `step` の最後の `balloon_events::settle(&mut state, &actions)`（返す指示の列の `Action::StartTalk` を最終の台本で控える） | script 種別のログを出す候補の所（4.3 節） |
| 運行の構成 | `msg.rs` の `KanadeConfig`（ゴーストの名前の欄は無い・構築は全部 `KanadeConfig::new` か `..KanadeConfig::new(…)`） | 欄を 1 つ足しても既存の構築点は壊れない |

### 2.3 その他

- **ゴーストの名前の源**: `crates/areka-ghost/src/config.rs` の `resolve_kanade_config(mount: &MountModel, …)` は `MountModel` を受け取る。`MountModel.names.name`（descript `name`）は、MCP の宛先の名前（`resolve::active` が `session.names()`＝`runtime.mount().names` から引く）と同じ源。名前が無いときの代わり（ゴーストのフォルダの絶対パス）は、`mount.shiori.dir`（＝ghost_root/ghost/master）の 2 つ上から作れるが、MCP 側の `session.ghost_dir()` と字が一致するかは設計で確かめる。
- **script 種別のログの受け手**: `crates/areka/src/log_history.rs`（`TARGET_SCRIPT = "areka::log::script"`・プロセス全体の受け手 `init()`・欄 `ghost`・`label`）と `doc/ssp-mcp/log-convention.md`。今この target へ出す本番の行は 0。履歴の層は info 以上を取るので、標準出力の層（`RUST_LOG` 既定 info）にも全部のトークの台本が出る。
- **SSP との差の一覧の書式**: `doc/ssp-mcp/get-status-diff-areka.md`（「項目｜areka｜SSP｜SSP の印」の表＋補足）。新しいファイル（例 `kanade-tools-diff-areka.md`）で足りる。`doc/ssp-mcp/areka-tools.md` は ④ が直すので触らない方がよい。
- **検体の README**: `crates/areka-seriko/tests/fixtures/trigger-intervals/README.md` の 64 行目（「`sakurascript` はまだ中身が無いので…読み替え」）＝要件 7.7 で戻す 1 文。

---

## 3. 要件と資産の対応（ギャップの印: 欠＝無いもの・不明＝調べが要る・制約＝既存の作りの縛り）

| 要件 | 使える資産 | ギャップ |
|---|---|---|
| 1.1 普段の状態で台本を新しいトークとして再生・`OK` | `start_talk`・`get_status.rs` の形・`outcome::ok("")` | **欠**: 外からの台本の `KanadeMsg` の変種・`Input` の変種・受け付けの判定・殻の返事 |
| 1.2 ゴースト自身の台本と同じ扱い | 再生の側は台本の出どころを見ていない（`StartTalk` に出どころの欄が無い） | なし（同じ `Action::StartTalk` で流せば同じに働く）。`\![reload,…]` の中身は `mcp-reload` |
| 1.3 受け付けた時点で答える | 殻は一括の処理の後に返信端へ送る（`RaiseEvent` と同じ） | **制約**: 翻訳の往復（`OnTranslate`）を同じ処理の中で済ませてから答えることになる（再生の終わりは待たない） |
| 1.4 空の `script` は `NG:script is empty` | `outcome::ng` | **欠**: `handle` の先頭の判定（kanade へ送らない） |
| 1.5 長さで断らない | UI スレッドは文字列を kanade へ渡すだけ | なし。展開と翻訳は kanade のスレッド |
| 1.6・2.9 ダミーを残さない | — | 書き換えだけ |
| 2.1 任意の名前を 1 回送る・止めない・警告しない | `events::raise`・`round_trip_raw` | **欠**: 任意の名前を運ぶ `EventId` の第 3 の変種（または同等）と送出の檻の受理の規則。**制約**: 既存の `RaiseEvent` の欄を足すと areka の 7 ファイルが壊れる（4.2） |
| 2.2 許可の表に在る名前でも Reference を補わない | `events::raise` は補わない（補うのは運行表の各イベントの組み立て関数） | なし（MCP の名前は表に在っても任意の名前の道で送れば補わない） |
| 2.3 返った台本を再生し、台本の文字列を答えに含める | `steady::on_reply` が再生・置き換えを既にする | **欠**: 最初の往復の台本の文字列を殻から返す型（`src/change.rs` の外）。**不明**: 2 行の区切りの字（SSP の生の応答で確かめる） |
| 2.4 返らないとき `OK:the ghost returned no script.`・今の再生に触れない | `raise_outcome_of` と同じ写し（空・空白だけ・204・エラー応答＝返事なし）・`steady` の `NoContent` の腕は何もしない | **制約**: エラー応答で `shiori_error_response`（warn）が出る＝要件 6.5 と食い違う（議題） |
| 2.5 SHIORI の答えが返った時点で答える | 殻の返事の送り方 | 翻訳の往復も同じ処理の中なので、厳密には「翻訳が終わった時点」になる（議題に足すほどではないが設計で明記） |
| 2.6 空の `event` は `NG:event is empty` | — | **欠**: `handle` の判定（空白だけを含む） |
| 2.7 制御文字は `NG:event or reference contains a control character` | — | **欠**: `handle` の判定。**不明**: 「制御文字」の範囲（`char::is_control` なら TAB も含む・欠番の印 `\u{0}` と翻訳の区切り `\u{1}` も弾けるので都合がよい） |
| 2.8 往復の失敗は 3.5 の文言 | 往復の失敗は今日どおり `to_unloading_fault` | **欠**: 殻の結果の「失敗」を MCP の側で「降りた」の文言に写す |
| 3.1 再生中の `sakurascript` は中断して再生 | `start_talk` が枠を差し替え、dispatcher が旧トークを閉じる | **欠**: 外からの台本の受け付けで帳簿を掃除する所（`clear_choice_ledger`・`choice_prev_talk`） |
| 3.2 再生中の `raise_event` | `steady::on_reply` の置き換えの腕 | **制約**: `origin` が `&'static str`。任意の名前の応答には固定のラベルを載せる（`EventId::Choice` が `"OnChoiceEvent"` を載せるのと同じ形）。名前が `OnSecondChange` そのものの頼みだけは置き換えの腕でなく破棄の腕（warn）に落ちる＝固定のラベルにすれば避けられる |
| 3.3 選択肢の最中は取り下げて再生・イベントを送らない | `steady` の置き換えの腕が帳簿を黙って消す | 外からの台本の側にも同じ掃除が要る（上の 3.1） |
| 3.4 中断の無効化モードでも割り込む | 置き換えは `no_user_break` を見ない | なし |
| 3.5 降りた・置き場が空 | 解決の段（置き場が空）・`get_status.rs` の `ReplyError::Dropped` の腕 | 同じ形で書く |
| 3.6 起動・終了・切替の手順の途中は断る | `Phase`（`Boot*`・`Close*`・`Change*`・`Unloading`）と `State::pending_close`・`State::pending_change` | **欠**: 判定の関数（`Steady` かつ保留の終了・保留の切替が無い、だけを受ける）と断りの結果。今の `on_raise_event` はこの判定を持たない。**不明**: シェル・バルーンの切替の印の台詞（`talk_gap` の見張り）の最中を「手順の途中」に数えるか（下の議題） |
| 3.7 SHIORI との往復の最中は後で処理 | inbox は往復の間ほかの知らせを処理しない | なし |
| 3.8 上限で先に答えた後の台本は普通に再生 | kanade は MCP の待ちを知らない | なし（返信端が落ちていても `debug!` で続ける形にそろえる） |
| 3.9 3.5・3.6 で warn を出さない | — | 新しい腕を `debug!` までにする（既存の `raise_event_not_steady` の warn の道を通さない） |
| 4.1 全部のトークを 1 回ずつ script のログへ | `settle` の出口・`log_history` | **欠**: ログの行・ゴーストの名前（`KanadeConfig`）・表示の語の材料。**制約**: 殻で出すとイベント名が無い（4.3 節） |
| 4.2 表示の語 `SSTP(Local,Auth)`・`Ghost:<イベント名>`・`Ghost` | `State::reply_source`（`translate::before` が応答の入力で取り出す）・`TranslateWait::source` | **欠**: 翻訳を通ったトークで元のイベント名を出口まで持ち越す欄（`on_done` が `TranslateWait` を取り出して消す） |
| 4.3 再生が始まらなければ残さない | 翻訳の輸送路の失敗は `on_done` が預けた一括を捨てる＝出口に `StartTalk` が出ない | 出口で出せば自然に満たす |
| 4.4 `sakurascript` の翻訳は Ref1＝`sstp-send,owned`・Ref2・3 欠番 | `on_translate` | **欠**: 出どころで Ref1 を変える口・SSTP の出どころを表す型（`SourceEvent` は `EventId` 必須）。`doc/COMPAT_ARCHITECTURE.md` の 331 行目（`OnTranslate` の Reference1 の行）を改める |
| 4.5 `raise_event` の翻訳はゴースト自身の台詞と同じ | `capture` が応答の元のイベント（任意の名前を含む）で預ける | 第 3 の `EventId` が `as_str` で名前を返せば今のまま満たす |
| 4.6 MCP 以外の翻訳を変えない | — | Ref1 の口を足すとき既定を欠番のままにする |
| 5.1〜5.3 `strict` を下流へ渡す | — | **欠**＋**制約**: 「再生する側」をどこまでと読むかで触る所が大きく変わる（4.1 節・議題） |
| 6.1 MCP 以外は許可の表のまま | `on_raise_event`・`round_trip_raw` | 新しい道を別の変種にすれば既存の道は 1 行も変わらない |
| 6.2 既存の `RaiseEvent` の呼び手を変えない | — | 同上（欄を足さない） |
| 6.3 採番と枠の規則を同じに | `start_talk`・`steady` の置き換えの腕 | 同じ関数を使えば満たす |
| 6.4 UI を塞がない | `later` | なし |
| 6.5 成功の答えで warn を出さない | — | **制約**: エラー応答の warn（2.4 の行） |
| 7.1 kanade の決定論テスト ⑴〜⑽ | `actor_raise_reply_tests.rs` の `spawn_rig`（偽の shiori）・`schedule/log_capture.rs`・`translate_path_tests.rs` | ⑽ のログは運行表の出口で出すなら `step` を呼んだスレッドで捕まえられる。殻で出すと殻のスレッドになり今の捕捉では拾えない（`actor-thread-log-capture` は未着手） |
| 7.2・7.3 MCP の側の決定論テスト | `get_status_tests.rs` の形・`log_history_convention_tests.rs` の「本物のマクロを `draft` に通す」形 | 7.3 は kanade のスレッドの行を MCP の側のテストで拾えない。**不明**: 行を組む関数を kanade から公開して、MCP の側のテストがその関数の出す行を `capture`→`draft` で読む形にするか |
| 7.4 ダミーのテストの書き換え | — | 2 ファイル |
| 7.5 差の一覧 | `get-status-diff-areka.md` の書式 | 新しいファイル |
| 7.6 実機の確認 | — | ⑥ `real-run-mcp-kit` と実走の 3 本のテストに触れないので、Claude Code から手で MCP を使う |
| 7.7 検体の README を戻す | 64 行目の 1 文 | 1 か所 |

---

## 4. 同じウェーブ（C9）の約束との照合

本 spec が触ってはならないもの: kanade の `src/change.rs`・`schedule/boot.rs`、`crates/areka-ghost/src/runtime.rs`（いずれも ④ `mcp-reload`）、`crates/areka-mcp/**`（⑧）、`crates/areka/src/main.rs`（②）、実走の 3 本のテスト（⑥）。

| # | 要件 | 素直に作ると触るファイル | 当たる約束 | 回避の形 |
|---|---|---|---|---|
| W1 | 5.1・5.3（`strict` を再生する側へ渡す） | `StartTalk`（`crates/areka-talk/src/lib.rs`）に欄を足すと、構造体の字面で組む `schedule/boot.rs` の 2 か所（起動の挨拶・`epilogue` つき）・`schedule/close.rs` の 1 か所・`crates/areka-ghost/src/prop_sink.rs`（テスト）・`crates/areka-sakura/src/drive.rs`（分解）が壊れる | **`schedule/boot.rs`（④）** | ⒜ `StartTalk` に触らず、kanade の中（`ActiveTalk` か新しい控え）に「このトークは strict か」をトークの番号つきで持つ＝再生する側＝kanade が「再生を始める指示」と結ぶ形と読む ⒝ `TalkCommand` に変種を足す（`crates/areka-ghost/src/dispatcher.rs` と殻の写しを直す・`boot.rs` には触らない）⒞ ④ と申し合わせて `boot.rs` の 2 か所だけを片方が直す。⒜ がいちばん小さく、⒝ は `mcp-strict-errors` の案 ⑵（印を再生の各所まで運ぶ）の入口になる。どちらにするかは `mcp-strict-errors` の議題（入口で診断するか印を運ぶか）と `script-security-level`（出どころの印の入れ物）にも掛かる |
| W2 | 2.1・6.2（任意の名前のイベント） | 既存の `KanadeMsg::RaiseEvent` に「許可の表を通さない」欄を足すと、構造体の変種の字面で組む areka の 7 ファイル（`emo2_boot/frame/switch.rs`・`input_events/file_drop.rs`・`install/desk.rs`・`update/desk.rs`・`update/worker.rs`・`menu/reset_pos_frame.rs`・`mcp/dump_surface_gpu_test_support.rs`）とそのテストが壊れる | **`crates/areka/src/update/desk.rs`（④）** | 新しい変種（例「外からのイベント」）を足す。既存の 7 ファイルは 1 行も変わらない（要件 6.2 も同時に満たす） |
| W3 | 2.3（台本つきの結果） | `RaiseOutcome` に変種を足す | **`src/change.rs`（④）** | 結果の型を新しいファイル（例 `src/external.rs`）に置く（brief どおり） |
| W4 | 4.1（ログのゴーストの名前） | 棚卸㉗の brief は `config.rs`・`runtime.rs` の 2 か所と書く | **`runtime.rs`（④）** | `resolve_kanade_config` が `MountModel` を受け取っているので、`config.rs` の中だけで `mount.names.name` を入れられる＝`runtime.rs` は不要。名前が無いときの代わり（フォルダの絶対パス）の字が MCP の宛先の値と一致するかだけ確かめる。どうしても `runtime.rs` が要る形になったら ④ と申し合わせて片方だけが触る |
| W5 | 3.6（手順の途中で断る） | 起動の相の判定を `schedule/boot.rs` に置く | `schedule/boot.rs`（④） | 判定は新しい `schedule/` の子で `Phase` を読むだけにする（`boot.rs` に腕を足さない）。`schedule/mod.rs` を割るときに型を同じ名前で見せる（`boot.rs` の `use super::{…}` が変わらない） |
| W6 | 7.6（実機の確認） | 実走のテストに場面を足す | 実走の 3 本（⑥） | 手で MCP を使う（要件のとおり） |
| — | `crates/areka-mcp/**`・`main.rs` | 触る必要なし | — | 0 |

ほかの重なり（約束の外・参考）:
- `schedule/close.rs`: ④ の brief は「降ろす側の扱い（`schedule/close.rs` か `msg.rs`）」を候補に挙げる。本 spec は W1 の ⒝・⒞ を採らなければ `close.rs` に触らない。
- `crates/areka-ghost/src/shiori_wiring.rs`: ② が `dontloadmodule` で触るか迷っている所。**本 spec は触らない**（② へ伝えてよい）。
- `crates/areka/src/mcp/mod.rs`: 約束の表には無いが brief は「触らない」。新しいファイルを足すと `mod` の宣言が要るので、共通の部品（送り口の引き方・「降りた」の文言）は `sakurascript.rs` に置いて `raise_event.rs` から `super::sakurascript::…` で使うか、2 本に重ねて書く。
- `doc/ssp-mcp/areka-tools.md`: ④ が直す。本 spec の差の一覧は新しいファイルに置く。

---

## 5. 作り方の選択肢

### 5.1 kanade の入口

**案 A: 既存の道を広げる**
- `KanadeMsg::RaiseEvent` に「許可の表を通さない」印と、台本つきの返信端を足す。`on_raise_event` の中で分ける。`sakurascript` は `RaiseEvent` と別に 1 変種。
- ✅ 変種が増えない ❌ W2 に当たる（④ の `update/desk.rs` ほか 7 ファイルとテストが壊れる）❌ `RaiseOutcome` を広げると W3 に当たる ❌ 要件 6.2「既存の呼び手の振る舞いと結果を変えない」を型の上で守りにくい

**案 B: 新しい入口を足す（2 変種＋新しいファイル）**
- `KanadeMsg` に「外からの台本」「外からのイベント」の 2 変種（どちらも返信端つき・`strict` つき）。殻の処理は新しい `actor_external.rs`、運行表の判断は新しい `schedule/external.rs`（受け付けの判定・`start_talk`・帳簿の掃除・任意の名前の GET の組み立て）、結果の型は新しい `src/external.rs`。`EventId` に第 3 の変種（例 `External(String)`）と送出の檻の規則を 1 つ。
- ✅ 既存の呼び手と `RaiseOutcome` を 1 行も変えない（W2・W3 を避ける）✅ 上限の近い `actor.rs`・`schedule/mod.rs` には振り分けの数行だけ ✅ brief の名前の方針（将来 SSTP が同じ変種を使える）に合う ❌ ファイルが増える ❌ 返事の作りが `RaiseEvent` と似た二重の形になる

**案 C: 混ぜる**
- 案 B の 2 変種を足すが、運行表の受け付けの判定（3.6）は既存の `on_raise_event` とも共有する（既存の道の「`Steady` だけ」の判定はそのまま・新しい道は保留の終了・切替も断る）。任意の名前の GET の組み立ては `events::raise` を `EventId` を受ける形に一般化する。
- ✅ 判定と組み立ての重複を減らす ❌ `events::raise` の呼び手（`on_raise_event`）も触る ❌ 既存の道の振る舞いを変えない証明（要件 6.1・6.2）がテストで要る

### 5.2 script 種別のログの置き場

| 案 | 所 | 長所 | 短所 |
|---|---|---|---|
| L1 | 殻 `actor.rs` の `execute_batch` の `Action::StartTalk` の腕（棚卸㉗の推し） | 実際に送る直前＝「再生を始める」に最も近い | 元のイベント名が無い（`origin` は `&'static str`・翻訳の後は `OnTranslate`）＝殻の中で別に控える仕組みが要る。捕捉が殻のスレッドになり今のテストの道具で拾えない。`actor.rs` は 901 行 |
| L2 | 運行表の出口（`step` の最後・`balloon_events::settle` と並べるか新しい子 `schedule/script_log.rs`） | 元のイベント（`reply_source`・`TranslateWait::source`）と最終の台本が手元にある。`step` は `config` を受けるので名前も読める。`step` を呼んだスレッドで捕まえられる＝決定論テスト ⑽ が書ける。`boot.rs`・`close.rs`・`steady.rs` の構築点に触らない | 運行表は「`tracing` 以外の副作用を持たない」純粋な層で、ログは許されている。ただし「指示を返した」と「殻が送れた」の間の失敗（再生の側が落ちている）でも 1 件残る |
| L3 | `StartTalk` に表示の語の欄を足し、殻か dispatcher で出す | 語が台本と一緒に運ばれる | 構築点 11 か所・`boot.rs`（④）・`script-security-level` と重なる＝**W1 と同じ壁** |

L2 を採る場合、翻訳を通ったトークのために「預けた一括の元の出どころ」を `TranslateWait` に持たせ、`on_done` から出口まで渡す欄が要る（`translate.rs` の変更）。

### 5.3 `sakurascript` の翻訳の預け方

- T1: `schedule/translate.rs` に「外からの台本を預ける」関数を足す（`capture` と同じ帳簿 `TranslateWait` を使い、`TranslateRequest` の元を「イベント」か「SSTP」かの 2 択の型にする）。`events::on_translate` は出どころで Ref1 を変え、SSTP なら Reference の列を 2 つ（Ref0・Ref1）で止める（末尾の欠番＝印が要らない・`replace_absent_markers` を触らない）。
- T2: 外からの台本を偽の「SHIORI の応答」として運行表に入れ、既存の `capture` に乗せる。❌ 「SHIORI 応答を外から注入できない構造」（`KanadeMsg` の説明・DD-2）を崩すので勧めにくい。

---

## 6. 設計の段で決めること（議題の候補）

1. **`strict` を運ぶ先（W1）**: kanade の中でトークの番号と結ぶだけにするか、`TalkCommand` まで運ぶか、`StartTalk` まで運ぶか（④ と申し合わせ）。`mcp-strict-errors` の未決の議題（入口で診断するか・印を再生の各所まで運ぶか）と `script-security-level`（出どころの印の入れ物）に掛かる。要件 5.3 の「再生する側が受け取った値」をどの境で見るかもここで決まる。
2. **script 種別のログの置き場（L1／L2／L3）**: 棚卸㉗の brief の推し（殻）と、テストの書きやすさ・イベント名の手に入りやすさ（運行表の出口）のどちらを取るか。
3. **SHIORI のエラー応答の warn と要件 6.5 の食い違い**: 外からのイベントの往復でエラー応答が返ると、送出点 `round_trip_request` が `shiori_error_response`（warn）を残す。要件 2.4 はこの場合を成功（`OK:the ghost returned no script.`）とし、要件 6.5 は成功の答えで warn 以上を出さないとする。外からのイベントだけ記録の水準を下げるか、エラー応答の warn は「ゴーストの不具合の知らせ」として残し要件 6.5 の例外に数えるか（要件の読み直しになるなら要件の討議へ戻す）。
4. **任意の名前の型と送出の檻の規則**: `EventId` の第 3 の変種の受理の規則（入口で空・制御文字を弾いた後は全部通すか・`OnTalk`／`OnHour` の恒久禁止を外からの頼みにも効かせるか）。殻の `origin` に載せる固定のラベルの綴り（例 `"OnExternalEvent"`）。
5. **手順の途中の判定の範囲（要件 3.6）**: `Steady` で保留の終了（`pending_close`）・保留の切替（`pending_change`）があるときを断るのは要件どおり。シェル・バルーンの切替の印の台詞（`talk_gap` の見張りが立っている間）を「手順の途中」に数えるか。数えないと、外からの台本が印の台詞を置き換え、シェルの切替の側が「台詞が始まったか」を突き合わせる所（`talk_gap::observe`）に影響しうる。
6. **2 行の答えの区切りの字（要件 2.3）**: SSP の生の応答（`\n` か `\r\n` か）。要件は「設計の段で確かめる」と書く＝SSP の MCP で 1 回測る（本書では測っていない）。
7. **「制御文字」の範囲（要件 2.7）**: `char::is_control`（C0・DEL・C1・TAB を含む）か、改行と NUL だけか。TAB を含む Reference を送りたい用途があるか。
8. **`sakurascript` の台本の MAKOTO の口の 2 つ目の引数**: 元のイベントの ID の代わりに何を渡すか（空文字・SSTP の語など）。本番は素通しなので今は振る舞いに出ない。
9. **ゴーストの名前の代わりの値の一致（W4）**: descript に `name` が無いゴーストで、kanade が `config.rs` で作るフォルダの絶対パスと、MCP の宛先の値（`resolve::listed_value`＝`std::path::absolute(session.ghost_dir())`）が字まで一致するか。一致しないなら `runtime.rs` から渡す形（④ と申し合わせ）か、名前が無いときは既定の名（`[SYSTEM]`）に倒すか。
10. **翻訳の失敗と `sakurascript` の答え**: 外からの台本の翻訳の輸送路が失敗すると、kanade は終了の系列（Fault）へ進み、殻の返事はその後に出る。このとき `OK` と答えるか、「降りた」の文言で答えるか（要件 1.3 は受け付けた時点の `OK`、要件 3.5 は降りたときの文言）。
11. **MCP の側の共通部品の置き場**: `mod.rs` に触らないため、送り口の引き方と「降りた」の文言を `sakurascript.rs` に置いて `raise_event.rs` から使うか、2 本に重ねて書くか（`get_status.rs` と合わせて 3 本の重複になる）。

---

## 7. 調べが要るもの（設計の段へ持ち越す）

- **R1（SSP で測る）**: `raise_event` の 2 行の区切りの字（議題 6）。測るなら `raise_event` を 1 回（`OnAiTalk` など台本を返すもの）呼んで生の JSON を残す。
- **R2（コード）**: 第 3 の `EventId` を足したとき、kanade の外で `EventId` を網羅の `match` で読む所（偽の SHIORI・`crates/areka/src/emo2_boot/spine_conformance_script.rs` ほか）が無いかの洗い出し。今見つけたのは kanade の `actor.rs` の 2 か所と `msg.rs` の `as_str` だけ。
- **R3（コード）**: 運行表の出口でログを出す場合、`TranslateDone` の入力の `step` で元の出どころを手に入れる道（`on_done` が消す `TranslateWait` から渡す）と、起動の挨拶（`BootVersion`）・終了の挨拶（`CloseTalkWait`）・切替（`ChangeTalkWait`・`ChangeCloseTalkWait`）・選択肢の `script:`（`steady_choice_script`）・アンカー（`schedule/anchor.rs`）の各道で表示の語が要件 4.2 のとおりに決まることの表。
- **R4（コード）**: MCP の側の決定論テスト（要件 7.3）で、kanade のスレッドで出る行をどう読むか。案: 行を組む関数を kanade から公開し、MCP の側のテストがその関数を呼んで `capture`→`log_history::draft` で読む（`log_history_convention_tests.rs` の `real_macros_with_the_convention_targets_draft_kind_name_label_and_body` と同じ形）。
- **R5（運用）**: 全部のトークの台本が info で標準出力にも出る（履歴の層のための info）。長い台本の起動の記録が増えることを `real-run-log-noise` の見張りと照らす（警告ではないので要件 6.5・7.6 ⑺ には当たらない）。
- **R6（行数）**: `actor.rs` 901・`msg.rs` 930（テストを出せば約 510）・`schedule/mod.rs` 969（型の塊を出せば約 570）・`schedule/events.rs` 850・`schedule/translate.rs` 251・`schedule/steady.rs` 957（触らない前提）。`events.rs` に任意の名前の組み立てと Ref1 の口を足しても 1,000 には届かない見込みだが、足す量は設計で数える。

---

## 8. 規模とリスク

- **規模: M〜L（14〜19 タスク）**。内訳の見立て: 上限の近いファイルを割る 2（`msg.rs` のテスト・`schedule/mod.rs` の型）／kanade の型と入口 3（2 変種と結果の型・`EventId` と送出の檻・殻の新しいファイル）／運行表 3（受け付けの判定と外からの台本・任意の名前の GET・翻訳の預けと Ref1）／script 種別のログ 2（名前の欄と `config.rs`・出口の行と表示の語）／MCP の側 2（`sakurascript`・`raise_event` の `handle` とテスト）／差の一覧と COMPAT の行 1／実機の確認と README 1〜2。`strict` を `TalkCommand` 以下まで運ぶなら +1〜3。
- **リスク: 中**。新しい技術も新しい並行の設計も無い（返事を待つ形は `get_status`、トークの開始は `start_talk`、置き換えは既存の腕）。一方で、⑴ 運行表の出口に全部のトークの記録を足すので、既存の決定論テストの記録の件数の主張（`log_capture` の `logged_once` など）に触れうる ⑵ 送出の檻に第 3 の出どころを足す（受理の規則を誤るとゴーストが止まる道に入る）⑶ ウェーブの約束（W1・W4）を設計で外さないことが要る。

---

## 9. 設計の段への申し送り

- **寄りの形**: 案 B（新しい 2 変種＋新しいファイル）＋ログの置き場 L2（運行表の出口）＋翻訳の預け方 T1、`strict` は議題 1 の答え次第で最小（kanade の中で結ぶ）から始める形が、ウェーブの約束をすべて避け、既存の呼び手を 1 行も変えない。ただし L1（棚卸㉗の brief の推し）と W1 の ⒝ は、後続の `mcp-strict-errors`・`script-security-level` の作り方と合わせて比べる価値がある。
- **最初の 2 タスク**は brief どおり上限の近いファイルを割る（`msg.rs` のテストを兄弟の `msg_tests.rs` へ・`schedule/mod.rs` の型の塊を兄弟へ出して同じ名前で見せる。`schedule/boot.rs` を変えないことを割り方の条件にする）。
- **② `startup-options-core` への申し送り**: 本 spec は `crates/areka-ghost/src/shiori_wiring.rs` に触らない。
- **④ `mcp-reload` への申し送り**: 本 spec は `crates/areka-ghost/src/runtime.rs` に触らない見込み（ゴーストの名前は `config.rs` の中で入れる）。議題 9 で `runtime.rs` が要ると決まったときだけ申し合わせる。`schedule/close.rs` は W1 の ⒝・⒞ を採らなければ触らない。
