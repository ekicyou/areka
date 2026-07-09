# ギャップ分析: areka-P0-emo-text-layer

> **調査日**: 2026-07-09 ／ **対象**: 確定済み requirements.md（R1〜R11）と既存コードベースの差分
> **前提**: requirements.md・spec.json は確定（本書は変更しない）。本書は設計判断の材料であり、実装方針の決定ではない。
> **言語**: ja（spec.json.language）

## 0. 要約（3〜5点）

- **上流契約はすべて実在・brief の実シンボル偵察は正確**（1点のみ既に解消済みの陳腐化あり）。`TextSink`/`TalkCue.at`/`VisualMount::text_slot`(pub(crate))/`SerikoSink`+`spawn_seriko`/balloon `Origin·WordWrapPoint·ValidRect·Font·FontColor`＋`wordwrappoint.y`転記＋2層マージ/wintf `TextDirection`4方向＋`Typewriter`/`TypewriterLayoutCache`＋DirectWrite縦書きレシピ——**全数コードで確認**（§2 対応表）。
- **欠落している能力＝「文字を描く層」本体**: (a) `TextSink` を実装し UI スレッドへ配送する受信アクター、(b) cue→行/グリフ状態の純粋状態機械、(c) balloon `Font`/領域を消費する DirectWrite レイアウト、(d) `writing_mode` 拡張キー（parser 転記 additive ＋本層の解釈）、(e) validrect あふれスクロール（全域再描画・可視窓/描画分離シーム）、(f) 行列変換領域＋内容キャンバス抽象、(g) `text_slot` 装着の公開経路（emo-present additive）、(h) 専用 example。現状の終端は `LogSink`（`crates/areka-ghost/src/sink.rs`・ログのみ）。
- **wintf テキスト資産は「そのまま消費」には整合しない**（重要な設計論点）。wintf `Typewriter` は自前 `TypewriterToken`/`TypewriterTimeline` IR を消費し、`TalkCue` は受けない。validrect あふれスクロール・内容キャンバス・行列領域・`writing_mode` 2層マージ・純粋状態機械は wintf 側に無い。**再利用できる核**は3点＝① `TextDirection`4方向 enum、② DirectWrite 縦書きレシピ（`SetReadingDirection(TOP_TO_BOTTOM)`＋`SetFlowDirection(RIGHT_TO_LEFT)`）、③ 注入時刻駆動の timeline 更新（`TypewriterTalk::update(current_time, timeline)`・sleep 不使用）と `IDWriteTextLayout` キャッシュ。lift（emo へ複製）か wintf 依存参照かは design 判断（§4 Options）。
- **fixture 実測で example の設計論点が3件確定**: (1) descript の `validrect` は全 0・画像別 `balloons0s.txt` が top46/bottom-56/left36/right-44 で上書き＝**有効 validrect は2層マージ後にのみ非退化**（マージ消費が必達）。(2) `wordwrappoint.y` は両層とも 0＝縦書き折返し軸が退化——**縦書き example は fixture 増補（`writing_mode` マーカー＋有意な `wordwrappoint.y`）が必要**（brief 既述）。(3) `writing_mode` マーカーは fixture・コード共に未存在＝既定 `horizontal_tb` の裏取り。
- **per-glyph pacing と cue 時刻の整合＝懸念は確認済みで前提成立**。sakura の `TalkCue.at` は `\w` 系 wait のみで累積し text 長を考慮しない（`drive.rs:294` `at: cue.start_time`・`drive.rs:530` テストが text chunk 群の `at==0.0` を表明）。**「text-layer 側 pacing が cue 時刻に影響しない」前提で開始可**（R10.2 と整合・厳密 SSP 互換 pacing は sakura への増分申し送り）。

## 1. 現状調査（Current State）

