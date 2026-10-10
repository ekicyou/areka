# Brief: areka-P0-mcp-kanade-tools

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本＝kanade（台本の実行と SHIORI の往復）を触る 3 ツールをまとめた。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

MCP でいちばん使われるのは「台本を流して見る」（`sakurascript`）と「イベントを起こして反応を見る」（`raise_event`）、その前に「今しゃべっているか」（`get_status`）。3 本とも kanade の中を触るので 1 spec にまとめる（別 spec にすると `KanadeMsg` と actor の同じ箇所を 2 本が触る）。

## Current State

- **get_status**: `crates/areka-kanade/src/status.rs` に 10 状態（`ExecutionState`・`ExecutionStatus::derive`・`render`＝カンマ区切り）がある。今は `talking` と `choosing` だけが実物から導かれ、他の 8 つは常に偽。生きたスナップショットは `schedule/mod.rs` の `State::snapshot()`。**外から問う口が無い**＝`KanadeMsg` に問い合わせの変種が要る（`ResourceQuery` と同じ作り）。`balloon(...)` の素は `EmoPresenter::target_visible`（`areka-emo-present/src/presenter/read.rs` 217 行目付近）。
- **sakurascript**: 外から台本を流す経路が無い。台本は kanade の `Action::StartTalk`（`schedule/steady.rs` 407・627 行目付近）からしか始まらない。talk の ID の採番と talk の枠を崩さない形で、SHIORI の `Value` と同じ経路へ流す変種が要る。SSP では「対象ゴースト自身の処理（Owned SSTP）と同様に扱われ、`\![reload,...]` なども実行できる」。しゃべっている最中に送ると中断または拒否（survey §3）。
- **raise_event**: `KanadeMsg::RaiseEvent`（`msg.rs` 222 行目付近）がある。ただし ⑴ 許可表 `ALLOWED_EVENT_IDS`（`schedule/events.rs` 82 行目付近）が任意のイベントを拒む、⑵ Steady の相でしか動かない、⑶ 結果 `RaiseOutcome`（`change.rs` 105 行目付近）が台本の文字列を持たない＝返り台本を MCP の結果に含められない。呼び手の手本は `install/desk.rs` 461 行目付近。

## Desired Outcome

- `get_status`: kanade の今の状態を SSP と同じ書式で返す（空＝idle）。どの旗が実物から導かれるかは `status-execution-states` の進み具合に従う（本 spec は口と書式を持つ）。
- `sakurascript`: 台本がゴースト自身の台本と同じ権限で再生される。返事は `OK`（strict 時の文言は survey §3。strict の中身は `mcp-strict-errors`）。
- `raise_event`: 任意のイベント ID と Reference 列で SHIORI を呼び、返った台本を再生し、**台本の文字列を結果に含める**。返らなければ `OK:the ghost returned no script.`。
- 3 本とも、しゃべっている最中・選択肢の最中・切替の最中の扱いを SSP に合わせて決める（要件）。

## Approach

`KanadeMsg` に「状態の問い合わせ」「外からの台本」の 2 変種を足し、`RaiseEvent` は MCP の呼び出しでは許可表を通さない印と、台本の文字列を返す結果を持たせる。`mcp-tool-entrances` のダミー 3 本の中身から、World の今のゴーストの kanade へ返事付きで送る。

## Scope

- **In**: 上の 3 ツールの中身・kanade の 2 変種と `RaiseEvent` の拡張・`strict` の引数を受けて**下流へ渡す口**（記録の中身は `mcp-strict-errors`）・**再生した台本を script 種別のログへ出す 1 行**（`mcp-log-history` と取り決める tracing の target 名で出す）・決定論テスト（偽 SHIORI）。
- **Out**: 8 状態の導出（`status-execution-states`）・strict の検出と記録（`mcp-strict-errors`）。

## Boundary Candidates

- kanade の inbox（新しい変種）と、MCP のツール（呼び手）の境。
- 「MCP から来た」という出どころの印（許可表の迂回・script ログの種別名 `[SSTP(Local,Auth)]` 相当）。

## Out of Boundary

