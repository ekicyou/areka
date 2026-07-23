# Gap Analysis: areka-P0-seriko-loop

> 実施日: 2026-07-23 ／ 対象: 確定済み requirements.md（R1–R9）＋ brief.md（2026-07-23 全面リフレッシュ版）
> 手法: 既存コードベースの実査（Grep/Glob/Read）による突合。brief の file:line 参照は mayuna-compose マージ前の座標のため一部ズレるが、構造的事実は現行コードで再確認済み。
> 本書は**情報と選択肢**の提示であり、最終決定は行わない（要件ディスカッションへ供給）。

---

## 1. Current State Investigation（実査サマリ）

### 1.1 検証できた brief の主張（現行コードで確認済み）

| 主張 | 実査結果 | 位置 |
|---|---|---|
| seriko 受信面は `Cue \| Close` のみ・時間源なし | ✅ `enum SerikoMsg { Cue(TalkCue), Close }` | `areka-seriko/src/actor.rs:52-58` |
| 単一発行点 `emit_display` が唯一の `SurfaceOutput::send` 呼出 | ✅ 確認。module doc に「時間駆動ループが同じ発行点を再利用できる」と明記 | `actor.rs:14-15, 134-136` |
| `ScopeStates` が `dynamic_binds` を per-scope 保持・`current_binds` が read-only 参照 | ✅ `dynamic_binds: HashMap<ActorKey, BindSet>`／`current_binds` フォールバック | `state.rs:76-96, 239-244` |
| bind ON/OFF を read-only で引ける（R3 ゲート） | ✅ `current_binds(&scope) -> &BindSet` ＋ `BindSet::contains(id)` | `state.rs:239`, `emo-compose/src/bind.rs:30` |
| 合成署名 `compose_into(.., active_binds)` に pattern 状態なし | ✅ `compose_into(out, world, atlas, surface_id, active_binds)` | `emo-compose/src/lib.rs:113-134` |
| 合成キャッシュキーに pattern 追加の予約記述が実在 | ✅ `struct ComposeKey { surface_id, binds }` ＋「将来 seriko がアニメ pattern 状態を…本キーへ追加する」 | `emo-present/src/cache.rs:43-52` |
| pattern0 厳格選択（index==0 のみ・疎最小フォールバック禁止）＝本 spec への宿題明記 | ✅ 「pattern0 を持たない bind animation…それらのフレームは seriko-loop（M-life）が再生する」 | `emo-compose/src/plan.rs:306-318` |
| ghost ticker はクロック注入可・絶対グリッド整列＋catch-up | ✅ `TickerConfig.clock: Box<dyn Fn()->MonotonicMs>`／`BoundarySchedule`（純粋層） | `areka-ghost/src/ticker.rs:47-156` |
| emo2 fixture の 2 系統定義 | ✅ sakura `animation1400.interval,bind+random,4`（pattern1→1412/0, pattern2→1411/150, pattern3→1410/22・pattern0なし・-1なし）／kero `animation0.interval,random,4` | `pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt:73-90, 427-454` |
| parser の interval 語彙 | ✅ `Interval::{Bind, Random{k}, BindRandom{k}}`（`#[non_exhaustive]`）／`Pattern{index:u32, surface_id:i64, wait:u32, x:i64, y:i64}` | `areka-parsers/src/shell/model.rs:107-147` |

### 1.2 brief が過小評価している事実（設計に影響する重要な発見）

**発見 A ── ticker は固定 2 系統であり「汎用 fan-out」ではない。**
brief は「`spawn_ticker<D: From<Tick>>` は汎用＝第3系統の additive 追加が最有力」とするが、実体は **`spawn_ticker<D>(config, kanade: Sender<KanadeMsg>, dispatcher: Sender<D>)`** で送出先が **kanade と dispatcher の 2 つに固定**（`ticker.rs:165-242`）。第3系統（seriko）を足すには `spawn_ticker` のシグネチャ・`TickerConfig`・`BoundarySchedule` 追加・呼び手（`runtime.rs:404`）の改修が要る。「既存2系統は不改変」ではあるが「純 additive（関数に触れない）」ではない。

**発見 B ── seriko は ghost の live-path では tick に到達していない。しかも別クレートに住む。**
ghost `runtime.rs` の boot 配線（`:381-408`）では ticker は kanade と dispatcher だけを叩く。**seriko は ghost では spawn されず、`areka` クレート（`emo2_boot`）で spawn され、dispatcher の broadcast sink 列（`options.sinks`）の1本として登録される**（`spine.rs:453`, `main.rs:352`）。つまり時間源（ghost）と seriko（areka）はクレートが分かれ、現状 tick は seriko inbox に一切届かない。第3系統を ghost 側 ticker に足す場合、seriko の inbox 送信端を areka から ghost boot へ逆流させて渡す配線が新たに要る。