### 1.1 ワークスペース構成と依存方向
- 全 crate は `crates/*` 配下の path 依存 workspace。`areka-emo-present` は `wintf`（path `../wintf`）・`areka-parsers`・`areka-emo-atlas`・`areka-emo-compose`・`areka-actor` に依存（`Cargo.toml` 実測）。**本ユニットの新 crate（想定 `areka-emo-text-layer`）は wintf・areka-sakura・areka-parsers・areka-actor・areka-emo-present へ path 依存できる**（循環なし）。
- 描画は UI スレッド固定（WUC/D2D・MTA＋`DQTAT_COM_NONE`＝記憶 areka-wuc-runs-on-mta-thread）。受信は worker、UI へは `spawn_ui`/`UiSender` で配送（`crates/areka-actor/src/ui.rs`）。

### 1.2 差し込み先＝emo-present の予約スロット（`crates/areka-emo-present/src/mount.rs`）
- `VisualMount`（**`pub(crate)`**）が窓 Entity の子として2つを spawn: surface entity（SpriteVisual＋αマスク）と **`text_slot`**（空 `Visual`・`Name("emo-text-layer-slot")`・surface の**兄弟かつ上位 z**）。`text_slot(&self) -> Entity` も **`pub(crate)`**。
- **公開面ゼロ**: presenter（`presenter.rs`）が `VisualMount` を target ごとに保持するが、`text_slot` へ到達する公開 API は無い。→ **本ユニットが emo-present へ additive な公開増分（`text_slot` 到達手段 or 装着 API）を所有**（R9.2・brief と一致）。
- 予約スロットは `Visual` のみで `VisualGraphics`/brush を持たない seam（test `text_slot_is_higher_z_sibling_with_name` が「内容なし」を表明）。装着時に surface 本体の再合成を強要しない設計（R9.3）と整合。
- `text_slot` は**窓（wintf）World の Entity**である（`VisualMount::attach(world, window, …)` は窓 World に spawn）。emo 本体の per-ghost World（記憶 areka-emo-ecs-foundation）とは別。→ **描画物の投入先は「窓 World の text_slot」・UI スレッド**という前提を design で確認。

### 1.3 上流出力契約（sakura・再定義しない）
- `crates/areka-sakura/src/sink.rs`: `pub trait TextSink { fn emit(&mut self, cue: TalkCue); }`（**infallible**・`SurfaceSink` と別 trait で型分離）。`MockSink` が両 trait を実装。
- `crates/areka-sakura/src/contract.rs`: `pub struct TalkCue { pub at: f64, pub actor: ActorKey, pub command: CueCommand }`。`at` は **talk 起点相対秒**。`cue_target_of` が **Text／NewLine／Clear／Choice → Balloon**、Emote／EntityRef → Shell、Custom → None。`CueCommand::Text(String)`・`NewLine { ratio: f32 }`・`Clear`・`Choice { id, text }`。Choice は本ユニットの管轄外（choice-render・シームのみ）。

### 1.4 donor パターン（seriko・`crates/areka-seriko/src/actor.rs`）
- `pub struct SerikoSink { tx: Sender<SerikoMsg> }` が `SurfaceSink` を実装し、`emit` で `SerikoMsg::Cue` を inbox へ橋渡し（送信失敗は `tracing::error!`・panic しない）。
- `pub fn spawn_seriko(resolver, static_binds: BindSet, out) -> (SerikoSink, ActorHandle)` が `areka_actor::spawn_actor::<SerikoMsg,_>` ＋ `run_inbox` で独立スレッド稼働。停止は **Close 受領 or 全 Sender drop** の2経路（クリーン終了・R1.4 と同型）。
- **写しでよいが差分1点**: seriko は `SurfaceOutput` へ発行（worker 内完結）だが、text-layer は **UI スレッドへ `UiSender` 配送**する（描画が UI 固定）。＝「受信端 worker → `UiSender::send` → UI アクターが描画状態更新」の二段。

