# ギャップ分析: areka-P0-sakura-dialogue-tags

> **フェーズ**: requirements 確定後のギャップ分析（kiro-validate-gap・2026-07-17）
> **位置づけ**: 情報提供であって決定ではない。選択肢と根拠を並べ、決裁は要件ディスカッションおよび design フェーズに委ねる。
> **原則**: 正典は ukadoc（一次 SSP HTML を含む）、emo2 は最小適合 fixture にすぎない。

---

## 0. 分析基盤の実測（brief の陳腐化チェック）

記憶 [[parallel-worktree-brief-staleness-rebase-before-design]] に従い、brief の file:line 引用を origin/main へ再突合せした。

| 項目 | 実測値 |
|---|---|
| HEAD | `5a1f6136`（本ブランチ・requirements.md＋spec.json を追加） |
| origin/main | `653ae3ea`（`docs(roadmap): 追記㉚` #67） |
| merge-base | `dd888f2f` |
| HEAD↔origin/main 差分 | `.kiro/steering/focus.md`・`.kiro/steering/roadmap.md`・本 spec の requirements.md/spec.json のみ |

**結論: crate コードは origin/main と同一**。したがって brief の実装偵察（2026-07-16）の file:line 引用は現時点でも有効であり、本分析はワークツリー上のコードをそのまま origin/main の実測として扱える。cue-playback-duration（#60）は既にマージ済みで、settled cue モデルは全て main 上に在る。

**ウェーブ編成上の位置**（roadmap 追記㉙/㉚）: 本 spec は **W1（先鋒・上流なし）**。編集面の事前割当は **「dola ＋ sakura compile ＋ move 結線」**。この割当は後述の「dola `CuePlayer` を触ってよいか」に直接効く（＝割当内）。`mayuna-compose`（W2）は compile.rs／`CueCommand` 衝突を避けるためウェーブで直列化済み。

---

## 1. 現状資産の棚卸（実測）

### 1.1 ② parsers — 完了済み（本 spec に parser 作業なし）

`crates/areka-parsers/src/sakura/model.rs` に 4 語彙すべてが転記済みで実在する:

| variant | 定義 | 備考 |
|---|---|---|
| `Choice(Choice)` | `model.rs:44` | `Choice{ disp, target, references: Vec<String> }`（`:97-104`） |
| `Cursor { x: String, y: String }` | `model.rs:46` | 文字列保持＝単位付き・空の区別を保つ |
| `Move(MoveArgs)` | `model.rs:54` | `MoveArgs{ args: Vec<String> }`（`:111-114`）＝生引数列 |
| `SystemVar(String)` | `model.rs:56` | 展開なしトークン |

`Instruction` は `#[non_exhaustive]`。R1.3（references 欠落なし）・R3.2（空の区別）・R4.2（空引数保持）に必要な情報は**上流に既に在る**。

### 1.2 ④ sakura compile — 4 語彙は catch-all で無音落ち（本 spec の主戦場）

- **catch-all**: `crates/areka-sakura/src/compile.rs:120-122` — `other => tracing::debug!(instruction = ?other, "M-boot 外タグを無視")`。Choice/Cursor/Move/SystemVar/GenericCommand/Raw が全てここへ落ちる。
- **除外の檻**: 同 `:511-544` `m_boot_outside_tags_are_ignored_without_cue_or_panic` が「Choice/Cursor/Move/SystemVar は 0 cue」を固定している。**R8.3 が要求する「意図的更新」の対象はこのテスト**。
- **`emit()` は `CueCommand` 専用**（`:146-153`）— `CuePayload::Command(command)` を固定で組む。**barrier（`CuePayload::Barrier`）を積む発行口が存在しない**（Missing・R2.1）。
- 既存規律（R8.4 が「一貫して適用」を要求）: 冒頭 `ClearAll` 単一前置（`:132-134`）・テキスト D 焼き込み＋`offset += D`（`:63-67`）・scope 転写（`:58-60`）・`End`/`Quit` 切詰め（`:108-116`）。

### 1.3 dola cue — 受け皿は「半分」在る（実測で精査）

| 資産 | 位置 | 状態 |
|---|---|---|
| `CueCommand`（10 variant） | `command.rs:126-161` | `Choice{ id, text }` 実在（`:134`）。**Cursor／Move は不在**（Missing） |
| `BarrierKind::WaitForChoice{ timeout: Option<f64> }` | `command.rs:91` | 実在。`timeout: None`＝無期限（R2.6 に適合） |
| `CuePayload::Barrier` | `command.rs:173` | 実在 |
| `to_talk_schedule` の barrier 写像 | `sheet.rs:185-187` | `CuePayload::Barrier → Entry::Barrier` **実装済み**＝台本から barrier が schedule へ通る |
| `TimedSchedule` の barrier 停止 | `schedule.rs:206-226` | barrier 到達で `current_barrier` を立てて `return`（停止） |
| `CuePlayer` 状態機械 | `runtime.rs:65-74` | `WaitingForChoice` 実在 |
| `pending_choices` 先積み | `runtime.rs:98, 193-203, 355-357` | 実装済み |
| `resolve_choice(id) -> Option<String>` | `runtime.rs:279-293` | 実装済み（id 照合・不一致は `None`） |
| `cue_target_of` | `sink.rs:50-67` | `Choice → Balloon`（`:60`）。catch-all なし＝**variant 追加時にコンパイラが網羅性を強制** |

### 1.4 ④ sakura drive — 完了判定は `is_completed()` gated

`drive.rs:270-290` `settle_after_tick` が `player.is_completed()` の真偽のみで `TalkDone` 送出／`Driving` 継続を分岐する（`:277`）。`SakuraMsg` は `Start`/`Tick`/`Close` の**3 種のみ**（`contract.rs:24-34`・`#[non_exhaustive]`）。

