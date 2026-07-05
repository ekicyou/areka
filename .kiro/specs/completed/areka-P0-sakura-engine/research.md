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

---

## 8. 設計フェーズ・シンセシス（design.md 生成時に確定・2026-07-05）

> **⚠ 履歴注記（2026-07-05・設計ディスカッション #3）**: 本節の DD-2（Duration 貫通）と
> DD-3（dola `TimedSchedule` 不採用・自前 expand）は **§9 で覆された**（→ f64 秒貫通・
> dola cue 採用）。本節は初版 design.md の判断記録として保存する（履歴を書き換えない）。

> Discovery 種別: **full**（greenfield クレート）。ただし §1〜§7 のギャップ分析で
> コードベース側の探索は完了済みゆえ、本フェーズは (1) ukadoc 正典による意味論確認、
> (2) 実シンボルの再検証（`Instruction` 14 variant / `reply_channel` consume / `TimedSchedule`
> の f64・Barrier/Routing）、(3) DD-1..8 の決着に集中した。subagent 分派は不要と判断（研究は
> 既に main context に揃っていた）。

### 8.1 ukadoc 正典確認（design 冒頭・DD-6/DD-8 の根拠）

- **`\p[ID]` / `\0`**: 「デフォルトのスコープは本体側になっている」「`\0` もしくは `\h` 本体側の
  スコープに移る」→ **既定話者スコープ = `n=0`（本体側）で確定**（DD-6・R5.3）。
- **`\e`**: 「この後に書かれたスクリプトは実行・表示されない」→ 終端切詰め（R6.5）の正典裏付け。
- **`\w時間`**: 「時間×50ms 分の時間ウェイト」→ 上流 `Wait(Duration)` の 50ms 換算済みを再確認
  （R2.3・sakura は再換算しない）。fixture 期待時刻は 50ms×n で既知。

### 8.2 Build vs Adopt / Simplification（展開エンジン）

- **DD-3（展開エンジン）= 自前 `expand` 採用・dola `TimedSchedule<T>` 不採用**。実ソース検証で
  `TimedSchedule` は (a) 時刻が `f64` 秒（`Wait(Duration)` から換算負債）、(b) `Barrier`/`Routing`/
  timeout を内包（sakura 未使用の不要概念）、(c) `Payload(f64, T)` の `T` に surface/text 混載→
  sink 振り分けが実行時 match、(d) NaN 時に配信順が黙って崩れる注意点あり、と判明。sakura は
  離散発火列・`Duration` 唯一真実・2 分岐静的分離・全域決定性を要するため自前純粋関数が要件と
  摩擦最小。dola の「タイミング層正本方針」は**駆動層の注入式 tick 観念への整合**として尊重し、
  dola を Cargo 依存には加えない（Simplification）。

### 8.3 Generalization（型の一般化）

- 出力発火は surface/text の 2 系統だが、駆動層の振り分けを型で閉じるため
  `TimedFire{at, output: FireOutput::{Surface|Text}}` の tagged 表現で一般化。sink は
  `SurfaceSink`/`TextSink` の 2 trait に分離（実行時 match を駆動層 1 箇所に閉じ、消費者は
  型で分離される）。scope は `SpeakerScope(u32)`（`Default=0`）newtype に一般化し両 sink 共通付与。

### 8.4 DD-1..8 の決着（design.md が正本・ここは索引）

