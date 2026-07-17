# Brief: areka-P0-seriko-loop

> **種別**: 本坑（main）増分。⑤ seriko 帰属（M-life 構成要素＝まばたき等の自律アニメーション）。roadmap 増分「⑤ seriko: `seriko-loop`（SERIKO ループ＝blink random/bind+random）」の brief 化（2026-07-16 実査注記を正式化）。
> **調査日**: 2026-07-16（再入精査⑧・2026-07-16 再入精査⑦の実査を継承）。
> **⛔ 時限ゲート（フェーズ別・2026-07-16 精密化）→ ✅ 解除（2026-07-17・cue-playback 完了＝追記㉗）**: ~~`areka-P0-cue-playback-duration`（実装中）の完了が tasks 生成・実装フェーズの前提~~ **→充足済み**＝seriko 受信面の `dola::CueSink` 化は main 着地済み（`actor.rs:122-126` `impl dola::cue::CueSink for SerikoSink`・受信面は `SerikoMsg = Cue | Close` のまま時間源なし＝Tick 注入口の増設要は不変・2026-07-17 実測）。全フェーズ着手可（**Fable 早期投入の1本目・推奨即時着手**——pattern 純関数・PatternState 公開形・乱数注入・合成入力拡張は cue モデル**非依存**）。着手時は settled コードを直接参照する。**完了ゲート（2026-07-17 合流裁定＝推奨から格上げ・roadmap 追記㉘）**: 本 spec の **/kiro-complete は `mayuna-compose` 完了が前提**——実機サインオフ必達条件「まばたき2系統」のうち sakura 側（bind+random）は bindgroup1400-1402 が既定 OFF（fixture に `default,1` 無し）で、ON にする唯一の源は `\![bind]` 貫通＝mayuna の成果物のため。決定論檻・kero 側（bind 非依存 `interval,random`）の実装/観測は mayuna 未完でも先行可。

## Problem

emo2 は起動しても**まばたきしない**。SERIKO の時限アニメ（interval,random 系）を駆動するランタイムが不在:

- **seriko に時間源が無い**（確認済み 2026-07-16）: 受信面は `SerikoMsg = Cue(TalkCue) | Close` のみ（`actor.rs:49-54`）＝**cue 到着駆動オンリー**。`handle_message` は `cue.at` をログ以外に使わず、クロック・スケジューラ・`Instant` は皆無。
- **pattern 状態が合成入力に無い**: emo-present のキャッシュキー（`cache.rs:46-47`・module doc `:8-10`）に「将来 seriko がアニメ pattern 状態を合成入力へ加える際は本キーへ追加する」の**予約記述が実在**。emo-compose 側は `plan.rs:11-14` が **pattern0 固定の合成規則**（pattern 進行の通貨なし＝本 spec の拡張対象）・予約シームは `world.rs:154-159`（将来の seriko system 統合用の脱出口）——いずれにせよ pattern を進めても表示に反映する通貨が無い。
- emo2 実例は**2系統**（fixture 実測 2026-07-16）: **kero＝`interval,random,4`**（`surfaces.txt:429-431` 系・`surface.append10,2100…` の blink・pattern に `overlay,-1`＝層クリア終端）／**sakura＝`interval,bind+random,4`**（`surfaces.txt:73,79,84`＝animation1400-1402・**まばたきカテゴリ bindgroup1400-1402〔shell descript.txt:50-52＝通常/半目/ジトー〕の bind が ON のときだけ** random 発火——bind ゲートの有無が2系統の差。**⚠️ 1400-1402 に `default,1` が無い＝既定 OFF**＝static_binds のままでは sakura は一切まばたきしない。なお scope doc §2 の「**目**カテゴリ bind ON かつ random」表記は fixture 実測と不一致——目カテゴリは別グループ bindgroup1300 系＝design で scope doc を訂正）。

## Current State（2026-07-16 実装偵察）

