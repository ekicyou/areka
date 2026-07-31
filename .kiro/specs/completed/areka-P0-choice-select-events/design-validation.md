# 設計バリデーションレポート — areka-P0-choice-select-events

> 実施: 2026-07-31（`kiro-validate-design`・非対話・subagent）
> 対象: `design.md`（2026-07-31 生成）／入力: `requirements.md`（Req1〜9）・`research.md`（DD-1〜DD-16）・`brief.md`・`.kiro/steering/`
> 検証方法: design.md の主張をコードベース実測（Grep/Read）で突合。行番号アンカー・型シグネチャ・既存アームの挙動を全点再確認した。

## 検証実測サマリ（design.md の主張 vs 実コード）

- **アンカー全数一致**: `ChoiceSelection`/`ChoiceSelectionInbox`（balloon.rs:43/:130）・`sakura: Sender<StartTalk>`（actor.rs:60）・execute-batch/reinject-last drive ループ（actor.rs:96-168）・egress チョークポイント（actor.rs:180-206・`is_allowed_event_id ∨ is_allowed_resource_id`）・`DispatcherMsg` 4 アーム＋`on_start` の同期 join（dispatcher.rs:99-127/:182-198）・stale Done 棄却（:131-149）・`Failed`→`Unloading{Fault}` 横断アーム（mod.rs:317-323）と prefetch 先行アーム前例（:313-315）・`awaits_reply` に `Steady` を含む（mod.rs:374-384）＝DD-12 の先行アーム挿入位置は成立・`on_resolve_choice` の即 settle（drive.rs:364-419）・`WaitingForChoice` 早期 return による schedule 側タイムアウト seam の死亡（runtime.rs:184-187 — 「死んだ seam」判定は正確）・`TimedSchedule.start_time/horizon`（schedule.rs:49/:74）＝`occupancy_horizon()` getter は実現可能（`CuePlayer::current_barrier()` は :372 で既に公開済み）・compile の `WaitForChoice{timeout:None}` 末尾 1 個 append（compile.rs:211-223）・steady origin リテラル match（steady.rs:190-212）・`ALLOWED_EVENT_IDS` SEAM（events.rs:54-68）・status.rs 導出表 choosing SEAM（:170）と `snapshot_of` 署名拡張 NOTE（:209-219）。
- **DD-1〜DD-16 全数決着**: design.md「設計裁定一覧」に 16 項全て裁定済み（silent drop なし）。DD-2 は要件フェーズ決着（Req2.9）の実装形のみ、DD-16 は縮退なし単一 spec 実装を明示。
- **要件トレーサビリティ完全**: Req1.1〜9.4 の全 44 受入条件が Traceability 表でコンポーネント／フローへ対応付く（欠落なし）。Req6.4/6.5 の「NOTIFY は Value を運べない型」による構造充足・Req6.3 の既存 `canonical_index` 充足も実コードと一致。
- **決定論檻**: Req9.2 (a)〜(e) が Testing Strategy に 1:1 で対応。カスケード段・タイムアウト・choosing 導出・stale 棄却・DD-12 檻・dispatcher 換算檻・dola getter 檻・sakura 通知檻を網羅。ただし (e) の一部に構成不能な檻がある（下記 Critical Issue 2）。

## Critical Issues

🔴 **Critical Issue 1**: DD-4/F1 の FIFO 決定性主張に残余レースがある（`unknown_talk_done` error! が正常系で発火し得る）
**Concern**: F1 の注記「旧 talk の `Done` は必ず `Start` より後に dispatcher inbox へ並び…kanade に旧 talk の `TalkDone` は届かない」は、Close 起因の `Interrupted`（`on_start` 内の同期 join 前に enqueue 済み）には成立するが、**resolve 起因の即時 `TalkDone{Ended}`**（drive.rs:372-376 の即 settle）には成立しない。`Done{Ended}` は talk アクタースレッドから投函されるため、relay スレッドが `ResolveChoice` と `Start` の 2 send の間で停滞すると（Defender 再スキャンによるスレッド飢餓の実績がある環境）、inbox 順が `[ResolveChoice, Done{Ended,old}, Start]` になり得る。このとき dispatcher は slot 未差替ゆえ `Done{Ended,old}` を kanade へ転送し、kanade は `unknown_talk_done`（**error! レベル**・mod.rs:285-289）を記録する。状態は防御破棄で無傷だが、正常系ユーザー操作で error ログが非決定的に発火する。
**Impact**: log-first 規律（error=真の欠陥）の汚染・実機サインオフのログ grep 判定（Req9.3）のフレーキー化・DD-4 の採用根拠「全順序が決定的」の過大主張。スレッドスケジューリング依存ゆえ檻で決定論的に再現できない＝設計で塞ぐしかない。
**Suggestion**: (i) kanade の choice 差替時に旧 talk_id を帳簿（`ChoiceState` 掃除規則）へ 1 世代保持し、当該 id の遅延 `TalkDone{Ended|Interrupted}` を info（stale）へ降格する防御アームを遷移規則 7 に追記する、または (ii) F1 注記の主張を「届き得るが防御破棄される（info）」へ訂正し `unknown_talk_done` の語彙を choice 差替直後のみ info 化する。いずれも tasks へ 1 項目追加で足りる。
**Traceability**: Req4.3/4.6（slot 調停）・Req1.6（ログ規律）・Req9.3（ログ grep サインオフ）
**Evidence**: design.md「F1: OnID 形選択の happy path」順序の決定性注記・「設計裁定一覧」DD-4／実測: dispatcher.rs:131-149・mod.rs:285-289・drive.rs:372-376

