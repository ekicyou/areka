# Brief: areka-P0-balloon-face-cue

> **位置づけ**: M-boot 追加ユニット（2026-07-11 新設）＝ **`areka-P0-emo2-boot` のブロッカー**。emo2-boot の要件精査（`/kiro-start` 要件ディスカッション議題1）で検出された「バルーン面切替 `\b` が cue ドメインの第一級市民でない」構造欠落を、絆創膏（no-op 裁定）でなく正面から解消する（開発者裁定 2026-07-11）。本 spec 完了まで emo2-boot は requirements-generated で中断保留。

## Problem

さくらスクリプトのバルーン面切替 `\b[ID]` は、シェルの `\s[ID]` と対をなす正典表示指令（バルーンサーフェス切替・`-1` で非表示）だが、areka の cue パイプラインでは**三重に無音破棄**され、表示層に一切届かない：

1. **parser**: `b` がタグ表に無く `Instruction::Raw("\b[ID]")` 落ち（`decode.rs:183` passthrough）。
2. **parser（旧形式・バグ級）**: 裸形 `\bN` は lexer が `Bare('b')`＋`Text("N")` に分割し、**数字がバルーン本文へ漏れる可視破損**（`lexer.rs:162-167`・shorthand 語は `w` のみ）。
3. **sakura compile**: Raw は catch-all で `tracing::debug!` 破棄（`compile.rs:81`）→ cue 0件。dola `CueCommand` にバルーン面 variant 自体が不在。

さらに構造問題として、**Balloon 分類 cue は全て TextSink（emo-text 文字状態機械）行き**であり、「バルーンへの表示指令」を運ぶ配管が存在しない（非 text コマンドの text sink 到着は設計 drop・`state.rs:195-197`）。バルーン面切替は文字状態でなく **compose/present 系の表示指令**であり、text 配管は誤配線になる。

この欠落により emo2-boot R5（`\b` 受信→no-op＋warn）は**前提破綻＝受入基準が永久空振り**と判明した（要件ディスカッション議題1・2026-07-11）。

## Current State（実シンボル調査 2026-07-11）

- **dola `CueCommand`**: 7 variant・`#[non_exhaustive]` 無し。variant 追加の強制コンパイル点は**実測3箇所のみ**——`contract.rs:63 cue_target_of`（catch-all 無し・意図的）／`emo-text state.rs:166 apply_cue`（同）／`ghost sink.rs:56 command_kind`（ログラベル）。**wintf は完全不関与**（`CuePayload` 不透明転送・`dispatch.rs` は `CueCommand` variant を match しない・NewLine 増分時も無改変だったと実測確認）。
- **経路**: `cue_target_of` は sakura drive（`drive.rs:216`）が呼ぶ。Shell→`SurfaceSink`（=`SerikoSink`）・Balloon→`TextSink`（=`EmoTextSink`）・None→error! skip。
- **seriko**: `dispatch` の command match は catch-all あり（`actor.rs:205`）だが Shell 分類のみ到着。per-scope 状態（`ScopeStates`）・冪等ガード・単一発行点 `emit_display`・`DisplayCommand::Show{scope,surface_id,binds}/Hide{scope}` は確立済み。**構築モデルの正典（roadmap）は「seriko の構築入力＝surfaces.txt＋balloon descript の両方」**（シェル/バルーン統一エンジン）＝バルーン面状態は seriko の領分として設計済み。
- **emo-present（朗報）**: **バルーン多面切替は既に動く**。`build_balloon_target`（`balloon.rs:120-166`）が `balloons{N}.png` を全数 surface id=N でロード（`enumerate_frames`・`frame_id`）・`ShowSurface{surface_id:N}` で切替可（`world.surface(0)`/`surface(1)` 両在テストあり）。`balloonc*/balloonk*/arrow*/marker*/online*` は除外済み。`Hide` は mount/chain 非破壊（再 Show はキャッシュ復帰・テストあり）。
- **危険域（未定義）**: 表示中バルーン target への**異 id 再 Show の回帰テストが無い**。mount/chain は再利用・`chain.upload` は自動リサイズだが、**異寸切替では `TextSlotView.surface_size` が変わり既存 binding が stale**＋emo-text の `ActorRender`（swapchain/executor）は `register_actor_view` 再呼出でも**再構築されない**（`actor.rs:351` の contains_key ガード）＝stale 資源ハザード。同寸切替なら slot Entity 安定・数値も不変。
- **fixture**: emo2 は `balloons0.png` 1枚のみ・OnBoot script（`boot.pasta` 等）は `\b` 不使用。本 spec はテスト用 fixture（`balloons1.png` 追加の test-local バルーン）を自前で持つ。

