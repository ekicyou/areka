# ギャップ分析（research.md） — areka-P0-balloon-face-cue

> 目的: 確定済み requirements.md と**既存コードベースの実シンボル**の差分を突合し、設計フェーズへ運ぶ実装戦略・選択肢・研究課題を提示する（決定はしない）。
> 実施日: 2026-07-11 / 対象言語: ja（spec.json）/ brief 引用シンボルは全て実ツリーで検証済み。

## 0. サマリ（3–5 点）

- **brief の現状認識は実シンボルと完全一致**。`\b` は parser でタグ表に無く `Raw` 落ち（`decode.rs:183`）／裸形 `\bN` は lexer が `Bare('b')`＋数字 `Text` に分割し本文漏れ（`lexer.rs:47` `SHORTHAND_WORDS=&['w']`＋`lexer.rs:162-167`）／`compile.rs:81` catch-all で `Raw` を `tracing::debug!` 破棄／dola `CueCommand`（7 variant・`command.rs:121`）にバルーン面 variant 不在——を実確認した。
- **上流 5 エンジンは全て完成済みで additive 増分のみ**。だが「バルーン面切替を運ぶ配管」は物理的に存在せず、`CueTarget::Balloon`→`TextSink`（emo-text 文字状態機械）の固定写像（`drive.rs:216-218`）が構造的な誤配線源。**中心的な設計判断は「バルーン面切替 cue をどの `CueTarget`／sink へ流すか」**（現行 2 分類 Shell/Balloon の意味論整理を伴う）。
- **強制コンパイル点は実測 3 箇所**（`contract.rs:63 cue_target_of`／`ghost sink.rs:56 command_kind`／`emo-text state.rs:195 apply_cue`——いずれも catch-all 無し）。seriko `actor.rs:170` は `cue_target_of` の外側で分類し内側 `205` に catch-all を持つため、**variant 追加時は seriko の機能追加が必要だが強制コンパイルはされない**（テスト檻で担保要）。wintf は完全不関与（brief どおり）。
- **emo-present は多面バルーンが既に動く**（`balloon.rs:120` `build_balloon_target`・`balloons{N}.png`→surface id=N・`world.surface(0/1)` テスト在）。`PresentCommand::ShowSurface{target,surface_id,binds,reply}`／`Hide`（`command.rs:43-59`）も具備。R6 が要求する**「異 id 再 Show の回帰檻」と同寸 `TextSlotView` 安定性テストのみが欠落**（本体改変不要の見込み）。
- **研究課題（設計フェーズ必須）**: (a) ukadoc の `\b` 意味論——MCP 検索は本調査でも空振り（`tag_b`／各種クエリ全滅・brief の 7 空振りを再現）確認済み、get_doc の id 特定 or SSP 実機で確定要。(b) 裸形 `\bN` の**多桁**取り込み方式（既存 shorthand は 1 桁のみ）。(c) `CueTarget`／`DisplayCommand` の拡張形。

---

## 1. Requirement → Asset マップ（ギャップ種別: Missing / Unknown / Constraint）

