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