### 1.5 ⓪ ghost / placement — 消費 API は在るが眠っている

- `move_window_to(world, window, x, y) -> bool`（`crates/areka/src/placement/follow.rs:500`）: `#[allow(dead_code)]`＋実コメント「呼び出し側（UI 配送ブリッジ結線）は後続 spec の領分（7.3）」。`BalloonFollow` による**バルーン随伴移動込み**（`:507-516`）＝R5.3 を既に満たす。対象不在／`WindowHandle` 未付与は `warn!`＋`false`（`:495-496`）＝R5.5 の縮退に適合。
- **座標契約**: 「物理 px 素通し（U4・再スケールなし）」（`:493`）＝**絶対スクリーン物理 px**。
- **sink スロットは 2 個固定**: `GhostBootOptions{ ghost_root, default_encoding, shiori, surface_sink, text_sink, ticker }`。`areka-ghost/src/sink.rs:52` に「`boot` の 2 スロット構造（`surface_sink`/`text_sink`）を保ったまま」と**明文の設計意図**がある。
- **sink デコレータの前例**: `ClockedTextSink<T: CueSink + Clone>`（`crates/areka/src/emo2_boot/talk_clock.rs:85-109`）＝内側 sink へ非改変転送しつつ横入りする既存パターン。
- **UI 配送の前例**: `PresentBridge`（`emo2_boot/mod.rs:269`）。

### 1.6 fixture 実測（emo2）

| 用途 | 位置 | 実物 |
|---|---|---|
| メインメニュー | `menu.pasta:15` | `\q[おしゃべり頻度,Onおしゃべり頻度メニュー]\n\q[エモの位置調整,Onエモの位置調整メニュー]\_l[5em,2lh]\q[閉じる,Onメニュー閉じる]` |
| 頻度メニュー | `menu.pasta:33` | `\q`×3＋`\n`×2＋`\_l[5em,2lh]`＋`\q[もどる,...]` |
| 位置調整メニュー | `menu.pasta:62` | `\q[調整,...]\_l[5em,2lh]\q[もどる,...]` |
| 位置調整実行 | `menu.pasta:65` | `\1\![move,-353,,,0,base,base]` |
| **初回起動** | `boot.pasta:79`（`＊OnFirstBoot` 直下の最初の行） | `\1\![move,-353,,,0,base,base]` |
| 撫で talk | `touch.pasta:78, :99` | `%username` を含む地の文 |

`\q` は**全て 2 引数形**（references 空）で、ID は `On〜` 形。`\![move]` は 2 箇所とも**同一の引数列**。

---

## 2. 要件 → 資産マップ（gap タグ）

| 要件 | 依存資産 | gap |
|---|---|---|
| R1.1/1.2 `\q`→choice cue・ラベル/ID 分離 | `CueCommand::Choice{id,text}` 実在 | **Constraint**: parser `disp/target` → dola `text/id` の名称写像を確定するのみ |
| R1.3 references 欠落なし | `Choice{id,text}` に**載せ先なし** | **Missing**（§5.1 で選択肢提示。R8.1 のワイヤ形不変と両立が要る） |
| R1.4 不透明転写・ID 解釈なし | compile が純関数 | なし |
| R1.5 現在スコープ帰属 | `emit()` の scope 転写（`compile.rs:146-153`） | なし |
| R1.6 記述順保存 | compile の逐次走査＋`CueSheet::new` 安定ソート＋同一 `at` FIFO | なし |
| R1.7 旧仕様形/`script:` 形の縮退 | parser 側の転記に依存 | **Unknown**（parser がこれらをどの variant へ落とすか要確認・emo2 未使用） |
| R2.1 barrier をちょうど 1 つ発行 | `BarrierKind::WaitForChoice` 実在／**compile に発行口なし** | **Missing**（`emit()` は `CueCommand` 専用・§1.2） |
| R2.2 全 choice cue より後 | compile の順序制御 | なし（compile 側で末尾に積むだけ） |
| R2.3 barrier 未解決中は talk 完了扱いしない | `TimedSchedule::is_completed()`＋`CuePlayer::tick`＋`drive.settle_after_tick` | **なし＝既存コードで構造的に充足**（§3.B に実測根拠） |
| R2.4 解決で再開 | `CuePlayer::resolve_choice` 実装済 | **Missing（seam）**: `SakuraMsg` に到達口がない（§3.B） |
| R2.5 `\q` 無しなら barrier 無し | compile の条件分岐 | なし |
| R2.6 タイムアウト指定しない | `WaitForChoice{ timeout: None }` | なし（語彙は保持済み） |
| R3.1–3.5 `\_l`→cursor cue（不透明） | **`CueCommand` に Cursor なし** | **Missing**（additive variant 追加） |
| R4.1–4.4 `\![move]`→move cue（不透明） | **`CueCommand` に Move なし** | **Missing**（additive variant 追加） |
| R4.5 move 以外は従来通り記録して継続 | catch-all 維持 | なし |
| R5.1 実際に窓が移動 | `move_window_to` 実在（dead_code） | **Missing（結線）**: sink スロット＋UI スレッド配送（§5.3） |
| R5.2 引数意味論を canon で解決 | — | **解決（canon あり）**＋一部 canon 沈黙（§3.A） |
| R5.3 バルーン随伴 | `BalloonFollow`（`follow.rs:507-516`） | なし＝既存 API が内包 |
| R5.4 time 付きは即時へ縮退・語彙保持 | — | なし（fixture は time 空＝canon 既定 0＝そもそも即時） |
| R5.5 対象不在は warn＋継続 | `move_window_to` の `warn!`＋`false` | なし |
| R6.1/6.2 永続値を書かない | `move_window_to` は DragEnd 観測点を経ない | **Constraint**: 永続状態が未実装（position-persist＝W3）＝檻の形が要決裁（§4.4） |
| R7.1/7.2 `%username` 展開・テキスト同格 | compile に展開アームなし | **Missing**（純関数＋Text 合流） |
| R7.3 値源を外部注入 | `GhostBootOptions` が自然な受け皿 | **Missing**（field 追加） |
| R7.4 未注入時は既定値 | — | **Unknown**（canon 沈黙・§3.C） |
| R7.5 未対応 `%名` は素通し | — | **Unknown**（canon 沈黙・§3.C） |
| R7.6 名前→値の写像・外部環境を読まない | — | なし（設計規律） |
| R8.1 既存 cue のワイヤ形不変 | `command.rs:462-507` が**期待 JSON リテラルで 8 variant を固定** | **Constraint**（強い檻・§5.1 に影響） |
| R8.2 未対応タグは記録して継続 | catch-all 維持 | なし |
| R8.3 除外集合から 4 語彙が卒業 | `compile.rs:511-544` の檻 | **Constraint**（意図的更新の対象） |
| R8.4 既存台本規則を対象タグにも一貫適用 | compile の既存構造 | なし |
| R8.5 無関心な表現者は良性スキップ | `cue_target_of` の網羅 match | **Constraint**: Cursor/Move の分類先が要決裁（§5.4） |
| R9.1–9.5 決定論検証 | 既存の純関数檻の流儀 | なし |
| R9.6 実機サインオフ | 実 emo2＋実 SHIORI＋実 DPI | **Constraint**（絶対パス必須・[[areka-placement-real-ghost-first]]） |

