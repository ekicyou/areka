# 実機確認の記録（areka-P0-file-drop・要件 9.10・9.11）

design の Testing Strategy「実機」の 6 項目（⑴〜⑹）。タスク 6.2 で記録する。

## 前置き（手で落とす前に書く）

- **何が起きないのが正しいか**:
  - 絵の外（キャラクターやバルーンの絵が無い透明な余白）へ落とすと、OS の決まりどおり**背後の窓**が受ける。areka の記録には `files_dropped`・`file_drop_received` が**何も出ない**のが正しい（⑸）
  - インストール対象を落としても、**表示中のゴースト（emo2）は替わらない**。`ghost_switch_requested` が出ないのが正しい（インストールと切り替えは別イベント・09-28 裁定）
  - 落としても**画面は何も出ない**（選ぶ画面・メッセージボックスとも 0）。知らせはゴーストの台詞だけ
- **落とす位置**: 絵の内側で、縁から離した所（顔や胴の真ん中）。キャラクター窓・バルーン窓とも、窓の矩形は絵より広く、余白はクリックが透けて背後へ抜ける
- **記録の水準**: `RUST_LOG=info,areka=debug,kanade=trace`。受け取り（`files_dropped`・`file_drop_received`・`file_drop_event_sent`・`install_*`）は info、SHIORI への送出の中身（`shiori_request id=… references=[…]`）は kanade の trace で読む
- **根と検体**（ワークツリーの `target\fd\` の下・`C:\` 直下には作らない）:
  - 根 `target\fd\root\`: `tools/package-alpha.ps1` で HEAD `c8e8ba28`（タスク 6.1 のコミット）から組んだ `areka-alpha-x64-20260929-c8e8ba2.zip`（全段 緑・未コミットの変更 0 件）を展開したもの。同梱の本番ゴーストは `ghost\emo2`
  - 検体 `target\fd\drop\`:
    - ⑴ `konnoyayame.nar`（ゴースト・根に無い）
    - ⑵ `emo2-kakukaku-offsetdpi.nar`（バルーン・根に無い）
    - ⑶ `pic.png` と `noext`（拡張子なし）
    - ⑷ フォルダ `d1`・`d2`
    - ⑹ `multi\R_POST_and_KOMAINU.nar` と `multi\emo2-kakukaku-wplimit.nar`（2 本を一度に）

## 結果

（開発者の実走の後に記録する）
