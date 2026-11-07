# ギャップ分析: audit-kanade

> 2026-11-06・枝 `claude/kiro-start-audit-kanade-99d972`（HEAD `cda8a46e`・`main` の `73c5b25d` を取り込み済み＝`main` は HEAD の祖先）。
> 物差しは ponytail のプラグインの `ponytail-audit`。要件の書く 5.3.2 の置き場（`~/.claude/plugins/cache/ponytail/ponytail/5.3.2/`）はもう無く、今入っている 5.3.3 の `~/.claude/plugins/cache/ponytail/ponytail/5.3.3/skills/ponytail-audit/SKILL.md` を使った（作法の中身は「最大 20 件」「読まなかった所」を含めて 5.3.2 と同じ形）。
> 作法のうち 2 つを外した（要件 1.3・1.4）: 「最大 20 件」は外して全件を載せる。「読まなかった所」は残さない（範囲の 146 ファイルを全部読んだ）。
> 読み方: 12 本の読み手（読むだけのサブエージェント）に範囲を重ならないように分けて全行を読ませ（1 本 3,000〜4,400 行）、出た指摘をここでまとめ、重なりを 1 件に寄せ、番号を振り直した。載せた指摘はどれも、こちらでファイルと行を開いて中身を確かめた（確かめられなかったものは落とした）。clippy は下の §1 のとおり実際に回した。

## 1. 範囲の測り直し（要件 1.2）

| | 起票のとき（main `e3c2cbe4`） | 棚卸㉟（main `18e4cb30`） | 今（HEAD `cda8a46e`） |
|---|---|---|---|
| ファイルの数 | 133 | 146 | **146**（Rust 145＋`Cargo.toml` 1。fixture のファイルは無い） |
| コードの行 | 10,281 | 約 11,200 | **11,153**（38 ファイル） |
| テストの行 | 33,493 | 約 36,100 | **36,075**（107 ファイル・`_tests.rs`／`tests/`／`_test_support.rs`） |
| `Cargo.toml` | — | — | 27 行 |

- 数え方の注: `src/schedule/log_capture.rs`（226 行）と `src/status.rs` の後半（307 行目からの `mod tests`）は、名前の決まりではコードに入るが中身はテスト。
- 1,000 行の近く: `src/actor_tests.rs` 983・`src/schedule/steady.rs` 973・`src/schedule/events.rs` 943・`src/schedule/steady_flow_tests.rs` 931・`tests/kanade/steady_test.rs` 924・`src/schedule/schedule_tests.rs` 903・`src/schedule/events_tests.rs` 887・`src/schedule/steady_choice_timeout_tests.rs` 876・`tests/kanade/failure_test.rs` 863・`src/actor.rs` 852。
- 棚卸㉟の後、範囲のファイルは 1 つも変わっていない（棚卸の数と同じ）。

### clippy の実測（要件 7）

`cargo clippy -p areka-kanade --all-targets --no-deps -- -D warnings` を一度回した（記録は `target\audit-kanade\clippy.txt`）。結果は**赤**（終了コード 101）。

- lib: `collapsible_if` 1 件（`src/shiori/real.rs:331`）。
- lib test: 「5 件」の内訳は `type_complexity` **4 件**（`src/actor_online_tests.rs:21`・`src/schedule/balloon_events_cleanup_tests.rs:189`・`src/schedule/user_break_tests.rs:127`・`src/schedule/schedule_tests.rs:419`）＋lib と同じ `collapsible_if` 1 件の重なり。申し送りの「`type_complexity` 5 件」は数え違い。
- **結合テスト（`tests/kanade.rs`）は lib が赤で止まるので調べられていない**。`-D warnings` を外して回し直すと（`target\audit-kanade\clippy-warn.txt`）、ここにさらに **11 件**ある: `manual_contains`（`iter().any()` の代わりに `contains()`）7 件（`close_test_boot_greeting_tests.rs:113・168・247・414`・`mouse_test_talk_start_tests.rs:394`・`steady_test.rs:351・480`）・`doc_lazy_continuation` 3 件（`close_test_boot_greeting_tests.rs:354-356`）・`drain_collect` 1 件（`common/common_mock_sakura.rs:296`）。
- 要件 7.1 の「警告 0 件で通る」には、この隠れた 11 件も直す要がある（指摘 #18）。

## 2. この範囲がしていること・想定した負荷

- **何をするか**: kanade はゴースト 1 体ごとに 1 本のスレッドで動く役者で、ゴーストの起動（`OnInitialize`→`OnFirstBoot`／`OnBoot`）・ふだん（毎秒の `OnSecondChange`・マウス・選択肢・バルーンのイベント・外からの台本 SSTP・翻訳 `OnTranslate`）・切替・終了（`OnClose`→別れの台詞→`Unload`）の段取りを決める。段取りは「状態＋入力→状態＋行い」の純粋な関数（`schedule/`）で、殻（`actor*.rs`）が SHIORI との往復を同期で回し、答えをすぐ入れ直す。SHIORI へは 32 bit の中継（`shiori/real.rs`）か同じプロセスの中の口で届く。
- **想定した負荷**: 利用者は 1 人・ゴーストは 1〜数体。プロセスは何日も動く。SHIORI の答え・ゴーストの台本・MCP から来る台本は**信用できない外からの入力**で、大きいことがある（「数 KB」で線を引かない）。毎秒の刻みとマウスの出来事が続けて来る。テストは全体テストで並列に、重い机で回る（並列で CPU が埋まる前提で、時刻で競う形は赤になりうる）。

## 3. 指摘の一覧（全 96 件）

組の名は audit の 3 つ: **Must fix（必ず直す）**・**Should fix（直したほうがよい）**・**Nice to have（できれば）**。番号は 3 つの組を通して 1 から。場所は `crates/areka-kanade/` からのパス。

- 数: **Must fix 2 件・Should fix 51 件・Nice to have 43 件**。
- 「範囲外」の欄: 直しが `crates/areka-kanade/` の外のファイルに及ぶとき、そのファイルと持ち主の spec。「—」は及ばない。要件 6 で触れないもの（`crates/areka/`・kanade の公開の口の形・`areka-talk`／`areka-actor` の形）に及ぶときは **【6.x】** と書いた（そのままでは直せない＝議題）。
- 「残す候補」の欄: 直さずに残してよい 3 種類（⑴ 開発者の裁定・⑵ わざと置いた先送りの印・⑶ SSP／ukadoc の正典に関わる振る舞いの変更）に当たるかもしれないもの。決めるのは要件の討議（要件 2.2）。「—」は当たらない。

### 3.1 Must fix（2 件）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 1 | `src/actor.rs:664-677`（`round_trip_raw` の送ってよいかの判定）・`src/schedule/events.rs:864-870`（`on_translate_sstp`）・`:877-927`（`on_choice_timeout`・`on_balloon_*`）・`:699-790`（選択肢・アンカーの組み立て） | SHIORI への要求の頭書きに、外から来た文字列の改行がそのまま入る。CR・LF・NUL を断る検査（`sstp::validate_notify`）が効くのは `EventId::Sstp` と `EventId::Property` の 2 つだけ。MCP の `sakurascript`（`crates/areka/src/mcp/sakurascript.rs:63-83`・空かどうかしか見ない）に `"\\0a\r\n\r\n\\e"` を送ると、台本は `OnTranslate` の Reference0 にそのまま載り、要求を組む `shiori-host32-host/src/shiori3.rs:138-150` も値をそのまま書くので、頭書きがそこで切れて Reference1 の `sstp-send,owned` が届かない。`"\r\nReference1: …"` を書けば偽の行も差し込める。再生の後の `OnChoiceTimeout`（Ref0＝台本まるごと）・`OnBalloonClose` などでも同じ。完了した `areka-P0-mcp-kanade-tools` は NOTIFY の道でこれを「信頼の境の検査」として塞いだが、SEND の道には無い（同じ決まりが場所で違う） | 送る点で、どの種類の ID でも「ID と各 Reference に CR・LF・NUL が無い」を必須にする（欠番の印 `ABSENT_REFERENCE`＝`"\0"` ちょうどは除く）。ただし送る点で断ると翻訳の失敗＝ゴーストが降ろされる（`Failed`→`Unloading{Fault}`）ので、SEND の入口（`schedule/sstp.rs` の `on_send`）で先に断る形と組みにする。台本を受け付けない返事の語（`SstpSendOutcome`）をどうするかは議題 | 入口で断るなら `crates/areka/src/mcp/sakurascript.rs` **【6.1】**。`SstpSendOutcome` に値を足すと `crates/areka` の `match`（同 `:94-105`）が要る **【6.3】**。根で断るなら `crates/shiori-host32-host/src/shiori3.rs`（持ち主 `audit-shiori-host32`・C18 には居ない） | ⑶ SSP の MCP `sakurascript` が生の改行を含む台本をどう扱うか（断る・消す・そのまま）は未実測 |
| 2 | `src/schedule/talk_gap.rs:64-86`（`begin`） | 再生中にゴーストの切替が保留（`pending_change`）になった後で、印つきの依頼（例 `OnShellChanging`）が来ると、`begin` は保留の**終了**だけを見て保留の**切替**を見ないので GET を送る。同じ一歩の `decide`（`:205-210`）は `pending_change` を見て `Left{GhostChange}` に決め、表示の側はシェル切替を取りやめる。ところがゴーストにはもう `OnShellChanging` が届いていて、その答えの台詞が今のトークを置き換えて流れる＝シェルは変わらないのに「シェルを変える」台詞を話し、`OnShellChanged` は来ない。テスト（`talk_gap_tests.rs:450-481`）は起動中と終了の保留しか見ていない | `:64` の条件に `\|\| state.pending_change.is_some()` を足す。テストの表に「切替の保留中は送らず `Left{GhostChange}`」を 1 行足す | — | — |

### 3.2 Should fix（51 件）

