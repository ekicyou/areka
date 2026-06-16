# final-launch: 最終起動テスト（S7）の実行と完全解消（タスク21）

- cell-id: final-launch
- task: 21（全60セル完了後の最終起動ゲート）
- 性質: 起動スモークテスト（実行時の生存確認・最終的な挙動非破壊検証）
- 最終判定: **S7 PASS**
- source code 変更: **なし**（修正不要で合格）。よって本タスクのコミットは docs（本断片 final-launch.md の追加）＋ tasks.md チェックボックス更新のみ — コミットは親が行う

全60セルタスク（19レビュー領域 × 3観点＝X1 含め、HEAD=`8e4809e` の X1-V まで）完了後の最終起動ゲート。`RUST_LOG=info` で areka を起動し、タイムアウト60秒内に初期化完了ログ `[GraphicsCore] Initialization completed` が出現すること、パニック・error レベルログ・異常終了コードがないことを確認した。**3合格要件すべてを満たし S7 PASS。** 起動約0.21秒で初期化完了ログ出現、stderr 完全に空、INFO レベルログのみ（WARN/ERROR/panic ゼロ）、強制終了まで正常稼働。フェーズ0実測〔約1秒で出現・stderr 空・正常稼働〕および同一 HEAD での先行実測とも整合する。

## 観点・基準・範囲

- requirements（source 番号）: 4.6（全タスク完了時にアプリ起動テストを最終的な挙動非破壊検証として実行）・4.7（最終起動テスト失敗時はデバッグで解消してから完了）・5.1（外部観測可能挙動を変更しない＝GUI/COM 領域の最終統合証拠）。
- design: スロット S7（design.md L137「`RUST_LOG=info` で `cargo run -p areka` を起動し、タイムアウト（既定60秒）内に初期化完了を示すログを確認後、プロセスを終了する。パニック・error レベルログ・異常終了コードがなければ合格」）、全体フロー Launch（L228-232）、Revalidation/Risk 表 L493「最終起動テスト失敗＝S7 不合格 → kiro-debug で解消するまで完了としない。直近のセル群のコミットを bisect 的に疑う」、検証戦略「最終 E2E」L509（GUI/COM 領域の挙動非破壊に対する最終の統合的証拠）、セル断片様式（L440）。
- 合否判定基準（tasks.md「実行記録」節 L23 で確定済み・これが正）:
  - **初期化完了ログ文字列**: `[GraphicsCore] Initialization completed`（`wintf::ecs::graphics::core`、INFO レベル）— ソース実体 `crates/wintf/src/ecs/graphics/core.rs:55` の `info!(...)`。
  - **補助確認**: areka 側ログ `シェルウィンドウとバルーンウィンドウを生成しました` — ソース実体 `crates/areka/src/main.rs:118` の `tracing::info!(...)`。
  - **タイムアウト**: 60秒。
  - **合格条件**: タイムアウト内に初期化完了ログが出現し、**パニック・error レベルログ・異常終了コードがないこと**（INFO/WARN は許容、WARN は内容記録）。
- 境界（boundary）: ソースコード・テスト・`vendors/`・機能spec文書は一切変更しない（S7 合格前提＝合格したため変更なし）。記録は `report/cells/final-launch.md` のみ（report 配下＝全セル境界内）。tasks.md のタスク21チェックボックス更新は全セル境界内（tasks.md 冒頭「境界の注」）。

## 実行手順（実測・本ランは新鮮なエビデンスで再実行したもの）

起点: クリーンなワークツリー、`HEAD=8e4809ecae8c32ae6d68cd6037f59f877af6342d`（X1-V コミット = 全60セルの最後。`git status --porcelain` 出力は本断片 final-launch.md の未追跡のみ＝ソース/テストはクリーンを実測確認）。

1. **ビルド**: `cargo build -p areka`
   - 結果: **成功（exit 0）**。`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.26s`（最新コミット状態で既ビルド済み・差分なしの増分ビルド）。
   - 成果物: `target/debug/areka.exe`（12,252,672 バイト、mtime `2026-06-16T02:12:08.982Z`）を実在確認。
2. **起動**: 環境変数 `RUST_LOG=info` を設定し、`target/debug/areka.exe` を `Start-Process -PassThru -WindowStyle Minimized`（バックグラウンド）で起動。
   - stdout → `target/s7_launch.out.log`、stderr → `target/s7_launch.err.log`（いずれも gitignore 済み `target/` 配下、`-RedirectStandardOutput`/`-RedirectStandardError`）。
   - PID=35876、spawn 時刻=`2026-06-16T03:17:38.310Z`（UTC）を記録。
