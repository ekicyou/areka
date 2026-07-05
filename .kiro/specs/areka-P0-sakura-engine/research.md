# ギャップ分析: areka-P0-sakura-engine

> 対象: 要件確定済み `requirements.md`（R1〜R11）に対する既存コードベースとのギャップ分析。
> 目的: 設計フェーズへ渡す実装戦略材料（複数案・トレードオフ・研究項目）を提示する。決定は下さない。
> 調査日: 2026-07-05 ／ 種別: 本坑（main・④ sakura 再生エンジン）

---

## 1. サマリ（3〜5 点）

- **上流契約はすべて実シンボルとして完成済み・非衝突の新設クレート**。`areka_parsers::sakura::parse(&str) -> Vec<Instruction>`（`Instruction` フラット enum・`Wait(Duration)` は 50ms 単位換算まで正規化済み）、`areka-actor`（`spawn_actor`/`run_inbox`/`reply_channel`／Close 即時停止規約）、`dola`（`DolaRuntime`／`TimedSchedule<T>` の `tick(current_time)`+`ready` 2 相 API・時刻注入式）が揃う。sakura エンジンはこれらを**消費するだけ**の greenfield クレート（`crates/areka-sakura` 想定）で、既存資産の改変を要さない。
- **中核は「純粋なタイムライン展開」（R2）で、既存の `dola::TimedSchedule<T>` が要件のほぼ全需に直接適合する**。`Entry::Payload(f64_offset, T)` の降順ソート＋末尾 pop、`tick(current_time)` で到達 payload を `ready_buffer` へ蒐集する形が、`Wait` を累積オフセットへ畳んで発火列を時刻順に出す R2/R9 の観測モデルと同型。ただし dola の時刻は **`f64` 秒**、`Instruction::Wait` は **`Duration`**——単位の橋渡しが設計判断になる。
- **上流契約（`StartTalk`/`TalkDone`）の型は本仕様には未着地——正本は kanade（並走・未実装）**。kanade クレートも `StartTalk`/`TalkDone` 型定義もリポジトリに存在せず、両ブリーフの散文で先決されているのみ。sakura は「消費・再定義しない」立場ゆえ、**型の物理的所在（kanade が未完なら暫定 owner をどこに置くか）が最大の未決事項**。
- **下流 2 分岐の出力契約（`SurfaceCommand` 級／`TextCommand` 級）は本仕様が正本だが、消費者（seriko/emo-text-layer）は未着手**。型定義の所在・`SurfaceArg` の不透明再輸出・`at`（時刻）の表現（`Duration` か `f64` か）を本仕様で確定する必要がある。
- **観測ハーネスは「実 kanade 不要・表示不要・時刻注入で決定的」（R9）——areka の既定規律に完全整合**。mock sink 2 本＋script 直入力で単一 pass/fail。実時間 sleep 非依存は `TimedSchedule`/`DolaRuntime` いずれの時刻注入 API でも構造的に成立する。

---

## 2. 要件 → 資産マップ（ギャップタグ: Missing / Unknown / Constraint / Reuse）

