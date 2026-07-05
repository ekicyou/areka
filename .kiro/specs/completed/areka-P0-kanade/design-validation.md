# 設計バリデーションレポート: areka-P0-kanade

- **実施日**: 2026-07-05
- **対象**: `design.md`（design-generated・確定版）／`requirements.md`（承認済・不変）
- **手法**: design-review.md プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）＋実コード・ukadoc 実文との突合（非対話・レポートのみ）

---

## 検証ログ（主張と実体の突合）

| design の主張 | 検証結果 |
|---|---|
| `areka-actor`: `spawn_actor(name, body) -> (Sender<M>, ActorHandle)`・`run_inbox`（handler `Err`→error!＋継続）・Close 即時 Break／全 Sender drop 正常終了・`reply_channel() -> (ReplySender, ReplyReceiver)`・`ReplyError::{Dropped, Timeout}`・`ActorHandle::join/is_finished` | ✅ **一致**（`crates/areka-actor/src/spawn.rs`・`reply.rs` 実読）。`ReplySender::send` は self consume＝応答高々 1 回、`recv` は Sender drop で `Dropped`（永久ブロックなし）——design の「宙吊りなし」保証の根拠が実在 |
| `shiori-host32-host`: `Shiori3Client::get(id, &[String]) -> Result<Option<String>, RequestError>`（200→Some／204→None）・`notify -> Result<(), RequestError>`（同期往復・応答破棄）・`RequestError{Handshake, Timeout, Ipc, Shiori}` | ✅ **一致**（`client.rs`・`error.rs` 実読）。`ShioriFailure` 4 語彙への機械的写像は 1:1 で成立。`ParentMessageWindow` が `!Send`＝専有スレッド前提も rustdoc に明記済み |
| `ShioriConnection` の `ParentMessageWindow`・`HelperHandle` | ✅ 両型とも `lib.rs` で公開済み（`pub use`） |
| ukadoc: OnSecondChange（Ref3=talk 可否・再生不能時 NOTIFY＋Ref3=0・返却スクリプト無視） | ✅ **実文一致**（`ukadoc:list_shiori_event:OnSecondChange:1`）——DD-6 の調停規則の正典根拠は正確 |
| ukadoc: OnClose（Ref0=user/system・OnCloseAll 204 後に続けて発生） | ✅ **実文一致**——「正典は OnCloseAll→(204)→OnClose」という DD-11 の差分認定も正確（design は差分を隠さず Revalidation Trigger 化） |
| ukadoc: `\-`＝本体終了・スクリプト再生後に終了 | ✅ **実文一致**（`ukadoc:list_sakura_script:_5c-:1`）——TalkDone{quit} 消費（再生完了後に終了）の意味論と整合 |
| 内部整合: 状態機械図 ⇔ Reference 表 ⇔ mock fixture ⇔ Req 1.6/3.4/4.x | ✅ 概ね整合。OnFirstBoot(Ref0="0")→204→OnBoot の固定運行（Req 1.6）が図・表・fixture・full_run シーケンスの全てで一貫。quit=true 全 Phase 横断遷移（Req 4.3）・pump ゲート復帰（Req 3.4）も図と補足に反映済み。残る曖昧点は下記 Critical Issues |

---

## Review Summary

設計は上流 2 クレートの実 API・ukadoc 実文の両方と正確に一致しており、机上の空論がない。DD-2（a-2 の Sender 循環欠陥の発見→a-1 採用）に代表される構造保証の詰めが高品質で、正典差分（close 順序）も隠さず DD-11 として管理している。残る指摘は実装着手前に明確化すべき記述上の曖昧点であり、アーキテクチャ上の欠陥ではない。

## Critical Issues

🔴 **Critical Issue 1**: `State` 型が未定義（Phase 外の帳簿の置き場が曖昧）
**Concern**: `step(state: State, ...) -> (State, Vec<Action>)` が唯一の遷移入口だが、design は `Phase` のみ定義し `State` 本体を定義していない。boot 中の `pending_close`（Boot* variant はフィールドなし）、Req 4.7 の deadline 計算基準「CloseTalkWait 進入時の直近 Tick 時刻」（Tick 未受領で close に入った場合の初期値含む）、talk_id 採番カウンタの置き場が宙に浮いている。
**Impact**: 実装時の解釈ブレ・tasks 分割の境界曖昧化。特に「Tick 未受領時の deadline 基準」は close_test の期待値に直結する。
**Suggestion**: tasks 生成時に `State { phase, last_now: Option<MonotonicMs>, next_talk_id, pending_close }` 相当のフィールド構成と「Tick 未受領時は最初の Tick 受領時点から期限起算」等の規則を task 記述へ明記する（design.md は不変のまま実装指針で吸収可能）。
**Traceability**: Req 4.7・4.2・3.2
**Evidence**: design.md「schedule 純粋運行状態機械」（step シグネチャ・Phase 定義）・System Flows 補足（pending_close）