## Desired Outcome

- `\b[ID]`（ブラケット形）＋旧形式 `\bN` が parse→compile→cue→seriko→表示指令発行まで**第一級で決定論的に流れる**: fixture script 直入力→mock 表示 sink 観測で「バルーン面切替指令（id 指定）／非表示指令（`-1`）」が観測できる（seriko-engine と同じ観測独立化・sleep 不使用・注入 Tick のみ）。
- 裸形 `\bN` の**本文数字漏れが根絶**される。
- emo-present に**バルーン target 異 id 再 Show の回帰檻**が立つ（同寸切替の表示等価・`TextSlotView` 安定性の固定）。
- 下流 emo2-boot が R5 を「実 cue が届く」前提へ書き換え可能になる（adapter がバルーン target へ配送するだけで面切替が成立する土台）。

## Approach

**A1: 統一 display 経路（`\s` と完全対称・採用）**

- **parser**: `Instruction::BalloonSurface(SurfaceArg)`（不透明転写・`\s` の `Surface(SurfaceArg)` と同流儀）＋ decode arm（ブラケット `\b[ID]`・裸形 `\bN` の両形）。`#[non_exhaustive]` ゆえ後方互換。
- **dola**: `CueCommand::BalloonSurface { .. }` 増分（sakura-engine が `NewLine` を増分した前例と同型・強制コンパイル点3箇所を同時更新）。
- **sakura**: compile 写像（`Instruction::BalloonSurface`→`CueCommand::BalloonSurface`・不透明転写）＋`cue_target_of` 分類。**分類先は design 冒頭確定**（本命: 表示系として `SurfaceSink`（=seriko）へ——シェル/バルーン統一原則・seriko 構築モデル正典と整合。`CueTarget` の意味論整理を含む）。
- **seriko**: per-scope バルーン面状態（既存 `ScopeStates` 流儀・冪等ガード）＋`DisplayCommand` のバルーン対象拡張＋`\b[-1]`→非表示。数値解決は素直な id（alias はシェル固有・バルーンに alias 正典は無い＝design で ukadoc 確認）。
- **emo-present**: additive 回帰テストのみ（バルーン target 異 id 再 Show・同寸 `TextSlotView` 安定性・crate 本体改変なし想定）。
- **正典確認**: ukadoc で `\b` の引数意味論（`-1` 非表示・旧形式・既定面 `balloon.defaultsurface`）を design 冒頭に確定（MCP 検索でタグ項目が引けなかったため get_doc/カテゴリ経由で精査）。

**棄却案**: A2=Balloon 分類のまま emo-text が presenter へ転送（✗ 文字状態機械が表示層を知る層違反）／A3=第3 sink（BalloonSink）新設（✗ `GhostBootOptions` 注入契約2本の改変＋ghost-setup 結線増＋seriko が持つ per-scope 状態・冪等ガードの再発明）。

## Scope

- **In**: parser `\b` 両形 decode（本文漏れ根絶含む）／dola `CueCommand` variant 増分（強制3箇所更新）／sakura compile 写像＋分類確定／seriko バルーン面状態＋`DisplayCommand` 拡張＋`-1` 非表示／emo-present additive 回帰（異 id 再 Show・TextSlotView 安定）／test fixture（多面バルーン）／ukadoc 正典意味論確定／決定論テスト網羅（全増分点）。
- **Out**: presenter への実配送結線（scope→TargetId 写像・UI 配送）＝**emo2-boot の adapter 責務**／異寸バルーン面切替時の文字層再装着ライフサイクル（design で境界裁定: M1=同寸前提＋異寸は warn ログ＋増分申し送り、を本命とする）／SERIKO バルーンアニメ／`\_b`（画像埋め込み・別タグ）／communicate 枠（`balloonc*`）・入力枠。