| 要件 | 技術的必要物 | 既存資産 | ギャップ |
|---|---|---|---|
| R1 talk 起動・script 受領 | `StartTalk{script, talk_id}` 受領、`parse` 呼出、talk_id の全出力への付与、空 script→即 `TalkDone{quit:false}` | `areka_parsers::sakura::parse`（実） | `StartTalk`/`talk_id` 型が未定義（正本=kanade 未実装）＝**Unknown**。parse 呼出自体は **Reuse** |
| R2 タイムライン展開（Instruction→時刻付き発火列） | `Wait(Duration)` を累積オフセットへ畳み、各発火へ相対時刻 `at` を付与、決定的 | `dola::cue::TimedSchedule<T>`（`Entry::Payload(f64,T)`＋`tick`/`ready` 2 相・降順ソート）／`DolaRuntime`（storyboard） | 展開ロジックは新規＝**Missing**。ただし配信エンジンは **Reuse 候補**。`Duration`↔`f64秒` 単位橋渡し＝**Unknown（設計判断）** |
| R3 surface 分岐（→seriko） | `SurfaceArg` を不透明のまま scope＋`at` 付きで別 sink へ | `SurfaceArg`（不透明 NewType・`as_str` のみ・実） | 出力メッセージ型 `SurfaceCommand` 級が未定義＝**Missing（本仕様が正本）** |
| R4 テキスト系分岐（→emo） | `Text`/`NewLine(ratio)`/`Clear` を scope＋`at` 付きで別 sink へ（字送りは持たない） | `Instruction::Text/NewLine/Clear`・`NewLineRatio`（実） | 出力型 `TextCommand` 級が未定義＝**Missing（本仕様が正本）** |
| R5 話者スコープの共通付与 | `SpeakerScope{n}` で現在 scope 更新、両 sink 発火へ有効 scope 付与、未指定は既定 scope | `Instruction::SpeakerScope{n}`（実） | scope 状態機械・既定値の型表現＝**Missing**。既定 scope の値（0 か）＝**Unknown** |
| R6 終端検出・`TalkDone` 返信 | `End`→quit:false／`Quit`→quit:true／末尾到達→quit:false、高々 1 回、終端後の命令破棄 | `Instruction::End/Quit`（実）／`reply_channel`（実・oneshot 相当で高々 1 回を型強制） | `TalkDone` 型が未定義＝**Unknown（正本=kanade）**。返信経路は `reply_channel` を **Reuse 候補** |
| R7 中断（Close） | 進行中再生の即時停止、残余 drain せず破棄、areka-actor 停止規約整合 | `areka-actor` の Close 即時停止規約（`run_inbox` の `Break`・実） | 状態機械での Close 割り込み結線＝**Missing**。停止規約自体は **Constraint（従う）** |
| R8 M-boot 外タグの寛容無視＋シーム | `Choice`/`Move`/`Cursor`/`SystemVar`/`GenericCommand`/`Raw` を実挙動なし＋ログ＋非 panic | 該当 `Instruction` variant（実・`#[non_exhaustive]`）／`tracing`（規約） | 無視分岐＋ログの実装＝**Missing**。拡張シームは enum の `#[non_exhaustive]` で **Reuse** |
| R9 決定的テスト・観測ハーネス | 時刻注入駆動、mock sink 2 本、fixture script 発火列/時刻/終端の決定性 | `TimedSchedule::tick(current_time)`／`DolaRuntime::tick`（時刻注入・実）／areka の決定的テスト規律 | mock sink・fixture・期待値表＝**Missing（新規テスト資産）**。時刻注入基盤は **Reuse** |
| R10 per-talk transient ライフサイクル | talk 起動時生成・終端時破棄、状態（累積時刻/scope）を次 talk へ持ち越さない | `spawn_actor`（talk ごと thread）／呼出側 sequencer（どちらも可） | 生成/破棄モデル（spawn 単位）＝**Unknown（設計判断）** |
| R11 失敗経路のログ規律・非パニック | 回復可能失敗は error ログ＋継続/観測可能終端、入力異常で非 panic、致命 panic 直前ログ、ログ無し失敗経路禁止 | areka「ログ無し失敗経路の禁止」規律／`areka-actor` handler Err はログして継続 | 失敗経路の網羅設計＝**Missing**。規律自体は **Constraint（従う）** |

---

## 3. 既存パターン・慣行（グラウンディング結果）

### 3.1 上流パーサ `areka_parsers::sakura`（実・完成済み）
- `parse(input: &str) -> Vec<Instruction>`（`Result` 無しの寛容パース）。
- `Instruction`（`#[non_exhaustive]`・`Clone/Debug/PartialEq`のみ、`Eq/Hash/serde` 無し＝`f32`/`Duration` を含むため）。
- **`Wait(Duration)` は上流で単位換算済み**: `decode.rs` に `WAIT_UNIT_MS = 50`、`\w[n]`/`\wN`=n×50ms、`\_w[ms]`=絶対 ms。**R2.3「Wait の Duration を唯一の真実として使い再換算しない」は上流実装で既に保証**されている（sakura エンジンは `\w` の 50ms 換算を再実装してはならない）。
- 値型は不透明 NewType＋read-only accessor（`SurfaceArg::as_str`／`NewLineRatio::ratio`）。R3.2 の「SurfaceArg を解釈・変換しない」は accessor を呼ばず値ごと転送すれば自然に満たせる。