---

## 3. 申し送り 3 議題（第一級・要件ディスカッションの主題）

### 3.A 議題①: positional `\![move,...]` の正典 — **解決した（canon は存在する）**

**問題の再現**: ukadoc MCP スナップショットの `\![move]` 項（`ukadoc:list_sakura_script:_5c_21_5bmove_5d:1`）は本文が「指定座標まで移動。**※下記参照**」＋named-arg の記述例のみで、**引数表そのものを落としている**。MCP だけを見ると `\1\![move,-353,,,0,base,base]` に正典の裏付けが無いように見える。

**実測**: 一次 SSP HTML（`https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html`）を WebFetch した結果、**legacy positional 形が正典に明記されている**ことを確認した:

```
\![move(async),X座標,Y座標,移動時間,移動基準,基準とするキャラ基準位置,動かすキャラ基準位置]
```

（新形＝SSP 2.3.85+ の `--X=` / `--Y=` / `--time=` / `--base=` / `--base-offset=` / `--move-offset=` / `--option=` は「全て省略可・順不同・フラグ名は大小文字不問」。両形が併存する。）

| 位置 | 引数 | 省略時（canon） |
|---|---|---|
| 1 | X 座標 | `"fix"`＝**その座標を保持**（原文「どちらかを省略するか、"fix"と指定した場合は、その座標を保持する。省略時は"fix"」） |
| 2 | Y 座標 | 同上 `"fix"` |
| 3 | 移動時間（ms） | `0`＝**即時** |
| 4 | 移動基準 | `screen` |
| 5 | 基準とするキャラ基準位置 | `left.top` |
| 6 | 動かすキャラ基準位置 | `left.top` |

- **移動基準**の語彙: `screen`（現在のモニタ）／`primaryscreen`／**数値スコープ ID**（原文「0,1等で指定したID番スコープ(`\0`,`\1`,`\p[2]`などの数に相当)のキャラクターとの相対座標」）／`me`（自分自身）／`global`（システム座標）。
- **基準位置**の語彙: `X基準.Y基準` 形。X ∈ {`left`,`right`,`base`,`center`} / Y ∈ {`top`,`bottom`,`base`,`center`}。原文「なお、**base は surfaces.txt 内の point.basepos 指定に従う**」。

**fixture `\1\![move,-353,,,0,base,base]` の canon 逐語解**:

| 位置 | 実値 | 解 |
|---|---|---|
| 1 X | `-353` | X = 基準点から −353px |
| 2 Y | （空） | **fix＝現在の Y を保持** |
| 3 time | （空） | **0＝即時**（R5.4 の「time 付きは縮退」は本 fixture では**そもそも発生しない**） |
| 4 移動基準 | `0` | **スコープ 0（本体＝むらさき）との相対座標** |
| 5 base-offset | `base` | scope0 の basepos |
| 6 move-offset | `base` | エモ自身の basepos |

→ 意味: **「エモ（`\1`）を、むらさき（scope 0）の basepos から X 方向 −353px の位置へ、Y は現状維持のまま、即時に移動する」**。brief の「エモが横へ動く」と一致し、`\1` 帰属（R4.4）とも整合する。

**残る canon 沈黙点（＝areka 裁量＋対応表・R5.2 が明示的に許す領域）**:

1. **裸の `base`（ドット無し）**: canon の形式は `X基準.Y基準` であり、`base` 単独は形式に非適合。しかし fixture は 2 箇所ともこれを使う（実在するゴーストの de-facto）。妥当解＝**両軸とも base**（`base.base` と等価）。要対応表記録。
2. **`point.basepos` が codebase に一切存在しない**（`grep basepos` → **0 件**・fixture の `surfaces.txt` を含む）。
   - canon 既定（`descript_shell_surfaces`）: **`point.basepos.x` 既定＝サーフェスの中心（幅÷2）／`point.basepos.y` 既定＝サーフェスの下端**。
   - **emo2 の `shell/master/surfaces.txt` は `point.basepos` を宣言していない**（実測）→ **canon 既定がそのまま適用される**。
   - ゆえに **既定 basepos（幅÷2, 下端）だけを実装すれば、emo2 は canon 通りに動く**。しかも本 fixture は Y=fix なので、**実際に効くのは `basepos.x = 幅÷2` のみ**。
   → §5.2 の選択肢へ。

