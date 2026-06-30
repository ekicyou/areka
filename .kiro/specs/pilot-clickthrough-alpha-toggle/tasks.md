# Implementation Plan

- [x] 1. Foundation: example 雛形と DPI 初期化
- [x] 1.1 `_template` から example を作成し PMv2 DPI 認識を設定した最小起動骨組み
  - 前段準備: worktree では `git submodule update --init --recursive` を実行後に `cargo build` が通ること（`wintf-winmsg-executor` 0.0.5・workspace `windows` features の依存解決）。これが満たせないと本タスクの成果物に到達できない
  - `crates/pilot/examples/_template/` の `main.rs`・`README.md` を `crates/pilot/examples/pilot-clickthrough-alpha-toggle/` へコピーして着手する
  - `main` 冒頭で `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` を呼び、失敗時は警告ログを出す
  - 葉ノード隔離を厳守: コードは `examples/` 配下のみ、他クレートからの inbound 依存（`pilot = { path = ... }`）を作らない、production／`pilot/lib.rs`／`pilot/Cargo.toml` を変更しない、新規依存を追加しない
  - 観測: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle` が起動し DPI 認識設定のログが出力され、`cargo build` が成功する
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 7.1, 10.1, 10.3_

- [ ] 2. Core: 判定純関数と透過窓
- [x] 2.1 (P) αマスク純関数（窓中心・半径 200px 円・物理座標）
  - `alpha_is_opaque(cursor, win_rect)` を副作用なしの純関数として実装する
  - 窓矩形の中心を中心とした半径 200px（物理ピクセル）の円の内側を不透明（クリックスルー OFF）、外側を透明（クリックスルー ON）と判定する
  - 固定スクリーン座標（旧 (960,540)）を前提にせず、円中心を窓矩形から実算出し、カーソル物理座標と同一基準で比較する（差し替えシーム＝将来は本坑が実描画αバッファ参照に置換）
  - 実描画αバッファ参照は実装しない
  - 観測: 代表入力（円中心・円周内外）で円内 true／円外 false を返すことを in-source の簡易確認で示す
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 7.2, 7.3_
  - _Boundary: AlphaMask_
  - _Depends: 1.1_

- [ ] 2.2 (P) 透過トップモスト窓の生成・円描画・クリック受領
  - `Window::new_ex(WindowType::TopLevel, WINDOW_EX_STYLE(WS_EX_TRANSPARENT | WS_EX_TOPMOST), state, wndproc)` で全域透明のトップモスト窓を生成する。`WS_EX_LAYERED` は付けない、`WM_NCHITTEST` は wndproc に書かない（自前ハンドルしない）
  - `WM_PAINT` で窓中心に半径 200px の円を GDI で描画する（描画円は AlphaMask の判定円と同一領域）
  - `WM_LBUTTONDOWN` を受領してログ出力し、円の色をトグル変更して再描画する
  - R2.3（`WS_EX_TRANSPARENT` 単独）は本タスクでは「生成時に当該 ex_style が付与されている」静的表明まで。プロセス越え透過の成否（T2/T6）の本検証は 5.2 で実施する
  - 観測: 起動すると全域透明＋中央円が表示され、円をクリックするとログが出て色が変わる
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 6.1, 6.2, 6.3_
  - _Boundary: TransparentWindow_
  - _Depends: 1.1_

- [ ] 3. Core: トグル機構（判定／適用の責務分離）
- [ ] 3.1 カーソル監視ワーカ（16ms・別 std::thread・判定・desired 公開・変化時 notify）
  - UI とは別の `std::thread` で 16ms 周期に `GetCursorPos`＋`GetWindowRect` を呼び、`alpha_is_opaque` で判定する。HWND は生値（`isize`）で受け取りワーカ内で再構成し、読み取り専用 API のみ呼ぶ（スタイル変更はしない＝`unsafe impl Send` ラッパ不要）
  - 望ましいクリックスルー状態を `AtomicBool desired_passthrough`（円外=ON／円内=OFF）で公開し、前回から変化したとき（および初回）だけ `event_listener::Event` で UI を起床する
  - `done: AtomicBool` が立ったらループを抜けて正常終了する。tokio は使わない（`event_listener`＋`std::thread`）
  - 観測: 稼働中にカーソルを円内外へ動かすと `desired_passthrough` の反転がログで確認でき、`done` セットでワーカが停止する（実際の透過の成否は 3.2／5.2 で検証）
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 5.1, 5.2, 10.2_
  - _Boundary: CursorWorker_
  - _Depends: 2.1, 2.2_

- [ ] 3.2 状態変化適用タスク（UI スレッド・差分時のみスタイル適用＋ログ）
  - `spawn_local` の async タスクで `event.listen().await` 起床後、`desired_passthrough` を読みローカル `applied` と比較する
  - 差分があるときのみ `SetWindowLongPtr(GWL_EXSTYLE)`＋`SetWindowPos(SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)` で `WS_EX_TRANSPARENT` を加除する。差分が無い間はスタイル適用 API を呼ばない
  - 切替時に「ON→OFF」「OFF→ON」と座標をログ出力する
  - 観測: カーソルが円境界をまたぐと ON↔OFF の切替ログが出て、円内／円外に留まる間は追加の `SetWindowPos` 呼出もログも出ない
  - _Requirements: 5.3, 5.4, 5.5_
  - _Boundary: StateApplier_
  - _Depends: 3.1_

- [ ] 4. Integration: 起動初期状態とライフサイクル
- [ ] 4.1 起動時初期状態確定とライフサイクル結線
  - 窓を初期 ex_style = `WS_EX_TRANSPARENT | WS_EX_TOPMOST`（クリックスルー ON）で生成し、UI 側 `applied` 初期値を ON に一致させる
  - 初回 notify 取りこぼし防止のため、UI で `event.listen()` を確立してからワーカを spawn する（または UI 起動直後に desired を一度ポーリングして初回適用する）。ワーカは初回を無条件判定とする＝StateApplier（3.2）の差分適用が起動時カーソル円内でも正しく初回 OFF へ収束する
  - 窓クローズ→`block_on` の future 完了→`done` を `store(true)`→ワーカ join で正常終了する（最終 notify で UI 側 listen を確実に解除）
  - 観測: 起動時にカーソルが円内でも初回 1 回だけ OFF が適用され、窓を閉じるとプロセスとワーカスレッドが正常終了する（タスクマネージャでスレッドが残らない）
  - _Requirements: 8.1, 8.2_
  - _Depends: 3.1, 3.2_

- [ ] 5. Validation: 一次記録と手動検証
- [ ] 5.1 (P) REPORT.md テンプレートと README 3 幕の整備
  - `REPORT.md` を design 定義のフォーマットで作成する（検証日／実行コマンド／環境、T1〜T8 合否・証跡台帳、必須合格基準 T1・T2・T3・T4・T6 の充足欄、人間が記入する総合判定 go／違う／直す＋理由・学び）
  - `README.md` を 3 幕で整える（動機＝対応本坑 `wintf-clickthrough-alpha-toggle` を名指し、概要＝実行法 `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`、検証結果＝判定欄）
  - 観測: `REPORT.md` と `README.md` が所定フォーマットで存在し、T1〜T8 の空台帳と実行コマンドが揃っている
  - _Requirements: 1.3, 9.5_
  - _Boundary: README.md, REPORT.md_

- [ ] 5.2 T1〜T8 の人間との手動検証と記録
  - 手順「人間の準備確認 → エージェントが `cargo run -p pilot --example pilot-clickthrough-alpha-toggle` を起動 → 結果のヒアリング」で T1〜T8 を手動検証する（背面アプリ＝デスクトップアイコン／ブラウザを用意、高 DPI 150% 構成も確認）
  - T1〜T8 の合否・証跡を `REPORT.md` に記入する。必須 T1・T2・T3・T4・T6、条件付き可 T5・T7・T8。Win32 API/挙動の不確実点に遭遇したら推測せず開発者に質問する
  - go／違う／直す の総合判定は開発者（人間）が下す。Claude Code 単独で合格判定して次フェーズに進まない
  - 観測: `REPORT.md` に T1〜T8 の合否・証跡が埋まり、必須合格基準の充足が判定され、人間が総合判定（go／違う／直す）を記入している
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.6, 10.4_
  - _Depends: 4.1, 5.1_
