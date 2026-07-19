# Brief: areka-P0-budoux-newline

> budoux 分かち書き境界での自動折り返しモード——balloon descript `budoux_newline` で opt-in する areka 拡張

## Problem

emo-text の自動折り返し（`layout.rs:191`・`wrap_threshold` 超過で行を閉じる）は**文字単位**であり、語や文節の途中でぶつ切りになる。emo2 実機バルーンは 1 行が狭く（sakura 側 balloons0.png 400px・閾値 366px・font 28px ≈ **全角 13 文字**／kero 側 balloonk0.png 288px ≈ **全角 9 文字**・実測 2026-07-18）、ぶつ切りが頻発して可読性を損なう。

## Current State

- **自動折り返しは実装済み**（overflow ではない）: `crates/areka-emo-text/src/layout.rs` の `LayoutEngine::layout()` が `inline_pos + advance > threshold` で行を閉じる（`layout.rs:191`）。行頭 1 グリフは閾値超過でも配置（無限折返し回避）。折返し軸は 3 方向共通の軸読み替え正準表で単一式。
- 改行は 2 系統: (a) 閾値超過の自動折返し（行送り = `line_pitch`）、(b) `TextItem::LineBreak { ratio }`（NewLine cue 由来・`layout.rs:211-224`）。
- 閾値の源は `TextRegion.wrap_threshold`（`region.rs:180-190`）＝ balloon descript `wordwrappoint`（負値=反対辺基準）、無指定時は validrect 遠辺へ縮退。
- 折返し判定は既に **DirectWrite 実グリフ advance**（`GlyphMetrics` 注入・テストは `FixedMetrics`）で行われる＝固定概算ではない。
- **areka 拡張キーの前例 = `writing_mode`**: parsers は生文字列転記（`parse.rs:96`→`BalloonModel.writing_mode`）・emo-text 側で語彙解決＋未知値 warn+フォールバック（`writing.rs:63-77`）。未知キーは完全一致引きで自然無視＝`budoux_newline` 追加は既存ゴーストに無害。
- **budouy crate は workspace 未依存**（新規依存）。pasta 上流（ekicyou/pasta）は `budouy 0.2.2`（features=`["vendored-models"]`）を pasta_lua 依存として使用中。crates.io 実在確認済み（2026-07-18）: `budouy` 0.2.2・Apache-2.0・repo=neodyland/budouy・2026-04 更新で現役。旧 port `budoux` 0.1.1（2022 停止）とは**別 crate**——採用は pasta と同じ **`budouy`** 一択。
- reveal（typewriter）は時刻ゲートのみ: トークの `TextItem` 列は state に**全文先着**し `RevealSchedule` が可視 prefix を進める＝レイアウトは全文 lookahead 可能（後述の再流し込み回避の根拠）。

## Desired Outcome

- balloon descript に `budoux_newline,1`（または `true`）と書くと、自動折り返しの**分割点が budoux 分かち書き境界（文節っぽい塊の境界）に揃う**。塊は行末で途中分割されず、丸ごと次行へ送られる（ワードラップ）。
- **既定 OFF**（キー無し・`0`/`false`＝現行の文字単位折返しのまま）。未知値は warn + OFF（`writing_mode` と同じ縮退姿勢）。
- **長大セグメント縮退（開発者裁定 2026-07-18・(a) 案）**: 塊が行頭からでも 1 行に収まらない場合のみ、**その塊に限って従来の文字単位折返しへ縮退**。はみ出し・無限ループなし。
- 明示改行（`\n` 系 LineBreak）の意味論は不変。縦書き（`vertical_rl`/`vertical_lr`）でも軸読み替えの上で同一規則。
- typewriter 中に配置済みグリフが後から行を移る「リフロー跳び」を起こさない（塊の先頭配置時に全文 lookahead で行送りを先決）。

## Approach

**emo-text 内完結**（開発者選択 2026-07-18）。折り返しはバルーン幅依存＝純粋な描画関心であり、`writing_mode` の確立済みの型に載せる:

1. **parsers**: `budoux_newline` を生文字列転記（`BalloonModel` へ Option<String> 追加・検証なし・転記層の規律通り）。
2. **emo-text 語彙解決**: `ResolvedBalloonText::resolve` 系で `1`/`true`→ON・`0`/`false`/欠落→OFF・未知値 warn+OFF。
3. **純粋層セグメンテーション**: budouy（vendored-models）でトーク全文（改行/Clear で区切られたテキストラン単位・actor 別）をセグメント境界列へ変換する純関数。境界計算は決定論＝GPU 不要で全網羅テスト可。
4. **layout 折返し判定の変更**: ON 時、塊の先頭グリフを置く時点で「塊全体が残り行幅に収まるか」を実 advance 合計で判定し、収まらなければ塊の前で行を送る（先決＝リフロー跳びなし）。行頭からでも収まらない塊はその塊のみ文字単位縮退。OFF 時は現行コードパス不変。
5. budouy 依存は `areka-emo-text` のみに追加（Apache-2.0・新規依存は開発者要望により承認済み）。

