# Requirements Document

## Project Description (Input)
arekaクレートのモック実装立ち上げ。ぱすたさんっぽいもの。
とりあえず、wintfを使って、ウィンドウドラック可能な透過ウィンドウ（シェル本体と縦書きバルーンっぽいウィンドウ）が立ち上がればよい。

## Introduction
arekaクレートのモック実装として、wintfフレームワークを使用した「ぱすたさん」風デスクトップマスコットのプロトタイプを立ち上げる。シェル本体（キャラクター画像表示）とバルーン（縦書き吹き出し）の2つの透過ウィンドウを表示し、マウスドラッグによるウィンドウ移動が可能な最小限の動作デモを実現する。正式なarekaバイナリクレート（Phase D）の前段階として、`examples/`配下のサンプルとして実装する。

## Requirements

### Requirement 1: シェルウィンドウ（キャラクター表示）
**Objective:** ユーザーとして、デスクトップ上にぱすたさんの立ち絵が透過ウィンドウで表示されてほしい。デスクトップマスコットらしい見た目の最小実装。

#### Acceptance Criteria
1. When arekaモックが起動した時, the Shell Window shall タイトルバー・枠線なしの透過ウィンドウとして画面上に表示する（`WS_POPUP`スタイル）
2. The Shell Window shall キャラクター画像（320×420px）を`BitmapSource`コンポーネントで表示する
3. The Shell Window shall 画像の透明部分（アルファ値0の領域）をクリックスルーとして扱い、背後のデスクトップやウィンドウを操作可能にする
4. When シェル画像アセットが見つからない場合, the Shell Window shall プレースホルダー矩形（半透明の塗りつぶし）を代替表示する

### Requirement 2: バルーンウィンドウ（縦書き吹き出し）
**Objective:** ユーザーとして、シェルの横にバルーン（縦書きテキスト表示領域）が表示されてほしい。タイプライター機能との将来的な統合を見据えた枠組み。

#### Acceptance Criteria
1. When arekaモックが起動した時, the Balloon Window shall シェルウィンドウの横に吹き出し風の透過ウィンドウとして表示する
2. The Balloon Window shall タイトルバー・枠線なしのポップアップウィンドウとする（シェルウィンドウと同様のスタイル）
3. The Balloon Window shall 薄い背景色を持つ矩形領域（`Rectangle` + `Brushes`）をバルーン本体として描画する
4. The Balloon Window shall `Typewriter`コンポーネントを使用し、縦書き(`TextDirection::VerticalRightToLeft`)でサンプルテキストを表示する
5. The Balloon Window shall シェルウィンドウの右側に、適切な間隔（約10〜20px）を空けて配置する

### Requirement 3: ウィンドウドラッグ移動
**Objective:** ユーザーとして、シェルウィンドウをマウスドラッグでデスクトップ上の任意の位置に移動できるようにしたい。

#### Acceptance Criteria
1. When シェルウィンドウの不透明部分を左クリック＆ドラッグした時, the Shell Window shall マウスに追従してウィンドウ位置を移動する
2. When シェルウィンドウがドラッグ移動された時, the Balloon Window shall シェルウィンドウとの相対位置を維持して追従移動する

### Requirement 4: 非同期デモフロー
**Objective:** 開発者として、モック実装の動作を自動で確認できるデモフローを持たせたい。

#### Acceptance Criteria
1. When モックが起動した時, the Demo shall `CommandSender`を使った非同期タスクでウィンドウ生成を実行する
2. The Demo shall コンソールに操作ガイド（ドラッグ操作の説明等）を出力する
3. The Demo shall 起動後60秒で自動的にウィンドウを閉じ、アプリケーションを終了する
4. The Demo shall `tracing-subscriber`で`RUST_LOG`環境変数によるログレベル制御に対応する

### Requirement 5: プロジェクト構成
**Objective:** 開発者として、既存のwintfワークスペース構成に沿った形でモック実装を配置したい。

#### Acceptance Criteria
1. The Mock Shell shall `crates/wintf/examples/areka_mock_shell.rs`として実装する（既存の`areka.rs`ダミーとは別ファイル）
2. The Mock Shell shall `wintf`クレートの公開APIのみを使用し、内部モジュールに直接依存しない
3. The Mock Shell shall `human_panic`、`tracing`、`tracing-subscriber`、`async-io`など既存サンプルと同じ依存クレートを利用する
4. Where シェル画像アセットが必要な場合, the Mock Shell shall `shell/`ディレクトリ配下の既存アセット、または`tests/assets/`の画像を参照する