### 3.2 アクター基盤 `areka-actor`（実・完成済み）
- 純粋層 `spawn`: `spawn_actor<M,F>(name, body) -> (Sender<M>, ActorHandle)`（std mpsc＋named thread）／`run_inbox<M,E>(rx, handler)`（`ControlFlow::Break`=即時 return＝Close 即時停止、`Err`=ログして継続、Disconnected=正常終了）。
- 純粋層 `reply`: `reply_channel<T>() -> (ReplySender<T>, ReplyReceiver<T>)`。`ReplySender::send(self)` が consume で「応答高々 1 回」を型強制——**R6.4「TalkDone を高々 1 回」に直結**。
- UI ブリッジ層 `ui`: `spawn_ui`/`UiSender`（async-channel・pump スレッド束縛）。sakura は render/window スレッドに束縛されない純粋再生ゆえ、**純粋層（`spawn_actor`）が第一候補**。UI 層は下流 sink（emo-present）側の都合。
- **停止規約が正本**: Close 即時停止・積み残し破棄・handler Err はログして継続・正常終了経路は Close/全 Sender drop の 2 経路。R7・R11 はこの規約に整合させるだけでよい。

### 3.3 タイミング層 `dola`（実・完成済み）
- `DolaRuntime`: `new`/`load_document(doc)`/`start(name, start_time: f64)`/`tick(current_time: f64)`/`last_result() -> &UpdateResult`。時刻は **f64 秒**。storyboard/variable/subscription 志向＝連続補間アニメ向け（sakura の離散発火列とは粒度が異なる）。
- `dola::cue::TimedSchedule<T>`: **本仕様の展開モデルに最も近い実資産**。`Entry<T>` = `Payload(f64, T)`／`Barrier`／`Routing` の 3 種。`insert`/`extend`（降順ソート維持）＋ `tick(current_time: f64)`（到達 payload を `ready_buffer` へ蒐集・冪等）＋ `ready`（DolaRuntime の `tick`/`last_result` と対称の 2 相 API）。**`Wait` を累積オフセットへ畳んで `Payload(offset, 発火)` を並べれば R2 の展開がほぼそのまま載る**。
- `clock::now()` は QPC ベースの実時刻取得（f64 秒）。**注入式ではない**——決定的テストは `now()` を呼ばず、テスト側が任意の `current_time` を `tick` へ渡す形で成立する（R9.1「実時間 sleep に依存しない」は tick への時刻注入で満たす）。

### 3.4 命名・配置慣行
- クレート命名は `areka-actor`／`areka-emo-atlas`／`areka-parsers` に倣い `crates/areka-sakura` が自然（brief 明記）。ワークスペースは `crates/*` glob 収集ゆえメンバー追加は非衝突・自動。
- テスト: in-source `#[cfg(test)]`＋fixture スモーク。決定的テストは時刻注入で実時間非依存。
- エラー型は `thiserror`。ログは `tracing`（error/debug）。ログ無し失敗経路の禁止。

### 3.5 上流契約の物理的所在（重要な未着地）
- `StartTalk`/`TalkDone` 型は**リポジトリに未定義**（`crates/**/kanade` は存在しない）。両ブリーフ（sakura/kanade）が「kanade が正本・sakura は消費」と散文で先決しているのみ。
- kanade は「並走可・talk 起動契約は先決済み」（sakura brief「Existing Spec Touchpoints」）＝**mock で独立観測**する前提。sakura はこの型を消費する立場だが、型が物理的に存在しないため、**設計で「暫定的にどこへ型を置くか」を決める必要**がある（DD-1）。

---

## 4. 実装アプローチ案（A/B/C）

本仕様は greenfield クレートゆえ「既存を拡張する（Option A）」の余地は小さく、争点は **①タイムライン展開の実現手段（dola 再利用 vs 自前）** と **②再生駆動の単位（アクター spawn vs 呼出側 sequencer）** の 2 軸。

