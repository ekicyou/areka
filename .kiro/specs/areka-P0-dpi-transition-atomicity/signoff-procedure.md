# 実機サインオフ手順書（areka-P0-dpi-transition-atomicity）

対応要件: **Requirement 8**（8.1 起動コマンド・target・水準・自動終了・判定語／8.2 充足＝遷移回数／8.3 機械判定／8.4 目視所見の併記／8.5 消灯した観測点を「発生 0 回」の根拠にしない）
対応設計: **C10 サインオフ手順書**・**C7 `transition_judge`**（判定器とランナー）

本書は「採取 → 判定 → 記録」を一巡させる手順だけを定める。結論（残存か縮退か・是正候補の採否）は書かない——それは task 4.3（確定台帳）と task 7.3（サインオフ実施）の仕事である。

---

## 0. 大前提（読む前に必ず）

### 0.1 本書は 2 度使う（採取ビルドの取り違えを禁じる）

| 実施 | タスク | 採取ビルド | 位置づけ |
| --- | --- | --- | --- |
| 1 回目 | task 4.2 | **是正未投入**（群 1〜3 の観測基盤だけが入ったコミット） | 是正前の**基準値**を採る |
| 2 回目 | task 7.3 | 是正投入後（群 5・群 7 まで着地したコミット） | 合否を出す |

**どちらの実施かを記録の先頭に必ず書く。** 是正前のログに対して合格が出ることはない（是正前は違反が出るのが正しい）ので、「PASS が出た」だけでは何も言えない。

### 0.2 本書の判定語は実コードから転記してある

本書に現れる観測レコードの語（`kind=` の値・`stage=` の値・フィールド名）は、次の**単一定義元**からの転記である。自分で言い換えたり短縮したりしてはならない。

| 語の種類 | 単一定義元 |
| --- | --- |
| レコード種別・段階・フィールド名・観測 target・行頭タグ（窓・モニタ側） | `crates/wintf/src/ecs/window/transition_diag.rs` |
| 配置側のレコード種別とフィールド名 | `crates/areka/src/placement/transition_diag.rs` |
| サーフェス更新の語 | `crates/areka-emo-present/src/presenter/transition_record.rs`（`presenter::` 再輸出） |
| 上限・違反・合否の語・環境変数名 | `crates/areka/src/placement/transition_judge_verdict.rs` |

一致は目視ではなく実行テストが固定している——`crates/areka/src/placement/transition_signoff_procedure_tests.rs` が本書を読み、⑴ 本書に載る観測行の例が発行側の語彙だけで書かれていること、⑵ 発行側のレコード種別 10 種が**すべて**本書に現れること、⑶ ランナーの入口の語が字面で載っていること、⑷ Report の出力例に並ぶ違反の行が、その上限系統で実際に出得るものであることを検査する。**本書の語を変えるなら、同じコミットでこのテストを通すこと。** 通らなければ本書は静かに嘘になる。

### 0.3 消灯した観測点を「発生 0 回」の根拠に用いない（要件 8.5・本書が制度化する唯一の禁止）

観測行が 0 件であることには 2 つの原因がある——**本当に起きていない**のと、**観測点が点いていない**のとである。この 2 つを区別せずに「0 件だから違反なし」と書くのは、本仕様が捕まえようとしている欠陥をそのまま見逃す形である。

ゆえに:

- 「0 件」を根拠に使う前に、§3 の点灯表でその観測点が**この採取で点いていた**ことを確かめる。
- §3.2 の「現時点のビルドでは 1 行も出ない観測点」については、**0 件を根拠に使ってはならない**。
- 判定器も同じ規則で作ってある——観測行が 1 行も無いログ、上限を 1 つも当てない組、フレーム番号が一様に 0 の系列は、いずれも**合格ではなく失敗**として落ちる。

---

## 1. 準備

### 1.1 パス変数（以降すべて絶対パスで扱う）

ゴースト一式を**相対パス**で渡すと `pasta.dll` の LOAD が `0x8007007E` で失敗する。位置引数も環境変数もすべて絶対パスで与えること。

```powershell
# リポジトリ（ワークツリー）ルート。自分の環境の実値へ置き換える。
$REPO    = (git rev-parse --show-toplevel).Replace('/', '\')
# 本書作成時点の実値の例:
#   C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb

$AREKA   = "$REPO\target\release\areka.exe"
$GHOST   = "$REPO\crates\pilot\examples\shiori-host-32\fixtures\emo2"
$BALLOON = "$REPO\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku"
```

起動前に `$GHOST\ghost\master\descript.txt` が実在することを確認する。位置引数は `argv[1]=ghost_root`・`argv[2]=balloon_root` である（`crates/areka/src/boot_config.rs:52-65` の `resolve_config_inputs`）。

