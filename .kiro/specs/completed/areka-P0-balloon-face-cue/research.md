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

> **開発者裁定（2026-07-11 要件ディスカッション#1・議題1）**: バルーン面引数は **cue シーム全域（parser／dola／sakura）を不透明文字列で運ぶ**（`\s` の `SurfaceArg`・dola `Emote{key:String}` と完全対称）。`\b[10]` の数値形も `\b[バルーン１]` の名前形も同一の忠実転写で扱い、**整数化・alias 解決は seriko（消費側）の下流責務**へ寄せる。M-boot の seriko は**数値解決のみ**（`-1`→非表示）＝非数値 key はログして skip し、**名前／alias 解決は将来の下流仕様が不透明 key の上に additive 追加する余地として残す**（実装しないが語彙で潰さない）。裸形 `\bN` は**単桁の数値 shorthand のみ**（`\w` 前例・classic `\sN` と正典整合）＝多桁・名前は必ずブラケット形。→ requirements.md R1（全面）・R4（数値解決＋非数値 skip の R4.5 新設）へ反映済み。D1/D2/D4/D5 は本裁定を前提に確定すること。

1. **[D1・研究必須] `\b` の正典意味論（ukadoc）**: `-1`=非表示センチネル／既定面 `balloon.defaultsurface`（既定 0）／裸形 `\bN` の**桁境界（単桁 shorthand 確定）**／**バルーン名前／alias の正典有無**（M-boot は非実装だが語彙は開けておく）を確定。**本調査で ukadoc MCP 検索は再度全滅**（`tag_b` get_doc not_found・キーワード検索 total 0）。→ `list_categories`＋sakurascript カテゴリ列挙で正 id を割り出す or SSP 実機挙動で確定。**設計フェーズ冒頭のブロッカー研究**。

2. **[D2] 裸形 `\bN` の単桁取り込み方式**: 裁定により裸形は**単桁の数値 shorthand のみ**（R1.2・`SHORTHAND_WORDS=&['w']` の 1 桁前例と対称）＝多桁・名前はブラケット形。よって既存 `WaitShorthand` と同型の**単桁 bare 分岐を `\b` に追加**するのが素直（lexer 層で `b` を shorthand 対象へ）。選択肢: (a) `SHORTHAND_WORDS` に `b` を加え単桁数字を面 shorthand として読む（`\w` 機構の一般化・**本命**）／(b) decode-level で `Bare('b')`＋直後 `Text` 先頭 1 桁を剥がす fold（`\q` 旧 fold 先例 `decode.rs:82,118`）。**本文数字漏れ根絶（R1.3）が必達**——`\b1`→面`1`（漏れなし）・`\b12`→面`1`＋本文`2`（`2` は正当本文）。lexer 層 vs decode 層のどちらで畳むかが層責務判断（`\w` が lexer 層ゆえ (a) が一貫）。

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

---

# 6. 設計フェーズ Discovery & 決定記録（2026-07-11・kiro-spec-design）

> Discovery 種別: **Extension（light discovery）** — 上流 5 エンジン完成済みの additive 増分。§0–5 のギャップ分析（実シンボル検証済み）を土台に、設計フェーズで D1–D8 を確定した。design.md が正本、本節は根拠ログ。

## 6.1 Research Log

### D1: ukadoc `\b` 正典意味論（解決・ブロッカー解消）

- **Context**: MCP キーワード検索が通算 10+ クエリ空振り（`\b[ID]`／`\b0`／`バルーン切り替え` 等全滅）。`list_categories` → sakurascript カテゴリは存在し、`\w時間`（id `ukadoc:list_sakura_script:_5cw_6642_9593:1`）等は引けるが `\b` エントリの id は特定不能だった。
- **Sources**: 最終的に **ukadoc 原典ページを直接取得**して確定 — https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html （併せて `balloon.defaultsurface` は MCP `ukadoc:descript_ghost:balloon.defaultsurface_2c_6570_5024:1` で取得）。
- **Findings**（原文転記に基づく確定事項）:
  1. **`\b[ID]`**: 「現スコープ側のバルーンをID番号のバルーンに変更する。**奇数はキャラクターの右側に表示するためのバルーンのために予約**されているため、使えるIDは0または偶数のみ。**`\b[-1]`でバルーンを非表示**。」→ `-1`＝非表示センチネル確定・**per-scope**（現スコープ側）確定。
  2. **裸形 `\bID`**: 「この場合**0～9のみ使用可能**。」→ **単桁 shorthand の開発者裁定が正典と一致**（`\sID` 裸形も同文言で 0–9 限定＝`\w` と同じ 1 桁機構）。
  3. **fallback 拡張**（SSP 2.6.34～）: `\b[ID1,--fallback=ID2,--fallback=ID3]` が存在する。M-boot スコープ外（後述 6.2 決定 2 の第 1 引数転写で ID1 として graceful に動く）。
  4. **`balloon.defaultsurface`（ghost descript）**: 既定面は数値・既定 0（`sakura.balloon.defaultsurface`／`kero.balloon.defaultsurface` の per-scope 版あり）。既定面の初期表示は**表示系起動側（emo2-boot adapter）の責務**であり seriko の状態初期値ではない。
  5. **バルーン名前/alias**: `\s[ID]` には「surfaces.txt の surface.alias または name で定義された文字列を ID の代わりに使用できる」と明記があるが、**`\b` にはその記述が無い**。→ M-boot「数値解決のみ・非数値は warn+skip」（R4.5）は正典安全側。名前解決は将来 additive の余地として不透明 key で温存。