### 1.5 balloon 領域・フォントモデル（`crates/areka-parsers/src/balloon/`）
- `model.rs`: `Origin`(x,y)・`WordWrapPoint`(x,y)・`ValidRect`(top/bottom/left/right)・`Font`(name/height/color)・`FontColor`(r,g,b) 全実装。各成分は独立 `Option`・私有フィールド＋read-only accessor・**全 `#[non_exhaustive]`**。→ `writing_mode` 転記フィールド追加は additive（後方互換・R5.6）。
- `parse.rs`: `parse(descript, image: Option<&…>)` が **descript 基層＋画像別後勝ちの2層マージ**（L37-51）。`map_merged` が各キーを完全一致で引く。**`wordwrappoint.y` は L74-75 で転記済み**（`get_scalar::<i32>(merged,"wordwrappoint.y")`）＝縦書き折返し軸は増分ゼロで消費可。`parse_str` は `kv::parse_kv` 経由の便宜入口。
- **モデル化 subset は幾何＋フォントのみ**。`anchor.*`/`cursor.*`/`number.*`/`communicatebox.*`/`arrow*`/`sstp*` はモデル化されない（fixture descript に実在するが非対象）。→ `writing_mode` は BalloonModel へ追加する**新規モデル化キー**。

### 1.6 wintf テキスト資産（`crates/wintf/src/ecs/widget/text/`）
- `label.rs` L12-17: `enum TextDirection { HorizontalLeftToRight, HorizontalRightToLeft, VerticalRightToLeft(日本語縦書き), VerticalLeftToRight }`（CSS writing-mode コメント付き＝`writing_mode` 語彙と1:1）。
- DirectWrite 縦書きレシピ（`draw_labels.rs` L107-113・`typewriter_layout.rs` L132-141）: `VerticalRightToLeft` → `SetReadingDirection(TOP_TO_BOTTOM)`＋`SetFlowDirection(RIGHT_TO_LEFT)`／`VerticalLeftToRight` → `…LEFT_TO_RIGHT`。**縦書き機構は実証済み**。
- `typewriter/mod.rs`: `Typewriter { font_family, font_size, direction: TextDirection, default_char_wait: f64 }`・`TypewriterTalk`（`update(current_time, timeline)` が **注入時刻駆動**・sleep 不使用）・`TypewriterLayoutCache`（`IDWriteTextLayout` 保持）。`mod.rs` が `Typewriter/TypewriterLayoutCache/TypewriterTalk/draw_typewriters/update_typewriters/init_typewriter_layout` を pub export。
- **ただし消費形が違う**: wintf は自前 `TypewriterToken`/`TypewriterTimeline` IR を消費し `TalkCue` を受けない。validrect あふれスクロール・内容キャンバス・行列領域・`writing_mode` 2層マージ・「純粋状態機械」は wintf に無い。＝**「そのまま接続」は不成立、核レシピの lift/参照が現実解**（§4）。

### 1.7 現状の終端（`crates/areka-ghost/src/sink.rs`）
- `LogSink`（`Clone,Copy`・無蓄積）が `SurfaceSink`＋`TextSink` の両方を実装しログのみ出力。M-boot 統合でここに実 sink を挿す（結線は emo2-boot）。→ 本ユニットは `TextSink + Clone + Send + 'static` を満たす sink 型を作るまで（R10.1）。

### 1.8 観測基盤（example）
- `build_balloon_target` は emo-present（`lib.rs`/`balloon.rs`）にあり `crates/areka/examples/emo-present.rs` が既に使用。→ 本ユニットの example はこれを土台に cue 列を注入時刻駆動で流す（R11.1）。**example は新規追加のみ**（`crates/areka` 既存ファイル不変・R11.7・window-placement 並走保護）。emo-present crate 側 examples に置く選択も可（design 裁量）。

## 2. brief 実シンボル偵察の検証（確認／訂正）