### Option A: dola `TimedSchedule<T>` を展開エンジンとして再利用（新クレートは薄い変換＋結線層）
- **構成**: sakura は「`Vec<Instruction>` → `Wait` を累積して `TimedSchedule<Fire>` を構築」する純粋変換関数＋「`tick(current_time)`→`ready` を 2 sink へ振り分け＋終端検出」する駆動層。
- **トレードオフ**:
  - ✅ 展開・時刻蒐集・冪等・降順ソートの実績コードを再利用（R2/R9 の骨格を借りられる）。テストは変換関数を純粋に単体検証可能。
  - ✅ 「dola＝タイミング層の正本方針」（brief）に沿う。
  - ❌ `TimedSchedule` は `Barrier`/`Routing` を内包し sakura に不要な概念が混入。`Fire` enum に surface/text を混載すると 2 分岐の型分離が実行時判定になる（sink 振り分けが match）。
  - ❌ `Duration`→`f64秒` 変換を全 `Wait` に適用（精度・単位境界の設計確定が必要）。

### Option B: 自前の軽量タイムライン展開（純粋関数）＋自前駆動ループ
- **構成**: `expand(instructions) -> Vec<TimedFire>`（`TimedFire{at: Duration, payload: SurfaceCmd | TextCmd}`）の純粋関数を本仕様が所有。駆動は `recv_timeout` 刻み or 外部 Tick 注入で `at <= elapsed` の発火を順次 sink へ。
- **トレードオフ**:
  - ✅ 2 分岐の型を最初から分離できる（`expand` が surface 列/text 列を別々に返す、または tagged enum を sink で振り分け）。sakura に不要な `Barrier`/`Routing` を持ち込まない。
  - ✅ `at` を `Duration` のまま保持でき、`Wait(Duration)` の単位橋渡しが不要（下流も `Duration` で受け取れる）。R2.3 の「Duration を唯一の真実」に最も素直。
  - ✅ 展開が純粋関数＝単体テストの主戦場（brief「Boundary Candidates」の第一層）。
  - ❌ 時刻到達判定・冪等・順序保証を自前実装（dola に既にある車輪の再発明・バグ余地）。
  - ❌ 「dola 経由が既定」という brief 方針からの逸脱理由を設計で明示する必要。

### Option C: ハイブリッド（純粋 `expand` は自前所有・駆動時刻源は dola/clock 規約に整合）
- **構成**: 展開は自前純粋関数（Option B の `expand`・`at: Duration` 保持で 2 分岐分離）＝**単体テストの決定的中核**。駆動（時刻進行）は areka-actor スレッド上で「外部注入 Tick or `recv_timeout`」で elapsed を進め、`expand` 結果を時刻順に sink へ流す。dola の `TimedSchedule` を「駆動層の内部実装」として採るか自前かは駆動層に閉じた選択とし、**純粋展開層の型（`Duration` ベース発火列）とテスト戦略は駆動実装から独立**にする。
- **トレードオフ**:
  - ✅ brief「三層（タイムライン展開＝純粋 / 再生駆動 / 出力結線）」に最も忠実。純粋層の決定性を駆動実装の選択から切り離せる（R9.4 の再現性を型で守る）。
  - ✅ 2 分岐分離・`Duration` 保持・単体テスト主戦場を確保しつつ、駆動層は後から dola へ寄せる余地を残す。
  - ❌ 層が 3 つに増え、駆動層の時刻源（Tick 注入 / dola / clock）を別途決める必要（決定の先送りに見えるリスク）。
  - ❌ 純粋層 ↔ 駆動層の境界型（発火列の受け渡し表現）を丁寧に設計しないと二重定義になる。

> **推奨の方向性（決定ではない）**: R9（決定的・純粋・単体テスト）と R2.3（Duration 唯一真実）を最優先するなら、**純粋展開層を自前 `Duration` ベースで所有する Option B/C 系**が要件との摩擦が小さい。dola `TimedSchedule` の再利用（Option A/C の駆動層）は「dola 既定方針」への整合として設計で比較する価値がある。最終判断は要件ディスカッション/設計へ委ねる。

---

## 5. 複雑度・リスク

