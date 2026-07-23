# Brief: areka-P0-seriko-loop

> **種別**: 本坑（main）増分。⑤ seriko 帰属（M-life 構成要素＝まばたき等の自律アニメーション）。roadmap 増分「⑤ seriko: `seriko-loop`（SERIKO ループ＝blink random/bind+random）」の brief 化。
> **調査日**: 2026-07-23（**全面リフレッシュ**＝mayuna-compose 実装完了後の実コード実査＋ukadoc 正典調査を反映。旧版 2026-07-16 の「要確定」事項を大幅解決）。
> **✅ ゲート全解除（2026-07-23 時点）**: ①cue-playback-duration ✅完了（seriko 受信面の `dola::CueSink` 化は settled・`actor.rs:123-127`）。②**mayuna-compose 完了・マージ済**（全タスク完・実機サインオフ済・2026-07-23 PR squash マージ＝`completed/areka-P0-mayuna-compose`）＝完了ゲート（追記㉘）の充足——動的 bind（`dynamic_binds`）・mustselect 排他・**emo-compose pattern0 厳格選択（実機第2欠陥の是正）**が着地済み。bind 読み口契約は**先決不要＝実在 API を消費するだけ**（下記）。`/kiro-start` で即着手可。

## Problem

emo2 は起動しても**まばたきしない**。SERIKO の時限アニメ（interval,random 系）を駆動するランタイムが不在:

- **seriko に時間源が無い**（2026-07-23 再実測）: 受信面は `SerikoMsg = Cue(TalkCue) | Close` のみ（`actor.rs:52-58`）＝cue 到着駆動オンリー。クロック・スケジューラは皆無。Tick variant の追加位置はここ。
- **pattern 状態が合成入力に無い**: 合成入力の署名は `Composer::compose_into(world, atlas, surface_id, active_binds)`（emo-compose `lib.rs:113-134`）＝pattern 進行の通貨なし。emo-present キャッシュキー `ComposeKey{surface_id, binds}`（`cache.rs:43-52`）に「pattern 状態を加える際は本キーへ追加する」予約記述が実在。
- **mayuna が静的側を正しく閉じた結果、動的側が本 spec に残った**: emo-compose `plan.rs:306-318` の pattern0 厳格選択（index==0 のみ・疎最小フォールバック禁止）は「pattern0 を持たない bind animation（まばたき等・pattern1 以降のみ）は静的土台を持たず、**それらのフレームは seriko-loop（M-life）が再生する**」とコード内に明記済み＝本 spec が引き取る宿題の物理正本。

## emo2 実例（fixture 正典・2026-07-23 実測）

**2系統**（`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/`）:

- **sakura＝`interval,bind+random,4`**（surfaces.txt:73-90・surface1000 ブロック）:
  - animation1400（まばたき:通常）: `pattern1,overlay,1412,0` → `pattern2,overlay,1411,150` → `pattern3,overlay,1410,22` — **pattern0 なし・`-1` 終端なし**（最終コマ 1410=開眼が残留＝意図された定義。デファクト §ukadoc 参照）
  - animation1401（半目）: `pattern1,overlay,1412,0` → `pattern2,overlay,1411,150` — 同型
  - animation1402（ジトー）: `pattern1,overlay,1412,0` → `pattern2,overlay,1413,150` — 同型
  - animation1403（----）: `interval,bind`（**random なし＝静的着せ替え**・pattern2 のみ・pattern0 なし→静的寄与ゼロで正）
  - **bindgroup1400-1403（descript.txt:50-53）に `default,1` は皆無＝既定 OFF**・**まばたきカテゴリは mustselect ではない**（mustselect は 腕/口/眉/目 の4つのみ descript.txt:75-78・複数同時 ON があり得る点に注意）。ON にする唯一の源は `\![bind,まばたき,…]` 貫通＝mayuna 成果物（実機サインオフで貫通確認済み）。
- **kero＝`interval,random,4`**（bind 非依存・surfaces.txt:427-454・`surface.append` 4本）:
  - append10,2100: `animation0.interval,random,4` / `pattern0,overlay,2106,0` → `pattern1,overlay,2110,40` → `pattern3,overlay,-1,80`（**`-1` 終端あり**）
  - append2200 / append2110 / append2210: 同型（wait 0/40/80/160）
- **wait 実測レンジ**: 0 / 22 / 40 / 80 / 150 / 160 — **単位は ms で確定**（下記 ukadoc）。