- SSTP の SEND／NOTIFY そのもの（予約）。同じ変種を将来 SSTP が使えるよう名前は SSTP 寄りに付けてよい。
- `\![raise]`／`\![embed]` などのタグ（別の正典の spec）。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`。
- **Downstream**: `mcp-strict-errors`（strict の口を使う）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `status-execution-states`（α 後・brief のみ。8 状態の導出の持ち主。本 spec は同じ `render` を使う消費者が 1 つ増えるだけ）。

## Constraints

- kanade の talk の ID と枠を崩さない（既存の決定論テストを 1 本も落とさない）。
- 規模 M〜L。kanade を触るのは同じウェーブで本 spec だけにする（干渉台帳）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。
- `raise_event` は kanade の許可の表（`ALLOWED_EVENT_IDS`・44 語）を通らない名前を起こす必要がある。`property-query-channels` の `\![get,property,<イベント名>]` も同じ迂回が要る＝先に着手した方が一度で設計する。`get_status` は `status-execution-states` が足す状態を読む。

## 2026-10-03 C4 の候補（10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。C3 に入れなかった理由: C3 の `balloon-lifecycle-events` と kanade を分け合う。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: L（14〜19 タスク）。20 を超えるなら `get_status`（問い合わせの変種 1 つと書式だけ・S）を先に切り出し、`sakurascript`＋`raise_event`（M〜L）を後にする。今の見積もりでは切らない。
- 前提の状態: `mcp-tool-entrances`（PR#223）・`status-execution-states`（PR#220）・`translate-pipeline`（PR#226）は着地済み。前提は満たす。kanade の進行の列（roadmap「直列の列」）では `mouse-drag-events`（C2-⑦）→ `balloon-lifecycle-events`（C3-④）→ `property-query-channels` → `network-update-canon-order` の後ろ＝それらと同時に走らせない。
- 崩れた前提／古くなった位置:
  - ダミーの場所: アプリ本体側 `crates/areka/src/mcp/{get_status,sakurascript,raise_event}.rs` の `handle`（各 `NG:not implemented yet` の 1 文）と各 `_tests.rs`（同じ文言を期待＝書き換える）。プロトコル側 `crates/areka-mcp/src/tools/` の同名 3 ファイルは定義と `Args`（`sakurascript`＝`script`・`ghost_name`・`strict: Option<bool>`、`raise_event`＝`event`・`references: Vec<String>`・`ghost_name`・`strict`）まで完成＝触らない見込み。kanade の返事は `areka_actor::ReplySender<具体の型>` で受け、`mcp::later(world, reply, 覗く関数)`（`crates/areka/src/mcp/mod.rs`）に預ける形が決まっている（`ReplyTo` は kanade へ渡せない）。kanade への送り口は `GhostSession::kanade()`。
  - `get_status`: `status-execution-states` が `talking`・`choosing` に加えて `nouserbreak`・`online`・`balloon(…)` も実物から導くようにした（`ExecutionSnapshot` の欄 `no_user_break`・`online`・`balloons`、外からの入口 `KanadeMsg::ExecutionState`）。**`balloon(…)` の素はもう `EmoPresenter::target_visible` ではなく kanade の中の写し（`ExternalStates.balloons`）**＝kanade に問えば全部そろう。外から問う口は今も無い（`KanadeMsg` に問い合わせの変種なし）。手本は `KanadeMsg::ResourceQuery`（`actor_resources.rs` が状態機械を経ずに答える）。
  - `raise_event`: `KanadeMsg::RaiseEvent { id, references, method, reply: Option<ReplySender<RaiseOutcome>> }`（`msg.rs`）は在る。許可表は `schedule/events.rs` の `ALLOWED_EVENT_IDS`（46 語）。`RaiseOutcome`（`change.rs`）は `NotAllowed`／`NotSteady`／`Script`／`NoReply`／`Failed` で台本の文字列を持たない。今の呼び手は 6 か所（`emo2_boot/frame/switch.rs`・`input_events/file_drop.rs`・`install/desk.rs`・`update/desk.rs` 2 か所・`update/worker.rs`）＝`RaiseOutcome` を変えると全部に波及するので、台本つきの結果は**別の型か別の変種**で返す方が触る所が少ない。
  - `sakurascript`: 外から台本を流す経路は今も無い。台本の起点は `Action::StartTalk`（`schedule/steady.rs`・`change.rs`・`close.rs`・`boot.rs`）。`StartTalk` の正本は `crates/areka-talk`（`areka_kanade::talk` は再エクスポート）。
  - **翻訳の経路が増えた**: `translate-pipeline` が SHIORI の台詞を再生の前に `OnTranslate` へ通すようにし（`schedule/translate.rs`・`actor_translate.rs`・`translate.rs`）、「Reference1 は常に欠番」を `doc/COMPAT_ARCHITECTURE.md` §8 に登記した。ukadoc の `OnTranslate` の Reference1 には SSP の出どころの語（`sstp-send`・`owned` など）がある＝MCP の台本を翻訳に通すか・通すなら Reference1 に何を入れるかを決める必要が出た（下の議題）。
  - kanade のファイルが上限に近い: `schedule/mod.rs` 937 行・`schedule/steady.rs` 929 行・`msg.rs` 902 行・`actor.rs` 876 行（上限 1,000）。新しい処理は兄弟の新規ファイル（`actor_resources.rs` の形）へ置く前提で見積もる。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/mcp/{get_status,sakurascript,raise_event}.rs` と各 `_tests.rs`
  - `crates/areka-kanade/src/msg.rs`（変種 2 つ＋`RaiseEvent` の拡張・名前の腕）・`actor.rs`（振り分け）・**新規**の殻の処理（例 `actor_external.rs`＋兄弟テスト）・`schedule/mod.rs`（`Input` の変種）・`schedule/steady.rs` または新規の `schedule/` の子（外からの台本の起動）・`schedule/events.rs`（許可表を迂回する印）・`change.rs`（台本つきの結果）・`lib.rs`（公開）
  - 翻訳に通すなら `schedule/translate.rs`・`translate.rs` と `doc/COMPAT_ARCHITECTURE.md` §8
  - script 種別のログの 1 行（`mcp-log-history` で取り決め済み: target `areka::log::script`・欄 `ghost`・`label`。正本は `doc/ssp-mcp/log-convention.md`）
  - 触らない: `crates/areka/src/mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/src/**`・`main.rs`・`ghost_session.rs`・`Cargo.toml`
