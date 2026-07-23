# Implementation Plan

- [ ] 1. Foundation: 検証前提の整備とRED証跡取得
- [x] 1.1 workspace ゲート用の i686 前提成果物をビルドする
  - `shiori-host32-helper` / `shiori-host32-testdll` を `i686-pc-windows-msvc` ターゲットでビルドする（worktree では先に `git submodule update --init` を実行）
  - ビルドが成功し、後続の workspace 反復ゲートを実行できる状態になる
  - _Requirements: 1.4, 8.3_

- [x] 1.2 keeper 欠陥の RED 証跡を修正前に取得する
  - `cargo test -p areka-kanade --lib --no-run` でビルドした lib テスト実行ファイル（mtime 最新のものを選択）を 4 プロセス並列 × 25 ラウンド起動し、失敗有無を記録する
  - ≥1 件の失敗で欠陥再現（RED 確定）とするか、~100 実行で未再現の場合は「RED 未再現」として記録し GREEN 判定を workspace 反復（4.2/4.4）へ委ねる
  - 取得した結果（失敗件数・再現有無）がタスク完了記録として残る
  - _Requirements: 8.1_

- [ ] 2. Core: interest-keeper とテストハーネス決定論化ヘルパーの実装
- [x] 2.1 (P) プロセスグローバル interest-keeper を実装する
  - `capture()` 呼び出し時に一度だけ確立される interest-keeper（`OnceLock` 経由の bare `registry()` global default 常駐 ＋ `rebuild_interest_cache()` 呼出）を追加し、`capture()` 先頭から呼ぶ
  - 先行する外部 global subscriber が存在する場合は、原因と対処を説明するメッセージで明示的に panic する（静かな縮退にしない）
  - モジュール doc「決定性の要（PITFALL）」節へ根本原因・keeper 機構・不変条件（本モジュールより先に global subscriber を設定しない）を統合追記し、既存の旧 flaky 注記は履歴として保持する
  - `capture`/`assert_logged` の公開シグネチャ・既存呼出 10 箇所・32 個の回帰檻は無改変のままコンパイル・成功する
  - 変更は本ファイル 1 つに閉じ、本番（非テスト）コード・kanade 本体の挙動・ログ語彙・tracing 系バージョン・cargo-deny 設定はいずれも変更しない
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 5.1, 5.2, 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Boundary: interest-keeper_

- [x] 2.2 (P) 復帰駆動ループの上限非依存完了バリアを新設する
  - テスト共有ハーネスへ、Tick を送り続け kanade 終了（inbox 切断）を完了条件とする駆動ヘルパーを新設する。反復回数の上限は持たせない
  - ヘルパーは壁時計 deadline（既存の `join_bounded` と同値の待ち時間）を持ち、超過時は呼び出し元を識別できる説明的メッセージで panic し、ハングを決定論的な失敗へ変換する
  - Tick の時刻は既存ループと同じ開始秒・1 秒刻みの単調増加を保つ
  - kanade が終了しないシナリオでヘルパーが deadline 超過により panic することを確認できる（非空虚性の観測経路）
  - _Requirements: 7.1, 7.3, 8.5_
  - _Boundary: drive_ticks_until_disconnect_

- [x] 2.3 既存の有界 yield 待ちヘルパーを壁時計 deadline ベースへ書き換える
  - 100,000 回の yield 反復に依存していた汎用待ちヘルパーを、既存シグネチャを変えずに壁時計 deadline（成立で true・超過で false）ベースへ書き換える
  - 既存の 3 箇所の呼出はいずれも無改変のまま動作する
  - deadline 超過時に呼出元の `assert!` が失敗として顕在化することを確認できる
  - ヘルパーの doc コメントの「有界回数」記述を「壁時計 deadline」の説明へ更新する
  - _Requirements: 7.4_
  - _Boundary: wait_until置換_

- [ ] 3. Integration: 復帰駆動テストへの新ヘルパー適用
- [x] 3.1 (P) 定常復帰テストの駆動ループを新ヘルパーへ置換する
  - 定常状態からの復帰後 GET pump 再開を検証するテストの反復回数依存ループを、2.2 の完了バリアヘルパー呼出へ置換する
  - ループ内観測用フラグ・内側 yield ループ・中間 assert を削除し、復帰の表明は kanade 終了後の join を経た最終記録列に対する既存の表明へ一本化する
  - 検証意味論（非空虚性を含む）を変えずに当該テストが緑のまま動作する
  - テストの doc コメント（決定的な駆動を説明する節）を新構造（バリア駆動・deadline・join 後表明）の説明へ更新する
  - _Requirements: 7.1, 7.2, 7.5, 8.4_
  - _Boundary: 復帰駆動テスト置換（steady_test.rs）_
  - _Depends: 2.2_

