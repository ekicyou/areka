# 設計検証レポート: areka-P0-idle-talk

- **対象**: `.kiro/specs/areka-P0-idle-talk/design.md`（phase: design-generated・2026-07-17）
- **検証**: `kiro-validate-design`（非対話・rules/design-review.md 準拠）
- **判定**: **GO**

## 検証サマリ

本設計は「新規経路を作らず、`Status` を4クレートへ層貫通させる」という主題に対し、既存アーキテクチャ（純粋状態機械＋アクターシェル＋メッセージ境界差替）を保存したまま最小侵襲で着地している。特筆すべきは**主要な設計判断が実コードと実 wire 捕獲ログで一次裏取りされている**点で、下記のとおり本レビューが独立に再実測した限り DD-IT-3/4/5/6/7/8 の前提はすべて事実と一致した。要件 1〜6 は Requirements Traceability 表で全 AC が構成要素へ写像され、実装経路は明瞭である。残る指摘は 3 件すべて**精緻化レベル**であり、アーキテクチャ上の不整合ではない。

## 独立検証した一次事実（本レビューの再実測）

| 設計の主張 | 実測結果 | 判定 |
|---|---|---|
| DD-IT-7: `round_trip_request` が `Action::ShioriRequest` の唯一の出口 | `actor.rs:111-113` が唯一の実行点。本番構築点は `boot.rs:44,58,69,121`／`steady.rs:65,74,178`／`mod.rs:170` のみで全て Action 経由 | **正** |
| DD-IT-7: 統合 mock は `ShioriMsg` 層で shiori アクターごと差替＝`handle_call` を通らない | `tests/kanade/common/mod.rs:264,368,531` が `ShioriMsg::Request` を直接受理 | **正** |
| DD-IT-4: `force_quit` は `Unloading{Forced}` 遷移**後**に OnClose を構築 | `mod.rs:167-173`（phase 代入 → notify 構築）＝送出時点 snapshot は talk 非アクティブ | **正** |
| `begin_close` も同型（ClosePending 遷移後に構築） | `steady.rs:177-178` | **正**（設計の単一規則が close 経路でも例外なく成立） |
| 既存 `build_request` 檻は `contains`／`starts_with` ベース＝Status 追記は非破壊 | `shiori3.rs:373-484` 全て `contains`/`starts_with`。`:434` の `!contains("Reference")` も正典語彙と非衝突 | **正** |
| `build_request` の現行ヘッダ順（Status 挿入位置の前提） | request-line→`Charset`→`Sender`→`ID`→`Reference*`→`SecurityLevel`（`shiori3.rs:93-115`）＝`Sender` 後・`ID` 前への挿入は自然 | **正** |
| File Structure Plan が close.rs を挙げないこと | `close.rs` は `#[cfg(test)]:180` 以降のみに `ShioriCall` 出現＝本番構築点なし | **正**（除外は妥当） |
| `ShioriBackend` 実装は5箇所 | `real.rs:58`(prod)＋テスト4＝5 | **正** |

## Critical Issues

### 🔴 Critical Issue 1: File Structure Plan が `Shiori3Client` 署名変更の爆風を数え落としている

**Concern**: `Shiori3Client::{get,notify}` へ `status: Option<&str>` を追加すると、`shiori-host32-host/tests/` の **e2e 3ファイル・計8呼出点**が破壊されるが、変更ファイル表に**1つも挙がっていない**——`shiori_request_e2e.rs:228,240,339`／`lifecycle_kill_e2e.rs:240,286`／`lifecycle_cyclic_e2e.rs:241,251,379`。加えて `ShioriRequest` へのフィールド追加は `shiori3.rs` in-source テストの**構造体リテラル7箇所**（`:363,386,403,425,442,458,474`）を悉く破壊する。設計末尾の「機械的追随」注記は文面上 **`ShioriCall` に限定**されており `ShioriRequest`／`Shiori3Client` を被覆しない。一方 Risks 表は「実装5箇所は既知（本書 File Structure Plan に列挙）」と**完全性を明示的に主張**している（この「5箇所」＝`ShioriBackend` 実装数としては正しいが、`Shiori3Client` 署名変更の爆風は別軸で未計上）。