3. **ログ監視**: 60秒タイムアウトのポーリングループ（200ms 間隔）で stdout を読み、ANSI 装飾を除去したうえで `[GraphicsCore] Initialization completed` の出現を監視（補助で `シェルウィンドウとバルーンウィンドウを生成しました` も確認）。各反復で `proc.HasExited` を確認し早期クラッシュを検出する設計。
   - 初期化完了ログ・補助ログともに出現を確認（`FOUND_INIT_LOG=True` / `AUX_FOUND=True`、`CRASHED_EARLY=False`、プロセスは生存継続＝早期終了せず）。
4. **終了**: `Stop-Process -Id 35876 -Force` でプロセスを確実に終了（`Stop-Process OK`）。終了後、`Get-Process -Name areka` ヒット0（`STRAY_AREKA_AFTER_KILL=0`、ストレイ/ゾンビ・残留ウィンドウなし）を実測確認。
   - 注: 前ランでは `taskkill /F /PID` を用いたが、本環境のコマンド安全フックが `/F` トークンを Remove-Item 対象パスと誤認しブロックするため、PowerShell ネイティブの `Stop-Process -Force` に変更した（プロセス終了の意味は同一）。
5. **後始末**: 捕捉ログ（`target/` 配下）は本断片への記録・独立レビュー完了後に削除（リポジトリにゴミを残さない。`target/` は `.gitignore` 行1で除外済みでありいずれにせよ非追跡）。

## 起動ログ（捕捉した実ログ全文・ANSI 装飾は除去して転記）

stdout（タイムスタンプは UTC。捕捉ファイル `target/s7_launch.out.log` 1,280 バイトには tracing の ANSI 装飾コードが含まれるため、判定スキャン・本転記ともに ESC シーケンス `\e[…m` を除去している）:

```
2026-06-16T03:17:38.405589Z  INFO wintf::process_singleton: Window classes created
2026-06-16T03:17:38.416027Z  INFO wintf::ecs::layout::systems::monitor_systems: [initialize_layout_root] Creating LayoutRoot singleton

areka モック実装 — ぱすたさん
================================
  ドラッグ移動: シェル画像を左クリック & ドラッグ
  終了:         シェル画像をダブルクリック

2026-06-16T03:17:38.428091Z  INFO areka: シェルウィンドウとバルーンウィンドウを生成しました
2026-06-16T03:17:38.430002Z  INFO wintf::ecs::graphics::systems::init: [init_graphics_core] GraphicsCore initialization started frame=1
2026-06-16T03:17:38.430293Z  INFO wintf::ecs::graphics::core: [GraphicsCore] Initialization started
2026-06-16T03:17:38.518040Z  INFO wintf::ecs::graphics::core: [GraphicsCore] Initialization completed
2026-06-16T03:17:38.518853Z  INFO wintf::ecs::graphics::systems::init: [init_graphics_core] GraphicsCore initialization completed frame=1
```

stderr: **空（0 バイト）**。

経過時間（spawn `03:17:38.310Z` 基準・アプリ自身のログタイムスタンプより算出）:
- Window classes created（`38.405589Z`）: +0.095 秒。
- 補助ログ `シェルウィンドウとバルーンウィンドウを生成しました`（`38.428091Z`）: **+0.118 秒**。
- 初期化完了ログ `[GraphicsCore] Initialization completed`（`38.518040Z`）: **+0.208 秒**。

いずれも 60 秒タイムアウトに対し圧倒的に余裕（フェーズ0実測「起動約1秒で出現」と整合。本ランは負荷が軽く更に高速）。ポーリング検出の実測経過（init +0.275 秒 / aux +0.276 秒）は 200ms ポーリング間隔の遅延を含むが、いずれもアプリのログ出力時刻と整合する。

注: ログには近接文字列の別行 `[GraphicsCore] Initialization started`（同モジュール・開始側）と、別モジュールの `[DCompGraphicsResource] Initialization completed`（`dcomp_resource.rs:61`）が存在しうるが、判定は **`[GraphicsCore] Initialization completed`（完了・開始でない）** の完全一致（部分文字列 Contains）で行い、誤検出を排除した（本ランの捕捉ログでは `[DCompGraphicsResource]` 行は未出力、`[GraphicsCore]` 系は started/completed が各1行）。

## 合否判定（3合格要件・実測根拠）