**帰結**: 「一次 HTML を取りに行く vs areka 裁量」の二択ではなく、**一次 HTML に正典が在ったので裁量は 2 点（裸 base・宣言 basepos の先送り）に縮んだ**。MCP スナップショットの欠落は既知の限界として扱い、正典参照は一次 HTML を併用する。

### 3.B 議題②: barrier vs horizon の緊張 — **矛盾しない（実測で確認）**

**結論**: settled な「占有 horizon 到達で完了・早期終了しない」と R2.3「選択肢を持つ talk は horizon で完了しない」は**直交する条件**であり、両立する。horizon は「**これより早く終わってはならない**」という**下限**であって「horizon に達したら必ず終わる」という上限ではない。barrier は「解決されるまで終わらない」という**別の停止条件**。両者は AND で結合されている。

**コード上の実測根拠**（すべて origin/main 同一のワークツリー実測）:

1. `TimedSchedule::is_completed()`（`schedule.rs:289-293`）:
   ```rust
   self.entries.is_empty()
       && self.current_barrier.is_none()   // ← barrier 保持中は false
       && self.current_offset >= self.horizon
   ```
   barrier が立っている限り `is_completed()` は**構造的に false**。
2. `CuePlayer::tick`（`runtime.rs:221-248`）: **barrier 判定を `is_completed()` 判定より先に行い `return` する**。
   ```rust
   Some(BarrierReached::Choice) => { self.state = WaitingForChoice; return; }   // :231-237
   ...
   if self.schedule.is_completed() { self.state = Completed; }                  // :246-248（到達しない）
   ```
   → **単一の tick が barrier 時刻と horizon の両方を飛び越えても、barrier が勝つ**。
3. `CuePlayer::tick` 冒頭（`:179-182`）: `Playing` でなければ `filtered_ready` を clear して early-return＝待機中は一切進行しない。
4. `drive.settle_after_tick`（`drive.rs:277`）: `player.is_completed()` が偽なら `Driving` を書き戻して `Continue`＝**`TalkDone` を送らない**。

→ **R2.3 は新規機構ゼロで既に充足される**。design で「horizon 完了と barrier の調停ロジック」を新設する必要は**ない**（作れば二重実装＝[[areka-cue-runtime-consolidated-in-dola]] 違反）。

**ただし真のギャップはここではなく seam にある（Missing）**:

- `SakuraMsg` は `Start` / `Tick` / `Close` の**3 種のみ**（`contract.rs:24-34`）。`CuePlayer` は `TalkPhase::Driving` の中に**所有されて閉じている**（`drive.rs:113-121`）。
- ゆえに **talk アクター境界の外から `resolve_choice(id)` へ到達する経路が存在しない**。R2.4「選択が解決されると再開する」は `CuePlayer` 単体としては実装済みだが、**誰も呼べない**。
- 「選択の解決を起こすのは下流（choice-render／choice-select-events）」（Boundary Context）である以上、**その口の形＝本 spec が正本として決めるべき契約**。`SakuraMsg::ResolveChoice(String)` 相当の additive 拡張（`#[non_exhaustive]` ゆえ追加は安全）が最有力だが、決裁は design。
- **副次論点**: `resolve_choice` 後 `settle_completion_after_resolve`（`runtime.rs:315-319`）が即 `Completed` を確定するが、**`drive` は `on_tick` でしか `settle_after_tick` を呼ばない**＝`TalkDone` は**次の `Tick` まで出ない**。`TickerMode::Real` の継続 tick 前提なら無害だが、design で明示確認したい（R-5）。

### 3.C 議題③: `%username` の値源・既定値・未知名

**canon の実測**:

| 出典 | 内容 |
|---|---|
| `ukadoc:list_sakura_script:_25username` | 「%username／ユーザー名。」**それだけ**。既定値の規定なし |
| `ukadoc:list_sakura_script:環境変数の記述例` | 例文「%username、ちょっと聞いてよ。%ms が…」のみ。既定値・未知名の規定なし |
| **`ukadoc:list_shiori_resource:username`** | **「username／ユーザーの名称。」＝`username` は SHIORI リソース**として canon に存在する |
| `ukadoc:list_propertysystem:username` | 「対象の username(呼ばれ方)。同時起動している場合、かつ相手が username の取得に対応している場合のみ取得できる。SSP 2.3.51 以降」＝**別概念**（他ゴーストの username 取得） |

→ **canon は「baseware 既定値」と「未知 `%名` の挙動」の双方について沈黙**（parent の指摘どおり確認）。一方で**値の正典的な源は SHIORI リソース**である、という重要な示唆が canon に在る（baseware の config ではない）。YAYA/里々 の Tips でも `username` はゴースト側変数として保持され、未設定時はゴースト自身が「名無し」等を代入している（＝**既定値はゴーストの責務**という de-facto）。

**要件との関係**: R7.3 は「起動構成として外部から注入可能（ハードコードしない）」、R7.4 は「未注入時は既定値へ展開（決定論的）」と**既に決めている**。したがってギャップは「M1 の値源をどうするか」ではなく、

> **注入シームの形を、将来 SHIORI リソース源（正典の値源）へ差し替えられる形にできるか**

である。記憶 [[defer-canon-with-full-vocabulary-and-tracking-spec]]（語彙を完全形で第一級保持・源のある分だけ実導出・残は縮退＋差替シーム・追跡 spec＋roadmap 宿題）の 4 点セットが直接適用できる。

**`%selfname`/`%keroname` の生源**: descript の `name`／`kero.name` が package-mount で既に着地している（記憶 [[areka-ghost-boot-descript-not-install]]）。R7.5 の素通し縮退で M1 は据え置くが、**名前→値の map（R7.6）を第一級に持てば、源が在る分だけ just-in-time で実導出できる**（Boundary Context「源が着地した時点で just-in-time」と整合）。→ §5.5 の選択肢へ。