## Scope

- **In**:
  - parsers への `budoux_newline` キー転記（balloon descript 基層＋画像別上書き層の後勝ちマージに乗る）
  - emo-text の語彙解決・budouy セグメンテーション純関数・layout 折返し判定の budoux モード
  - 長大セグメントの文字単位縮退
  - 縦書き両対応（軸読み替え正準表上で同一規則）
  - reveal（typewriter）との整合＝塊先頭配置時の先決 lookahead
  - 決定論テスト檻（境界計算・折返し判定・縮退・OFF 不変の全網羅／`FixedMetrics` 注入）
  - 実機確認（AREKA_APP_SMOKE_EXIT_MS 有界 auto-exit＋出力画像の AI vision 目視——emo-text byte 等価の盲点対策）
- **Out**:
  - sakura compile／cue 語彙の改変（境界ヒント cue 案は層違反として棄却済み——折返し可否はバルーン幅依存＝emo の関心）
  - pasta／fixture の改変（fixture balloon descript へのキー追記は実機確認用の最小編集のみ可）
  - 禁則処理（行頭句読点回避等）の独自実装——budoux 境界が実用上吸収する分のみ・専用禁則エンジンは作らない
  - `wordwrappoint`／`validrect` の解決規則変更（閾値の源は不変・分割点の選び方だけ変える）
  - 明示改行（`\n` 系）・スクロール／あふれ機構の意味論変更

## Boundary Candidates

- **セグメント境界の置き場**: 純粋モジュール新設（layout 隣接）が理想。**`state.rs` は非改変が理想**（W3 choice-render の宛先ファイルのため・干渉回避）。`TextItem` 列→境界列の写像を state の外で計算できるかは design で確定。
- **budouy 呼び出し粒度**: テキストラン単位（改行/Clear 区切り・actor 別）。グリフ列と文字列の対応（サロゲート/結合文字）は既存 glyph 化経路の単位に従う。
- **OFF 経路の不変保証**: 既存の文字単位折返しテスト檻が OFF 時の非回帰檻を兼ねる構造にする。

## Out of Boundary

- newline-defer が扱う「改行マーカーの遅延実体化」意味論（本 spec は自動折返しの分割点選択のみ）。
- SSP 互換性の主張はしない——SSP に本機能は無い**純 areka 拡張**（`writing_mode` と同格・snake_case・prefix 無しの拡張キー規約）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo-text-layer`（折返し・レイアウト基盤）／`completed/areka-P0-emo-text-viewbox`（スクロール——非改変で乗る）／**`areka-P0-newline-defer`（先行必須・下記 Constraints）**／crate `budouy` 0.2.2（新規依存）
- **Downstream**: `areka-P0-emo2-conformance-e2e`（W5・適合走行）／将来の M2 テキスト装飾（分割点情報は装飾単位としても再利用余地）

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-emo-text-layer`（折返し判定の分割点選択を拡張・OFF 時不変）
- **Adjacent**: `areka-P0-newline-defer`（**同 `layout.rs` 直接衝突**——LineBreak 分岐 211-224 と折返し分岐 191 は隣接・テスト檻も共有）／`areka-P0-choice-render`（W3・同 crate `state.rs` 宛先——本 spec は state.rs 非改変が理想）／`areka-P0-collision-geometry`（emo-compose/present 側・交差なし）

## Constraints

- **ウェーブ編成（少しでも干渉するならウェーブを分ける・roadmap 追記㉙㉚）**: **`areka-P0-newline-defer` の完了後に直列で着手**（両者とも `layout.rs` の折返し/LineBreak 分岐とテスト檻を直接編集＝同時進行は確実に衝突）。W3 `choice-render` とは state.rs 非改変を保てばファイル素——design で配置確定まで W3 併走を仮定しない。
- 新規依存 `budouy 0.2.2`（features=`["vendored-models"]`・Apache-2.0）は開発者指名により承認。ネットワーク不要のモデル同梱形を使う（決定論・オフライン CI 整合）。
- 決定論テスト網羅必達・実装第一（テストのため実装を歪めない）・ログ無し失敗経路の禁止。
- 実機サインオフは有界 auto-exit＋ログ grep の定石（AREKA_APP_SMOKE_EXIT_MS=180000・RUST_LOG=info,kanade=trace）＋出力画像 AI vision 目視。実機確認は fixture balloon descript へ `budoux_newline,1` を追記して行う。
