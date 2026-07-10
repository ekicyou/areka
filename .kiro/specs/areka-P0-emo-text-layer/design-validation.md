# 設計バリデーションレポート: areka-P0-emo-text-layer

> **実施日**: 2026-07-10 ／ **対象**: design.md（確定版・847行）／ requirements.md（R1〜R11）／ research.md ／ steering
> **検証方法**: design-review.md プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）＋設計中の実コード主張の実地照合（mount.rs / sink.rs / contract.rs / ui.rs / parse.rs / wintf text 資産 / fixture / emo-present example）

## Review Summary

設計は要件 R1〜R11 の全 AC をトレーサビリティ表で 1:1 に写像し、要件ディスカッションの拘束裁定 5 点（①描画は emo 所有・wintf は donor（lift）に限定 ②新 crate `areka-emo-text` ③マルチアクター ActorKey ルーティング ④後出し優先＋ガードは上流 kanade ⑤DPI を最初から一級市民）を**すべて遵守**している。設計中の実コード主張（`mount.rs:224` の pub(crate) `text_slot`・`sink.rs:23` の infallible `TextSink`・`UiSender` の Clone+Send 非ブロック契約・wintf 縦書きレシピ・fixture の `wordwrappoint.y,0` 退化・排他 system による毎フレーム駆動の先例）を全数照合し、**虚偽・陳腐化はゼロ**だった。残る懸念は中核レイアウト機構の測定順序と DPI 契約の供給源という精緻化レベルの 2 点であり、アーキテクチャ破壊級の欠陥は無い。

## Critical Issues（2件・アーキテクチャ破壊級なし）

### 🔴 Critical Issue 1: GlyphMetrics の測定順序（probe layout）と描画整合の invariant が未確定

- **Concern**: LayoutEngine の折返し決定は `GlyphMetrics::advance(ch, font_height)` の事前注入を要するが、実行時実装 `DWriteMetrics` の典拠は「生成した TextLayout の cluster metrics」（draw.rs 節）とされる。行 TextLayout は折返し決定の**後**にしか存在せず（鶏卵）、測定用レイアウト（probe layout）の生成規約が書かれていない。また per-char・文脈非依存の advance は、DirectWrite が行 TextLayout 描画時に行う shaping（カーニング・プロポーショナル幅）と乖離しうる。
- **Impact**: 乖離が出ると「LayoutEngine が決めた折返し位置」と「実際に描かれた行幅」がずれ、wordwrappoint 超過・validrect あふれの視覚欠陥になる。fixture（日本語・等幅系）では実質顕在化しないが、欧文プロポーショナルで顕在化する。R4.5 の分離線（アルゴリズム分岐なし）の成立条件そのもの。
- **Suggestion**: tasks 生成時に次のどちらかを確定する——(a) `DWriteMetrics` の典拠を「未折返しの測定専用 TextLayout（probe）」と規約化し、描画行の cluster advance と一致する invariant（同一 format・同一テキストなら advance 同値）をテストで檻化する。(b) M1 は CJK 等幅前提を明文化し、描画時の実行幅超過は validrect クリップで吸収する縮退規約を書く。いずれも design の構造は不変（追記で足りる）。
- **Traceability**: R4.5・R6.1-6.3・R7.5
- **Evidence**: design.md「LayoutEngine + GlyphMetrics（layout.rs）」「DrawExecutor（draw.rs）」の DWriteMetrics 記述

### 🔴 Critical Issue 2: image px 原寸の供給源が未指定（k≠1 時の換算規約）＋実行時 k≠1 経路の検証空白