| # | 合格要件 | 結果 | 実測根拠 |
|---|---------|------|---------|
| 1 | 初期化完了ログがタイムアウト（60秒）内に出現 | **PASS** | `[GraphicsCore] Initialization completed` をアプリログ時刻 **+0.208 秒**（`03:17:38.518040Z`）で捕捉。ポーリングは `FOUND_INIT_LOG=True`（検出経過 +0.275 秒）。補助ログ `シェルウィンドウとバルーンウィンドウを生成しました` も +0.118 秒で出現（`AUX_FOUND=True`）。 |
| 2 | パニック・error レベルログがない | **PASS** | 結合ログ（ANSI 除去）正規表現スキャン: `panicked at\|thread … panicked\|\bpanic\b` ヒット **0**、行頭 tracing `ERROR ` レベル **0**、大小無視 `error` 文字列 **0**、行頭 `WARN ` **0**。stderr **0 バイト**。全ログ INFO レベルのみ。記録すべき WARN なし。 |
| 3 | 異常終了コードがない | **PASS** | アプリは起動後 +0.2 秒で初期化完了し、監視中ずっと正常稼働（`PROC_ALIVE_BEFORE_KILL=True`・`CRASHED_EARLY=False`）。**アプリ自身は panic 終了・非ゼロ異常終了をしていない**（自己終了せず生存継続）。検証完了後に検査者が `Stop-Process -Force` で意図的に終了（成功）したものであり、これは正常な検査終了。終了後 PID 消滅・ストレイ areka プロセス **0** を確認。 |

3要件すべて充足 → **S7 合格。**

## 失敗時対応（該当なし）

S7 は本実行で合格したため、kiro-debug による根本原因解消・直近セル群のコミットの bisect 的疑い・巻き戻しは**不要**（tasks.md 21 本文・design.md L493 の失敗時手順は発動せず）。ソースコード修正なし。

## 観測した WARN ログ

**なし**（全ログ INFO レベル。WARN/ERROR/panic ともゼロ）。留意すべき補助ログは正常系の起動シーケンス（Window classes 作成 → LayoutRoot 生成 → シェル/バルーンウィンドウ生成 → GraphicsCore 初期化 started→completed → init system 完了）のみで、すべて期待どおり。

## 後始末・境界遵守

- 捕捉ログ（`target/s7_launch.out.log`・`target/s7_launch.err.log`）は独立レビューでの突き合わせ用に一時温存し、記録・レビュー完了後に削除する。`target/` は `.gitignore` 行1で除外済みであり、いずれにせよ追跡対象・リポジトリルートにゴミは残さない。
- GUI プロセスは確実に終了（PID 35876 消滅・ストレイ 0・残留ウィンドウなし）。ゾンビプロセスなし。
- ソースコード/テスト/`vendors/`/機能spec文書 未変更。記録は本断片＋ tasks.md のタスク21チェックボックス更新のみ（コミットは親が実施）。

## 自己レビュー

- **合格の非偽装**: 初期化完了ログ `[GraphicsCore] Initialization completed` が実際に出現したことを**捕捉した実ログ全文**（`03:17:38.518040Z` の行）で確認。経過時間 +0.208 秒・終了状況（Stop-Process 成功、アプリ自己終了なし）はすべて実測値で、推測なし。stderr 0 バイト・panic/error/WARN 各 0 ヒットは正規表現スキャンの実数。
- **合否基準の正当性**: tasks.md「実行記録」L23 で確定済みの S7 基準（初期化完了ログ文字列・60秒タイムアウト・パニック/error/異常終了コードなし）に厳密準拠。ログ文字列はソース（`core.rs:55`・`main.rs:118`）と完全一致を事前確認のうえ、完了側（started でない）で判定。
- **記録の実測整合**: ビルド exit 0、HEAD=`8e4809e`（クリーン）、spawn `03:17:38.310Z`、init-complete `03:17:38.518040Z`（+0.208s）、補助ログ +0.118s、stderr 0 バイト、Stop-Process 成功・PID 消滅・ストレイ 0 — すべて cargo/PowerShell/実ログ実測と一致。フェーズ0実測（約1秒で出現・stderr 空・強制終了まで正常稼働）とも整合。
- 結論: 全60セルの改善（テスト +289 特性化テスト・デッドコード除去・フレーキー決定論化・SAFETY 注記格上げ等、いずれも挙動非破壊）の最終統合証拠として、areka は `RUST_LOG=info` 起動で panic/error なく約0.2秒で初期化完了し正常稼働した。GUI/COM/DirectComposition 領域を含む挙動非破壊が S7 で確認された。**S7 PASS** — 修正不要で合格、本タスクの完了条件（S7 合格エビデンスの final-launch.md 記録）を充足。