| DD | 論点 | 決定 |
|---|---|---|
| DD-1 | `StartTalk`/`TalkDone` の暫定所在 | **sakura `contract` モジュールが暫定所有**。kanade 完成時に kanade へ移譲し `contract` は re-export へ差替（下流 import パス `areka_sakura::contract::*` を不変に保つ）。 |
| DD-2 | 時刻貫通型 | **`Duration` で貫通**（`at: Duration`）。`f64` 秒換算しない（R2.3）。 |
| DD-3 | 展開エンジン | **自前純粋 `expand`**（§8.2）。 |
| DD-4 | per-talk 生成単位 | **`spawn_actor` スレッド単位**。Close 割り込みが `run_inbox` の `Break` へ自然に載り、終端で body 復帰＝状態破棄（R10）。 |
| DD-5 | 出力契約型 | `SurfaceCommand{scope, surface: SurfaceArg, at}` / `TextCommand{scope, kind: TextKind, at}`（`TextKind::{Text, NewLine(NewLineRatio), Clear}`）。`SurfaceArg`/`NewLineRatio` は `areka_parsers::sakura` から re-export（二重定義しない）。 |
| DD-6 | 既定 scope | **`SpeakerScope(0)`（本体側・ukadoc 確認）**（§8.1）。 |
| DD-7（RN-8） | `TalkDone` 返信機構 | **`reply_channel` 往復**（`StartTalk.reply: ReplySender<TalkDone>`）。`ReplySender::send` の `self` consume が「通算高々 1 回」を型強制（R6.4/R7.4/R7.5）。 |
| DD-8 | M-boot タグ表 | `Instruction` 全 14 variant＋未知 variant の「実挙動/無視ログ/シーム」表を design.md に確定（Choice/Cursor/Move/SystemVar/GenericCommand/Raw ＝無視ログ＋シーム）。 |

### 8.5 依存方向（design.md「Allowed Dependencies」正本・索引）

`areka_parsers` / `areka-actor`（外部） → `contract`（メッセージ型・暫定） → `expand`（純粋展開）
→ `playback`（駆動・アクター結線） → テストハーネス。`expand` は clock/sink/talk_id/アクターを
知らない純粋層（依存方向の中核・R9.4 の決定性を型で守る）。

### 8.6 設計レビューゲート結果

**1 パスで通過**（repair パス 0）。機械チェック: 全要件 ID（R1.1〜R11.4）が traceability と
コンポーネントブロックに出現・Boundary 4 節が実体・File Structure Plan に具体パス・
コンポーネント↔ファイル 1:1（orphan なし）・境界↔ファイル整合。判断レビュー: 要件被覆・
アーキ準備・境界明確（DD-1 の移譲シーム明示）・実装可能（三層＝境界タスク）を確認。
軽微修正のみ（dola を Cargo 依存に含めない旨の文言整合）。**未解決の要件ギャップ無し。**

---

## 9. 設計ディスカッション #3: dola 基盤化ピボット（2026-07-05・design.md 改訂 2 の正本）

> 設計バリデーション（`design-validation.md`・NO-GO 軽微）後の設計ディスカッションで、
> 開発者が **「sakura は dola の上に建てるエンジンとして設計せよ」** と方向を確定した。
> 初版 design.md の DD-3（dola cue 不採用）は下記の新証拠により**覆される**。
> 本節は §7/§8 の記録を書き換えず追記する（履歴保存）。

### 9.1 ピボットの証拠（実ソース精査・2026-07-05）

初版 DD-3 の比較は `TimedSchedule<T>` 単体を「汎用配信エンジン」として見ていたが、
dola cue モジュール**全体**と wintf 側の消費実装を直読すると、cue ドメインは
**さくらスクリプト再生のために purpose-built された基盤**であることが確認された:

- **`ActorKey`**（`crates/dola/src/cue/command.rs`）: doc に「さくらスクリプトの `\0`(さくら)/`\1`(うにゅう) に相当するが、文字列ベースで任意の名前を許容する」と明記＝話者スコープの受け皿そのもの。
- **`CueTarget::{Shell, Balloon}`**: 「Shell（キャラクター描画）— Emote, EntityRef を主に消費／Balloon（テキスト表示）— Text, Clear, Choice, WaitForChoice を主に消費」＝本仕様の下流 2 分岐（seriko／emo text-layer）の型がすでに存在する。
- **`CueCommand`**: `Text(String)`/`Clear`/`Emote{key}`/`Choice{id,text}`/`EntityRef`/`Custom` — sakura の実挙動タグ（Text/Clear/Surface）と R8 シーム（Choice）を直接受ける語彙。
- **`BarrierKind::{WaitForInput, WaitForChoice, Timeout}`**: `\x`（クリック待ち）・`\q`（選択肢待ち）の将来写像先が **primitive として既に存在**し、wintf `CueQueue` に消費実装（Choice 先積みプロトコル・バリア状態遷移・タイムアウト）まである。
- **wintf 本番配送パイプライン**（`crates/wintf/src/ecs/cue/`）: `CueSheet → PendingCueSheet → dispatch_pending_cue_sheets → per-entity CueQueue（dola TimedSchedule<CueCommand> 内包）→ pop_ready → 消費者`、`EntityRegistry` が `(ActorKey, CueTarget) → Entity` を解決、`FrameTime(f64)` が駆動＝**再生の「配送側半身」は稼働済み**。sakura が cue ドメインで喋れば ghost-setup の結線は `CueSheet`（serde 可）の手渡しで足りる。
- **roadmap 正本**（`.kiro/steering/roadmap.md`）: 「runtime 制御階層 kanade／sakura／seriko（…**両 anim engine は dola 上**）」「（sakura-engine の項）**時間軸は dola✅（時刻注入 tick＝決定的）**」。

初版 DD-3 の判断は「自前型で作ると ghost-setup で自前型→cue の翻訳層が必ず要る」という
統合コストを見落としていた。**最初から cue ドメインで出力する**ことで翻訳層が消える。

### 9.2 覆された判断（DD-2 / DD-3）と新決定

| DD | 初版（§8.4） | **改訂 2（本節が正本）** |
|---|---|---|
| DD-2 | `Duration` で貫通・f64 換算しない | **f64 秒（dola ドメイン）で貫通**。`Wait(Duration)` は compile 内 1 箇所で `as_secs_f64()` 累積＝**単位換算であって `\w`×50ms の再導出ではない**（R2.3 不変）。不変条件: 生成 offset は有限非負 `Duration` 由来の累積和＝**構成的に有限・非負**→ dola が文書化する NaN ハザード（schedule.rs NOTE(D3-V)・sheet.rs P25）は本経路で発生し得ない（放電） |
| DD-3 | 自前純粋 `expand`・dola cue 不採用（Cargo 依存にも加えない） | **dola cue 採用・dola を P0 Cargo 依存へ**。compile は `CueSheet`＋`TalkEndReason` を返し、駆動は `TimedSchedule<TalkCue>`（`TalkCue{at, actor, command}`）。初版の懸念への回答: f64 換算=1 箇所不変条件付き／Barrier・Routing=「不要概念」でなく M-dialogue 写像先として温存する資産／2 分岐振り分け=`cue_target_of` 1 関数に閉じる |

**`compile_sheet` は不採用**（重要な新発見）: `dola::cue::compile_sheet` は最小 `start_time` を
0 基準へ正規化するため**先頭の `\w`（冒頭待ち）を消す**（例 `\w9テキスト` の 0.45s→0s）。また
`CompiledCue` は actor/at を payload 外へ置くため headless 駆動の観測（R9.2 の at 観測）に不適合。
sakura の compile は構成的に 0 起点なので正規化不要＝駆動層の独自アダプタ `to_schedule` で
`Entry::Payload(start_time, TalkCue)` を直接挿入する（design.md に禁止事項として明記）。

### 9.3 DD-9: `NewLine` の cue 表現（新規決定）

`Instruction::NewLine(NewLineRatio)` に対応する `CueCommand` が dola に無い。選択肢:

- **(a) dola `CueCommand` へ `NewLine { ratio: f32 }` variant を追加（採用）**
- (b) `CueCommand::Custom{command: "newline", params: {ratio}}`（dola 不改変・stringly-typed）