- **Implications**: 奇数予約は**作法であって engine gate ではない**（seriko はパリティ検査しない。存在しない面は emo-present の EmptyComposition→Hide 縮退＋warn の既存挙動が受ける）。テスト fixture は正典に倣い偶数 id（0/2）を用いる。

### 強制コンパイル点の実測再確認（設計フェーズ時点）

- `areka-sakura/src/contract.rs` `cue_target_of`（catch-all 無し・`Custom→None` のみ）／`areka-ghost/src/sink.rs` `command_kind`（7 arm 網羅）／`areka-emo-text/src/state.rs` `apply_cue`（`Emote|EntityRef|Custom` 非消費 arm）— **3 箇所とも実読で確認**。
- `dola/src/cue/command.rs` の `cue_command_seven_variants` テスト（7 個数え上げ）と serde roundtrip テストは variant 追加で更新要（機械的）。
- **wintf 不関与を再確認**: `CueTarget` は wintf では tuple キー／registry 名前空間としてのみ使用（`tracker.rs`/`registry.rs`）で exhaustive match 無し。`CueCommand` も match しない。
- seriko `actor.rs` の外側分類（`Some(other)` catch-all）と内側 command match（`other` catch-all）は **variant 追加でコンパイル強制されない** → 消費経路はテスト檻で担保（design Testing Strategy）。

### E2E 同期機構の発見（決定論・sleep 不使用）

- `spawn_talk`（`areka-sakura/src/drive.rs`）は `StartTalk{talk_id, script}` を受け内部で parse→compile する（script 直入力の起点）。Tick は `TalkHandle.inbox` へ `SakuraMsg::Tick(f64)` 注入。
- **同期チェーン**: `done.recv()`（TalkDone 受領）→ talk スレッド終了で move 済み `SerikoSink`（唯一の Sender）が drop → seriko inbox **disconnect** → `run_inbox` 正常終了 → `seriko ActorHandle.join()`。**既存の 2 停止経路（Close/disconnect）だけで新 API 不要**・sleep/polling ゼロ。
- areka-seriko は areka-sakura／areka-parsers に既に regular 依存 → E2E は areka-seriko 側 `tests/` に新規依存ゼロで置ける。

## 6.2 Design Decisions（D1–D8 確定）

### 決定 1（D3・中心判断）: 分類先は Option A ＝ `cue_target_of(BalloonSurface) → CueTarget::Shell`

- **あるべき姿**: `CueTarget` の 2 分類の実態は「Shell＝サーフェス消費系（seriko）／Balloon＝文字状態機械（emo-text）」であり、理想形は `Surface`/`Text` へのリネーム（意味論正化）。
- **理想形を今採らない理由（solve/not-solve 対比）**:
  - **リネーム（理想形）**: ✅意味論完全 ❌ serde ワイヤ互換破壊（`CueTarget` は Serialize/Deserialize）・wintf の tuple キー／EntityKey 名前空間／既存テスト群への横断破壊＝ **R2.3「既存配送対象を変更しない」違反**。
  - **第 3 variant 追加（B'案）**: ✅名前は正しい ❌同一 sink（SurfaceSink）へ流れる**擬似スロット**が生まれ wintf の per-actor スロットモデル（`EntityKey::Actor(ActorKey, CueTarget)`）を無消費者のまま拡張・drive/seriko 両方の追随も結局必要＝改変量最大で得るのは名前だけ。
  - **Option A（採用）**: ✅ dola `CueTarget`／`drive.rs`／wintf 完全無改変・seriko は既に Shell 分類を全量受領済み・「seriko＝表示状態の唯一の所有者」（シェル/バルーン統一エンジン正典）と意味的に整合 ❌ `Shell` の名が広義化（名前負債）。
