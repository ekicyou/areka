# Brief: areka-P0-dpi-window-vanish

> **性格**: 実機不具合の**調査先行 spec**（診断 → 根本原因確定 → 理想形修正）。憶測修正は禁止＝原因未確定のまま実装フェーズへ進まない（開発者選択 2026-07-18「診断 repro を先に」）。
> **由来**: 2026-07-18 `areka-P0-idle-talk` Task 6 実機サインオフ直後の実機運転で開発者が報告。idle-talk 本体は「OnSecondChange 自発トーク確認＝完了」で**不変**（本事象は表示系＝idle-talk へ混ぜない・開発者裁定）。

> **📌 2026-07-31 追記(52)陳腐化補正（W4 完走・本ブロックが㊹㊵以下より優先）**:
> - **確定事実④（唯一確定の設計 gap）は W4 emo-dpi-scaling が解消済み**: WM_DPICHANGED は wintf window_proc/mod.rs:48 → window_pos.rs:274 配送 → **`run_dpi_phase`（frame.rs:865-873・doc :841-847）が `Changed<DPI>` 観測→`refresh_scale`→char 窓 `resize_window_to`（アンカー保存）／balloon 窓 `resize_window_keep_position` で同一フレーム内リサイズ**。＝旧診断ログの「窓 336x400 物理固定」は現行コードでは再現しないはず。**診断 Q1〜Q4 は新ビルド（k 追従込み実機）で再実施が必須**＝結果次第で本 spec は「再現せず・掃除のみ」へ**縮退し得る**（縮退時は GhostWindows despawn 掃除＋回帰檻のみで完了・W5 の1席は空く）。
> - 残件は現存: `GhostWindows` despawn 未掃除（現所在 **spawn.rs:115**・`ScopeWindows` :101）・「Anchored 未付与」warn は follow.rs:559 → **:792**・終了時 `Anchored` WARN 追跡（追記㊾申し送り）。follow.rs 檻 :24-26 → **:22-25**（文言現存）・resolver.rs DPI 不変テスト :303-309 以降現存（`DPIS=[96,120,144,192]` :304）。
> - 判定は絶対 px でなく**比**・`refresh_scale` は k 変化時でも 2 経路で None（追記㊾）＝診断時の観測点。

> **📌 2026-07-24 追記㊹陳腐化補正**: W5 は **4本同居**へ改訂（`kero-balloon` 追加・実測で互いに素・本 spec の placement follow/spawn＋wintf window_proc/drag 単独所有は不変）。アンカー1件付替え——確定事実6の「`GhostWindows` despawn 未掃除（placement/mod.rs:99）」は現在 **spawn.rs:109-130（`GhostWindows` Resource 定義）**が正・「Anchored 未付与」warn は follow.rs:559 に現存（事象は真・位置のみ失効）。follow.rs:24-26 檻・resolver.rs:303-318 DPI 不変テストは 2026-07-24 実測一致。W4 の開始は割込 `wintf-gpu-test-crash` 完了後＝本 spec の修正フェーズはその分後段（診断/文書フェーズの随時先行可は不変）。

> **📌 2026-07-23 追記㊵ウェーブ更新（本文の旧ウェーブ番号より優先）**: 攻め5ウェーブ再編により本 spec の実装は **W5**（`collision-dpi-hittest` ∥ `choice-select-events` と3本同居・ファイル集合は互いに素＝本 spec は placement follow/spawn＋wintf window_proc/drag を単独所有）。前提: W4 `position-persist`（follow.rs/spawn.rs 解放）＋W4 `emo-dpi-scaling`（**確定事実④「WM_DPICHANGED でも窓 336x400 物理固定」＝唯一確定の設計 gap を W4 が先に解消する**＝本 spec の診断 Q1-Q4 は dpi-scaling 着地後の実機状態で再評価してから修正フェーズへ）。診断/文書フェーズの随時先行可は不変。

## Problem

実機（マルチモニタ・混在 DPI 125%/200%・screen 座標 3200 超）で emo2 を運転中、**キャラ「えも」（kero 側・scope 1）の窓とバルーンが消失**し、「むらさき」（sakura 側・scope 0）だけが残った。開発者所感「知らないうちに消えてて再現性が微妙」＝消失の瞬間は未目撃・決定的な再現手順は未確立。

**キャラ帰属の正**（2026-07-18 開発者訂正）: emo2 の sakura 側（scope 0）＝**むらさき**（残った方）／kero 側（scope 1）＝**えも**（消えた方）。

## Current State — 診断済み証跡の詳細記録（2026-07-18・本 spec の出発点）