**発見 C ── seriko は SERIKO アニメの pattern/interval 定義を保持していない（最大の未提供能力）。**
`spawn_seriko(resolver: SurfaceResolver, static_binds, bind_resolver, out)`（`actor.rs:163-182`）が受け取るのは **alias→surface id 表**（`SurfaceResolver`＝`BTreeMap<String, Vec<u32>>`）だけで、`interval,random,N` / pattern 群（wait/surface_id/x/y）の**タイムライン定義を一切持たない**。それらは emo-compose の `EmoWorld`（`SurfaceMaster.animations`）に住み、UI スレッド所有。ループを回すには「どの surface にどの interval アニメがあり、各 pattern の (surface_id, wait, x, y) が何か」を seriko 側で引ける**新しいアニメーション表（timeline source）を spawn_seriko の入力に追加**する必要がある。brief の「pattern タイムライン純関数」はロジックには言及するが、**定義データの供給経路**を明示していない。これは新規 crate 内モジュール＋新規 boot 時テーブル構築を要する。

**発見 D ── `DisplayCommand` は意図的に `#[non_exhaustive]` ではない。**
`output.rs:27` に「variant 追加時にコンパイラが下流 match の追随を強制する文化を維持する」と明記。pattern コマを合成入力へ載せる際、`Show` にフィールド追加 or 新 variant いずれでも **`map_display_command`（`adapter.rs:34-66`）の網羅 match が壊れる＝追随改修が必須**（意図された設計・悪いことではないが作業点）。一方 `PresentCommand` は `#[non_exhaustive]`（`command.rs`）で拡張余地あり。

---

## 2. Requirement → Asset Map（要件別ギャップ）

タグ: **[Missing]**=新規実装 ／ **[Extend]**=既存拡張 ／ **[Constraint]**=既存制約 ／ **[Reuse]**=既存流用

| 要件 | 必要能力 | 既存資産 | ギャップ |
|---|---|---|---|
| R1 自律 tick 源 | cue 非従属クロック／単調時刻 tick／非表示でも停止しない／既存 tick 不改変 | `TickerConfig.clock` 注入・`BoundarySchedule`・`Tick{now}` | **[Extend/Missing]** 供給先に seriko を追加（発見A/B）。非表示非従属は UI vsync 駆動を選ばなければ自然充足 |
| R2 `random,N` 毎秒抽選 | 表示中×非再生中で毎秒1/N・先頭コマから再生・再生中は非再抽選・アニメ独立 | なし | **[Missing]** 抽選ロジック＋乱数注入シーム（先例なし）。表示中判定は `ScopeState::Shown` 流用 **[Reuse]** |
| R3 `bind+random,N` ゲート抽選 | bindgroup OFF は判定不発／ON かつ非再生で1/N／read-only 参照／fixture 既定 OFF | `current_binds`/`BindSet::contains` | **[Reuse]** ゲート読取。**[Missing]** 抽選本体は R2 と共通 |
| R4 pattern タイムライン進行 | wait[ms] 累積・現在コマ1枚・`-1` 停止/リセット・末尾残留・1ms 単位 | `Pattern{surface_id:i64, wait:u32, x, y}` model | **[Missing]** 進行評価器（純関数）。定義データの**供給経路が未整備**（発見C） |
| R5 PatternState 拡張 | 合成入力に PatternState 第一級追加・cache キー拡張・animation ID 整列へ合流・空なら従来一致 | `compose_into` 署名・`ComposeKey`・plan.rs 整列規則・予約記述 | **[Extend]** 署名＋キー＋公開型（本 spec が正本）。整列規則自体は不変 **[Reuse]** |
| R6 冪等発行 | 変化 tick のみ単一発行・不変 tick は再発行なし・既存冪等継承 | `emit_display`・`ApplyOutcome`/`BindApplyOutcome` の Changed/Unchanged | **[Reuse/Extend]** PatternState 版の Changed ガードを追加 |
| R7 決定論と検証可能性 | 時刻・乱数を注入シーム経由・sleep 無し tick 駆動・本番は実源接続・失敗はログ | clock 注入先例・`MockSurfaceOutput`・`Close→join` 同期 | **[Missing]** 乱数注入シーム（`Fn` クロージャ先例のみ・rng 先例なし）。時刻は先例流用 **[Reuse]** |
| R8 スコープ規律 | random/bind+random の2つのみ駆動・他語彙は完全形保持・口パク/`\i[N]`/動的 bind/talk 除外 | parser `Interval` は 3 種のみ（`#[non_exhaustive]`）| **[Constraint]** parser は他語彙を未モデル化＝「定義に現れない」。seriko 側の interval 分岐を完全形（非駆動を明示）で持つ設計判断が要る |
| R9 実機2系統サインオフ | 実 emo2/実 DPI で kero・sakura まばたき・人間目視・デファクト2点を確定挙動化 | 実機サインオフ流儀（`AREKA_APP_SMOKE_EXIT_MS`＋ログ grep）| **[Reuse]** 手順。**[Constraint]** sakura は `\![bind,まばたき,通常,1]` 貫通で ON を作る前提（mayuna 成果物） |

---

## 3. Implementation Approach Options

分割対象を「①時間源結線」「②タイムライン純関数＋抽選」「③合成入力拡張」「④アニメ定義供給」の 4 論点で捉えると、各論点に独立の選択肢がある。