| Req | 要求の技術要素 | 既存資産（実シンボル） | ギャップ |
|-----|----------------|------------------------|----------|
| **R1** `\b` パース（両形・本文漏れ根絶・不透明転写） | `sakura/decode.rs`（`decode_tag` L161・`\s`→`Surface(SurfaceArg)` L174）／`sakura/model.rs`（`Instruction` L25 `#[non_exhaustive]`・`SurfaceArg` L58）／`sakura/lexer.rs`（`SHORTHAND_WORDS` L47・bare 分割 L162-167） | **Missing**: `Instruction::BalloonSurface(SurfaceArg)` variant＋decode arm（`\b[ID]`）。**Missing/Unknown**: 裸形 `\bN` の**多桁**捕捉（既存 shorthand は 1 桁・`\bN` は連続数字＝R1.2）。lexer 拡張 or decode-level fold の選択（§3-D2）。 |
| **R2** cue 語彙の第一級化（不透明 key 転写・additive・catch-all 禁止維持） | `dola/cue/command.rs`（`CueCommand` L121 7 variant・`Emote{key}` L127 が不透明 key 先例） | **Missing**: `CueCommand::BalloonSurface{ key: String }`（`Emote` と対称）。**Constraint**: `#[non_exhaustive]` 無し＝追加は強制コンパイル点 3 箇所を同時更新（下表）。 |
| **R3** コンパイル写像＋表示系への分類（文字状態機械へ配線しない・配送不能はログ） | `sakura/compile.rs`（`Instruction::Surface`→`Emote` L50-57／catch-all `other` L81-83）／`sakura/contract.rs`（`cue_target_of` L63 catch-all 無し・`CueTarget{Shell,Balloon}`）／`sakura/drive.rs`（分類→2 sink 振分 L216-223） | **Missing**: compile arm（`BalloonSurface`→cue）。**Unknown（中心判断）**: 分類先。`CueTarget::Balloon`→`TextSink`（emo-text）は誤配線ゆえ流用不可。seriko（`SurfaceSink`）へ届ける分類形の確定要（§3-D3）。 |
| **R4** seriko の per-scope バルーン面状態＋表示指令発行（`-1` 非表示・冪等・**alias 適用せず素の id**・シェル状態不改変） | `seriko/actor.rs`（`handle_message` L157・Shell のみ処理 L170・`resolve` alias 適用 L215）／`seriko/state.rs`（`ScopeStates.apply` L90・`ScopeState{Shown,Hidden}` L23・冪等ガード）／`seriko/output.rs`（`DisplayCommand{Show{scope,surface_id,binds},Hide{scope}}` L21） | **Missing**: バルーン面 per-scope 状態（シェル状態と分離）＋バルーン向け `DisplayCommand`（拡張形）＋素の id 解決（`SurfaceResolver` を通さない）。**Constraint**: 既存シェル経路（Emote→alias 解決→`ScopeStates`）を不変に保つ。 |
| **R5** 決定論 E2E 観測＋全増分点テスト（mock sink・sleep 不使用・注入 Tick） | seriko `MockSurfaceOutput`（`output.rs:49`）／sakura `MockSink`／`spawn_seriko`＋`join` 同期パターン（`actor.rs:304`）／compile 純関数テスト群 | **Missing**: 多面バルーン fixture（`balloons1.png` 等 test-local）＋各増分点の実行テスト＋E2E（script 直入力→mock 表示 sink 観測）。既存の観測独立化パターンを再利用可。 |
| **R6** emo-present バルーン target 再表示の回帰保証（同寸異 id 再 Show・`TextSlotView` 安定・**本体 test-only**） | `emo-present/balloon.rs`（`build_balloon_target` L120・多面動作）／`presenter.rs`（`TextSlotView` L81・`surface_size` L104・Hide→再 Show テスト L950） | **Missing**: 異 id 再 Show の回帰テスト（同寸・`TextSlotView` 安定性固定）。**危険域（Unknown/申し送り）**: 異寸切替で `TextSlotView.surface_size` 変化＋`ActorRender` 非再構築（`emo-text actor.rs:351` `contains_key` ガード）＝stale 資源。R6 は**同寸**に限定（異寸は B5 申し送り）。 |
| **R7** 非退行（additive・新規依存なし・Rust 2024／tokio 禁止） | ワークスペース全体・`cargo test --workspace` | **Constraint**: 前例（sakura-engine→dola `NewLine`／emo-text-layer→emo-present 増分）に倣う。i686 成果物前提（workspace test 緑化の既知制約・MEMORY）。 |

### 強制コンパイル点（`CueCommand` variant 追加で機械的に更新が必要な no-catch-all 3 箇所）

| # | 位置 | 役割 | 追加時の対応 |
|---|------|------|--------------|
| 1 | `areka-sakura/src/contract.rs:63` `cue_target_of` | CueCommand→CueTarget 分類（catch-all 無し・意図的） | `BalloonSurface` の分類先を明示（§3-D3 の判断が直撃） |
| 2 | `areka-ghost/src/sink.rs:56` `command_kind` | ログラベル文字列（catch-all 無し） | `"BalloonSurface"` ラベル 1 行追加（挙動不変） |
| 3 | `areka-emo-text/src/state.rs:195` `apply_cue` | 非 Balloon-text command の防御無視 arm（catch-all 無し） | `BalloonSurface` を**非消費 arm**へ追加（挙動不変・文字状態機械は消費しない） |