#### 本番のコード（段取りと殻）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 3 | `src/schedule/change.rs:84-122`（`on_raise_event`・`on_property_event`） | 定常かどうかしか見ず、終了の保留・切替の保留を見ない。マウス（`steady.rs:82`）・外からの台本（`sstp.rs:36` の `accepts`）・台詞の切れ目（`talk_gap.rs:64`）は保留中は送らない＝同じ決まりが入口で違う。再生中に利用者が終了を選んで保留になったところへファイルのドロップ・更新・位置の初期化（どれも `RaiseEvent`）や `\![get,property]` の返しが来ると、GET が出て答えの台詞が今の台詞を置き換え、`OnClose` がその台詞の終わりまで遅れる（切替も同じ） | 両関数の「定常でなければ捨てる」の判定に `pending_close.is_some() \|\| pending_change.is_some()` を足す（warn して捨てる） | コードは無し。`crates/areka/src/update/worker.rs:137` が保留中に `NotSteady` を受けるようになる（振る舞いの変化を確かめる） | ⑶ 終了の保留中に来た通知の依頼を SSP が送るかは未確認 |
| 4 | `src/schedule/mod.rs:479-498`（`on_talk_done` の `Some(_) \| None` の腕）・消す側 `steady.rs:843`・`sstp.rs:54・128` | 「置き換えた古いトークの遅い完了」を info で捨てる 1 世代の控え（`choice_prev_talk`）は、選択肢による置き換えのときだけ入り、マウスの答え・SSTP の台本による置き換えでは `None` に消される。トーク A が終わって dispatcher が A の完了を投げたのと同じ時に kanade が A を B へ置き換えると、A の完了が `unknown_talk_done`（ERROR）になる＝正常な操作で error が出る（コメント `:482-485` の「真に未知の id 専用」が破れる） | どの置き換えでも、古い番号を控えに入れる（名前を `prev_talk` に） | — | ⑴ `types.rs:224-226`「消去点は…次の slot 差替（マウス由来の置換を含む）である（C4 規則 9）」＝控えを消す決めを変える |
| 5 | `src/actor_sstp.rs:26-37`（`first_reply_of`）と `src/schedule/steady.rs:799-818・831-850`（定常の Value の腕）・`src/schedule/sstp.rs:126` | 「空白だけの台本」の扱いが 3 か所で 2 通り。返事の側（`first_reply_of`）と SSTP NOTIFY の腕は空白だけを「答え無し」とし、steady は空白だけの台本で新しいトークを始める。再生中にメニューから `OnResetWindowPos` を頼み、ゴーストが `"\r\n"` を返すと、呼び手は「何も起きなかった」と受け取って窓を並べ直すのに、kanade は今のトークを空のトークで置き換える。`actor_raise_reply_tests.rs:155-187` はこのずれを固定している | 「台本があるか」を 1 つの関数（`!s.trim().is_empty()`）にまとめ、3 か所から呼ぶ。steady では空白だけを 204 と同じに | — | ⑴ `change.rs:122`「写すのは最初の往復の結果だけで、運行表の判断は変えない」／⑶ 空白だけの台本を SSP が再生するかは未確認 |
| 6 | `src/schedule/user_break.rs:24-66`（`on_user_break`）・`src/msg.rs:241-247`（`KanadeMsg::UserBreak{scope}`） | 利用者の中断の知らせはスコープしか持たず、どのトークのバルーンが押されたか分からない。読み終えたトーク X のバルーンをダブルクリックした瞬間に毎秒の問い合わせで次のトーク Y が始まり X の表示がまだ残っていると、kanade は利用者が一度も見ていない Y を止め、`OnBalloonClose(X)` の代わりに Y の台本で `OnBalloonBreak` を送る。時間切れの知らせ（`BalloonTimeout{talk_id}`）は番号を持ち同じ行き違いを捨てられる＝同じ種類の入力で決まりが違う | 表示の側が押されたバルーンのトークの番号を載せ、番号が今のトークと合うときだけ止める（違えば表 C へ） | `crates/areka/src/input_events/user_break.rs:338` **【6.1】**・`KanadeMsg::UserBreak` の欄を足すと組み立ての形が変わる **【6.3】** | ⑴ 要件 2.5「場面でもスコープ番号でも結論を変えない」の受理の規則を細かくする変更 |
| 7 | `src/schedule/steady.rs:555-601`（`fire_choice_timeout_if_due`・呼び手は `on_tick` だけ） | 選択待ちの時間切れは kanade の刻み（1,000 ms ごと・`crates/areka-ghost/src/ticker.rs:38-52`）でしか判定されない。`\![set,choicetimeout,500]` で表示の終わりが 10,001 なら期限は 10,501 なのに 11,000 の刻みまで出ない（最大 999 ms 遅れる）。「時刻は正確に扱う・待ちは丸めない」の大原則に反する | 期限に合わせて 1 回だけ起こす口を入れるか、細かい刻みで届ける。kanade の中だけでは閉じない | `crates/areka-ghost/src/ticker.rs`／dispatcher（持ち主 `audit-ghost-catalog`＝着地済み・C18 には居ない） | ⑴ 完了 `areka-P0-choice-select-events` の design.md:566「Tick（steady）…既存 pump 処理に先行して deadline 判定」（10-05 の大原則より前の決め） |
| 8 | `src/schedule/steady.rs:570-580`（`choice_timeout_ledger_stale`） | doc（`:545-554`）自身が「到達したら不変条件の破れ」と書く防御の枝が `trace!` しか出さない。将来の改修で帳簿と今のトークがずれると時間切れは二度と出ず、選択待ちが黙って無期限になり、帳簿も残るので毎秒同じ枝を通り続ける | `warn!` に上げ、その場で帳簿を消す（`clear_choice_ledger(state, "choice_timeout_ledger_stale")`） | — | ⑴ doc が「発火せず trace で観測して通常 Tick へ譲る」を選んだ理由として書く（ログの重さを選んだ理由は無い） |
| 9 | `src/actor_resources.rs:36-63`（`outcome_of`）と `src/schedule/boot.rs:160-196`（`on_prefetch_reply`） | SHIORI の答えをリソース照会の結果へ写す処理が 2 つ。コメントは「同じ写像」と言うが既に違う（想定外の答えの理由が `"unexpected"` と `"unexpected prefetch outcome: {kind}"`）。片方だけ直すと、ユーザー名の先読みとメニューの照会で同じ答えの扱いが分かれる | `outcome_of` を `schedule/resources.rs`（純粋な層）へ移して両方から呼ぶ。boot は固有の warn と記録だけ残す | — | — |
| 10 | `src/schedule/close.rs:141-167`（`on_close_talk_wait` の刻み）と `src/schedule/change.rs:304-321`（`on_talk_wait` の刻み）・`change.rs:12` の doc・`change.rs:319` | 「最初の刻みで期限を決める→過ぎたら降ろす→過ぎていなければ待つ」が 2 つの写し。doc は「`deadline_from` だけを共有する」と言うが、2 つとも `now + close_talk_deadline_ms` を直に書き `deadline_from`（`close.rs:181`）を使わない。期限切れのときの選択の帳簿の消去は change にだけあり（`:319`・いつも空なので何もしない）、読み手が「決まりが違う」と迷う | 判定を 1 つの関数にして両方から呼ぶ（最低限、式を `deadline_from(Some(now), config)` に）。`:319` の空振りの消去は消すか両方に揃える | — | — |
| 11 | `src/schedule/steady.rs:408-424・630-646・800-812・836-850`・トークの番号の手書きの採番 `mod.rs:117-118`・`steady.rs:409・631・801・837`・`boot.rs:291・312`・`change.rs:241・403`・`close.rs:66` | 「番号を取る→`Steady{Some(ActiveTalk)}`→`StartTalk::new`」が `mod.rs:111` の `start_talk` と同じ中身で steady に 4 つ写され、`+= 1` の採番が 10 か所にある。1 か所で採番を忘れると同じ番号が 2 回使われ、dispatcher の古い知らせの判定が狂う。`ActiveTalk` に欄が増えると 4 か所が食い違う | steady の 4 つを `super::start_talk(..)` に（出どころは後で `talk_origin::stamp` が上書きするので振る舞いは同じ）。採番は `take_talk_id(&mut State)` を 1 つ。steady.rs は約 40 行減る | — | — |
| 12 | `src/schedule/events.rs:318-320`（`is_allowed_choice_event`）・`src/schedule/anchor.rs:30-36`（`plan_anchor`）・`src/schedule/choice.rs:71` | 「`On` で始まる」の判定が 3 つ。送る点の規則（`events.rs`）だけを直すと、振り分けでは名前のイベントとして組んだのに殻が `event_id_not_allowed` で失敗にし、アンカーではそれが `anchor_shiori_failed_as_204` の error に化けて黙って 204 になる | `plan_anchor` と `choice.rs:71` で `events::is_allowed_choice_event(id)` を呼ぶ | — | — |
| 13 | `src/schedule/events.rs:545-565`（`on_mouse_move`）・`:583-608`（`on_mouse_double_click`）と `:642-665`（`mouse_get`） | 7 つの Reference の並び（Ref6＝`mouse`・Ref4 の `None` を空に）が 3 か所に写されている。デバイスの種類を引数に足すとき（`REF6` の印が予告する変更）に 1 か所だけ古いまま残る | 2 つの関数の中身を `mouse_get(..)` の 1 行に（約 30 行減る＝943 行の events.rs に余裕） | — | — |
| 14 | `src/shiori/real.rs:144-200`（`impl ShioriBackend for ShioriConnection` の `get`／`notify`／`*_with_headers`） | 「client を作る→版を渡す→リソースなら印」の 4 行が 2 組（4 か所）ある。殻が呼ぶのはヘッダつきの 2 つだけで、頭なしの `get`／`notify` は本番では通らない（trait が求めるので在る）。片方だけ直すと版や印が食い違う。テストの本文検査（`real_tests.rs:728` の `&mut self.negotiator` 4 件・`real_resource_mark_tests.rs` の `for_resource(` 2 件）がこの写しの形を縛っている | 私的な `fn client(&mut self, id, resource) -> Shiori3Client<'_>` を 1 つ作り 4 つの手から呼ぶ。本文検査の件数を合わせる | — | ⑴ 本文検査は完了 `charset-canon` 要件 5.1・`shiori2-protocol` 要件 3.3／3.7 を引く（件数は変わるが趣旨は保てる） |
| 15 | `src/schedule/boot.rs:304・323`（`origin: "boot"`）と `src/schedule/sstp.rs:15-16`（`BOOT_GREETING_ORIGIN`）・`src/schedule/sstp_tests.rs:539` | 起動の挨拶の見分けが、boot の直書きと sstp の定数の綴りが一致することに頼る。`sstp_tests.rs:539` の `assert_eq!(BOOT_GREETING_ORIGIN, "boot")` は定数と同じ字を比べるだけで、boot の綴りが変わっても赤にならない | boot の 2 か所で定数を使う（または定数を boot.rs へ）。定数どうしの assert を消す | — | — |
| 16 | `src/lib.rs:38-42`（`schedule` モジュール全体への `#[allow(dead_code)]`） | 理由に挙げた 2 つの欄のうち `ActiveTalk.origin` は今は読まれている（`schedule/sstp.rs:33` の `accepts` ほか）。読まれないのは `ClosePending.reason` だけなのに、許しが約 60 ファイルのモジュール全体に掛かり、`schedule` に新しく死んだコードができても警告が出ない | 許しを `ClosePending.reason` の 1 欄へ移す（または欄ごと消す）。モジュールの許しを外し、出てくる警告を直す（何が出るかは cargo で見る＝Research Needed） | — | ⑵ 理由つきの許しだが、理由の半分が切れている |
| 17 | `tests/kanade/common/mod.rs:19-25`（`#![allow(dead_code)]`）・`:74-78`（`#[allow(unused_imports)]`）・`common/common_bounded.rs:22-24`（`run_bounded`）・`common/common_mock_shiori.rs:214-215`（`BlockOn::Notify`） | 許しの理由は「4.1 時点のプレースホルダ段階に限定」で、その段階はとうに終わっている。`run_bounded` は `tests/`・`src/` のどこからも使われない（しかも閉包の中の panic を「possible hang」と報告する）。`BlockOn::Notify` も使われない（使うのは `BlockOn::Get` だけ）。許しがあるので、新しく使われなくなった道具に誰も気づかない | `run_bounded`・その再輸出・`#[allow(unused_imports)]`・`mod.rs:14` の言及・`BlockOn::Notify` を消し、`#![allow(dead_code)]` も外す | — | ⑵ 理由つきの許しだが、書いた限界（プレースホルダ段階）が切れている |
| 18 | `src/shiori/real.rs:331`（`collapsible_if`）・`src/actor_online_tests.rs:21`・`src/schedule/balloon_events_cleanup_tests.rs:189`・`src/schedule/user_break_tests.rs:127`・`src/schedule/schedule_tests.rs:419`（`type_complexity`）・`tests/kanade/` の 11 件（§1） | **要件 7（申し送りの件 1）**。`-D warnings` で赤。申し送りの数（`type_complexity` 5）は 4＋重なり 1 で、結合テストの 11 件（`manual_contains` 7・`doc_lazy_continuation` 3・`drain_collect` 1）は lib の赤に隠れて数えられていない | `real.rs:331` は `if a && let …` に（clippy の示す形・振る舞いは同じ）。`type_complexity` は型の別名を 1 つずつ（`spawn_with` の戻りの組ほか）。結合テストの 11 件は clippy の示す形に。直す前と同じテストを同じ結果で通す（要件 7.2） | — | — |

