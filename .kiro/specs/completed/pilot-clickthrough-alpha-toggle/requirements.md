# Requirements Document

## Introduction

本仕様は先進坑（pilot・使い捨て）である。目的は、Windows デスクトップマスコットの中核要件である「キャラクター描画領域だけクリックを受け、透明領域は背面アプリ（別プロセス）へクリックを透過させる」を、`WS_EX_TRANSPARENT` をαマスクに応じて動的に付け外しする方式で成立させられるかを使い捨ての実証で先に潰すことにある。既存の `areka`/`wintf` はこれを ULW（UpdateLayeredWindow）の alpha-0 自動透過で実現済みだが、ULW は CPU ビットマップ方式で DComp スワップチェーン合成と併用不可であり、別プロセス透過のために DComp（GPU 合成）描画を諦める踏み絵になっている。DComp 描画を捨てないまま別プロセス透過を成立させる第 4 の手（`WS_EX_TRANSPARENT` 動的トグル）の実現可能性を、独立 example として最小実装し、試験項目 T1〜T8 を人間とともに手動検証し、開発者が go／違う／直す を判定できる状態にする。

本仕様の成果物はコードではなく知見（go／違う／直す ＋ 学び）であり、一次記録は example の `README.md`（3 幕構成）と `REPORT.md`（指定フォーマット）である。先進坑コードは production に被依存しない葉ノード（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/`）に隔離し、いつでも安全に捨てられる状態を保つ。

## Boundary Context

- **In scope**:
  - 透過トップモスト窓＋中央の不透明領域を持つ最小 PoC の実装（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/`、`_template` をコピーして着手）。
  - 別スレッドが 16ms 周期でカーソル位置を取得し、仮のαマスク関数（ウィンドウ中央を中心とする半径 200px の円判定。描画される不透明円と同一領域・実スクリーン物理座標で判定）に問い合わせ、円内＝クリックスルー OFF／円外＝クリックスルー ON を動的に切り替える挙動。
  - 状態変化したフレームでのみ拡張スタイルを適用する状態変化最適化。
  - 試験項目 T1〜T8 の人間との手動検証。
  - `REPORT.md`（指定フォーマット）と README 3 幕の作成。
- **Out of scope**:
  - 本体 `wintf`/`areka` への接続（本坑 `wintf-clickthrough-alpha-toggle` の領分）。
  - 実描画αバッファ参照（PoC は仮の円判定でよい）。
  - ULW/DComp バックエンドの撤去・改変。
  - 新しい大型クレート（winit/tauri 等）の追加。
  - 先進坑コードの production 流用（コピペ donor 禁止・本坑はクリーンに掘り直す）。
- **Adjacent expectations**:
  - 下流の本坑 `wintf-clickthrough-alpha-toggle` は、本 pilot の go 判定を `_Depends(confirmed):` 前提依存とする。本 pilot はその go ゲートとなる知見を提供するだけで、本体αマスク関数（実描画αバッファ参照）や ULW/DComp バックエンド改変は所有しない。
  - 隣接の完了済み `event-hit-test-alpha-mask`（既存αヒットテスト）および ULW 切替基盤（`com/ulw.rs`・`CompositionMode`）には本 pilot では触れない。
  - go 判定は開発者（人間）が下す。Claude Code 単独で合格判定して次フェーズに進まない。

## Requirements

### Requirement 1: 先進坑規律と葉ノード隔離

**Objective:** As a 開発者, I want 先進坑コードが production の出荷グラフに被依存しない葉ノードに隔離されていること, so that 実証が終われば（go／違う／直す のいずれであっても）コードを安全に捨てる、または知見クレートへ隔離保全できる

#### Acceptance Criteria
1. The pilot example shall `crates/pilot/examples/pilot-clickthrough-alpha-toggle/` 配下にのみコードを配置する。
2. The pilot example shall 他クレートからの inbound 依存（`pilot = { path = ... }` の追加）を一切持たない。
3. When 実証の成果物を整理するとき, the pilot shall コードではなく知見（go／違う／直す ＋ 学び）を一次成果物として扱う。
4. The pilot example shall 既存の `_template` をコピーして着手し、example フォルダ名を spec 名 `pilot-clickthrough-alpha-toggle` に一致させる。
5. While 先進坑コードを実装している間, the pilot shall 品質基準（整形・命名・テストの厳格さ）を緩めてよいが葉ノード隔離だけは厳守する。