- **名前負債の処置**: dola `CueTarget::Shell` の doc comment を「表示系（サーフェス消費・seriko が消費: シェル面＋バルーン面）」へ更新（doc-only・挙動不変）。**リネーム本体は将来の cue-routing 再編 spec へ明示申し送り**（未解決のまま登記）。

### 決定 2（D4）: `CueCommand::BalloonSurface { key: String }`（`Emote` 完全対称）

- 8 番目の variant・不透明 key 転写・stateless（面の現在状態は seriko 所有）。serde は externally-tagged で additive。`Custom` 逃がしは配送不能袋小路ゆえ不採用（brief 既決）。
- **ブラケット引数の転写幅**: `\s` の実装（`args` の**第 1 引数のみ** `SurfaceArg` へ）と完全対称に、`\b[...]` も第 1 引数のみを key とする。ukadoc fallback 形 `\b[2,--fallback=4]` は ID1=`"2"` として graceful 動作（fallback 意味論は将来 additive・既知の制限として登記）。

### 決定 3（D2）: 裸形 `\bN` は lexer shorthand 機構の一般化で取り込む

- `SHORTHAND_WORDS = &['w', 'b']` へ拡張し、内部トークン `Token::WaitShorthand(u8)` を `Token::Shorthand { word: char, n: u8 }` へ一般化（`pub(crate)` 内部型＝公開 API 不変）。decode 側で `'w'`→`Wait`／`'b'`→`BalloonSurface` へ分岐。
- 本文数字漏れ根絶（R1.3）は lexer 層で構造的に解決: `\b1`→面`"1"`（Text 出力なし）・`\b12`→面`"1"`＋本文`"2"`・`\b2[x]` は既存 `\w2[x]` と同じく非 shorthand（`Tag{word:"b2"}`→Raw passthrough）・数字無し裸 `\b` は従来どおり `Bare('b')`→`Raw("\b")`。
- decode-level fold 案（`\q` 旧 fold 先例）は棄却: `\w` が lexer 層で解決済みの先例と層責務が割れる。

### 決定 4（D5）: seriko 状態＝`ScopeStates` 同居 map・指令＝`DisplayCommand` 新 variant

- **状態**: `ScopeStates` に `balloon: HashMap<ActorKey, ScopeState>` を**シェル map と別 map で同居**追加し `apply_balloon()` を新設（`ScopeState{Shown,Hidden}`・冪等ガード・未知 scope への Hide 一度発行、を既存 `apply()` と同一規律で鏡映）。シェル map・`apply()` は無改変（R4.6 の構造的担保）。別構造体案は所有者分裂（No Hidden Shared Ownership 違反気味）ゆえ棄却。
- **指令**: `DisplayCommand::ShowBalloon { scope, surface_id }`／`HideBalloon { scope }` の**新 variant**（案 i）。既存 `Show`/`Hide` へ target フィールド追加（案 ii）は既存契約の形状変更＝下流（emo2-boot adapter・既存テスト）破壊ゆえ棄却。
- **`binds` は載せない**: バルーンに着せ替え bind は M-boot 不存在。adapter が `PresentCommand::ShowSurface{binds: BindSet::default()}` を組む。SERIKO バルーンアニメ導入時の Revalidation Trigger として登記。
- **`#[non_exhaustive]` は付けない**: workspace 内部契約はコンパイラ強制（catch-all 禁止文化）を優先。variant 追加時は下流 match が明示追随する（本 spec 自身がその実演）。
- **数値解決**: `resolve.rs` に純関数 `resolve_balloon_key(&str) -> BalloonResolve` を新設（バルーン専用の解決結果型＝シェルの `SurfaceTarget` 非干渉・R4.6）。`-1`→`Hide`・`0..=u32::MAX`→`Show`・**非数値（名前形）→`NameForm`**・**数値だが不正（`-2`・範囲外・u32 超過）→`Invalid`**（**alias 表を引かない**＝R4.4）。ログ水準を類別（設計ディスカッション#1 裁定・2026-07-12）: `NameForm`＝M-boot 未対応の正当構文→**warn!**（`EntityRef` 先例・将来の名前解決 additive 余地）／`Invalid`＝破損入力→**error!**（シェル経路 `actor.rs:216-222` と同水準）。actor は `Show`/`Hide` のみ `SurfaceTarget` へ写して `apply_balloon` へ渡す。

