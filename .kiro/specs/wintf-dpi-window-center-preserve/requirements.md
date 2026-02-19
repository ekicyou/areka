# Requirements Document

## Project Description (Input)
DPI200%モニタ（左）からDPI125%モニタ（右）へドラッグしようとすると失敗して元モニターの位置にウィンドウが戻ってしまう。DPIが切り替わったときにウィンドウサイズが変わる（DPI小→物理サイズ縮小）と、ウィンドウの左上座標がそのままでは中心座標がずれ、ウィンドウが元の200%モニター側に戻ってしまう。DPI変更によるレイアウト変更時（WM_DPICHANGED → ECS tick によるサイズ変更適用）において、ウィンドウの中心座標が変化しないように位置を補正する必要がある。

## Introduction

`wintf-dpi-aware-layout` 仕様で確立されたレイアウト主導方式（`BoxStyle.size` = 論理px をソースオブトゥルース、DPI変更時はサイズ不変）において、DPIスケールの変化によるウィンドウ物理サイズの拡大・縮小時に、ウィンドウの中心座標が保持されないバグを修正する。特に高DPI→低DPIモニターへのドラッグ移動時に、物理サイズ縮小に伴う中心座標のずれにより、ウィンドウが元のモニターに引き戻される問題を解決する。

## Requirements

### Requirement 1: DPIサイズ変更時の中心座標保持

**Objective:** 開発者として、DPI変更に伴うウィンドウ物理サイズの変化時にウィンドウの中心座標（物理px）が不変となることを保証したい。ユーザーがモニター間でウィンドウを移動する際に、ウィンドウが意図通りの位置に留まるようにするため。

#### Acceptance Criteria
1. When DPI変更によりウィンドウの物理サイズが変化した場合, the wintf layout system shall ウィンドウの左上座標を補正し、変更前後でウィンドウの中心座標（物理px）が同一になるようにする
2. When ウィンドウ物理サイズが `(old_w, old_h)` から `(new_w, new_h)` に変化した場合, the wintf layout system shall 左上座標の補正量を `((old_w - new_w) / 2, (old_h - new_h) / 2)` として算出する
3. The wintf layout system shall サイズ変更と位置補正を単一の `SetWindowPos` 呼び出しでアトミックに適用する

### Requirement 2: 高DPI→低DPIモニター間ドラッグ移動

**Objective:** ユーザーとして、高DPIモニターから低DPIモニターへウィンドウをドラッグで移動した際に、ウィンドウが正常に移動先モニターに留まることを期待する。ウィンドウが元のモニターに引き戻されないようにするため。

#### Acceptance Criteria
1. When ウィンドウを200%モニターから125%モニターへドラッグした場合, the wintf window system shall ウィンドウが移動先の125%モニター上に正しく配置される
2. When DPI縮小（例: 200%→125%）によりウィンドウ物理サイズが減少した場合, the wintf window system shall 中心座標保持補正により、ウィンドウ中心が移動先モニター領域内に維持される
3. If ドラッグ中にDPI変更による中心座標補正が行われた場合, the wintf window system shall 再度の `WM_DPICHANGED` 発火（元のDPIへの巻き戻し）を引き起こさない

### Requirement 3: 低DPI→高DPIモニター間ドラッグ移動

**Objective:** ユーザーとして、低DPIモニターから高DPIモニターへウィンドウをドラッグで移動した際にも、同様にウィンドウが正常に配置されることを期待する。DPI増加方向でも一貫した動作にするため。

#### Acceptance Criteria
1. When ウィンドウを125%モニターから200%モニターへドラッグした場合, the wintf window system shall ウィンドウが移動先の200%モニター上に正しく配置される
2. When DPI増加（例: 125%→200%）によりウィンドウ物理サイズが増加した場合, the wintf window system shall 中心座標保持補正により、ウィンドウ中心が移動先モニター領域内に維持される

### Requirement 4: ECSパイプラインとの統合

**Objective:** 開発者として、中心座標保持ロジックが既存のECSレイアウトパイプライン（`update_arrangements_system` → `propagate_global_arrangements` → `window_pos_sync_system` → `apply_window_pos_changes`）と自然に統合されることを保証したい。既存のレイアウト主導方式の設計原則を壊さないため。

#### Acceptance Criteria
1. The wintf layout system shall `BoxStyle.size`（論理px）をソースオブトゥルースとする既存の設計原則を維持する
2. When DPI変更時にサイズ補正と位置補正を適用する場合, the wintf layout system shall `DpiChangeContext` の仕組みを活用し、`WM_WINDOWPOSCHANGED` ハンドラの echo/bypass 判定と整合する
3. The wintf layout system shall 中心座標保持のための位置情報（変更前の物理サイズまたは中心座標）を `DpiChangeContext` もしくは同等の仕組みで DPI変更フロー内に伝達する

### Requirement 5: 単一DPIモニター環境での無影響

**Objective:** 開発者として、単一DPIモニター環境やDPI変更が発生しない通常操作において、中心座標保持ロジックが動作に影響を与えないことを保証したい。既存の動作を壊さないため。

#### Acceptance Criteria
1. While DPI変更が発生していない場合, the wintf window system shall ウィンドウの移動・リサイズ操作に追加の位置補正を行わない
2. While 単一DPIモニター環境で動作している場合, the wintf window system shall 既存の動作と同一のウィンドウ配置結果を維持する
3. When ユーザーが手動でウィンドウをリサイズした場合, the wintf window system shall 中心座標保持補正を適用しない（DPI変更起因のサイズ変更にのみ適用される）

### Requirement 6: ログ出力

**Objective:** 開発者として、DPI変更時の中心座標保持補正の適用状況をトレースログで確認できるようにしたい。デバッグと動作確認を容易にするため。

#### Acceptance Criteria
1. When DPI変更に伴う中心座標保持補正が適用された場合, the wintf layout system shall 補正前の中心座標、補正後の中心座標、適用された位置オフセットを `debug!` レベルでログ出力する
2. When DPI変更が発生したが位置補正が不要であった場合（サイズ変化なし）, the wintf layout system shall その旨を `trace!` レベルでログ出力する
