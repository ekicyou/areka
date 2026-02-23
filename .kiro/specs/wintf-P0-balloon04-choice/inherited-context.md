# 親仕様からの引継ぎコンテキスト

> **親仕様**: `wintf-P0-balloon-system`
> **対象要件**: R10（選択肢バルーン）

---

## 参照すべき設計情報

### design.md

- **ChoiceBalloon コンポーネント定義**: 選択肢専用バルーンウィンドウ。**テキストバルーンとは独立した別ウィンドウ**として生成され、同キャラクターに紐付いて配置（`BalloonWindow` と同じ `anchor: Entity` パターン）
- **ChoiceItem コンポーネント定義**: 各選択肢アイテム（`item_id`, `text`, `is_hovered`, `is_focused`）
- **FocusIndex コンポーネント定義**: キーボードフォーカス管理。`current: usize` で現在フォーカス位置を追跡、上下キーで変更
- **ChoiceSelected イベント型**: 選択肢選択イベント（`Phase<ChoiceSelected>::Bubble` で配信）。`item_id: String`, `index: usize`
- **エンティティ階層**: ChoiceBalloon → ChoiceFrame → ChoiceContainer → ChoiceItem（LayoutRoot の直下に BalloonWindow と並列配置）
- **on_add パターン**: BalloonWindow パターンと同等（Window + Visual + 子エンティティ生成）
- **flexbox column レイアウト**: 選択肢を縦並び表示
- **キーボードナビゲーション**: `WM_KEYDOWN` → 上下キーで `FocusIndex.current` 変更 → Enter で `ChoiceSelected` 発火
- **モジュール配置**: `ecs/widget/balloon/choice.rs`

### research.md

- **設計決定への直接的な依存なし**: ChoiceBalloon は独立ウィンドウのため、グリフパイプライン（D3）やクリッピング（D6）の影響を受けない
- **注意**: `research.md` の G11（HitTestPoint API）はリンクのヒットテスト用であり、選択肢のキーボードナビゲーションとは無関係。キーボード要件は R10.5 由来

---

## 子仕様スコープ

- **選択肢専用バルーンウィンドウ（ChoiceBalloon）の生成・配置・管理**（BalloonContentArea 内配置ではない）
- flexbox column レイアウトによる選択肢テキスト縦並び表示
- ホバー・フォーカス状態のビジュアルフィードバック
- マウスクリック / キーボード（上下キー・Enter）での選択操作
- 選択結果を `ChoiceSelected` イベントとしてスクリプトエンジンへ配信（イベントシステム経由）