### 1.2 ビルド（release・x64 本体 ＋ i686 ホスト成果物）

```powershell
cargo build --release -p areka --bin areka
cargo build --release -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item "$REPO\target\i686-pc-windows-msvc\release\shiori-host32-helper.exe" "$REPO\target\release\" -Force
```

- **プロファイルは `release` を使う。** 実機専用の判定量（可視化から窓書込までの経過・一括書込の総所要）は µs の実時間であり、`debug` ビルドの値は上限の目安（画面の更新周期 1〜2 回分）と比較できない。第 1 段再観測も `release` で採ってあり（`reobservation-2026-08-15.md` §1）、比較可能性のためにも合わせる。
- **32bit ホスト成果物は必須扱いとする。** `default_helper_exe_path()`（`crates/areka/src/boot_config.rs:77-84`）は**実行ファイル隣**の `shiori-host32-helper.exe` を解決するので、`target\release\` へ置くこと。helper が無くてもキャラ窓は出るが**バルーン窓は発話が来るまで出ない**——随伴バルーンの同一フレーム性（要件 4.3）の判定材料が採れない。
- ビルドに使ったコミット SHA を控える（§7 の記録に要る）。

### 1.3 実機構成の要件

- ゴーストが載るモニタで **OS 設定の拡大率を切り替えられる**こと。第 1 段再観測は主モニタ 2880×1800 を **200% ↔ 100%**（dpi 192 ↔ 96）で往復した。
- **本仕様のスコープは OS 設定からの拡大率変更のみ**である（`requirements.md` の Out of Scope）。キャラ窓を拡大率の異なる別モニタへ**ドラッグで移す**遷移は対象外——寸法の追従は成立済みと確認されており、本書はそちらを採らない。ただし本仕様の是正がその場面を壊していないことは別途（要件 10.7 の回帰テスト）で見る。
- 採取中は**画面ロック・スリープ・リモートデスクトップ接続を行わない**（モニタ列挙が変わり、別事象が混入する）。
- 2 体（2 スコープ）を起動する構成で採ること。連鎖（隣接）と随伴バルーンの判定はスコープが 2 つ無いと成立しない。

### 1.4 profile ディレクトリをセッションごとに新品にする

位置永続が前回の位置を復元すると初期配置が非決定になり、接地点差の読みが汚れる。

```powershell
$PROFILE_DIR = "$env:LOCALAPPDATA\areka-diag\atom-profile-$(Get-Date -Format yyyyMMdd-HHmmss)"
New-Item -ItemType Directory -Force -Path $PROFILE_DIR | Out-Null
```

環境変数 `AREKA_PROFILE_DIR` で切り替わる（`crates/areka/src/boot_config.rs:95`）。

---

## 2. 起動設定

### 2.1 ログ directive（**D-ATOM**・要件 8.1 の中核）

```
info,wintf::transition=debug,areka::placement::diag=debug,areka_emo_present=debug
```

各セグメントが**実際に**何を開くか。設計 C10 の字面をそのまま採るが、意味は次のとおりであって「crate ごとに 1 セグメント要る」という意味ではない。

| セグメント | 開くもの | 省くと |
| --- | --- | --- |
| `info` | 既定水準（`crates/areka/src/main.rs:128` のフォールバックと同値）。`warn!`／`error!` は常に通る | 縮退や上限超過の警告が消える |
| **`wintf::transition=debug`** | **遷移観測チャネルの全レコード**（§3.1 の観測点はすべてここで点く）。3 crate が同じ target へ出す（`crates/wintf/src/ecs/window/transition_diag.rs:54` の target 定数と `:604-605` の唯一の発行点） | **§3 の点灯表が丸ごと暗転する**（下記 §2.2） |
| `areka::placement::diag=debug` | 配置診断チャネル（`[diag.window_move]`・`[diag.monitor_snapshot]`・`[diag.monitor]`。target 定数は `crates/areka/src/placement/diag.rs:69`） | スコープとキャラ窓 entity の対応表・起動時モニタ列挙が採れない（§4.3 の裏取りが不能） |
| `areka_emo_present=debug` | 段階別計時行 `perf(apply_show): 段階別計時`（末尾に `frame=` を持つ・`crates/areka-emo-present/src/presenter/timing.rs:201-220`。target はモジュールパス既定） | 描画側の所要をフレーム番号で突き合わせられない |

### 2.2 `wintf::transition=debug` を落としてはならない

観測チャネルの target は **`wintf::transition`** という**明示リテラル**であって、モジュールパス（`wintf::ecs::window::transition_diag`）ではない。`EnvFilter` の target 照合は素の文字列前方一致なので:

- `wintf::ecs::window=debug` は `wintf::transition` の接頭辞に**ならない**——1 行も出ない。
- `wintf::ecs::window::transition_diag=debug` も**当たらない**。
- `wintf=debug` なら当たる（が、無関係な行が大量に混ざるので使わない）。

**第 1 段再観測（`reobservation-2026-08-15.md` §1）の `RUST_LOG` はこのセグメントを持たない。** 観測チャネル自体が当時まだ存在しなかったためであり、あのログに `kind=` の行が 1 件も無いのは正常である。**2 つの採取を「観測点の点灯」の面で比較してはならない。**

### 2.3 有界自動終了（唯一の終了経路）

環境変数 **`AREKA_APP_SMOKE_EXIT_MS`**（`crates/areka/src/main.rs:777` の定数・ゲートの配線は `:677` 以降）。値はミリ秒の非負整数で、指定 ms 後に起動窓を despawn して正常終了する。空・非数値・負値・溢れは**ゲート OFF**（自動終了しない）。

- 既定として **`480000`（8 分）** を用いる。第 1 段再観測が同じ値で §4 の下限を踏破している。**閾値でも必須値でもない**——操作に要する時間を上回れば足りる。
- **窓を閉じる操作で終えてはならない。** areka のゴーストはクリック（ダブルクリック）で閉じる設計であり、押下の瞬間に配置系がドラッグ準備へ入る。§4.4 の無効化チェックが必ず失敗する。
- Ctrl+C で終えてもならない（終了時の観測行が欠ける）。

### 2.4 ログ保存先

**リポジトリ配下へ生ログを置かない**（巨大かつ実機固有・コミット対象外）。

```powershell
$RUN    = Get-Date -Format yyyyMMdd-HHmmss
$LOGDIR = "$env:LOCALAPPDATA\areka-diag\atom-$RUN"
New-Item -ItemType Directory -Force -Path $LOGDIR | Out-Null
$LOG    = "$LOGDIR\atom-signoff.log"
```

併せて `$LOGDIR\meta.txt` に §7 の記録票を書く。

### 2.5 起動コマンド（絶対パス・そのまま貼れる形）

```powershell
$env:RUST_LOG                = "info,wintf::transition=debug,areka::placement::diag=debug,areka_emo_present=debug"
$env:AREKA_APP_SMOKE_EXIT_MS = "480000"
$env:AREKA_PROFILE_DIR       = $PROFILE_DIR

