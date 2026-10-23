# Brief: shiori2-protocol

> 起票: 2026-10-23（`areka-P0-dot-sakura-specimen` のタスク 5.1・開発者と `/kiro-discovery`）。根拠は `.kiro/specs/completed/areka-P0-dot-sakura-specimen/verification.md` の 5 節・8 節の件 1 と件 7。
> **開発者の条件（2026-10-23）: 次のリリース v0.1.0 までに治ること。ただし roadmap.md はこの起票では触らない**（版の段と並びは次の棚卸で決める）。

## Problem

SHIORI/2.x の口だけを持つゴーストが、areka では一言も話さない。開発者のゴースト「．さくら」（`dot_sakura.nar`・SHIORI は華和梨 7.4）を実機で起こすと、起動・時間の通知・`OnClose`・`OnTranslate` の要求が全部 `status 400` で断られ（`shiori_error_response` 132 行）、台本が 1 本も返らなかった。台本が来ないので、キャラクターの窓に面も 1 度も出なかった（`dump_surface` が 1 人目・2 人目とも `NG:No surface has been shown in this scope yet`）。

## Current State

- 要求の文を組むのは `crates/shiori-host32-host/src/shiori3.rs` の `build_request` 1 か所（呼び手は 32 ビットの経路 `shiori-host32-host/src/client.rs` と、同じプロセスの経路 `crates/areka-ghost/src/shiori_inproc.rs`）。要求の行は `GET SHIORI/3.0`・`NOTIFY SHIORI/3.0` の 2 つだけで、SHIORI/2.x の要求（`GET Sentence`・`GET String`・`NOTIFY OwnerGhostName`・`TRANSLATE Sentence` など）も、`GET Version` による版の確かめも無い。
- 応答の読み取り `parse_status_code`（同 `shiori3.rs`）は最初の数の字句を状態の番号として取り、`SHIORI/x.y` の字句は見ない。
- 400／500 は `crates/areka-kanade/src/actor.rs` の `round_trip_request` で `warn!(event="shiori_error_response")` を 1 行出し、GET は `NoContent`・NOTIFY は `Notified` として続ける（決めた形・記憶「SHIORI のエラー応答は致命ではない」）。数の閾値も版の切り替えも無い。
- 32 ビットの中継（`crates/shiori-host32-helper`）は要求のバイトを解釈せず素通しする。文は 64 ビットの側で組む。
- 起動で台本が来ないとき、キャラクターの窓は面を出さない（`crates/areka/src/emo2_boot/frame/attach.rs` の注記＝SSP に合わせて「シェル非表示（surface -1）」が既定・最初の面は台本の最初の `\s[N]` で出る）。

## Desired Outcome

SHIORI/2.x の口だけを持つゴースト（華和梨 7.4 の「．さくら」を検体とする）が、areka で SSP と同じように話す。起動の挨拶・時間のトーク・終了のトークが出て、面も出る。SHIORI/3.0 のゴーストの振る舞いは変わらない。

## Approach

ベースウェアの側で相手の SHIORI の版を確かめ、2.x の相手にはイベントを 2.x の要求へ組み替えて送り、応答を 3.0 と同じ形へ戻す。版の確かめ方（`GET Version` を先に送るか、3.0 の要求に 400 が返ったら 2.x へ移るか、その両方か）と、3.0 のイベントから 2.x の要求への対応表は、要件の段で ukadoc から引き、ukadoc で読めない点は SSP の MCP で測ってから決める（記憶「ukadoc で読めない点は議題の前に SSP の MCP で測る」）。

## Scope

- **In**:
  - 件 1: SHIORI/2.x の要求の組み立てと応答の読み取り・相手の版の確かめ・3.0 のイベントから 2.x の要求への組み替え（`GET Sentence`〔イベントの台本〕・`TRANSLATE Sentence`〔`OnTranslate`〕・`NOTIFY`〔通知だけのイベント〕・`GET String` など、正典が定める範囲）。2.x の応答の `Sentence:` などを 3.0 の `Value:` と同じ扱いへ戻す。
  - 件 2: 起動しても面が 1 度も出ない件（どっとさくらの件 7）。台本が来るまで面を出さないのは SSP に合わせた決めなので、件 1 が直れば消える見込み。件 1 の後に実機で確かめ、消えなければ正典（SHIORI が何も返さないときに既定の面を出すか）を確かめてから扱いを決める。
  - 決定論のテスト（2.x の偽の SHIORI で、要求の文と応答の戻しを固定）と、どっとさくらの実機の 1 回。
- **Out**:
  - 32 ビットの中継（helper）の中身（バイトの素通しのまま）。
  - SHIORI の文字コードの扱い（`areka-P0-charset-canon` の持ち場）。
  - MAKOTO（`makoto-dll-host`・`EXECUTE MAKOTO/2.0` は別の口）。

## Boundary Candidates

- 要求の文の組み立てと応答の読み取り（`shiori3.rs` の 1 か所＋2.x の対の作り）。
- 版の確かめと、kanade の往復（`round_trip_request`・`round_trip_raw`）での組み替えの分岐。

## Out of Boundary

- 400／500 を致命としない決め（変えない）。
- 起動で台本が来ないときに面を出さない SSP に合わせた決め（件 2 は確かめるだけ）。

## Upstream / Downstream

- **Upstream**: なし（今ある SHIORI の往復の上に足す）。
- **Downstream**: SHIORI/2.x のゴースト全般（華和梨・古い里々など）。`dot_sakura` を検体にした実機の確かめ。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `makoto-dll-host`（同じ DLL の口・wire は「SHIORI/2.x 風」）・`areka-P0-charset-canon`・`mcp-shiori-query`・`shiori4-api`（SHIORI の層を触る）・`audit-shiori-host32`（`shiori-host32-*` の見直し＝触るファイルが重なる）。

## Constraints

- 正典は ukadoc（SHIORI/2.x の節）。読めない点は SSP の MCP で測る。
- 決定論のテスト必達・SHIORI/3.0 のゴーストの既存テストは緑のまま。
- 検体 `dot_sakura` は `vendors/sample_ghost/dot_sakura.nar`（`sample-ghost-kit` の `SAMPLES` に登記済み）。