## ukadoc 正典確定（2026-07-23 調査済み・design での再調査不要）

- **`random,N` ＝「そのサーフェスである間、毎秒 1/N の確率で再生」**（正典明文）。sometimes=毎秒1/2・rarely=毎秒1/4 の一般形。**毎秒・アニメごと独立に再抽選**。
- **`bind+random,N` ＝「その着せ替えが ON の場合に、毎秒 1/N の確率で発生」**（SSP 拡張・正典明文）＝bindgroup ON がゲート・OFF 中は判定自体が走らない。
- **wait 単位＝SERIKO/2.0 は 1ms**（1.x は 10ms。`descript { version,1 }` が 2.0 宣言・emo2 は 2.0）。wait は「**そのコマに切り替わるまでの待ち時間**」（前コマからの遅延・コマ自身の表示時間ではない）。SSP 拡張で `最小-最大` のランダム範囲記法あり（emo2 未使用＝型シームのみ）。
- **pattern の surface `-1` ＝そのアニメーションの停止＋ベース表示へリセット**／`-2`＝実行中の他の全アニメ停止（いずれも method/x/y 無視）。
- **コマ合成規則＝「各コマはそれ以前のコマをリセットして新たにベースサーフェスへ合成」**（累積しない）＝**PatternState はアニメ ID ごと「現在コマ1枚」**（overlay 集合の累積ではない）。
- **`animation-sort` 既定 descend**・複数 interval アニメは既定で並行可（排他は `option,exclusive` のみ・bind interval には exclusive 未定義）。
- interval 全語彙（型シーム用）: sometimes/rarely/random,N/periodic,N/always/runonce/never/yen-e/talk,N（口パク）/bind/bind+always/bind+runonce/bind+random,N。
- **正典に明文なし（デファクト推定・design で檻の期待値として明記すること）**: (a) `-1` なしで末尾到達→**最終コマ残留のままアニメ終了状態**（emo2 の 1400 系は最終コマ=開眼ゆえ無害・むしろ意図）／(b) **再生中のアニメは interval 再抽選の対象外（restart しない）**が SSP 通説＝「非再生中のみ毎秒判定」を安全側実装とする。

## Current State（2026-07-23 実装偵察・mayuna 後）

- **受信面**: `SerikoMsg`（actor.rs:52-58）・`CueSink` impl（actor.rs:123-127）・inbox ハンドラ `handle_message`（actor.rs:192-435・FIFO 単一スレッド）。モジュール doc（actor.rs:14-15）が「時間駆動ループが同じ発行点を再利用できる」と明記済み。
- **単一発行点＋冪等**: `emit_display`（actor.rs:134-136）が唯一の `SurfaceOutput::send` 呼出。冪等は状態層戻り値（`ApplyOutcome::Changed`/`BindApplyOutcome::Changed` のみ発行）。
- **bind 読み口＝実在 API（契約先決は解消済み）**: `ScopeStates::current_binds(&scope) -> &BindSet`（state.rs:239-244）＋`BindSet::contains(id)`（emo-compose/src/bind.rs:30）。「scope0 で 1400 が ON か」が一行で読める。`ScopeStates` は `static_binds`（既定）＋`dynamic_binds: HashMap<ActorKey, BindSet>`（per-scope・state.rs:76-96）。
- **PatternState の自然な置き場**: `ScopeStates` に `dynamic_binds` と並ぶ per-scope フィールドとして同居（`commit_bind`（state.rs:320-340）の「冪等→書込→Shown なら再発行」パターンを鏡映）。
- **合成側の合流点**: `flatten_surface`（plan.rs:247-351）——有効 bind 収集（:275-280）→animation-sort 2 段規則（:285-298・Descend＝id 昇順描画＝画家のアルゴリズム）→pattern0 blit（:300-345）。PatternState の transient コマは**同じ animation ID キーで整列規則に自然合流**（1400 系は 13xx より大 id＝上層で正しい）。
- **表示経路**: seriko `DisplayCommand`（output.rs:28-42）→`PresentBridge`（areka/src/emo2_boot/adapter.rs:78-118・純変換 `map_display_command` :34-66）→`PresentCommand::ShowSurface{target,surface_id,binds,..}`（`#[non_exhaustive]`＝拡張余地あり・command.rs:39-67）→UI 側 `run_drain_phase`（frame.rs:500-512）→compose キャッシュ（presenter.rs:225-233）。
- **Tick 供給の既存資産**: areka-ghost ticker は**既に 2 系統**（dispatcher 向け 50ms＋kanade 向け 1000ms・ticker.rs:47-74）で `TickerConfig.clock: Box<dyn Fn()->MonotonicMs>` **注入可**・絶対グリッド整列＋catch-up 一度きり（`BoundarySchedule` ticker.rs:82-156）。`spawn_ticker<D: From<Tick>>`（ticker.rs:165-242）は汎用＝**seriko 向け第3系統の additive 追加が最有力**。本番結線は main.rs:168 付近。
- **決定論の先例**: ticker `shared_clock` 注入（ticker.rs:417-491）・kanade `KanadeMsg::Tick{now}` 直投函・`MockSurfaceOutput`＋`Close→join` 同期（sleep 不使用）。**乱数 seed 注入の先例のみ無し**＝クロック注入クロージャと同型の `Fn` 注入シームを新設（本番は entropy 源・テストは固定列）。