---

## 4. brief 未記載の新規発見ギャップ（重要）

### 4.1 【最重要】`Choice` cue は sink へ broadcast されない — 下流が受け取れない

**実測**: `CuePlayer::tick`（`runtime.rs:191-216`）は ready から `Choice` を**分離**して `pending_choices` へ先積みし、**`filtered_ready` に入れない**。broadcast は `filtered_ready` のみを配る:

```rust
for cue in self.schedule.ready() {
    match &cue.command {
        CueCommand::Choice { id, text } => { self.pending_choices.push(...); }   // ← 分離
        _ => self.filtered_ready.push(cue.clone()),
    }
}
...
if self.schedule.remaining() < remaining_before {
    for cue in &self.filtered_ready { for sink in self.sinks.iter_mut() { sink.emit(cue.clone()); } }
}
```

この挙動は檻で固定済み — `crates/dola/tests/cue/runtime_test.rs:156-163`:
> 「Choice は ready の action cue として surface されない（先積みプロトコル）」

**矛盾**: 一方で relevance 単一権威 `cue_target_of(Choice) = Some(CueTarget::Balloon)`（`sink.rs:60`）は「Choice は Balloon 演者（emo-text／choice-render）の担当」と宣言している。**分類器は Balloon 行きと言うが、実際には Choice はどの sink にも届かない**＝現状は**死んだ分類**。

**fixture が壊れる**: `menu.pasta:15` は

```
\q[おしゃべり頻度,…]  \n  \q[エモの位置調整,…]  \_l[5em,2lh]  \q[閉じる,…]
```

＝**choice 群と `\n`／`\_l` が交互に並ぶ**。Choice を平坦な `pending_choices` バッグへ分離すると、演者が観測できるのは

- broadcast 列: `[ClearAll, NewLine, Cursor(5em,2lh)]`
- 別バッグ: `[頻度, 位置調整, 閉じる]`

となり、**「閉じる はカーソル移動の後に置かれる」「位置調整 は改行の後」という配置情報が再構成不能**。3 メニューすべてが同型（`menu.pasta:15/33/62`）で、`\_l` は常に**最後の選択肢の直前**に置かれる＝「戻る/閉じるを定位置に置く」という体裁の意図が明白。これが再現できないと **R3（`\_l`＝選択肢の区切り位置指定）が空証明**になり、Introduction の「メニューの体裁が崩れる」を解消できない。

**なぜ本 spec の決裁事項か**: R1 は本 spec を「choice cue 形＝下流の正本」と定義し、choice-render（W4）／choice-select-events（W5）は「消費のみ」。**消費の seam が届かない形のままでは下流が設計不能**。かつ編集面割当（W1＝dola を含む）により、`CuePlayer` の変更は本 spec の担当範囲内。

**なお R9.2 との関係**: R9.2 の「期待される cue 列」は **compile 出力の `CueSheet`** に対する檻であり、compile は記述順を保つので R9.2 自体は充足可能。**壊れるのは配送側**であり、R9.2 だけを緑にしても実機のメニュー体裁は直らない（＝檻が実機を担保しない典型）。

→ 選択肢は §5.1。

### 4.2 `emit()` に barrier 発行口が無い

`compile.rs:146-153` の `emit()` は `CueCommand → CuePayload::Command` 固定。R2.1 の barrier 発行には `CuePayload::Barrier(BarrierKind::WaitForChoice{ timeout: None })` を積む別口が要る（機械的・小規模）。`Cue.duration` は barrier では 0（`sheet.rs` の doc「Barrier/Routing は presentation でなく…値は 0」）。

### 4.3 move 消費者の「座」が無い＋スレッド境界

- **座**: `GhostBootOptions` は `surface_sink`/`text_sink` の **2 スロット固定**で、`areka-ghost/src/sink.rs:52` に「2 スロット構造を保ったまま」と明文の設計意図がある（`DiscardSink` はそのために存在する）。Move の消費者を置く場所が無い。
- **スレッド**: `CueSink::emit` は **talk アクタースレッド**上で呼ばれる（`drive.rs` の `spawn_actor` 内）。`move_window_to(&mut World, ...)` は **UI スレッド専用**（bevy `World`・D2D 単一スレッド＋window アフィニティ・[[areka-concurrency-model]]）。**直接呼べない**＝`PresentBridge` 相当の UI 配送ブリッジが必須。
- **参考**: `CuePlayer` 側は `sinks: Vec<Box<dyn CueSink>>`＋`register_sink`（`runtime.rs:103, 155`）で**既に任意個の sink に対応済み**。制約は `spawn_talk` の署名と `GhostBootOptions` の型だけ。

### 4.4 R6／R9.5 の「永続化を汚さない」檻は対象が未実装

`areka-P0-position-persist` は **W3・未着手**（本ブランチに永続状態の実装なし）。「永続値を書かない」ことの決定論 assert は、**まだ存在しない永続状態**に対しては直接書けない。現状 `move_window_to` は DragEnd 観測点（`on_char_drag_end`/`on_balloon_drag`）を経ないので構造的には満たすが、**檻の形（何を観測して否定するか）**は design 決裁。候補: (a) 経路の構造檻（`move_window_to` が単一ライター経路を経ないことをコードで固定）、(b) position-persist 着地後に檻を追加する申し送り、(c) 「第二の位置ライターを作らない」ことの型シーム。

---

## 5. 実装アプローチの選択肢

### 5.1 【核心】choice の配送 seam（§4.1 への解）