### Requirement 2: 透過トップモスト窓と不透明領域

**Objective:** As a 開発者, I want 全域透明で中央に不透明な四角領域を持つトップモスト窓が生成されること, so that クリックスルーの ON/OFF 切替を観察できる検証台が用意される

#### Acceptance Criteria
1. When PoC を起動したとき, the pilot application shall 透過したトップモストのウィンドウを表示する。
2. The pilot application shall ウィンドウ全域を透明とし、中央に不透明な円領域を定義する。この描画される不透明円は、αマスクの判定領域（R4）と同一の領域とする。
3. The pilot application shall `WS_EX_LAYERED` を付けず `WS_EX_TRANSPARENT` 単独で別プロセス透過を成立させる。
4. The pilot application shall `WM_NCHITTEST` を自前ハンドルしない。
5. The pilot application shall HWND が `!Send` であっても Win32 慣例に従い状態をスレッド跨ぎで共有してよい。

### Requirement 3: カーソル監視ワーカ

**Objective:** As a 開発者, I want 別スレッドが周期的にカーソル位置を監視しαマスクへ問い合わせること, so that カーソル位置に応じてクリックスルー状態を自動で切り替えられる

#### Acceptance Criteria
1. While PoC が稼働している間, the cursor-monitoring worker shall 16ms 周期でカーソル位置を取得する。
2. The cursor-monitoring worker shall UI スレッドとは別スレッドで動作する。
3. The cursor-monitoring worker shall スレッド跨ぎの起床通知を非同期ランタイム非依存の手段で行う。
4. When カーソル位置を取得したとき, the cursor-monitoring worker shall αマスク関数に当該位置の透明／不透明を問い合わせる。

### Requirement 4: αマスク関数の差し替えシーム

**Objective:** As a 開発者, I want αマスク判定が独立した関数として差し替え可能であること, so that PoC では仮の円判定を使い、将来（本坑）は実描画αバッファ参照に差し替えられる

#### Acceptance Criteria
1. The alpha-mask function shall ウィンドウクライアント中央を中心とする半径 200px の円の外側を透明扱い、内側を不透明扱いと判定する（仮実装）。この判定領域は R2.2 で描画する不透明円と一致させる。
2. The alpha-mask function shall カーソル位置を入力として透明／不透明の判定を返す独立した差し替えシームとして実装される。
3. Where 仮の円判定が用いられる場合, the pilot shall 実描画αバッファ参照を実装しない。
4. The alpha-mask judgment shall プライマリモニタ専用の固定スクリーン座標を前提とせず、カーソルの物理スクリーン座標と、ウィンドウ位置から実算出した円の物理スクリーン位置とを同一座標基準で比較する。

### Requirement 5: 状態変化検出と拡張スタイル適用（状態変化最適化）

**Objective:** As a 開発者, I want クリックスルー状態が変化したフレームでのみ拡張スタイルを適用すること, so that 毎フレームの無駄な API 呼び出しを避けつつ正しく状態が切り替わる

#### Acceptance Criteria
1. When カーソルが円内にあるとき, the pilot shall クリックスルーを OFF（不透明領域としてクリックを受領）にする。
2. When カーソルが円外にあるとき, the pilot shall クリックスルーを ON（背面へ透過）にする。
3. When クリックスルー状態が前回フレームから変化したとき, the pilot shall `SetWindowLongPtr(GWL_EXSTYLE)` と `SetWindowPos(SWP_FRAMECHANGED)` を呼び出して `WS_EX_TRANSPARENT` を付け外しする。
4. While クリックスルー状態が前回フレームから変化していない間, the pilot shall 拡張スタイル適用 API（`SetWindowPos` 等）を呼び出さない。
5. When クリックスルー状態の切替が起きたとき, the pilot shall その切替をログに出力する。

### Requirement 6: 不透明領域のクリック受領とフィードバック

