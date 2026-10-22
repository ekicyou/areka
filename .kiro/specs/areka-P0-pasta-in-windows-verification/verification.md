# ぱすたの実機の検証の記録

spec `areka-P0-pasta-in-windows-verification` の最初の走行の記録。手順は `doc/pasta-real-run.md`。時刻はすべて日本時間。

## 1. 同定

| 項目 | 値 |
|---|---|
| コミット | `c3964dba`（`git -C <w> rev-parse --short HEAD`。手順の 3 節の画面の読み方の直しは作業中の差分で、`areka.exe` には関わらない） |
| debug 版 | `<w>\target\debug\areka.exe`（33,629,696 バイト・2026-10-23 00:03:48 に `cargo build -p areka` で作った物） |
| i686 の補助 | `<w>\target\debug\shiori-host32-helper.exe`（263,168 バイト・2026-10-23 00:04:58）。`sync-i686-helper.ps1` が 1 回目に `placed i686 helper: ...`、2 回目に `helper is i686: target\debug\shiori-host32-helper.exe` を出し、どちらも `sync exit=0` |
| ぱすたの書庫 | `pasta-in-windows.nar` の `.sha256` の値 `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（書庫の中の `ghost/master/pasta.toml` は `version = "1.0.0"`。descript に版の行は無い） |
| 道具 | rustc 1.99.1・cargo 1.99.1 |

`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`。

### 準備の結果

手順の 3 節の準備の塊を、ワークツリーの外のフォルダ（このセッションの一時の作業フォルダ）を作業ディレクトリにした `pwsh` で、先頭の `$w` の行だけ替えてそのまま 2 回回した（2026-10-23 00:01〜00:04 と 00:05）。2 回目の出力:

```text
sample-fetch: pasta-in-windows: present
sample-fetch: fetched 0, present 1, failed 0
fetch exit=0
pasta sha256: ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6
...\target\pasta-verify\root\ghost\emo2
...\target\pasta-verify\root\ghost\pasta-in-windows
...\target\pasta-verify\root\balloon\emo2-kakukaku
...\target\pasta-verify\root\balloon\StayseeBalloon
build exit=0
build i686 exit=0
helper is i686: target\debug\shiori-host32-helper.exe
sync exit=0
```

- 根 `<w>\target\pasta-verify\root` に `ghost\emo2`・`ghost\pasta-in-windows`・`balloon\emo2-kakukaku`・`balloon\StayseeBalloon` の 4 つが揃った。縦書きのバルーンはゴーストのフォルダの中（`ghost\pasta-in-windows\pasta_tategaki\descript.txt`）に在る。本走行では根の `balloon\` に足していない。
- 記憶の置き場 `<w>\target\pasta-verify\profile` は空（0 件）。
- ぱすたの書庫は取り寄せの段の置き場（`<w>\target\nar-samples\fetched\`）に在り、最新の `.sha256` と合ったので取り直していない（`present 1`）。

### 書庫の中身の自動テストの較正（タスク 1.2）

`crates/sample-ghost-kit/tests/pasta_inner_balloon_test.rs` の較正のテスト `a_vertical_balloon_outside_the_ghost_folder_is_rejected`（縦書きのバルーンをゴーストのフォルダの外に置いた固定入力）が、置き場の誤りを捉えることを確かめた。

- 判定を「根から再帰して最初に見つかった `pasta_tategaki/descript.txt` でよい」形に緩めると、較正のテストが `a_vertical_balloon_outside_the_ghost_folder_is_rejected ... FAILED`（`1 passed; 1 failed`）になった。
- 判定を元に戻すと緑に戻った。
- 固定入力の `install.txt` には、設計の 2 行（`type,ghost`・`directory,pasta-in-windows`）に `name,` の行を足している。足さないと `NarArchive::open` が `MissingRequiredKey { key: "name" }` で断る。

## 2. 機械の構成

| 項目 | 値 |
|---|---|
| OS | Windows 11 Pro 26H2（10.0.26300・UBR 9711） |
| 画面 1 | `\\.\DISPLAY1`・主画面・位置 (0, 0)・2880×1800（物理画素）・DPI 192＝拡大率 200% |
| 画面 2 | `\\.\DISPLAY2`・位置 (-2560, 195)・2560×1600（物理画素）・DPI 144＝拡大率 150% |

- 位置と拡大率は、手順の 3 節の命令（プロセスを画面ごとの DPI に合わせてから `Screen.AllScreens` と `GetDpiForMonitor` を読む）で 2026-10-23 00:06 に読んだ。
- 拡大率の違う 2 つの画面（200% と 150%）が在るので、段 5 は回す。

## 3. 環境変数と記録の水準

手順の 4 節の起動の命令で設定する値。走行の前に替えたものは無い。段の判定に別の分岐が要って対象を足したときは、4 節（走行）にその値を書く。

| 名前 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT` | `<w>\target\pasta-verify\root` | 根 |
| `AREKA_PROFILE_DIR` | `<w>\target\pasta-verify\profile`（準備で毎回空にする） | 記憶の置き場。空なので、起動のイベントは `OnFirstBoot` になる |
| `RUST_LOG` | `info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug` | バルーンの決め方の分岐が記録に出る水準 |
| `AREKA_APP_SMOKE_EXIT_MS` | `3600000`（60 分） | 安全弁。7 段の所要（30〜45 分の見込み）より長く |
| `NO_COLOR` | `1` | 記録を色の制御文字の無い平文にする |
| `AREKA_NO_ALERT`・`AREKA_USER_ROOT` | 消す | 本物の利用者の置き場（`%LOCALAPPDATA%\areka`）に触らない |