& $AREKA $GHOST $BALLOON 2>&1 | Tee-Object -FilePath $LOG
```

`2>&1 | Tee-Object` で標準出力・標準エラーを 1 本にまとめて保存しつつ画面でも見る（目視所見を取りながら採るため画面表示は残す）。

---

## 3. 観測点 × ログ target × 水準 の点灯表（要件 8.5）

すべての遷移観測レコードは行頭タグ `[transition]` と接頭語 `frame=<u32> t_us=<u64> kind=<種別>` を持ち、フィールドは `名前=値` の空白区切り、値が無いものは番兵 `-` で埋まる。

### 3.1 D-ATOM で点灯する観測点（発行点が着地済み）

| # | `kind=` | 発行点（file:line） | ログ target | 水準 | この観測点で読むもの |
| --- | --- | --- | --- | --- | --- |
| T1 | `kind=monitor` | `crates/wintf/src/ecs/layout/systems/monitor_systems.rs:347-348` | `wintf::transition` | `debug` | **遷移の起点**。`old_dpi`／`new_dpi` が異なる行が 1 遷移の先頭 |
| T2 | `kind=enqueue` | `crates/wintf/src/ecs/window/command.rs:251-252` | 同上 | `debug` | 窓へのジオメトリ指令が積まれた点。`merged_into_seq` が合流の先着 seq |
| T3 | `kind=flush`（`stage=begin`／`stage=end`） | `crates/wintf/src/ecs/window/command.rs:309` / `:364` | 同上 | `debug` | 一括書込 1 区間。`stage=end` の `total_us` が**実機専用の判定量**の 1 つ |
| T4 | `kind=write`（`stage=flush`） | `crates/wintf/src/ecs/window/command.rs:335` | 同上 | `debug` | 窓ごとの書込回数・書込フレーム・書込後矩形（`ax`〜`ah`）・`call_us` |
| T5 | `kind=write`（`stage=sync`） | `crates/wintf/src/ecs/window_proc/window_pos.rs:468` | 同上 | `debug` | **経路 A の裏取り**（OS 提案位置の同期書込）。`origin=dpi-suggested` と対で読む |
| T6 | `kind=msg` | `crates/wintf/src/ecs/window_proc/window_pos.rs:59` / `:336`・`crates/wintf/src/ecs/window_proc/lifecycle.rs:140` | 同上 | `debug` | 窓書込の内側で OS 同期処理が走っているか（`in_swp`・`since_flush_us`） |
| T7 | `kind=surface`（`stage=upload`／`visualize`／`skipped`） | `crates/areka-emo-present/src/presenter/show.rs:349` / `:390`・`crates/areka-emo-present/src/presenter/refresh.rs:83` / `:100` | 同上 | `debug` | 描画内容が新しい寸になった時刻。**窓矩形との食い違い**を測る片側 |
| T8 | `kind=ground` | `crates/areka/src/placement/follow/window_move.rs:341` → `crates/areka/src/placement/transition_diag.rs:525` | 同上 | `debug` | 接地点と作業領域下端の差（`diff`）。下端吸着のキャラ窓のみ |
| A1 | `[diag.window_move] route= entity= kind= scope= …` | `crates/areka/src/placement/diag.rs` | `areka::placement::diag` | `debug` | スコープ ↔ キャラ窓 entity の対応（§4.3 の裏取り） |
| A2 | `perf(apply_show): 段階別計時`（末尾に `frame=`） | `crates/areka-emo-present/src/presenter/timing.rs:201-220` | `areka_emo_present` | `debug` | 描画側の段階別所要をフレーム番号で突き合わせる |

### 3.2 現時点のビルドでは 1 行も出ない観測点（発行点が未着地）

語彙（レコードを組む純関数と定数）は着地しているが、**呼び出す発行点がまだ無い**。`RUST_LOG` を何にしても出ない。

| `kind=` | 純関数 | 発行点が入るタスク |
| --- | --- | --- |
| `kind=snapshot` | `crates/areka/src/placement/transition_diag.rs:321` | task 5.1（作業領域源を実行時に同期する） |
| `kind=hold` | `crates/areka/src/placement/transition_diag.rs:343` | task 5.4（拡大率と表の整合待ちを設ける） |
| `kind=chain` | `crates/areka/src/placement/transition_diag.rs:379` | task 5.6（遷移後に連鎖を一度だけ解き直す） |

> **task 4.2（是正前の基準値）の採取では、この 3 種は必ず 0 件になる。** これを「整合待ちは 1 度も起きなかった」「連鎖の解き直しは 0 回だった」と読んではならない——**観測点が無いだけ**である（要件 8.5）。task 7.3 の採取では 3 種とも点いているはずなので、**まず 3 種が 1 行でも出ていることを確かめてから**件数の議論に入る。

### 3.3 観測レコードの書式（**形の例**・値は実測ではない）

各種別の必須フィールドが全部載った形を 1 本ずつ示す。実機ログでは行頭に `DEBUG wintf::transition: ` が付く。

```
[transition] frame=118 t_us=412 kind=monitor entity=5v0 old_dpi=192 new_dpi=96 old_wa=0,0,2880,1704 new_wa=0,0,1440,852
[transition] frame=118 t_us=455 kind=snapshot monitors=2 m0=96:0,0,1440,852 m1=144:-1707,0,0,1067
[transition] frame=117 t_us=980 kind=hold entity=9v0 scope=0 win_kind=char window_dpi=192 table_dpi=96 since_frame=117 decision=hold site=dpi
[transition] frame=118 t_us=1180 kind=surface stage=upload target_id=0 w=382 h=537 resized=true reason=-
[transition] frame=118 t_us=1204 kind=surface stage=visualize target_id=0 w=382 h=537 resized=- reason=-
[transition] frame=250 t_us=90 kind=surface stage=skipped target_id=1 w=- h=- resized=- reason=k-unchanged
[transition] frame=118 t_us=1402 kind=enqueue hwnd=0x1234 origin=DpiReproject scope=0 win_kind=char merged_into_seq=-
[transition] frame=118 t_us=1500 kind=flush stage=begin count=4 since_tick_us=1450 total_us=-
[transition] frame=118 t_us=1512 kind=write stage=flush seq=0 hwnd=0x1234 origin=DpiReproject scope=0 win_kind=char x=1120 y=315 cx=382 cy=537 flags=0x14 ax=1120 ay=315 aw=382 ah=537 call_us=6100 ok=true
[transition] frame=118 t_us=1520 kind=ground scope=0 ground_y=852 wa_bottom=852 diff=0 route=DpiReproject
[transition] frame=118 t_us=1530 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x1234 in_swp=true since_flush_us=30
[transition] frame=118 t_us=1786 kind=flush stage=end count=4 since_tick_us=1450 total_us=286
[transition] frame=118 t_us=1800 kind=chain stage=realigned scopes=2 moved=1 reason=-
```

読み方の要点:

- **`win_kind=` が窓の種別**（`char`／`balloon`／読めなければ `-`）。接頭語の `kind=` は**レコード種別**であって窓の種別ではない。1 行に同じ名前を 2 度出さないための命名であり、混同すると遷移の切り出しそのものが壊れる。
- **窓の写像**は `kind=surface` の `target_id` の偶奇で決まる（偶数＝キャラ窓／奇数＝バルーン窓、スコープは `target_id / 2`）。`kind=surface` の行はスコープも種別も運ばない。
- `t_us` は tick 開始からの経過であってフレーム間で連続しない。**判定にはフレーム番号を使い、`t_us` は同一フレーム内の順序と所要にだけ使う。**

### 3.4 消灯・未着地を「発生 0 回」の根拠にしない

採取の冒頭で、次を確かめてから先へ進む。**どれか 1 つでも 0 件なら、その採取では対応する観測点を根拠に使えない。**

```powershell
foreach ($k in 'monitor','enqueue','flush','write','msg','surface','ground','snapshot','hold','chain') {
  $n = (Select-String -Path $LOG -SimpleMatch "kind=$k" | Measure-Object).Count
  "{0,-9} {1}" -f $k, $n
}
```

`monitor`〜`ground` の 8 種が 0 件なら **`wintf::transition=debug` の入れ忘れ**を最初に疑う（§2.2）。`snapshot`／`hold`／`chain` の 0 件は §3.2 のとおり是正前では正常である。

---

## 4. 充足条件（要件 8.2）

### 4.1 充足条件は経過時間ではなく**遷移回数**

**キャラ窓の各スコープについて、低→高・高→低の各方向 3 回以上**の拡大率遷移をログから数えられること。2 体（2 スコープ）構成なら遷移の下限は 6 回（両スコープが同一モニタに載っていれば、1 回の拡大率変更で両スコープが同時に遷移する）。

「8 分回した」「何度も切り替えた」は充足の根拠にならない。**数えた数字だけが根拠である。**

### 4.2 遷移の数え方（判定器と同じ規則）

遷移の起点は **`kind=monitor` かつ `old_dpi` と `new_dpi` が異なる行**であり、次の起点の直前までが 1 遷移である（判定器の切り出しと同一規則）。方向は同一行の `old_dpi` と `new_dpi` の大小で決める（`new > old` が低→高、`new < old` が高→低、等しければ**どちらにも数えない**）。

```powershell
$records = Get-Content $LOG | Where-Object { $_ -match '\[transition\]' }
$transitions = New-Object System.Collections.Generic.List[object]
$current = $null
foreach ($line in $records) {
  if (($line -match '\bkind=monitor\b') -and
      ($line -match '\bold_dpi=(\d+)\b')) {
    $old = [int]$Matches[1]
    if ($line -match '\bnew_dpi=(\d+)\b') {
      $new = [int]$Matches[1]
      if ($old -ne $new) {
        $current = [pscustomobject]@{
          Frame  = [int]([regex]::Match($line, '\bframe=(\d+)\b').Groups[1].Value)
          Old    = $old
          New    = $new
          Scopes = (New-Object System.Collections.Generic.List[string])
        }
        $transitions.Add($current) | Out-Null
      }
    }
  }
  if ($current -and ($line -match '\bkind=write\b') -and ($line -match '\bwin_kind=char\b') -and
      ($line -match '\bscope=(\d+)\b') -and (-not $current.Scopes.Contains($Matches[1]))) {
    $current.Scopes.Add($Matches[1])
  }
}