**Impact**: tasks 生成は File Structure Plan から導出されるため、タスク表が不完全になる。`cargo test --workspace` 緑化は kiro-complete の DoD Gate であり、未計上のまま着手すると host32 クレートのテストコンパイルが落ちる。本 spec は roadmap W1「契約正本の先鋒」＝後続 spec が rebase する土台であり、爆風の過少申告は下流の見積りを直接歪める。

**Suggestion**: 変更ファイル表へ `shiori-host32-host/tests/{shiori_request_e2e,lifecycle_kill_e2e,lifecycle_cyclic_e2e}.rs`（各 `None` 追加）を追記し、末尾の機械的追随注記の適用範囲を `ShioriCall`／`ShioriRequest`／`Shiori3Client` の3型へ拡張する。いずれもコンパイラ捕捉・純機械作業ゆえ**是正コストは表への追記のみ**。

**Traceability**: Req 2.3・5.3（wire 層の Status 発行と観測）
**Evidence**: design.md §File Structure Plan「変更ファイル」表・同 §Risks & Open Items「4クレート横断の破壊的変更」行

### 🔴 Critical Issue 2: 檻違反を `ShioriFailure::Shiori` へ写すと区別語彙が汚染される

**Concern**: ID ホワイトリスト違反時に `ShioriOutcome::Failed(ShioriFailure::Shiori(..))` を返す設計だが、`ShioriFailure::Shiori` は `msg.rs:117-119` で「SHIORI **エラー応答**」と定義され、`real.rs:86-93` の `map_error` が `RequestError::Shiori` から**機械的に写す**ためだけの variant である。檻違反時は**リクエストが送出されていない**＝SHIORI 応答は存在しないため、これは範疇錯誤であり、`completed/areka-P0-kanade` Req6.1 が意図的に保存してきた区別語彙（Handshake/Timeout/Ipc/Shiori）を壊す。

**Impact**: 診断の誤帰属。areka 内部の実装バグ（events.rs へ ID を足して `ALLOWED_EVENT_IDS` を忘れる等——`areka-P0-input-events` が最初に踏む導線）が「pasta が SHIORI エラーを返した」として観測される。しかも当該経路は `to_unloading_fault` 経由で**ゴースト全体を落とす**ため、誤帰属のコストが最大化する地点でもある（fail-fast 自体は設計の明示的選択であり争点にしない）。

**Suggestion**: `ShioriFailure` へ `Internal(String)`（内部規律違反・境界を跨がない）を1 variant 追加し、檻違反はそこへ写す。`map_error` は `RequestError` からの機械的写像という不変条件を保ったまま、`Internal` は kanade 内部でのみ構成される。状態機械側の扱い（fault 終端）は変更不要ゆえ差分は小さい。

**Traceability**: Req 3.1・3.2（ホワイトリスト檻）
**Evidence**: design.md §Error Handling「Error Categories and Responses」表 1行目・§actor.rs egress チョークポイント Service Interface doc

### 🔴 Critical Issue 3: `snapshot_of(phase: &Phase)` は宣言済みシームを受け取れない

**Concern**: 設計は `fn snapshot_of(phase: &Phase) -> ExecutionSnapshot`（`&Phase` のみを入力）を置く一方、`ExecutionSnapshot` の SEAM 注記は将来フィールドの**源＝窓 geometry（UI スレッド）・運搬＝Tick 付帯**と明記する。`minimizing`／`balloon`／`opening` および Ref1/Ref2（見切れ・重なり）は**運行 Phase から導出不能**であり、シーム発動時には `snapshot_of` の**署名自体の変更**が要る。ゆえに Req1.6／2.5/2.6 が謳う「フィールドを1本足すだけ・送出契約不変」は、`snapshot_of` の口においては文字どおりには成立しない。併せて `ExecutionSnapshot::INACTIVE` が phase 非依存の**第2の構成経路**を与えるため（M1 は `snapshot_of(非 Steady{Some}) == INACTIVE` ゆえ等価だが）、非 phase 由来状態の着地後に boot/close 系列で黙って誤値を生む余地が残る。

**Impact**: M1 の正しさには影響しない（実導出は `talking` のみ・源は Phase）。影響は将来の差替コストと、DD-IT-4 が掲げる「例外のない単一規則」の実効性に限られる。

