# 設計バリデーション: wintf-winmsg-executor

> 本レポートは確定済み design.md / requirements.md / research.md ＋ steering ＋ 先進坑 README
> （`crates/pilot/examples/wintf-winmsg-executor/README.md`・検証事実の正本）に対する
> 非対話バリデーションである。確定済み議題①（Option B = `WinThreadMgr` 全撤去＋新 facade・consumer 全面追従）
> ②（R3 = UI スレッド async のみ移行・`WintfTaskPool` 温存）は固定制約として扱い再検討しない。

## 設計レビュー要約

本設計は「ライブラリ委譲＋薄いアダプタ層（`WinApp` facade）」という適切なパターンを選び、要件 1.1〜7.4 を
トレーサビリティ表で全件被覆し、各論点（block_on 終了規律・CS_DBLCLKS 補填・Entity 配送のクロージャ化・
tick 再入二重防御）に対し先進坑 README の実証事実を根拠として明確な判断を下している。判断の確度・境界
分離・撤去計画の機械性はいずれも実装着手に十分なレベルにある。残る論点は実装フェーズで解消可能な粒度
（ハンドラ再配線の機械的範囲・終了 future への入力経路・初期化順序）であり、構造的な設計矛盾は検出されない。

## Critical Issues（最大 3 件・設計ディスカッションへ送る）

### 🟡 Critical Issue 1: `dispatch_window_message` への移設に伴う各ハンドラの World/Entity 自己解決の再配線範囲が過小評価されうる

- **Concern**: 既存ハンドラ（`window_proc/*.rs` 7 ファイル）は内部で `try_get_ecs_world()` ＋
  `get_entity_from_hwnd(hwnd)` を**計 31 箇所**自己呼び出しして World と Entity を取得している
  （`crates/wintf/src/ecs/window_proc/` 実測）。設計は `dispatch_window_message(world, entity, msg)` で
  world/entity を外から渡す形へ寄せると述べ、かつ「ハンドラのシグネチャは `(hwnd, message, wparam, lparam)`
  を維持できる」「Entity を要するハンドラのみ薄く改修」と記す。この 2 記述は緊張関係にあり、グローバル
  （`ECS_WORLD: OnceLock<SendWeak>` / `get_entity_from_hwnd`）撤去後は **31 箇所すべての解決点を
  引数経由へ書き換える必要**があるため「薄い改修」の範囲が曖昧。
- **Impact**: 撤去（要件 5.1/5.2）と新配送（要件 2.3/2.4）の交点。再配線漏れがあると「旧 API 参照を
  残さずビルド成功」（5.2）が満たせず、配送同等性（2.4）の単体テストも書けない。
- **Suggestion**: 実装フェーズで「ハンドラへ `(world: &Rc<RefCell<EcsWorld>>, entity: Entity, msg)` を
  渡す統一シグネチャ」へ機械的に揃える方針を tasks 化し、31 箇所の解決点を引数置換対象として明示列挙する。
  設計文の「シグネチャ維持可能」記述は「Entity 引数を 1 つ足す薄い改修」へ表現を一段具体化すると齟齬が消える。
- **Traceability**: 2.3, 2.4, 5.2
- **Evidence**: design.md「EntityWndprocBridge / Implementation Notes」「Modified Files: window_proc/mod.rs」、
  実コード `crates/wintf/src/ecs/window_proc/mod.rs:42-102` ＋ 31 箇所の自己解決呼び出し。

### 🟡 Critical Issue 2: shutdown future を発火する `App::on_window_destroyed` の発火経路の所在が未確定（message_window 撤去との依存順）

- **Concern**: 終了規律は「`run()` が `block_on(shutdown_signal.await)`、最後のウィンドウ破棄で
  `App::on_window_destroyed` が `event_listener::Event` を notify → future 完了 → 正常復帰」と確定。
  一方で `App::on_window_destroyed` は現状 message_window へ `WM_LAST_WINDOW_DESTROYED` を PostMessage
  する ECS 側コードであり、設計は同時に「message_window / `set_message_window` を撤去候補」とする。
  shutdown_signal の `Event` を **app.rs（ECS 層）から誰が capture して渡すか**（`WinApp` が生成した
  `Event` をどの経路で ECS 側へ届けるか）が「event_listener か `Rc<Cell<bool>>` か」の二択提示に留まり
  確定していない。依存方向（COM→ECS→Message を厳守・ECS から上向き facade 依存を作らない）の制約下で、
  ECS 層が facade の `Event` を握る配線は設計上の要注意点。
- **Impact**: 要件 1.3/1.4/1.5 の中核。配線を誤ると（a）panic 回避が崩れる、または（b）ECS→facade の
  上向き依存が混入し Boundary Commitments（依存方向）に反する。
