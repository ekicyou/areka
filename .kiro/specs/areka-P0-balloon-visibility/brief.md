# Brief: areka-P0-balloon-visibility

> バルーン表示ライフサイクル（自然な表示・消去・再表示）。`/kiro-discovery` 2026-07-23 発。
> 実測（コード配管・7項目）と正典（ukadoc・6項目）の二重裏取り済み。file:line は本日時点の実測。

> **📌 2026-07-23 追記㊵陳腐化補正（本ブロックが以下の本文より優先）**:
> - **M1 編入・裁可済み**（2026-07-23 開発者裁定＝追記㊲の「裁可待ち」決着）・**ウェーブ配置=W6 単独**（攻め5ウェーブ再編＝旧 W7.5 表記は失効。前提: W3 `seriko-loop`〔frame.rs 同一関数 `Emo2Wiring`/`run_emo2_frame` の解消〕・W4 `choice-interact`〔バルーンポインタ配線 donor〕・W5 `dpi-window-vanish`〔spawn.rs 解放〕・W3 `sylphya`〔timeout 既定値のプロパティ化〕）。
> - **本 brief は main 着地済み**（PR #79・2026-07-23 マージ）＝本文末尾の「branch 未着地」自己申告は失効。
> - e2e 適合項目へのバルーン表示ライフサイクル追補は W7 `emo2-conformance-e2e` 着手時に実施（roadmap W7 行に登記済み）。

## Problem

現在の areka はゴースト起動時から**バルーンが常時表示**されている。実際の伺か（SSP de-facto）の自然な挙動は:

1. **実際に喋る内容が発生するまでバルーンは表示されない**
2. **キャラ（scope）ごとに喋り出したときにはじめて表示**される。片方が喋り出しても、喋っていない方の scope のバルーンまで表示開始されることはない
3. **talk 再生終了から一定時間（例: 30秒）経過**し、かつバルーンが**アクティブフォーカスを持っていなければ**、全表示中バルーンが消える
4. **次のさくらスクリプトで会話が開始した瞬間**にバルーンは再表示される

（開発者指定 2026-07-23・本 spec の要求の正本）

## Current State（2026-07-23 実測）

**手段は貫通済み・頭脳が不在**、が現状の一言要約。

- **起動時常時表示の主犯**: `crates/areka/src/emo2_boot/frame.rs:438-446` `run_attach_phase` が attach ゲート成立フレームに**バルーン target へ無条件 `ShowSurface{surface_id:0}`** を発行。実機欠陥 #5 の「first `\s` まで非表示」修正（`frame.rs:396-403`）は**シェル側のみ**でバルーンに未適用。
- **装着順序の罠**: `text_slot_view` 取得（`frame.rs:449`）が初回 ShowSurface に依存＝**スロット取得と可視化の分離**が改修の前提（emo2-boot 申し送り「text_slot_view は初回 ShowSurface まで None」と同根）。
- **非表示の実行経路は貫通済み**（balloon-face-cue 遺産）: seriko `apply_balloon`（`crates/areka-seriko/src/state.rs:149,170`）→ `DisplayCommand::HideBalloon{scope}` → adapter `map_display_command`（`crates/areka/src/emo2_boot/adapter.rs:61-64`）→ `PresentCommand::Hide` → presenter `apply_hide`（`crates/areka-emo-present/src/presenter.rs:379,393`）→ `VisualMount::set_visible(false)`＋HitTest 停止（`mount.rs:193`）。冪等ガード（状態不変時は再発行しない）も seriko に既存。
- **scope 情報は cue 全段に載る**: `\0`/`\1`/`\p[N]` は `crates/areka-sakura/src/compile.rs:48,64-65` で `scope`→各 cue の `actor`（`ActorKey`）へ転写・broadcast 配送で全 sink が選別可。
- **TalkDone は kanade 止まり**: `TalkDone{reason 3値}`（`crates/areka-talk`）は dispatcher（`crates/areka-ghost/src/dispatcher.rs:131-149`）→ kanade `on_talk_done`（`steady.rs:226`）まで。**emo/UI へは未配線**＝talk 終了信号の UI 配線が新設面。
- **時間源**: ghost ticker（`crates/areka-ghost/src/ticker.rs:47-66`・kanade 1000ms Tick）／UI 側 `FrameTime`＋`TalkClock`（`crates/areka/src/emo2_boot/talk_clock.rs:21,63`・`frame.rs:683` 毎フレーム解決）。
- **フォーカス観測の土台ゼロ**: バルーン窓は `HitTest::none()`＋ポインタハンドラ無し（`spawn.rs:163-178`・DD-IE-12 の意図的除外）。`OnDrag(on_balloon_drag)` のみ装着済み（バルーン単独ドラッグは window-placement で実装済み）。wintf のポインタ/hit-test 機構自体は流用可。
- **内容クリアと窓非表示は独立レイヤ**: `Clear`/`ClearAll`（cue-playback-duration 由来・talk 冒頭に `ClearAll` 単一前置＝`compile.rs:210-226`）は emo-text の**内容消去**。窓の可視は emo-present の別配線。内容を消しても空のバルーン枠は表示されたまま。