> 補足: seriko `actor.rs:205` は内側 match に catch-all を持つため**コンパイルは通ってしまう**。R4 の機能（バルーン面消費）は別途 seriko 側で実装しないと `BalloonSurface` が seriko へ届いても黙って skip される。テスト檻で消費経路を担保すること。

---

## 2. 実装アプローチ（A/B/C）

brief 採用案 **A1「統一 display 経路（`\s` と完全対称）」** を軸に、CueTarget/DisplayCommand 拡張の粒度で 3 案を対比する。いずれも parser・dola・emo-text ログ arm・emo-present 回帰テストは共通で、差は**「バルーン面切替 cue を seriko へ届ける経路の作り方」**にある。

### Option A: 既存 `CueTarget::Shell`（=`SurfaceSink`=seriko）を再利用（最小改変）
- **内容**: `cue_target_of(BalloonSurface)`→`CueTarget::Shell`。`drive.rs` の振分は無改変（Shell→`surface_sink`）。seriko 側で `handle_message` に `BalloonSurface` 消費分岐＋バルーン面状態＋バルーン向け `DisplayCommand` を追加。
- **拡張点**: dola／parser／seriko（actor+state+output）／emo-text ログ arm／ghost ラベル。`drive.rs`・`GhostBootOptions`・ghost-setup 結線は**無改変**。
- **Trade-offs**: ✅ 配線変更が最小・`drive.rs`/注入契約に触れない ✅ seriko 構築モデル正典（surfaces.txt＋balloon descript 両方）と整合 ❌ `CueTarget::Shell` の名が「シェル」を騙る（バルーン面も Shell 分類＝意味論の濁り）❌ seriko `handle_message` が「Shell 分類の中に Emote と BalloonSurface が混在」を捌く。

### Option B: `CueTarget` に表示系 variant を新設（意味論を正す）
- **内容**: `CueTarget` を表示宛先で再整理（例 `Shell`/`BalloonSurface`/`BalloonText`、または `Surface(SurfaceKind)`＋`Text`）。`drive.rs` の振分表を `BalloonSurface`→`surface_sink` へ拡張。`cue_target_of` は素直に対応付け。
- **拡張点**: Option A ＋ `drive.rs` 振分（`contract.rs` の `CueTarget` enum 拡張は `drive.rs:216`／seriko `actor.rs:170`／emo-text routing の match も追随要）。`GhostBootOptions`/sink 本数は不変（seriko=`SurfaceSink` のまま）。
- **Trade-offs**: ✅ 分類語彙が実態に一致（誤配線の温床を根絶）✅ 将来 choice-render/communicate 枠の追加に耐える ❌ `CueTarget` は複数箇所で match される横断型ゆえ改変波及が広い ❌ enum 拡張の破壊的変更（既存テスト更新）。

### Option C: 第 3 sink（BalloonSink）新設 — **棄却（brief 明示）**
- **内容**: seriko/emo-text とは別に BalloonSink を立て、`drive.rs` が 3 分類。
- **Trade-offs**: ❌ `GhostBootOptions` 注入契約 2 本の改変＋ghost-setup 結線増 ❌ seriko が持つ per-scope 状態・冪等ガード・単一発行点の**再発明** ❌ シェル/バルーン統一エンジン原則に反する。→ 採らない。

> **推奨の起点**: 「seriko が表示状態の唯一の所有者」という統一原則からは **Option A（最小）or B（意味論正化）** の二択。A は速いが `CueTarget::Shell` の名前負債を残す。B は正しいが横断改変。**DisplayCommand の拡張形（下記 D5）と、`CueTarget` 名前負債の許容度**が決め手。設計フェーズで A/B を確定する。

---