- **タイムライン展開（R2）**: 効果 **S〜M**／リスク **Low**。`Wait` 累積は単調加算の純粋写像。dola 再利用なら S、自前でも中規模。争点は単位（Duration/f64）と 2 分岐型分離のみ。
- **駆動・アクター結線（R1/R6/R7/R10）**: 効果 **M**／リスク **Medium**。per-talk transient の生成単位（spawn か sequencer か）、Close 割り込みと終端の相互作用、`TalkDone` 返信経路（`reply_channel` か通知メッセージか）が絡む。areka-actor 規約に載せれば車輪の再発明は不要だが、上流型の未着地（DD-1）が結線を保留させる。
- **出力契約の型定義（R3/R4/R5）**: 効果 **S**／リスク **Medium**。本仕様が正本ゆえ設計自由度が高い一方、下流（seriko/emo）未着手で消費者検証ができず「契約の妥当性」を fixture 観測でしか担保できない。`at` の表現・scope 表現・`SurfaceArg` 再輸出の確定が要る。
- **観測ハーネス（R9）**: 効果 **M**／リスク **Low**。時刻注入基盤は既存。mock sink 2 本＋fixture 期待値表の作成が主。fixture の `\w[n]` 期待時刻は 50ms×n で既知（上流換算済み）。
- **総合**: 効果 **M（3〜7 日）**／リスク **Medium**。技術的難所は無く既存資産で大半が賄えるが、**上流型の物理的未着地**が結線設計の主リスク。

---

## 6. 設計フェーズへの研究項目（Research Needed）

- **RN-1**: `StartTalk`/`TalkDone` 型の**物理的所在**。kanade 未実装の間、(a) sakura クレートに暫定 owner を置き kanade 完成時に移譲、(b) 共有契約クレート（例 areka-actor 隣接）へ置く、(c) kanade クレートを先に最小定義——のいずれか。brief の「kanade 正本・再定義しない」を破らずに物理的にコンパイル可能にする手段を設計で確定。
- **RN-2**: 時刻/オフセットの**単位橋渡し**。`Instruction::Wait(Duration)` と dola の `f64 秒`／出力契約の `at` を、どの型（`Duration` 保持 or `f64秒` 換算）で貫くか。精度・境界・下流消費者（seriko/emo）の受け取り易さを材料に。
- **RN-3**: dola `TimedSchedule<T>` **再利用 vs 自前展開**の最終比較。`Barrier`/`Routing` 不使用の割り切り、`Fire` 型への surface/text 混載可否、per-talk での `TimedSchedule` 生成コスト、決定性の型保証。
- **RN-4**: per-talk transient の**生成単位**。`spawn_actor` で talk ごとスレッド起動 vs 呼出側（kanade/ghost-setup）での逐次 sequencer。判断材料: transient 生成コスト・Close 割り込みの実装容易性・dola/駆動状態の持ち方・R10 の状態非持ち越し保証。
- **RN-5**: **出力契約メッセージ型**の確定（本仕様が正本）。`SurfaceCommand{scope, surface: SurfaceArg, at}` 級／`TextCommand{scope, kind: Text|NewLine(ratio)|Clear, at}` 級の具体フィールド・scope 表現（既定値含む）・`SurfaceArg` の不透明再輸出方法（`areka_parsers` からの re-export か own newtype か）。
- **RN-6**: **既定話者スコープ**の値（R5.3）。`\0`/`\p[0]` 相当の `n=0` を既定とするか。ukadoc の scope 正準（`\0`/`\1`/`\p`）を design 冒頭で確認（brief「ukadoc 必読」指示）。
- **RN-7**: **M-boot 再生対象タグ表**の作成（brief 具体指示）。`Instruction` 全 variant について「実挙動 / 無視ログ / シーム」を design 冒頭で表化し、fixture 期待値へ反映。`\w` の 50ms は上流換算済みだが ukadoc で終端規律（`\e`/`\-`）と scope 正準を再確認。
- **RN-8**: **`TalkDone` 返信機構**の選択。`reply_channel`（request/reply oneshot・R6.4 の高々 1 回を型強制）か、kanade inbox への片方向通知メッセージか。**中断時の送出有無は要件ディスカッション #1 で解決済み**＝中断も終端理由 `Interrupted` を伴う終端信号を返す（無音破棄しない）。残るは返信機構の型選択と reason 3 値（`Ended`/`Quit`/`Interrupted`）を運ぶ `TalkDone` の具体型（DD-1 と統合）。

---

## 7. 設計判断アイテム（要件ディスカッションへの供給・番号付き）