- **パーサは完了済み**: `interval` 3種（bind/random/bind+random）・animationN 集約・疎 pattern・負センチネル（`-1` 層クリア）は shell-parse ✅ が転記済み（emo-compose の正規化形が保持）。
- **発行点は再利用可能**: seriko の単一発行点 `emit_display`（`actor.rs:117-119`）は「将来の seriko-loop の再利用シーム」と文書化済み（`actor.rs:14-16,113-116`）。冪等ガード（状態不変時は再発行しない）の規律も確立済み。
- **bind 状態との合流**: `static_binds`（`state.rs:42-70`）は `mayuna-compose` が動的化予定——**bind+random の発火ゲート（まばたきカテゴリ bind ON か）は bind 状態を読む**＝mayuna の bind 置き場と本 spec の pattern 置き場は**別スロットだが同居**（state.rs 近接・契約先決要）。**fixture 既定 OFF ゆえ動的 bind 無しでは sakura まばたきが観測不能＝mayuna 先行推奨の実質根拠**。
- **Tick の供給元候補**: ghost-setup ✅ の ticker（絶対グリッド整列・kanade へ毎秒 Tick）——ただし blink の pattern wait は sub-second（fixture 実測値 0/40/80/150/160・**単位は 10ms/1ms のいずれか＝ukadoc 必読で確定**）＝**毎秒 tick では粗い**。フレーム駆動（emo2_boot `present_frame` 毎フレーム UI 駆動）or 専用時間源の選定が design の主題。

## Desired Outcome

emo2 の**まばたき2系統**（kero=random・sakura=bind+random）が実機で動き、pattern 進行は**注入時刻＋注入乱数で決定論的に檻へ入る**。pattern 状態が合成入力の第一級通貨（予約 TODO の実装）になる。

**✔ 観測（単一 pass/fail）**: 決定論（注入 tick・注入 seed・sleep 不使用）＝(a) random,4 の発火判定列（seed 固定）→pattern タイムライン（wait 累積・overlay 重ね・`-1` クリア終端）の期待 pattern 状態列 (b) bind+random は**まばたきカテゴリ bind** OFF で**不発火**・ON で発火（ゲート檻・fixture 既定は OFF） (c) pattern 変化時のみ合成入力再発行（冪等ガード継承） (d) 合成入力（surface_id＋BindSet＋pattern 状態）→emo-compose 合成が pattern overlay を重ねた golden 一致。＋実機＝実 emo2・実 DPI で**むらさき（まばたきカテゴリ bind ON 時＝既定 OFF ゆえ ON 状態を作る手順込み）とエモの両方がまばたき**する人間サインオフ。

## Approach

1. **Tick 注入口の増設**（settled 受信面へ）: cue-playback 完了後の seriko 受信面（`dola::CueSink` 実装＋actor inbox）へ **時刻注入の口**を additive 追加。供給元（フレーム駆動 vs ticker 細分化 vs 専用 thread）は design で確定——**決定論テストは注入 tick のみで完結**が絶対条件（実機だけが実時間源に接続）。
2. **pattern タイムライン純関数**: `interval 判定（random 抽選・bind ゲート）→ pattern 進行（wait 累積・N コマ・-1 終端）→ 現在の overlay 集合` を純関数化（GPU 不要で全網羅・[[test-only-decision-branches-not-proven-wiring]]）。乱数は**注入 seed**（`random,4`＝「毎秒 1/4」級の SSP de-facto 確率解釈を ukadoc で確定）。
3. **合成入力の拡張**（予約 TODO の実装）: `(surface_id, BindSet)` → `(surface_id, BindSet, PatternState)` 級へ。emo-compose 合成プラン（`plan.rs:11-14`）が pattern overlay を animation ID 昇順の重ねに合流・emo-present `ComposeCache` キー（`cache.rs:44-47`）も同拡張（容量1メモ化の設計思想は不変——pattern 変化＝キー変化＝再合成）。
4. **発行**: 既存 `emit_display` 単一発行点＋冪等ガードを継承（pattern 不変フレームは再発行しない＝毎フレーム再合成の禁止）。
5. **スコープ規律**: M-life の実需は**まばたきのみ**（interval 3種のうち bind は静的＝mayuna 領分・random/bind+random が本 spec）。sometimes/always/talk 等の他 interval・move/base 等の他 method は**未使用＝型シームのみ**（[[areka-two-animation-engines]] の②エンジン最小実装）。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **mayuna-compose と state.rs 近接（契約先決）**: mayuna＝bind 置き場の動的化／本 spec＝pattern 置き場の新設——**別スロット・別アーム**だが同 crate 同 file 近接。**bind 状態の読み口**（bind+random ゲートが参照）だけ契約先決（mayuna の動的 bind マップを read-only 参照する形）。**推奨順序: mayuna 先行 → 本 spec が動的 bind を読む**（逆順なら static_binds を暫定参照し mayuna が差し替え）。
- **cue-playback-duration が絶対上流**（時限ゲート・受信面）: Tick 口は settled `CueSink` 受信面へ足す。**cue タイムライン（talk）と pattern タイムライン（ループ）は別の時間系**——SERIKO ループは talk に従属しない自律駆動（dola 台本の外・[[areka-dola-absolute-time-sync-broadcast]] の「動的制御は dola 外側」に整合）＝dola cue モデルには**触らない**。
- **合成入力の拡張形＝本 spec が正本**: `PatternState` の公開形（emo-compose/emo-present が消費）は本 spec が確定。emo-compose の合成規則（animation ID 昇順）は不変（消費のみ）。
- **emo2-boot の毎フレーム駆動義務**: `present_frame` 毎フレーム UI 駆動（emo2-boot 申し送り）がフレーム tick の自然な供給点候補——採用時は emo2_boot 結線の小増分（frame.rs）を含む。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16）