## Canon（ukadoc 裏取り・2026-07-23）

**正典が明確に規定**:
- **`\![set,balloontimeout,時間]`**: ms 指定・**カウント起点「スクリプトの表示が終わってから」**・当該スクリプト中のみ有効・0/-1=タイムアウトしない・省略=デフォルト値（＝ベースウェア本体設定の存在が前提）。
- **SSP 本体設定「喋りタイムアウト」の存在**は `OnSurfaceRestore` の記述（「喋りタイムアウト設定秒数+15秒後＝**バルーンが閉じてから**15秒後に発生」）が正典裏付け。
- **SHIORI イベント語彙**: `OnBalloonClose`（Ref0=表示中スクリプト）／`OnBalloonTimeout`（選択肢以外のタイムアウト・Ref0=スクリプト,Ref1=残り時間）／`OnBalloonBreak`（SSTP 以外の中断・Ref0=スクリプト,Ref1=**scope 番号**,Ref2=中断位置〔タグ込み文字数〕）。OnBalloonClick は**存在しない**（クリック閉は OnBalloonClose に集約）。
- **寿命に影響するタグ**: `\x`（クリック待ち・クリック後 scope リセット）／`\x[noclear]`（内容と scope 保持）／`\t`（タイムクリティカル・「**バルーンダブルクリックによる中断**」がスクリプトブレークとして正典明記）／`\*`（**選択肢**タイムアウト抑止）。選択肢系 `\![set,choicetimeout]`/`OnChoiceTimeout` は**別系統**（choice-select-events 所有・二重所有禁止）。

**ukadoc 無規定＝SSP de-facto**（開発者指定挙動が本 spec の正）:
- バルーン出現の正確な条件（scope 切替 vs コンテンツ発生）——ただし `\_s`「両**バルーンに表示**」等、表示はメッセージ（コンテンツ）駆動の語法が傍証。
- マウスオーバーでの延命・`\e` 後の残存明文・無発話 scope のバルーン非表示。

**emo2 実物（fixture 実測）**: dic 全体に `OnBalloonClose`/`OnBalloonTimeout`/`OnBalloonBreak` ハンドラ・`balloontimeout`・`\x` は**皆無**（`OnBalloonChange`＝メニューからのバルーン切替のみ存在・本件と別物）。→ SHIORI 発火系の M1 縮退は実物根拠あり。

## Desired Outcome

実機 emo2（実 pasta.dll・実 DPI）で: 起動→**バルーン不可視**→起動挨拶 talk で**発話 scope のバルーンのみ出現**（内容と同時）→talk 表示終了から既定時間（暫定 30 秒）経過＋フォーカス無しで**全表示中バルーン消滅**→次の自発 talk 開始で**再出現**——が決定論テスト（注入時刻・sleep 不使用）と実機サインオフ（有界 auto-exit＋ログ grep 定石）の両方で確認される。

## Approach