$low2high = ($transitions | Where-Object { $_.New -gt $_.Old }).Count
$high2low = ($transitions | Where-Object { $_.New -lt $_.Old }).Count
"ATOM-QUOTA: transitions=$($transitions.Count) low2high=$low2high high2low=$high2low"
$transitions | Format-Table Frame, Old, New, @{n='Scopes';e={ $_.Scopes -join ',' }}
```

判定語 **`ATOM-QUOTA: PASS`**（`low2high >= 3` かつ `high2low >= 3` かつ §4.3 が PASS）／**`ATOM-QUOTA: FAIL`**。

### 4.3 各スコープが実際に遷移したことの確認

上の表の `Scopes` 列に、**起動しているキャラ窓のスコープ番号がすべて**現れていること（各方向 3 回以上の遷移それぞれについて）。現れないスコープがあれば、そのスコープは低→高／高→低の回数に数えない。

スコープと entity の対応は `areka::placement::diag` の `[diag.window_move]` 行（`kind=char` の行の `scope=` と `entity=`）で裏取りできる。**あるスコープが 1 度も現れないとき、それを「そのスコープは書込 0 回で済んだ」と読んではならない**——キャラ窓がそもそも生成されていない可能性と区別できない。まず `[diag.window_move]` でそのスコープのキャラ窓が実在することを確かめる。

### 4.4 無効化チェック（ドラッグ・クリック禁止）

採取中は**ゴースト窓を一切マウスで掴まない。** ドラッグはもちろん、ゴースト上での左ボタン押下も行わない。

- **理由（要件 8.2 の観測条件）**: ドラッグはキャラ窓の位置を配置系の別経路（追従）で書き換える。その書込は `kind=write` に現れ、遷移 1 回あたりの窓書込回数の上限判定へそのまま混ざる。混ざった時点で「遷移が余分に書いたのか、人が動かしたのか」が原理的に区別できなくなる。単クリックも押下の瞬間にドラッグ準備へ入るので同じである。
- 拡大率の変更は「設定 › システム › ディスプレイ › 拡大縮小」から行う。

```powershell
$drag = (Select-String -Path $LOG -SimpleMatch '[start_preparing]' | Measure-Object).Count
if ($drag -eq 0) { 'ATOM-NO-DRAG: PASS' } else { "ATOM-NO-DRAG: FAIL ($drag 件)" }
```

> **この判定語は `warn` 以下の水準に依存しない**が、`[start_preparing]` はドラッグ層の `debug` 行である。**D-ATOM はドラッグ層の target を開いていない**ので、この検査は「消灯している観測点の 0 件」に当たる（§0.3）。ゆえに**この 1 語だけは目視の宣言で代える**——「採取中にゴースト窓へ一切触れていない」ことを採取者が §7 の記録票へ明記し、上のコマンドは補助として回す。ドラッグ層を開いて `ATOM-NO-DRAG` を機械判定にしたい場合は、D-ATOM に `wintf::ecs::drag=debug` を足したうえで**足したことを記録票に書く**（directive を変えた採取は前回と直接比較できない）。

---

## 5. 採取セッションの手順

1. §1.2 のビルドを行い、コミット SHA を控える。
2. §1.4 の新品 profile ディレクトリと §2.4 のログ保存先を作る。
3. §2.5 の起動コマンドを実行する。ゴースト 2 体とバルーンが出るまで待つ。
4. §3.4 の点灯確認を（起動直後のログに対して）1 度回し、8 種が点いていることを確かめる。**点いていなければここで中止し、directive を直して採り直す。**
5. **ゴースト窓に一切触れずに**、OS 設定でゴーストの載るモニタの拡大率を **200% ↔ 100% で 3 往復以上**切り替える。1 回切り替えるごとに数秒待ち、窓の動きが落ち着いてから次へ進む。
6. 各切り替えの直後、**目視所見**を `$LOGDIR\meta.txt` へ書く（§6.5 の様式）。少なくとも「跳ね（旧寸の窓が一瞬見える／窓が段階的に動く）の有無」と「二体の隙間の有無」と「接地点の浮きの有無」を毎回記録する。
7. **`AREKA_APP_SMOKE_EXIT_MS` の自動終了を待つ。これが唯一の正規終了経路である**（§2.3）。
8. §3.4・§4.2・§4.4 を回し、`ATOM-QUOTA` と `ATOM-NO-DRAG` を記録する。充足していなければ**採り直す**（同じログへ追記しない）。

---

## 6. 判定（要件 8.3・8.4）

### 6.1 ランナーの実行

判定は決定論テストと**同一の純関数**で行う。判定の実装をここで書き直さない（別実装を持った瞬間に「決定論テストが緑でもサインオフだけ別の判定を通る」形が生まれる）。

```powershell
$env:AREKA_TRANSITION_LOG = $LOG
cargo test -p areka transition_signoff -- --ignored --nocapture
```

- 環境変数名は **`AREKA_TRANSITION_LOG`**、値は**絶対パス**。
- `--ignored` は無視指定のテストだけを走らせるので、このフィルタで走るのは **`judges_a_real_machine_transition_log`** 1 本だけである。
- **静かに成功しない**: 環境変数が無い・パスが読めない・フォルダを渡した・観測行が 1 行も無い——いずれも合格ではなく**失敗**として落ちる。「違反 0 件」という出力が不備なパスから出ることはない。
- 出力（`--nocapture`）を**全文**保存して §7 へ添付する。

### 6.2 Report が刷るもの・刷らないもの

出力は次の形である。

```
[judge] records=<解析できた観測行> transitions=<遷移の本数> unassigned=<最初の起点より前の行> (malformed <うち語彙違反>)
  transition #1 frame=<起点フレーム> dpi <old> -> <new> records=<この遷移の行数>
    deterministic: 1 件の違反
      - 可視化と書込のフレーム差 1 > 上限 0（scope=0 win_kind=char）
    signoff: 1 件の違反
      - 一括書込の総所要 286000µs > 上限 16700µs