### 決定 5（D1 派生）: 既定面・奇数予約は本 spec の非責務

- `balloon.defaultsurface`（既定 0）の初期表示は emo2-boot adapter の起動時責務（seriko 状態は未設定から始まり、初回 `\b[N]` は必ず Changed）。奇数 id はパリティ検査しない（正典は「予約」＝作法であり、実在しない面の縮退は emo-present 既存挙動が受ける）。

### 決定 6（D7）: 文字層は同寸保持（作業ゼロ）・異寸はスコープ外（B5 申し送り）

- 層分離（バルーン枠=emo-present surface／文字=text_slot 別 visual）ゆえ、seriko がバルーン面切替で `Clear` を発行しない限り**文字層は構造的に無傷＝保持がデフォルト**。M1 裁定＝同寸保持（追加実装なし）。異寸切替の文字層再装着（`ActorRender` 再構築）は B5 増分へ申し送り（R6 は同寸限定で檻を張る）。

### 決定 7（D8・R6/R5.5）: emo-present 回帰は presenter.rs `#[cfg(test)]` 内 additive・fixture は test-local 合成

- 既存 `hide_then_reshow_recovers_display_from_cache`（presenter.rs:954）の流儀を複製し、**2 面同寸 assets ヘルパ**（id 1000/3000 等・同 w×h・別バイト）＋「異 id 再 Show」テストを追加。`text_slot_view()` スナップショットの前後一致（slot/window/surface_size/scale）で TextSlotView 安定性を固定。crate 本体（非 test コード）は無改変。
- R5.5 の多面バルーン fixture は **test-local 合成**（balloon.rs の TempDir＋MemoryDecoder 流儀で `balloons0.png`＋`balloons2.png`＝正典の偶数 id）。emo2 実 fixture へは手を入れない。

### 決定 8（E2E 配置・R5）: areka-seriko `tests/balloon_face_e2e.rs`

- script 直入力（`StartTalk{script}`）→ `spawn_talk`（surface_sink=`SerikoSink`・text_sink=test-local Null）→ Tick 注入 → `done.recv()` → SerikoSink drop（disconnect）→ `seriko join()` → `MockSurfaceOutput` 照合。新規依存ゼロ・sleep ゼロ。cross-thread ログ檻は既存流儀どおり `handle_message` 同期呼び出し＋`capture_logs` の単体側で張る。

## 6.3 Synthesis 記録

- **一般化**: 単桁 shorthand 機構を `Token::Shorthand{word,n}` へ一般化（interface の一般化・実装は `w`/`b` の 2 語に限定）。
- **Build vs Adopt**: 新規外部依存なし（R7.2）。全て既存機構（shorthand・ScopeStates 規律・MockSurfaceOutput・TempDir/MemoryDecoder・spawn/join 同期）の再利用。
- **簡素化**: 第 3 sink なし／`CueTarget` 拡張なし／`DisplayCommand` の binds なし／`#[non_exhaustive]` なし／既定面状態なし——いずれも「今の要件が要求しない just-in-case」を排除。拡張余地は不透明 key と variant 追加の 2 シームで担保。

## 6.4 Risks & Mitigations（設計フェーズ更新）

- **seriko 追随漏れ（コンパイラ非強制）** — actor 内側 match の catch-all により variant 追加が黙殺され得る → E2E＋handle_message 単体檻で「ShowBalloon/HideBalloon が実際に出る」ことを直接固定。
- **裸形 lexer 介入の回帰** — 1 パススキャナの shorthand 分岐変更 → 既存 `\w` 全テスト緑維持＋`\b` 境界ケース（`\b`単独・`\b12`・`\b2[x]`・`\b1[`）の檻を追加。
- **`\s` 対称性の暗黙破壊** — `\b` arm 追加時に既存 arm へ触らない（R1.6）→ parsers 既存テスト全緑で担保。
- **名前負債の固定化** — `CueTarget::Shell` 広義化を doc に明記＋将来 spec 申し送りを design.md Out of Boundary に登記（黙って風化させない）。

## 6.5 References

- [UKADOC さくらスクリプト一覧](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html) — `\b[ID]`／裸形 `\bID`（0–9）／`\b[-1]` 非表示／奇数予約／fallback 形の正典。
- [UKADOC ゴースト descript.txt](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html) — `balloon.defaultsurface,数値`（既定 0）。
- [UKADOC バルーン構成](https://ssp.shillest.net/ukadoc/manual/manual_balloon.html) — `balloons*.png` ファイル族。