#### `src` のテスト

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 19 | `src/actor_tests.rs:137-160`（`shiori_disconnected_send_failure_terminates_into_fault`）・`:164-200`（`sakura_disconnected_start_talk_failure_continues_run`） | 前者は起動の直後に `drop(kanade_tx)`（`:149`）するので、故障の道が壊れて kanade が止まらなくなっても受信口が閉じて終わり、緑のまま。後者は最後に送る Close で必ず止まるので、「運行が続く」を見ていない | 前者は消す（`tests/kanade/failure_test.rs:750` が同じことを見る）。後者は join の後に偽の shiori の記録に `Unload` が無いことを足す | — | — |
| 20 | `src/actor_resources_tests.rs:17-48`（`all_phases`）・`:94-99` | 「相が増えたらコンパイルが判断を求める」と言うが、効くのは `expected_queryable` だけ。`all_phases` に切替の 4 相（`ChangePending`・`ChangeTalkWait`・`ChangeClosePending`・`ChangeCloseTalkWait`）が無く、`queryable` が切替中に真を返すように壊れても赤にならない | 4 相を足し、件数 12 を 16 に | — | — |
| 21 | `src/translate_tests.rs:29-43`（`seams_are_plain_string_closures_movable_to_another_thread`） | テスト自身が書いた閉包の出力を期待と比べるだけ。`Send` かどうかは `spawn_kanade_translating` がスレッドへ移すところでコンパイルが保証している | 消す | — | — |
| 22 | `src/msg_tests.rs:36-64・71-116・123-207・249-266・268-307・309-351・428-432・434-486`（8 本）・`:353-377` | 書いた値を読み戻す・derive した `PartialEq`／`Ord` や標準のチャンネルを試すだけで、試されるコードが無い。`existing_eight_kanade_msg_variants…` は手で書いた名前を手で書いた名前と比べるだけで、変種を足すたびに直す手間だけが残る（コメントに 15 回分の跡）。`shiori_failure_display_matches_vocabulary` は `msg_fault_tests.rs:17-49` と 4/5 が重なる | 8 本を消す（約 330 行）。`Shiori("400")` の Display の 1 行だけ `msg_fault_tests` へ移す | — | — |
| 23 | `src/status.rs:379-410・412-438・447-482・484-497`（`mod tests` の 4 本） | 兄弟の `status_derive_tests.rs` の 48 通りの表と `reversed_and_duplicated_bindings_render_ascending_once` が全部覆う。`…entire_bool_input_space` は「入力空間を全部」と名乗りながら 1 軸しか見ず、同じ 2 つの assert を 2 回書く。導き方を変えると同じ期待を 2 か所で直す | 4 本を消す（約 120 行・`status.rs` のテストの塊が無くなる） | — | — |
| 24 | `src/actor_translate_tests.rs:332-360`（`translate_reply_level_rides_on_the_result_and_makoto_gets_only_the_script`） | 名前は「MAKOTO の口は台本だけを受ける」だが、見ているのは呼ばれた回数（`len()==1`）だけ。違う台本を渡すように壊れても緑 | `observed.makoto[0].0` が期待の台本と等しいことを足す | — | — |
| 25 | `src/schedule/schedule_variant_tests.rs:1-132`（ファイル全体）・`mod.rs:652-655` | 実行時の確かめは `tag(&X) == "X"` だけで落ちようがない。「変種が消えたらコンパイルが止まる」は本番の網羅の `match`（`route` の Input 21 種・`phase_label` の Phase 15 種・`execute_batch` の Action 9 種）が既に果たす。`{ .. }` で欄の変化も捕まえない。名前の「five_plus_choice_two」「eight_plus_choice_two」も今の数と合わない | ファイルと `mod` の宣言を消し、`schedule_tests.rs` の `choice_input`・`mouse_move` の `pub(super)` を外す | — | ⑴ コメントが Req4.4「既存 variant が無改変」を挙げる（本番の `match` で同じ保証が成り立つ） |
| 26 | `src/schedule/schedule_tests.rs:766-797`（`state_snapshot_preserves_the_talk_axis_of_phase`） | 期待値 `talk_active_of(&phase)` が実装（`types.rs:307-308`）と同じ関数の呼び出しで、判定が誤っても一緒に動いて緑。相ごとの「再生中か」は `external_state_tests.rs:454` が手書きの表で正しく縛っている | 消す | — | — |
| 27 | `src/schedule/resources.rs:374-387`・`:392-406`・`:343-354` の後半・`:164-169` | derive した比較を試すだけ・テスト自身の閉包を試すだけ・`resource_username` は `resource_get("username", ..)` そのものなので常に一致・`:185` の厳密な一覧に含まれる。どれも壊しても赤にならない | 3 本と `:343-354` の後半を消す | — | — |
| 28 | `src/schedule/choice.rs:212-228`（`plan_cascade_is_deterministic_for_repeated_input`）・`:362-375`（`choice_deadline_is_deterministic_for_repeated_input`） | 状態を持たない純粋な関数を同じ引数で 8 回呼んで比べるだけ。毎回同じ間違いを返しても通る。値の正しさは他のテストが縛る | 2 本を消す | — | ⑴ Req2.5「同一入力で同一結果」の証跡として置いた形（値のテストで足りる） |
| 29 | `src/schedule/steady_flow_tests.rs:297-319` と `:417-443` | 入力（`steady_some(TalkId(3),6)`・Value `"late"`・出どころ `OnSecondChange`）も確かめる中身も同じテストが 2 本 | 片方を消す（931 行のファイルが減る） | — | — |
| 30 | `src/schedule/change.rs:219・277・293・326・430`（`change_phase_unexpected`・`change_input_ignored`） | boot の同じ種類の守りの腕は `schedule_log_firing_tests.rs` が発火を見るのに、切替の 5 つはどのテストも名前で引かない。warn を消すと、切替の答え待ちに届いた想定外の入力が記録なしで捨てられても全部緑 | `schedule_log_firing_tests.rs` に `change::step` を直に回すテストを 2 本（`ChangePending` に `Notified`・`ChangeTalkWait` に `ShioriReply`） | — | — |
| 31 | `src/schedule/change.rs`（`ChangeState`「受理で立ち `Unloading` まで残る」）・横断の腕 `mod.rs:550-555` | `crates/areka` の切替（`emo2_boot/ghost_switch.rs:439`）は、停止通知に切替の中身があれば SHIORI の失敗の後でも切替を続ける。ところが kanade 側に「切替の途中で `Failed` → `Unloading{Fault}` でも `state.change` が残る」を見るテストが無い（`change_tests.rs` に `Failed` の入力は 0 件）。誰かが故障の道で帳簿を消すと、失敗で切替をやめてアプリが終わるようになるが赤にならない | `change_tests.rs` に 1 本（受理→`Failed(Timeout)`→`Unloading{Fault}` かつ `change.is_some()`） | — | — |
| 32 | `src/shiori/real_tests.rs:442-461`・`:463-483`・対象 `src/shiori/real.rs:420-435` | 名前は「warn を出す」「error を出す」だが記録を捕まえていない。`real.rs:422` の `warn!`・`:431` の `error!` を消しても緑（記録の無い失敗の道を誰も止めない）。さらに Clean でない降ろしの後の `unloaded = true`（`:421`）を消すと、次の手空きで `helper_exited` の error と `ShioriDown` が誤って出るが、どのテストも赤にならない（偽物の状態は常に Running） | `log_capture_kit::capture` で包み `unload_non_clean`（WARN）・`unload_failed`（ERROR）を 1 件ずつ見る。非 Clean のテストで降ろした後に状態を `Exited` にし、続く手空きで `ShioriDown` が来ないことを見る | — | — |
| 33 | `src/shiori/real_idle_tests.rs:12-14・88-100・191-262`（`IDLE_WAIT` 2 秒・本物の周期 500 ms） | 4 本とも本物の時計で競う（上限 2 秒の内に手空きが 2 回回ること・最初の `ShioriDown` が届くことを求める）。全体テストで CPU が埋まると、壊れていなくても赤になりうる（余裕は 4 倍だけ）。1 本ごとに 1〜2 秒眠り遅い | `run_shiori_loop`（私的）に周期を引数で渡し（本番は `IDLE_INTERVAL`）、テストは数 ms の周期と長い上限で回す | —（`tests/kanade/idle_pump_test.rs` は `IDLE_INTERVAL` を読むだけ） | — |
| 34 | `src/schedule/sstp_tests.rs:22-24・112-173・499-531` と `src/schedule/sstp_notify_tests.rs:25-87・422-467`（ほか `talk_gap_tests.rs:76-86`・`translate_path_tests.rs:347-357`） | テストの下ごしらえ（`boot_until_greeting_playing` 約 30 行・`steady`・`active`・`strings`・`config`・`warn_or_above`・`change_request`・`fingerprint`）が 2 つのファイルに同じ形で在る。`fingerprint` は `sstp_notify` の欄が片方にしか無く既にずれている | テスト専用の道具（`translate_test_support.rs` か `steady::test_support`）に 1 つずつ置き、両方から呼ぶ。`fingerprint` は `sstp_notify` 入りに揃える | — | — |
| 35 | `sstp_tests.rs:53-65` と `sstp_notify_tests.rs:254-267`・`sstp_tests.rs:67-78` と `sstp_notify_tests.rs:626-641`・`sstp_tests.rs:80-106` と `sstp_notify_tests.rs:544-567` | 同じ主張（Reference を補わない・Status の導き方・Ref1＝`sstp-send,owned`）を 3 組 2 か所ずつで縛る。仕様が変わると両方が赤になり、片方だけ直して重なりが残る | 各組の片方を消す | — | — |
| 36 | `src/schedule/boot_sequence_tests.rs:233・283・317・364・490・523`・`boot_reply_branch_tests.rs:416・499・551・585`・`schedule_log_firing_tests.rs:11-30・112-132`・`close.rs:209-255`（テスト）・小さな道具の写し（`boot_root_tests.rs:15`／`boot_reload_tests.rs:15` の `cfg`・同 `:68`／`:43` の `get_ids`・`boot_test_support.rs:3・8` の素通しの `initial()`／`config()`） | `State::initial()` と同じ値を 12〜17 欄ずつ手で並べた上で `..State::initial()` で埋める書き方が約 14 か所（約 200 行）。しかも `mouse_click`・`sstp_notify` が抜けていて揃っていない。`State` に欄を足すと、手で写した古い一覧が残る | `State { phase, last_now, next_talk_id, ..State::initial() }` だけにし、小さな道具は `boot_test_support.rs` へ寄せる | — | — |