### 論点① 時間源の結線（発見A/B に対応）

- **Opt-1A（別ticker・areka 側）**: seriko 専用 ticker インスタンスを `areka` の `emo2_boot`（`spawn_seriko` の隣）で起動し、seriko inbox 送信端へ直接刻む。
  - ✅ ghost `spawn_ticker` に触れない（既存2系統完全不改変）・seriko と同一クレートで配線が閉じる・専用周期を自由に選べる（論点②の細粒度と整合）
  - ❌ ticker インスタンスが増える（スレッド1本増）・グリッド整列を共有したい場合は別途配慮
- **Opt-1B（ghost ticker 第3系統）**: `spawn_ticker` を kanade/dispatcher/seriko の3系統へ拡張。
  - ✅ tick 供給を1アクターへ集約・絶対グリッド共有が自然
  - ❌ seriko 送信端を areka→ghost boot へ逆流させる配線が新設（発見B）・`spawn_ticker`/`TickerConfig`/`runtime.rs` 改修（発見A）・クレート責務が滲む
- **Opt-1C（dispatcher broadcast 相乗り）**: dispatcher が既に受ける tick を seriko へ中継。
  - ❌ dispatcher は cue 台本の時間系（talk）で、pattern ループは talk 非従属（brief「別の時間系」）＝意味論が濁る。非推奨

> **供給元の対抗軸**: UI フレーム（vsync）駆動は R1.3「非表示でも停止しない」に反する（vsync は非表示で止まりうる）＋ worker 境界への逆流ゆえ、いずれの案でも tick は**専用アクター（ticker 系）由来が本線**。

### 論点② タイムライン純関数＋毎秒抽選（R2/R3/R4/R7 の核）

- **Opt-2A（純関数＋薄い状態・推奨方向）**: `(注入乱数・bind ゲート・非再生中のみ)→毎秒抽選 → (wait[ms]累積・現在コマ1枚・-1停止/末尾残留)→コマ進行 → PatternState` を GPU 非依存の純関数群に切り出し、`areka-seriko` 内の新モジュール（例 `loop.rs`/`timeline.rs`）へ。`BoundarySchedule`（`ticker.rs`）の「絶対グリッド・1回発火・catch-up」設計を**毎秒（1000ms）抽選境界**の写像として流用。
  - ✅ [[deterministic-test-coverage-mandate]] を全網羅で満たせる・注入 tick/乱数のみで sleep 不要
  - ❌ 「毎秒抽選」と「サブ秒コマ進行（wait 最小 22ms）」の**二層時間**を1つの tick 系で扱う設計が要る（下記 設計判断5）
- **Opt-2B（アニメごと状態機械をアクター内に直書き）**: 抽選・進行を `handle_message` の Tick 分岐へ手続き的に実装。
  - ❌ 決定論テストが実アクター経由になりがちで檻が太る・純関数分離の利点を捨てる。非推奨

### 論点③ 合成入力の PatternState 拡張（R5・DisplayCommand は非 non_exhaustive）

- **Opt-3A（`Show` にフィールド追加）**: `DisplayCommand::Show{scope, surface_id, binds, pattern_state}` ＋ `compose_into(.., active_binds, pattern_state)` ＋ `ComposeKey{surface_id, binds, pattern_state}`。
  - ✅ 「Show=1面の完全な合成入力」という現行意味論と素直に一致・`map_display_command` の1 arm 改修で済む
  - ❌ pattern 更新のたびに Show を再発行（冪等ガードで PatternState 差分時のみ・R6 で吸収）
- **Opt-3B（新 variant `ShowFrame`）**: transient コマ専用の別 variant を追加。
  - ✅ 静的 Show と動的コマの発行意図を分離
  - ❌ `#[non_exhaustive]` でない全 match（adapter/present）へ新 arm 追加＝波及大・cache/compose 側で結局同じキーへ合流させる必要があり利得薄。基本 Opt-3A を軸に

> `PresentCommand::ShowSurface` は `#[non_exhaustive]` ゆえフィールド追加は下流に優しい（`adapter.rs` の写像1点で pattern を載せ替える）。

### 論点④ アニメ定義の供給（発見C・R4 の前提）

- **Opt-4A（boot 時に seriko 専用テーブル構築）**: parser `Shell`（`SurfaceMaster.animations`）から `interval,random`/`bind+random` のアニメ＋pattern 群を抜いた**不変テーブル**を boot で構築し、`SurfaceResolver` と同様に `spawn_seriko` へ値渡し。
  - ✅ seriko が UI スレッド World に触れず自己完結・決定論テストで表を注入可能・[[areka-parser-transcribes-tree-downstream]] と整合（転記済みモデルから引く）
  - ❌ 定義の二重保持（emo-compose と seriko）だが read-only ＆ boot 一度きりゆえ許容
- **Opt-4B（emo-compose EmoWorld を参照）**: 実行時に World を引く。
  - ❌ World は UI スレッド所有・seriko は別スレッド＝クロススレッド共有 or ハンドオフが要り重い。非推奨

