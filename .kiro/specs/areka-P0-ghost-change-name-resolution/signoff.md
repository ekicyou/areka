# 実機確認の記録（要件 8.1〜8.3）

- 日時: 2026-09-27 21:00〜21:05 JST（ログの時刻は UTC で 12:00〜12:05）
- HEAD: `f02bd753`（ブランチ `claude/areka-p0-ghost-name-resolution-0d4fff`・作業ツリーに差分なし）
- 環境: Windows 11 Pro 10.0.26200
- 実行体: `cargo build -p areka -j 4` を HEAD で回し、もう一度回して「建て直し無し」を確かめてから根へ複製した。i686 の helper は PowerShell で `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`（建て直し無し）

  | ファイル | 機械種別 | SHA-256（`target\` の元と根の複製で一致） |
  |---|---|---|
  | `areka.exe`（x64 debug） | `0x8664` | `FEE4AD0C52FB447BD6E84E14D982C2320E655A99E2DBA2F0854C7050E24025EC` |
  | `shiori-host32-helper.exe`（i686） | `0x014C` | `83496CD8EF0ED684C6878FDCE4BE906F8EF99C9B974C6CB06C5AE468ACEECAF4` |

## 検体（根は 1 つ・短い絶対パス）

完了 `ghost-shell-balloon-switch` の `signoff.md` と同じ組み立て。目録を **emo2 と検体の 2 体だけ**にした（`random` は他のゴーストが居る限り今のゴーストを候補から外すので、切替先は必ず emo2 になる）。

```
C:\tmp\areka-signoff-gcnr\root\
  areka.exe                  … target\debug\areka.exe の複製
  shiori-host32-helper.exe   … i686 の helper
  ghost\emo2\                … 前回の根（C:\tmp\areka-signoff-gsw\root）の emo2 の複製
  ghost\rpost_plus\          … R_POST_and_KOMAINU の丸ごとの複製（前回の rpost_auto から作り直し）
  balloon\emo2-kakukaku\、balloon\StayseeBalloon\ … 前回の根の複製
```

- `rpost_plus` の差し替えは 2 か所だけ（どちらも Shift_JIS のまま）:
  - `ghost\master\dic02_Event.txt` の `＊OnBoot` の本文を `：\+` の 1 行に（バイト列 `81 46 5C 2B`）
  - `ghost\master\descript.txt` の `name` を `Ｒポストランダム` に
- SHIORI は里々（`satori.dll`・i686 の helper 経由）。複製元に `ghost\master\profile\areka\sylphya.toml`（起動記録あり）があるので、起動時に里々へ送られるのは `OnBoot`（`OnFirstBoot` ではない）

## 全走行に共通の設定

- 起動は argv なし。ランチャー `C:\tmp\areka-signoff-gcnr\tools\launch.ps1`（前回の `launch.ps1` の根を差し替えたもの）が環境変数を設定して起動し、終了を待って終了コードを残す
- 環境変数:
  - `NO_COLOR=1`・`AREKA_NO_ALERT=1`・`AREKA_ROOT` は外す
  - `AREKA_APP_SMOKE_EXIT_MS`（前確認 30000・本走行 60000・較正 20000）＝全走行が自分で終わる
  - `AREKA_PROFILE_DIR` は走行ごとに**新しく作ったフォルダ**（開発者のアプリの記憶は読まず・書かない）。ランチャーはフォルダが既に在れば止まる
  - `RUST_LOG=info,kanade=trace,areka_kanade=trace,areka=debug,areka_sakura=debug,areka_emo_text=info,areka_emo_present=info,areka_ghost=debug,ghost-shutdown=debug,ghost-boot=debug,shiori-actor=debug,wintf::ecs::window_proc::lifecycle=debug`（design の `info,areka=debug,kanade=trace` を含み、組み立ての「無視」の `debug!` の target `areka_sakura::compile` を明示した）
- 最初に検体を起こす方法: 新しいフォルダに `sylphya.toml` の 1 行 `[last] ghost = "rpost_plus"` だけを置いた（前回の走行 ⑦ と同じ）。記憶の置き場が本当に空だと既定の emo2 が起き、argv で検体を渡すと今のゴーストが目録の外の扱いになって `random` の候補に検体自身も入る（裁定 9.2）ため、どちらも「検体 → emo2」を一意に見る形にならない
- 記録: `C:\tmp\areka-signoff-gcnr\logs\<走行>.out.log`／`.err.log`／`.exit.txt`
- プロセス: 各走行の前後で `areka.exe`・`shiori-host32-helper.exe` がどちらも 0 件であることを確かめた

## 前確認: 里々が `\+` を素通しする

`kanade=trace` の記録は SHIORI への**送出**（`shiori_request`）だけで、応答の文字列は載らない（下の「気付いたこと」1）。そこで、既存の任意の追験テスト（実 DLL へ `OnBoot` を送り、受け取った Value をそのまま表示する）を検体の `satori.dll` へ向けた。

```
$env:HOST32_PASTA_DLL='C:\tmp\areka-signoff-gcnr\root\ghost\rpost_plus\ghost\master\satori.dll'
cargo test -p shiori-host32-host --test shiori_request_e2e request_e2e_real_pasta_optional -- --nocapture --exact
```

- 出力: `実 pasta OnBoot Value 受領（R6.5）: "\\0\\+\\e"`（Rust の表記。中身は `\0\+\e`）・テストは合格
- 里々は `\+` を自分の記法と読まず、前後に `\0` と `\e` を足しただけで返した

続けて areka の走行 `pre`（前確認・30 秒）でも同じ流れ（`ghost_switch_resolved name=random to=emo2` → `ghost_switch_requested raise_event=false` → `ghost_switch_done`）を見た。終了コード 0（31.55 秒）・ERROR 0。

## 本走行: 検体の `OnBoot` の `\+` → emo2

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gcnr\tools\launch.ps1 -Run main -Profile C:\tmp\areka-signoff-gcnr\prof-main -SmokeMs 60000
```