🔴 **Critical Issue 2**: 檻 9.2(e)「カスケード中の二重注入棄却」は actor ハーネス経由では構造的に構成不能
**Concern**: 設計自身が明記するとおり、カスケードは 1 つの drive 呼出内で**同期完結**する（execute-batch/reinject-last・mock SHIORI も同期応答）。ゆえに `ChoicePhase::Cascading`/`TimeoutInFlight` は inbox メッセージ境界を跨いで観測されず、Testing Strategy (e) の「カスケード中の二重注入棄却」（`choice_rejected_busy`）を `Harness` 注入で作ることはできない。当該分岐は防御アームとして正当だが、檻の書き方が未指定のまま tasks へ流れると「檻が書けない」で実装フェーズが空転するか、分岐が未検証のまま残る（Req9.1「全ての判断分岐」違反）。
**Impact**: Req9.1/9.2(e) の網羅主張と檻の器（Harness 前提）の不整合。決定論テスト必達規律（deterministic-test-coverage-mandate）の穴になる。
**Suggestion**: Testing Strategy に「(e) の busy 棄却は `step()` 直呼びの純関数檻（`State` に `ChoicePhase::Cascading` を直接構成して `Input::Choice` を与える）で固定する」と 1 行明記する。step は公開済みの純関数であり既存檻流儀（判断分岐のみ檻・配線は再テストしない）とも一致する。
**Traceability**: Req9.1・Req9.2(e)・Req1.1
**Evidence**: design.md「System Flows F1」同期完結注記・「Testing Strategy」Integration Tests (e)・「C4 遷移規則」1／実測: actor.rs:96-168（drive ループ）

（3 件目に相当する重大欠陥は検出されなかった。`ActiveTalk.script` 追加に伴う boot/close 側構築点の適応漏れはコンパイラが強制するため機械的・Modified Files の「既存檻の機械的適応」行に包含とみなす。）

## Design Strengths

1. **kanade 単一調停＋既存同期ループへの相乗りが一回性を構造保証する**: カスケード全段が 1 drive 内で同期完結するため「段の途中に別入力が割り込まない」が機構でなく構造で成立し、Req1.1/4.6/5.4 の高々 1 回性が状態フラグに依存しない。既存 SEAM コメント（events.rs:54-58・status.rs:170・contract.rs:38）が全て本形を前提に申し送り済みで、実測でも全アンカーが設計の記述どおりだった（陳腐化ゼロ）。
2. **裁定の完全記録と境界規律**: DD-1〜16 の全裁定に根拠と却下案が併記され、正典裁定 8 項が provenance 3 値（ukadoc/ssp_secondary/areka_discretion）で対応表へ転記される設計。タイムアウトの二重権威禁止（schedule 側死 seam の不使用明記）・compile 無改変（time-directives との境界明示）・Req4.5 の prefetch 同型先行アームなど、隣接 spec との境界が全て文書化されている。

## Final Assessment

**Decision: GO**

**Rationale**: 既存アーキテクチャとの整合は実測で全点確認でき、要件网羅・裁定記録・決定論檻の設計はいずれも実装可能な水準にある。検出した 2 件はいずれも局所的な主張訂正／檻の書き方の明記であり、アーキテクチャ変更を要さず tasks フェーズの項目追加（Issue 1 の防御アーム＋Issue 2 の step 直呼び檻の明記）で吸収できる。

**Next Steps**:
1. 設計ディスカッションで Issue 1 の対処方式（防御アーム追加 (i) か主張訂正＋ログ降格 (ii) か）を裁定し design.md へ反映する。
2. Issue 2 の檻方式（step() 直呼び純関数檻）を Testing Strategy へ 1 行追記する。
3. その後 `/kiro-spec-tasks areka-P0-choice-select-events` でタスク生成へ進む（DD-16 の 3 段順序指針に従う）。