- **案 A: `Choice` も broadcast する**（分離を外し、`pending_choices` は resolve 照合用の**並行 index** として維持）
  - ✅ 記述順・交互配置が broadcast 列に保存＝**fixture メニュー体裁が再構成可能**・R3 が意味を持つ
  - ✅ `cue_target_of(Choice)=Balloon` という settled 分類器と**コードが一致**（死に分類を解消）
  - ✅ 編集面は W1 割当内（dola）
  - ❌ settled な cue-playback の観測挙動を変更＝`runtime_test.rs:156-163` の檻を**仕様変更として意図的更新**（R8.3 と同型の手続きが要る）
  - ❌ 下流が Choice を 2 経路で見る（broadcast＋`pending_choices`）＝役割の説明責任
- **案 B: 先積みのまま・choice-render が `pending_choices` を pull**
  - ✅ settled コード無改変
  - ❌ `CuePlayer` は talk アクター内に閉じており、sink である choice-render から**到達不能**（新たな pull 経路＋ロック/共有が要る＝受動ランタイムの設計思想に逆行）
  - ❌ **交互配置が失われ fixture メニューが再現不能**＝R3 が空証明・Introduction の「体裁が崩れる」を解消できない
- **案 C: 責務で二分した二重（案 A の明示版）**
  - **broadcast＝配置/表示情報**（choice-render が cue 列として消費）／**`pending_choices`＝解決照合**（choice-select-events が `resolve_choice` の id 照合に使う）
  - ✅ 下流 2 spec の責務境界（表示 vs 解決）と**そのまま一致**＝契約が説明しやすい
  - ✅ 案 A の利点をすべて持つ
  - ❌ 契約文書量が増える
- **検討順の目安**: **C ≈ A ＞ B**（B は R3／fixture 要件を満たせない公算が高い）

### 5.2 `\![move]` 意味論の実装範囲（§3.A への解）

- **案 A-1: canon 既定 basepos のみ実装＋宣言 `point.basepos` は型シーム**
  - `basepos = (幅÷2, 下端)`（canon 既定）を placement 側で算出。**emo2 は宣言なし＝canon 既定が適用される正規の経路**であり、fixture は canon 通りに動く
  - 本 fixture は Y=fix ゆえ実際に効くのは `basepos.x = 幅÷2` のみ
  - 宣言 `point.basepos` の実導出（shell-parse 拡張）は**追跡 spec ＋ roadmap 宿題**へ＝[[defer-canon-with-full-vocabulary-and-tracking-spec]] の 4 点セット
  - ✅ W1 の編集面割当内・実機で canon 一致・語彙は完全形で保持
- **案 A-2: `point.basepos` を本 spec で実導出**
  - ❌ 完了済 `shell-parse` の編集面へ越境・W1 割当（dola＋sakura compile＋move 結線）を超える
  - ❌ **emo2 が宣言しないので実機で観測不能**＝檻が空回りする（既定経路しか走らない）
- **案 A-3: 裸 `base` を canon 既定 `left.top` へ丸める**
  - ❌ canon は「base は point.basepos に従う」と**明記**＝沈黙ではなく**明確な逸脱**。実機で幅÷2 分ずれ、R9.6 の目視サインオフで露見する
- **検討の目安**: **A-1**

### 5.3 move 消費者の座（sink スロット・§4.3 への解）

- **案 S-1: 3 スロット化**（`GhostBootOptions` に `move_sink` 追加・`spawn_talk` を 3 sink へ）
  - ✅ 「1 演者＝1 sink」の原則に忠実
  - ❌ 2 スロット構造（`sink.rs:52` 明文）＋`spawn_talk` 署名＋全 boot 呼出（test 含む）へ波及。次の演者（W4 choice-render）でまた 4 スロット化する圧力
- **案 S-2: デコレータで既存 sink を包む**（`ClockedTextSink` 前例・`talk_clock.rs:85-109`）
  - ✅ 署名不変・既存パターン再利用・波及最小
  - ❌ 「Move は seriko の cue ではない」のに surface スロットへ相乗り＝層の説明が濁る（名前負債の再生産）
- **案 S-3: sink を可変長へ**（`Vec<Box<dyn CueSink>>` / builder）
  - ✅ **`CuePlayer` は既に任意個対応**（`runtime.rs:103, 155`）＝制約は `spawn_talk`／`GhostBootOptions` の型のみ
  - ✅ W4 choice-render の演者追加にも効く（同じ議論を 2 度しない）
  - ❌ boot 呼出側の書き換え＋「2 スロット構造」の意図的更新
- **検討の目安**: **S-3 ≳ S-1 ＞ S-2**（`CuePlayer` が既に Vec ゆえ S-3 が構造に素直）
- **いずれの案でも必須**: UI スレッド配送（`PresentBridge` 相当の `UiSender`）。sink は talk スレッド上。

### 5.4 dola cue 語彙の増分形

- **Cursor / Move**: **新規 variant 追加**（externally tagged ゆえ既存 8 variant のワイヤ形は不変＝R8.1 適合・`balloon-face-cue` の `BalloonSurface` 追加が前例）。ペイロードは不透明（[[areka-surface-args-opaque-string-downstream-resolve]]）: `Cursor{ x: String, y: String }` / `Move{ args: Vec<String> }` が parser 形と素直に対応。
- **`cue_target_of` の網羅**（`sink.rs:50-67` は catch-all 無し＝**コンパイラが追加を強制**）:
  - `Cursor → Balloon`（emo-text／choice-render が消費）が自然。
  - `Move` は **Shell でも Balloon でもない**（ghost/placement 行き）。選択肢: (i) `CueTarget` に第 3 variant（例 `Window`/`Placement`）を additive 追加（`CueTarget` は serde 型・`EntityKey`/`RoutingCommand` から参照されるが variant 追加は後方互換）、(ii) `None` を返す（＝`Wait`/`Custom` と同じ「担当演者なし」扱い。しかし ghost が実際に担当するので分類器が再び嘘をつく）。→ **(i) を推奨検討**。
