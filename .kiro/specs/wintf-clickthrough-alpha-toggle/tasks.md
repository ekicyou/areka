# Implementation Plan

> 着手前規律（R6.5）: 既存コードへ触れるタスク（1.1／3.2／4.1）は、編集前に「変更対象ファイルと変更内容」を依頼者へ提示し確認を得てから着手する。追加 ex-style・`WM_NCHITTEST` ハンドラ・依存追加が必要と判断した場合は独断で入れず理由を添えて確認する（R6.4）。

- [ ] 1. Foundation: ex-style 動的トグル API とモジュール骨格
- [x] 1.1 `WS_EX_TRANSPARENT` 動的トグル最小 API の実装
  - 対象 HWND の `WS_EX_TRANSPARENT` ビットのみを引数フラグへ一致させ、`SetWindowLongPtr(GWL_EXSTYLE)` ＋ `SetWindowPos(SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)` で反映する（`apply_initial_state` のレシピ準拠）
  - `WS_EX_LAYERED` は操作しない・`WM_NCHITTEST`→`HTTRANSPARENT` ハンドラは追加しない。既存 `WinStyle::commit`・既存ビルダーは不変
  - ユニットテスト: 適用後に現在 ex-style を読み戻し、`WS_EX_TRANSPARENT` のみが変化し `WS_EX_NOREDIRECTIONBITMAP` 等の他ビットが保存されること（観測可能な完了条件）
  - _Requirements: 1.2, 3.3, 6.1, 6.2, 6.3, 6.5_
  - _Boundary: ExStyleToggle_
- [x] 1.2 (P) クリック透過モジュール骨格と監視対象レジストリ
  - `ecs/clickthrough/` を新設し `ecs` へ宣言追加。ビルドが通ること
  - 監視対象（window Entity ＋ 対応 HWND ＋ `last_applied` 状態）を保持するレジストリを実装。初期 `last_applied` は不透過（`Opaque`）
  - 観測可能な完了条件: レジストリへ対象窓を登録・除去でき、`last_applied` の既定が `Opaque` になっているユニット確認
  - _Requirements: 1.5, 3.2_
  - _Boundary: ClickThroughRegistry_

- [ ] 2. Core: カーソル監視ワーカと判定ロジック
- [x] 2.1 (P) カーソル監視ワーカ（別スレッド・`event_listener` 起床・RAII）
  - 専用ワーカスレッドで `GetCursorPos`（screen physical）を継続取得し、カーソル移動時のみ `event_listener::Event` で UI スレッドを起床する。`&World` は触れない
  - 最新座標を原子的に保持し、`latest_pos.store(...)` → `event.notify(...)` の順序を厳守（逆順による座標遅延レース回避）。tokio・外部 async ランタイム不使用
  - `Arc<AtomicBool>` stop_flag ＋ `Drop` で `stop→join` の RAII（`VsyncEventBridge` 準拠）
  - 観測可能な完了条件（統合テスト）: `spawn`→`drop` でワーカが確実に stop/join され、カーソル移動で notify が発火し UI 側が最新座標を読めること
  - _Requirements: 3.1, 3.4, 4.1, 4.2_
  - _Boundary: CursorMonitorBridge_
  - _Depends: 1.2_
- [x] 2.2 (P) 状態遷移判定の純関数（差分ガード・ドラッグ抑止）
  - ヒット結果（`Option<Entity>`）・ドラッグスナップショット・`last_applied` から「今回適用すべき変化」を返す World 非依存の純関数を実装
  - `Some`→不透過／`None`→透過の写像、`last_applied` と同一なら適用なし（差分ガード）、ドラッグ中は透過 ON へ遷移させない、`JustEnded` 観測で抑止解除し再収束
  - 観測可能な完了条件（ユニットテスト網羅）: 差分ガード・ドラッグ抑止・`JustEnded` 再収束の各ケースが期待どおり判定されること
  - _Requirements: 3.2, 3.3, 5.1, 5.2, 5.3_
  - _Boundary: ClickThroughController_
  - _Depends: 1.2_

- [ ] 3. Integration: 判定ループ結線と起動フック
- [x] 3.1 UI スレッド判定・適用ループの結線（二重起床・post-tick 評価）
  - `spawn_local` の async ループを UI スレッドで駆動。起床契機はカーソル移動 notify と既存 VSync tick の二重化。listen-before-work 規律を踏襲
  - 評価は当該フレームの ECS tick 完了後（post-tick・`GlobalArrangement`／`AlphaMask`／`DragState` 確定後）に実行。レジストリの各対象窓についてワーカ最新カーソル座標を窓クライアント座標へ変換し `hit_test_in_window` を呼ぶ（座標変換は既存経路へ委譲）
  - 判定結果で desired を決め、`last_applied` と異なる時のみトグル API を 1 回適用し、適用成功後にレジストリへ書き戻す単一経路。表示層（GPU 合成）は変更しない
  - 観測可能な完了条件（統合テスト）: 起床（notify／tick いずれも）→ post-tick で `hit_test_in_window` が呼ばれ、変化時のみ `apply_click_through` が発火。静止カーソルでも tick 起床で表示更新に追随
  - _Requirements: 1.1, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.3, 4.3, 5.2, 8.1, 8.2, 8.3_
  - _Boundary: ClickThroughController_
  - _Depends: 1.1, 2.1, 2.2_