**統一原理: 「バルーンの可視性＝可視コンテンツの存在に従属」**——emo テキストエリアの「ビューボックスは可視コンテンツを実際に置いた瞬間のみ拡張」モデル（newline-defer 討議 2026-07-18 確立）と**同型**。show/hide を場当たりのフラグでなく単一原理から導出する:

1. **show**: scope へ**最初の可視コンテンツが配置された瞬間**、当該 scope のバルーンのみ `ShowSurface`（要求 1・2・4 を単一規則で充足）。
2. **talk 冒頭 `ClearAll` → 全バルーン即 Hide**: 内容が消えた枠を残さない。前 talk のバルーンが表示中でも新 talk 開始で一旦消え、発話 scope だけがコンテンツ配置で再表示（「喋っていない方のバルーンは出ない」の成立機構）。
3. **timeout hide**: talk 表示終了（正典カウント起点＝dola 占有 horizon / `TalkDone(Ended)`）から既定時間（暫定 30s）経過し、フォーカス抑止が無ければ全表示中バルーンを Hide。
4. **明示指令 `\b[-1]`（既存経路）は常に即時・本 spec は非干渉**。

**推奨アーキテクチャ（A案）: UI 層 BalloonVisibility コントローラ**（emo2_boot 新モジュール・frame 駆動）
- emo＝UI 層全般の所有者（roadmap「emo の責務範囲」宣言）に整合。show トリガ（可視コンテンツ配置）は emo-text 状態遷移の観測＝UI スレッド内で完結。タイマは `FrameTime`。talk 終了信号は TalkDone の UI 配線（PresentBridge 同型 mpsc）または dola horizon の UI 側観測（design で確定）。
- **棄却 B案（seriko 頭脳）**: ShowBalloon/HideBalloon 発行点は既存だが、seriko は時間源なし（Tick 口は seriko-loop W4 で増設予定）・talk 終了もフォーカスもコンテンツ配置も知らない＝3 知識全部の注入が要り太る。
- **棄却 C案（kanade 頭脳）**: StartTalk/TalkDone/1s Tick は既有だが、表示状態・コンテンツ配置・フォーカスは UI 側知識＝kanade が表示詳細を持つのは責務違反。ただし ukadoc `Status` の **`balloon`（表示中バルーン ID 群）** 報告のための UI→kanade 状態通知の口（input-events `MouseWiring` 同型）は**どの案でも別途必要**——`status-execution-states` 台帳の `balloon` 状態の**源が本 spec で着地**する点に注意。

## Scope

- **In**:
  - 起動時無条件 ShowSurface（`frame.rs:438-446`）の撤去＋**text スロット取得と可視化の分離**
  - コンテンツ駆動 per-scope show／`ClearAll` 連動全 hide／timeout 全 hide（既定値暫定 30s・値の源は討議）
  - talk 終了信号の UI 配線（TalkDone or horizon・design で確定）
  - フォーカス抑止（M1 の定義は討議——最低限バルーンドラッグ中は抑止。hover は配線新設の要否込みで討議）
  - 決定論テスト網羅（注入時刻・fake 信号）＋実機サインオフ
- **Out**:
  - `OnBalloonClose`/`OnBalloonTimeout`/`OnBalloonBreak` の **SHIORI 発火**（emo2 消費者ゼロ実測。語彙は本 brief に完全収録・発火シーム〔kanade 送出ホワイトリスト＋UI→kanade 通知路〕を型シームで予約・実発火は追跡台帳へ——defer-canon 4点セット）
  - `\x`/`\x[noclear]` クリック待ち（emo2 不使用・語彙とシームのみ）
  - `choicetimeout`/`OnChoiceTimeout`/`\*`（choice-select-events 所有・選択肢表示中の timeout 抑止は**連携シーム**のみ本 spec）
  - communicatebox/inputbox の寿命・`OnSurfaceRestore`・バルーン切替 UI・`OnBalloonChange` 対応

## Boundary Candidates