- **必読**: `descript_shell_surfaces`（SERIKO 節）の **`interval,random,N`／`interval,bind+random,N` の正確な意味論**（N の単位＝発火確率 1/N 毎秒か・pattern wait の単位〔10ms/1ms〕・pattern 終端後の挙動・`overlay,-1` の層クリア規則）。`animation*.option,exclusive` 等の関連オプション（emo2 未使用なら型シーム）。
- **具体指示**: design 冒頭で「random 抽選の SSP de-facto（毎秒抽選・確率 1/N）」を ukadoc で確定し、注入 seed の檻の期待値に固定すること。fixture 実測（kero `animation0.interval,random,4`・pattern0-3 の wait/ID 列・sakura 1410-1413）との突合表を design.md に載せること。

## Scope

- **In**: Tick 注入口（settled 受信面へ additive）／pattern タイムライン純関数（random 抽選・bind ゲート・wait 累積・-1 終端）／注入 seed 檻／合成入力の PatternState 拡張（compose plan＋present cache の予約 TODO 実装）／冪等発行／実機まばたき2系統サインオフ。
- **Out**: 動的 bind 切替（**mayuna-compose**・read-only 参照のみ）／talk cue 再生（cue-playback）／sometimes/always/talk 等の他 interval・move 等の他 method（emo2 未使用＝型シーム）／口パク lipsync（emo2 は bind 表現＝未使用）／SERIKO の `\i[N]` 明示再生タグ（emo2 未使用）。

## Boundary Candidates

- pattern 純関数（全網羅）／時間源結線（薄い・design 主題）／合成入力拡張（公開形の正本）／実機サインオフ。

## Out of Boundary

- 合成そのもの（emo-compose）・表示（emo-present）——拡張キーの消費側は既存規則のまま。

## Upstream / Downstream

- **Upstream**: **`areka-P0-cue-playback-duration`（時限ゲート・settled 受信面）**／**`areka-P0-mayuna-compose`（bind 読み口・推奨先行）**／`completed/areka-P0-seriko-engine`（emit_display・冪等・ScopeStates）／`completed/areka-P0-emo-compose`＋`-present`（予約 TODO の宿主）／`completed/areka-P0-shell-parse`（interval/pattern 転記）。
- **Downstream**: M-life（統合点）／`areka-P0-emo2-conformance-e2e`（まばたき2系統を適合項目に）／将来の SERIKO 拡張 interval（M2・型シーム）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-seriko-engine`（受信面・状態・発行点への additive 増分）。
- **Adjacent**: `areka-P0-mayuna-compose`（state.rs 近接・bind 読み口の契約先決・推奨は逐次〔mayuna→本 spec〕）／`areka-P0-cue-playback-duration`（受信面の書き換え主＝ゲート）。
- **Consumes**: emo-present `cache.rs:46-47`（＋module doc `:8-10`）の予約記述・emo-compose `world.rs:154-159` の seriko 統合用脱出口（`plan.rs:11-14` は pattern0 固定規則＝**拡張対象**であって予約 TODO ではない——2026-07-16 検証で書き分け）。

## Constraints

- Rust 2024・新規 crates.io 依存なし・tokio 不使用。
- **決定論**: 注入 tick＋注入 seed で全経路網羅（[[deterministic-test-coverage-mandate]]）——実時間源への接続は実機のみ。乱数はテスト注入可能な形に（本番 seed の出所も design で明示）。
- 冪等発行（pattern 不変時は再発行しない）・ログ規律（[[areka-log-first-no-silent-failure]]）。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI でまばたき2系統の人間サインオフ（[[areka-placement-real-ghost-first]]）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。