🔴 **Critical Issue 2**: Req 3.1 の文言と DD-6 精密化の緊張（GET/NOTIFY の切替）
**Concern**: Req 3.1 は「定常運転状態＋Tick → OnSecondChange を **GET** として発行」と無条件に読めるが、design は active talk 中は **NOTIFY（Ref3=0）** で発行する（DD-6・ukadoc 正典どおり）。talk 重複調停は要件の design 送り事項であり消化として正当だが、「定常運転状態」＝「talk 非再生中」という要件解釈は明文化されていない。
**Impact**: 検収時に Req 3.1 の字面と steady_test の期待値（talk 中 NOTIFY Ref3=0）が食い違って見え、レビューで手戻りし得る。
**Suggestion**: design 議論で「Req 3.1 の『定常運転状態』は talk 非再生中を指し、talk 再生中は DD-6（正典 Ref3 意味論）が適用される」ことを合意事項として記録する（要件改稿は不要）。
**Traceability**: Req 3.1（vs Boundary Context「design 送り事項: talk 重複時の調停規則」）
**Evidence**: design.md DD-6・「pump ゲート（steady.rs）」・ukadoc OnSecondChange 実文（検証ログ参照）

🔴 **Critical Issue 3**: ForceQuit 遅延（最大 60s）と OS シャットダウン猶予の乖離
**Concern**: DD-2 のトレードとして、実経路で in-flight SHIORI 呼出中の ForceQuit は `AREKA_SHIORI_REQUEST_TIMEOUT_MS`（既定 60s）まで処理が遅延し得る。OS シャットダウンの実猶予（通常数秒〜20s 程度）を超え得るが、結線側（app-shell/ghost-setup）への具体的な申し送り（shutdown 時の timeout 短縮・プロセス kill 容認等）が Supporting References に含まれていない。
**Impact**: M1 の mock 観測では顕在化しないが、実運用の shutdown 体験・データ保全（将来の position-persist）に影響する設計負債が暗黙化する。
**Suggestion**: 「OS 側の最終強制力に委ねる」の割り切りは妥当として維持しつつ、ghost-setup への申し送り事項（shutdown 経路では短い timeout 構成 or kill 容認）を design 議論で明文化し、ghost-setup brief へ転記する。
**Traceability**: Req 4.4
**Evidence**: design.md DD-2「トレード」・§Risks「ForceQuit 遅延」・Supporting References（申し送りは Tick 供給のみ）

## Design Strengths

1. **上流実 API との厳密整合＋構造保証の質**: 全消費シンボルが実コードと一致（本レポート検証ログ）。特に a-2（応答回送）案の Sender 循環＝Req 4.9 恒久不成立をレビューゲートで発見し a-1（oneshot 同期往復）へ反転した経緯（research §10）は、「全 Sender drop→正常終了」を規約消費だけで構造的に成立させており、テスト不能な宙吊りリスクを設計段階で根絶している。
2. **正典の単一正本化と差分の可視化**: Reference 表→`events.rs`→mock fixture→ハーネス assert の三点一正本（Req 7.1 の導出構造）により、正典解釈のズレが一点修正で全体に波及する。ukadoc 実文との spot-check は全て一致し、唯一の意図的差分（close 順序）も DD-11 として隠さず Revalidation Trigger 化されている。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ整合（areka-actor 規約消費・host32 依存の単一ファイル隔離・tokio 不使用）・要件網羅（Traceability 表で Req 1〜7 全 AC が実装点に写像済み）・実 API/正典との一致が確認でき、実装経路は明確。3 件の指摘はいずれも記述の明確化・申し送りの明文化であり、設計の骨格を変えるものではない（tasks 生成時と design 議論で吸収可能）。

**Next Steps**:
1. design 議論（kiro-design-discussion）で Critical Issues 1〜3 を確認・合意事項として記録
2. `/kiro-spec-tasks areka-P0-kanade` で実装タスク生成（Issue 1 の State フィールド構成・deadline 初期値規則を task 記述へ反映）