- **可視性の頭脳（新設・本 spec 所有）** vs **表示の実行手段（既存・presenter/seriko 経路を消費するだけ）**
- **窓の可視**（emo-present `VisualMount`）vs **内容**（emo-text `Clear`/`ClearAll`）——独立レイヤのまま、頭脳が両方を観測して可視を導出
- **talk 生死の知識**（kanade/dispatcher/dola）vs **表示判断**（UI 層）——信号だけを橋渡し

## Out of Boundary

- 選択肢のタイムアウト・確定カスケード（choice-render/interact/select-events の三連 spec）
- Status 全 10 状態の語彙管理（status-execution-states 台帳——本 spec は `balloon` 状態の**源**として台帳 1 件を実導出解禁するかを討議）
- バルーン内テキストレイアウト・スクロール（emo-text 完了領域）

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo2-boot`（frame/adapter/PresentBridge）・`completed/areka-P0-balloon-face-cue`（Show/HideBalloon 経路）・`completed/areka-P0-cue-playback-duration`（占有 horizon・`ClearAll` 前置）・`completed/areka-P0-emo-text-layer`（コンテンツ配置の観測点）・`completed/areka-P0-ghost-setup`（dispatcher/ticker）・`completed/areka-P0-input-events`（UI→kanade 配線 donor・dblclick 中断）・`completed/areka-P0-window-placement`（バルーンドラッグ・follow）
- **Downstream**: `areka-P0-status-execution-states`（`Status: balloon(ID群)` の源）・`areka-P0-choice-render`/`-interact`/`-select-events`（選択肢表示中の延命連携）・`areka-P0-emo2-conformance-e2e`（適合項目への「バルーン表示ライフサイクル」追補を e2e 着手時に検討）

## Existing Spec Touchpoints

- **Extends**: なし（新境界）
- **Adjacent（編集面の干渉＝ウェーブ配置の根拠）**:
  - `seriko-loop`（**W4**）: `frame.rs` **単独所有**——本 spec の主改修点と同一ファイル → W4 完了が前提
  - `choice-render`（W4）: emo-present `presenter.rs` additive → 同前
  - `choice-interact`（**W5**）: バルーンへの実ポインタ配線＝hover 抑止の donor → W5 完了が前提
  - `choice-select-events`（W6）: kanade events 面（SHIORI 発火シームを実装する場合に交差）
  - `emo-dpi-scaling`（W6）: broad な emo 基盤改修 → 並走不可
  - `collision-dpi-hittest`/`dpi-window-vanish`（W7）: emo-present hit_region・placement follow/spawn——ファイル単位では非交差だが「少しでも干渉するならウェーブを分ける」方針に従い併走は推奨しない
  - `mayuna-compose`（**W2・discovery 時点で実装中**）: 実編集面に `emo2_boot/frame.rs` を含む（下記追補の実測）→ ウェーブ直列（W2 完了後に W7.5）が安全根拠。**「ファイルが別だから安全」ではない**点に注意

## Constraints

- **ウェーブ配置: W7 完了後・W8（e2e）前の単独ウェーブ（仮称 W7.5）を推奨**——上記干渉台帳より W4/W5 は必須前提・W6/W7 とも分離が安全。**開発者裁可待ち**（roadmap 追記㊲）。
- タイムアウト既定値は暫定 30s（開発者例示）。**値の源**は sylphya（W3 完了済み前提の「名前で引ける値」）プロパティ化を討議。`\![set,balloontimeout]` の M1 受理は `\!` 汎用キャリア規律の**時間指令 allowlist** 候補として討議。
- 決定論テスト必達（注入時刻・sleep 不使用）・実機サインオフは有界 auto-exit＋ログ grep 定石。
- Rust 2024・新規依存なし想定・`cargo test --workspace` green（i686 host-32 成果物ビルド後）。

## Open Questions（要件/設計討議へ持ち越し）

1. **フォーカスの定義（M1）**: (a) バルーン hover（DD-IE-12 の意図的除外を部分解除・choice-interact のポインタ配線を donor に新設）／(b) バルーンドラッグ中のみ（既存 OnDrag で観測可・最小）／(c) 選択肢・クリック待ち状態はシームのみ。推奨は (b)＋(a) を設計で見極め。
2. フォーカス喪失時の挙動: 残カウント再開か・カウントリセットか・即時 hide か（SSP de-facto 未検証）。
3. `balloontimeout` タグの M1 受理有無（emo2 不使用＝縮退可・ただし時間指令 allowlist として自然）。
4. SHIORI 発火（OnBalloonClose/Timeout/Break）の M1 有無——推奨: 非発火＋シーム（fixture 実測根拠）。
5. `Status: balloon(ID群)` の M1 実導出有無（源は本 spec で着地・UI→kanade 通知路の要否と一体）。
6. 中断系（dblclick スクリプトブレーク等）での即時 hide の要否（`\t` 正典・input-events の中断挙動との整合）。
7. talk 終了信号の実装形: TalkDone の UI 配線 vs dola 占有 horizon の UI 側観測（`TalkClock` 近傍）。

## 追補: セッション内 Q&A 確定事項（2026-07-23・discovery 同日）

### 実現層の確定（開発者質問への回答）

主役は **⑥ emo 帰属**（roadmap「emo の責務範囲＝UI 層全般」宣言どおり）。ただしコードの落ち先は seriko/emo-present の**内部ではなく `crates/areka/src/emo2_boot/` 統合層**（新モジュール＋`frame.rs:438-446` 是正）＝`areka-P0-emo2-boot` と同じ統合層性格の spec。内訳:

| 役割 | 置き場 | 新規/消費 |
|---|---|---|
| 頭脳（可視性コントローラ） | `emo2_boot/` 新モジュール＋`frame.rs` 是正 | **新設** |
| 表示実行 | emo-present `VisualMount` show/hide | **既存消費**（balloon-face-cue 経路） |
| talk 終了信号の UI 配線 | ⓪ghost/③kanade `TalkDone`→UI の薄い線（kanade 本体無改修） | **新設** |
| フォーカス観測 | バルーン窓ポインタ配線（現 `HitTest::none()`） | **新設** |

### mayuna-compose（W2・着せ替え）との干渉実測

- **結論: 衝突しない。ただし根拠は「ファイルが別」ではなく「ウェーブ直列」**（W2 完了マージ→…→W7.5 着手）。
- 実測（`git diff --stat main...claude/areka-p0-mayuna-compose-47cd1c`・2026-07-23）: mayuna の編集面は seriko `state.rs`(+522)/`actor.rs`(+761)/新規 `bind.rs`(+723)・emo-present `cache.rs`(+78)・parsers `package/*`・**`emo2_boot/frame.rs`(+6)** ——frame.rs は本 spec の頭脳配置先と**同一ファイル**。並走させれば当たる。
- ただし mayuna の frame.rs 変更は `:337` 付近の struct 分解へ `bind_resolver: _` 追加＋テストのみ＝**主犯 `:438-446` は無傷**。
- mayuna は「シェルは初 `\s` まで非表示・bind は初 `Show{binds}` に載る」構造を強化中＝本 spec がバルーンへ適用したい原理の**先例**。本 spec は mayuna settle 後の main の上に積む関係。
- 教訓: 本 brief の file:line 引用は W2〜W7 のマージで陳腐化しうる（並走 brief 陳腐化の既知則）→ **着手時（設計前）に settled main へ実測再突合**すること（`git log <base>..origin/main`＋`git diff --stat`・`git show origin/main:path` は PowerShell で）。

### 継続時の所在（重要・別セッション申し送り）

- 本 brief と roadmap 追記㊲は **branch `claude/areka-ghost-balloon-behavior-28c177`**（collision-geometry worktree 転用）に commit `3d520542`＋追補 commit として存在し、**main 未着地**。
- 別セッションで継続する場合: このブランチを拾う（同 worktree 続行 or main へ PR 着地させてから新 worktree）こと。**main から新 worktree を切ると本 brief が見えない**——input-events 完了時の「brief は main に存在」前提と異なる点に注意。