#### 結合テスト（`tests/kanade/`）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 37 | `tests/kanade/status_query_test.rs:339-348`（`k4_query_after_close_ends_with_reply_dropped`）・`:26-28`（`send`） | kanade は Close を受けた瞬間に受信箱を捨てる（`src/actor.rs:143-146`）。テストのスレッドが Close を送ってから `StatusQuery` を送るまでに止められると、kanade が先に止まり、`send` の `expect("send to kanade inbox")` が panic して赤になる（並列で重いときに出る） | `:348` の送りだけ結果を捨てる形にする（送れなくても返信端は捨てられ、受け手には `Dropped` が届く＝主張は同じ） | — | — |
| 38 | `tests/kanade/failure_test.rs:704-717`（`error_response_at_close_get_closes_silently`） | 偽の sakura との TalkDone のやり取りを経ずに終わるのに、`sakura.started()` を sakura の join より前に読む。kanade が誤って別れのトークを起こしてすぐ止まっても、記録前の空の写しを読んで `all(..)` が緑のまま（`common_mock_sakura.rs:39-47` の「並行読みの罠」そのもの） | `drop(sender)` の後に `join_bounded_then_commands(..)` で読み、`Start` だけを拾って判定する | — | — |
| 39 | `tests/kanade/failure_test.rs:92-108`（`error_logged_with`）・`:150-160`・`:201-205・790-794・854-858`・`common/common_mock_shiori.rs:105-113`（`FailKind::make`） | 記録の捕まえ口は結合テストの実行ファイル全体で 1 つ（`log-capture-kit/src/global.rs:64-91`）。`choice_test_stage_failure_tests.rs` も同じ文言 `"injected ipc failure"` を注入するので、起動の道が `shiori_failed` を出さなくなっても、選択のテストが先に走っていれば Ipc の回は緑のまま。文言の写しも 2 か所（偽物だけ変えると本番が無事でも赤） | `FailKind::make` を公開して `witness` を `kind.make()` に。failure_test が注入する文言をこのテストにしか無いものにする（`FailOn` に文言を持たせる） | — | — |
| 40 | `tests/kanade/idle_pump_test.rs:40-59`（`sync_send_to_idle_window_returns_within_bound`） | 期限 `started + SEND_BOUND`（2 秒）の中に、窓の生成・`cmd.exe` の起動・同期送出・手空きの周期 500 ms が全部入る。重いと死活報告が間に合わず赤。doc（`:42-43`）は「全体 5 秒以内」と言いコードと合わない | 期限を `started + DEFAULT_TIMEOUT` に（doc の 5 秒と揃う） | — | — |
| 41 | `tests/kanade/full_run_test.rs:50-58・90-199`（`full_run_boot_pump_close_terminates_deterministically`） | 名前に「pump」とあり刻みを 3 本送るのに、`OnSecondChange` について何も確かめない。kanade が刻みを全部捨てるように壊れても緑 | 記録の中の `OnSecondChange` が 3 件あることを 1 行で見る（GET か NOTIFY かは問わない） | — | — |
| 42 | `tests/kanade/close_test_boot_greeting_tests.rs:19-34`（`wait_until`・呼び手 `:110-116・244-250・411-417`）・`tests/kanade/idle_pump_test.rs:216-223` | `yield_now` だけで最大 5 秒回り、回るたびに記録全体を錠つきで複製する。並列実行で kanade のスレッドに CPU が回らず期限切れの偽の赤が出る形で、同じ失敗の型は `common_bounded.rs:88-96` に実例つきで書かれ、そちらは sleep に替えて直してある | `wait_until` は消す（刻みは `release_all` より先に入口へ送っているので順は決まる・join の後の確かめで足りる）か、`drive_ticks_until_disconnect` と同じ短い sleep に。`idle_pump_test.rs:222` も短い sleep に。`:109` の誤った注も消す | — | — |
| 43 | `tests/kanade/mouse_test_test_support.rs:15-22`（`is_mouse_get`／`mouse_gets`）・`mouse_test_drag_tests.rs:35-42`（`drag_gets`）・使う所 `mouse_test_phase_guard_tests.rs:56・128・211`・`mouse_test_drag_tests.rs:239・278・320` | 「送らない」の確かめが「GET で ID が 2 つのどれか」だけを拾う。起動の前や終了の途中のマウスが誤って NOTIFY の `OnMouseMove` として出るように壊れても、記録には残るのに拾われず緑 | 「送らない」の確かめだけ `!recorded.iter().any(\|c\| c.id.starts_with("OnMouse"))` にする | — | — |
| 44 | `tests/kanade/common/common_harness.rs:46-66・75-105・116-145・152-177・184-214・227-257・267-295`（`spawn_harness*` 7 つ） | 駆動の組み立てがほぼ同じ写しで 7 つ。配線を 1 か所変えると 7 か所を直す要があり、漏れた組み立て方だけ違う振る舞いになる | 私的な `assemble(shiori, stop_sink, sakura_builder)` を 1 本にし、7 つは 1 行で呼ぶ（公開の名前は残す） | — | — |
| 45 | `tests/kanade/common/common_mock_shiori.rs:43-77・151-191・305-372`・`common/common_mock_sakura.rs:136-176・255-356` | 偽の shiori の受信の輪（呼び出しの記録・Unload／Close の扱い）が 3 つ、偽の sakura が 2 つ。Unload の記録の形を変えると 3 か所を揃える要があり、漏れるとその偽物を使うテストだけ期待がずれる | sakura は `spawn_mock_sakura` を `spawn_mock_sakura_gated(.., Vec::new()).0` に。shiori は失敗の注入と塞ぎの注入を受ける 1 本の輪に | — | — |
| 46 | 後始末の手順「Harness を分ける→join→送り端を捨てる→`started`／`commands`→sakura を join→記録を読む」が約 30 か所: `mouse_test_drag_tests.rs:52-67`・`mouse_test_event_layout_tests.rs:58-75`・`mouse_test_phase_guard_tests.rs:42-53・113-125・196-208`・`mouse_test_talk_start_tests.rs:78-91・204-216・360-372・495-508`・`steady_test.rs:120-139・427-442・579-593・850-865`・`external_status_test.rs:54`・`status_query_test.rs:91`・`choice_test_*`・`close_test_*`・`boot_test`・`full_run_test`・`failure_test`（5 か所）。`Driven` の型も `steady_test.rs:77` と `mouse_test_event_layout_tests.rs:10` に 2 つ | 手順を変えるとき（たとえば ForceQuit で終わる檻で `commands()` を使うと 100 回中 7〜11 回赤になる＝`common_mock_sakura.rs:39-47`）に直し漏れが残り、毎回読み手が「`commands()` でよいか」を判断する要がある | `common_harness.rs` に `Harness::finish(label)` を 1 つ置き（中ではいつも `join_bounded_then_commands`）、各所はそれを呼ぶだけに。`Driven` も 1 つに | — | — |
| 47 | `choice_test_test_support.rs:106-111`・`status_query_test.rs:64-69`・`external_status_test.rs:33-38`（`pumps` 3 つ）・`close_test_boot_greeting_tests.rs:42-51` と `steady_test.rs:156-165`（`resumed_get_after_*`）・`status_query_test.rs:26-37・119-124` と `external_status_test.rs:26-51`（`send`／`tick`／`binding`）・`translate_test.rs:103-122`（`record`＝`common_recording.rs:50` の `RecordedCall::from_call` の写し） | 小さな道具の写し。判定（たとえば pump の ID）を直すとき 1 か所漏れると檻ごとに判定がずれる。`RecordedCall` に欄が増えると `record` だけ古いまま | `common/common_recording.rs` に 1 つずつ置いて写しを消す。`record(&call)` は `expected_call(call.clone())` に | — | — |
| 48 | `tests/kanade/choice_test_timeout_directive_tests.rs:19-20`（`DISPLAY_END_MS`）・`:33-64`（`run_choice_wait` の前半） | 共有の `establish_choice_wait` と違うのは指令 1 つだけで、Boot→Tick(1_000)→ChoiceWaiting の注入の列と定数（`CHOICE_DISPLAY_END_MS` の写し）を自前で書き直している。片方だけ起点を変えると 2 本の檻が別々の期限を前提にする | `establish_choice_wait` に `directive: Option<f64>` を足し（ほかの呼び手は `None`）、写しを消す | — | — |
| 49 | `tests/kanade/steady_test.rs:334-349・457-473・613-629・645-660`・`:612-640`（輪） | 期待値 `RecordedCall { .. references: match events::on_second_change(..) .. }` を 4 回手で組む（共通の `expected_call(..)` と同じ値）。`:457-473` は Status `"talking"` を手で書き込み、ファイル自身の「ハードコードしない」の決まりに反する。`:612-640` の輪は記録を一度も見ず、直後の `:643-668` と `events_tests.rs` の `on_second_change_*` が同じことを見る | 4 か所を `expected_call(..)` にし、輪（29 行）を消す（924→約 840 行） | — | — |
| 50 | `tests/kanade/boot_test.rs:28-128` と `full_run_test.rs:96-118` | 期待する起動の列 6 件の組み立てが 2 つのファイルにそのまま写され、Fixture・QuitPolicy・先頭 6 件まで同じ。起動の列を変えるたびに両方を直す | boot_test を消して `tests/kanade.rs` の `mod boot_test;` も消す。残すなら期待の列を common の 1 関数に | — | ⑴ 冒頭が「起動系列そのものへ焦点を絞った適合検証（Req 1.5）」と要件 1.5 の専用テストとして名指す |
| 51 | `tests/kanade/mouse_test_talk_start_tests.rs:319-424`（cage 9） | 送る順も確かめ（NOTIFY が出る・steady の台詞は 1 本）も `steady_test.rs:381` の `active_talk_tick_emits_notify_ref3_zero` と同じ（doc も「同一イディオム」）。足しているのは「マウスの GET が無い」だけで、マウスを入れていないので落ちようがない | cage 9 を消し、steady_test 側の名前を cage 8 の説明に書く | — | ⑴ ファイル先頭「両者は同一テスト群に対で配置する（設計 Integration #3）」 |
| 52 | `tests/kanade/mouse_test_talk_start_tests.rs:456-557`（cage 10） | 駆動は cage 8（`:166`）と同じで、確かめる中身（2 本・番号 1 と 2・台詞・最後が Unload）は cage 8 が全部見る。cage 8 が緑なら必ず緑 | cage 10 を消す | — | ⑴ 説明に「設計 Integration Tests #1〜#4」の #4 |
| 53 | `tests/kanade/mouse_test_event_layout_tests.rs:304-356`（cage 4 `no_content_produces_no_start_talk`） | 204 を明示して入れるが既定と同じ値。駆動は cage 1（`:90`）と座標が違うだけで、足しているのは「最後が Unload」だけ | 「最後が Unload」を cage 1 へ移し cage 4 を消す | — | ⑴「設計 #4 の意図を明示する」（`:307`・Req8.1(d)） |

### 3.3 Nice to have（43 件）