- 議題（答えで作業が変わるものだけ）:
  - MCP の `sakurascript` の台本を `OnTranslate` に通すか。通すなら Reference1 を欠番のままにするか SSP と同じ出どころの語にするか（SSP に同じ台本を送って `OnTranslate` の Reference1 を実測する）。答えで翻訳の経路と §8 の登記が変わる。
  - 許可表の迂回は `property-query-channels`（`\![get,property,<イベント名>]`）も要る＝先に着手した方が一度で作る（棚卸⑳の注記のまま）。kanade の列では `property-query-channels` が前なので、そちらが作った迂回を使う形になる見込み。
- 見つけた穴: なし（`RaiseOutcome` を広げると呼び手 6 か所に波及する点は設計の注意）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模（タスク数）と切るかどうか: 3 本まとめて L（14〜19）。**切る提案（大きさでなく並走のため）**: `get_status` だけを新しい spec `mcp-get-status`（S・4〜6）へ出す。`get_status` は kanade の運行（`schedule/` の下）に触らず、殻がその場で答える形（`KanadeMsg::ResourceQuery` と同じ作り・`State::snapshot()` を読んで `ExecutionStatus::derive(..).render()` を返すだけ）で足りるので、kanade の進行の列を待たずに走れる。残る本 spec（`sakurascript`＋`raise_event`）は M〜L（11〜15）で、それ以上は切らない。
- 前提の状態: 満たす。C3 で MCP のツール 4 本（`get_property`・`get_expression_table`・`get_log`・`dump_surface`／`dump_balloon`）が本物になり、今も `NG:not implemented yet` で答えるのは 4 本だけ＝本 spec の 3 本（`crates/areka/src/mcp/{get_status,sakurascript,raise_event}.rs` の `handle`）と `reload`。`mouse-drag-events`（PR#240）も着地済み。kanade の進行の列では今 `balloon-lifecycle-events` → `sakura-time-critical` → `property-query-channels` の後ろ。**優先度の 3 段（バグ → MCP など → その他）に照らすと、その他の段の `sakura-time-critical`・`property-query-channels` より前に出してよい**（許可の表の迂回は先に着手した本 spec が作り、`property-query-channels` がそれを使う）。
- 崩れた前提／古くなった位置:
  - 許可の表 `ALLOWED_EVENT_IDS`（`schedule/events.rs` の定数の定義）は 46 語 → **48 語**（`mouse-drag-events` が `OnMouseDragStart`・`OnMouseDragEnd` を足した）。
  - `KanadeMsg::RaiseEvent` の返事つきの呼び手は 7 か所（棚卸㉑の 6 か所＋テストの支え `crates/areka/src/mcp/dump_surface_gpu_test_support.rs`）。`RaiseOutcome::` を読む所は `install/worker.rs`・`update/worker.rs`・同じテストの支えの 3 つ。台本つきの結果は別の型で返す方針のまま。
  - `mcp::later`（`crates/areka/src/mcp/mod.rs`）は使う側ができた（`dump_surface`・`dump_balloon`）。`#[allow(dead_code)]` は外れた。手本は `dump_surface.rs` の `handle`。
  - kanade のファイルの行数: `schedule/steady.rs` 947・`schedule/mod.rs` 938・`msg.rs` 909・`actor.rs` 876（上限 1,000）。`sakurascript` の起動の処理と `Input` の変種は新しいファイルへ置く前提のまま。
  - 書き出しの約束（script 種別のログの 1 行）は `doc/ssp-mcp/log-convention.md` で確定済み（target `areka::log::script`）。