- **Suggestion**: shutdown_signal を `EcsWorld` の Resource（または `App` フィールド）として ECS 層に
  保持し、`WinApp` 構築時に注入（下向き注入＝依存方向を保つ）する形を設計ディスカッションで一案として
  確定する。`Rc<Cell<bool>>`＋tick タスク polling は await 不能で tail race 補填と相性が悪いため
  `event_listener::Event` 推奨の現記述を「確定」へ格上げする。
- **Traceability**: 1.3, 1.4, 1.5, 6.3
- **Evidence**: design.md「ShutdownPolicy / Responsibilities・Implementation Notes」「Open Questions:
  message_window の要否」、research.md「Decision: 終了規律＝shutdown future を block_on で待つ」。

### 🟡 Critical Issue 3: CS_DBLCLKS 補填の「初回ウィンドウ生成直後 1 回」タイミングの確実性に依存

- **Concern**: `DblClkClassFixup` は「最初のウィンドウ生成後 `SetClassLongPtrW(hwnd, GCL_STYLE, cur|CS_DBLCLKS)`
  をプロセス共有クラスへ 1 回」適用する。これは「補填前に生成された最初のウィンドウ自身」も含め全窓へ
  クラス style 変更が波及する前提だが、CS_DBLCLKS はクラス単位で OS のダブルクリック合成を制御するため、
  補填**前**に到達したそのウィンドウの初回クリック系列でダブルクリックが取れない瞬間が理論上ありうる。
  areka はダブルクリック終了がプライマリ UX（structure.md）であり、初回ウィンドウ＝シェル窓である。
- **Impact**: 要件 2.5/6.1 の回帰条件（ダブルクリック終了の実証）。実害は「起動直後の極初回ダブルクリック
  のみ不発」という軽微なものに留まる見込みだが、設計が「リスク Low」と断ずる根拠（OS がクラス style 変更を
  即時反映するか／生成直後即補填で間に合うか）は静的解析では未確認。
- **Suggestion**: 補填を「ウィンドウクラス登録の直後（＝最初の `util::Window` 生成のさらに前段で、
  ライブラリの `Once` 登録完了を観測できる地点）」へ寄せられるか実装フェーズで検証する。困難なら
  「初回ウィンドウ生成より前にダミー MessageOnly 窓で 1 回クラスを実体化し補填」する案を代替として
  research.md に追記。フォールバック（wndproc 内自前検出・既記録）への退避基準も明文化する。
- **Traceability**: 2.5, 6.1
- **Evidence**: design.md「DblClkClassFixup / Responsibilities・Risks」「Open Questions: CS_DBLCLKS 補填方式」、
  先進坑 README（CS_DBLCLKS 検証は先進坑のスコープ外＝本設計固有の未実証点）。

## 設計の強み

1. **判断の根拠が先進坑実証で裏付けられている**: block_on panic 規律（`expect("received unexpected quit message")`）、
   `new_checked_ex` の RefCell が nested 719 回で `reentry_body_ran=false`/`double_tick=false`、coverage 99.7%、
   `Pin<&S>` 経由 state 到達（GWLP_USERDATA 全廃）が README で実測済みであり、設計の主要判断（終了規律・
   tick 二重防御・配送のクロージャ化）が思弁でなく検証事実に接地している。二重化せず README を正本参照する
   規律（要件 7.4）も守られている。
2. **境界分離と撤去計画の機械性**: 「ライブラリ委譲で済む処理（pump/状態保持/wake/モーダル/wndproc 再入防止）」と
   「wintf 固有結線（Entity 配送/13 本 tick/終了規律/CS_DBLCLKS）」を明確に切り分け、新設 `runtime/` を
   レガシーと物理分離。Removed/Modified Files と 8 段階 Migration が列挙され、要件 5（撤去）の機械的遂行が
   担保されている。依存方向（COM→ECS→Message）・`unsafe` 局所化・tokio 非依存も steering と整合。

## 最終判定

- **判定: GO**
- **Rationale**: 要件 1.1〜7.4 を全件被覆し、主要論点は先進坑の実測事実に接地した確定判断で閉じている。
  検出した 3 件はいずれも構造的矛盾でなく実装フェーズで解消可能な配線・範囲・タイミングの具体化要求であり、
  許容リスクの範囲内（議題①②は固定制約として尊重済み）。
- **Next Steps**:
  1. 設計ディスカッション（kiro-design-discussion）で上記 3 件を確認・解消方針を合意（特に Issue 2 の
     shutdown_signal 注入方向と Issue 1 のハンドラ統一シグネチャ）。
  2. `/kiro-spec-tasks wintf-winmsg-executor` でタスク化。ハンドラ 31 箇所の引数置換・CS_DBLCLKS 補填の
     タイミング検証・shutdown future 注入経路を明示タスクへ分解する。
  3. 実ビルド/実行検証は実装フェーズ（vendors/pasta submodule populate 後）で実施。

---
判定日: 2026-06-29 ／ 対象: design.md（FINALIZED）／ モード: 非対話静的解析（workspace cargo 未実行）
