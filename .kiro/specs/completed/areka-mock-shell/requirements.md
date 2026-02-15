# Requirements Document

## Project Description (Input)
arekaクレートのモック実装立ち上げ。ぱすたさんっぽいもの。
とりあえず、wintfを使って、ウィンドウドラック可能な透過ウィンドウ（シェル本体と縦書きバルーンっぽいウィンドウ）が立ち上がればよい。

## Introduction
`crates/areka/` バイナリクレートを新規に作成し、wintfフレームワークを使用した「ぱすたさん」風デスクトップマスコットの最小動作実装を立ち上げる。シェル本体（キャラクター画像表示）とバルーン（縦書き吹き出し）の2つの透過ウィンドウを表示し、マウスドラッグによるウィンドウ移動が可能な状態を実現する。crates.ioへの公開予約を兼ね、公開可能な状態のクレート構成とすることがゴールである。

## Requirements

### Requirement 1: シェルウィンドウ（キャラクター表示）
**Objective:** ユーザーとして、デスクトップ上にぱすたさんの立ち絵が透過ウィンドウで表示されてほしい。デスクトップマスコットらしい見た目の最小実装。

#### Acceptance Criteria
1. When arekaモックが起動した時, the Shell Window shall タイトルバー・枠線なしの透過ウィンドウとして画面上に表示する（`WS_POPUP`スタイル）
2. The Shell Window shall キャラクター画像（320×420px）を`BitmapSource`コンポーネントで表示する
3. Where `wintf-P0-click-through`が実装済みの場合, the Shell Window shall 画像の透明部分（アルファ値0の領域）をクリックスルーとして扱い、背後のデスクトップやウィンドウを操作可能にする（※本仕様のスコープ外。wintfの`HTTRANSPARENT`有効化に依存）

### Requirement 2: バルーンウィンドウ（縦書き吹き出し）
**Objective:** ユーザーとして、シェルの横にバルーン（縦書きテキスト表示領域）が表示されてほしい。タイプライター機能との将来的な統合を見据えた枠組み。

#### Acceptance Criteria
1. When arekaモックが起動した時, the Balloon Window shall シェルウィンドウの横に吹き出し風の透過ウィンドウとして表示する
2. The Balloon Window shall タイトルバー・枠線なしのポップアップウィンドウとする（シェルウィンドウと同様のスタイル）
3. The Balloon Window shall 薄い背景色を持つ矩形領域（`Rectangle` + `Brushes`）をバルーン本体として描画する
4. The Balloon Window shall `Typewriter`コンポーネントを使用し、縦書き(`TextDirection::VerticalRightToLeft`)で以下のテキストを表示する:
   > みんながもってる、記憶の糸。
   > 
   > 生まれてから、続いている、
   > 長い長い、一本の道。
   > 
   > そう、きっと、一本道。
   > いつか来る、終わりの日まで。
   > 
   > ぱすた
5. The Balloon Window shall シェルウィンドウの右側に、適切な間隔（約10〜20px）を空けて配置する

### Requirement 3: ウィンドウ操作（ドラッグ移動・終了）
**Objective:** ユーザーとして、シェルウィンドウをマウスドラッグで移動でき、ダブルクリックでアプリケーションを終了できるようにしたい。タスクバー非表示のため、シェル自体が終了手段を提供する。

#### Acceptance Criteria
1. When シェルウィンドウの不透明部分を左クリック＆ドラッグした時, the Shell Window shall マウスに追従してウィンドウ位置を移動する
2. When シェルウィンドウがドラッグ移動された時, the Balloon Window shall シェルウィンドウとの相対位置を維持して追従移動する
3. When シェルウィンドウをダブルクリックした時, the Shell Window shall 全ウィンドウを閉じてアプリケーションを終了する

### Requirement 4: 起動・ログ
**Objective:** 開発者として、モック実装の起動フローとデバッグ手段を整えたい。

#### Acceptance Criteria
1. When モックが起動した時, the Mock Shell shall `CommandSender`を使った非同期タスクでウィンドウ生成を実行する
2. The Mock Shell shall コンソールに操作ガイド（ドラッグ移動・ダブルクリック終了の説明）を出力する
3. The Mock Shell shall `tracing-subscriber`で`RUST_LOG`環境変数によるログレベル制御に対応する

### Requirement 5: クレート構成
**Objective:** 開発者として、`crates/areka/`をcrates.io公開可能なバイナリクレートとして正しく構成したい。

#### Acceptance Criteria
1. The areka crate shall `crates/areka/Cargo.toml`を持つバイナリクレートとし、ワークスペース（`members = ["crates/*"]`）の自動検出で認識されること
2. The areka crate shall `crates/areka/Cargo.toml`にて`publish = true`（ワークスペースデフォルトの`publish = false`を上書き）とし、crates.io公開に必要なメタデータ（`name`, `version`, `description`, `license`, `edition`）を設定する
3. The areka crate shall `wintf`クレートの公開APIのみを使用し、内部モジュールに直接依存しない
4. The areka crate shall `human_panic`、`tracing`、`tracing-subscriber`など既存クレートと同じワークスペース依存を利用する
5. The areka crate shall シェルアセットを`crates/areka/shell/`に配置する（既存の`shell/`ディレクトリから`git mv`で移動）
6. When areka クレート作成後, the workspace shall 既存のダミー`crates/wintf/examples/areka.rs`を削除し、`structure.md`のarekaクレートのStatusを更新する