1. **上流契約型の暫定所在**（RN-1）: `StartTalk`/`TalkDone` を kanade 完成前にどこで定義しコンパイル可能にするか（sakura 暫定 / 共有クレート / kanade 先行最小定義）。
2. **時刻の貫通型**（RN-2）: 発火時刻 `at` と待ち累積を `Duration` で貫くか `f64秒` へ換算するか。下流契約の `at` 型も同時決定。
3. **展開エンジンの実現手段**（RN-3）: dola `TimedSchedule<T>` 再利用か自前純粋 `expand` か（Option A/B/C の選択）。
4. **per-talk transient の生成単位**（RN-4）: `spawn_actor` スレッド単位か呼出側 sequencer か。
5. **出力契約メッセージ型の確定**（RN-5）: `SurfaceCommand`/`TextCommand` の具体形・scope 表現・`SurfaceArg` 再輸出方法（本仕様が正本）。
6. **既定話者スコープ値**（RN-6）: 未指定時の既定 scope（`n=0` 等）と ukadoc 正準の整合。
7. **`TalkDone` 返信機構**（RN-8・要件ディスカッション #1 で中断時挙動は解決済み）: 中断（Close）時も終端理由 `Interrupted` を伴う `TalkDone` を**返す**（無音破棄しない）ことで確定＝終端信号は end/quit/interrupt を通算 1 回・reason 3 値化（`quit: bool`→`{Ended, Quit, Interrupted}`）。**残る設計論点**は (a) 返信機構＝`reply_channel` 往復か kanade inbox への片方向通知か、(b) reason を運ぶ `TalkDone` の具体型形状・所在（DD-1 と統合）。
   - **kanade 側の状態管理構造（付記・kanade の領分）**: talk は SSP 流の逐次実行（同時に高々 1 balloon・新 talk は現 talk を中断）ゆえ、kanade は `HashMap` での多重管理を要さず、**単一の current スロット＋単調増加 talk_id（stale 終端信号の棄却用）**で足りる見込み。talk_id は map 索引ではなく「打った talk と返ってきた終端信号の相関・競合時の stale 判定」に使う。最終決定は `areka-P0-kanade` の領分であり本仕様は規定しない。sakura 側の契約（talk_id エコー＋reason 付き単一終端信号）はこのいずれの構造でも過不足なし。
   - **上書き・中断規律（ukadoc 確認済み・要件ディスカッション #2）**: SSP 既定は**後出し優先（last-writer-wins）**＝新 talk が現行を中断・上書きする（里々 tips「新たなイベントで上書きされてしまう」で確認）。ukadoc の sakura script 一覧に「現行が新 talk の上書きを拒否する」専用タグは見当たらず、近縁は `\![enter,nouserbreakmode]`（＝**ユーザーのダブルクリック中断**の無効化・新 talk 上書きとは別軸）と `\C`（新規側が直前バルーンへ**追記**＝現行が拒否する機構ではない）。
     - **source 特権（`nouserbreakmode` 全文の含意）**: `\![enter,nouserbreakmode]` は「**通常 SSTP 不可・Auth.SSTP（Owned SSTP）のみ可**」＝発行に**出所の特権**を要する。SSP には owned（ゴースト自身の SHIORI）＞外部 SSTP（untrusted）の特権ヒエラルキーがあり、**後出し優先は同格ピア間の既定にすぎず特権が順序を上書きし得る**。
     - **多トリガ→単一 Close funnel**: 中断トリガは複数——(1) 新 talk scheduling、(2) ユーザーのバルーン中断（skip・emo(UI) 由来）、(3) ゴースト切替、(4) 将来の外部 SSTP——だが全て **kanade（運行表）へ集約**され、sakura の中断入力は **`Close` 単一**（R7）。sakura はトリガ種別を不可知。honor/拒否の調停（上書きガード・`nouserbreakmode` による user-break 無効化・source 特権）は kanade scheduling＋後続 M の拡張点で、対応 `\![...]` は M-boot 外＝R8 の無視＋シーム。
     - **M-boot での帰結**: talk 出所はゴースト自身の SHIORI（kanade 経由）ただ 1 つ＝単一 owned source ゆえ source 特権も user-break 調停も moot。全 talk が Close で上書き＝「同時に有効なトークは 1 本」＝単一 current スロットで整合。ガード状態の sakura→kanade 露出は後続 M の拡張点。
8. **M-boot 再生対象タグ表の粒度**（RN-7）: `Instruction` 全 variant の「実挙動/無視ログ/シーム」表を design 冒頭で確定し fixture 期待値へ反映。