| brief の主張 | 実体（ファイル:行） | 判定 |
|---|---|---|
| emo-present `VisualMount::attach` が `emo-text-layer-slot` を予約・`VisualMount`/`text_slot` は `pub(crate)` | `emo-present/src/mount.rs:78,94,224` | **確認** |
| sakura `pub trait TextSink { fn emit(&mut self, cue: TalkCue); }`（別 trait・infallible） | `areka-sakura/src/sink.rs:23-26` | **確認** |
| `TalkCue { at: f64（talk起点相対秒）, actor, command }`・`cue_target_of` の Balloon 分類＝Text/NewLine/Clear/Choice | `areka-sakura/src/contract.rs:46-73` | **確認** |
| seriko `SerikoSink`（`SurfaceSink`・inbox 橋渡し）＋`spawn_seriko(resolver, static_binds, out)` donor | `areka-seriko/src/actor.rs:60,133` | **確認** |
| balloon `Origin/WordWrapPoint/ValidRect/Font/FontColor`・負値=反対辺基準・各成分 `Option` | `areka-parsers/src/balloon/model.rs:110-268` | **確認** |
| descript＋画像別2層マージ実装済み（後勝ち） | `areka-parsers/src/balloon/parse.rs:37-51` | **確認** |
| `wordwrappoint.y` 転記済み（parse.rs L75） | `areka-parsers/src/balloon/parse.rs:73-76` | **確認**（行番号も一致） |
| wintf `TextDirection`4方向・縦書き `SetReadingDirection(TOP_TO_BOTTOM)`＋`SetFlowDirection(RIGHT_TO_LEFT)` | `wintf/.../text/label.rs:12-17`・`draw_labels.rs:107-113` | **確認** |
| wintf `Typewriter`(font_family/font_size/direction/default_char_wait)・`TypewriterLayoutCache`(IDWriteTextLayout) | `wintf/.../text/typewriter/mod.rs:41-50,284-289` | **確認** |
| ghost-setup `GhostBootOptions.text_sink`（構築時注入・setter なし） | 記憶・ghost-setup 完了 spec（本ユニットは注入しない＝emo2-boot 領分） | **確認**（本書は該当型を消費しない） |
| **【訂正】** `model.rs:6` doc が旧名 `text-layer`/`surface-engine` を参照（着手時に修正の宿題） | `areka-parsers/src/balloon/model.rs:6` は現在「下流 `emo-text-layer`／`emo`（render）」と**既に固有名化済み** | **陳腐化＝解消済み**（brief Constraints の宿題は不要・design で二重修正しないこと） |

**追加の実測所見（fixture）**:
- `fixtures/emo2/emo2-kakukaku/descript.txt`: `font.name,Yu Gothic UI`／`font.height,28`／`origin 0,0`／`wordwrappoint.x,-34`・`wordwrappoint.y,0`／`validrect` 全 0。→ **font は present**（SSP 既定 MS ゴシックへの fallback R4.2 は欠落時のみ発火＝unit テストは合成欠落ケースを要する）。
- `fixtures/emo2/emo2-kakukaku/balloons0s.txt`（画像別）: `validrect.top,46`／`bottom,-56`／`left,36`／`right,-44`・`wordwrappoint.x,-49`。→ **有効 validrect は2層マージ後のみ非退化**（あふれ/スクロール判定に必須）。
- `writing_mode` マーカーは fixture・製品コードに**未存在**（grep は spec 群と wintf label.rs コメントのみ）＝既定 horizontal の裏取り。

## 3. 要件→資産マップ（ギャップ・タグ）