- **`Choice` の references（R1.3・§2 の Missing）**: `CueCommand::Choice{id,text}` に載せ先がない。**R8.1 の制約が強い** — `command.rs:462-507` の `existing_eight_variants_wire_forms_are_unchanged_by_additive_extension` が期待 JSON リテラル `{"Choice":{"id":"yes","text":"はい"}}` を**そのまま固定**している。
  - **案 (i)**: `Choice` に `#[serde(default, skip_serializing_if = "Vec::is_empty")] references: Vec<String>` を追加。→ references 空（＝emo2 の全 `\q`）では**シリアライズ形が現行と完全同一**＝上記の檻を壊さず R8.1 を満たし、旧資産も `default` で読める。R1.3 も満たす。**最有力**。
  - **案 (ii)**: 別 variant（`ChoiceEx` 等）を新設 → 同型 2 変換の重複＝[[areka-cue-runtime-consolidated-in-dola]] の精神に反する。
  - **案 (iii)**: references を捨てる → **R1.3 に正面から違反**（emo2 未使用でも「欠落させない」が要件）。
  - 名称写像: parser `disp/target` → dola `text/id`（R1.2 の「区別可能な別データ」は既に満たす）。

### 5.5 `%username` 展開と値源（§3.C への解）

- **展開位置**: compile で `SystemVar(name)` → 値を `CueCommand::Text` へ合流（R7.2「通常のテキストと同じ扱い」＝D 焼き込み・記述順保存が自動的に効く）。emo-text は関知しない。
  - **論点**: 展開後テキストと隣接する地の文の **Text cue 併合可否**。`touch.pasta:78` は `仕方ないなあ～、%usernameったらもう♪` ＝ `Text("仕方ないなあ～、") SystemVar("username") Text("ったらもう♪")`。併合しないと 3 つの Text cue になり、**D（再生時間）は文字数比例ゆえ合計は不変**だが cue 数と R9.4 の期待列が変わる。design 決裁事項。
- **値源の形**:
  - **案 U-1: `GhostBootOptions` に `system_vars: BTreeMap<String,String>`（名前→値の map）**＋既定値
    - ✅ R7.6「名前→値の写像」を**そのまま型で表現**・語彙を完全形で第一級保持（[[defer-canon-with-full-vocabulary-and-tracking-spec]]）
    - ✅ `%selfname`/`%keroname` は descript の `name`/`kero.name` から**源が在る分だけ just-in-time で実導出**へ拡張できる（Boundary Context と整合）
    - ✅ 将来 SHIORI リソース源（canon の正典的な値源）へ差し替えるシームになる
  - **案 U-2: `username: Option<String>` 単発 field**
    - ✅ 最小
    - ❌ 語彙の第一級保持に反する（`%username` だけ特別扱い＝素の最小化＝[[defer-canon-with-full-vocabulary-and-tracking-spec]] が戒める形）
  - **検討の目安**: **U-1**
- **既定値（R7.4・canon 沈黙）**: 決定論であればよい。候補: `"あなた"`／`"ユーザー"`／`"名無し"`（YAYA Tips の de-facto）。**対応表へ記録**。
- **未知 `%名`（R7.5・canon 沈黙）**: 「元の記述 `%名前` をテキストとしてそのまま出力＋記録」と**要件が既に確定**。map に無い名前＝素通し、で実装は自明。ただし**どこまでが「システム変数名」か**（`%m*` 系・`%property[...]` の括弧形）の切り出しは parser 側の `SystemVar(String)` に依存 → R-1 相当の確認事項。

### 5.6 全体アプローチ（A/B/C 枠）

- **Option A（既存拡張中心）**: dola cue 語彙に Cursor/Move を additive 追加・`Choice` に references 追加・compile へ 4 アーム＋barrier 発行口・`CuePlayer` の Choice 配送更新・move 消費は既存 sink のデコレータ（S-2）。
  - ✅ 新規ファイル最小 ❌ move が seriko スロット相乗りで層が濁る
- **Option B（新規中心）**: 新しい cue ランタイム／別配送機構を建てる。
  - ❌ **不採用が妥当**: cue 再生制御の dola 一本化は settled（[[areka-cue-runtime-consolidated-in-dola]]）。二重ランタイム＝車輪の再発明。
- **Option C（ハイブリッド・推奨検討）**: 語彙・compile・Choice 配送は既存拡張（A と同じ）＋ **move 末端消費のみ新規コンポーネント**（ghost 側 `MoveSink` ＋ UI 配送ブリッジ）＋ **sink スロットの意図的更新（S-3）**。
  - ✅ 「cue 制御は dola・演者は各自 1 sink」という settled 構造に忠実／move の層帰属が明確／W4 の演者追加にも効く
  - ❌ boot 呼出側（test 含む）の書き換えが要る
- **検討の目安**: **C**

---

## 6. Effort / Risk

| 境界 | Effort | Risk | 一行根拠 |
|---|---|---|---|
| dola cue 語彙増分（Cursor/Move variant・`Choice.references`） | **S** | **Low** | externally tagged で既存ワイヤ形不変・`balloon-face-cue` の additive 実績・`skip_serializing_if` で既存檻も維持可 |
| compile 4 アーム＋barrier 発行口＋除外檻の意図的更新 | **S–M** | **Low** | 純関数・全網羅可能・`emit()` に Barrier 口を足すのみ |
| **choice 配送 seam（`CuePlayer` 更新＋settled 檻の意図的更新）** | **M** | **Medium** | settled cue-playback の観測挙動を変更し、下流 2 spec の契約正本になる＝取り返しの効きにくい決裁 |
| choice 解決 seam（`SakuraMsg` 拡張） | **S** | **Low–Medium** | `#[non_exhaustive]` ゆえ追加は安全だが、下流 W5 の消費契約を先決する |
| `%username` 展開＋値源注入 | **S** | **Low** | 純関数＋`GhostBootOptions` への field 追加・決定論 |
| **move 末端結線（sink スロット＋UI 配送＋既定 basepos＋scope 解決）** | **M–L** | **Medium–High** | スレッド境界跨ぎ＋座標系（物理/論理混在の既知欠陥 [[areka-window-placement-dpi-coordinate-defect]]）＋実機サインオフ必須＋scope0 窓位置とサーフェス幅の取得元が未確定 |
| 決定論檻（script→cue 列・全網羅） | **S–M** | **Low** | script 直入力・sleep 不使用・既存の檻の流儀を踏襲 |
| **合計** | **M–L** | **Medium** | 語彙・compile は機械的だが、choice 配送 seam と move 末端の 2 点にリスクが集中 |