診断 repro を1回実施済み。実機 10 分運転（`RUST_LOG=info,wintf::ecs::window_proc=debug,wintf::ecs::drag=debug,areka::placement=debug`・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了・実 emo2＋実 pasta.dll）で意図的に DPI 境界跨ぎドラッグを行いログ捕捉。ログ実体はセッション scratchpad（揮発）ゆえ**鍵となる行を本 brief へ転記**する。

### 確定事実（ログ実測）

1. **ドラッグは 1:1 追従・暴走なし**。窓 entity 6v0 のドラッグ（05:51:14.460〜05:51:16.929）で、マウスと窓オフセットの差は開始から終了まで **-137px 一定**：
   ```
   05:51:14.460 [drag] Dragging started entity=6v0 start_x=2883 start_y=1909
   05:51:14.470 [dispatch_drag_events] DraggingState inserted entity=6v0 initial_window_x=2746 initial_window_y=1700
   05:51:16.929 [drag] Dragging ended entity=6v0 x=-670 y=1790 cancelled=false
   05:51:16.935 [DragEnd] Arrangement.offset unchanged window_entity=6v0 offset=(-807,1765)
   ```
   マウス移動量 = -670-2883 = **-3553**／窓 offset 移動量 = -807-2746 = **-3553**（完全一致）。DPI 境界（x≈-289 で WM_DPICHANGED 発火）を跨いだ後も連続・滑らかに追従（2746→…→23→-259→-289→-329→…→-652）。急なワープ・飛びは**ゼロ**。
2. **WM_DPICHANGED は 24 回発火**（entity 6v0/5v0・dpi 120↔192＝125%↔200% のモニタ境界跨ぎ実在）。ハンドラの実挙動は `[WM_DPICHANGED] DPI component directly updated (Changed<DPI>)` のみ。
3. **`guarded_set_window_pos` は 0 回**＝「WM_DPICHANGED→suggested_rect 再配置が placement と衝突する二重位置権威」という静的解析仮説（2026-07-18 read-only 調査で立てたもの）は**この repro では反証**。
4. **窓サイズは DPI 変化でリサイズされない**——dpi 120→192 を跨いでも 336x400（物理）固定。areka の基本設計は **DPI 追従（高DPIでマスコット拡大）**（記憶 areka-dpi-following-core-design・k=1.0 は途中状態）ゆえ、**これが現時点で唯一確定している設計 gap**。200% モニタ上では論理半分の大きさに見える。
5. **旧 placement DPI 座標欠陥（2026-07-05）は現行コードで修正済み＝誤帰属禁止**。`follow.rs:24-26` に「DPI 再スケール（dpi/96 乗除）を一切挟まない」明示檻・`resolver.rs:303-318` に 4 段階 DPI 不変テスト・`wire_drag` シンボルは現存せず。記憶 areka-window-placement-dpi-coordinate-defect の旧2バグ（resolve 単位混在・二重スケール）へ帰属させないこと。
6. **`Anchored 未付与` WARN は別件の良性シャットダウン競合**（`GhostWindows` を despawn 時に未掃除・`placement/mod.rs:99`）。消失の原因ではない。
7. 消失ドラッグの**4秒後**の DragEnd は `x=2856 offset=(2721,1700)`＝プライマリモニタ上へ復帰している（操作者が引き戻した可能性が高い）。

### 未確定（本 spec の調査タスクが確定させるもの）

- **Q1: 消失は「暴走」か「操作どおり」か**——窓がマウスより速く逃げたのか、操作どおり左の 200% モニタへ運ばれただけなのか（診断ドラッグでは 1:1 だったが、当初報告の消失時の挙動は未観測）。
- **Q2: 消失時の「えも」の所在**——全モニタ外（真の画面外）か／左 200% モニタ上に小さく居た（非リサイズで半分サイズ＝見落とし得る）か。モニタレイアウト（各 `Monitor.work_area` の実座標範囲）と消失時窓座標の突合が必要。
- **Q3: 当初報告の消失はドラッグ起因か**——「知らないうちに消えてた」＝ドラッグ以外の経路（自動再配置・re-snap・追従計算）の可能性は排除できていない。
- **Q4: バルーン同時消失は follow の随伴か**——バルーンはキャラ窓へ追従（`follow.rs`）ゆえ、キャラ窓がどこへ行こうとバルーンが随伴して「両方消えた」ように見えるのは整合的。独立バグかは未検証。

## Desired Outcome