**推奨の組合せ（叩き台）**: Opt-1A ＋ Opt-2A ＋ Opt-3A ＋ Opt-4A。理由: 既存2系統 ticker を触らず、seriko クレート内で時間源・定義供給・純関数を閉じ、合成入力拡張は現行意味論に素直。ただし論点①は Opt-1B との比較で開発者判断を要する（下記 設計判断1）。

---

## 4. Effort & Risk

| 論点 | Effort | Risk | 根拠 |
|---|---|---|---|
| ① 時間源結線 | S (Opt-1A) / M (Opt-1B) | Low / Medium | 1A は既存 ticker 不改変で薄い。1B はクロスクレート逆流配線でリスク上昇 |
| ② タイムライン純関数＋抽選 | M | Medium | 二層時間（毎秒抽選×サブ秒コマ進行）と乱数注入シーム新設が新規。ロジック自体は純関数で網羅可 |
| ③ 合成入力拡張 | M | Medium | 公開型（PatternState）の正本確定＋compose/present/adapter の署名/キー/match 追随。回帰檻あり（cache.rs に前例テスト） |
| ④ アニメ定義供給 | S–M | Low | 転記済み parser モデルからの表構築。boot 一度きり |
| **全体** | **L（1–2週間）** | **Medium** | 4 論点＋実機2系統サインオフ。個々は既存パターン流用だが結線面が広い |

---

## 5. Research Needed（design で詰める項目）

- **R-1**: 「毎秒 1/N 抽選」の**抽選境界の正典**。tick 周期がサブ秒（例50ms）でも抽選は 1000ms 境界ごとに1回か。ukadoc の「毎秒」を絶対グリッド（`BoundarySchedule` 1000ms）へ写像する妥当性を design で明文化（実機齟齬時は SSP 実観察で裏取り・brief 方針）。
- **R-2**: 乱数注入シームの形。`Box<dyn Fn() -> u32>`（clock 注入と同型）か、`Fn(bound) -> u32` か、seed 付き決定論 PRNG か。本番 entropy 源も併記（`rand` は dola が既に依存・`tech.md`）。
- **R-3**: PatternState の公開データ形（本 spec が正本）。`HashMap<animation_id, 現在コマ{surface_id:i64, x, y, method}>` 程度か、より薄い表現か。空集合＝従来一致（R5.4）を byte 等価で担保する形。
- **R-4**: transient コマの `method`（overlay 等）を M-boot で解釈するか（emo2 は全て overlay）。R8 の未使用 method を完全形保持しつつ非駆動にする範囲。
- **R-5**: pattern ループの対象スコープ（シェル面のみか・バルーン面は対象外か）。現状 fixture はシェルのみ。

---

## 6. 設計判断アイテム（要件ディスカッションへ供給・番号付き）

1. **時間源の結線方式**: Opt-1A（areka 側 seriko 専用 ticker・既存2系統不改変）か Opt-1B（ghost `spawn_ticker` 第3系統・送信端を areka→ghost 逆流）か。クレート責務境界（時間源=ghost / seriko=areka）とスレッド本数のトレードオフ。
2. **tick 周期の粒度**: wait 最小 22ms に対し dispatcher 既定 50ms は粗い。seriko 専用周期（例 16–33ms）を置くか、「次コマ期限まで sleep せず注入 tick で刻む／期限計算式で吸収」か。決定論テストは注入 tick のみで完結が絶対条件。
3. **`SerikoMsg::Tick { now: MonotonicMs }` の additive 追加と From 実装**: tick キャリア型（`Tick` 直か seriko 固有か）と `impl From<Tick> for SerikoMsg`（選んだ ticker 経路に依存）。
4. **アニメ定義の供給経路（発見C）**: seriko へ interval/pattern タイムライン表を渡す新入力を `spawn_seriko` へ追加する形（Opt-4A のテーブル型・boot 構築点）。**これは brief に明示されていない必須の追加入力**。
5. **二層時間の分離設計**: 「毎秒抽選（1000ms 境界・アニメ独立・非再生中のみ）」と「再生中のコマ進行（wait[ms] 累積・現在コマ1枚）」を、1 つの tick 系でどう合成するか。`BoundarySchedule` 流用の可否と、per-animation 再生状態の持ち方。
6. **合成入力拡張の形（R5・論点③）**: `DisplayCommand::Show` へ `pattern_state` フィールド追加（Opt-3A）か新 variant（Opt-3B）か。`DisplayCommand` が非 `#[non_exhaustive]` ゆえ `map_display_command` の追随が確定発生する点の合意。
7. **PatternState の公開型正本**: emo-compose `compose_into` 署名・emo-present `ComposeKey`・`PresentCommand::ShowSurface` へどう載せるか（R-3）。容量1メモ化の思想不変（cache.rs 既述）。
8. **乱数注入シームの契約（R7）**: 形・本番 entropy 接続・テスト固定列（R-2）。clock 注入クロージャと同型で新設する方針の確認。
9. **デファクト推定2点の確定挙動化（R9.4）**: 「-1 無し末尾到達＝最終コマ残留でアニメ終了状態」「再生中は再抽選対象外（restart しない）」を檻の期待値として実装し、実機齟齬時のみ SSP 実観察で裏取り。要件で既に確定挙動として要求済み＝design で純関数の期待値へ焼き込む。
10. **スコープ規律の実装形（R8）**: parser `Interval` は 3 種のみ（`#[non_exhaustive]`）。seriko 側 interval 分岐を「random/bind+random のみ駆動・他は完全形で非駆動明示」に保つ表現（match の網羅と将来 additive シーム）。

