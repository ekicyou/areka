# Brief: areka-P0-choice-interact

> `\q` 選択肢の**対話面**——実ポインタ→hover 追従→クリック確定→`ChoiceSelection` 発行（2026-07-19 追記㉟＝choice-render 2分割の対話半分・開発者裁定）

> **📌 2026-07-24 追記㊹陳腐化補正（W3 完走・本ブロックが㊵以下より優先）**:
> - **上流 choice-render ✅完了（2026-07-24・実機サインオフ済）＝「供給予定」だった契約3 API は実物**: 所在は emo-present presenter.rs でなく **emo-text `actor.rs`（`TextLayerRuntime`）**——`ChoiceHitRow` :150・`inject_choice_hover` :366・`choice_hit_rows` :389・`choice_active` :400。
> - emo-text の「良性スキップアーム（state.rs:234-241）」は撤去済み＝Choice cue は実消費（state.rs:367-400・`ChoiceSpan` :254）。
> - **実機 hover 注入導線が donor として実在**: `emo2_boot/hover_inject.rs`＋frame.rs:707-711（env `AREKA_CHOICE_HOVER_INJECT`・本番既定無効）＝design で実ポインタ駆動との置換/共存を裁定すること。
> - spawn.rs:80（BalloonWindowMarker＋DragConfig）・input_events/mod.rs:96（DD-IE-10）・dola runtime.rs:102/:246/:291・contract.rs:38／drive.rs:167,:355 は 2026-07-24 実測で全て一致＝そのまま前提にできる。
> - W4 開始は割込 `wintf-gpu-test-crash` 完了後（追記㊹裁定）。

> **📌 2026-07-23 追記㊵ウェーブ更新**: 攻め5ウェーブ再編により本 spec は **W4**（`position-persist` ∥ `emo-dpi-scaling` と3本同居）。本文の「W4 choice-render」「W5（単独）」等は「W3 choice-render」「W4 同居」へ読み替える。**同居の事前割当契約**: `spawn.rs` は position-persist 単独所有＝本 spec のバルーンポインタ配線は **input_events モジュール＋emo-text 幾何消費で完結**させる（バルーン窓は `BalloonWindowMarker`＋DragConfig 済〔spawn.rs:80〕＝spawn.rs 改変不要見込み。設計が spawn.rs 改変を要求したらその部分を W5 へ＝エスケープ条項）。W5 `collision-dpi-hittest` が `input_events/mod.rs` を後続共有するため、バルーンハンドラ増設は同ファイルの DPI 素通し規約（mod.rs:96・DD-IE-10）を壊さない形で。

## Problem

choice-render（描画半分・W4）は選択肢 resident の描画・`\_l` 消費・ハイライト描画を**注入 hover 状態**で決定論的に実現するが、実ポインタがバルーン窓からどう届き、どの行が hover 中で、クリックがいつ「選択確定」になるかの**対話面が無所属**になる。この配線が無いと M-dialogue の「ダブルクリック→メニュー→選択→遷移」一周は完走しない。

## Current State

- **choice-render（W4・先行）が正本を供給予定**: 選択肢**行ヒットジオメトリ**（行矩形群）＋ **hover 状態 API**（注入 hover の設定/解除口）。本 spec はその消費者。
- **バルーン窓ポインタ配線の donor は W2 input-events**（キャラ窓のポインタ配線と同型・配線モジュール側で増設・`frame.rs`/`spawn.rs` 本体の扱いは design で確定）。
- **下流機構は settled**（2026-07-19 実測）: dola `CuePlayer` の `pending_choices`（runtime.rs:102）・`resolve_choice`（runtime.rs:291）・`WaitingForChoice` 遷移（runtime.rs:246）。sakura `SakuraMsg::ResolveChoice`（contract.rs:38・handler drive.rs:167/355）＝talk アクター境界の型付き入力（sakura-dialogue-tags W1 完了で実物）。**`resolve_choice` を外部から直接呼ぶ経路は存在しない**（drive.rs:343 doc）＝解決の配送は kanade 経由が正規。
- emo-text の Choice cue 良性スキップアーム（state.rs:234-241）は choice-render（描画）が消費予定。

## Desired Outcome

- バルーン上の実ポインタ移動で hover 行が追従し（choice-render の hover API 呼出）、クリック確定で **`ChoiceSelection`（本 spec が契約正本・choice-select-events が消費）** が一度だけ発行される。
- talk 切替・choice 消滅時の stale クリックは棄却される（原子性ガード）。
- 実機で「メニュー表示→ポインタで行ハイライト→クリック」まで到達する（カスケード発火は choice-select-events の領分）。

## Approach

W2 input-events のポインタ配線 donor に倣い、バルーン窓のポインタイベントを emo-text の行ヒットジオメトリへ写像→hover API 駆動→クリック時に行 id を `ChoiceSelection` へ確定。決定論檻は**注入ポインタ列**（実窓不要の純粋判定＋配線の存在チェック）で全網羅し、実機はポインタ→ハイライト→クリック到達の目視サインオフ。

## Scope

- **In**:
  - バルーン窓ポインタ→選択肢行 hit 判定（choice-render の行ジオメトリ契約を消費）
  - hover 状態の更新駆動（choice-render の hover API 呼出・自前描画はしない）
  - クリック→`ChoiceSelection` 発行（**契約正本**: 選択 id・ラベル・scope 等のワイヤ形は requirements で確定）
  - stale クリック棄却（talk 切替・choice 消滅後の原子性）
  - 決定論檻（注入ポインタ列・sleep 不使用）＋実機サインオフ
- **Out**:
  - 選択肢の描画・レイアウト・ハイライトの**描画**（choice-render W4）
  - SHIORI カスケード（OnChoiceSelectEx→OnChoiceSelect→任意名直接発火）・`Status: choosing`・timeout（choice-select-events W6）
  - `resolve_choice` の直接呼出（配送は kanade 経由の正規経路のみ）
  - キャラ窓側ポインタ配線の変更（input-events W2 の成果を消費のみ）

## Boundary Candidates

- ポインタ配線の増設位置（配線モジュール側 vs frame.rs）——W2 input-events の実装形に倣う（design で確定）。
- `ChoiceSelection` の配送先（kanade inbox の型）——choice-select-events との契約辺（ワイヤ形は本 spec 正本・受信処理は select-events）。

## Out of Boundary

- ホイール/キーボードでの選択肢操作（M2）。
- 選択肢以外のバルーン内リンク（`\_a` 等・emo2 未使用）。

## Upstream / Downstream

- **Upstream**: `areka-P0-choice-render`（W4・行ジオメトリ＋hover API 正本）／`areka-P0-input-events`（W2・ポインタ配線 donor）／`completed/areka-P0-sakura-dialogue-tags`（choice cue 形・ResolveChoice 口）／`completed/areka-P0-cue-playback-duration`（CuePlayer/pending_choices）
- **Downstream**: `areka-P0-choice-select-events`（W6・ChoiceSelection 消費＝カスケード発火）／`areka-P0-emo2-conformance-e2e`（W8・適合項目 #7/#8）

## Existing Spec Touchpoints

- **Extends**: なし（新設・choice-render 旧 brief の対話系スコープを継承）
- **Adjacent**: `areka-P0-choice-render`（同 crate emo-text——本 spec は対話面のみ・描画面不触の割当を design で厳守）

## Constraints

- **ウェーブ**: **W5（単独）**——W4 choice-render 完了後（行ジオメトリ＋hover API が実物になってから）。
- 決定論テスト網羅必達・ログ無し失敗経路の禁止。
- 実機サインオフは実 emo2・実 DPI・絶対パス起動の定石。