## Desired Outcome

emo2 の**まばたき2系統**（kero=random・sakura=bind+random）が実機で動き、pattern 進行は**注入時刻＋注入乱数で決定論的に檻へ入る**。pattern 状態が合成入力の第一級通貨（予約 TODO の実装）になる。

**✔ 観測（単一 pass/fail）**: 決定論（注入 tick・注入乱数・sleep 不使用）＝(a) 毎秒 1/4 抽選（注入列固定）→pattern タイムライン（wait[ms] 累積・コマ=現在1枚・`-1` 停止/末尾残留）の期待 PatternState 列 (b) bind+random は bind OFF で**判定自体不発**・ON で発火（ゲート檻・fixture 既定 OFF）・再生中は再抽選対象外 (c) PatternState 変化時のみ再発行（冪等ガード継承） (d) 合成入力（surface_id＋BindSet＋PatternState）→emo-compose 合成が transient コマを animation ID 整列で重ねた golden 一致（kero `-1` 到達でベース復帰）。＋実機＝実 emo2・実 DPI で**むらさき（`\![bind,まばたき,通常,1]` ON 状態を作る手順込み）とエモの両方がまばたき**する人間サインオフ。

## Approach

1. **Tick 注入口の増設**: `SerikoMsg::Tick { now: MonotonicMs }` 級を additive 追加（actor.rs:52-58）。**供給元の本命＝ghost ticker 第3系統**（`TickerConfig`/`spawn_ticker` の additive 拡張・注入クロック既設・グリッド整列済み）——UI フレーム駆動（vsync）は「非表示時に vsync が止まる＝まばたきも止まる」リスクと worker アクター境界への逆流があり次点。**tick 周期は design で確定**（wait 最小 22ms ゆえ 50ms 流用では粗い可能性——専用周期 or「次コマ期限」計算式で吸収。決定論テストは注入 tick のみで完結が絶対条件）。
2. **pattern タイムライン純関数**: `毎秒抽選（注入乱数・bind ゲート・非再生中のみ）→ コマ進行（wait[ms] 累積・現在コマ1枚・-1 停止/末尾残留終了）→ PatternState` を純関数化（GPU 不要で全網羅・[[test-only-decision-branches-not-proven-wiring]]）。乱数は注入 `Fn` シーム新設（先例なし・クロック注入と同型で）。
3. **合成入力の拡張**（予約 TODO の実装）: `compose_into(.., active_binds)` → `(.., active_binds, pattern_state)` 級へ。合流点は `flatten_surface` の animation ID 整列（既存規則不変・transient コマが同キーで合流）。emo-present `ComposeKey` も同拡張（容量1メモ化の思想は不変——pattern 変化＝キー変化＝再合成）。`PresentCommand` は `#[non_exhaustive]`＝ShowSurface フィールド拡張/新 variant の両選択肢あり。
4. **発行**: `emit_display` 単一発行点＋冪等ガード継承（PatternState 不変 tick は再発行しない＝毎フレーム再合成の禁止）。
5. **スコープ規律**: 実装は random／bind+random の2つのみ。他 interval（sometimes/rarely/periodic/always/runonce/never/yen-e/talk）・`-2`・wait 範囲記法・exclusive 等 option は**未使用＝型シーム/語彙のみ完全形で保持**（[[defer-canon-with-full-vocabulary-and-tracking-spec]]）。