11. **描画メソッド（method）の転記拡張と駆動範囲（要件ディスカッション #2 で scope 拡大確定・2026-07-23）**: 既存 `Pattern` モデルは `{index, surface_id, wait, x, y}` で **method 欄が無く**、正典（ukadoc `animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X,Y`＝method 先頭位置・旧形式は第3位置）を不完全転記していた＝転記層の欠落。本 spec は scope を上流 pattern モデルへ拡大し **method を忠実に転記**（描画メソッド語彙は overlay/base/move/scaling/start/stop/alternativestart/alternativestop/parallelstart/parallelstop/insert/auto 等。一部は surface/wait/XY を無視する独自意味論を持つ＝単なる合成フラグではなく制御フロー含む）。**設計で詰める点**: (a) parser `Pattern` への method フィールド追加の型（完全語彙 enum か文字列忠実転記か・新旧位置差の吸収）＝転記層の忠実性を保つ形／(b) PatternState の method 搬送形（R-3 と統合）／(c) 合成側の method ディスパッチ——emo2 は overlay のみ実 fixture を持つゆえ overlay を駆動しテスト、他メソッド（制御系 start/stop/parallel/alternative・幾何 move/scaling・着せ替え insert）は完全形保持のまま非駆動（R8.4）＝どのメソッドに実 semantics を与え、どれを型シームに留めるかの線引き。**注意**: `-1`/`-2` 時 method 無視（R4.3）は method 欄存在が前提。

---

## 7. Next Steps

- 本ギャップ分析を要件ディスカッション（`kiro-requirements-discussion`・チャット窓）で§6 の設計判断を詰める。
- その後 `kiro-design areka-P0-seriko-loop` で設計生成へ。design では特に §5 の R-1/R-2/R-3 と §6-1/6-4/6-5 を最優先で確定すること。
- brief の file:line 参照は mayuna-compose マージ後の現行座標へ読み替え済み（本書§1.1 が現行座標の正）。

---

# Design Phase 追記（2026-07-23・kiro-spec-design）

> 上掲 §1–§7 はギャップ分析（要件フェーズ）の記録。本追記は design 生成時の discovery（light・Extension）
> と synthesis の結論を記録する。design.md が正本、本書は根拠と代替案の台帳。

## Summary

- **Feature**: `areka-P0-seriko-loop`
- **Discovery Scope**: Extension（integration-focused light discovery・main context 実査。新規外部依存なしゆえ Web 調査は不要）
- **Key Findings**:
  - `areka-emo-compose` に描画メソッドの**完全語彙 registry が既存**（`method.rs`: `ComposeMethod`/`BlendKind`/`from_name`・overlay 同義 add/bind 写像・未知は `Unknown` 吸収・`is_implemented()` は Overlay のみ true）。§6-11 の「完全形の型値」は新造不要＝**adopt**。
  - parser `decode_animations` は現状 **`overlay` 以外の pattern 行を丸ごと落とす**（`decode.rs:335` の `== Some("overlay")` フィルタ）＝転記の穴の物理位置を特定。method 欄追加と同時にこのフィルタを撤去し全メソッドを転記する。
  - `EmoWorld` は `surface_ids()`/`surface(id)`/`SurfaceMaster.animations`（parser `Animation` 素通し保持）を**公開済み**で、`normalized.rs` に「seriko が再利用する」と明記あり＝アニメ定義表は **EmoWorld からのスナップショット構築**が最短（append 展開・fold 意味論を再実装しない）。
  - `spawn_ticker` は kanade/dispatcher 2 系統固定だが、`BoundarySchedule`（絶対グリッド・catch-up 1 回）は純粋層で再利用可能＝**同クレート内 additive な第 3 の汎用単発スポーナー**（クロージャ配送）で既存 2 系統を不改変に保てる。
  - `From<Tick>` 境界の第 3 系統化は **orphan rule で不成立**（`Tick` は ghost、`SerikoMsg` は seriko、impl を書けるのは両クレートのみ→不要な依存が生じる）。クロージャ配送 `Box<dyn FnMut(Tick)+Send>` なら型結合ゼロ。

## Research Log（design 実査）