**Suggestion**: DD-IT-4 か `snapshot_of` の注記へ一行、**シームは「フィールド追加」だけでなく「`snapshot_of` の入力追加（Tick 付帯の受領）」を含む**旨を明記する（例: 将来形 `snapshot_of(&Phase, &TickExtras)`）。併せて `INACTIVE` の用途を「Phase を持たない構築点専用」と限定するか、boot/close も `snapshot_of(&state.phase)` へ一本化して構成経路を1つに畳む。設計文の明確化で足り、実装差分は不要。

**Traceability**: Req 1.6・2.5・2.6（実測差替シーム・契約不変）
**Evidence**: design.md §status.rs State Management（`ExecutionSnapshot` の SEAM コメント・`INACTIVE`）・§File Structure Plan `schedule/mod.rs` 行・§設計判断 DD-IT-4

## Design Strengths

1. **主張が一次証拠で裏打ちされている（要件の推奨すら実測で覆した）**: DD-IT-7 は要件 3.1 の推奨チョークポイント（`handle_call`／`run_shiori_loop`）を、「統合 mock は `ShioriMsg` 層で shiori アクターごと差し替わるため `handle_call` を通らない」という**実測**に基づいて `round_trip_request` へ変更した。本レビューが独立に再確認したところ、この反証は正しく、`round_trip_request` こそ本番・mock 双方が必ず通る唯一点である＝Req3.1 の規範節「全 `ShioriCall` 構築点を被覆」を真に満たす。DD-IT-5/6（空集合→行省略・Status の wire 位置）も実 SSP 2.3.86 捕獲ログで裏取りされ、さらに pasta の消費側コード（`virtual_dispatcher.lua:98,123` の完全一致比較）まで読み込んで Ref1/Ref2 固定 `"0"` の無害性を確認している。伝聞や fixture 推測に依らない検証姿勢は本プロジェクトの steering（正典は ukadoc・emo2 は聖典でない）と完全に整合する。

2. **不整合クラスを型で消し、縮退を正しい4点セットで行っている**: DD-IT-3 の単一 `ExecutionSnapshot` は Ref3 と `Status.talking` の共通の源となり、「Ref3=`"1"` かつ `Status: talking`」という不整合を**表現不能**にする。加えて M1 の縮退は `defer-canon-with-full-vocabulary-and-tracking-spec` の要求どおり、①語彙は全10状態を第一級保持 ②源のある `talking` のみ実導出 ③残は非アクティブ縮退＋差替シーム ④追跡 spec（`areka-P0-status-execution-states`／`areka-P0-choice-select-events`）＋Revalidation Triggers 登記——を満たす。消費側 fail-open（DD-IT-9）という**自分に不都合な発見**を隠さず Risks と Req2.6 ただし書きへ登記し、解禁条件（複合値 wire での消費側互換検証）を所有者へ申し送った点は特に誠実である。

## Final Assessment

### 判定: **GO**

**Rationale**: 既存アーキテクチャとの不整合はなく（境界・依存方向・純粋状態機械・汎用 wire codec 原則をすべて保存）、要件 1〜6 の全 AC が Traceability 表で構成要素へ写像され、実装経路は File Structure Plan まで具体化されている。主要判断は実コード・実 wire・消費側コードで一次裏取り済みで、本レビューの独立再実測でも齟齬が出なかった。指摘3件はいずれも**表への追記・variant 1本・注記1行**で閉じる精緻化であり、着手を止める性質のものではない（Issue 1 のみ tasks 生成前の反映が望ましい）。

### Next Steps

1. **Issue 1 を tasks 生成前に反映**（変更ファイル表へ host32 e2e 3ファイルを追記・機械的追随注記を3型へ拡張）——タスク表の完全性と DoD Gate（`cargo test --workspace`）に直結するため優先。
2. Issue 2・3 は設計ディスカッションで是非を確認（`ShioriFailure::Internal` の追加可否／`snapshot_of` シーム注記）。いずれも却下しても M1 の正しさは損なわれない。
3. 上記の後 `/kiro-spec-tasks areka-P0-idle-talk` でタスク生成へ進む。