- **Concern**: `TextRegion::resolve(model, image_size, mode)` は**画像座標空間**の原寸を要する（負値=反対辺基準の解決に必須）が、供給源とされる `TextSlotView.surface_size` は**物理 px 原寸**と定義されている。現行契約（k=1.0 恒常）では同値ゆえ動くが、k≠1 導入時に `/k` 換算が必要になる旨がどこにも書かれておらず、論理/物理混在事故（記憶 areka-window-placement-dpi-coordinate-defect）の再発芽になりうる。また実行時 k≠1 は上流未実装ゆえ純粋テスト（k=1.25/2.0）でしか検証されない。
- **Impact**: 「座標空間は 2 つだけ」という本設計の最重要不変条件の唯一の綻び目。将来の DPI スケーリング導入（Revalidation Trigger 記載済み）時に静かに壊れる。
- **Suggestion**: tasks で (a) `TextSlotBinding` 構築時に `image_size = surface_size / k`（ceil/floor 規約込み）を一点定義し region.rs の単体テスト（k=1.25/2.0）に含める、(b) 実行時 k≠1 の検証空白は「上流が k≠1 を供給し次第 example 再実行」を DoD 申し送りとして明文化する。設計の 2 空間モデル自体は正しく、追記で閉じる。
- **Traceability**: R4.3・R4.6・R10.4・R11.9
- **Evidence**: design.md「TextRegion / ScaleContract（region.rs）」「TextSlotBinding」（surface_size: 物理原寸）・「DPI/スケール契約（一枚定義）」

## Design Strengths

1. **DPI 2 空間契約の構造化が模範的**: 「画像座標空間と物理座標空間の 2 つだけ・論理 px を存在させない・k の適用は SetTransform 一点」という一枚定義は、placement DPI 欠陥の教訓を型と不変条件へ直接昇華しており、拘束裁定⑤（DPI 一級市民）の最良の実現形。軸読み替え正準表・DPI 契約表・descript キー 3 分類表という「本書が正典」宣言群も、SSP de-facto 不在領域の裁量を漏れなく文書化している。
2. **決定論檻の徹底と実コード接地**: 純粋層（state/writing/region/layout/canvas）の windows 非依存分離＋GlyphMetrics 注入＋readback ピクセル述語（AA 依存の golden 回避）は決定論テスト網羅方針と完全整合。設計中の実シンボル主張を全数照合して虚偽ゼロ——差し込み先（pub(crate) text_slot・donor パターン・UiSender 契約・排他 system 駆動先例）がすべて実在し、実装リスクが大幅に低い。

## 拘束裁定の遵守確認（要件ディスカッション 2026-07-09/10）

| 裁定 | 遵守 | 証拠 |
|---|---|---|
| ① 描画は emo 所有（wintf は donor・実行時依存にしない） | ✅ | Allowed Dependencies（wintf テキスト widget 非依存の明記・レシピ lift）・DrawExecutor |
| ② 新 crate `areka-emo-text` | ✅ | 冒頭マッピング注記・File Structure Plan |
| ③ マルチアクター ActorKey ルーティング | ✅ | `ActorKey → ActorTextState`／`ActorKey → TextSlotBinding`・actor→target は結線側所有（emo-present は actor を知らない） |
| ④ 後出し優先・ガードは上流 kanade | ✅ | Non-Goals・R3.6/R10.5 トレース・Clear の未リビール破棄・ガード型の不在 |
| ⑤ DPI 一級市民 | ✅ | ScaleContract・2 空間正準・k 一点適用・k≠1 純粋テスト＋実 DPI 手順（Issue 2 の供給源追記のみ要） |

## Final Assessment

- **Decision**: **GO**
- **Rationale**: 拘束裁定 5 点を完全遵守し、全 AC のトレーサビリティと実コード接地が確認できた。残 2 件は中核機構の精緻化（測定順序の規約化・換算一点の明文化）であり、tasks 生成時の追記で閉じる規模——アーキテクチャの再設計を要しない。
- **Next Steps**:
  1. 設計ディスカッションで Issue 1（probe layout 規約 vs CJK 前提＋クリップ縮退）と Issue 2（image_size 換算の一点定義）の裁定を取る
  2. 裁定を design.md へ追記（または tasks の実装規約として確定）
  3. `/kiro-spec-tasks areka-P0-emo-text-layer` へ進む