| # | 場所 | 中身 | 直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 54 | `src/actor.rs:771-779`（`round_trip` の `ReplyError::Timeout` の腕） | `recv()` は `Dropped` しか返さない（`areka-actor/src/reply.rs:55-57`）＝「期限内に届かなかった」の error は決して出ない | `Err(_)` を `Dropped` の腕にまとめる | — | — |
| 55 | `src/actor.rs:783-786`（`send_shiori`） | 呼び手 1 つの素通しで、返す未達のメッセージは呼び手が捨てる | `shiori.send(msg).is_err()` に置き換える | — | — |
| 56 | `src/actor.rs:672` と `:676` | `Sstp` と `Property` の腕が同じ検査 | `EventId::Sstp(name) \| EventId::Property(name) =>` にまとめる | — | — |
| 57 | `src/actor_stop.rs:33-37`（`handoff_of`）・呼び手 `actor.rs:326・366` | 切替の相の間、刻み（毎秒）・TalkDone・入れ直しのたびに `OnGhostChanging` の台本を複製する。使うのは `Unloading` のときだけ | `Phase::Unloading` でなければ `None` を返す 1 行 | — | — |
| 58 | `src/actor_sstp.rs:28-29・92`（`script.clone()`・`answer` の `s.clone()`）・呼び手 `actor.rs:255-259・346-350` | 最初の往復が Value なら、入力の種類を問わず（毎秒の GET・マウスも）台本を丸ごと複製し、SstpNotify ではもう一度複製する | `answer` が値で受け取り、`:92` の複製を消す | — | — |
| 59 | `src/schedule/balloon_events.rs:282・306-319`（`send`／`shown_script`） | 受け手は `&str` しか取らないのに台本を `String` に複製して返す＝3 つのバルーンのイベントごとに大きい台本の写しが 1 つ余計 | `&str` を返す（無ければ `""`） | — | — |
| 60 | `src/schedule/script_log.rs:48-52`（`record`） | 再生の開始を含まない一歩（毎秒の刻み・マウスの移動）でも表示の語の `String` を 1 つ確保する | 空でない `StartTalk` を見つけたときだけ作る | — | — |
| 61 | `src/lib.rs:84-91・100-103`（`pub mod events`・`pub mod resources` の再輸出） | `allowed_static`・`is_allowed_choice_event`・`on_balloon_break`・`on_balloon_close`・`on_balloon_timeout`・`on_ghost_changed`・`value_replaces_active_talk`・`resource_get` の 8 つは `crates/areka`・`crates/areka-ghost`・`tests/` のどこからも使われない（名前で grep して 0 件）。doc は「`tests/` が使うものだけを出す」と言い実態と合わない | 8 つを `pub use` の列から消す（外の使い手が無いので要件 6.3 に触れない） | — | — |
| 62 | `src/schedule/resources.rs:97-99`（`resource_username`） | 中身は `resource_get("username", ..)` を返すだけの素通し | 消すなら呼び手で `resource_get` を書く | `crates/areka-ghost/tests/ghost/spine_e2e_test.rs:168` が使う公開の口 **【6.3】** | ⑴ 要件 6.3（`areka-ghost` が使う公開の口は消せない）＝直さずに残す候補 |
| 63 | `src/status.rs:29-36`（`Minimizing`・`Induction`・`Passive`・`TimeCritical`・`Opening`）・`:73-114`（`OpeningKinds`・`OpeningKind`） | 本番は一度も作らない（テストの外の使い手 0 件）。しかも `:75` は「空集合は Opening 自体を非アクティブとする」と書くのに `OpeningKinds::new(vec![])` は `opening()` を出す＝将来この欄に源をつないだ人が踏む | 残すなら空の Opening を出さないように `render` を直す（または消す） | — | ⑴⑵ `status.rs:23`「残 5 状態は語彙に留め…（status-execution-states Req6.1/6.2）」・`:167`「行の存在自体が語彙保持の表明である（Req2.5）」 |
| 64 | `src/schedule/steady.rs:825-828`（`on_reply` の `Steady{Some}`＋`Notified` の腕）・doc `:744-757`・`mod.rs:564-567` | 定常の `Notified` は `mod.rs:564` が先に受けて返し、例外の道（`:546`）でも先の腕が受けるので、この腕には届かない。`Steady{None}`＋`Notified` は steady 側では warn・mod.rs では「正常な完了」と言い分が違う。doc の「mod.rs は非 Failed 応答のみを委譲」も古い（`:546` は Failed も渡す） | 届かない腕を消して `other => steady_reply_unexpected` に任せ、doc を今の道に合わせる | — | — |
| 65 | `src/schedule/steady.rs:857・865-867`・`events.rs:300-305`（`value_replaces_active_talk`） | 述語に「捨てる名前」を 1 つ足すと、その答えは最後の `_` の腕に落ち、「active talk 中に OnSecondChange の Value」という嘘の文言で warn になる | 最後の腕の文言を「置き換えない出どころの Value」に | — | — |
| 66 | `src/schedule/steady.rs:44`（`_config`）・`:54`（`let _ = &other;`）・`:459・678・953`・`src/schedule/close.rs:89-93・101-105・169-173`（`other => { let _ = other; .. }`） | 使わない引数・何もしない `let _`・束ねてすぐ捨てる腕（`_ =>` と同じ）。想定外の答えの warn に「Notified か Unloaded か」が載らない | `:54` を消し、`let _ = other` の腕は `_ =>` に（warn に答えの種類を 1 語足す）。`_config` は呼び手 5 か所と一緒に外すか残す | — | — |
| 67 | `src/shiori/probe.rs:252`（`ShioriBusy::Request(id) if id == "OnClose"`） | 記録の段の名 `on_close_notify` は後始末の OnClose 通知の意味なのに id しか見ないので、後始末より前から待っていた OnClose の GET（利用者が閉じ、SHIORI が固まり、その後に OS の終了が来た）も同じ名になる。振る舞いは変わらず記録だけ取り違える | `ShioriBusy::Request` に GET か NOTIFY かを持たせ、NOTIFY の OnClose だけを `on_close_notify` に | 公開の enum の形が変わるが、外の使い手は 0 件（grep） | ⑴ 完了 `areka-P0-session-mark-residue` の design.md:214「`Request(id)` で `id == "OnClose"` → `on_close_notify`」 |
| 68 | `src/schedule/translate_path_tests.rs:118-125`（`step_untranslated`）・元の `mod.rs:141-158` | `step` から翻訳の入口と出口を除いた手書きの写し。既に `talk_origin::stamp` と `script_log::record` が抜けていて、⑶「翻訳なしと同じ」は古い `step` と比べている | 比べる相手を `Translated::local(ORIGINAL)` で回した `step` どうしの比較に替えて写しを消す（残すなら `step` 側に「この写しも直す」の 1 行） | — | ⑴ ファイル冒頭の doc が写しを持つ理由を書く |
| 69 | `src/schedule/talk_gap_tests.rs:509-529・681-702` | 頼んでいない答え（`OnMouseDoubleClick` の Value）を直に入れるので、毎回 `translate_source_missing` の error を通る＝本番では起きない道。この守りを故障へ倒すと関係ない理由で赤になる | `translate_path_tests.rs` の `steady_replace` と同じく、先に `Input::Mouse` を入れて GET を送ってから答える | — | — |
| 70 | `src/schedule/balloon_events_cleanup_tests.rs:292-302` | 「期限が未設定ならもう 1 度刻む」の逃げ道は `steady_none()` が `last_now = Some(1000)` で始まるので一度も走らない。入口で期限が決まらなくなる退行を黙って吸う | 枝を消し、刻み 1 回で `DeadlineExceeded` を見る | — | — |
| 71 | `src/schedule/user_break_tests.rs:21-48`（`not_playing`・`playing`） | 中身が一字一句同じ 2 つの関数 | 1 つ（`with_phase`）に | — | — |
| 72 | `src/schedule/mouse_events.rs:130-137`（`mouse_extended_unexpected_kind`） | ほかの守りの腕の warn は `schedule_log_firing_tests.rs` が見るのに、この腕だけテストが無い | `on_mouse_extended` に `MouseEventKind::Move` を直に渡し、行いが空で WARN が出ることを 1 本 | — | — |
| 73 | `src/schedule/steady.rs:448-461`（`on_cascade_reply`）・`:667-680`（`on_timeout_reply`） | Notified／Unloaded を「204 と同じ」に倒す枝のテストが無い（同じ種類の枝は `mouse_events_tests.rs:444`・`translate_tests.rs:132` に在る）。壊れると選択待ちのトークが止まったまま進まない | 2 つの答えを輪で流し、残段なしなら `ResolveChoice`・時間切れなら `CancelChoice` が 1 つずつ出ることを 1 本 | — | — |
| 74 | `src/schedule/steady_choice_tests.rs:344-370・373-400`・`steady_choice_timeout_tests.rs:342-370` と `steady_choice_timeout_tests.rs:670-705・708-738・741-763` | `steady::step` を直に呼ぶテストが `step()` 経由のテストと 3 組重なる（直に呼ぶ方だけが捕まえる故障は無い） | 直に呼ぶ 3 本を消す | — | ⑴ doc に「層を分けた単体檻」と意図が書かれる |
| 75 | `src/schedule/steady_choice_timeout_tests.rs:25-43`（`steady_waiting`）と `steady_test_support.rs:67-81`（`steady_with_ledger`） | 違いは台本と期限を引数で渡すかだけの写し | `steady_with_ledger` で作り、台本と期限だけ上書きする | — | — |
| 76 | `src/schedule/schedule_tests.rs:563-589` | `:536-561` と同じ入力で、確かめる事柄も部分集合 | 消す | — | — |
| 77 | `src/actor_tests.rs:578-599・722-769・113-133` | 7(a) の部分集合・10(a) と 7(b) の重ね・`tests/kanade/failure_test.rs:431` と重なる（983 行のファイル） | 消す | — | — |
| 78 | `src/actor_tests.rs:8-20`（`run_bounded`） | 中の `join().expect` が panic すると完了の合図が来ず、5 秒待って「possible hang」と出る＝原因を取り違える（`areka-ghost` の写しは直してある） | `Disconnected` を「panic した」と分ける | — | — |
| 79 | `src/actor_sstp_tests.rs:9-69`（3 本）・`src/actor_stop_notify_tests.rs:225-236`・`src/msg_tests.rs:385-426` と `src/change_tests.rs:16-21` | 230・251・305 行目のテストに全部含まれる 3 本・`:181-198` と別の値で同じことを見る 1 本・`KanadeConfig::new` の既定値のテストが 2 つのファイルに分かれている | 重なる 4 本を消し、既定値のテストを 1 つに | — | ⑴ choice_timeout の 30,000 は「裁定 5」（値は残しまとめるだけ） |
| 80 | `src/talk.rs` の `mod tests` | 中のテストは `areka-talk` の生成関数を確かめ直すだけ（モジュール自体は crate の中の 48 ファイルが使うので残す） | テストを消す | — | — |
| 81 | `src/schedule/events_change_tests.rs:33-47`（`assert_eq!(ALLOWED_EVENT_IDS.len(), 66)` と増えた経緯の注）・`events_tests.rs:189-217`（doc「期待 23 集合と完全一致」） | 完全一致の比べ（`events_tests.rs:219-290`）が数を既に縛るので、語を 1 つ足すたびに 2 つのテストと 2 つの注を直す。doc の「23」は今の 66 と合わない | 長さの assert と数え上げの注を消し、doc の件数を消す | — | — |
| 82 | `src/schedule/boot_sequence_tests.rs:355-363`（`close_request_during_boot_records_pending_only`） | 後から足した `BootPrefetch` と `BootVersion{talk: Some}` が回す相の一覧に無い | 一覧に 2 つ足す | — | — |
| 83 | `src/shiori/real_tests.rs:121`（`boxed()`） | `Box::new` を渡すだけの包み（`real_idle_tests.rs` は直に書く） | 消す | — | — |
| 84 | `tests/kanade/common/common_fixture.rs:23-51・86-108・241-269` | `MouseResponse` と `ChoiceResponse` が同じ形。マウスの表は 2 つの ID しか引かず、`with_mouse_response("OnMouseClick", ..)` と書くと黙って 204。`ChoiceResponse::NoContent` と `boot_script`／`steady_script`／`farewell_script` の欄は外で一度も使われない | 表を 1 本の `HashMap<&str, Response>` に、enum を 1 つに。台詞の 3 欄は消して定数を使う | — | — |
| 85 | `tests/kanade/mouse_test_test_support.rs:4`（`move_input`）・`mouse_test_drag_tests.rs:18`（`drag_input`）・`mouse_test_event_layout_tests.rs:16`（`dbl_input`）・`translate_test.rs:261・278` | マウスの入力を組む関数が `kind` だけ違う形で 5 つ | test_support に `mouse_input(kind, ..)` を 1 つ | — | — |
| 86 | `tests/kanade/full_run_test.rs:156・185・187-192`・`idle_pump_test.rs:132-135・154`・`steady_test.rs:199-207・211-216・314-321・767-773・880-886`・`mouse_test_talk_start_tests.rs:278-284・411-416・536-549` | 前の assert が通れば必ず通る assert（直前の `>` の後の `assert_ne!`・末尾が Unload と分かった後の確かめ・番号を確かめた後の HashSet・マウスを入れていないのに「マウスの GET が無い」ほか） | 消す | — | — |
| 87 | `tests/kanade/steady_test.rs:252`（`sporadic_value_starts_unique_talk`）と `:703`（`spontaneous_talk_egress_sweep_only_allowed_ids_no_ontalk_onhour`） | fixture・刻み 8 回・終わりの旗まで同じ `drive_steady` を 2 回回す | 後のテストの確かめを前のテストの末尾へ移し 1 本に | — | ⑴「いずれも additive・既存 4 テストは無改変」「Testing Strategy #13」（`:50-51`） |
| 88 | `tests/kanade/mouse_test_drag_tests.rs:218・255・297`（K5・K5'・K6） | 定常でないときに捨てる腕（`src/schedule/mod.rs:190-192`）も終了の保留の守り（`steady.rs:82`）も入力の種類を見る前に効くので、cage 5a・5b・6 と種類だけ替えた同じテスト | 消すか、cage 5a・5b・6 の入力にドラッグを 1 つ足して 1 本に | — | ⑴ 完了 `areka-P0-mouse-drag-events` の要件 5.2・5.3・9.1 ⑹（`:208・288`） |
| 89 | `tests/kanade/close_test_handshake_tests.rs:194-209`（`silent_close_on_204_terminates_without_extra_events`） | 「204 で即終了するので記録しそびれる競り」の注は今は成り立たず（close は挨拶の TalkDone を待ち、偽物は返す前に記録する）、10,000 回の `yield_now` の輪は一度も回らない | 輪と注を消し、`:69` と同じ 1 行に | — | — |
| 90 | `tests/kanade/choice_test_test_support.rs:27-32`・`choice_test_timeout_tests.rs:37-42・76-82` | 「既定の時間切れ 30 秒を値そのものとして固定する」と言うが、期限手前の刻みが `CHOICE_DEADLINE_MS - 1_000` なので 29,001〜30,000 のどれでも緑（値は `src/msg_tests.rs:402` が別に縛るので隠れる退行は無い） | 期限手前の刻みを `CHOICE_DEADLINE_MS - 1` に（時間の指令の檻と同じ形） | — | — |
| 91 | `tests/kanade/close_test_handshake_tests.rs:453-464・481-497`（`force_quit_*`） | doc は「NOTIFY は Unload の直前」と言うが `notify_index < unload_index` しか見ない（すぐ前後は `src/actor_tests.rs:251` が縛る）。`force_quit_onclose_notify` は 1 か所でしか使わない包み | `assert_eq!(notify_index + 1, unload_index)` にし、包みを中に書く | — | — |
| 92 | `src/schedule/script_log.rs:16-17`（`TARGET`）と `crates/areka/src/log_history.rs:82`（`TARGET_SCRIPT`）・一致の確かめ `crates/areka/src/mcp/sakurascript_log_tests.rs:26` | 同じ target の文字列が 2 つあり、テストで一致を縛っている（areka は kanade に依るので 1 つで足りる） | `log_history.rs` で `areka_kanade::script_log::TARGET` を使い、一致のテストを消す | `crates/areka/src/log_history.rs`・`sakurascript_log_tests.rs` **【6.1】** | ⑴ `script_log.rs:16`「areka のテストで一致を固定」＝分けた形を書いた決め。要件 6.1 で今は直せない |
| 93 | `src/schedule/events.rs:92-96`（`REF2_WHEEL_M1` の doc）・`:17-74`（冒頭の「単一正本」の表） | doc は「M1 はホイールイベントを送出しない」と言うが今は `OnMouseWheel` を実際の量つきで送る（`mouse_events.rs:109-118`）。表は `OnFirstBoot` を「Ref0=`"0"`（固定）」のまま（実際は `vanish_count`・`:340-346`）で、`OnDestroy`・マウスの 9 語・`OnFileDrop2`・`OnDirectoryDrop`・SSTP の `OnTranslate` の行が無い | doc を「移動の Ref2 は常に "0"（ホイールの量は `OnMouseWheel` が運ぶ）」に。表の行を直し、足りない行を足す（または「全部は載せない」と書く） | — | — |
| 94 | 本番のコードの古い注: `src/msg.rs:8-15・58・116・120・215`・`src/actor_stop.rs:41`・`src/actor_resources.rs:30-31`・`src/schedule/boot.rs:3-6`・`src/schedule/resources.rs:6-7・32・36・38・104-106・205`・`src/schedule/mod.rs:39・391-397`・`src/schedule/types.rs:34・297-303`・`src/schedule/balloon_events.rs:254-256`・`Cargo.toml:20-21` | 依存の一覧（「std・crate::talk・ReplySender のみ」）・停止通知を写す点・マウスの種類（「移動／ダブルクリック」・今は 7 種）・「既存の呼び手はすべて sink が None」（本番 `crates/areka/src/emo2_boot/spine.rs:786` は `Some`）・照会できない相の列挙に切替の 4 相が無い・「タスク 2.3 が本体を実装する」・送出の検査の置き場（今は `round_trip_raw`）・「α」の語・「枠 9 種」と「7 種」・壊れた doc リンク `[crate::lib]`・`[crate::schedule::mod]`・「到達点は 3 つ」（今は 4）・`snapshot_with_choice` の引数（呼び手 2 つとも `true` を直に渡す）・「3 つの入口に共通する門」（使うのは 2 つ）・`shiori-host32-ipc` は「real.rs のテストのためだけ」（結合テスト 2 本も使う） | 文を今の事実に書き換える | — | — |
| 95 | `src` のテストの古い注: `steady_flow_tests.rs:173・293-295・321-325・413-416`・`schedule_log_firing_tests.rs:404-407・649-650`・`schedule_tests.rs:82`・`boot_sequence_tests.rs:584-614` | 「close::step は現状 stub」・「origin の match は wildcard にしない」（`steady.rs:865` は `_`）・「マウス／非マウス」の説明・空の節見出し・「M1 では到達しない想定」（DD-11 で正規の到達点）・テストの手順の注が実際の起動の段と違う | 今の振る舞いに合わせて書き直すか消す | — | — |
| 96 | 結合テストの古い注と探し方の写し: `tests/kanade.rs:3-5`・`common/mod.rs:19-24・32`・`common/common_bounded.rs:66-67`・`boot_test.rs:7・70-71・114・121`・`failure_test.rs:4`・`idle_pump_test.rs:43`・`mouse_test.rs:1-19`・`steady_test.rs:4・6-10・38・61・99・397・784`・`mouse_test_talk_start_tests.rs:39・155-156・305`・`choice_test.rs:11・39・58-59`・`choice_test_rejection_tests.rs:27`・`close_test.rs:4-32`・`close_test_handshake_tests.rs:401-402・453-454`・`prefetch_test.rs:19・22-33`・`real_helper_test.rs:19・48-81`（`resolve_helper_exe`） | 「空のプレースホルダ」「以後編集不要」・「兄弟 9 本」（今 14）・「yield_now 1 回で足る」（コードは sleep）・「先頭 4 件」（6 件）・「4 系」（6 系）・存在しない Test-B・「6 檻」（cage 1〜10 と K3〜K7）・「2 パターン」と 3 つ・挨拶の台詞の記述（fixture は挨拶なし）・テストの置き場の名（`steady::tests` は今 `steady::flow_tests`）・`TalkCommand::Cancel`（今 `CancelChoice`）・`--test-threads=1`（強いる設定は無い）ほか。`resolve_helper_exe` は同じ探し方の 7 つ目の写し | 数と名前を今に合わせ、プレースホルダの段落を消す。探し方の写しは寄せるなら共通の試験の道具へ（別作業でよい） | まとめるなら `crates/areka-ghost/tests/ghost/common.rs`（持ち主 `audit-ghost-catalog`・着地済み）・`crates/shiori-host32-host/tests/*.rs` 5 本（`audit-shiori-host32`・C18 には居ない） | — |

