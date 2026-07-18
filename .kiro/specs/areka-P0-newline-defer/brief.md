# Brief: areka-P0-newline-defer

> SSP 準拠の改行遅延（deferred newline）——改行マーカーは「次の文字が実際に出力されるまで」行送りを保留する

## Problem

実機（emo2 fixture・pasta SHIORI）で、キャラ A からキャラ B へ会話が移るとき、A のバルーンが**意図せず1行分改行（スクロール）する**。A でトークが終了する場合は A の最終トークに改行は入らない（2026-07-18 開発者報告）。

## Current State

### 症状の因果連鎖（2026-07-18 discovery で実測確定）

1. pasta の `sakura_builder.lua`（spot 切替時の段落区切り・`spot_newlines` 既定 1.5）は `…Aテキスト\n[150]\1Bテキスト…` の形＝**切替タグの前**に `\n[150]` を発行する。さくらスクリプト意味論上、この改行はスコープ A のバルーン宛て。
2. areka-emo-text の現行実装は改行マーカーを**到着即時**に反映する:
   - `layout.rs:211-224` — `TextItem::LineBreak` で即座に行を閉じ `block_pos += pitch × ratio`
   - `layout.rs:748` — 末尾改行（全グリフ可視）は空の新行を開く（檻あり）
   - `layout.rs:1016` — **末尾の空行も「最新行」としてあふれ判定に参加**（檻あり）
   - `draw.rs:1705` — あふれ発火後は先頭行が供給面から消える（行単位スクロール・檻あり）
3. よってバルーンが満杯に近いとき、trailing `\n[150]` だけであふれが発火→先頭行がスクロールアウト＝「1行改行したように見える」。

### SSP の正準挙動（開発者観測・本 spec の準拠目標）

SSP では同一スクリプトでこの現象は**起きない**。SSP は改行を「実際に次の文字が出力されるまで」**遅延（pending）**しており、実体化しない改行は行を開かず・あふれ判定にも参加しない、と推測される（開発者観測 2026-07-18。ukadoc はこの粒度の描画意味論を記述していないため、要件フェーズで必要なら実機 SSP での追観測を任意で行う）。

### pasta 側は正当（重要・犯人説の撤回記録）

SSP の遅延意味論の上では、pasta が段落区切りを**切替タグの前**（＝直前話者のスコープ宛て）に置く設計は正しく機能する:
- A→B で A に積まれた pending 改行は、A が再登壇して次のグリフを出すときに初めて実体化＝**段落区切りとして意図通り**
- A が二度と話さなければ pending のまま蒸発＝**不可視**

discovery 初期の「pasta の off-by-one」診断は誤りとして撤回（2026-07-18・開発者の反証「SSP では出ない」が契機）。**pasta・fixture・compile.rs（scope 切替で改行を発行しない）は全て非改変**。

## Desired Outcome

- 改行マーカー（`\n`／`\n[ratio]`）は**同一バルーン内で次のグリフが実際に配置されるまで行送りを保留**する。
- 保留中の改行は行を開かず・あふれ判定に参加せず・スクロールを誘発しない。
- 次のグリフ配置時に保留分（連続改行は ratio 累算）が一括実体化し、その時点であふれ判定・スクロールが従来通り働く。
- 実体化しないまま talk が終わった保留改行は蒸発する（`\c`／`ClearAll` でも破棄）。
- 実機で「A→B 切替時の意図せぬ1行スクロール」が消え、A→B→A の段落区切りは維持される。

## Approach

areka-emo-text の**純粋層（layout）で即時反映を pending 化**する。判断分岐は純粋層に閉じており GPU 不要で全網羅可能（檻対象＝判断分岐のみの方針通り）。既存の即時意味論を檻に入れているテスト群（`layout.rs:748/1016` 系・`draw.rs:1705` 系ほか）は仕様判断の変更に伴い**更新**（陳腐化でなく意味あり→更新、の方針）。

## Scope

- **In**:
  - layout 純粋層の改行遅延化（pending 蓄積・ratio 累算・次グリフ配置時の一括実体化・あふれ判定/スクロールの実体化時評価）
  - タイプライタ（reveal）との整合: 可視 prefix 内の改行マーカーは「次のグリフが reveal されたとき」に実体化（完了 spec emo-text-layer R2.2「即時反映・後出し優先」の**意味論改訂**）
  - `\c`／`ClearAll`／talk 終了時の pending 破棄
  - 縦書き（軸読み替え正準表）でも同一規則
  - 既存檻の更新＋遅延意味論の新檻（決定論・純関数・全網羅）
  - 実機確認（AREKA_APP_SMOKE_EXIT_MS 有界 auto-exit＋ログ grep の定石・A→B 切替トークで現象消失）
- **Out**:
  - pasta／fixture／サブモジュールの改変（正当と裁定済み）
  - areka-sakura compile の改変（scope 切替で改行を発行しておらず無関係）
  - バルーンのあふれ→スクロール機構それ自体の変更（実体化タイミングだけを変える）
  - `\_l` 等の他レイアウト系タグの新規対応

## Boundary Candidates

- **pending 状態の置き場**: layout 純粋層の入力走査内で完結するか（走査中のローカル状態で足りる見込み・`layout.rs:211` の分岐を「即 push」→「pending 加算」へ）、canvas 側の item 保持（`canvas.rs:465`「空行も住人・行 index 1:1」不変条件）に触れるか——**canvas の item 列は非改変が理想**（改行マーカーは住人のまま・実体化判定は layout の解釈で吸収）。design で確定。
- **あふれ判定の入力**: 「最新行」の定義から未実体化の空行を除く（`visible_window` 決定の入力整形）。

## Out of Boundary

- SSP の他の描画差異（フォント・descent 等）は emo-text-byte-equiv 系の既知論点であり本 spec は扱わない。
- `\n[percent]` の ratio 解釈そのもの（既存正準表）は不変。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo-text-layer`（改訂対象の意味論 R2.2・R7 系あふれ判定）／`completed/areka-P0-cue-playback-duration`（NewLine cue は瞬時 duration 0——cue 層は非改変）
- **Downstream**: `areka-P0-emo2-conformance-e2e`（W5・適合走行で本修正の実機効果を最終確認）／`areka-P0-choice-render`（W3・同 crate の `state.rs` を編集——**本 spec は layout.rs/draw.rs 系でファイル素だが同 crate 近接**）

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-emo-text-layer`（意味論改訂＝R2.2 即時反映→遅延・末尾空行のあふれ参加→実体化時参加）
- **Adjacent**: `areka-P0-choice-render`（emo-text `state.rs:224-229` が宛先・本 spec は不触）／`areka-P0-collision-geometry`（emo 系だが emo-compose/present 側・交差なし）

## Constraints

- **ウェーブ編成との関係（少しでも干渉するならウェーブを分ける・roadmap 追記㉙㉚）**: W1 の3本（idle-talk／collision-geometry／sakura-dialogue-tags）とは**共有ファイル 0**（emo-text はどの W1 ユニットの編集面でもない）＝ W1 と並走可、または W1 前の単発挿入。**W3 の choice-render と同 crate**（別ファイル）のため、**W3 開始前に完了させる**こと。
- 決定論テスト網羅必達・実装第一（テストのため実装を歪めない）・ログ無し失敗経路の禁止。
- 実機サインオフは有界 auto-exit＋ログ grep の定石（AREKA_APP_SMOKE_EXIT_MS=180000・RUST_LOG=info,kanade=trace）＋出力画像の AI vision 目視（emo-text の盲点対策）。