```

- **上限は 2 系統あり、別々に評価して両方が並ぶ。** `deterministic` は回帰テストが固定する決定論の量——起点から最終書込までのフレーム差・**可視化と窓書込のフレーム差**・**随伴バルーンの同一フレーム性**・窓ごとの書込回数・経路 A の件数（札と段の両方）・接地点差・連鎖の回数。`signoff` は**実機専用**の量——可視化から窓書込までの経過（µs）・一括書込の総所要（µs）で、非決定ゆえ回帰テストでは値を固定しない。**どちらか一方の `PASS` を全体の合格と読まない。**
- **フレームを単位に測る量はすべて `deterministic` 側である。** 「可視化と書込のフレーム差」と「随伴バルーンが別フレームで書かれた」は名前が実機の症状に似ているが、`signoff` の下には**出得ない**（`signoff` はフレーム差の上限を 1 つも当てないため）。同じ食い違いを µs で測るのが `signoff` 側の「可視化から書込まで」であり、**両者は別の量である**。§6.6 の 2 行を埋めるとき、違反行がどちらのブロックの下にあったかで系統を判断すること——文言から推測しない。
- **`PASS` の系統については、量そのものは刷られない。** 違反があった量だけが実測値つきで並ぶ。task 4.2 は「是正前の基準値」を残すのが目的なので、**`PASS` になった量も §6.3 で生ログから起こして記録する。**
- 実機専用の上限（`Bounds::signoff` の 2 値）は本書作成時点で**暫定値**（`16700` µs＝画面更新周期 60Hz 1 回分）である。**確定値は task 4.3 が task 4.2 の実測から決めて確定台帳へ根拠つきで登記し、判定器の定数を差し替える。** 暫定値のままの `signoff: PASS` を「所要は上限内だった」と読んではならない。
- 整合待ち（`kind=hold`）を含む遷移だけがフレーム差の許容に待ちフレーム数を足せる。この許容も本書作成時点では判定器側の**暫定値**であり、本番定数は task 5.4 で入る。

### 6.3 判定量を生ログから起こす（記録用）

`PASS` の系統も含めて数字を残すため、遷移ごとに次を集計する（§4.2 のスクリプトで得た遷移の区切りをそのまま使う）。

| 記録する量 | 生ログからの起こし方 |
| --- | --- |
| 起点から最終書込までのフレーム差 | 起点行の `frame=` と、その遷移の最後の `kind=write` 行の `frame=` の差 |
| 窓ごとの書込回数 | その遷移の `kind=write` を `scope=` と `win_kind=` で数える |
| 経路 A の件数 | その遷移の `kind=write` のうち `origin=dpi-suggested` の件数と、`stage=sync` の件数（**両方**。食い違えば札か段のどちらかが壊れている） |
| 随伴の同一フレーム性 | キャラ窓の `kind=write` の `frame=` と、同一スコープのバルーンの `origin=BalloonFollow` の `kind=write` の `frame=` が一致するか |
| 可視化と窓書込の食い違い（フレーム） | 当該窓の `kind=surface stage=visualize` の `frame=` と、当該窓の `kind=write` の `frame=` の差（見送り窓は除く）＝`deterministic` 側の量 |
| 可視化と窓書込の食い違い（時間） | 同一フレームの `kind=surface stage=visualize` の `t_us` と、当該窓の `kind=write` の `t_us` の差（µs）＝`signoff` 側の量。窓ごとに遷移区間内の**最大**を採る |
| 一括書込の総所要 | `kind=flush stage=end` の `total_us` |
| 接地点差 | `kind=ground` の `diff`（負＝浮き） |
| 連鎖の解き直し回数 | その遷移の `kind=chain stage=realigned` の件数（§3.2 のとおり task 5.6 まで 0 件） |

### 6.4 フレーム番号が一様に 0 の系列は「1 フレームで完了」と読まない

`frame=0` は tick が始まる前の縮退値である。**系列の `frame` が全部 0 のログは、フレーム差が 0 なのではなく判定不能である**（要件 8.5）。判定器はこの形を検出して `フレーム差が判定不能（一様に 0／読めない）` という違反を立てる。この違反が出たログで「有界フレーム数以内だった」と書いてはならない——**観測基盤の刻印が壊れている**ので、原因を直してから採り直す。

### 6.5 目視所見の併記（要件 8.4）

**目視所見は機械判定と対等の入力である。** 拡大率の切り替え 1 回ごとに次の様式で記録する。

```
# meta.txt の目視所見
[遷移 1] 200% -> 100%  跳ね: あり（旧寸のキャラが一瞬残って見えた）
                       二体の隙間: あり（左のキャラとの間が空いた）
                       接地点: 浮き あり（足元がタスクバーの上に浮いた）
