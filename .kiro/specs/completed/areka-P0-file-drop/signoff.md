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

- 日時: 2026-09-29 20:47〜20:49 JST（記録の時刻は UTC で 11:47〜11:49）
- HEAD: `c8e8ba28`・環境: Windows 11 Pro 10.0.26200
- 起動: 開発者が PowerShell で起動（`NO_COLOR=1`・`RUST_LOG=info,areka=debug,kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS=600000`＝安全弁・働く前に手で終了）。標準出力は `target\fd\root\run.log`（110,202 行）
- 操作: 開発者が Explorer から手で落とした。⑴⑵ は `vendors\sample_ghost\` の原本から、⑶⑷⑹ は主のキャラクター窓（`24v0`・scope 0）と相方のバルーン窓（`25v0`・scope 1）の両方へ 1 回ずつ落とした。最後は右クリックの「終了」

### ⑴ `.nar` をキャラクター窓へ: 合格

```
11:47:16.853148 files_dropped entity=24v0 count=1
11:47:16.854115 file_drop_received scope=0 count=1 files=0 dirs=0 installs=1 elapsed_ms=0
11:47:16.854316 install_order_queued origin=WindowDrop count=1
11:47:16.860284 install_begin archive=…\vendors\sample_ghost\konnoyayame.nar origin=WindowDrop
11:47:16.979025 install_done kind=Ghost places=["…\\target\\fd\\root\\ghost\\konnoyayame"]
11:47:16.986750 last_installed_recorded folder=konnoyayame
11:47:16.993718 shiori_request method=GET id=OnInstallCompleteEx references=["ghost", "はろーYAYAワールド", "…\\ghost\\konnoyayame"]
11:47:17.001647 shiori_request method=GET id=OnInstallComplete references=["ghost", "はろーYAYAワールド", ""]
```

### ⑵ `.nar` をバルーン窓へ: 合格

```
11:47:37.604124 files_dropped entity=25v0 count=1
11:47:37.604644 file_drop_received scope=1 count=1 files=0 dirs=0 installs=1 elapsed_ms=0
11:47:37.604800 install_order_queued origin=WindowDrop count=1
11:47:37.639862 install_done … kind=Balloon places=["…\\target\\fd\\root\\balloon\\emo2-kakukaku-offsetdpi"]
```

- 落とした窓 `25v0` は相方（scope 1）のバルーン窓（起動時の `[zorder-pair] declared scope=1 char_entity=26v0 balloon_entity=25v0`）。`scope=1` は相方のキャラクターの番号で、design ⑵ の「`scope` がキャラクターの番号」どおり（要件 1.4）。テスト 14 と同じ振る舞い

### ⑶ インストール対象でないファイル（`.png` と拡張子なし）: 合格

```
11:48:08.406231 file_drop_received scope=0 count=2 files=2 dirs=0 installs=0 elapsed_ms=0
11:48:08.406415 shiori_request method=GET id=OnFileDrop2 references=["…\\drop\\noext\u{1}…\\drop\\pic.png", "0", "\u{1}image/png"]
11:48:10.859160 shiori_request method=GET id=OnFileDrop2 references=["…\\drop\\noext\u{1}…\\drop\\pic.png", "1", "\u{1}image/png"]
```

- Reference が 3 つ。Reference0 と Reference2 の要素数が同じ（2）で、拡張子なしの MIME は空のまま区切りが残った（要件 4.2〜4.6）
- 実 HDROP から `read_dropped_paths` の成功の枝を踏んだ（タスク 1.2 の申し送り: 決定論テストでは踏めない枝）

### ⑷ フォルダ 2 つ: 合格

```
11:48:18.336538 shiori_request method=GET id=OnDirectoryDrop references=["…\\drop\\d1", "0"]
11:48:18.337848 shiori_request method=GET id=OnDirectoryDrop references=["…\\drop\\d2", "0"]
11:48:20.713890 shiori_request method=GET id=OnDirectoryDrop references=["…\\drop\\d2", "1"]
11:48:20.715116 shiori_request method=GET id=OnDirectoryDrop references=["…\\drop\\d1", "1"]
```

- 1 回の投げ込みで 2 回ずつ、OS が渡した一覧の順どおり（2 回目は Explorer が d2 を先に渡した＝`files_dropped` の一覧の順と送出の順が同じ）

### ⑸ 絵の外へ: 合格（記録に何も出ない）

- `files_dropped` は全体で **8 件**で、⑴ 1・⑵ 1・⑶ 2・⑷ 2・⑹ 2 の 8 回と数が一致した。絵の外への投げ込みに対応する `files_dropped`・`file_drop_received` は 0 件

### ⑹ 複数の `.nar` を一度に: 合格

```
11:49:03.899458 file_drop_received scope=0 count=2 files=0 dirs=0 installs=2 elapsed_ms=1
11:49:03.899675 install_order_queued origin=WindowDrop count=2
11:49:03.930228 install_done … kind=Balloon … emo2-kakukaku-wplimit
11:49:03.996039 install_done … kind=Ghost … R_POST_and_KOMAINU
11:49:04.027299 shiori_request method=GET id=OnInstallCompleteAll references=["balloon\u{1}ghost", "kakukaku for emo2 (windowposition-limit verification)\u{1}Ｒポストと狛犬", "…"]
11:49:06.577031 file_drop_received scope=1 count=2 files=0 dirs=0 installs=2 elapsed_ms=1
11:49:06.767863 shiori_request method=GET id=OnInstallCompleteAll references=["ghost\u{1}balloon", …]
```

- 1 回の投げ込みの 2 本が 1 つの依頼にまとまり、各書庫の `OnInstallCompleteEx`・`OnInstallComplete` の後に `OnInstallCompleteAll` が **1 回**（2 回の投げ込みでそれぞれ 1 回）。2 回目は ⑹ の 1 回目で入れた物の上書き（表示中でない物なので降ろしは無い）

### 全体の判定

- **表示中のゴーストは替わらなかった**: 起動時の 1 件（`ghost_switch_done`・迎え入れの予約の外）の後、`ghost_switch_requested` は 0 件。`last_installed_recorded` は記録されたが切り替えは起きない（インストールと切り替えは別イベント）
- **受け取りの所要**: `file_drop_received` の `elapsed_ms` は 0〜1 ms（書庫 2 本の目次を読む回で 1 ms）
- **失敗の記録は 0 件**: 最初の投げ込みの後、`WARN`・`ERROR` は 0 行。`files_dropped_read_failed`・`file_drop_probe_failed`・`file_drop_archive_unreadable`・`file_drop_no_kanade`・`file_drop_send_failed` も 0 件
- **画面は何も出なかった**: 投げ込みの経路に選ぶ画面・メッセージボックスは無い（記録にも画面の行は無い）
- **終了はきれい**: `app_exit origin=KanadeStopped(Quit) closed=4` → `session_mark_cleared`
- **記録の水準の差**: design は `wintf::ecs::window_proc=debug` まで開けると書いたが、この走行の `RUST_LOG` には入れていない。成功の線（`files_dropped`）は info で見え、wintf 側の debug の分かれ道（部品なし・破棄済み）はこの走行では踏まないので、判定は変わらない