- [x] 3.2 WinApp 起動フックと窓ライフサイクル結線
  - `runtime` の結線点で機構（ワーカ生成・event 共有・async ループ）を起動し、監視対象窓を登録する最小フックを追加。既存 tick/vsync 結線に相乗り
  - 窓破棄時にレジストリから除去。shutdown（World 失効）で async ループ終了・ワーカ join。既存の透過・ヒットテスト・ウィンドウ管理を破壊しない
  - 観測可能な完了条件: `WinApp` 起動でワーカ＋判定ループが稼働し、機構ハンドル drop（shutdown）で停止・join されること。窓破棄で対象から外れ適用がスキップされること
  - _Requirements: 7.2, 1.5, 6.4, 6.5_
  - _Boundary: runtime wiring_
  - _Depends: 3.1_

- [ ] 4. areka 実効化・検証・ドキュメント
- [x] 4.1 areka の WUC 化と機構登録
  - shell 窓・balloon 窓を `CompositionMode::DComp`（WUC）へ切替え（`ex_style` は factory の `compute_ex_style` が自動計算ゆえ変更不要）、両 window Entity を機構へ登録する
  - wintf ライブラリの ULW バックエンドは残置（本坑では areka のみ WUC 化）。後続 `wintf-ulw-removal` が areka を巻き込まない状態を作る
  - 観測可能な完了条件: areka がビルドでき、起動で shell/balloon が GPU 合成（WUC）経路のマスコットとして表示され、機構へ登録されていること
  - _Requirements: 1.5, 7.1, 7.4, 6.4, 6.5_
  - _Boundary: crates/areka_
  - _Depends: 3.2_
- [ ] 4.2 areka 実動検証（透過・座標一致・ドラッグ安定）
  - 透明領域クリック→背面プロセスへ透過、キャラ領域クリック→areka が受領を目視確認
  - 高 DPI 150% ＋ マルチモニタ（異倍率）＋ ウィンドウ移動で、見た目のキャラ領域と当たり判定領域が一致すること。キャラ不透明部を掴んでのドラッグ中に透過が入らず、終了後に再収束すること
  - 観測可能な完了条件: 上記検証項目のチェックリストが実マスコットで全て満たされること
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 5.1, 5.2, 8.1, 8.2, 8.3, 7.2_
  - _Depends: 4.1_
- [x] 4.3 (P) リリースビルド互換・依存最小検証
  - 機構込みでリリース最適化（`opt-level='z'`, `lto=true`）ビルド・動作すること
  - 新規依存を追加せず依存最小を維持すること（`Cargo.toml` 差分なし）
  - 観測可能な完了条件: リリースビルド成果物（`target/release/areka.exe`）が生成されること
  - i686（32bit）ビルドは本機構の検証対象外（開発者指示 2026-07-02）: 本機構は areka／wintf 本体（x64＋arm64 ネイティブ）側で動作し、i686 は SHIORI helper 隔離トラックゆえ。詳細は Implementation Notes 参照
  - _Requirements: 9.1, 9.2_
  - _Boundary: build config_
  - _Depends: 4.1_
- [x] 4.4 (P) `docs/click_through.md` の作成
  - 二層分離（表示層／当たり判定層）の概要、`WS_EX_TRANSPARENT` 動的トグル＋カーソル監視＋シーングラフ・ヒットテスト連動の流れ、不採用理由（ULW／`HTTRANSPARENT`／Layered 描画）、API 使用例、既知の制約（`SWP_FRAMECHANGED` 副作用・ポーリング周期・ドラッグ抑止）を記す
  - ULW 撤去確定時に更新すべき対象（`tech.md`「ULW 一択」相当記述・`roadmap.md`・正本 `doc/COMPAT_ARCHITECTURE.md`）を申し送りとして明示（本坑では実更新しない）
  - 観測可能な完了条件: `docs/click_through.md` が新規に存在し、上記 5 要素と不採用理由（`HTTRANSPARENT` 含む）・更新対象申し送りを含むこと
  - _Requirements: 10.1, 10.2, 10.3, 7.3, 6.3_
  - _Boundary: docs_
  - _Depends: 3.1_

## Implementation Notes