- 触るファイル（並走の照合用）:
  - 切った後の `mcp-get-status`: `crates/areka/src/mcp/get_status.rs`・`get_status_tests.rs`・`crates/areka-kanade/src/msg.rs`（変種 1 つと名前の腕）・`actor.rs`（振り分けの腕 1 つ）・新規 `actor_status.rs`＋兄弟テスト。`lib.rs`（`ExecutionStatus` は公開済み）・`schedule/` には触らない。
  - 残る本 spec: `crates/areka/src/mcp/{sakurascript,raise_event}.rs`＋各 `_tests.rs`・`crates/areka-kanade/src/{msg.rs, actor.rs, lib.rs}`・新規の殻の処理（例 `actor_external.rs`）・`schedule/mod.rs`（`Input` の変種）・新規 `schedule/` の子・`schedule/events.rs`（迂回の印）・台本つきの結果の新しい型のファイル（`src/change.rs` に置かなければ `mcp-reload` と重ならない）・翻訳に通すなら `schedule/translate.rs`・`translate.rs`。
  - 触らない: `crates/areka/src/mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/src/**`・`main.rs`・`ghost_session.rs`。
- 議題（答えで作業が変わるものだけ）:
  - `get_status` を `mcp-get-status` へ切るか（切れば C4 で `balloon-lifecycle-events` と並べられる）。
  - SSP の旗には `changing`（survey §3 の表）があり、areka の 10 状態（`crates/areka-kanade/src/status.rs` の `ExecutionState`）にも ukadoc の SHIORI/3.0 の `Status` にも無い。足すか（足すなら `status.rs` と切替の写しの通り道を触る）。
  - 前回からの 2 つ（`sakurascript` の台本を `OnTranslate` に通すか・許可の表の迂回の持ち主）はそのまま。
- 見つけた穴: なし。

### 棚卸㉒の裁定（2026-10-05）

- `get_status` を `mcp-get-status`（S・4〜6）へ切り出した（並走のため）。本 spec は `sakurascript`＋`raise_event` だけ＝M〜L（11〜15）。`msg.rs`・`actor.rs` を分け合うので `mcp-get-status` と同じウェーブに置かない。
- kanade の進行の列で、その他の段の `sakura-time-critical`・`property-query-channels` より前へ出した（優先度の 3 段）。許可の表を迂回して任意の名前のイベントを送る口は本 spec が作り、`property-query-channels`・`mcp-shiori-query` はそれを使う。

## 2026-10-05 `mcp-dump-images-residue` からの引き継ぎ（実機確認 1 件）

- 前の spec `areka-P0-mcp-dump-images` の要件 7.7 ⑵（`sakurascript` で表情を変えた後に `dump_surface` で撮ると、変わった姿が原寸で返る）は、`sakurascript` が無かったため、開発者の了承のうえキャラクターのダブルクリックで表情を変える代わりの手順で確かめた。
- 本 spec の実機確認で、`sakurascript` で表情を変えてから `dump_surface` を 1 回呼び、変わった姿が返ることを本来の手順で撮り直し、本 spec の `verification/signoff.md` に残す。
- 出どころ: `areka-P0-mcp-dump-images-residue` の要件（Boundary Context の Out of scope・件 6）。棚卸㉒の C4 の約束で、`mcp-dump-images-residue` は項目 1〜5 だけを持つ。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - `get_status` は `mcp-get-status` で着地した。問い合わせの知らせ（`KanadeMsg::StatusQuery`）に答える関数は kanade の殻 `crates/areka-kanade/src/actor.rs` の中にあり、棚卸㉒が見込んだ新しいファイル `actor_status.rs` は作られていない。
  - `choice-script-prefix` が「台本 1 つを新しいトークとして始める」関数 `start_talk` を `crates/areka-kanade/src/schedule/steady_choice_script.rs` に作った（使えるのは `schedule` の中だけ。同 spec の設計書は「`sakurascript` はこの関数を呼べる・置き場を移してもよい」と書く）。番号の採番と枠の差し替えは使い回せる。再生中の置き換えのときの選択肢の帳簿の掃除は呼び手の仕事。
  - `balloon-lifecycle-events` が許可の表（`schedule/events.rs` の `ALLOWED_EVENT_IDS`）を 48 語から 51 語にし、時間切れの知らせ専用の入口（`KanadeMsg::BalloonTimeout`）を足した。`raise_event` で `OnBalloonTimeout` などを頼まれたら、渡された Reference をそのまま送る（kanade は補わない）でよいかを要件で確かめる。
  - 許可の表との照合は `schedule/events.rs` ではなく、`schedule/change.rs` の `on_raise_event` と、殻（`actor.rs`）の返事を決める所の 2 か所にある。イベント名の型（`msg.rs` の `EventId`）は固定の綴りと選択肢用の 2 種だけで、任意の名前を運ぶ形が無い。
  - `mcp-ghost-name-match` で宛先の型 `ActiveGhost` に本体側の名前の欄が増えた（テストで字面で組むときは欄を 1 つ足す）。今も `NG:not implemented yet` を返すのは `sakurascript`・`raise_event`・`reload` の 3 本。