## クロスユニット契約（2026-07-23 更新）

- **mayuna-compose**: ~~契約先決要~~ → **解消済み＝実在 API 消費**（`current_binds`/`BindSet::contains`）。PatternState は `dynamic_binds` と同居の別スロット・read-only 参照のみ（bind 書込側は不改変）。
- **合成入力の拡張形＝本 spec が正本**: `PatternState` の公開形（emo-compose/emo-present が消費）は本 spec が確定。emo-compose の整列規則（animation ID 昇順・pattern0 厳格選択）は不変（合流のみ）。
- **ticker 第3系統**: `TickerConfig`/`spawn_ticker<D: From<Tick>>` は汎用設計済み＝additive 追加で既存 2 系統不改変。結線は main.rs（ghost-setup 帰属の小増分）。
- **dola cue モデル不改変**: cue タイムライン（talk）と pattern タイムライン（ループ）は**別の時間系**——SERIKO ループは talk 非従属の自律駆動（[[areka-dola-absolute-time-sync-broadcast]] の「動的制御は dola 外側」に整合）。

## Scope

- **In**: Tick 注入口（SerikoMsg additive＋ticker 第3系統）／pattern タイムライン純関数（毎秒抽選・bind ゲート・wait[ms] 進行・-1 停止・末尾残留・再生中非再抽選）／注入乱数シーム新設／合成入力の PatternState 拡張（compose 署名＋present cache キー）／冪等発行／実機まばたき2系統サインオフ。
- **Out**: 動的 bind 切替（mayuna-compose ✅・read-only 参照のみ）／talk cue 再生（cue-playback ✅）／他 interval・`-2`・wait 範囲記法・exclusive（emo2 未使用＝型シーム）／口パク lipsync（interval,talk＝emo2 未使用）／`\i[N]` 明示再生タグ（emo2 未使用）。

## Boundary Candidates

- pattern 純関数（全網羅）／時間源結線（ticker 第3系統・薄い）／合成入力拡張（公開形の正本）／実機サインオフ。

## Out of Boundary

- 合成そのもの（emo-compose）・表示（emo-present）——拡張キーの消費側は既存規則のまま。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-mayuna-compose`（✅完了・マージ済・動的 bind＋pattern0 厳格選択）／`completed/areka-P0-cue-playback-duration`（settled 受信面）／`completed/areka-P0-seriko-engine`（emit_display・冪等・ScopeStates）／`completed/areka-P0-emo-compose`＋`-present`（予約 TODO の宿主）／`completed/areka-P0-ghost-setup`（ticker）／`completed/areka-P0-shell-parse`（interval/pattern 転記✅）。
- **Downstream**: M-life（統合点・idle-talk✅/collision-geometry✅/input-events と合流）／`areka-P0-emo2-conformance-e2e`（まばたき2系統を適合項目に）／将来の SERIKO 拡張 interval・口パク（M2・型シーム）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-seriko-engine`（受信面・状態・発行点への additive 増分）。
- **Adjacent**: `completed/areka-P0-mayuna-compose`（state.rs 同居・read-only 消費）／areka-ghost ticker（第3系統 additive）。
- **Consumes**: emo-compose `plan.rs:306-318`（pattern0 厳格選択＝本 spec への宿題明記コメント）・emo-present `cache.rs:43-52` の予約記述・`PresentCommand` `#[non_exhaustive]` 拡張余地。

## Constraints

- Rust 2024・新規 crates.io 依存なし・tokio 不使用。
- **決定論**: 注入 tick＋注入乱数で全経路網羅（[[deterministic-test-coverage-mandate]]）——実時間源への接続は実機のみ。乱数注入は `Fn` シーム新設（先例なし・本番 entropy 源も design で明示）。
- 冪等発行（PatternState 不変時は再発行しない）・ログ規律（[[areka-log-first-no-silent-failure]]）。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI でまばたき2系統の人間サインオフ（[[areka-placement-real-ghost-first]]）。
- 正典は ukadoc（本 brief に 2026-07-23 調査済み転記）・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。デファクト推定2点（末尾残留・再生中非再抽選）は design で檻の期待値として明文化し、実機で挙動齟齬が出たら SSP 実観察で裏取り（[[areka-emo2-mustselect-required-not-deferrable]] の教訓と同型）。