| 要件 | 既存資産 | ギャップ（タグ） |
|---|---|---|
| R1 cue 受信アクター＋UI 配送・終了規律 | seriko donor（`spawn_seriko`/`SerikoSink`）・`spawn_ui`/`UiSender`・`run_inbox` | **Missing**: TextSink 実装アクター本体（worker 受信→UiSender 配送の二段）。パターンは既存で低リスク |
| R2 純粋テキスト状態機械 | 先例なし（seriko の `ScopeStates` は surface 状態） | **Missing**: cue→行/グリフ状態の純粋遷移（append/newline/clear/scroll 判定）。DirectWrite 非依存で檻化 |
| R3 typewriter 逐次表示（注入時刻駆動） | wintf `TypewriterTalk::update(current_time,…)`（注入駆動の実証）・`default_char_wait` | **Constraint/Unknown**: 機構は wintf にあるが IR 形が違う。lift/参照は design。per-glyph 所有は R3.2 で本層 |
| R4 Font/領域解決＋DirectWrite レイアウト | balloon `Font`/`Origin`/`WordWrapPoint`/`ValidRect`・wintf `IDWriteTextLayout` 生成 | **Missing**: balloon モデル→DirectWrite 写像。**Unknown**: 反対辺基準（負値）→実 px 解決規則（parser は符号保持のみ） |
| R5 `writing_mode` 2層マージ | 2層マージ機構・`#[non_exhaustive]` モデル・`TextDirection` 語彙1:1 | **Missing**: parser 転記フィールド（additive）＋本層の解釈・未知値 warn fallback・M2 予約名（text_orientation/text_combine_upright）の記録 |
| R6 縦横の軸解釈 | `wordwrappoint.y` 転記済み・wintf 縦書きレシピ | **Missing/Unknown**: 縦書き origin/wordwrappoint/validrect の軸読み替え規則（**areka 独自・SSP de-facto 不在**＝design の1枚表で明文化） |
| R7 あふれスクロール | 先例なし | **Missing**: 全域再描画スクロール＋「可視窓決定（純粋）/描画実行」分離シーム（viewbox 化の移行点）。**Unknown**: 横スクロールの行単位/アニメ有無（SSP de-facto 確認） |
| R8 行列変換領域＋内容キャンバス抽象 | surface 合成の行列原則（同型・emo-compose） | **Missing**: 変換行列付き領域＋「バルーン内容キャンバス」抽象（テキスト＝最初の住人・`\_b` 画像＝後続住人のシーム）。M1 実挙動は恒等/平行移動＋テキストのみ |
| R9 予約スロット装着 | `text_slot`（pub(crate)）・独立レイヤ更新（再合成不要）設計 | **Missing**: emo-present への additive 公開増分（到達手段 or 装着 API）＋choice-render 再利用シーム |
| R10 クロスユニット契約シーム | `TextSink+Clone+Send+'static` 要件・sakura `at` 実装 | **Constraint**: sink 型提供のみ（結線は emo2-boot）。`\f`/`disable.font.*`/DPI(96) は型シーム/素通し |
| R11 専用 example | `build_balloon_target`・emo-present example 土台 | **Missing**: cue 注入駆動 example＋fixture 増補（writing_mode マーカー・有意 wordwrappoint.y）＋構造テスト群 |

複雑度シグナル: 外部統合（DirectWrite・WUC UI スレッド）＋新規ワークフロー（状態機械・スクロール・軸回転）＋型シーム多数。純アルゴリズム部（状態機械・可視窓決定・マージ解決）は決定論テスト適性が高い。

## 4. 実装アプローチ（Options A/B/C）

### Option A: 既存 wintf typewriter を text_slot へ載せて参照消費
`text_slot` Entity に wintf `Typewriter`/`TypewriterTalk` を attach し、既存 system（`update_typewriters`/`draw_typewriters`/`init_typewriter_layout`）に描画させる。TalkCue→TypewriterToken の薄い変換だけ書く。
- ✅ 新規コード最小・逐次表示/縦書きレシピ/`IDWriteTextLayout` を再実装しない。
- ❌ **要件充足に不足**: validrect あふれスクロール・内容キャンバス・行列領域・`writing_mode` 2層マージ・「純粋状態機械」は wintf に無い。cue でなく token IR 前提。純粋層の決定論檻（R2/R5/R7）を wintf 内部へ持ち込めない。
- 判定: **単独では棄却濃厚**（純 typewriter デモには最短だが M1 要件群を満たさない）。

### Option B: 新 crate で全所有＋wintf からレシピを lift（複製）
`areka-emo-text-layer` を新設し、(受信アクター＋UI 配送)/(純粋状態機械)/(DirectWrite レイアウト)/(内容キャンバス＋行列領域)/(スクロール) を自前実装。wintf からは **DirectWrite 縦書き Set*Direction レシピと `IDWriteTextLayout` 利用手順を lift（コピー）**、`TextDirection` 相当は自前 or 借用。
- ✅ 要件を完全充足・層境界明快・純粋層を決定論檻へ隔離しやすい・per-ghost/UI 境界を自制御。emo の自前合成哲学（記憶 areka-emo-own-compositor-atlas）と同型。
- ✅ wintf への新規結合を増やさず emo 側で完結（将来 lift 集約の布石）。
- ❌ 新規ファイル多・DirectWrite コードの二重管理（wintf と emo で縦書きレシピが重複）。
- 判定: **有力**。lift 範囲を最小（縦書きレシピ＋レイアウト生成）に絞れば churn 限定。