起こし方は `<w>\target\debug\areka.exe --ghost "<w>\target\pasta-verify\root\ghost\pasta-in-windows"`（作業ディレクトリは `<w>\target\pasta-verify`）。

### この回で手順へ戻した直し

- 3 節: 拡大率を「Windows の設定の『ディスプレイ』で読む」から、命令で読む形（`GetDpiForMonitor` の DPI を 96 で割る）に替えた。設定の画面を開かずに、回す者が命令だけで記録に書けるようにするため。拡大率の違う画面が無いときは走らせる前に「段 5 は回していない」と書く旨も足した。

## 7. α の決まり

ukadoc が書いていない所（絵の薄い画素の当たり）についての areka の決め。コードは変えていない。

### 決まり

- **当たり**: 画素の α が 0.5 以上（8 ビットの値で 128 以上）の所だけを当たりにする。α が 0.5 未満の所は当たりにしない（7.1）。
  - シェルとバルーンの絵の当たり: `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs` の `ALPHA_THRESHOLD: u8 = 128`。
  - ウィジェットの当たり: `crates/wintf/src/ecs/layout/hit_test/mod.rs` の 2 か所の `128.0 / 255.0`（見た目の不透明度 × 塗りの α と比べる）。
- **描き方**: α が 1 以上の画素はすべて描く。今の描き方のまま（7.2）。アトラスの切り詰め（`trim.rs` の α > 0）は当たりとは別の話で、これも触っていない。
- **SSP との違い**: SSP は α が 0 でなければ当たる見込み。これは開発者の見立てで、測っていない（7.4）。areka はこれに合わせない。

### 裁定

- 日付: 2026-10-22（開発者）。
- 出どころ: 本 spec の設計の話し合いの議題 1（コミット `3f539637`「α の当たりは 0.5 以上のまま SSP に合わせず、α の測りを外す」）と、要件 7.1・7.2。
- 理由（開発者の言葉）: 「ヒットテストのルールはwintfの決定にも影響する。α＜0.5の領域はそもそもヒットテスト領域にすべきではない。」
- 描き方: SSP が α 8 以下の画素を描かなかった件は SSP のバグで解消済みのため、写さない（同日の開発者の裁定・要件 7.2）。
- 走行で「SSP では当たるのに areka では通り抜ける薄い所」が見えたら、5 節の崩れた所の表に「方針で受けない」として、この裁定の日付と出どころをつけて書く（7.3）。

### 境のテストの確かめ（2026-10-23・コミット `c8c80bd6` の上）

既存の境のテストを変えずに回した。どれも緑。机は要らない（クレートのテストだけ）。

| 命令 | 確かめる所 | 結果 |
|---|---|---|
| `cargo test -p wintf --lib test_threshold_boundary_128` | `alpha_mask.rs` で α がちょうど 128 の画素が当たる | 1 件 ok・失敗 0 |
| `cargo test -p wintf --lib test_hit_test_entity_bounds_alpha_boundary` | `hit_test/tests_ex/opacity.rs` の線のすぐ上と下（`_above`・`_below`） | 2 件 ok・失敗 0 |
| `cargo test -p wintf --lib hit_test::tests_ex::opacity` | `opacity.rs` の全部 | 6 件 ok・失敗 0 |

`alpha_mask.rs` と `hit_test/mod.rs` の `main` との差分は 0 行（`git diff main -- <2 つのファイル>` の出力が空）。
