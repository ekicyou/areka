# Requirements Document

## Project Description (Input)
DirectCompositionベース⇒UpdateLayeredWindowベースへ変更マウスクリックのクリックスルーが出来ないことが判明し、DirectComposition描画ではデスクトップマスコット描画は不可能と結論付けた。そのため、描画をD3D11⇒D2D1⇒UpdateLayeredWindow()+WS_EX_LAYEREDレンダリングへと変更する。そのための要件変更やリジェクトコードの範囲、新規実装の必要量など、影響範囲を深掘り調査し、実装指針・変更ストーリーや計画を策定せよ。
順序としては、先にD3D11＋D2D1ベースの新しい合成スタックやシステムを作成し、DCompベースの合成パイプラインと同じ程度の実装が出来たことを確認してから、DCompパイプラインからD2D11パイプラインへ変更、最後にUpdateLayeredWindow()+WS_EX_LAYEREDレンダリングの実装を行う案がある。旧実装を参照しつつ、新しい実装を検討し、最後にまとめて削除するのが望ましい。本仕様設計段階で最適な置き換えプランを策定し、子仕様を決定する。

本仕様のゴールは実装指針ドキュメントの作成と、フェーズ番号を振った子仕様ドキュメントの作成とする。子仕様は実装指針ドキュメントを参照するように作成する。

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