## 4. 指摘にしなかったもの（確かめた結果）

- **同期の往復**: 殻（`actor.rs` の `drive_translating`／`execute_batch`）は SHIORI との往復を同期で回し、答えをすぐ入れ直す（`actor.rs:7-17`）。そのため「GET を出してから答えが戻るまでに外の入力が割り込む」競りは段取りの内側では起きない。選択の段の帳簿・アンカーの段・SSTP の答え待ちが混ざる道も見つからなかった。
- **本番のコードの `unwrap`・`expect`・`panic!`** は 0 件（`log_capture.rs:193` の `panic!` はテスト専用の表明）。`let _ = reply.send(..)`・`let _ = on_down.send(..)`・`actor.rs:540` の `let _ = shiori.send(Close)` は、相手がもう居ないときだけ失敗し、直前に error が出ているか返事の約束として doc に書かれている＝黙る失敗ではない。
- **一時フォルダ**: 範囲のテストに `std::env::temp_dir()`・`%TEMP%`・`C:\` 直下を使う所は無い（grep 0 件）。`C:\ghost\next` などはパスの値として渡すだけ。本物の sleep は `common_bounded.rs` の 200 µs（理由つき）・`idle_pump_test.rs` の「応答なし」の判定（OS の本物の時間に依るのでテストの本質）だけ。
- **終了の道**: 別れの台詞は終わり方（Ended・Interrupted・Quit・中断＋予約・期限切れ・強制終了）を問わず終了へ進み、テストもある（開発者の裁定「終了イベントのトークは必ず終了で終わる」）。
- **選択肢つきの別れの台詞で 30 秒待って終わる**・**ホイールと出入りの送信の失敗だけ終了へ倒れる**・**初回の挨拶を中断すると起動の記録が書かれない**・**`ChangeTalkWait` の終了要求が期限なしの定常の台詞になる**: どれも完了した spec の裁定・決め（`areka-P0-balloon-break` 裁定 6・`mouse-click-wheel-events` 決定 7・`areka-P0-position-persist` design §220・要件 2.9）のとおり。
- **`RaiseEvent` の Reference の改行**: 呼び手はアプリの中だけで、外から来る更新のファイル名は `:` を断るので頭書きに差し込めない（#1 の SEND の道とは別）。
- **SHIORI の答え由来の台本の改行**: 答えは LF で行を切るので、答え由来の台本に LF は入らない（危ないのは MCP の SEND 由来＝#1）。
- **`OnSecondChange` の Ref0**: 本番の時刻は `GetTickCount64` で正典どおり（OS の起動からの時間）。
- **重さ**: 台本の写しはトーク 1 本分だけで溜まり続ける入れ物は無い。許可表の線形の探し（66 語）は往復 1 回に 1 回。
- **`probe_tests` の `checked_sub(120s)`**: 起動から 2 分以内の機械では panic するが、実害は小さいので指摘にしない（指摘にするなら Nice to have）。
- **範囲の外で見つけたもの（その範囲の持ち物として残す・要件 6.7）**: `crates/areka/src/emo2_boot/spine_conformance_script.rs:250-277` の注が `events.rs` の古い行番号（`:48`・`:53`・`:172-174`）を指している（今は 96・101・465 行など）。持ち主は `audit-emo2-boot-spine`（`crates/areka/` は C18 で `placement-crate-split` が書き換える）。

### わざと置いた先送りの印（⑵ の候補・今も限界の内側）

- `src/schedule/events.rs:86・89` の `SEAM(Req1.6)`（`REF1_OFFSCREEN_M1`・`REF2_OVERLAP_M1` を "0" 固定）——守られている。
- `src/schedule/events.rs:97-101` の `REF6_DEVICE_MOUSE`（α 後のシーム）——今もマウスだけで守られている。
- `src/schedule/events.rs:894-895` の OnBalloonBreak の Ref2 を空（追跡先 `balloon-break-position`）——その spec のフォルダと roadmap の行が在り有効。
- `src/schedule/steady.rs:110` の `SEAM(Req7.3)`（パッシブモードのドラッグの抑え）・`src/status.rs:179-191・245-255` の `SEAM`／`NOTE`（源の無い 5 状態と offscreen／overlapping）——どれも今も非アクティブで守られている。
- `src/schedule/resources.rs:38` の `SEAM(α 後・159 項目汎用化)`——語彙は足す方式で今 15 個、守られている（「α」の語だけ古い＝#94）。
- `src/actor.rs:310`・`src/schedule/events.rs:641` の `#[allow(clippy::too_many_arguments)]`（引数 8 個）——理由は今も成り立つ。
- `shortcut:`・`ponytail:` の印は範囲に 1 つも無い。
- `src/lib.rs:41` の `#[allow(dead_code)]` と `tests/kanade/common/mod.rs:25・77` の許しは限界が切れている＝#16・#17 で指摘にした。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1（洗い出し） | 本書 §1・§3 | 済み（全 146 ファイル・96 件） |
| 2（討議で確定） | 本書の「残す候補」の欄・§8 | 討議で 3 種類の裁定を仰ぐ（**Constraint**） |
| 3（全部直す・呼び手まで） | 「範囲外」の欄 | 範囲の外へ及ぶ直しが 7 件（#1・#3〔振る舞いだけ〕・#6・#7・#62・#92・#96〔任意〕）。うち【6.x】に当たる #1（入口で断る形）・#6・#62・#92 は今の形では直せない＝**Constraint** |
| 4（テストで縛る） | 既存の檻の形（`log_capture_kit::capture`・`steady_test_support`・`tests/kanade/common`） | Must fix と Bug の各件に落ちるテストが要る（#1〜#8・#37〜#43 など）＝**Missing** |
| 5（決まり） | 1,000 行の番人・一時フォルダの番人・`with_default` の番人 | 1,000 行の近いファイルが 10 本（§6）＝**Constraint**。依存は増えない（どの直しも今の依存の中） |
| 6（範囲の外） | — | `crates/areka/` に及ぶ #1・#6・#92 と、公開の形に及ぶ #1・#6・#62 は議題（§8）。C18 で並走中の spec の持ち物に及ぶ直しは無い（`areka-talk`・`areka-actor` にも及ばない） |
| 7（clippy） | §1 の実測 | 申し送りの 6 件に加え、結合テストの隠れた 11 件（#18）＝**Missing**。数え違い（`type_complexity` は 4 件）＝要件 7.1 の文言の議題 |
| 8（かけ直し 0 件） | — | 最後のタスクで同じ作法でかけ直す |

## 6. 1,000 行の番人と上限の近いファイル