- 触るファイル:
  - `crates/areka/src/mcp/{sakurascript,raise_event}.rs` と各 `_tests.rs`
  - kanade: `msg.rs`（926 行）・`actor.rs`（900 行・振り分けの行だけ）・`lib.rs`・`schedule/mod.rs`（955 行・入力の種類と振り分け）・`schedule/change.rs`（`on_raise_event`）・`schedule/events.rs`（任意の名前で要求を組む形）・`schedule/steady_choice_script.rs`（`start_talk` を使える範囲か置き場）
  - 新規: 殻の側の処理（例 `actor_external.rs`）・台本つきの結果の型のファイル・`schedule/` の子（外からの台本の受け付け）と各兄弟テスト・SSP との差の一覧（`doc/ssp-mcp/` の下）
  - 翻訳に通すなら `schedule/translate.rs`・`translate.rs`・`doc/COMPAT_ARCHITECTURE.md` §8
  - 触らない: kanade の `src/change.rs`・`schedule/boot.rs`（`mcp-reload` と並べるための約束）・`schedule/steady.rs`（950 行）・`crates/areka/src/mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/**`・`main.rs`
- 規模: M〜L（12〜16 タスク）。`msg.rs` を分ける 1 タスクの分だけ棚卸㉒（11〜15）より増えた。
- 先に要るもの: 働きの前提はすべて着地済み＝今すぐ始められる。
  - 同じファイルを触るので同じウェーブに置けない相手: `farewell-talk-status`（`schedule/mod.rs`）・`script-security-level`（`msg.rs`・`actor.rs`・`schedule/mod.rs`・`schedule/change.rs`・`steady_choice_script.rs`）・`mcp-shiori-query`（`msg.rs`・`actor.rs`）・`anchor-tag-canon`（`msg.rs`・`lib.rs`・`schedule/mod.rs`・`schedule/events.rs`）・`sakura-time-critical`・`property-query-channels`。
  - 重なり 0: `mcp-reload`（上の約束を守れば）・`mcp-stdio-bridge`・`dump-balloon-debug-timeout`。
  - 本 spec を待つ未完了の spec は 5 本（`mcp-strict-errors`・`mcp-user-response`・`script-impact-tiers`・`mcp-shiori-query`・`property-query-channels`）。
- 優先度の区分: A（開発者の指示「ssp mcp tool の完全移植のための spec 群を立ち上げて」）。
- 要件定義のモデル: Fable（再生中・選択肢の最中・切替の最中に届いたときの扱いを SSP に合わせて決める・翻訳に通すかの分かれ目・返事を待つ間の順序）。
- 分割の案: なし（一度切り出した spec）。上限の近いファイルの分け方だけ決めておく: `msg.rs` は本 spec が最初のタスクで分ける（ファイルの中のテストの塊 約 415 行を兄弟のテストファイルへ出すと本体は約 510 行）。`schedule/mod.rs` は本 spec の後で約 975〜980 行になる見込みで、次に足す spec が分けることになる＝後ろに 5 本以上並ぶ本 spec が先に分けておくのが安い。
- 見つけた穴・古くなった記述: 棚卸㉒の節の `actor_status.rs` は実在しない。同じ節が許可の表の迂回の置き場を `schedule/events.rs` と書いたのは不正確（照合は `schedule/change.rs` と `actor.rs`）。roadmap の「3 段目の spec が触るファイル」の表の本 spec の行に `get_status.rs` が残っている。