### 時間源結線の実装形（§6-1/6-3 の確定）
- **Context**: Opt-1A（areka 側専用 ticker）vs Opt-1B（ghost ticker 第 3 系統）。
- **Sources**: `areka-ghost/src/ticker.rs`（`BoundarySchedule` pub(crate)・`spawn_ticker` 2 系統固定）、`runtime.rs:401-408`（TickerMode 分岐）、`areka/src/emo2_boot/mod.rs`／`spine.rs`（boot 配線・TickerMode::Disabled の決定論ハーネス）。
- **Findings**: seriko inbox 送信端（`SerikoSink`）は areka（emo2_boot）に住み ghost boot へは `CueSink` として渡るのみ。ghost に `Sender<SerikoMsg>` を型付きで渡すには逆流配線が要る（発見 B 再確認）。`From<Tick> for SerikoMsg` は orphan rule で areka には書けない。
- **Implications**: **ghost ticker.rs に additive な汎用単発レーン `spawn_loop_ticker(LoopTickerConfig, deliver: Box<dyn FnMut(Tick)+Send>)` を新設**（`BoundarySchedule` 再利用・既存 `spawn_ticker` 無改変）し、**areka が SerikoSink クローンを閉じ込めたクロージャで結線**する。時間源機構は ghost 帰属・結線は areka 帰属でクレート責務が保たれ、型結合（ghost⇄seriko）はゼロ。

### 描画メソッド転記の型（§6-11(a) の確定）
- **Context**: parser `Pattern` の method 欄を完全語彙 enum にするか文字列忠実転記にするか。
- **Sources**: `areka-parsers/src/shell/model.rs`（opaque NewType 規律・`ElementPath`/`AliasKey` 先例）、`decode.rs`（overlay フィルタ）、`areka-emo-compose/src/method.rs`（完全語彙 registry 既存）。
- **Findings**: parser は「忠実な転記層・意味解釈は下流」（steering）。emo-compose に完全語彙 enum（`#[non_exhaustive]`・同義写像・Unknown 吸収）が既に存在する。
- **Implications**: **parser は opaque NewType `DrawMethod(String)` で原文忠実転記**（新形式=第 1 位置。旧形式（第 3 位置）は正典位置として doc 転記し、旧形式キー行の字句解析は emo2 subset 外＝現行 lexer は `animationN.patternM` キーのみ）。**型値の完全形保持（8.4）は既存 `ComposeMethod` を adopt** し、seriko の表構築時に `ComposeMethod::from_name` で 1 回解決する。二重の語彙 enum を作らない。

### PatternState の住処と形（§6-7／R-3 の確定）
- **Context**: 公開型の正本をどのクレートに置くか・形。
- **Sources**: 依存方向（emo-compose ← seriko／emo-compose ← emo-present）、`cache.rs:43-52` 予約記述、`BindSet` の住処（emo-compose）。
- **Findings**: `BindSet` が emo-compose に住み seriko/present 双方が消費する先例。`ComposeKey` は `PartialEq+Eq` 要求。`ComposeMethod` は `Eq` 導出済み。
- **Implications**: **`PatternState` は emo-compose 新モジュール `pattern.rs` が正本**（`BTreeMap<u32 /*animation id*/, PatternFrame>`・正準順序で Eq 安定・`Default`=空）。`PatternFrame { surface_id: u32, method: ComposeMethod, x: i64, y: i64 }`。空 ⇒ 従来合成と byte 等価（5.4 の golden で檻）。センチネル（負値）は評価器が解決し PatternState には正の現在コマのみ載る。

### 二層時間と抽選境界（§6-2/6-5／R-1 の確定）
- **Context**: 「毎秒 1/N」と wait 最小 22ms のサブ秒コマ進行を 1 tick 系でどう合成するか。
- **Sources**: `ticker.rs` BoundarySchedule 意味論、brief の ukadoc 正典転記（毎秒・wait 定義）、emo2 実測 wait レンジ（0/22/40/80/150/160）。
- **Findings**: コマ進行を「経過時刻→現在コマ」の**関数**として評価すれば tick 周期に依らず正しく、粗い tick では中間コマが自然にスキップされる（現在コマ 1 枚意味論と整合・catch-up 安全）。抽選は 1000ms 絶対グリッド境界の跨ぎ検出で 1 回（catch-up 時も 1 回＝ghost ticker と同じ政策）。
- **Implications**: tick 供給は**専用レーン既定 16ms**（60Hz 近似・wait 最小 22ms を 1 tick 以内で拾う・非表示でも止まらない worker スレッド）。評価器は (a) 1000ms 境界の抽選層と (b) 経過時刻→現在コマの進行層を持つ純粋関数群で、注入 tick 列＋注入乱数列のみで全経路決定論。

### 乱数注入シーム（§6-8／R-2 の確定）
- **Context**: 形・本番 entropy・新規依存禁止。
- **Sources**: `TickerConfig.clock` 注入先例、tech.md（`rand` は dola 依存）、workspace 制約（新規 crates.io 依存なし）。
- **Findings**: 必要なのは「[0,k) の一様整数」1 種のみ。`std::hash::RandomState` は OS entropy 由来のシードを無依存で得られる。
- **Implications**: **`LoopRng = Box<dyn FnMut(u32) -> u32 + Send>`**（クロック注入と同型・戻りは [0,bound) 一様）。seriko が **`seeded_rng(seed: u64)`**（SplitMix64＋乗算シフト縮約・純粋・テスト可能）を提供し、**本番シードは areka 結線層が `RandomState` 経由で採取して `info!` でログ**（再現可能性の観測点）。テストは固定列クロージャを注入。

