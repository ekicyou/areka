---
inclusion: fileMatch
fileMatchPattern: '**/*.rs'
updated_at: 2026-03-07
---

# Logging Guidelines (tracing)

このプロジェクトでは `tracing` クレートを使用した構造化ロギングを採用しています。

## 依存関係

```toml
# ライブラリ（wintf, dola）
tracing = { workspace = true }

# アプリケーション（areka, examples）
tracing-subscriber = { workspace = true }  # env-filter feature有効
```

## ログレベル選択基準

| レベル | 用途 | 例 |
| -------- | ------ | ----- |
| `error!` | 致命的エラー、回復不能な失敗 | COM API失敗、リソース作成失敗 |
| `warn!` | 回復可能なエラー、警告、非推奨の使用 | 無効なパラメーター、フォールバック発生 |
| `info!` | ライフサイクルイベント | 初期化完了、終了、ディスプレイ構成変更 |
| `debug!` | 開発者向け詳細情報 | エンティティ作成、コマンド実行、状態変更 |
| `trace!` | 高頻度イベント、詳細トレース | WMメッセージ、フレームごとの処理、描画詳細 |

## 構造化フィールドの規約

よく使用するフィールド名を統一：

```rust
// Entity識別子
debug!(entity = %entity_name, "message");
debug!(entity = ?entity, "message");  // Entity IDのDebug出力

// ウィンドウハンドル（16進数）
trace!(hwnd = format!("0x{:X}", hwnd.0), "message");

// フレーム番号
trace!(frame = frame_count.0, "message");

// エラー詳細
error!(error = ?e, "operation failed");
error!(error = %e, hresult = format!("0x{:08X}", e.code().0), "COM error");

// サイズ・座標
debug!(width = width, height = height, "size");
debug!(x = pos.x, y = pos.y, "position");
```

## 書式パターン

### スコーププレフィックス

ログメッセージには、コンポーネント名または関数名ベースのスコープ文字列を含める：

```rust
info!("[GraphicsCore] Initialization completed");
debug!(entity = %name, "[init_window_graphics] WindowGraphics created");
info!("[ClipDemo] Creating ULW + DComp clip windows");
trace!(frame = frame_count.0, "[commit_composition] DComp device not available");
```

- フレームワーク内部は関数名ベースのプレフィックスを優先する
- サンプルやアプリケーション層はコンポーネント名ベースのプレフィックスでもよい
- 同一モジュール内で両形式を混在させるより、ファイル単位で一貫させる

### 構造化フィールド優先

文字列補間より構造化フィールドを優先：

```rust
// Good: 構造化フィールド
debug!(
    entity = %entity_name,
    width = width,
    height = height,
    "[deferred_surface_creation] Creating Surface"
);

// Avoid: 文字列補間
debug!("[deferred_surface_creation] Creating Surface for Entity={}, size={}x{}", entity_name, width, height);
```

## Subscriber初期化（アプリケーション側）

```rust
use tracing_subscriber::EnvFilter;

fn main() {
    // RUST_LOG環境変数対応、未設定時はinfoレベル
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();
}
```

## 環境変数によるフィルタリング

```powershell
# infoレベル以上
$env:RUST_LOG="info"; cargo run -p areka

# debugレベルも表示
$env:RUST_LOG="debug"; cargo run -p wintf --example clip_demo

# wintfクレートのみtrace
$env:RUST_LOG="wintf=trace"; cargo run -p wintf --example taffy_flex_demo_old

# 特定モジュールのみ
$env:RUST_LOG="wintf::ecs::graphics=debug"; cargo run -p wintf --example multi_backend_demo
```

## ライブラリ vs アプリケーション

- **ライブラリ（wintf, dola）**: `tracing`マクロでログを発行するのみ。Subscriber初期化は行わない。
- **アプリケーション（areka, examples）**: `tracing-subscriber`を使用してSubscriberを初期化し、`RUST_LOG` で出力を制御する。

これにより、ライブラリ使用時にSubscriber未設定であればログ出力はゼロコストとなる。

Document logging patterns, not every call site.
