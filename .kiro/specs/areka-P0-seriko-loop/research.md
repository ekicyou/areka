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

---

## 7. Next Steps

- 本ギャップ分析を要件ディスカッション（`kiro-requirements-discussion`・チャット窓）で§6 の設計判断を詰める。
- その後 `kiro-design areka-P0-seriko-loop` で設計生成へ。design では特に §5 の R-1/R-2/R-3 と §6-1/6-4/6-5 を最優先で確定すること。
- brief の file:line 参照は mayuna-compose マージ後の現行座標へ読み替え済み（本書§1.1 が現行座標の正）。