---

## 7. Research Needed（design へ持ち越し）

- **R-1**: `\_l` の canon 詳細（em/lh/%/裸数値/`@` 相対/省略の正確な語彙と既定）。ukadoc MCP で `\_l` 項の id を引けなかった（`search_docs` 不一致・`get_doc` の id 推測失敗）。**本 spec は不透明転写ゆえ非ブロッカー**（parser `Cursor{x:String,y:String}` が既に区別を保持）。消費＝choice-render（W4）の領分。必要なら一次 HTML を参照。
- **R-2**: parser が `\q` の旧仕様形（`\q[ID][タイトル]`）・`script:` 形・複数 ID 形を**どの variant へ落とすか**（R1.7 の「記述を失わない縮退」の実体）。emo2 未使用。
- **R-3**: `\![move]` の `--option=` 引数（新形にのみ登場）の語彙。M1 未使用＝型シーム候補。
- **R-4**: 移動基準 `me`/`global`/`primaryscreen` の解決規則。fixture は数値 scope のみ＝縮退＋語彙保持の候補。
- **R-5**: `resolve_choice` 後に `TalkDone` が**次の `Tick` まで出ない**点（`drive` は `on_tick` でしか settle しない）が、`TickerMode` の実運用で問題化しないか。
- **R-6**: **座標系の確定**（最重要の技術リスク）。`move_window_to` は物理 px 素通し（`follow.rs:493`）。一方 scope0 の窓位置・サーフェス幅（basepos.x=幅÷2 の算出元）が**論理 px か物理 px か**を実装前に確定する必要がある。記憶 [[areka-window-placement-dpi-coordinate-defect]]（`Monitor.work_area`=物理・`WindowPos`=物理・`BoxStyle` Px=論理の混在で過去に窓が画面外へ消えた）＋[[areka-placement-real-ghost-first]]（実 DPI で検証せよ）。**dpi=96 では自己整合して差が出ない＝テストで捕まらない**ことに注意。
- **R-7**: SSP の de-facto: script `\![move]` 後の位置保存有無（canon に永続化規定なし）。brief で既知・`emo2-conformance-e2e` の実機判断へ申し送り済み。

---

## 8. design フェーズへの推奨

### 8.1 先に決めるべき契約（下流 W4/W5 が待つ・本 spec が正本）

1. **choice cue の payload 形** — `references` の載せ方（§5.4 案 (i) が R8.1 の檻と両立する唯一の形に見える）。
2. **choice の配送 seam**（§5.1）— broadcast か pull か。**これが本 spec 最大の決裁**。fixture の交互配置（`\q \n \q \_l \q`）を再構成できる形でなければ R3 と Introduction の「体裁が崩れる」が解消しない。
3. **選択待ち barrier の並び規則と解決 seam**（§3.B）— barrier 発行は compile 側で完結するが、`resolve_choice` への到達口（`SakuraMsg` 拡張の要否）は W5 の消費契約を先決する。

### 8.2 推奨アプローチ

**Option C**（§5.6）＋ **§5.1 案 C** ＋ **§5.2 案 A-1** ＋ **§5.3 案 S-3** ＋ **§5.5 案 U-1**。
いずれも「語彙は完全形で第一級保持・源のある分だけ実導出・残は縮退＋差替シーム・先送りは追跡 spec＋roadmap 宿題」（[[defer-canon-with-full-vocabulary-and-tracking-spec]]）と整合する。

### 8.3 決裁の要らない確認事項（既に充足）

- **R2.3（選択肢待ちで完了しない）は既存コードで構造的に充足**（§3.B）。新規の完了調停ロジックを設計しないこと。
- **R5.3（バルーン随伴移動）は `move_window_to` が既に内包**（`follow.rs:507-516`）。
- **R5.4（time 付き移動の縮退）は fixture では発生しない**（`\![move,-353,,,0,base,base]` の time は空＝canon 既定 0＝即時）。語彙保持のみで足りる。

---

## 9. 付録: canon 引用の出典

| 主張 | 出典 |
|---|---|
| `\![move]` legacy positional 形・省略時既定・移動基準の語彙・`base`=point.basepos | 一次 SSP HTML `https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html`（MCP スナップショットは引数表を欠落＝「※下記参照」で切れている） |
| `point.basepos.x` 既定＝幅÷2 / `point.basepos.y` 既定＝下端 | `ukadoc:descript_shell_surfaces:point.basepos.x,座標` / `…point.basepos.y,座標` |
| `%username`＝「ユーザー名。」のみ（既定値・未知名は沈黙） | `ukadoc:list_sakura_script:_25username` / `…:環境変数の記述例` |
| `username` は SHIORI リソースでもある | `ukadoc:list_shiori_resource:username` |
| `username` プロパティは別概念（同時起動相手の呼ばれ方・SSP 2.3.51+） | `ukadoc:list_propertysystem:username` |
| 未設定 username はゴースト側が既定を代入する de-facto | `yaya:Tips/ユーザーの名前を覚える`（「名無し」代入例） |