## Architecture Pattern Evaluation（design 確定分）

| Option | Description | Strengths | Risks / Limitations | 判定 |
|---|---|---|---|---|
| Opt-1A改（ghost に additive 汎用レーン＋areka 結線） | `spawn_loop_ticker`（クロージャ配送・BoundarySchedule 再利用）を ghost ticker.rs へ追加、areka が SerikoSink クローンで結線 | 既存 2 系統不改変・型結合ゼロ・グリッド整列/catch-up を再利用・時間源機構の帰属が ghost に残る | ticker スレッド +1 本 | **採用** |
| Opt-1B（spawn_ticker 第 3 系統） | 既存関数を 3 系統へ拡張 | tick 集約 | `From<Tick>` orphan rule／seriko 送信端の ghost への逆流配線／既存 2 系統改修 | 棄却 |
| Opt-2A（純関数＋薄い状態） | 抽選・進行を純関数群へ、アクターは配線のみ | 決定論全網羅・sleep 不要 | 二層時間の設計が要る（解決済み・上記） | **採用** |
| Opt-3A（`Show` フィールド追加） | `DisplayCommand::Show`/`ShowBalloon` に `pattern` を追加 | 「Show=1 面の完全な合成入力」意味論と一致・網羅 match が追随を強制 | 下流 match の連鎖改修（意図された設計） | **採用** |
| Opt-4A改（EmoWorld スナップショット表） | boot 時に `AnimationTable::from_world(&EmoWorld)` で不変表を構築し `spawn_seriko` へ値渡し | append/ターゲット展開・fold 意味論を再実装しない・表注入で決定論テスト可 | 定義の read-only 二重保持（boot 一度きり・許容） | **採用**（Opt-4A の供給元を parser `Shell` 直から fold 済み EmoWorld へ精緻化） |

## Design Decisions（design.md へ反映済みの確定）

### Decision D-1: 時間源＝ghost 汎用単発レーン＋areka 結線（§6-1/6-3）
- **Selected**: `areka-ghost/ticker.rs` に `LoopTickerConfig{interval(既定16ms), clock(既定 GetTickCount64)}`＋`spawn_loop_ticker(config, deliver: Box<dyn FnMut(Tick)+Send>)` を additive 追加。areka `wire_emo2_boot` が `SerikoSink::send_tick(now_ms)` を叩くクロージャで結線。`SerikoMsg::Tick{now_ms: u64}` を additive 追加（素の u64＝seriko に新規依存なし）。
- **Rationale/Trade-offs**: 既存 2 系統完全不改変（1.4）・orphan rule 回避・スレッド 1 本増は許容。停止は `TickerMsg::Close`＋join（main の shutdown 順序: loop ticker close → ghost shutdown → seriko join）。
- **Follow-up**: 停止後 tick の `send_tick` 失敗は debug!（shutdown 期待事象・PresentBridge 先例）。

### Decision D-2: 二層時間の純関数化（§6-5・R-1）
- **Selected**: 抽選層＝1000ms 絶対グリッド境界の跨ぎ検出（跨ぎ数によらず判定 1 回）。進行層＝再生開始時刻からの経過 → 累積 wait デッドライン列の「現在コマ」関数（`Pending/Active(i)/Stopped/FinishedResidual(i)`）。tick 周期非依存・catch-up 安全。
- **Trade-offs**: 粗い tick では中間コマがスキップされ得る（現在コマ意味論として正・16ms 既定で実用上不発生）。

### Decision D-3: デファクト 2 点の期待値焼き込み（§6-9・9.4）
- **Selected**: (a) `-1` 無し末尾到達＝`FinishedResidual(last)`（最終コマを PatternState に残したまま非再生化＝再抽選対象へ復帰）／(b) 再生中（`Pending/Active`）は抽選対象外。純関数の檻の期待値として明文化。実機齟齬時のみ SSP 実観察で裏取り。

### Decision D-4: PatternState 正本と合流規則（§6-6/6-7）
- **Selected**: emo-compose `pattern.rs` が正本。`compose_into(.., active_binds, pattern)` 拡張。`flatten_surface` の層(ii)で「有効 bind pattern0 の集合 ∪ PatternState のコマ集合」を **同一の animation ID 整列**（animation-sort 2 段規則・不変）へ合流し、**同 ID はコマが pattern0 寄与を置換**（各コマは直前をリセットしてベースへ・4.2）。合流は top-level surface のみ（コマは表示中 surface のアニメに属する）。`ComposeKey{surface_id, binds, pattern}` 拡張・容量 1 メモ化不変。
- **Trade-offs**: `DisplayCommand`（非 non_exhaustive）の網羅 match 追随が compose/present/adapter/frame に連鎖（意図された強制追随・発見 D）。