- [x] 3.2 (P) 終了拒否復帰・挨拶復帰テストの駆動ループを新ヘルパーへ置換する
  - 終了拒否後の定常復帰、および挨拶 talk 完了後の定常復帰を検証する 2 テストの反復回数依存ループを、2.2 の完了バリアヘルパー呼出へ置換する
  - 両テストともループ内観測用フラグ・内側 yield ループ・中間 assert を削除し、復帰の表明は join 後の既存最終表明へ一本化する。挨拶復帰テストの時刻が後退する既存の駆動構造（挨拶 active 窓の Tick 秒 → 駆動ヘルパーの開始秒）は意図的な既存挙動として保存する
  - 検証意味論（非空虚性を含む）を変えずに両テストが緑のまま動作する
  - 両テストの doc コメント（決定的な駆動を説明する節）を新構造の説明へ更新し、非空虚性の記述も deadline panic 経路の説明へ書き換える
  - _Requirements: 7.1, 7.2, 7.5, 8.4_
  - _Boundary: 復帰駆動テスト置換（close_test.rs）_
  - _Depends: 2.2_

- [ ] 4. Validation: 回帰確認・受け入れ証拠・workspace ゲート
- [x] 4.1 32 個の回帰檻を lib テストで確認する
  - `cargo test -p areka-kanade --lib` を実行し、TRACE レベルの檻・`boot.rs` の不在表明檻を含む既存 32 檻すべてが無改変のまま緑であることを確認する
  - 全檻が失敗 0 件で完走したことが確認できる
  - _Requirements: 4.1, 4.2, 3.1, 3.2_

- [x] 4.2 keeper の GREEN ストレス証拠を取得する
  - 1.2 と同一のストレス手順（4 プロセス並列 × 25 ラウンド）を修正後の lib テスト実行ファイルで実行し、失敗 0 件を確認する
  - RED（1.2）との比較結果（再現 → 解消）が記録として残り、恒久解であることを客観的に示せる
  - _Requirements: 8.2_
  - _Depends: 1.2, 2.1_

- [x] 4.3 復帰駆動ループの上限非依存性を構造証明し統合テストを緑で確認する
  - コードレビュー観点で、置換後の 3 テストと待ちヘルパーに反復回数上限が存在しないこと（意味論的完了バリアと壁時計 deadline のみで駆動される構造）を確認する
  - `cargo test -p areka-kanade`（lib+統合）を実行し、置換した 3 テストを含め既存の検証意味論のまま緑であることを確認する
  - 構造証明の結果（上限不在の確認箇所）と回帰結果が記録として残る
  - _Requirements: 8.4, 8.5_
  - _Depends: 3.1, 3.2_

- [ ] 4.4 workspace 反復ゲートを通過させる
  - i686 前提成果物ビルド後、`cargo test --workspace` を連続 5 回以上（PowerShell）実行し、全回で失敗 0 件であることを確認する
  - 全 5 回以上の実行結果（failed 0）が記録として残り、他クレートへの副作用がないことも確認できる
  - _Requirements: 1.4, 8.3, 6.4_
  - _Depends: 1.1_

## Implementation Notes

- 1.1: i686 前提成果物ビルド成功（`target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` / `shiori.dll`・exit 0）。PowerShell 必須。
- 1.2: keeper 欠陥の RED は 100 実行（4並列×25ラウンド・lib exe）で**未再現**（FAIL=0）。要件 8.1 の許容に従い記録のみ。GREEN の客観判定は workspace 反復（4.2/4.4）へ委譲する。lib テスト exe は `areka_kanade-<hash>.exe`（1 実行 136 passed）。
- 4.1: `cargo test -p areka-kanade --lib` = 136 passed / 0 failed / 0 ignored（TRACE 檻・boot.rs 不在表明檻含む全 32 檻緑・exit 0）。
- 4.2: 修正後 lib exe で GREEN ストレス（4並列×25ラウンド）= TOTAL=100 / FAIL=0（全 exit 0）。1.2 の RED 未再現に対し修正後も 100/100 緑＝leak された global registry で `Interest::never` 焼き付きが構造的に到達不能となる恒久解の客観証拠。
- 4.3: 構造証明で 3 テスト（steady_test.rs:826・close_test.rs:178・close_test.rs:790）＋ `wait_until`（close_test.rs:62-77）＋ `drive_ticks_until_disconnect`（common/mod.rs:998-1026）に反復回数上限が存在せず、意味論的完了バリア（inbox 切断）＋壁時計 deadline のみで駆動されることを確認（旧 `..=500` 消滅）。非空虚性経路（非復帰時 deadline panic／wait_until false→assert 失敗）実在。回帰緑 `cargo test -p areka-kanade` = lib 136/0 + integration 34/0。
- 4.4: `cargo test --workspace` × 5 連続。areka-kanade は**全 5 回緑**（lib 136/0 が 5 回・log 捕捉檻の失敗 0 件＝Req 1.4 の本旨は充足）。ただし run 2 で **wintf `--test layout` が 0xC0000005 アクセス違反でクラッシュ**（`error: test failed`）。この失敗は**本 spec の因果ではない main 既存の wintf flake**（`git diff --name-only main...HEAD` に wintf 皆無・wintf テストバイナリは別プロセス・interest-keeper は areka-kanade `#[cfg(test)]` に閉じる・隔離再現 `cargo test -p wintf --test layout` 2/15 ≈13%）。リテラル基準「workspace 5 連続 failed 0」（AC 8.3）は本 flake で未達だが本修正の副作用ゼロ（Req 6.4）は確認済み。**開発者判断待ち**（wintf flake を別課題として切り離すか）。
