# REPORT: pilot-clickthrough-alpha-toggle 検証結果

> 本 REPORT は T1〜T8 の機械的な合否・証跡の詳細台帳である（根拠）。結論（go／違う／直す ＋ 学び）は README の「検証結果」に記す。
> 検証手順（R9.1）: 人間の準備確認 → エージェントが `cargo run -p pilot --example pilot-clickthrough-alpha-toggle` を起動 → 結果のヒアリング。
> go 判定は開発者（人間）が下す。Claude Code 単独で合格判定して次フェーズに進まない（R9.6）。**＝下記「総合判定」は空欄のまま人間の記入を待つ。**

- 検証日: 2026-07-01
- 実行コマンド: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`
- 環境: Windows 11（PMv2 = PER_MONITOR_AWARE_V2 設定成功）/ DPI 倍率: 要記入 / モニタ構成: 要記入。観測ログ上の窓（=クライアント, WS_POPUP 化後）矩形 ≈ 2880x1550 物理px。

## 結論（核心 Unknown は肯定的に決着）

**`WS_EX_TRANSPARENT` 動的トグルは、DComp（`WS_EX_NOREDIRECTIONBITMAP`）描画を捨てないまま、別プロセスへのクリック透過を成立させられる。** 実機で「円外＝背面へ透過／円内＝受領」＋「見える円と反応領域の座標一致」を人間が確認（"一致しました"）。ただし成立には当初仕様に無かった **2つの必須条件**があった:

1. **`WS_EX_LAYERED` を"同伴フラグ"として立てる**（`UpdateLayeredWindow`/`SetLayeredWindowAttributes` は呼ばない＝レイヤード描画としては使わない）。これが無いと `WS_EX_TRANSPARENT` 単独では DComp 窓でマウス透過が効かず、窓が全クリックを吸う。DComp 描画は `WS_EX_LAYERED` フラグ併設でも消えず共存できた。
2. **枠なし窓（`WS_POPUP`）にして client==window にする**。DComp（`CreateTargetForHwnd`）は**ウィンドウ原点**から合成するため、非クライアント領域（タイトルバー/枠）があると「見える円（描画）」と「反応領域（判定）」がタイトルバーぶん縦にずれる。`WS_POPUP` で解消。

> 当初仕様の R2.3「`WS_EX_LAYERED` 無し・`WS_EX_TRANSPARENT` 単独」は**誤りだった**（単独では不成立）。正しくは「`WS_EX_LAYERED` はフラグのみ・レイヤード描画には使わない（ULW/SLWA 非呼出）」。本坑仕様へ反映すべき最重要知見。

## T1〜T8 合否台帳

| # | 試験項目 | 期待結果 | 合否 | 証跡（観測内容・ログ抜粋） |
|---|---------|---------|------|----------------------------------------|
| T1 | 起動確認 | 透過トップモスト窓＋中央の不透明円が表示される | ✅ | `ShowWindow(SW_SHOW)` 追加後に円が可視（当初は ShowWindow 未呼出で不可視だった＝実装欠陥を修正）。背景透過＋中央円。 |
| T2 | 円外でのクリック透過 | 円外クリックで背面が反応（窓は受領しない） | ✅ | `WS_EX_LAYERED` 同伴後に成立。円外は透過し窓が受領しない（`WS_EX_LAYERED` 無しでは全クリックを吸っていた）。 |
| T3 | 円内でのクリック受領 | 円内クリックで `WM_LBUTTONDOWN`（受領＋色トグル） | ✅ | 透過 OFF 区間で円内クリック `client=… → 円内（正常受領）` を多数。色トグル＋DComp 再描画も動作。 |
| T4 | 状態切替の発火 | 円境界をまたぐ瞬間に ON↔OFF ログ | ✅ | `[applier] ON→OFF (…0x280028→0x280008)` / `OFF→ON` をカーソル追随で出力。 |
| T5 | 状態変化なし時の非発火 | 留まっている間 SetWindowPos 非呼び出し | ✅（暫定） | 同一領域滞在中は applier の API 呼出/ログなし（差分＋notify-on-change の二重ガード）。 |
| T6 | マルチプロセス透過 | 背面ブラウザのリンクが円外クリックで開く | ✅（要 明示確認） | T2 成立＝別プロセス透過の機構は確認済み。背面ブラウザのリンクを円外クリックで開く**明示テストは要実施**（人間）。 |
| T7 | DPI 環境での座標一致 | 高 DPI でも円判定が見た目と一致 | ✅（部分） | `WS_POPUP` 化で見える円と反応領域が一致（人間確認："一致しました"）。高 DPI 150% 等の明示検証は要実施。 |
| T8 | 終了処理 | 窓を閉じるとプロセス・ワーカが正常終了 | ✅ | **ダブルクリック終了**（後述）で実機確認: ログに `ダブルクリック → shutdown` → `WM_CLOSE 受領…清掃終了` → `監視ワーカ停止（done → join 完了）` が並び、**プロセスは exit code 0 で正常終了**（従来の force-kill は 255）。close→done→join→`DestroyWindow` の単一経路が機能。 |

## 必須合格基準（T1・T2・T3・T4・T6）の充足

- すべて ✅ か: **実質はい（T1/T2/T3/T4 ✅、T6 は機構確認済みで背面ブラウザ明示テストのみ残）。** 条件付き（T5 ✅暫定 / T7 ✅部分 / T8 ✅実機確認済）。

## 実装で判明・修正した副次事項（再現に必須）

1. `Window::new_ex`（wintf-winmsg-executor 0.0.5）は `WS_VISIBLE` を立てず `ShowWindow` も呼ばない → `ShowWindow(SW_SHOW)` 明示必須（でないと不可視）。
2. DComp は**ウィンドウ原点**から合成 → 枠付き窓だと判定/描画がずれる。`WS_POPUP`（client==window）で解消。判定・描画は同一原点・半径 `RADIUS`。
3. `ClientToScreen` は `windows::Win32::Graphics::Gdi`、`GetClientRect` は `WindowsAndMessaging`。
4. 診断: `WM_SETCURSOR` で十字カーソル（透過 OFF 区間のみ届く＝反応領域の可視化）。

## 追加検証で得た知見（ドラッグ移動・終了）

検証の過程で、マスコット実用に不可欠な2点を追加実装し実機確認した（人間 "OK"）:

- **ドラッグ移動（不透明部を掴んで窓を動かす）**: `WM_LBUTTONDOWN` で `SetCapture`＋アンカー（カーソル−窓原点）記録、`WM_MOUSEMOVE` で追従 `SetWindowPos`、`WM_LBUTTONUP` で `ReleaseCapture`。DComp 合成内容・判定円も窓と一緒に動くため座標一致は保たれる。
- **【最重要の罠】ドラッグ中は表示位置に関わらず `WS_EX_TRANSPARENT` を外したまま維持する**: ドラッグ中にカーソルが（追従遅延等で）判定円を外れると、通常なら applier が透過 ON へ戻す→**`SetCapture` が無効化されドラッグが崩壊**する。対策として `dragging: Arc<AtomicBool>` を設け、**ドラッグ中は applier の透過トグルを一切抑止**（`if desired != applied && !dragging`）、終了時に `state_changed` を notify して透過状態を再収束させる。modal な `HTCAPTION` 任せにせず自前 `SetCapture`＋明示フラグで制御すると挙動が読め、この罠を確実に回避できる（実装者が嵌まりやすい要点）。
- **ダブルクリック終了（`WM_LBUTTONDBLCLK`→shutdown）**: 枠なし窓は閉じる UI が無いため。`WM_CLOSE` と同じ清掃経路に合流し、T8 の正常終了を実機で確認する手段も兼ねた（クラスは `CS_DBLCLKS` 登録済み＝dblclk が届く）。

## 設計の最重要洞察：表示層と当たり判定層は独立した別レイヤー

本 pilot の一番深い結論。「DComp を保ったままクリックスルー」を成立させた根本原理は、**表示と当たり判定が別々のレイヤーに属し独立に制御できる**こと:

- **表示レイヤー ＝ DirectComposition visual tree**。これは HWND の redirection 描画の**「外」にあるコンポジタ層**で、`Commit` した内容を DWM が保持する（WM_PAINT/ULW 不要で維持される＝"置きっぱなし"が効く）。`WS_EX_NOREDIRECTIONBITMAP` で redirection surface を消すと、表示はこの DComp 層だけが担う。**visual の content には `IDCompositionSurface` でも合成用 swapchain（`CreateSwapChainForComposition`）でも載せられる** ＝ D3D の 3D／Live2D 等の GPU 描画をそのまま透過合成できる。
- **当たり判定レイヤー ＝ HWND の窓矩形＋拡張スタイル**。`WS_EX_TRANSPARENT`（それを効かせる `WS_EX_LAYERED` フラグ）で制御。**DComp の per-pixel α は当たり判定に一切関与しない**。

この2層が独立ゆえ:
1. `WS_EX_NOREDIRECTIONBITMAP` と `WS_EX_LAYERED` は**表示手段として競合しない**（LAYERED を ULW/SLWA で表示に使わず、当たり判定スイッチとしてのみ借りるため）。ドキュメントの「should not mix」は"表示の二重化を避けよ"の意であって、両ビットを立てること自体のハード禁止ではない（実測: ex_style `0x280028` で両立）。
2. だから **DComp の GPU 描画（3D／Live2D 含む）を維持したまま、当たり判定だけ `WS_EX_TRANSPARENT` トグルで別制御**できる。
3. これが **3D／Live2D デスクトップマスコットが「透過表示＋キャラ部だけクリック受領」を実現している技術的からくり**：合成 swapchain に GPU でキャラを描き、DComp で per-pixel 透過合成し、αマスク連動の `WS_EX_TRANSPARENT` トグルでキャラ以外をクリックスルーさせる。本 pilot はその最小骨格（円＝キャラ相当）を実証した。

本坑はこの「**表示 ＝ DComp（surface でも swapchain でも可）／当たり判定 ＝ HWND スタイル**」の分離を設計の土台に据えること。将来 3D／Live2D を載せる場合も、当たり判定側（αマスク→トグル）はこの pilot と同一のまま、表示側の content を swapchain に差し替えるだけで拡張できる。

## 本坑（wintf-clickthrough-alpha-toggle）への申し送り

- ex_style は `WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT`（TRANSPARENT のみ動的トグル・他は固定）。`WS_EX_LAYERED` はフラグのみ・ULW/SLWA 非呼出。
- 窓は枠なし（`WS_POPUP` 相当）。判定円/描画円は同一原点（client==window）・物理座標。
- R2.3 の「`WS_EX_LAYERED` 無し」制約は**撤回**し「`WS_EX_LAYERED` フラグ併設・レイヤード描画非使用」へ改める。
- **ドラッグ移動を実装する場合は「ドラッグ中は位置に関わらず `WS_EX_TRANSPARENT` を外したまま維持」を必須要件化**（`dragging` フラグで applier のトグルを抑止・終了時に再収束）。自前 `SetCapture`＋明示フラグ制御を推奨。
- 残タスク（人間の明示確認推奨）: T6 背面ブラウザのリンクを円外クリックで開く明示確認、T7 高 DPI 150% での座標一致明示確認。（T8 は本 pilot で実機確認済み。）

## 総合判定（人間が記入）

- go / 違う / 直す: ____（**未記入＝人間の判断待ち**）
- 理由・学び: ____