**Objective:** As a 開発者, I want 不透明領域へのクリックが当該プロセスに届きフィードバックされること, so that 不透明領域が実際にクリック可能であることを目視確認できる

#### Acceptance Criteria
1. When 不透明な円領域がクリックされたとき, the pilot application shall ウィンドウプロシージャで `WM_LBUTTONDOWN` を受領する。
2. When 不透明な円領域がクリックされたとき, the pilot application shall その受領をログに出力する。
3. When 不透明な円領域がクリックされたとき, the pilot application shall 円領域の色をトグル変更する。

### Requirement 7: DPI 認識とマルチモニタ／高 DPI 整合

**Objective:** As a 開発者, I want 高 DPI・マルチモニタ環境でも座標判定が見た目と一致すること, so that プライマリ専用前提に陥らず実運用に近い条件で実現可能性を判断できる

#### Acceptance Criteria
1. When `main` が開始するとき, the pilot application shall 冒頭で `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` を呼び出す。
2. While 高 DPI 環境（150% 等）で稼働している間, the pilot application shall 円の判定領域と画面に見えている不透明円が一致するよう、カーソルの物理スクリーン座標と窓位置から実算出した円の物理位置で判定する。
3. The pilot application shall マルチモニタ・高 DPI 環境を前提とし、プライマリモニタのみを前提にしない。

### Requirement 8: 終了処理

**Objective:** As a 開発者, I want ウィンドウを閉じるとプロセスとワーカスレッドが正常終了すること, so that リソースリークやハングなく検証を反復できる

#### Acceptance Criteria
1. When ウィンドウが閉じられたとき, the pilot application shall プロセスを正常終了する。
2. When ウィンドウが閉じられたとき, the pilot application shall カーソル監視ワーカスレッドを正常終了する。

### Requirement 9: 手動検証と go 判定・一次記録

**Objective:** As a 開発者, I want 試験項目 T1〜T8 を人間とともに手動検証し合否を REPORT に記録すること, so that 開発者が go／違う／直す を一次記録に基づいて判定できる

#### Acceptance Criteria
1. When 実証を検証するとき, the verification process shall 人間の準備確認 → エージェントによるプログラム起動 → 結果のヒアリング の手順で T1〜T8 を手動検証する。
2. The verification process shall T1（起動確認＝透過トップモスト窓表示）, T2（円外でのクリック透過＝背面アプリ反応）, T3（円内でのクリック受領＝WndProc に WM_LBUTTONDOWN）, T4（状態切替の発火＝円境界をまたぐ瞬間の ON↔OFF ログ）, T5（状態変化なし時の非発火＝留まっている間 SetWindowPos 非呼び出し）, T6（マルチプロセス透過＝背面ブラウザのリンクが円外クリックで開く）, T7（DPI 環境での座標一致）, T8（終了処理＝窓を閉じるとプロセス・ワーカスレッドが正常終了）の各期待結果を検証する。
3. The pilot shall T1・T2・T3・T4・T6 がすべて合格したことを必須合格基準とする。
4. Where T5・T7・T8 が条件付き合格となる場合, the pilot shall 合格または軽微な条件付き合格（理由明記）として扱う。
5. When 検証が完了したとき, the pilot shall 合否を問わず `REPORT.md`（指定フォーマット）と README 3 幕（動機・概要・検証結果）を一次記録として作成する。
6. The pilot shall go 判定を開発者（人間）の判断に委ね、Claude Code 単独で合格判定して次フェーズに進まない。

### Requirement 10: 技術・可搬性制約

**Objective:** As a 開発者, I want 既定のスタック制約と 32bit 可搬性が守られること, so that pilot の知見が本坑の前提条件と整合し移植時の齟齬を避けられる

#### Acceptance Criteria
1. The pilot example shall Rust 2024・`windows` 0.62.2 系・`event_listener` 5 の範囲で実装される。
2. The pilot example shall tokio を使用せず `event_listener` ＋ `std::thread` でスレッド構成を組む。
3. The pilot example shall 32bit 可搬性を崩さない。
4. When Win32 API またはクレート仕様で不確実な点に遭遇したとき, the pilot shall 推測で進めず開発者に質問する。
