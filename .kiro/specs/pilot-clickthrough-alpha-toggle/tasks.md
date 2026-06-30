# Implementation Plan

- [ ] 1. Foundation: 既存実装の撤去と DComp 前提の最小起動骨組み
- [x] 1.1 既存実装コード（GDI 時代の main.rs 中身）の削除
  - `crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs` から旧設計（GDI 描画前提）の実装を撤去する。具体的には `init_dpi_awareness`／`alpha_is_opaque`／`const RADIUS`／`#[cfg(test)] mod tests`／旧 `use` を削除し、プレースホルダ `fn main() {}` のみへ戻す
  - 旧 doc コメントの「2.2 の GDI 描画円」等 GDI 前提の記述も削除する（後続 1.2 で DComp 前提の骨組みを置き直す）
  - `README.md`（一次記録）は残す（整備は 6.1）。`REPORT.md` はまだ存在しない（6.1 で新規作成）ので触れない
  - 観測: `main.rs` に `init_dpi_awareness`／`alpha_is_opaque`／`#[cfg(test)]`／`RADIUS` の grep ヒットが 0 件になり、`cargo build -p pilot --example pilot-clickthrough-alpha-toggle` が（空 main で）成功する
  - _Requirements: 1.1, 1.5_

- [x] 1.2 `_template` から再 scaffold し PMv2 DPI 認識を設定した最小起動骨組み
  - 前段準備: worktree では `git submodule update --init --recursive` を実行後に `cargo build` が通ること（`wintf-winmsg-executor` 0.0.5・workspace `windows` features の依存解決）。これが満たせないと本タスクの成果物に到達できない
  - `crates/pilot/examples/_template/main.rs` を雛形として `crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs` を DComp 前提で置き直す（1.1 で削除した中身の代わり）
  - `main` 冒頭で `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` を呼び、失敗時は警告ログを出す
  - 葉ノード隔離を厳守: コードは `examples/` 配下のみ、他クレートからの inbound 依存（`pilot = { path = ... }`）を作らない、production／`pilot/lib.rs`／`pilot/Cargo.toml` を変更しない、新規依存を追加しない（DComp に必要な `windows` features は workspace で既に有効）
  - 観測: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle` が起動し DPI 認識設定のログが出力され、`cargo build` が成功する
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 7.1, 10.1, 10.3_
  - _Depends: 1.1_

- [ ] 2. Core: αマスク判定純関数
- [x] 2.1 (P) αマスク純関数（窓中心・半径 200px 円・物理座標）
  - `alpha_is_opaque(cursor, win_rect)` を副作用なしの純関数として実装する
  - 半径を `const RADIUS: i32 = 200` として定義し公開する。この定数は 3.2 の DComp 描画円と共有し、「見た目の円」と「判定の円」を同一領域に保つ（R2.2／R4.1）
  - 窓矩形の中心を中心とした半径 200px（物理ピクセル）の円の内側を不透明（クリックスルー OFF）、外側を透明（クリックスルー ON）と判定する
  - 固定スクリーン座標（旧 (960,540)）を前提にせず、円中心を窓矩形から実算出し、カーソル物理座標と同一基準で比較する（差し替えシーム＝将来は本坑が実描画αバッファ参照に置換）。実描画αバッファ参照は実装しない
  - 観測: 代表入力（円中心・円周内外・非対称矩形・負座標）で円内 true／円外 false を返すことを単体テストで示す
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 7.2, 7.3_
  - _Boundary: AlphaMask_
  - _Depends: 1.2_

- [ ] 3. Core: DComp 透過窓（NOREDIRECTIONBITMAP・視覚透過）
- [x] 3.1 (P) NOREDIRECTIONBITMAP トップモスト透過窓の生成と最小ライフサイクル
  - `Window::new_ex(WindowType::TopLevel, WINDOW_EX_STYLE(WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TRANSPARENT), state, wndproc)` で窓を生成する（初期状態＝クリックスルー ON）。`WS_EX_LAYERED` は付けない、`WM_NCHITTEST` は wndproc に書かない（自前ハンドルしない）
  - wndproc は `Fn`（`FnMut` 不可）。内部可変は `Cell` で持つ。`WM_CLOSE` を受領してシャットダウン event を notify し、`block_on(async{..})` の await 中の future を完了させる（lib は `WM_CLOSE` を握り `DestroyWindow` しない＝アプリ側で終了シグナルを出す。窓破棄は `Window` の Drop が担う）
  - 本タスクは「窓ローカルなクローズ機構（block_on ループ＋WM_CLOSE→event→future 完了）」までを担う。ワーカ join・初回状態収束の完全結線は 5.1（カーソルワーカは 4.1 で初めて存在するため本タスクでは join できない）
  - R2.3（`WS_EX_TRANSPARENT` 単独支配）は本タスクでは「生成時に当該 ex_style が付与されている」静的表明まで。プロセス越え透過の成否（T2/T6）の本検証は 6.2 で実施する
  - 観測: 起動すると（描画はまだ無いので）透明窓が生成され、`WM_CLOSE`（窓を閉じる）でプロセスが正常終了する
  - _Requirements: 2.1, 2.3, 2.4, 2.5_
  - _Boundary: TransparentWindow_
  - _Depends: 1.2_

- [x] 3.2 DComp パイプライン構築と初回描画（透明クリア＋不透明円）
  - D3D11CreateDevice(BGRA)→DXGI factory→`IDXGIFactory2::CreateSwapChainForComposition`（premultiplied alpha）→`DCompositionCreateDevice`→`CreateTargetForHwnd(hwnd, topmost)`→`CreateVisual`→`SetContent(swapchain)`→`SetRoot`→`Commit` の visual tree を構築する
  - back buffer に Direct2D で描画する: `Clear`(透明 α=0)→窓中心・半径 `RADIUS`(=200) の `FillEllipse`(不透明 α=1)。描画円は AlphaMask（2.1）の判定円と同一定数・同一中心算出を共有する（R2.2／R4.1）。`Present`＋`Commit`
  - GDI／`WM_PAINT`／`InvalidateRect`／DWM extend-frame glass は描画経路に使わない（NOREDIRECTIONBITMAP 窓は redirection surface を持たず GDI/glass は画面に出ない＝DComp visual tree が唯一の描画手段）
  - 本タスクは設定単体では何も観測できない（NOREDIRECTIONBITMAP ゆえ円描画まで画面は空）ため、パイプライン構築と初回描画を 1 つの観測可能単位とする
  - 観測: 起動すると背景透明・中央に不透明円が表示される
  - _Requirements: 2.2, 7.2_
  - _Boundary: TransparentWindow_
  - _Depends: 3.1, 2.1_

- [ ] 3.3 クリック受領による色トグルと DComp 再描画
  - `WM_LBUTTONDOWN` を受領してログ出力し、円の塗り色をトグル変更して DComp 経路で再描画する（D2D 再 `FillEllipse`＋`Present`＋`Commit`）
  - 再描画に GDI／`WM_PAINT`／`InvalidateRect` を用いない（DComp visual tree のみ）
  - 観測: 中央円をクリックするとログが出て円の色が変わる
  - _Requirements: 6.1, 6.2, 6.3_
  - _Boundary: TransparentWindow_
  - _Depends: 3.2_

- [ ] 4. Core: トグル機構（判定／適用の責務分離）
- [ ] 4.1 カーソル監視ワーカ（16ms・別 std::thread・判定・desired 公開・変化時 notify）
  - UI とは別の `std::thread` で 16ms 周期に `GetCursorPos`＋`GetWindowRect` を呼び、`alpha_is_opaque` で判定する。HWND は生値（`isize`）で受け取りワーカ内で再構成し、読み取り専用 API のみ呼ぶ（スタイル変更はしない＝`unsafe impl Send` ラッパ不要）
  - 望ましいクリックスルー状態を `AtomicBool desired_passthrough`（円外=ON／円内=OFF）で公開し、前回から変化したとき（および初回）だけ `event_listener::Event` で UI を起床する
  - `done: AtomicBool` が立ったらループを抜けて正常終了する。tokio は使わない（`event_listener`＋`std::thread`）
  - 観測: 稼働中にカーソルを円内外へ動かすと `desired_passthrough` の反転がログで確認でき、`done` セットでワーカが停止する（実際の透過の成否は 6.2 で検証）
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 5.1, 5.2, 10.2_
  - _Boundary: CursorWorker_
  - _Depends: 2.1, 3.1_

- [ ] 4.2 状態変化適用タスク（UI スレッド・差分時のみ WS_EX_TRANSPARENT を加除）
  - `spawn_local` の async タスクで `event.listen().await` 起床後、`desired_passthrough` を読みローカル `applied` と比較する
  - 差分があるときのみ `SetWindowLongPtr(GWL_EXSTYLE)`＋`SetWindowPos(SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)` で **`WS_EX_TRANSPARENT` のみ**を加除する。`WS_EX_NOREDIRECTIONBITMAP`／`WS_EX_TOPMOST` は保持する。差分が無い間はスタイル適用 API を呼ばない
  - 切替時に「ON→OFF」「OFF→ON」と座標をログ出力する
  - 観測: カーソルが円境界をまたぐと ON↔OFF の切替ログが出て、円内／円外に留まる間は追加の `SetWindowPos` 呼出もログも出ない
  - _Requirements: 5.3, 5.4, 5.5_
  - _Boundary: StateApplier_
  - _Depends: 4.1_

- [ ] 5. Integration: 起動初期状態とライフサイクル
- [ ] 5.1 起動時初期状態確定とライフサイクル結線
  - 窓を初期 ex_style = `WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TRANSPARENT`（クリックスルー ON）で生成し、UI 側 `applied` 初期値を ON に一致させる
  - 初回 notify 取りこぼし防止のため、UI で `event.listen()` を確立してからワーカを spawn する（listen-then-spawn）。ワーカは初回を無条件判定とする＝StateApplier（4.2）の差分適用が起動時カーソル円内でも正しく初回 OFF へ収束する
  - 窓クローズ→`block_on` の future 完了→`done` を `store(true)`→ワーカ join で正常終了する（最終 notify で UI 側 listen を確実に解除）
  - 観測: 起動時にカーソルが円内でも初回 1 回だけ OFF が適用され、窓を閉じるとプロセスとワーカスレッドが正常終了する（タスクマネージャでスレッドが残らない）
  - _Requirements: 8.1, 8.2_
  - _Depends: 3.3, 4.1, 4.2_

- [ ] 6. Validation: 一次記録と手動検証
- [ ] 6.1 (P) REPORT.md テンプレートと README 3 幕の整備
  - `REPORT.md` を design 定義のフォーマットで作成する（検証日／実行コマンド／環境、T1〜T8 合否・証跡台帳、必須合格基準 T1・T2・T3・T4・T6 の充足欄、人間が記入する総合判定 go／違う／直す＋理由・学び）
  - `README.md` を 3 幕で整える（動機＝対応本坑 `wintf-clickthrough-alpha-toggle` を名指し、概要＝実行法 `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`、検証結果＝判定欄）
  - 観測: `REPORT.md` と `README.md` が所定フォーマットで存在し、T1〜T8 の空台帳と実行コマンドが揃っている
  - _Requirements: 1.3, 9.5_
  - _Boundary: README.md, REPORT.md_

- [ ] 6.2 T1〜T8 の人間との手動検証と記録
  - 手順「人間の準備確認 → エージェントが `cargo run -p pilot --example pilot-clickthrough-alpha-toggle` を起動 → 結果のヒアリング」で T1〜T8 を手動検証する（背面アプリ＝デスクトップアイコン／ブラウザを用意、高 DPI 150% 構成も確認）
  - T1〜T8 の合否・証跡を `REPORT.md` に記入する。必須 T1・T2・T3・T4・T6、条件付き可 T5・T7・T8。Win32 API/挙動の不確実点に遭遇したら推測せず開発者に質問する
  - go／違う／直す の総合判定は開発者（人間）が下す。Claude Code 単独で合格判定して次フェーズに進まない
  - 観測: `REPORT.md` に T1〜T8 の合否・証跡が埋まり、必須合格基準の充足が判定され、人間が総合判定（go／違う／直す）を記入している
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.6, 10.4_
  - _Depends: 5.1, 6.1_

## Implementation Notes

- 3.2: 判定円（`alpha_is_opaque`）も描画円（DComp surface）も `GetWindowRect` 中心基準で算出している。R4.1 の文言は「クライアント中央」だが、ボーダーレスの `WindowType::TopLevel` ツール窓では client≈window で中心が一致する（design.md §座標手順で T7 へ委譲済み）。**T7/6.2 の目視で「見えている円」と「判定円」がズレないか一度確認すること**。ズレた場合のみ本坑でクライアント矩形基準へ補正。
- 3.2: DComp は `IDCompositionSurface` 経路（`CreateSurface`→`BeginDraw::<ID2D1DeviceContext>`→`EndDraw`→`Commit`）を採用（swapchain 経路より配線が単純）。`BeginDraw` が返す atlas オフセット POINT を円中心へ加算して吸収（`SetTransform`/windows-numerics 名指し回避）。描画 DC は `SetDpi(96)` で 1DIP=1px 固定し PMv2 物理px の判定円と一致させている。