- **`WS_EX_LAYERED` 同伴フラグは透過成立の必須条件（4.2 実動検証で発覚・2026-07-02 依頼者確認済み＝R6.4 充足）**: 初回実動検証でクリック透過が不成立。原因は design が pilot 知見を誤転写していたこと——pilot REPORT（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/REPORT.md` L15/L18/L68）は「`WS_EX_TRANSPARENT` 単独では DComp 窓のマウス透過が効かず窓が全クリックを吸う。`WS_EX_LAYERED` をフラグのみ（ULW/SLWA 非呼出）併設して成立（実測 ex_style 0x280028 で DComp 描画と共存）」と実証していたが、design は「単独で成立する pilot 実証済み」と逆に記載していた。修正: `win_style::apply_layered_companion`（LAYERED を立てるのみ・冪等）を新設し、`evaluate_targets` が登録窓の初回評価で 1 回適用（`ClickThroughTarget.layered_applied` フラグ・成功時のみ真・失敗は次サイクル再試行）。factory `compute_ex_style` は byte-for-byte 契約ゆえ不変。design/docs の該当記述は訂正済み。**教訓: pilot の REPORT.md（実測台帳）を正とし、spec 転写を鵜呑みにしない**。

- `PhysicalPoint` は文脈で2型ある: `crate::ecs::PhysicalPoint`（=`Point`・**i32**、cursor/registry/drag 用）と `crate::ecs::layout::hit_test::PhysicalPoint`（=`PointF`・**f32**、`hit_test_in_window` の引数型）。座標変換は i32 のまま `client = cursor_screen - WindowPos.position` を計算し、`hit_test_in_window` 呼び出し時に `PointF::new(x as f32, y as f32)` へ明示キャストする（符号: client = cursor − position）。3.1 で確認済み。
- クリック透過機構の結線 API（3.1 確定）: `ClickThroughController::start(world: Weak<RefCell<EcsWorld>>, registry: Rc<RefCell<ClickThroughRegistry>>, wake_event: Arc<event_listener::Event>) -> ClickThroughHandle`。二重起床は**単一の共有 `Arc<Event>`** を cursor worker と VSync tick 源の両方が notify する方式（select 併用なし）。`CursorMonitorBridge::spawn(wake_event.clone())` で worker が同一 event を叩く。tick 源が wake_event を post-tick で notify する結線は **3.2 の領分**。registry は `Rc<RefCell<..>>` 共有ゆえ start 後に窓を register/remove 可能（handle 経由）。
- `last_applied` 単一所有: 書き戻しは `apply_click_through` が `Ok` の時のみ（`Err` は据え置き＋`warn!`・次サイクル再試行）。
- cargo は **PowerShell 必須**（Git Bash の coreutils `link.exe` が MSVC link を遮蔽）。worktree では `git submodule update --init`（vendors/pasta）済み。
- **32bit/i686 検証はスコープ外（4.3・開発者指示 2026-07-02）**: 本機構は areka／wintf 本体（**x64＋arm64 ネイティブ**）の UI スレッド上で動作し、i686（32bit）は x86 SHIORI 駆動 helper のみに隔離されるトラックゆえ本機構は含まれない。当初 tasks/requirements が steering roadmap の全体制約「32bit 可搬性を崩さない」を誤って本機構の i686 ビルド完走検証へ転写していたため、requirements Req9.2（32bit）を削除し 9.3→9.2（依存最小）へ繰り上げ、design/tasks/brief/research を整合。**参考実測**: 検証時 `cargo build -p wintf --target i686-pc-windows-msvc` は既存の `crates/wintf/src/api.rs`（`Get/SetWindowLongPtrW` の `isize` 契約 vs i686 の `i32`）由来 E0308 で失敗するが、これは本坑の変更前から存在する wintf 共有基盤の既存非互換であり本機構の回帰ではない（本坑差分に api.rs は含まれない）。steering roadmap の 32bit 制約は host-32／shiori-abi トラックには有効ゆえ steering は変更しない。
- 4.3 完了エビデンス（release/依存最小）: `cargo build --release -p areka` exit 0・`target/release/areka.exe`（2,497,536 bytes）生成・`opt-level='z'`/`lto=true`/`codegen-units=1` 適用確認（Req9.1）。本坑差分は clickthrough／areka のみで `Cargo.toml` 差分なし＝新規依存ゼロ（Req9.2）。GUI 実起動は headless 環境ゆえ未実施（実動確認は 4.2 の領分）。
- areka（4.1）向け登録面（3.2 確定）: `wintf::ecs::clickthrough::ClickThroughRegistryHandle`（**pub** NonSend リソース・`run()` で World へ挿入済み）を `world.get_non_send_resource::<ClickThroughRegistryHandle>()` で取得し `register(entity, hwnd)` する。eval ループと**同一 `Rc<RefCell<ClickThroughRegistry>>`** を共有ゆえ登録は即反映。窓破棄は `prune_dead_targets`（entity 存在判定・`evaluate_targets` 冒頭で自動除去）が処理するので areka 側の明示 remove は必須でない（despawn で十分）。