1. 消失の**根本原因が実機証跡で確定**している（Q1〜Q4 が全て回答済み）。
2. 確定原因が**理想形で修正**されている。確定内容次第の候補（あるべき姿比較は要件/設計で）: (a) WM_DPICHANGED 時の **DPI 追従リサイズ＋下端 re-snap**（基本設計への追従・`surface-resize-resnap` の資産再利用）／(b) **モニタ外逸失の防御**（可視領域 clamp・画面外からの復帰手段）／(c) その他確定機構への対処。
3. **実 DPI（≠96）回帰檻**が確立している——dpi=96 では全スケール恒等＋WM_DPICHANGED 不発で欠陥が隠れる（実証済み）。headless シームが無い現状では、実機再現手順の文書化 or 新規シーム（k 注入等）の追加が要る。
4. 同域の小掃除: `GhostWindows` の despawn 掃除（良性だが本 spec 域内）。

## Approach

**診断先行の2段構え**（開発者選択済み）: ①診断フェーズ＝モニタレイアウト実測（EnumDisplayMonitors 相当のログ増設 or 既存ログ突合）＋消失時の窓座標トレース＋Q1〜Q4 の確定。②修正フェーズ＝確定原因に対する理想形修正＋実 DPI 檻。要件討議で「DPI 追従の実装範囲」（本 spec で窓リサイズまで踏むか・point÷k ヒットテストは collision-geometry 側か）の線引きを確定する。

## Scope

- **In**: 消失の原因確定・確定原因の修正・実 DPI 回帰檻・GhostWindows despawn 掃除。
- **Out**: idle-talk（SHIORI Status/talk）＝無関係・混ぜない／DPI 追従の全面実装（テキスト・合成・ヒットテストの k≠1.0 全系）＝要件討議で線引き（collision-geometry の point÷k は同 spec 側・記憶 areka-dpi-following-core-design）／SSP 互換の位置復元 UI（M2）。

## Boundary Candidates

- 診断（観測増設＋実機 repro）と修正（機構変更）のフェーズ境界。
- placement（areka::placement＝窓位置の単一権威）と wintf window_proc（WM_DPICHANGED/ドラッグ配送）の層境界。

## Out of Boundary

- 位置の永続化（`ghost.dat`）は `areka-P0-position-persist` の所有。ただし**相互作用に注意**: 画面外位置を保存→次回起動で見えないゴースト、の合成不具合があり得る（position-persist 側要件との突合事項として申し送り）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-window-placement`（物理 px 単一通貨・DragPositionPolicy 単一ライター）・`completed/areka-P0-surface-resize-resnap`（`project_anchor`/`resize_window_to`＝リサイズ＋re-snap の再利用資産）・wintf drag/window_proc 基盤。
- **Downstream**: `areka-P0-position-persist`（W3・画面外位置の保存問題）・`areka-P0-emo2-conformance-e2e`（適合一周の実機安定性）。

## Existing Spec Touchpoints

- **Extends**: なし（新規調査）。
- **Adjacent**: `position-persist`（**follow.rs/spawn.rs 共有＝干渉**・W3）／`collision-geometry`（DPI 下ヒットテスト＝隣接・W1 進行中）。
- **編集面の見込み**: `crates/areka/src/placement/{follow,spawn,mod}.rs`・`crates/wintf/src/ecs/window_proc/{window_pos,mouse_move}.rs`・`crates/wintf/src/ecs/drag/*`。
- **着手時期**: 診断・要件・設計の**文書/観測フェーズは随時先行可**（フェーズ別ゲート精密化の原則）。**実装は W3（position-persist）完了後**——follow.rs/spawn.rs 干渉のウェーブ直列化（開発者方針「少しでも干渉するならウェーブを分ける」）。正式なウェーブ割当は次の合流/編成セッションで（記憶 portfolio-convergence-decided-in-separate-session）。

## Constraints

- **実 DPI（≠96）実機必須**: dpi=96 は自己整合して欠陥を隠す（記憶 areka-placement-real-ghost-first／areka-window-placement-dpi-coordinate-defect）。本番 emo2 表示＋実マルチモニタで検証。
- **有界実機 repro の定石**（記憶 areka-real-machine-signoff-bounded-auto-exit）: i686 helper を先ビルド→`target\debug\` へコピー、`$env:RUST_LOG="info,wintf::ecs::window_proc=debug,wintf::ecs::drag=debug,areka::placement=debug"`・`$env:AREKA_APP_SMOKE_EXIT_MS`（大きな値）で起動し `*>` でログ捕捉。ANSI 色コード混入ゆえ grep は素の部分文字列で。
- placement の規律を壊さない: 物理 px 単一通貨・DPI 再スケール（dpi/96）を挟まない檻（`follow.rs:24-26`）・単一ライター原則。
