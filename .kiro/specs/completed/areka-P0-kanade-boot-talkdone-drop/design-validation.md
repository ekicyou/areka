# 設計検証レポート — areka-P0-kanade-boot-talkdone-drop

- 実施: 2026-09-17（kiro-validate-design・非対話・設計文書は変更しない）
- 入力: `spec.json`（language=ja・design generated / 未承認）・`requirements.md`（確定）・`design.md`・`research.md`・`brief.md`・steering（`product.md`／`tech.md`／`structure.md`／`logging.md`）
- 検証規律: 設計がコードについて述べる主張はすべて当該ファイルを開いて確かめた（下記 §1）。到達可能性は `mod.rs` → `boot.rs` の実経路を辿って確かめた。

---

## 0. Review Summary

設計は「`boot::step` にガード付きの `TalkDone` 腕 1 本と受理関数 1 本を足す」に閉じており、`mod.rs` の横断→委譲の順序規律・`boot_input_ignored` の単一発行点・`steady.rs`／`close.rs`／`actor.rs` 非接触という境界を守っている。要件 1.1〜5.6 の 25 件はすべて traceability 表に対応があり、RED テストは今日のコードで実際に赤になる経路を踏み、較正テスト T4 も実在のワイルドカード腕を駆動する。設計がコードについて述べた事実のうち 2 件（`boot.rs` の既存 import・`CloseReason` の比較）が実際と異なり、そのまま書くとコンパイルが通らないが、いずれも 1 行の是正で済む。

---

## 1. 設計のコード主張の裏取り（file:line）

| 設計の主張 | 確認先 | 結果 |
|---|---|---|
| `boot::step` の腕は `Boot`・`ShioriReply`・`CloseRequest` ＋ ワイルドカード（warn `boot_input_ignored`）で、`TalkDone` の腕が無い | `crates/areka-kanade/src/schedule/boot.rs` の `pub(crate) fn step`（27〜36 行） | ✅ 確認。ワイルドカードの注記は「Tick・TalkDone など」（32 行） |
| `current_talk_id` は `Steady{Some}`・`BootVersion{Some}`・`CloseTalkWait` を突合対象に含む | `schedule/mod.rs` の `fn current_talk_id`（681〜694 行） | ✅ 確認（688〜690 行に `BootVersion { talk: Some(active), .. }`） |
| `on_talk_done` は一致・`Ended`／`Interrupted` を `dispatch_phase` へ流し、`dispatch_phase` は `BootVersion{..}` を `boot::step` へ渡す | `mod.rs` の `fn on_talk_done`（539・543 行の `dispatch_phase(state, Input::TalkDone(done), config)`）・`fn dispatch_phase`（641〜646 行） | ✅ 確認。**受理腕は実際に到達する**（`Quit` は 521〜530 行で横断終端・委譲へ来ない） |
| `actor.rs` の `drive` は往復応答を同じ呼出の中で `Input::ShioriReply` として再投入し、往復の無い指示列が返るまで反復する | `crates/areka-kanade/src/actor.rs` の `fn drive`（154〜198 行・`loop` と `last_reply` の再投入） | ✅ 確認 |
| 受信箱は 1 メッセージにつき `drive` を 1 回呼ぶ | `crates/areka-actor/src/spawn.rs` の `run_inbox`（138〜150 行）・`actor.rs` の受信ハンドラ（101〜142 行・`KanadeMsg::TalkDone(td) => Input::TalkDone(td)` は 110 行） | ✅ 確認 |
| `to_baseware_version` は必ず `basewareversion` の往復を同じ指示列に積む | `boot.rs` の `fn to_baseware_version`（277〜280 行・無条件 `actions.push(Action::ShioriRequest(events::baseware_version(..)))`） | ✅ 確認 |
| `Phase` は `Debug` を持たない（`State`・`ActiveTalk` も） | `mod.rs` 91 行・122 行・168 行（いずれも derive なし） | ✅ 確認 |
| `Boot` → `Notified` → `NoContent` → `Value("greeting")` の 4 入力で `BootVersion{Some(id=1)}` へ（`config()` は `first_boot: true`） | `msg.rs` の `KanadeConfig::new`（340 行 `first_boot: true`）・`boot.rs` の `on_reply`（58〜68 行 BootInit→BootPrefetch／`on_prefetch_reply` 189〜202 行 first_boot→BootType／88〜91 行 BootType の Value → `to_baseware_version`）・`mod.rs` 590〜592 行（BootPrefetch の応答は横断判定より先に `boot::step` へ） | ✅ 確認。`next_talk_id` は 1 起点（`State::initial`）ゆえ id=1 |
| 行数: `boot.rs` 307／`boot_sequence_tests.rs` 584／`schedule_log_firing_tests.rs` 626／`boot_test_support.rs` 46 | `wc -l` | ✅ 一致。増分見積もり（+23／+136／+19／+34）でも 1,000 行に遠い |
| `boot.rs` は `crate::talk::TalkDone` を**既に import している** | `boot.rs` 11 行 `use crate::talk::{StartTalk, TalkId};` | ❌ **不一致**。`TalkDone` は import されていない（→ 重要事項 1） |
| ヘルパは戻る前に `pending_close == pending` を表明する | `msg.rs` 30〜31 行 `#[derive(Debug, Clone, Copy)] pub enum CloseReason` | ❌ **不一致**。`CloseReason` は `PartialEq` を持たず `Option<CloseReason>` の `==` はコンパイルできない（既存テストは `matches!`／`is_none()` で書いている・`boot_sequence_tests.rs` 358 行）（→ 重要事項 1） |
| `logged_once(..).fields["talk_id"] == "1"` で値まで突合できる | `log_capture.rs` の `CapturedEvent.fields`（58 行）・既存の使用例 `steady_choice_timeout_tests.rs` 111 行 `fired.fields.get("talk_id").map(String::as_str)` | ✅ 可能（u64 は Debug 経路で `"1"` になる・`field_of` 74〜92 行） |
| `spine_conformance_support_tests.rs` の「残る危険（本檻では直せない）」段落が本欠陥を逐語登記している | 597〜603 行（`schedule/boot.rs:32-36` と `mod.rs:681-694` を指す） | ✅ 確認 |