**決定 = (a)**。根拠: ①改行はさくらスクリプトのテキスト系一級指令（`\n`/`\n[percent]`）で
M1 主経路（emo text-layer）の消費対象＝`Custom`＋`DynamicValue` では型安全性を失い f32 が
JSON 動的値になる（Type Safety 原則違反）。②steering 記憶「正規実装・小細工禁止」＝正規経路の
ための隣接クレート増分はスコープ内。③source-breaking 影響を実測: `CueCommand` は
`#[non_exhaustive]` でないが、**ワークスペースに exhaustive match は存在しない**——
wintf `queue/mod.rs` の 2 match は `other =>`/`_ =>` catch-all、tests は `matches!`／個別構築のみ、
dola 側は `cue_command_six_variants` テストと doc の「6 バリアント」表記の更新（意味整合）のみ。
serde は variant 追加＝後方互換（現行ワークスペースに cue の永続化経路なし）。
`Entry<CueCommand>` 128B サイズテストにも影響なし（`NewLine{f32}` < `Text(String)`）。

### 9.4 アクター/駆動モデルの確定（validation Issues 1〜3 の解決・discussion 合意）

- **Issue 1（Close 配送端の欠落）**: `spawn_talk(start, sinks) -> TalkHandle{inbox: Sender<SakuraMsg>, actor: ActorHandle}`。`spawn_talk` が spawn 直後に `SakuraMsg::Start(start)` を inbox へ**自己投函**し、以降 kanade/テストは inbox へ `Tick`/`Close` のみ送る＝**投函経路 inbox 一貫**。単一 inbox の全順序が「Start 先行」「Close と Tick の順序確定」を保証（DD-11）。
- **Issue 2（注入時刻と Close 即時性の両立）**: 正準ループは `run_inbox` そのもの＝時刻も `SakuraMsg::Tick(f64)` として inbox へ入る（二重待機が消滅）。**`Tick` の意味論を「talk 起点からの経過秒（0 起点・単調非減少・有限）」に固定**（DD-10）。駆動は `TimedSchedule::new(0.0)`＋`tick(elapsed)` の恒等対応。絶対時刻 epoch（QPC）の知識と `Instant` は sakura から完全排除。本番 ticker（kanade/clock アクターが `clock::now()` から elapsed を算出し実 cadence で送る）は**スコープ外シーム**（ghost-setup/kanade 領分）。**新規ガード**: `TimedSchedule::tick` の冪等早期 return は ready バッファを保持するため、同時刻再 tick で ready() を再読すると**二重発火**し得る→駆動層が直前 tick 値を保持し `t <= prev` を no-op 化（設計固定）。非有限 Tick は `error!`＋無視（NaN 全量配信ハザードの遮断）。
- **Issue 3（高々 1 回機構の一本化）**: `ReplySender::send(self)` の move-consume を**唯一の機構**とし終端済みフラグは持たない（初版是正のまま維持）。body の `Option<TalkState>` は所有権スロット（FnMut 越しの move-consume 表現）でありフラグではない。全終端経路は「take → send → 直後 Break」の対＝Break 後はスレッド消滅ゆえ終端後 Close は構造的に再返信不能（R6.4/R7.5）。

### 9.5 不変の契約（議題 #1/#2・Category-A 是正の維持）

`TalkDone{talk_id, reason: {Ended, Quit, Interrupted}}`／通算高々 1 回／Close 単一 funnel
（トリガ不可知）／`StartTalk`・`TalkDone` の暫定所在=sakura contract モジュール（DD-1）／
`TalkId(u64)` newtype／ガード系タグ=R8 無視＋シーム——はすべて**不変**。R8 シームはむしろ
強化された: `Choice` → `CueCommand::Choice` 先積み＋`Barrier(WaitForChoice)`、`\x` 系 →
`Barrier(WaitForInput)` という **dola 既存 primitive** を M-dialogue の写像先として明記できる。

### 9.6 依存の反転と handoff 成果物

- **dola = P0 Cargo 依存**（cue モジュール）。`runtime`/`DolaRuntime` は使用しない。
- **wintf は依存しない**（headless・ECS 非依存は不変）。ECS 統合は ghost-setup の結線シーム:
  **handoff 成果物 = `CompiledTalk::sheet: CueSheet`（serde 可）**＝wintf パイプライン
  （`PendingCueSheet` → dispatch）がそのまま消費できる形。
