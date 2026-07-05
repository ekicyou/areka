# 設計検証レポート: areka-P0-sakura-engine（検証 2 回目・改訂 2 対象）

> 対象: 確定済み `requirements.md`（R1〜R11）＋ **design.md 改訂 2（dola cue ドメイン基盤への全面再設計・DD-1..11）**。
> 検証種別: 実装前 設計品質レビュー（非対話・GO/NO-GO 判定）。前回レポート（初版 design 対象・NO-GO）は本レポートで**全面置換**。
> 検証日: 2026-07-05 ／ 検証者: kiro-validate-design（subagent・非対話）
> 実シンボル照合: `dola/src/cue/{schedule.rs, command.rs, sheet.rs}`・`wintf/src/ecs/cue/queue/mod.rs`（＋tests 6 ファイル）・`areka-actor/src/{spawn.rs, reply.rs}`・`areka-parsers/src/sakura/model.rs` を直読し、散文主張を全件裏取りした。

---

## 設計レビュー要約

改訂 2 は「compile（純粋）／drive（per-talk アクター）／sink（trait＋mock）」の三層を dola cue ドメイン上に再構築し、前回 NO-GO の Critical Issues 1〜3 を **DD-11（`TalkHandle{inbox, actor}`＋Start 自己投函）・DD-10（`Tick` を inbox メッセージ化＋二重発火ガード）・move-consume 一本化**で全て解消している（下記「前回イシューの解消確認」）。dola 採用の根拠（`ActorKey` doc の `\0`/`\1` 言及・`CueTarget::{Shell,Balloon}`・wintf 配送半身の稼働）、`compile_sheet` 不採用の理由（min 正規化が先頭 `\w` を潰す）、DD-9 touch point（wintf の match は `other =>`/`_ =>` catch-all のみ・exhaustive match 不在）は**すべて実ソースと一致**した。残る指摘は 3 件とも局所的な文言確定レベルで、アーキテクチャの妥当性を損なわない。

### 前回イシューの解消確認（旧 design-validation 対象・実シンボル照合済み）

| 旧 Issue | 判定 | 根拠 |
|---|---|---|
| 1. `spawn_talk` が Close 配送端を捨てる | **解消** | `TalkHandle{inbox: Sender<SakuraMsg>, actor: ActorHandle}` は `spawn_actor -> (Sender<M>, ActorHandle)`（`spawn.rs` L39）と型整合。`Start` は spawn 直後に自己投函＝単一 inbox の全順序で「Start 先行・Close/Tick 順序確定」が成立 |
| 2. 注入時刻と Close 即時性の両立が未規定 | **解消** | 時刻自体を `SakuraMsg::Tick(f64)` として inbox に載せ、正準ループを `run_inbox`（`rx.recv()` 単一待機）に一本化＝二重待機が消滅。`TimedSchedule::tick` の冪等早期 return（`schedule.rs` L144: `offset <= current_offset && !ready_buffer.is_empty()` で **ready_buffer を保持したまま return**）が同時刻再 tick 後の `ready()` 再読で二重送出を招く点まで正しく特定し、駆動層の単調ガードで遮断している（ただし下記 Issue 2 の初期値未規定に注意） |
| 3. 高々 1 回機構の曖昧さ（フラグ併記） | **解消** | `ReplySender::send(self)`（`reply.rs` L38・consume）を唯一機構とし「終端済みフラグは持たない」と一本化。`Option<TalkState>` の take は所有権移動でフラグでない旨も明記。Break 後はスレッド消滅ゆえ構造的に再返信不能 |

---

## Critical Issues（最大 3・いずれも文言確定レベル）

### 🔴 Critical Issue 1: EmptyEnd 経路が `TalkDone{Ended}` 固定で、`\-` 単独 script（sheet 空＋`end=Quit`）が R6.2 に違反する

