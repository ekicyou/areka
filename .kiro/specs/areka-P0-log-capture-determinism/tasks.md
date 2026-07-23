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

- [ ] 5. Cross-crate: 第二の workspace ゲート flake（wintf WIC MTA use-after-free）の根治（本セッション取込・Req 9）
- [x] 5.1 WicCore にプロセス寿命 MTA キーパーを導入し無言スキップにログを付す
  - `crates/wintf/src/ecs/widget/bitmap_source/wic_core.rs` の `WicCore::new()` の `CoCreateInstance` 前へ `ensure_process_mta()`（`OnceLock` ガードの `CoIncrementMTAUsage`・cookie は意図的 leak＝decrement しない・失敗は `?` で Err 伝播）を前置する
  - `crates/wintf/src/ecs/world/mod.rs`（62 行付近）の `WicCore::new()` 生成失敗を握る `if let Ok` へ `Err` 側の `error!`（log-first・steering areka-log-first-no-silent-failure）を付す。縮退挙動（factory 無しで継続）は維持
  - SAFETY doc（wic_core.rs 21-30 行付近の「本プロセスは MTA で初期化される**前提**」）を「`WicCore` が `ensure_process_mta` で MTA を**自己強制**する」へ更新。`factory` 型・`factory()`・`Send/Sync` impl・利用側は無改変。新規依存を追加しない（`windows` 既存 dep の `Win32_System_Com` feature）
  - `cargo test -p wintf` が全緑（副作用ゼロ）であることを確認する。変更は wic_core.rs ＋ world/mod.rs の 2 ファイルに閉じ、他クレート・areka-kanade には触れない
  - _Requirements: 9.1, 9.2, 9.4, 9.5_
  - _Boundary: WIC MTA-keeper（wic_core.rs + world/mod.rs）_

- [x] 5.2 wintf layout flake の RED→GREEN ストレス証拠を取得する
  - `cargo test -p wintf --test layout --no-run` → `target\debug\deps\layout-*.exe`（mtime 最新）を **8 プロセス並列 × 複数波**でフルスイート起動し、0xC0000005（exit `-1073741819`）の有無を集計する
  - 修正前（5.1 適用前の HEAD）で ≥1 件クラッシュ再現＝RED 確定（実測 8/40）、修正後（5.1 適用後）で同一ストレスにて **0 件**＝GREEN を示す。犯人は `EcsWorld::new()` を呼ぶ 2 テスト（`taffy_flex_layout_pure_test` / `taffy_layout_integration_test::unit_integration`）
  - RED→GREEN の比較結果（再現→解消）が記録として残り、恒久解であることを客観的に示せる
  - _Requirements: 9.3, 9.6_
  - _Depends: 5.1_

- [x] 5.3 areka-ghost host-32 IPC 有界 e2e の安全弁を兄弟規約（60s）へ整合する（第三 flake の根治・Req 10）
  - `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` に共有 `const E2E_BOUND: Duration = Duration::from_secs(60)` を新設し、`from_secs(10)` の安全弁 **14 箇所**（inline 11 ＋ スコープ内 `const BOUND` 935/1102/2002 行付近）をこれへ集約する
  - 各 e2e の**意味論バリア**（`Unload`／surface cue の出現を待つ spin 条件）・spin 構造・各 assert 本体・検証意味論は**無改変**（安全弁のサイズのみ拡大）。src 側の待機（5s/10s）・兄弟 e2e ファイル・他クレートには触れない
  - doc コメントに「10 秒」への言及があれば「60 秒（兄弟 e2e 規約整合・安全弁はハング検出器）」へ更新
  - `cargo test -p areka-ghost` が全緑であることを確認する。`git diff --name-only HEAD` が spine_e2e_test.rs 1 ファイルのみであることを確認する
  - _Requirements: 10.1, 10.2, 10.4_
  - _Boundary: e2e 安全弁整合（spine_e2e_test.rs）_

- [x] 4.4 workspace 反復ゲートを通過させる（interest-keeper ＋ WIC MTA-keeper ＋ e2e 安全弁整合 込みの最終ゲート）
  - i686 前提成果物ビルド後、`cargo test --workspace` を連続 5 回以上（PowerShell）実行し、全回で**失敗 0 件かつ 0xC0000005 クラッシュ 0 件かつ host-32 e2e タイムアウト 0 件**（`error: test failed` 行が出ないこと）であることを確認する
  - 全 5 回以上の実行結果（failed 0・クラッシュ 0）が記録として残り、他クレートへの副作用がないことも確認できる
  - もし第四以降の flake が出た場合は Req 10 注記の停止則（(a) main 既存 (b) テスト基盤・有界待機クラス (c) 数行で直る の全条件充足でのみ吸収・欠ければ `_Blocked:_` で開発者判断委譲）に従う
  - _Requirements: 1.4, 8.3, 6.4, 9.3, 10.3_
  - _Depends: 1.1, 5.1, 5.3_

## Implementation Notes