- `src/actor_tests.rs`（983 行）: #19・#77 で減る。#1 のテストはここへ足さず新しい兄弟のファイル（例 `actor_header_guard_tests.rs`）へ。
- `src/schedule/steady.rs`（973 行）: #11（約 −40）・#64・#66 で減る。#7・#8 の直しは数行。
- `src/schedule/events.rs`（943 行）: #13 で約 −30。#1 の検査は `actor.rs` に置けば events.rs は増えない。#93 の表を足す直しは行が増えるので、#13 と組みにする。
- `src/schedule/steady_flow_tests.rs`（931 行）: #29 で減る。足すテストは無い。
- `tests/kanade/steady_test.rs`（924 行）: #49 で約 −80、#46 でさらに約 −40。
- `src/schedule/schedule_tests.rs`（903 行）: #26・#76 で減る。#30 のテストは `schedule_log_firing_tests.rs`（720 行）へ。
- `tests/kanade/failure_test.rs`（863 行）: #38・#39 は数行。**このファイルは `crates/log-capture-kit/tests/with_default_guard_test.rs:145` の例外表に名指しされている**（全体の記録の捕まえ口を使うファイルの一覧）。ケースを兄弟のファイルへ分けて捕まえ口を移すと例外表を直す要が出る＝その表は `log-capture-kit` のテスト（`audit-test-kits` の持ち物・要件 6.2 の「`placement-crate-split` が触る `log-capture-kit` のテスト 2 つ」に当たるかは設計で確かめる）＝分けない形を推す。
- `src/shiori/real_tests.rs`（740 行）: #32 で約 +60（800 行前後）＝上限の内。

## 7. 実装の進め方の案

- **案 A（今のファイルを直す・推す）**: 指摘ごとに今のファイルの中で直し、テストの道具は今ある置き場（`src` は `steady_test_support.rs`・`boot_test_support.rs`・`translate_test_support.rs`、結合テストは `tests/kanade/common/`）へ寄せる。新しい本番のファイルは作らない（#1 のテストの兄弟のファイルだけ足す）。
  - 良い: 今の層と決まりのまま・差分が小さい。悪い: 1,000 行の近いファイルが多いので、減らす直し（#11・#13・#29・#49）を足す直しより先に並べる要がある。
- **案 B（テストの道具を先に寄せる）**: #44〜#47 の共有の道具（`Harness::finish`・`assemble`・偽物の輪・小さな道具）を先に作ってから、ほかのテストの直しに入る。
  - 良い: 後のテストの直しが共有の道具の上で短く済む。悪い: 結合テストのほぼ全ファイルに一度に触る。
- **案 C（組ごと）**: Must fix を先に 1 段で直して全体テスト、次に Should fix、最後に Nice to have。
  - 良い: 危ない所（#1・#2）が早く直る。悪い: 同じファイルに 3 回触る（`steady.rs`・`events.rs`・`steady_test.rs` は 3 つの組に跨る）。
- 規模の見立て: **L**（1〜2 週間）。理由: 96 件・1 クレート（コードが一番多い範囲）・範囲の外の議題 4 件。危うさ: **中**（#1 の断り方と返事の語、#7 の時刻の起こし方、#33 の周期の差し替えは新しい形を引く。ほかは既にある形の中）。
- 減る行の見立て: 落ちないテスト・重なったテスト・道具の写しを消して、テストの側で約 −1,200〜−1,600 行（#22 約 330・#23 約 120・#25 132・#36 約 200・#44〜#46 約 300・#49 約 80 ほか）。本番の側は約 −100 行（#11・#13・#14・#64）。依存は 1 つも増えない。

## 8. 討議に渡す議題の種（要件 2）

1. **直さずに残す候補（⑴ 開発者の裁定・完了した spec の決め）**: #4（C4 規則 9 の控えの消去点）・#5（最初の往復の結果だけを写す）・#6（要件 2.5 の受理の規則）・#7（選択の時間切れを毎秒の刻みで判定する完了した設計）・#8（trace で観測する doc の決め）・#14（本文検査の件数）・#25（Req4.4）・#28（Req2.5 の証跡）・#50（要件 1.5 の専用テスト）・#51〜#53・#87・#88（設計の Integration テストの対・完了 spec の要件）・#62（要件 6.3）・#63（語彙を残す裁定）・#67（design.md:214）・#68（写しを持つ理由）・#74（層を分けた単体檻）・#79（裁定 5 の値）・#92（areka のテストで一致を固定）。
2. **直さずに残す候補（⑵ 先送りの印）**: #63（5 状態の語彙）。#16・#17 は印の限界が切れているので残せない。
3. **直さずに残す候補（⑶ SSP／ukadoc）**: #1（SSP の MCP `sakurascript` が生の改行をどう扱うか）・#3（終了の保留中に来た通知の依頼を SSP が送るか）・#5（空白だけの台本を SSP が再生するか）。どれも ukadoc では引けていない（**Research Needed**）。
4. **範囲の外・公開の口に及ぶ直し（要件 6.5＝裁定を仰ぐ）**:
   - #1: 入口で断るなら `crates/areka/src/mcp/sakurascript.rs`【6.1】。kanade の中（`on_send`）で断るとき、返事の語 `SstpSendOutcome` に「台本を断った」を足すと `crates/areka` の `match` が要る【6.3】。案: (a) 今ある `NotAccepted` で返す（形は変わらない・意味が少しずれる）／(b) 送る点だけで断る（翻訳の失敗＝ゴーストが降ろされる）／(c) `crates/areka` の作り直しの後に値を足す（追跡の spec が要る）。
   - #6: `KanadeMsg::UserBreak` にトークの番号を足すには `crates/areka/src/input_events/user_break.rs` と公開の形に及ぶ【6.1・6.3】＝今は直せない。案: 追跡の spec へ回すか、⑴ として残す。
   - #62: `resource_username` は `areka-ghost` のテストが使う公開の口【6.3】＝残す。
   - #92: `crates/areka/src/log_history.rs` に及ぶ【6.1】＝今は直せない。
   - #7: `crates/areka-ghost` の刻み（持ち主 `audit-ghost-catalog` は着地済み・C18 に居ない）に及ぶ。要件 6.6 の申し合わせは要らない。
5. **要件 7 の文言**: 申し送りの「`type_complexity` 5 件」は実測で 4 件（＋`collapsible_if` の重なり 1）。さらに結合テストに 11 件が隠れている。要件 7.1 の「警告 0 件で通る」はこの 11 件も含めて直す形で読む（clippy の機械的な直しは `clippy-199-lints` の持ち物だが、要件 7 が `-D warnings` の緑を求めるので本 spec で直す）でよいかを確かめる。
6. **1,000 行の番人**: 上限の近いファイルが 10 本（§6）。減らす直しを先に並べる。`failure_test.rs` は番人の例外表に名指しされるので分けない。
7. **タスクの数と切り分け（要件 2.6）**: 直しをファイルのまとまりで束ねた見立て＝Must fix 2 件で 2 タスク・Should fix 51 件で 16〜18 タスク（本番 16 件で 6〜7・`src` のテスト 18 件で 5〜6・結合テスト 17 件で 5）・Nice to have 43 件で 7〜9 タスク・clippy とかけ直しと全体テストで 2 ＝**約 27〜31 タスク**で 20 を越える。切り分けの案:
   - 案 ①（組で切る・推す）: 1 本目＝Must fix＋Should fix＋要件 7＋かけ直し（約 20〜22）・2 本目（同じ範囲）＝Nice to have（約 8〜10）。1 本目が 20 を少しまたぐなら、結合テストの写しの寄せ（#44〜#47）を 2 本目へ回す。
   - 案 ②（本番とテストで切る）: 1 本目＝本番のコードに触る指摘（#1〜#18・#54〜#67・#93・#94）（約 14〜16）・2 本目＝テストだけの指摘（約 15〜17）。要件 2.6 の「組ごと」からは外れる。
   - どちらでも要件 8（かけ直し 0 件）は 2 本目の終わりで見る形になる＝1 本目のかけ直しは「1 本目の受け持つ指摘が 0 件」で数える決めが要る。

## 9. 調べが要るもの（Research Needed）

- SSP の MCP `sakurascript` が生の CR・LF を含む台本をどう扱うか（#1）。SSP の MCP の答えを測るには開発者の許しが要る。
- 終了の保留中・切替の保留中に来た通知の依頼（`OnResetWindowPos`・ファイルのドロップほか）を SSP が SHIORI へ送るか（#3）。
- 空白だけの台本（`"\r\n"` ほか）を SSP が再生する（今のトークを置き換える）か（#5）。
- `src/lib.rs` の `schedule` モジュールの `#[allow(dead_code)]` を外したとき、`ClosePending.reason` のほかに何が警告になるか（#16）。cargo で見る。
- `KanadeMsg::UserBreak` の行き違いの窓（#6）が表示の側でどれくらいの長さか（表示の入れ替わりの速さ）。測っていない。
- #4（置き換えと完了の行き違い）・#37（K4 の送りの競り）・#38（記録前の写し）・#42（`yield_now` の飢え）・#40（期限の 2 秒）は、コードを読んでの推定で、走らせての再現はしていない。直しのテストで再現の形（直しを外すと赤）を作る。
- `failure_test.rs` を例外表に載せている `with_default_guard_test.rs` が、要件 6.2 の「`placement-crate-split` が触る `log-capture-kit` のテスト 2 つ」に当たるか（§6）。

## 10. 要件の討議の前に測った SSP の振る舞い（2026-11-06）

ukadoc に定めが無い #1・#5 を、討議の前に SSP の MCP で測った（起動中の SSP・ゴースト「えも2DEBUG」・reload も設定の変更もしていない）。

- **#1（生の改行を含む台本）**: MCP の `sakurascript` に、生の改行を 2 つ含む台本（1 行目「改行の前」・2 行目「Reference1: injected」・3 行目「改行の後」＋`\e`）を送った。SSP は断らず `OK` を返して再生した。台本の記録（`get_log` script・#301）には「改行の前Reference1: injected改行の後」と、改行が抜けて 1 行につながった形で残った。strict の誤りの記録は 0 件。＝SSP は生の改行で台本を断らない。見える結果は「改行を取り除いて再生する」と同じ。
- **#5（空白だけの台本）**: 8 秒待つ長い台本を送った直後に、生の改行 1 つだけの台本を送った。直後の `get_status` は空（話していない）で、台本の記録には空の台本が #305 として残った。対照として長い台本だけを送ると `talking,balloon(0=0)` になった。＝SSP は空白だけの台本も 1 本の台本として再生し、話し中のトークを置き換える。
- **#3（終了・切替の保留中に来た通知の依頼）は測っていない**。測るには SSP のゴーストを閉じる・切り替える操作が要り、開発者の SSP の状態を動かすため。
- SHIORI への要求の記録（`ssp_shiori_log.txt`）は、今のゴーストには無かった（在るのは 11-01 で止まった別のゴースト `dot_sakura` のものだけ）。よって `OnTranslate` の Reference0 に改行がどう載ったかは見ていない。

## 11. 設計の決め（2026-11-06・`/kiro-spec-design audit-kanade -y`）

- 発見の種類: **light**（今のクレートの直し・新しい依存も外のサービスも無い）。外の調べ（WebSearch）は要らなかった。読んだ手引き: `kiro-spec-design` の `design-principles`・`design-discovery-light`・`design-synthesis`・`design-review-gate`、設計の雛形、完了 `audit-ghost-catalog` の design.md（形を写した）。
- §9 の調べのうち、コードを読んで決まったもの:
  - **#16**: `cargo rustc -p areka-kanade --lib -- --force-warn dead_code`（記録 `target\audit-kanade\dead-code-forced.txt`）で、`schedule` への許しの下に隠れていたのは `Phase::ClosePending` の欄 `reason`（`types.rs:136`）の 1 件だけ。テストの組み立て（`--profile test`・`dead-code-forced-test.txt`）では 0 件。欄は本番で読まれず、終了の理由は `state.pending_close` が持つ＝欄ごと消す（本番 3 か所・テスト約 20 か所）。
  - **`with_default_guard_test.rs`**: `placement-crate-split` が触る `log-capture-kit` のテスト 2 つは `file_length_guard_test.rs` と `temp_path_guard_test.rs`（同 spec の要件）。`with_default_guard_test.rs` は入らない。`failure_test.rs` は分けないので例外表は変わらない。
  - **#6 の運び道**: 押されたバルーンの台詞のトークの番号を持つのは表示の側の `BalloonVisibilityState.talk_id`（`emo2_boot/balloon_visibility.rs:451`・`BalloonTimeout` と同じ源）だけ。`Emo2Wiring.balloon_visibility`（`emo2_boot/frame/wiring.rs:120`・`pub(in crate::emo2_boot)`）の中にあり、`input_events` から読めない＝読み口を 2 つ（`balloon_visibility.rs`・`frame/wiring.rs`）足す要がある。`frame/wiring.rs` は `crate::placement::zorder_group_ledger` を名指す（36 行目）＝`placement-crate-split` との申し合わせの点。入力の側の旗の線（`NoUserBreakSignal::TalkStarted`）は、入力の段が表示の更新の段より先に回るため、Y の始まりが届いた時点で画面は X のまま＝番号が食い違うので採らない。