- 依存方向: `dola::cue`/`areka_parsers`/`areka-actor` → `contract` → `compile` → `sink` → `drive` → tests。

### 9.7 M-boot テスト方針（R9・f64 決定性）

- 決定性は「注入 `Tick(f64)` 列の直入力」で担保（実時間 sleep・`clock::now()`・`Instant` 不使用）。
- **f64 累積の決定性**: IEEE 754 加算は決定的。テスト期待値は 10 進リテラル直書きでなく
  **実装と同一の `as_secs_f64()` 累積**で計算し表現誤差を排除（0.05 は 2 進で非正確表現）。
- mock sink は `Arc<Mutex<Vec<TalkCue>>>` 共有蓄積＝`(actor, command, at)` を観測。
- compile 純粋テストが主戦場（不変）。追加の固定テスト: 先頭待ち保存（`compile_sheet` 不使用の
  証明）・冪等/逆行 Tick の二重発火なし・非有限 Tick 無視。

### 9.8 設計レビューゲート結果（改訂 2）

**1 パスで通過**（repair パス 0）。機械チェック: 全要件 ID（R1.1〜R11.4）が traceability と
コンポーネントブロックに出現・Boundary 4 節実体・File Structure Plan 具体パス
（`crates/areka-sakura/src/{lib,contract,compile,drive,sink,error}.rs`＋Modified=
`crates/dola/src/cue/command.rs`/`mod.rs`）・コンポーネント↔ファイル 1:1（orphan なし）・
境界↔ファイル整合（dola 増分は DD-9 として境界宣言済み）。判断レビュー: validation の
Critical Issues 1〜3 がそれぞれ DD-11（TalkHandle・inbox 一貫）・DD-10（Tick=経過秒・
run_inbox 正準・二重発火ガード）・move-consume 一本化（維持）で解消されていることを確認。
**未解決の要件ギャップ無し。**

### 9.9 DD 索引（改訂 2 時点の有効判断）

| DD | 論点 | 有効な決定（正本） |
|---|---|---|
| DD-1 | `StartTalk`/`TalkDone` 暫定所在 | sakura `contract` 暫定所有・kanade 移譲時 re-export 切替（不変） |
| DD-2 | 時刻貫通型 | **f64 秒（dola ドメイン）**・`as_secs_f64()` 換算 1 箇所・有限非負不変条件（§9.2） |
| DD-3 | 発火列/配信エンジン | **dola cue 採用**（`CueSheet`＋`TimedSchedule<TalkCue>`・`compile_sheet` 禁止）（§9.2） |
| DD-4 | per-talk 生成単位 | `spawn_actor` スレッド単位（不変） |
| DD-5 | 出力契約型 | **cue ドメインで実現**: `TalkCue{at: f64, actor: ActorKey, command: CueCommand}`＋`cue_target_of`（Shell/Balloon 分類）。scope は `ActorKey(n.to_string())` 転写（旧 `SpeakerScope(u32)` newtype は廃止・実体解決は下流 registry） |
| DD-6 | 既定 scope | 既定 0 → `ActorKey("0")`（ukadoc 確認・不変） |
| DD-7 | `TalkDone` 返信機構 | `ReplySender` move-consume 唯一機構（不変・フラグ禁止） |
| DD-8 | M-boot タグ表 | cue 写像込みで design.md に更新（Choice/`\x` の写像先=dola 既存 primitive を明記） |
| DD-9 | `NewLine` の cue 表現 | **dola `CueCommand::NewLine{ratio: f32}` 追加**（§9.3・touch points 実測済み） |
| DD-10 | `Tick` の意味論 | **talk 起点経過秒（0 起点・単調・有限）**＋`TimedSchedule::new(0.0)`・二重発火/非有限ガード（§9.4） |
| DD-11 | 駆動の対外 I/F | `spawn_talk -> TalkHandle{inbox, actor}`・Start 自己投函・投函経路 inbox 一貫（§9.4） |