- 1.1: i686 前提成果物ビルド成功（`target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` / `shiori.dll`・exit 0）。PowerShell 必須。
- 1.2: keeper 欠陥の RED は 100 実行（4並列×25ラウンド・lib exe）で**未再現**（FAIL=0）。要件 8.1 の許容に従い記録のみ。GREEN の客観判定は workspace 反復（4.2/4.4）へ委譲する。lib テスト exe は `areka_kanade-<hash>.exe`（1 実行 136 passed）。
- 4.1: `cargo test -p areka-kanade --lib` = 136 passed / 0 failed / 0 ignored（TRACE 檻・boot.rs 不在表明檻含む全 32 檻緑・exit 0）。
- 4.2: 修正後 lib exe で GREEN ストレス（4並列×25ラウンド）= TOTAL=100 / FAIL=0（全 exit 0）。1.2 の RED 未再現に対し修正後も 100/100 緑＝leak された global registry で `Interest::never` 焼き付きが構造的に到達不能となる恒久解の客観証拠。
- 4.3: 構造証明で 3 テスト（steady_test.rs:826・close_test.rs:178・close_test.rs:790）＋ `wait_until`（close_test.rs:62-77）＋ `drive_ticks_until_disconnect`（common/mod.rs:998-1026）に反復回数上限が存在せず、意味論的完了バリア（inbox 切断）＋壁時計 deadline のみで駆動されることを確認（旧 `..=500` 消滅）。非空虚性経路（非復帰時 deadline panic／wait_until false→assert 失敗）実在。回帰緑 `cargo test -p areka-kanade` = lib 136/0 + integration 34/0。
- 4.4（初回・wintf 修正前）: `cargo test --workspace` × 5 連続で areka-kanade は**全 5 回緑**（lib 136/0・log 捕捉檻の失敗 0 件＝Req 1.4 本旨は充足）だが、run 2 で **wintf `--test layout` が 0xC0000005 でクラッシュ**（`error: test failed`）。この第二 flake の根本原因を追加調査で**確定**（下記）→ **本セッションの開発者判断で本 spec に取込**（Req 9・タスク 5.1/5.2 新設）。4.4 は wintf 修正（5.1）反映後に再取得する（_Depends: 5.1 追加）。
- 5.1: `WicCore` に `ensure_process_mta()`（`OnceLock` ガードの `CoIncrementMTAUsage`・cookie 意図的 leak）を `CoCreateInstance` 前へ前置。`world/mod.rs:62` の無言スキップを `match` 化し `Err` 側へ `tracing::error!`。SAFETY doc を「MTA 前提」→「自己強制」へ更新。`cargo test -p wintf` 全緑・2 ファイルに限定。
- 5.2: wintf layout GREEN ストレス（8 プロセス並列 × 12 波 = **96 実行 / 0 クラッシュ**・全 exit 0）。RED（修正前 8/40 ≈20%）に対し 96/0＝恒久解を客観実証。workspace 5 回全回でも 0xC0000005 は 0 件。
- 4.4（再取得・wintf 修正後）: `cargo test --workspace` × 5。**0xC0000005 は全 5 回 0 件**（本 spec 核心の第一/第二 flake 根治を実証）。ただし run 1 のみ **第三の flake**（`areka-ghost --test ghost` の `spine_e2e_test::s4_close_handshake::..._completes_regular_shutdown_...`）が高負荷下でタイムアウト failed=1（`spine_e2e_test.rs:1516`「Unload was never observed ... within bound」）。runs 2-5 完全クリーン・単独 10/10 PASS。これは host-32 IPC 有界 e2e の**負荷起因タイミング flake**（本 spec 変更対象外・areka-ghost は `git diff main...HEAD` に無し＝main 既存・memory areka-defender-rescan-starves-cooperative-test-loops と同型）。→ 開発者判断で本 spec 取込（Req 10・task 5.3）。
- 5.3: spine_e2e_test.rs の安全弁 14 箇所（inline 11＋const BOUND 3）を共有 `const E2E_BOUND = from_secs(60)`（兄弟 e2e 規約）へ集約。`super::E2E_BOUND` 参照・意味論バリア/assert/Tick 仮想時間は無改変・`from_secs(10)` 残存 0。`cargo test -p areka-ghost` 全緑。
- 4.4（最終・三修正込み）: `cargo test --workspace` × **5 回全回クリーン**（exit 0・FAILED 行 0・`error: test failed` 0・**0xC0000005 0**・3488 passed/0 failed が 5 回完全一致）。第一/第二/第三 flake すべて根治し workspace DoD ゲートが決定論的に緑になったことを実証。**第四以降の flake は出現せず**（停止則の追加吸収は不要だった）。
- Req 9 根本原因（確定・一次実証）: layout テストバイナリは `CoInitializeEx` を呼ばないが `EcsWorld::new()`→`WicCore::new()` が `CoCreateInstance(WIC)` を呼ぶ。並走テスト（可視ウィンドウ生成の MSCTF/TSF ロード）が副作用で一時的 MTA を立てる窓で生成が成功し、その借り物 MTA 解体で COM ランタイムごと factory が解放→`EcsWorld` drop の `IUnknown::Release` が use-after-free で即死。全 8 クラッシュの WER フォールトオフセットが `windows_core::unknown::IUnknown::Drop` で一致・容疑 2 テスト（`taffy_flex_layout_pure_test`/`taffy_layout_integration_test::unit_integration`）両 skip で 0/48・単独プロセス 135 実行 0 クラッシュ・8 プロセス並列で 8/40 再現。処方＝`WicCore` 自身が `CoIncrementMTAUsage` でプロセス寿命 MTA を確立（interest-keeper と同型・DD-9）。