## Boundary Candidates

- **純粋 cue 化シーム**（parser＋dola＋sakura＝決定論・World/COM 非依存）と**表示状態消費シーム**（seriko＝actor・mock sink 観測）の2段。
- emo-present 検証は dev-test 増分のみ（本体非改変が原則・要 design 確認）。

## Out of Boundary

- emo2-boot の R5 改稿・adapter 配送・窓装着・`present_frame` 駆動（emo2-boot が再開時に所有）。
- バルーン文字層の異寸再装着改修（emo-text `ActorRender` 再構築）＝増分候補として申し送り。
- 二人立ちのバルーン target 割当本格化（M-dual）。

## Upstream / Downstream

- **Upstream**: areka-parsers（sakura parse ✅）・dola cue ✅・areka-sakura ✅・areka-seriko ✅・areka-emo-present ✅——全て完成済み・**additive 増分のみ**（前例: sakura-engine→dola `NewLine`／emo-text-layer→emo-present `TextSlotView` 増分・dola `Ord`）。
- **Downstream**: **`areka-P0-emo2-boot`（本 spec 完了がブロッカーゲート・再開時に R5 改稿＋adapter バルーン配送で消費）**。将来: M-dual（バルーン target 複数化）・choice-render（バルーン住人追加）が拡張形を消費。

## Existing Spec Touchpoints

- **Extends（additive 増分）**: `completed/areka-P0-sakura-parse`（Instruction 増分）・`completed/`dola cue-system（variant 増分）・`completed/areka-P0-sakura-engine`（compile/分類）・`completed/areka-P0-seriko-engine`（状態＋DisplayCommand）・`completed/areka-P0-emo-present`（回帰テストのみ）。
- **Adjacent（強制コンパイル点の機械的追随）**: `completed/areka-P0-emo-text-layer`（`state.rs apply_cue` に非消費 arm 1本・挙動不変）・`completed/areka-P0-ghost-setup`（`sink.rs command_kind` ログラベル1行）。
- **Blocked downstream**: `areka-P0-emo2-boot`（active・中断保留）。

## Constraints

- Rust 2024・tokio 禁止・新規外部依存なし・完成エンジンへの増分は additive（既存テスト全緑維持・`cargo test --workspace` exit 0）。
- 決定論テスト網羅必達（sleep 不使用・注入 Tick のみ・ログ発火/エラー写像も檻に入れる）。
- 正典は ukadoc（emo2 fixture は最小サンプルにすぎない・`\b` 意味論は ukadoc で確定してから実装）。
- `CueCommand` の catch-all 禁止規律を維持（強制コンパイル点は縮めない）。

## 検出ブロッカー登記（emo2-boot 要件定義/gap 由来・2026-07-11・処置先明記）

| # | 検出問題 | 処置先 |
|---|---|---|
| B1 | `\b` の cue ドメイン三重欠落（parser タグ表なし／compile 破棄／CueCommand variant 不在） | **本 spec** |
| B2 | 裸形 `\bN` の本文数字漏れ（可視破損） | **本 spec**（parser arm） |
| B3 | バルーン表示指令の配管不在（Balloon 分類→TextSink 誤配線） | **本 spec**（分類・経路設計） |
| B4 | emo-present バルーン target 異 id 再 Show の回帰欠落＋異寸 TextSlotView stale | **本 spec**（同寸回帰檻）＋異寸は増分申し送り |
| B5 | emo-text `ActorRender` 再登録時の stale 資源ハザード（異寸） | 境界裁定の上、増分申し送り（design で確定） |
| B6 | emo2-boot R5 の受入基準空振り（要件欠陥） | **emo2-boot 再開時に R5 改稿**（本 spec 完了が前提） |
| B7 | emo2-boot 要件ディスカッション残議題（依存解釈 R10.5・spine 観測境界 R8） | emo2-boot 再開時にディスカッション続行 |
| B8 | gap 設計判断 9件中 #6（`\b` 落ち先）は本 spec へ吸収・他8件は emo2-boot design 持ち | 変更なし（記録のみ） |