- **#1 の二段目**: 送る点（`round_trip_raw`）で、ID に CR・LF・NUL があれば今の `event_id_not_allowed`（`error!`＋`Failed(Internal)`）で断る。Reference は取り除いて `warn!`（`reference_line_break_removed`）を残して送る（同じ点の欠番の印の置き換えと同じ形・SSP の見え方と同じ）。断る形は、翻訳の要求なら `translate_failed`→`Unloading{Fault}`、毎秒の問い合わせなら横断の腕で `Unloading{Fault}` になり、SHIORI の答え由来の値 1 つでゴーストが降りるので採らない。`ShioriFailure` に値を足す形・`Failed(Shiori)` に化けさせる形も採らない（語彙が増える・記録が嘘を言う）。外の値が ID になる選択・アンカーの応答待ちの失敗は、今の規律で 204 と同じに扱われる。
- **#7**: 期限が決まる時点は kanade に分かる＝プッシュ。kanade は `Action::ArmChoiceWake(期限)` を出し、殻が `KanadeConfig.choice_wake`（`Sender<MonotonicMs>`）へ送る。`areka-ghost` の結線が中継（`spawn_relay::<MonotonicMs, TickerMsg>`・今の「中継 2 本で輪を解く」と同じ形）で刻み役の `TickerMsg::WakeKanadeAt` に写し、刻み役は待ちの残りに 1 回だけの起こしを足して、期限を過ぎた時の本物の時刻で `KanadeMsg::ChoiceTimeoutDue { now }` を送る。kanade は今の `fire_choice_timeout_if_due` をその時刻で呼ぶ（pump は出さない）。毎秒の刻みでの判定は残す（起こしの線の無い構成・知らせの遅れを拾う）。採らない: 刻みを細かくする（1 秒の周期が崩れ 50 ms の丸めが残る）・余分な `Tick`（選択が先に済むと OnSecondChange が余分に出る）・kanade に時計を持たせる（別の時計を混ぜて観測より先の時刻を作りうる）・dispatcher に計らせる（期限の写しは kanade の領分・50 ms の丸め）・共有の数を刻み役が読む（変わる時点が分かるのにプル）。完了 `areka-P0-choice-select-events` の design.md:566「Tick で判定」は要件 9.5 のとおりこの設計で改めた（保管の写しは直さない）。
- **#33**: 私的な `run_shiori_loop` に手空きの周期を引数で渡す（本番は `IDLE_INTERVAL`）。テストは周期 5 ms・上限 10 秒。
- **#6 の形**: `KanadeMsg::UserBreak { scope, talk_id: Option<TalkId> }`。`None` は「表示の側が観測できない」（`balloon_visible: Option<bool>` と同じ決まり）で、kanade は「合わない」に入れて表 C へ。`press_with_visibility`・`on_box_press` の引数は変えない（`user_break_tests.rs` の呼び出しの行を変えないため・要件 9.9）。
- **#61 と #62 の重なり**: #62 の呼び手が `resource_get` を使うので、#61 で再輸出から消すのは 7 つ（`resource_get` は残す）。
- **#4**: 欄 `choice_prev_talk` を `prev_talk` に改名（17 ファイル）。名前が中身と食い違ったままだと、かけ直しの audit がまた指す。
- **束**: ファイルのまとまりで 19 本（要件 9.10）。1,000 行の近いファイルを減らす束（T1〜T4）を先に、範囲の外へ及ぶ #7（T16）・#6（T17）を後ろに置いた。
- **合成（design-synthesis）**: 一般化＝「頭書きを切る 3 字」は `crate::sstp::LINE_BREAKERS` 1 つにまとめ、入口の検査・送る点の検査・SEND の取り除きの 3 か所が使う。採る・作る＝標準の `str::replace`／`contains` と今の中継の部品で足り、新しい部品は作らない。削る＝#7 の取り消しの知らせは作らない（古い起こしは空振りして `trace!` だけ）。

## 12. 着手の前の基準（2026-11-07・タスク 1.1）

`placement-crate-split` は着地済みで `main` をこの枝へ取り込み済み（`8f989143`）。`crates/areka-placement` があり、`crates/areka-kanade` に `crate::placement` の名指しは 0 件。コードは変えていない。

- **テスト**（`cargo test -p areka-kanade`・緑）: lib（`src/lib.rs`）713 通過・失敗 0・無視 0／結合（`tests/kanade.rs`）86 通過・失敗 0・無視 0／doc 0 件。直しの後も、この 799 本が同じ結果であること（要件 4.5・7.2。足したテストの分だけ数は増える）。
- **clippy**（`cargo clippy -p areka-kanade --all-targets --no-deps`）: 警告 16 件（lib 1・lib test 5 のうち 1 は lib と重なり・結合 11）。内訳は `manual_contains` 7・`type_complexity` 4・`doc_lazy_continuation` 3・`collapsible_if` 1・`drain_collect` 1。§1「clippy の実測」と同じ。
- **行数**: 「1,000 行の番人」の表の 13 ファイルは、表の「今」の数と全部同じ（最大は `src/actor_tests.rs` の 983）。束の順を見直す要は無い。

## 13. かけ直しの結果（タスク 19.2・2026-11-07）

### やり方

- 要件 1 と同じ作法でかけた。数の上限を置かず、`crates/areka-kanade` の 147 ファイル（うち `.rs` 146）を全部の行まで読む。読むだけの読み手 8〜9 人に範囲を分けて渡し、出た指摘はどれもその行を開いて本当かを確かめてから数えた。
- 3 回目からは、Nice の組は本物の欠陥のときだけ挙げる決まりにした（嘘を言う注・使われないコード・揃えて変えないと食い違う写し・決して落ちない assert）。好みの書き方の違いは挙げない。
- 5 回目からは、指摘ごとに「本番の振る舞い（production）」か「テスト・注（test-doc）」かの印を付けた。
- 各回の指摘は全部直し、回ごとに 1 つずつコミットした。残すと裁定した #63 の部分は数えない。

### 回ごとの数

| 回 | 指摘 | 組の内訳 | 本番の振る舞いが変わった直し | コミット |
|---|---|---|---|---|
| 1 | 165 | Must 0・Should 14・Nice 151 | ゴーストの切替の保留中はマウスの入力を捨てる（`mouse_change_pending`）。記録の event `talk_done_stale_choice` を `talk_done_stale` に改名。`boot_unexpected_reply` に `reply` の欄を足した。クレートの根からの `ExternalStates` の再輸出をやめた | `22266c24` |
| 2 | 94 | Should 17（Must・Nice の分けは記録していない） | クレートの根からの再輸出をさらに 5 つやめた。最初の答えの写しは、RaiseEvent か SSTP の返信を待つ人が居るときだけ取る | `365f411d` |
| 3 | 35 | Should 2（同上） | `shiori_wait_cut` の文言。バルーンのダブルクリックを断る記録に、押されたトークの番号を載せた | `4c8ae3a7` |
| 4 | 36 | Should 1（同上） | 資源の問い合わせは、`ALLOWED_RESOURCE_IDS` に無い ID を往復の前に断る（`resource_id_not_allowed`）。`change_rejected_phase` の理由に `close_pending` を足した | `a9bda95c` |
| 5 | 29 | Must 0・Should 0・本番の振る舞い 0（全部 test-doc） | なし（使われない腕を消した・`pub` を `pub(crate)` にした・記録の文言を直した） | `3810676e` |

### 裁定

- 字面の 0 件には収束しなかった（165→94→35→36→29）。
- 5 回目で打ち切った。開発者の決まり（`audit-seriko` のときの裁定「かけ直しは 5 回目で区切ってよい。条件は Must が 0 件で、本番の振る舞いの指摘が 0 件のこと。その回の指摘を直してから区切る」）による。5 回目は 2 つの条件をどちらも満たした（Must 0 件・本番の振る舞いの指摘 0 件）。その回の 29 件は直して `3810676e` に入れた。

### 直さずに残す 1 件

§3 の表の行をそのまま写す。

| # | 場所 | 中身 | 直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 63 | `src/status.rs:29-36`（`Minimizing`・`Induction`・`Passive`・`TimeCritical`・`Opening`）・`:73-114`（`OpeningKinds`・`OpeningKind`） | 本番は一度も作らない（テストの外の使い手 0 件）。しかも `:75` は「空集合は Opening 自体を非アクティブとする」と書くのに `OpeningKinds::new(vec![])` は `opening()` を出す＝将来この欄に源をつないだ人が踏む | 残すなら空の Opening を出さないように `render` を直す（または消す） | — | ⑴⑵ `status.rs:23`「残 5 状態は語彙に留め…（status-execution-states Req6.1/6.2）」・`:167`「行の存在自体が語彙保持の表明である（Req2.5）」 |

### 範囲の確かめ（diff）

- 触ってはならない所: `git diff 1806882f^..HEAD --stat -- vendors crates/areka-talk crates/areka-actor Cargo.lock` の出力は空（0 行）。パスが実在して空なのではないことを `git ls-files` で確かめた（`vendors` → `vendors/pasta`・`crates/areka-talk` → `crates/areka-talk/Cargo.toml`・`crates/areka-actor` → `crates/areka-actor/Cargo.toml`・`Cargo.lock` → `Cargo.lock`）。
- `crate::placement::` を名指す行: `git diff 1806882f^..HEAD -U0` の足した行・消した行のうち、`crate::placement::` を含むものは 0 行。
- `crates/areka-kanade` の外で変わったファイル（`git diff 1806882f^..HEAD --stat -- . ':!crates/areka-kanade' ':!.kiro'`）は 17 個で、どれも design.md 「範囲の外で触るファイル」の表にある:
  - `crates/areka-ghost/src/ticker.rs`・`ticker_tests.rs`・`runtime.rs`（#7）
  - `crates/areka-ghost/src/runtime_tests.rs`・`tests/ghost/spine_e2e_test_s2_connect_failure.rs`・`spine_e2e_test_s3_helper_liveness_detected.rs`・`spine_e2e_test_s6_full_disconnect.rs`（#7）
  - `crates/areka-ghost/tests/ghost/spine_e2e_test.rs`・`sylphya_integration_test.rs`（#62・後者は #7 も）
  - `crates/areka/src/log_history.rs`・`mcp/sakurascript_log_tests.rs`（#92）
  - `crates/areka/src/input_events/user_break.rs`・`user_break_tests.rs`・`shell_box_handler_tests.rs`・`user_break_talk_id_tests.rs`（新）（#6）
  - 申し合わせの 2 つ `crates/areka/src/emo2_boot/balloon_visibility.rs`・`emo2_boot/frame/wiring.rs`（#6・各 6 行）
- 表の文より多く変えた所が 1 つある: `crates/areka-ghost/src/runtime_tests.rs` は、表の「欄を全部挙げて分けている所に `choice_wake_relay: _` を足す」に加えて、起こしの中継が本物の刻み役のときだけ起きることを見るテスト `choice_wake_relay_exists_only_with_the_real_ticker` を 1 本足した（タスク 16.4・持ち主の表の行は同じ #7）。
- `placement-crate-split` の触るファイル（design.md 「範囲の外」に挙げた `log-capture-kit` の `file_length_guard_test.rs`・`temp_path_guard_test.rs`、wintf の `ecs/window_proc/` の 2 つ、`tools/perf/fixtures/`・`tools/perf/fixtures-loop/`）は、上の 17 個に 1 つも入っていない。