### ガード付き腕のコンパイル可否
`Input::TalkDone(done) if matches!(state.phase, Phase::BootVersion { talk: Some(_) }) => on_talk_done(state, done)`
- `Input::TalkDone(TalkDone)` はタプル形（`mod.rs` 48 行）・`TalkDone { talk_id: TalkId, reason: TalkEndReason }` は `Copy`（`areka-talk/src/lib.rs` 111〜115 行）。
- `matches!` は `Some(_)` を束縛しないので `state.phase` を move しない。腕本体で `state` を move できる（既存の `Input::ShioriReply { outcome, .. } => on_reply(state, outcome, config)` と同型）。
- → **原理上コンパイルする**（`TalkDone` の import を足せば）。

### RED テスト（T1）が今日のコードで赤になるか
`BootVersion{Some(1)}` に `TalkDone{1, Ended}` → `mod.rs` 突合一致 → `dispatch_phase` → `boot::step` → ワイルドカード（33〜36 行）→ WARN `boot_input_ignored`・相は `BootVersion{Some}` のまま。
- ⑴ `matches!(phase, BootVersion{talk: None})` が偽 → **赤**。
- ⑵ `assert_not_logged("boot_input_ignored")` が WARN を検出 → **赤**。
- ⑶ `Notified` → `boot_complete` → `Steady{talk: Some(1)}`（`boot.rs` 109〜111 行の引き継ぎ）。
- ⑷ `CloseRequest{User}` → `steady::on_close_request` の `Steady{Some}` 枝（`steady.rs` 874〜877 行）→ `steady_close_pending`・指示 0 件 → `actions[0]` が範囲外で panic → **赤**。
- → 3 段で独立に赤。直した後は ⑴⑵ が緑・⑶ `Steady{None}`・⑷ `begin_close`（`steady.rs` 888〜902 行）で `OnClose` GET＋`ClosePending`。T2 の Tick 経路も `steady.rs` 692〜694 行（`Steady{None}` の Tick で `pending_close.take()` → `begin_close`）で成立する。

### 較正テスト（T4）が恒真でないか
`run_step(Phase::BootVersion{talk: Some(..)}, Input::Tick{..})` → `mod.rs` 434 行 `Input::Tick { now } => dispatch_phase` → `boot::step` → 受理腕は `Tick` に一致しない → ワイルドカード → WARN。`assert_logged(WARN, "boot_input_ignored")` は実在の腕を踏む。受理腕が `Tick` まで受理する退行（例: `_ if matches!(..BootVersion{Some}..)` のような書き方）で赤になる。**恒真ではない**（`capture` は捕捉が働かなければ panic する・`log_capture.rs` 131〜138 行）。

### 要件の確定前提との整合
設計 Overview「到達可能性」は要件 Introduction／Out of scope／5.5 と同じ結論（純粋状態機械の欠陥・アクター経由では到達不能・`-p areka-kanade` が欠陥の検出器・`-p areka --bin areka` は非回帰の検出器）。矛盾なし。

---

## 2. Critical Issues（最大 3）

