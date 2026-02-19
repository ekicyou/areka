# Requirements Document

## Project Description (Input)
DPI200%モニタ（左）からDPI125%モニタ（右）へドラッグしようとすると失敗して元モニターの位置にウィンドウが戻ってしまう。DPIが切り替わったときにウィンドウサイズが変わる（DPI小→物理サイズ縮小）と、ウィンドウの左上座標がそのままでは中心座標がずれ、ウィンドウが元の200%モニター側に戻ってしまう。DPI変更によるレイアウト変更時（WM_DPICHANGED → ECS tick によるサイズ変更適用）において、ウィンドウの中心座標が変化しないように位置を補正する必要がある。

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