## 3. 設計判断アイテム（要件ディスカッションへ供給）

1. **[D1・研究必須] `\b` の正典意味論（ukadoc）**: `-1`=非表示センチネル／既定面 `balloon.defaultsurface`（既定 0）／裸形 `\bN` の許容／**バルーンに alias 正典が無いこと**を確定。**本調査で ukadoc MCP 検索は再度全滅**（`tag_b` get_doc not_found・キーワード検索 total 0）。→ `list_categories`＋sakurascript カテゴリ列挙で正 id を割り出す or SSP 実機挙動で確定。**設計フェーズ冒頭のブロッカー研究**。

2. **[D2] 裸形 `\bN` の多桁取り込み方式**: 既存 shorthand（`SHORTHAND_WORDS=&['w']`）は**1 桁のみ**・`\bN` は連続数字（R1.2）。選択肢: (a) lexer に多桁対応の新機構（例 `Token::BalloonShorthand(String)` or bare の後続数字取り込み）／(b) decode-level fold（`Bare('b')`＋直後 `Text` の先頭数字列を剥がす——`\q` 旧 2 連 fold の先例あり `decode.rs:82,118`）／(c) M-boot は `\b[ID]` ブラケット形のみ厳密対応し裸形は最小限。**本文数字漏れ根絶（R1.3）が必達**ゆえ (c) でも漏れ防止処理は要る。lexer 層 vs decode 層のどちらで畳むかが層責務判断。

3. **[D3・中心判断] バルーン面切替 cue の分類先（`CueTarget`／sink）**: 現行 `CueTarget::Balloon`→`TextSink`（emo-text）は誤配線ゆえ流用不可。Option A（`Shell` 再利用・名前負債）vs Option B（`CueTarget` 意味論再整理＋`drive.rs` 振分拡張）。**`cue_target_of`（強制点1）の戻り値設計がここで確定**。配送不能（`None`）時の error ログ（R3.3）は既存 `drive.rs:219-222`／seriko `actor.rs:181-188` の流儀を踏襲。

4. **[D4] dola `CueCommand::BalloonSurface` の形**: brief 本命＝`Emote{key}` と対称の**不透明 key 転写**（`{ key: String }`）。`Custom{command,params}` 逃がしは `cue_target_of`→`None` の袋小路ゆえ不採用（brief 明示）。dola は stateless 転送語彙・面の現在状態は seriko 所有。→ 形はほぼ確定だがフィールド名/doc を design で固定。

5. **[D5] seriko の状態表現と `DisplayCommand` 拡張形**: (a) 状態——シェル面 `ScopeState` と**別の per-scope バルーン面状態**を持つ（同一 `ScopeStates` にフィールド追加 vs 別 map vs 別構造体）。冪等ガードは既存流儀を再利用。(b) 指令——`DisplayCommand::Show{scope,...}`/`Hide{scope}` は現状シェル専用（scope キー）。バルーン向けをどう表すか: **(i) 新 variant** `ShowBalloon{scope,surface_id}`/`HideBalloon{scope}`／**(ii) 既存 variant に宛先判別フィールド追加** `target: DisplayTarget{Shell,Balloon}`。emo2-boot adapter が消費する下流契約ゆえ、拡張の安定性（`#[non_exhaustive]` 化含む）を design で確定。(c) 解決——バルーン面は**素の id**（alias 適用せず・R4.4）＝`SurfaceResolver` をバイパスする経路。

6. **[D6] scope→バルーン表示 target 写像はスコープ外**: seriko は `DisplayCommand`（scope 付き）を発行するのみ。scope→`TargetId`／UI 配送は emo2-boot の adapter 責務（Out of scope）。本 spec の観測は mock 表示 sink 止まり。→ 境界の再確認（設計で seriko 出力契約＝adapter 入力契約の形を固める）。

