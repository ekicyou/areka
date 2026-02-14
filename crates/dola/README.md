# dola — Declarative Orchestration for Live Animation

> プラグイン間で共有可能な、シリアライズ可能アニメーション宣言フォーマット

---

## 概要

**dola** は、Windows Animation Manager の概念（変数・トランジション・キーフレーム・ストーリーボード）をプラットフォーム非依存のデータモデルとして再構成したクレートです。アニメーション定義を JSON / TOML / YAML で記述し、実行時に解釈・再生する宣言的アプローチを採用しています。

---

## 対応フォーマット

| フォーマット | feature flag | デフォルト |
|-------------|-------------|:---------:|
| JSON | `json` | ✅ |
| TOML | `toml` | — |
| YAML | `yaml` | — |

```toml
# Cargo.toml での指定例
[dependencies]
dola = { path = "../dola" }                           # JSON のみ（デフォルト）
dola = { path = "../dola", features = ["toml"] }      # JSON + TOML
dola = { path = "../dola", features = ["json", "toml", "yaml"] }  # 全フォーマット
```

---

## 基本的な使用例

### Storyboard 定義（JSON）

```json
{
  "variables": [
    { "name": "opacity", "initial_value": 0.0 }
  ],
  "transitions": [
    {
      "name": "fade_in",
      "variable": "opacity",
      "value": { "final": 1.0 },
      "duration_ms": 500,
      "easing": { "name": "ease_in_out" }
    }
  ],
  "storyboards": [
    {
      "name": "appear",
      "entries": [
        { "transition": "fade_in" }
      ]
    }
  ]
}
```

### Builder API

```rust
use dola::{DolaDocumentBuilder, StoryboardBuilder};

let doc = DolaDocumentBuilder::new()
    .variable("opacity", 0.0)
    .transition("fade_in")
        .variable("opacity")
        .final_value(1.0)
        .duration_ms(500)
        .ease_in_out()
        .done()
    .storyboard("appear")
        .entry("fade_in")
        .done()
    .build()?;
```

---

## API 概要

### 主要な型

| 型 | 説明 |
|----|------|
| `DolaDocument` | アニメーション定義のルートドキュメント |
| `DolaDocumentBuilder` | ドキュメントの Builder |
| `AnimationVariableDef` | アニメーション変数の定義（名前・初期値・範囲） |
| `TransitionDef` | トランジション定義（対象変数・目標値・時間・イージング） |
| `TransitionValue` | トランジションの値指定（final / delta / velocity） |
| `Storyboard` | トランジションの実行順序を定義するコンテナ |
| `StoryboardEntry` | ストーリーボード内の個別エントリ |
| `EasingFunction` | イージング関数（名前付き / パラメトリック） |
| `EasingName` | プリセットイージング名（ease_in, ease_out, ease_in_out 等） |
| `PlaybackState` | 再生状態（idle / playing / paused / completed） |
| `DynamicValue` | 実行時の動的値（f64 / bool / string） |
| `DolaError` | エラー型 |

### バリデーション

`Validate` トレイトにより、ドキュメントの整合性チェックが可能です：

```rust
use dola::Validate;

let doc = dola::DolaDocument::from_json(json_str)?;
doc.validate()?;  // 変数参照の整合性、トランジション定義の妥当性を検証
```

---

## feature flags

| flag | 依存クレート | 説明 |
|------|------------|------|
| `json` (default) | `serde_json` | JSON シリアライズ/デシリアライズ |
| `toml` | `toml` | TOML シリアライズ/デシリアライズ |
| `yaml` | `serde_yaml` | YAML シリアライズ/デシリアライズ |

全フォーマット共通で `serde` による derive マクロを使用しています。
