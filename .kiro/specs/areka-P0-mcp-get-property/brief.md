# Brief: areka-P0-mcp-get-property

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントがゴーストを作るとき、プロパティシステム（`currentghost.*`・`system.*` など）を読んで状態を確かめたい。SSP の `get_property` がこれを担う。

## Current State

- `SylphyaReader::resolve_dotted_str`（`crates/areka-sylphya/src/reader.rs` 90 行目付近）は同期・ロック無しで、どのスレッドからも呼べる。今の呼び手は SHIORI 側の `GetProperty`（`crates/areka/src/shiori_host.rs` 247〜256 行目）。
- ランタイムの reader は私有の欄（`crates/areka-ghost/src/runtime.rs` 163 行目付近）で、外へは `GhostParts` からしか出ていない＝取り出し口が要る。
- 値の網羅（`currentghost.*` の多くが未実装）は別の spec の持ち物（下の Existing Spec Touchpoints）。

## Desired Outcome

- `get_property(property_name, ghost_name?)` が、SHIORI の `GetProperty` と**同じ解決**で値を素の文字列で返す（survey §3: `currentghost.name` → `Emily/Phase4.5`）。
- 無い名前は `NG:Cannot find such property name.`（`isError: true`）。
- 本 spec の後にプロパティの網羅が進めば、MCP 側は何もせずに答えが増える。

## Approach

`mcp-tool-entrances` が置いたダミーの中身を書く。reader の取り出し口を `GhostRuntime`／`GhostSession` に足し、UI スレッドの処理で引く（切替の後も今のゴーストの reader を引く）。

## Scope

- **In**: reader の取り出し口・`get_property` の中身・「無い名前」と「値が空」の区別（SHIORI の `GetProperty` の既存の区別に合わせる）・決定論テスト。
- **Out**: プロパティの値そのものの追加。

## Boundary Candidates

- sylphya の読み手（既存）と MCP のツール（新）の間の取り出し口 1 つ。

## Out of Boundary

- `currentghost.*`・一覧系・`status`・`zorder` などの値の実装（下の既存 spec）。
- 書き込み（SSP の MCP に `set_property` は無い）。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `property-query-channels`・`property-ipc-transport`・`currentghost-property-tree`・`property-catalog-lists`（いずれも α 後・brief のみ）。これらが値を足すと本ツールの答えが増える。

## Constraints

- 規模 S。`crates/areka-ghost/src/runtime.rs` を触る＝同じウェーブの `mcp-reload` と重なりうる（干渉台帳を見よ）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。

## 2026-10-03 ウェーブ C3-⑥（予定・10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。`mcp-tool-entrances`（C2）の design が固定した「自分のツールのファイル」と、同じ C3 の他の spec（`balloon-lifecycle-events` は kanade・`balloon-font-file` は emo-text）の触るファイルを、着手の前に照合する。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S（4〜6 タスク）。切らない。
- 前提の状態: `mcp-tool-entrances` は着地済み（PR#223・`completed/areka-P0-mcp-tool-entrances/`）。前提は満たす。
- 崩れた前提／古くなった位置:
  - ダミーの場所が決まった: アプリ本体側 `crates/areka/src/mcp/get_property.rs` の `handle(_world, _ghost, _args, reply)`（本体は `reply.send(outcome::ng("not implemented yet"))` の 1 文）と、そのテスト `get_property_tests.rs`（`NG:not implemented yet` を期待＝中身を入れたら書き換える）。プロトコル側 `crates/areka-mcp/src/tools/get_property.rs` は定義の逐語と `Args { property_name: String, ghost_name: Option<String> }` まで完成済み＝触らずに済む見込み。
  - `ghost_name` の解決は `mcp/mod.rs` の `dispatch`（`Omitted::UseActive`）が済ませてから `handle` に `&ActiveGhost` が来る。今のゴーストの実行系は `GhostSlot` → `GhostSession::runtime()`（`ghost_session.rs`）。
  - reader の取り出し口はまだ無い: `GhostRuntime` の欄 `sylphya_reader`（`crates/areka-ghost/src/runtime.rs` の `pub struct GhostRuntime`）は私有のまま、外へ出るのは `into_parts` の `GhostParts.sylphya_reader` だけ。足すのは `GhostRuntime` の読み口 1 本（例 `sylphya_reader(&self) -> &SylphyaReader`）。
  - 問い手（`AskerContext`）は SHIORI の `GetProperty`（`crates/areka/src/shiori_host.rs` の `ShioriHostSink::GetProperty`）と同じく、そのゴーストの問い手 `areka_ghost::sylphya_wiring::ghost_asker_id(&mount.shiori.dir)` で組める（`GhostRuntime::mount()` は公開済み）＝`runtime.rs` に問い手の欄を足さずに済む。
  - `SylphyaReader::resolve_dotted_str` は同期で `DottedResolution::{Value, NotFound}` の 2 つ。「無い名前」＝`NotFound` → `NG:Cannot find such property name.`、「値が空」＝`Value("")` → 空の本文（`outcome::value("")`）で区別できる。同期なので `mcp::later` は使わずその場で答えられる。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/mcp/get_property.rs`・`crates/areka/src/mcp/get_property_tests.rs`
  - `crates/areka-ghost/src/runtime.rs`（`GhostRuntime` の読み口 1 本）
  - 触らない: `crates/areka/src/mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/src/**`・`main.rs`・`ghost_session.rs`・`Cargo.toml`
- 議題（答えで作業が変わるものだけ）: なし（「無い名前」と「値が空」の区別は SHIORI の `GetProperty` の既存の区別に写すだけ）。
- 見つけた穴: なし。注意 1 点＝実行系の無い単位（`GhostSession::for_test`・LogSink へ倒れた単位で reader が無い形）では `resolve::active` が `None` を返すので `handle` まで来ないが、World に置き場が無いときも panic せず `NG:` で答える約束（tool-entrances の design「mcp/<ツール>.rs」）を守る。

## 2026-10-04 ウェーブ C3-⑦（棚卸㉑）

- 段は「優先」。C3 は 11 本並走（`roadmap.md`「ウェーブ編成」の C3 の行が正本）。着手は最新の main から。
- 同じウェーブの約束: `crates/areka/src/mcp/mod.rs`・`handler.rs` を触らない。足すファイルは自分のツールのファイルの子モジュールにする。エンジンの側は `crates/areka-ghost/src/runtime.rs` の読み口 1 本だけ。