- **Concern**: System Flows は「compile 結果の `sheet` が空なら時間軸駆動せず即 `TalkDone{Ended}`→`Break`」、状態遷移図も「EmptyEnd --> Done: TalkDone Ended」と **reason を Ended に固定**している。しかし compile の Postconditions は「`end` は `Ended` か `Quit` のみ」であり、script が `\-` 単独（または `\w` 系のみ＋`\-`）の場合 **sheet 空＋`end=Quit`** が生成される。この経路で Ended を返すと R6.2（`Quit`→`TalkDone{Quit}`）に違反する。
- **Impact**: kanade の close 握手が quit 意図を取りこぼす（quit はゴースト終了系の運行判断に直結）。設計文書内の自己矛盾のまま実装へ渡すと、状態遷移図に忠実な実装が AC 違反を作り込む。
- **Suggestion**: EmptyEnd の送出を `TalkDone{reason: compiled.end}` に改める（R1.4 の「空 script/空列→Ended」は compile が空入力に `end=Ended` を返すことで自然に満たされる）。状態遷移図の注記も「TalkDone(end)」へ修正。
- **Traceability**: R6.2 ／ R1.4。
- **Evidence**: design.md「System Flows > 再生駆動と終端・中断のライフサイクル」（EmptyEnd 注記・状態遷移図）と「compile > Postconditions（`end` は `Ended` か `Quit` のみ）」の突合。

### 🔴 Critical Issue 2: 単調 Tick ガードの「直前値」の初期値が未規定——初期値 0.0 だと最初の `Tick(0.0)` が no-op になり at=0 発火が飲まれる

- **Concern**: 駆動層ガードは「直前 tick 値を保持し `t <= 直前値` の Tick は no-op」と定義されるが、`TalkState.last_tick` の**初期値（型）が未規定**。素朴に `f64 = 0.0` で初期化すると、契約上正当な最初の `Tick(0.0)`（テスト注入列は「0.0→…」と明記）が `0.0 <= 0.0` で no-op となり、`start_time=0.0` の cue（`\w` を先行しない全 cue＝fixture の冒頭 Text/Surface）が発火しない。待ちを含まない script を `Tick(0.0)` 単発で駆動する場合は **`TalkDone` が永遠に返らず統合テストがハング**する。
- **Impact**: R9.3（fixture の期待発火列）・R9.1 の主検証経路が実装の初期値選択ひとつで壊れる、発見コストの高い罠。ガード自体は正しい（`schedule.rs` L143-145 の早期 return が ready_buffer を保持する以上必須）ため、初期値だけが穴。
- **Suggestion**: `last_tick: Option<f64>`（初期 `None`＝最初の有限 `Tick` は必ず有効）と design に一文固定する。統合テストに「`Tick(0.0)` 単発で at=0 cue が発火し完了する」ケースを追加。
- **Traceability**: R9.1, R9.3 ／ R2.1（at=0 の意味論）。
- **Evidence**: design.md「`Tick` の意味論（固定）」の冪等・逆行ガード段落＋「drive > 高々 1 回の唯一機構」の `TalkState{.., last_tick}`（型無記載）、および `dola/src/cue/schedule.rs` L143-145。

### 🔴 Critical Issue 3: 同時刻 cue の順序保存が `to_schedule` で未規定——`extend` を使うと同一 `at` の Text/NewLine 列が逆順配信される