[遷移 2] 100% -> 200%  跳ね: なし
                       二体の隙間: なし
                       接地点: 浮き なし
```

**機械判定と目視所見が食い違う場合は合格としない。** 食い違いには 2 方向あり、どちらも不合格である。

| 食い違いの向き | 意味 | 採るべき行動 |
| --- | --- | --- |
| 判定 PASS ／ 目視で跳ねあり | 判定量が症状を捕まえていない | 上限か判定量の側を疑い、確定台帳へ「判定に載らない症状」として登記する |
| 判定 FAIL ／ 目視で跳ねなし | 上限が実機の性質に対して厳しすぎるか、観測が壊れている | 上限の根拠を確定台帳で見直す。**目視に合わせて上限を緩めてはならない**（観測装置を被検体に合わせて曲げる行為に当たる） |

### 6.6 合否の書き方

```
ATOM-SIGNOFF: PASS|FAIL
  ATOM-QUOTA:        PASS|FAIL (transitions=N low2high=N high2low=N scopes=…)
  ATOM-NO-DRAG:      PASS|FAIL
  DETERMINISTIC:     PASS|FAIL (違反 N 件)
  SIGNOFF-BOUNDS:    PASS|FAIL (違反 N 件・上限は暫定/確定)
  VISUAL:            跳ねなし|跳ねあり（遷移ごとの内訳は meta.txt）
  AGREEMENT:         一致|食い違い
