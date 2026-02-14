# Requirements Document

## Project Description (Input)
wintfのWM_NCHITTESTハンドラにおけるクリックスルー（HTTRANSPARENT）の有効化。現在`nchittest_cache.rs`では`hit_test_in_window()`の結果を取得しているが、常に`HTCLIENT`を返しており`HTTRANSPARENT`は`#[allow(dead_code)]`で封印されている。ヒットテスト結果がNone（透明領域）の場合に`HTTRANSPARENT`を返すことで、BitmapSourceのαマスク等に基づくWindowsレベルのクリックスルーを実現する。「HTTRANSPARENT を返すとマウスイベントがブロックされてしまう」という既存コメントの問題を調査・解決し、ECSポインターイベントとの共存を確保する。

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