- **Concern**: `\w` を挟まない連続命令（例: `Text→NewLine→Text→Surface`）は**全て同一 `start_time`** を持つ（さくらスクリプトの常態）。`TimedSchedule` は降順ソート＋末尾 pop であり、`insert()` は `partition_point` により同値オフセットの**既存要素より前**へ挿入するため 1 件ずつの挿入なら FIFO が保存されるが、`extend()` は push 後の安定降順ソートゆえ**同値グループが LIFO（逆順）で配信される**（`schedule.rs` L87-118 実測）。design の `to_schedule` は「`Entry::Payload(start_time, TalkCue)` を直接挿入する」とだけ述べ、**insert/extend の別も同時刻順序の不変条件も規定していない**。Invariants の「発火順は `at` 昇順」は同値時の順序を語らない。
- **Impact**: 実装が `extend` を選ぶと同時刻のテキスト断片・改行が逆順で emo text-layer へ届き、表示文字列が壊れる（R4.1/R4.2 の実質破綻）。同一 tick 内の `ready()` 順序は下流の唯一の順序情報であるため、決定性テスト（R9.3/R9.4）の期待値定義にも直結する。
- **Suggestion**: 「`to_schedule` は `CueSheet::cues()` の並び順（compile の script 出現順）に **1 件ずつ `insert()`** し、同一 `at` の cue は script 出現順（FIFO）で配信される」を Invariant として明文化。単体/統合テストに「同時刻の Text/NewLine/Text が出現順で届く」ケースを追加。
- **Traceability**: R4.1, R4.2, R2.5, R9.3, R9.4。
- **Evidence**: design.md「drive > Service Interface > `to_schedule`」「Invariants（発火順は at 昇順）」と `dola/src/cue/schedule.rs`（`insert` L87-99 / `extend` L106-118: 安定ソート＋末尾 pop の同値逆順）の突合。

---

## 設計の強み（1〜2）

- **「配送側半身の再利用」というピボットの核が実ソースで完全に裏付けられている**: `ActorKey` doc の `\0`/`\1` 言及、`CueTarget::{Shell, Balloon}` の消費区分、wintf `CueQueue`（`TimedSchedule<CueCommand>` 内包・Choice 先積み・バリア消費実装）まで、research §9 と design の引用が全て実物と一致。特に **`compile_sheet` の min 正規化が先頭 `\w` を潰す**という不採用理由（`sheet.rs` L104-115 で確認: `\w9テキスト` の 0.45s が 0s へ潰れる）と `CompiledCue` が actor/at を payload 外へ置く不適合の指摘は、安易な既存 API 流用を先回りで封じた高品質な実装前調査である。DD-9 の touch point 実測（wintf の match は全て catch-all・dola の 6 バリアントテスト/doc のみ要更新）も検証と一致した。
- **終端の高々 1 回が「型＋構造」の二重で閉じている**: `ReplySender::send(self)` の move-consume（型）に加え、全終端経路を「take → send → 直後 Break」の対とし Break＝スレッド消滅で終端後 Close の再返信を構造的に不能化。テスト計画（自然終端後 Close の send 失敗観測・冪等/逆行 Tick・非有限 Tick・先頭待ち保存・as_secs_f64 累積での期待値計算）が設計上の各ガードと 1:1 に対応しており、R6.4/R7.5 の検証可能性が高い。

---

## 最終判定

### 判定: GO（条件付き——上記 3 件を設計ディスカッション／design 微修正で確定してからタスク生成へ）

### 根拠
前回 NO-GO の 3 イシューは DD-10/DD-11/move-consume 一本化で**全て実シンボル整合の形で解消**され、dola 基盤化の中核主張（cue ドメイン写像・`compile_sheet` 禁止・DD-9 非破壊性・二重発火ハザード）は実ソース照合で全件裏付けられた。残る 3 件は EmptyEnd の reason 固定・ガード初期値・同時刻順序という**いずれも 1〜2 文の設計文言確定で閉じる局所欠陥**であり、アーキテクチャ・境界・要件被覆（R1.1〜R11.4 全 ID がトレーサビリティに出現・写像表で R3.2/R4.2/R5 の実現形まで具体）に構造的欠陥はない。

### Next Steps
1. `kiro-design-discussion` で Issue 1〜3 を確定する（①EmptyEnd→`TalkDone{compiled.end}`、②`last_tick: Option<f64>`＝初期 `None`、③`to_schedule` は cues 順の逐次 `insert()`＋同時刻 FIFO を Invariant 化）。
2. 反映後 `/kiro-spec-tasks areka-P0-sakura-engine` へ進む。compile 純粋層・contract 層は現状のまま実装着手可能。
