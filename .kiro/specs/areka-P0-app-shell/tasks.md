# Implementation Plan

- [ ] 1. モックデモを example へ移設し保全する
- [x] 1.1 モック UI 本体（定数・マーカー・生成関数・登録システム・ハンドラ・main 結線・操作ガイド）を `examples/mock-shell.rs` へコピーして構築する
  - シェル／バルーン窓の生成関数・ドラッグ／ダブルクリックハンドラ・クリック透過登録システムを新規 example ファイルへ移す
  - example 独自の tracing subscriber 初期化を追加し、`windows_subsystem` 属性は付与しない
  - `cargo run -p areka --example mock-shell` が起動し、シェル＋バルーン 2 窓が表示され、ドラッグでバルーンが追従し、ダブルクリックで全窓が終了する（従来デモと同一挙動）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6_

- [x] 1.2 モック UI ユニットテストを移設先へ同居させる
  - 現 `src/tests.rs` の内容を `examples/mock-shell.rs` 末尾の `#[cfg(test)] mod tests` として移設する
  - モック固有アセット参照（`shell/base.png`）・座標定数・表示テキストが example 側にのみ存在し、`src/` 配下から参照されないことを確認する
  - `cargo build -p areka --example mock-shell` が移設テストモジュールを含めてビルド成功する（`cargo test` の標準ハーネスでの実行は要求しない）
  - _Requirements: 1.5, 6.3, 6.4_
  - _Boundary: mock-shell example_

- [ ] 2. 骨格の新規責務（構成解決・検証用ダミー窓）を実装する
- [x] 2.1 (P) ゴースト／バルーンのルートパスを解決する純粋関数を実装する
  - 起動引数（位置引数 2 個）が与えられた場合はその値を採用し、欠落時は `CARGO_MANIFEST_DIR` 相対の既定パスへフォールバックする
  - 単体テストで「引数 2 個あり」「引数なし」「ghost のみ引数あり」の 3 分岐が期待どおりのパスを返すことを確認する
  - 関数は `std::env::args`／`std::path` のみに依存し、追加の外部依存もマウント処理も行わない
  - _Requirements: 3.1, 3.3, 3.4, 6.1_
  - _Boundary: resolve_config_inputs_

- [x] 2.2 (P) 検証用ダミー窓を開く replace-me シームを実装する
  - `WinApp` の共有参照を受け取り、ゴースト内容・配置・座標・DPI ロジックを持たない最小の窓エンティティを ECS 経由で spawn する
  - ダブルクリック時にダミー窓エンティティを despawn するハンドラを、現デモの `on_shell_pressed` の despawn 経路に倣って実装する
  - 手動起動後にダミー窓をダブルクリックすると despawn され、`WindowRegistry` が空へ遷移することを確認する
  - _Requirements: 2.5, 4.2_
  - _Boundary: open_startup_window_

- [x] 2.3 検証用ダミー窓に env ゲート付き自動 close 機構を追加する
  - 指定環境変数（`AREKA_` 冠規約）が設定されている場合のみ、`wintf::executor::spawn_local`（`wintf` が再公開する `executor` エイリアス経由・`wintf_winmsg_executor` を直接 import しない）で一発の非同期タスクを投入する
  - タスクは `async_io::Timer::after` で指定ミリ秒スリープした後、world の弱参照を経由してダミー窓エンティティを despawn する
  - 環境変数が未設定のときはこの機構が一切発火せず、ダミー窓は利用者の close を待ち続けることを確認する
  - _Requirements: 4.1_
  - _Depends: 2.2_
  - _Boundary: open_startup_window_

- [ ] 3. 骨格 main.rs を組み上げる
- [ ] 3.1 main.rs をモック UI 除去後の骨格へ書き換え、新規要素を結線する
  - モック UI 塊（定数・マーカー・生成関数・登録システム・ハンドラ・窓生成結線・操作ガイド）と `#[cfg(test)] mod tests;` 宣言を main.rs から除去する
  - 既存の tracing subscriber 初期化・`human_panic::setup_panic!()`・SHIORI モジュール宣言 5 本・e2e テスト宣言 3 本・`shiori_demo::run_demo_if_enabled()` 呼び口・`windows_subsystem` 属性を維持する
  - `resolve_config_inputs` の呼び出しと解決結果のログ出力、`open_startup_window` シームの呼び出し、`main` 自身による `app.run()` 呼び出しをこの順で結線する
  - `cargo run -p areka`（引数なし）が構成入力をログ出力した後に検証用ダミー窓を表示し、ダブルクリックで exit 0 により正常終了することを確認する
  - `cargo run -p areka -- <ghost> <balloon>` が引数値を解決結果ログへ反映することを確認する
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.2, 3.5, 4.1, 4.3, 5.1, 5.3, 5.4, 6.1_
  - _Depends: 1.1, 1.2, 2.1, 2.2, 2.3_
  - _Boundary: 骨格 main_

- [ ] 4. 回帰・実証検証を行う
- [ ] 4.1 (P) SHIORI 契約チェーンの e2e 回帰テストを実行し green を確認する
  - `cargo test -p areka` を実行し、`shiori_e2e_tests`／`shiori_lifecycle_e2e_tests`／`shiori_reference_e2e_tests` が全て成功することを確認する
  - `shiori_demo` の env-gate 単体テスト（ゲート無効／有効の両分岐）が成功することを確認する
  - テスト結果が全て green であることをコマンド出力で確認する
  - _Requirements: 5.2, 6.2_
  - _Depends: 3.1_
  - _Boundary: 残置 SHIORI 群_

- [ ] 4.2 (P) 骨格の boot→loop→exit を自動 smoke テストで証明する
  - env ゲート（2.3）を有効にした `cargo run -p areka` の子プロセスを起動する統合テストを実装する
  - テストは境界時間（タイムアウト番犬）内にプロセスが exit 0 で終了することを assert する
  - smoke テストを実行し、実際に境界時間内で exit 0 が観測されることを確認する
  - _Requirements: 4.1, 2.4_
  - _Depends: 2.3, 3.1_
  - _Boundary: 骨格 main, open_startup_window_

## Implementation Notes

- **wintf の `Window` on-add フックが `WindowPos::default()`（CW_USEDEFAULT）を自動挿入する**（`crates/wintf/src/ecs/window/components.rs` の `on_window_add`・位置未指定時のみ）。ゆえにダミー窓 builder が `WindowPos` を一切セットしなくても entity には `WindowPos::default()` が付く＝これが「座標/DPI を主張しない既定配置」の正しい姿。テストは「存在する `WindowPos` が `WindowPos::default()` と等しい」ことを assert して非主張を証明する（task 2.2）。
- **example 内 `#[test]` は標準ハーネス（`cargo test -p areka`）では走らない**。example のテスト実行/コンパイル検証は `cargo test -p areka --example mock-shell` を使う（task 1.2）。標準スイートの緑判定対象は bin/lib のユニット＋SHIORI e2e（task 1.2 時点で 61、以降タスクごとに増加）。
- **main.rs のインライン test モジュール名は衝突回避で使い分ける**: `mod tests;`（file・モック用・3.1 で削除）／`mod config_input_tests`（2.1）／`mod startup_window_tests`（2.2）。3.1 でモック除去後に整理する。