### Option C（推奨候補）: ハイブリッド（新 crate 所有＋wintf を型/レシピ参照）
新 crate で受信アクター・純粋状態機械・内容キャンバス/行列シーム・スクロール分離シームを**所有**しつつ、安定した wintf 型（`TextDirection`）は**依存参照**、DirectWrite レイアウト補助は lift か薄いラッパで**再利用**。実装順は横書き先行→縦書き後続（`writing_mode` 抽象＝方向写像/折返し軸/スクロール軸の切替点は最初から構造保持・R6.4）。
- ✅ B の要件充足を保ちつつ縦書きレシピの重複を回避（`TextDirection` を1:1写像先として借用）。段階実装で早期に横書き pass/fail を観測可能。
- ✅ viewbox 化（emo-text-viewbox）・choice-render の下流シームを「描画実行差し替え/行レイアウト返却」で自然に残せる。
- ❌ wintf ↔ emo の型依存境界（どこまで借りどこから lift か）を design で1本線引きする必要（曖昧だと不整合）。
- 判定: **推奨**。B との差は「wintf 型をどれだけ借りるか」のみ＝design の lift/参照粒度の1判断に帰着。

## 5. 工数・リスク

- **工数: L（1〜2週）**。新 crate＋DirectWrite レイアウト＋縦書き＋あふれスクロール＋受信アクター/UI 配送＋emo-present additive＋parser additive＋example＋構造/単体テスト群。純粋層・マージ解決は既存パターンで短工数だが、DirectWrite 写像と軸回転規則の確定が中核工数。
- **リスク: Medium**。
  - 低減材料: 受信アクター（seriko donor）・UI 配送（`spawn_ui`）・縦書き（wintf 実証）・2層マージ（実装済み）・注入時刻駆動（sakura/wintf 双方に前例）・per-glyph 独立性（sakura `at` 実測で確認済み）。
  - 残存不確実性: ① 縦書き origin/wordwrappoint/validrect の軸読み替え規則（areka 独自・典拠不在）② 内容キャンバス/行列領域の抽象形（過剰設計と最小シームの境目）③ lift か wintf 依存かの粒度④ DirectWrite metrics 依存部と非依存部の分離線（R2.4/R4.5/R7.5 の「構造テスト」成立条件）。

## 6. 設計フェーズへの申し送り（Research Needed）

1. **ukadoc `descript_balloon` テキスト描画系キー全量**＋emo-present から継承の**「枠描画/テキスト領域/M1 対象外」3分類表**を完遂（`font.*`/`origin`/`wordwrappoint`/`validrect`/`disable.font.*`/`dpi`）。
2. **ukadoc `list_sakura_script` テキスト系タグ**（`\n`/`\c`/`\f[...]`/`\_b` 全 variant・特に `--option=fixed`＝内容キャンバス二層の典拠）＋**emo2 boot script の実使用タグを fixture 実測**して M1 実挙動 subset を確定（未使用はシーム）。
3. **縦書き軸読み替え規則の1枚表**（横書き top/bottom/left/right が縦書きでどう回るか・折返し＝`wordwrappoint.y`・スクロール＝横）——areka 独自解釈を design 正本として明文化。
4. **lift vs 参照の粒度**: wintf `TextDirection`・DirectWrite 縦書きレシピ・`IDWriteTextLayout` 生成のどこを借り、どこを emo へ複製するか（Option B/C の分岐点）。
5. **DirectWrite metrics 依存/非依存の分離線**: 折返し位置・行送り・スクロール発火・`writing_mode` 2層マージ解決を metrics 非依存の構造テストへ隔離する境界を確定（R2.4/R4.5/R7.5/R11.6）。
6. **example 用 fixture 増補**: `writing_mode` マーカー（descript／画像別）と有意な `wordwrappoint.y` の付与方針（現 fixture は y=0 で縦書き折返しが退化）。マーカー無し既定 `horizontal_tb` の裏取りも兼ねる。
7. **DPI**: `descript_balloon.dpi`（省略時 96）を M1 は 96 前提素通しで可か（window-placement brief の同キー注記と整合）。
8. **per-glyph pacing の申し送り確認**: sakura `at` は text 長非考慮（`drive.rs:294/530` で確認済み）。厳密 SSP 互換 pacing が必要と判明時のみ sakura 増分 issue（本ユニットで sakura 改変せず・R10.2）。