流れ（時刻は UTC・秒）:

| 時刻 | 事象 |
|---|---|
| 12:03:09.857 | `ghost_resolved route=Memory dir=…\ghost\rpost_plus` |
| 10.119 | 検体の窓が出た（「本物のゴースト窓を開きました」scopes=[0, 1]） |
| 10.228 | `OnBoot` GET `["master"]` → 台本 `\0\+\e`（前確認より・本走行のログは送出だけ） |
| 10.310 | **`ghost_switch_resolved name=random to=emo2 position=None`** |
| 10.311 | **`ghost_switch_requested from=Some("Ｒポストランダム") to=えも？？ raise_event=false origin="automatic"`**・`change_accepted`（同じ値） |
| 10.329 | `unload_clean`（`OnGhostChanging`・`OnClose` は送らない＝黙って降ろす） |
| 10.339 | `ghost_switch_down_ms` ms=5 |
| 10.344 | `switch_drop_recorded last_ghost="emo2" mark="えも？？"` |
| 10.353 | `windows_closed_for_restart` closed=4（検体の窓 4 枚が消えた） |
| 11.413 | `last_used_deferred ghost=Some("emo2")`・11.414 `ghost_switch_booted ghost=Some("emo2") attempt=Target` |
| 11.438 | emo2 の窓が出た（scopes=[0, 1]） |
| 11.749 | `OnGhostChanged` GET `["ポスト", "", "Ｒポストランダム", "C:\tmp\areka-signoff-gcnr\root\ghost\rpost_plus", "", "", "", "master"]` → emo2 は 204 |
| 11.750 | `OnBoot` GET → emo2 の挨拶「こんばんはー！」「夜やけど元気やでー！」「その元気、」「どこから湧くの。」 |
| 11.758 | `last_used_recorded ghost="emo2"`・`session_mark_steady ghost=えも？？` |
| 11.761 | **`ghost_switch_done ghost=Some("emo2") attempt=Target`** |
| 12:04:10.118 | 自動終了 `app_exit origin=Smoke` → `OnClose` NOTIFY `["user"]` → 10.185 `session_mark_cleared` |

- 終了コード **0**（60.44 秒）。走行後の記憶: `[last] ghost = "emo2"`・`running = ""`
- 目印の件数:

  | 目印 | 件数 |
  |---|---|
  | `ghost_switch_resolved`（name=random・to=emo2） | 1 |
  | `ghost_switch_requested`（raise_event=false）／`change_accepted` | 1／1 |
  | `ghost_switch_unknown`／`ghost_switch_busy` | 0／0 |
  | `ghost_switch_down_ms`／`windows_closed_for_restart` | 1／1 |
  | `ghost_switch_booted`（Target） | 1 |
  | `ghost_switch_done` | 2（起動時の定常到達の debug `stage=None` 1・切替を終える info 1） |
  | `ghost_switch_cancelled`／`_target_fault`／`_default_fault`／`_fatal` | 0／0／0／0 |
  | 窓を開いた回数 | 2（検体・emo2） |
  | `OnGhostChanging`／`OnGhostChanged`／`OnBoot`／`OnFirstBoot` | 0／1／2／0 |
  | 組み立ての「無視」の `debug!`（`M-boot 外タグを無視`） | **0** |
  | ログ全体の `\+` の綴り／`Raw(` | 0／0 |
  | `app_exit` | 1（origin=Smoke のみ） |
  | `ghost_quit`／`alert` | 0／0 |
  | ERROR 行／パニック | 0／0 |

- WARN は 10 件で、すべて検体と既定バルーンに由来するもの（StayseeBalloon のスコープ 1 の面の縮退 6・emo2 の全透明の要素 `purple/a/null.png` 2・折返し基準 1）と自動終了の `force_quit` 1。前回の記録と同じ種類

### 「無視」が 0 件であることの較正