### Decision D-5: method の駆動線引き（§6-11(c)・8.4）
- **Selected**: parser は `DrawMethod(String)` 忠実転記（全 pattern 行を転記・overlay フィルタ撤去）。seriko 表構築時に `ComposeMethod::from_name` で 1 回解決し `PatternFrame.method` に完全形で搬送。合成は `Overlay` のみ駆動（blit）・非 Overlay は warn!＋当該コマ不描画（完全形保持のまま非駆動）。plan の bind pattern0 blit にも同じ method ゲートを追加（従来は decoder フィルタが overlay を保証していた前提の是正）。`-1` は method/x/y 無視で停止（4.3）・`-1` 以外の負値（`-2` 等）は warn!＋自アニメ停止扱い（他アニメ停止は駆動しない＝8.2）。
- **Rationale**: emo2 golden は byte 不変（全 overlay）。語彙は registry（ComposeMethod）一本で二重定義なし。

### Decision D-6: 状態の住処と冪等（§6-6・R6）
- **Selected**: `ScopeStates` に per-(scope, 面種スロット) の PatternState を同居（`dynamic_binds` 鏡映）。`commit_pattern` は commit_bind と同型（冪等ガード→書込→Shown なら Show/ShowBalloon 再発行）。surface 切替（apply）は当該スロットの PatternState を空へ戻し LoopRuntime の再生状態もリセット。発行は emit_display 単一点を継承。
- **面種非仕切り（要件裁定 (a)）**: 評価器・表・PatternState・commit は面種非依存。シェル map／バルーン map の両表示エントリを同一経路で評価し（表はシェル世界とバルーン世界の 2 つ＝**ID 名前空間の別**であって能力の仕切りではない）、`ShowBalloon` にも `pattern` を搬送する。emo2 でバルーン表が空なのはデータ事実。

### Decision D-7: 抽選順序の決定論（7.2）
- **Selected**: 1 境界での抽選は「スロット（scope 昇順→シェル→バルーン）→ animation id 昇順」の固定順で乱数を消費。注入乱数列テストの期待値が一意に定まる。

### Decision D-8: bind ゲートは抽選時のみ（3.1 文言どおり）
- **Selected**: `bind+random` のゲートは抽選判定時に `current_binds(scope).contains(anim.id)` を read-only 参照（面種によらず scope の bind 集合を一様参照）。再生中の bind 変化は再生を中断しない（要件が抽選判定のみを規定・最小決定論）。実機齟齬が観測されたら SSP 実観察で裏取り（9.4 と同じ流儀）。

### Decision D-9: interval 未認識語彙の忠実転記（design 討議 #1・2026-07-23 開発者裁定）
- **Selected**: `Interval` に `Other(Box<str>)` variant を追加（`#[non_exhaustive]` 内・下流非破壊）。`decode_animations` の未認識 interval キーワードの **fallback-Bind を撤去**し原文を `Other` へ忠実転記。表構築は `Other` を非採録・debug! に元語彙を明示（診断可能）。emo-compose の bind 分類は `Bind` のみ＝`Other` は静的経路にも乗らない（保持されるが駆動されない）。
- **Rationale**: method 裁定（要件討議 #2）と同型——転記層は落とさない・黙らない。R8.2「完全形保持」が字義どおり成立し、[areka-log-first-no-silent-failure]／[areka-parser-transcribes-tree-downstream] と整合。emo2 は interval 3 種のみ使用ゆえ観測不変（golden 影響ゼロ）。
- **Rejected**: (A) decode に warn! 1 行のみ（fallback-Bind 温存＝誤分類自体が残る）／(C) 追跡 spec 送りの明記のみ（転記の穴を知りながら残す）。

## Risks & Mitigations（design 追加分）

- `DisplayCommand`/`PresentCommand` 拡張の match 連鎖漏れ — 非 non_exhaustive のコンパイル強制＋spine e2e golden で検出。
- parser overlay フィルタ撤去による下流挙動変化（非 overlay pattern がモデルへ流入） — plan/表構築の method ゲート＋emo2 golden byte 不変檻で回帰を封じる。
- 16ms 常時 tick のアイドルコスト — 変化なし tick は評価のみで無発行（6.2）・スレッド 1 本＋軽量メッセージで許容。実測で問題なら周期は config 1 点で調整可。
- 乱数の自前 PRNG（SplitMix64）の品質 — まばたき用途（1/4 抽選）に十分。シードをログし再現可能性を確保。
- 実機で残留/非再抽選のデファクトが SSP と齟齬 — 9.4 の手順（SSP 実観察→期待値更新）で吸収。

## References

- ukadoc 正典転記は brief.md「ukadoc 正典確定（2026-07-23 調査済み）」を正とする（design での再調査不要と明記済み）。
- 実装先例: `areka-ghost/src/ticker.rs`（BoundarySchedule）・`areka-seriko/src/state.rs`（commit_bind 鏡映元）・`areka-emo-compose/src/method.rs`（完全語彙 registry）・`areka-emo-present/src/cache.rs`（キー拡張予約）。