## 7. 設計判断アイテム（要件ディスカッションへ供給・番号付き）

> 要件は確定済み。以下は「確定要件の実現手段」に関する未決事項であり、design/要件ディスカッションでの明示裁定を要する。

1. **新 crate か既存拡張か**: ~~`areka-emo-text-layer` 新設（Option B/C）が層境界上妥当だが、emo-present 内モジュール化も理論上可。~~ → **【裁定 2026-07-09 discussion #1】** 4つ目の emo crate **`areka-emo-text`**（atlas/compose/present と同格・単一トークン命名）で確定。spec/feature 名は `areka-P0-emo-text-layer` 維持（crate↔spec 名マッピングを requirements 明記）。emo-present 内モジュール化は sakura 依存の逆流＋並走保護規約違反ゆえ棄却。
2. **wintf 資産の lift/参照粒度**（§6-4）: ~~`TextDirection` 借用＋DirectWrite レシピ lift（Option C）を既定線とするか、wintf typewriter system 群への依存を増やすか（Option A 寄り）。~~ → **【裁定 2026-07-09 discussion #1】** 描画は emo 所有。縦書き `Set*Direction` レシピは emo へ **lift（複製）**・wintf のテキスト widget を実行時依存にしない。wintf は窓/surface 手渡し（ComposedSurface/swapchain）と donor に留める。Option A/B/C の「wintf 寄せ（W1/W2）」は棄却。
3. **内容キャンバス/行列領域の抽象形**: ~~「バルーン内容キャンバス（テキスト＝最初の住人・`\_b`＝後続住人）」と「変換行列付き領域」を M1 でどこまで型に出すか。~~ → **【裁定 2026-07-09 discussion #1】** R8 を **emo 共有描画基盤**（統一 resident/行列モデル・住人＝グリフ/画像/将来 SERIKO サーフェス同格）へ格上げ。M1 実装住人はテキストのみ。スコープ (X)＝収束を設計するが実体は `areka-emo-text` 内・emo-compose は改変しない（共有 canvas 抽出・シェル/バルーン融合・背景 SERIKO 住人化は後続 roadmap 予約）。
4. **縦書き軸読み替え規則**（§6-3）: areka 独自の1枚表を design 正本として確定（典拠不在ゆえ本 spec が正典を作る）。
5. **描画物投入先の物理**: `text_slot`（窓 World Entity・UI スレッド）へ「emo 自前オフスクリーン D2D 合成→brush 手渡し」か「窓 World の描画 system に委ねる」か。emo 自前合成哲学・独立レイヤ更新（再合成不要・R9.3）との整合で確認。
6. **emo-present 公開増分の最小形**: `text_slot: Entity` の getter 公開 か、装着専用 API（描画物を受け取り slot へ装着）か——最小公開面を design 判断（R9.2）。
7. **example の配置と fixture 増補**（§6-6）: emo-present crate examples か `crates/areka/examples` 新規か。縦書き観測のための fixture マーカー/`wordwrappoint.y` 付与の具体値。
8. **`LogSink` の扱い**: 本ユニットは sink 型を提供するのみ（`LogSink` を置換しない・結線は emo2-boot）を明確化（責務境界の再確認）。