`areka_sakura` の記録は本走行に 1 行も無かったので、0 件が「捨てられたから 0」でないことを確かめた。検体をもう 1 つ複製し（`root-cal\ghost\rpost_cal`・目録はこの 1 体だけ）、`OnBoot` を `：\+[x]`（角括弧付きは別名にしない＝生の綴りになる形）にして同じ `RUST_LOG` で起こした。

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gcnr\tools\launch.ps1 -Run cal -Profile C:\tmp\areka-signoff-gcnr\prof-cal -SmokeMs 20000 -RootName root-cal -Seed rpost_cal
```

- `DEBUG areka_sakura::compile: M-boot 外タグを無視 instruction=Raw("\\+[x]")` が 1 件出た・切替の記録は 0 件・終了コード 0（21.36 秒）
- 同じ設定で本走行の `\+` は「無視」に 0 件＝`\+` は生の綴りに落ちず切替の要求として届いた

## 判定

- 要件 8.1: **合格**。`\+` を言う検体の `OnBoot` の台本から、`ghost_switch_resolved`（name=random・to=emo2）と `ghost_switch_requested`（raise_event=false）が 1 件ずつ出て、検体を降ろし（窓 4 枚を閉じた）、同じプロセスで emo2 が起きて挨拶した（`ghost_switch_done` attempt=Target）。利用者から見ると、検体（ポストと狛犬）が現れて一言も話さずに消え、約 1 秒後に emo2 が現れて挨拶する
- 要件 8.2: **合格**。判定に使う分岐の level（areka の `debug`・`areka_sakura` の `debug`・kanade の `trace`）まで開けて記録を取り、本書に残した。組み立ての「無視」は較正の上で 0 件
- 要件 8.3: 下の申し送りのとおり（`lastinstalled` は実機で確かめていない）
- 要件 7.5（`.kiro/steering/roadmap.md` の本仕様の行）: このタスクでは触っていない。`/kiro-complete` の ROADMAP 更新で完了へ改める

## `ghost-install` への申し送り（要件 6.6・8.3）

- `\![change,ghost,lastinstalled]` は**実機で確かめていない**。本番で記録を書く呼び手（ゴーストのインストール）がまだ無いため。判断の分岐は、記録を手で入れた World の決定論テストで固定済み
- 記録を書く口と記録の型の名前は次の 2 つで、変えない:
  - 記録の型: `LastInstalledGhost`（プロセスの中だけの記録・フォルダ名 1 つ・ファイルや記憶へは書かない・使っても消えない）
  - 書く口: `record_last_installed(world, folder)`（置き換えて `info!(last_installed_recorded)` を 1 件残す。今は `#[allow(dead_code)]` で呼び手 `areka-P0-ghost-install` を注釈している）
- `ghost-install` がインストールの成功時にこの書く口を呼べば `\![change,ghost,lastinstalled]` が動く。実機の一周（インストール → `lastinstalled` で切替 → `ghost_switch_resolved name=lastinstalled` → `ghost_switch_done`）は `ghost-install` の実機確認で行う

## 気付いたこと

1. **design の「`kanade=trace` の応答の生文字列」は今の記録では見られない**: kanade の trace は送出（`shiori_request`・method・id・Reference）だけで、応答の Value は載らない。前確認は既存の追験テスト `request_e2e_real_pasta_optional` を里々の DLL へ向けて行った（上記）。今後の実機確認で同じ確かめ方をするなら、この手順を使うとよい
2. **止まった時間**: 降ろし始め（`ghost_switch_down_ms` の時刻から 5 ms を引いた時刻）から emo2 の窓まで、本走行は 1,103 ms、前確認の走行は 593 ms だった（どちらも debug 版）。時間の大半は `windows_closed_for_restart` から `ghost_switch_booted` まで（emo2 の構成の読み取りとシェルの焼き）。本仕様の要件ではなく、前回の同じ切替（R_POST → emo2・496 ms）と同じ区間のばらつきとして記す
3. `\+` は正典どおり `raise_event=false` なので、検体に `OnGhostChanging` は送られず、`OnGhostChanged` の Ref1（送り出しの台本）は空だった。emo2 は 204 を返して `OnBoot` へ続いた（完了 `ghost-shell-balloon-switch` と同じ経路）

## 開発者が自分の目で見直す手順

1. i686 の helper と `areka` を建て、上の「検体」の形で `C:\tmp\` の下に根を組む（目録は emo2 と検体の 2 体だけ）
2. PowerShell で `$env:AREKA_PROFILE_DIR='<新しいフォルダ>'; Remove-Item Env:AREKA_ROOT -ErrorAction SilentlyContinue` とし、そのフォルダに `sylphya.toml`（`format-version = 1`・`[last]`・`ghost = "rpost_plus"`）を置いて、根の `areka.exe` を argv なしで起動する
3. ポストと狛犬の窓が一瞬出て、何も話さずに消え、emo2 が現れて挨拶する。`$env:RUST_LOG='info,areka=debug'` で起動すれば `ghost_switch_resolved name=random to=emo2` が見える