🔴 **重要事項 1**: 設計が前提とするコードの事実 2 件が実際と異なる（そのまま書くとコンパイルが通らない）
- **Concern**: (a) Allowed Dependencies が「`boot.rs` が既に import している …`crate::talk::TalkDone`」と述べるが、`boot.rs` 11 行の import は `StartTalk, TalkId` のみで `TalkDone` は無い。(b) 共有ヘルパの仕様「戻る前に `pending_close == pending` を表明する」は、`CloseReason` が `PartialEq` を持たない（`msg.rs` 30 行 `derive(Debug, Clone, Copy)`）ためコンパイルできない。
- **Impact**: 影響は小さい（(a) は `use crate::talk::{StartTalk, TalkDone, TalkId};` の 1 語追加・(b) は `matches!` 2 本への書き換え）。だが設計文書を写して実装すると最初の `cargo test` がコンパイルエラーで止まり、「RED の記録」（要件 5.2）を取る前に実装者が設計から逸れる。
- **Suggestion**: (a) Allowed Dependencies を「`boot.rs` に `crate::talk::TalkDone` の import を 1 語追加する（新規の外部依存ではない）」へ。(b) ヘルパの表明を `matches!` で書く——`pending` が `Some(reason)` なら `assert!(matches!(state.pending_close, Some(r) if r.as_ref_str() == reason.as_ref_str()))`、`None` なら `assert!(state.pending_close.is_none())`（`CloseReason` に `PartialEq` を足す案は `msg.rs` を触るので採らない）。
- **Traceability**: 要件 5.1／5.3（ヘルパ）・5.2（RED 先行の記録）。
- **Evidence**: design.md「Boundary Commitments › Allowed Dependencies」・「Testing Strategy › 共有ヘルパ」。

（重要事項 2・3 は無し。要件 1.1〜5.6 に設計の対応が無いものは無く、到達しない腕・恒真のテストも無く、確定前提との矛盾も無い。）

---

## 3. Design Strengths

1. **到達可能性の議論が実コードで裏付いている。** `actor.rs` の `drive`（同期再投入）・`run_inbox`（1 メッセージ 1 `drive`）・`to_baseware_version`（無条件の往復）の 3 点を引いて「アクター経由では `BootVersion` を受信箱が観測しない」を示し、要件の確定前提（純粋状態機械の欠陥・検出器の区別）と一致している。Revalidation Triggers がこの 3 点の変化をそのまま再検証条件にしているのも良い。
2. **受理腕をガード形にして `boot_input_ignored` の発行点を 1 か所に保った。** 受理関数内に第 2 の防御腕を作らず、外れる `TalkDone` は既存のワイルドカードへ落ちる。これで完了仕様 e2e の点灯語の契約（発行点 1 か所）と log-first（沈黙の経路なし）が同時に成立し、T1（赤→緑）と T4（受理腕の広がり検出）が互いに補完する。

---

## 4. Final Assessment

**Decision: GO**（条件付き＝重要事項 1 の 2 か所をタスク生成時または実装時に是正する。設計の構造・境界・テスト計画には変更不要）。

**Rationale**: 設計の要である「`mod.rs` の突合済み通知が `boot::step` へ実際に届く」「今日のコードで RED が赤」「T4 が恒真でない」の 3 点をコードで確認できた。残る不一致は import 1 語と表明の書き方だけで、設計の判断を変えない。

**Next Steps**:
1. `/kiro-spec-tasks areka-P0-kanade-boot-talkdone-drop` でタスク生成。ヘルパのタスクに重要事項 1 (b) の `matches!` 形を、`boot.rs` のタスクに (a) の import 追加を書く。
2. 実装は設計「検証手順」1〜4 の順（テスト先行コミット→直し→2 コマンド全走→1,000 行の番人）。名前指定実行は `--lib` 必須。

---

## 5. Non-critical Observations（ディスカッションでの仕分け用）

1. **`spine_conformance_support_tests.rs` の `boot.rs` 行番号ドリフトは設計の列挙より多い。** 設計／research §9.2 は `:31`・`:285-288` のずれを挙げるが、同ファイル 566 行の `boot.rs:253-273`・663 行の `boot.rs:276-280` も腕の追加でずれる（挿入点が `step` 内＝ファイル先頭側のため、以降の行番号はすべて動く）。「何の定義行か」で指す規律の適用は本仕様の範囲外で良いが、列挙の網羅性だけ訂正しておくと後で探す手間が減る。
2. **T1 ⑷ の「直す前」の赤の出方。** 設計は「`steady_close_pending` で指示 0 件」と書くが、実際には `actions[0]` の添字が範囲外で panic して赤になる（同じ赤・失敗メッセージが変わるだけ）。RED の記録に残す文言はこの panic になる。
3. **T4 の `assert_not_logged("boot_talk_done")` は直す前は恒真**（発行点が無い）。較正の実体は WARN の存在側であり、直した後に初めて不在側が意味を持つ。設計の位置づけどおりで問題ないが、RED 記録で T4 を「緑」と書くときはこの理由を添えると誤解が無い。
4. **ガードを外した無条件受理の退行は検出器が無い**（設計 Risks に明記済み）。`mod.rs` の突合により到達不能なので害は無く、レビューでの形の確認に委ねる判断は妥当。
5. **既存テスト `boot_greeting_talkdone_correlates_without_unknown_error` の段コメントが古い**（「BootInit→BootType」と書くが実際は BootInit→BootPrefetch→BootType）。要件 4.5 により書き換えないのは正しい。新ヘルパの段コメントは実際の相で書くこと。
6. 検証手順 3 の「`crates/areka` の非回帰は単独で走らせる」（壁時計デッドラインの飢餓）は research §9.6 にあり design 本文には無い。タスクの検証欄に写すと良い。