7. **[D7] 文字層の保持/クリア裁定**: 層分離（バルーン枠=emo-present surface／文字=別 visual text_slot）ゆえ**同寸面切替は文字層無傷＝保持がデフォルト（作業ゼロ）**。M1 裁定候補（本命）= 同寸保持＋**異寸はクリア＋warn＋増分申し送り**（B5）。SSP 実挙動（切替時の文字保持有無）確認とセットで design 冒頭確定。R6 の危険域（`TextSlotView.surface_size` 変化・`ActorRender` 非再構築 `emo-text actor.rs:351`）は同寸に限定して回避。

8. **[D8] R6 回帰テストの配置**: emo-present **本体 test-only 追加**（crate 改変なし想定・R6.3）。既存 Hide→再 Show テスト（`presenter.rs:950`）に倣い、異 id 再 Show（同寸）＋`TextSlotView` 安定性を追加。異寸は檻に含めない（申し送り）。

---

## 4. 工数・リスク

| 増分点 | 工数 | リスク | 一言根拠 |
|--------|------|--------|----------|
| parser（`\b[ID]` decode arm＋`Instruction::BalloonSurface`） | S | Low | `\s`＝`Surface` の完全対称・`#[non_exhaustive]` で後方互換。 |
| parser 裸形 `\bN`（本文漏れ根絶） | S–M | **Medium** | 多桁取り込みの層責務判断（D2）。lexer 拡張なら 1 パススキャナへの介入注意。 |
| dola `CueCommand::BalloonSurface` | S | Low | `NewLine` 増分の前例と同型・強制点 3 箇所は機械的。 |
| sakura compile＋分類（`cue_target_of`） | S–M | **Medium** | 分類先（D3）＝中心判断。Option B なら `CueTarget` 横断改変で M。 |
| seriko 状態＋`DisplayCommand` 拡張＋`-1` 非表示 | M | **Medium** | 状態表現・指令拡張形（D5）＋素 id 解決経路＋シェル経路の非退行担保。 |
| emo-present 回帰テスト（同寸異 id 再 Show） | S | Low | 本体不変・既存テスト流儀の複製。 |
| test fixture（多面バルーン）＋E2E 決定論檻 | S–M | Low–Medium | 既存 mock/join 同期パターン再利用。fixture 自前用意（`balloons1.png` 等）。 |
| **合計** | **M（3–7 日）** | **Medium** | additive・上流完成済み。中心リスクは D3 分類設計と D5 seriko 拡張の 2 点に集中。異寸ハザード（B5）はスコープ外化で回避。 |

**リスク集中点**: (1) D3（`CueTarget`/sink 分類）——誤ると emo-text 誤配線が残る or 名前負債。(2) D5（seriko 状態＋DisplayCommand 契約）——emo2-boot adapter の下流契約を左右。(3) D2（裸形多桁）——本文漏れ根絶の可視破損回帰。(4) 研究 D1（ukadoc）未確定のまま実装着手すると `-1`/既定面/alias 有無を誤る恐れ。

---

## 5. 設計フェーズへの申し送り

- **優先アプローチ**: brief 採用 A1（`\s` 完全対称の統一 display 経路）を維持。CueTarget 拡張は **Option A（最小・名前負債許容）vs Option B（意味論正化・横断改変）** を design 冒頭で択一。DisplayCommand は D5 の (i)/(ii) を確定。
- **研究課題（design 冒頭・着手前に解く）**:
  1. **ukadoc `\b` 意味論**（`-1` 非表示・既定面 `balloon.defaultsurface`・裸形・**alias 非適用**）——MCP 全滅につき `list_categories`→sakurascript カテゴリ列挙で正 id 特定 or SSP 実機。
  2. 裸形 `\bN` 多桁の層責務（lexer vs decode fold）。
  3. seriko 出力契約（`DisplayCommand` 拡張形）＝emo2-boot adapter 入力契約の整合。
- **非スコープの再確認**: presenter 実配送結線（scope→TargetId）＝emo2-boot／異寸文字層再装着（B5）＝増分申し送り／SERIKO バルーンアニメ・`\_b`・communicate 枠。
- **次アクション**: `/kiro-requirements-discussion areka-P0-balloon-face-cue` で本ギャップの設計判断（特に D1/D2/D3/D5）を収集・分類し、design フェーズへ確定を持ち越す。