```

**`ATOM-SIGNOFF: PASS` は上の 6 項目がすべて PASS ／ 跳ねなし ／ 一致のときに限る。**

---

## 7. 記録（受け渡し契約）

生ログ・Report 全文・meta.txt は**リポジトリ外**（§2.4 の `$LOGDIR`）へ置く。仕様の文書へは**引用と数字だけ**を転記し、生ログの保存パスと採取条件を併記する。

`$LOGDIR\meta.txt` の記録票:

| 項目 | 形式 |
| --- | --- |
| 実施区分 | task 4.2（是正前の基準値） ／ task 7.3（合否） |
| コミット SHA | `git rev-parse HEAD` の値 |
| ビルドプロファイル | `release`（変えたなら実値と理由） |
| ビルドコマンド | §1.2 のとおり ／ 変えたなら実コマンド |
| 実機構成 | モニタ台数・各モニタの解像度と拡大率（dpi）・作業領域下端・主副・ゴーストの載るモニタ |
| OS | `[System.Environment]::OSVersion.Version` の値 |
| `RUST_LOG` | 実際に与えた文字列（**§2.1 から変えたなら必ず明記**） |
| `AREKA_APP_SMOKE_EXIT_MS` | 実値・終了経路が自動終了であったこと |
| profile | 新品ディレクトリの実パス |
| 採取日時 | 開始・終了 |
| ゴースト／バルーン | 絶対パス |
| 生ログ | 絶対パス・行数 |
| 点灯確認 | §3.4 の 10 行の出力そのまま |
| 充足 | `ATOM-QUOTA:` の 1 行＋遷移一覧（フレーム・old・new・スコープ） |
| 無効化チェック | `ATOM-NO-DRAG:` の 1 行＋「窓へ触れていない」旨の宣言 |
| 判定 | ランナーの `--nocapture` 出力全文 |
| 判定量 | §6.3 の表を遷移ごとに埋めたもの |
| 目視所見 | §6.5 の様式（遷移ごと） |
| 合否 | §6.6 の 7 行 |

---

## 8. 本書のメンテ規約

- 観測レコードの語（`kind=`・`stage=`・フィールド名）と、ランナーの入口の語（環境変数名・観測 target・行頭タグ・Report の 2 系統名と合格語・ランナーのテスト名）、および **§6.2 の Report 出力例に並ぶ違反行がどちらの上限系統に属するか**は、**`crates/areka/src/placement/transition_signoff_procedure_tests.rs` が本書を読んで一致を検査している**（§0.2 の検査 ⑴〜⑷）。発行側の語を変えるなら、同じコミットで本書も直すこと。とりわけ §6.2 の出力例へ違反行を足す・動かすときは、その系統で実際に出得る違反かをテストが確かめるので、通してから報告すること。
- §3.2 の「発行点が未着地」の表は、task 5.1／5.4／5.6 が着地したら**その行を §3.1 へ移し、発行点の file:line を埋める**。移し忘れると、点いている観測点を「出ないもの」として扱い続けることになる。
- §6.2 の実機専用の上限が task 4.3 で確定値へ差し替わったら、「暫定」の記述を実値と根拠へ書き換える。
- file:line の参照は本書作成時点（群 1〜3 着地）のものである。ずれたら本書を直す——ずらしたまま放置すると、本書は読めるが辿れない文書になる。
