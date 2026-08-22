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
| T2 | `kind=enqueue` | `crates/wintf/src/ecs/window/command.rs:671-678` | 同上 | `debug` | 窓へのジオメトリ指令が積まれた点。`merged_into_seq` が合流の先着 seq |
| T3 | `kind=flush`（`stage=begin`／`stage=end`） | `crates/wintf/src/ecs/window/command.rs:739-746` / `:796-803` | 同上 | `debug` | 一括書込 1 区間。`stage=end` の `total_us` が**実機専用の判定量**の 1 つ。**バッチ化（task 7.2）以後、窓が実際に動く時間はここへ集まる** |
| T4 | `kind=write`（`stage=flush`） | `crates/wintf/src/ecs/window/command.rs:777-792` | 同上 | `debug` | 窓ごとの書込回数・書込フレーム・書込後矩形（`ax`〜`ah`）・`call_us`・**`in_batch`**（`DeferWindowPos` の 1 バッチで投入されたか・task 7.2）。`in_batch=true` のとき `call_us` は**投入だけ**の所要である |
| T5 | `kind=write`（`stage=sync`） | `crates/wintf/src/ecs/window_proc/window_pos.rs:467-491` | 同上 | `debug` | **経路 A の裏取り**（OS 提案位置の同期書込）。`origin=dpi-suggested` と対で読む |
| T6 | `kind=msg` | `crates/wintf/src/ecs/window_proc/window_pos.rs:59` / `:336`・`crates/wintf/src/ecs/window_proc/lifecycle.rs:140` | 同上 | `debug` | 窓書込の内側で OS 同期処理が走っているか（`in_swp`・`since_flush_us`） |
| T7 | `kind=surface`（`stage=upload`／`visualize`／`skipped`） | `crates/areka-emo-present/src/presenter/show.rs:349` / `:390`・`crates/areka-emo-present/src/presenter/refresh.rs:83` / `:100` | 同上 | `debug` | 描画内容が新しい寸になった時刻。**窓矩形との食い違い**を測る片側 |
| T8 | `kind=ground` | `crates/areka/src/placement/follow/window_move.rs:342` → `crates/areka/src/placement/transition_diag.rs:539` | 同上 | `debug` | 接地点と作業領域下端の差（`diff`）。下端吸着のキャラ窓のみ |
| T9 | `kind=snapshot` | `crates/areka/src/emo2_boot/frame/work_area_sync.rs:127` → `crates/areka/src/placement/transition_diag.rs:459` | 同上 | `debug` | 作業領域源を作り直したフレームと、作り直した後の全モニタの拡大率・作業領域。**差し替えが起きたフレームにだけ出る**（同じ表のフレームでは 1 行も出ない＝それが正常） |
| T10 | `kind=hold` | `crates/areka/src/emo2_boot/frame/dpi.rs:302`（`site=dpi`・`apply_dpi_phase_gate` → `dpi_sync.rs:232`）・`crates/areka/src/emo2_boot/frame/scale_text.rs:170`（`site=reconcile`）・`crates/areka/src/emo2_boot/frame/drain_resnap.rs:234`（`site=resnap`）・**`crates/areka/src/emo2_boot/frame/work_area_sync.rs:238`（`site=work-area-resnap`・task 6.5 で追加）** → 後 3 者は `crates/areka/src/placement/dpi_sync.rs:270` → いずれも `crates/areka/src/placement/dpi_sync.rs:278` → `crates/areka/src/placement/transition_diag.rs:464` | 同上 | `debug` | 窓の拡大率とモニタ表の**整合待ち**。`decision=` が `hold`／`proceed`／`proceed-after-timeout`、`site=` が判定を下した点（**4 点**＝`dpi`／`reconcile`／`resnap`／`work-area-resnap`。task 6.5 で 4 点目が入った）。**判定が下ったフレームにだけ出る**（待ちの起きないフレームでは 1 行も出ない＝それが正常） |
| T11 | `kind=chain`（`stage=armed`／`realigned`／`deferred`） | `crates/areka/src/placement/chain_realign.rs:117`（armed）/ `:181`（realigned）/ `:223`（deferred）→ いずれも `crates/areka/src/placement/transition_diag.rs:518` | 同上 | `debug` | DPI 遷移後の連鎖再解決。**task 5.6 の着地で点灯した**。`armed`＝武装（拡大率変化を伴う再射影）、`realigned`＝解き直し実施（`scopes`／`moved`）、`deferred`＝見送り（`reason`）。**武装・解決・見送りが起きたフレームにだけ出る**（会話中の表情差替では 1 行も出ない＝それが正常） |
| A1 | `[diag.window_move] route= entity= kind= scope= …` | `crates/areka/src/placement/diag.rs` | `areka::placement::diag` | `debug` | スコープ ↔ キャラ窓 entity の対応（§4.3 の裏取り） |
| A2 | `perf(apply_show): 段階別計時`（末尾に `frame=`） | `crates/areka-emo-present/src/presenter/timing.rs:201-220` | `areka_emo_present` | `debug` | 描画側の段階別所要をフレーム番号で突き合わせる |

### 3.2 「0 件」の読み方（**発行点は 10 種すべて着地済み**）

**task 5.6 の着地（`kind=chain`）をもって、§3.1 の 10 種すべてに発行点が入った。** 「発行点が未着地ゆえ何をしても出ない」種は**もう無い**——ゆえに 0 件を見たら、まず `RUST_LOG` の directive（§2.2）を疑い、次に「そのフレームでは起きなかった」を疑う。以下は種別ごとの「0 件が正常であり得る条件」である。

> `kind=chain` は **task 5.6 の着地で点灯した**（§3.1 の T11 へ移動済み）。**task 4.2（是正前の基準値）のログでこの種が 0 件なのは「観測点が無かったから」であって「解き直しが 0 回だった」からではない**（要件 8.5）——是正前後の比較でこの種の件数を差分に使ってはならない。**task 7.3 の採取では点いているはずなので、まず 1 行でも出ていることを確かめてから件数の議論に入る。** 拡大率が変わる遷移を採ったのに `stage=armed` が 1 行も無ければ、武装トリガ（`frame/dpi.rs` の 3 連言）が走っていない徴候である。
>
> `kind=snapshot` は **task 5.1 の着地で点灯した**（§3.1 の T9 へ移動済み）。ただし出るのは**モニタ表が変わったフレームだけ**なので、定常運転で 0 件なのは正常である——遷移を 1 度も起こさずに採取したログで 0 件だったことを「同期が動いていない」根拠にしない。
>
> `kind=hold` は **task 5.4 の着地で点灯した**（§3.1 の T10 へ移動済み）。出るのは**整合ゲートが判定を下したフレームだけ**——拡大率の相が窓を触ったフレーム（＝遷移）である。ゆえに遷移を含む採取なら `decision=proceed` の行は必ず出る（**遷移を採ったのに `kind=hold` が 0 件ならゲートが走っていない**）。一方 `decision=hold` の行が 0 件なのは正常であり得る——実機の 12 遷移はすべて表更新が先で、待ちの起きない順序だったからである（確定台帳 L2）。**`decision=` の値まで見ずに件数だけで語らないこと。**

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
[transition] frame=118 t_us=1512 kind=write stage=flush seq=0 hwnd=0x1234 origin=DpiReproject scope=0 win_kind=char x=1120 y=315 cx=382 cy=537 flags=0x14 ax=1120 ay=315 aw=382 ah=537 call_us=6100 ok=true in_batch=true
[transition] frame=118 t_us=1520 kind=ground scope=0 ground_y=852 wa_bottom=852 diff=0 route=DpiReproject
[transition] frame=118 t_us=1530 kind=msg msg=WM_WINDOWPOSCHANGED hwnd=0x1234 in_swp=true since_flush_us=30
[transition] frame=118 t_us=1786 kind=flush stage=end count=4 since_tick_us=1450 total_us=286
[transition] frame=118 t_us=1800 kind=chain stage=realigned scopes=2 moved=1 reason=-
```

読み方の要点:

- **`in_batch=` は一括書込がバッチで投入されたか**（task 7.2 の B-2b）。`true` なら当該区間の全書込が `BeginDeferWindowPos`／`EndDeferWindowPos` の 1 バッチで適用されており、`call_us` は**投入だけ**の所要である（窓が実際に動く時間は同じ区間の `kind=flush stage=end` の `total_us` に入る）。`false` はバッチが使えず 1 本ずつへ**縮退**した徴候で、同じ区間に `DeferWindowPos batch unavailable` の `WARN` が必ず並ぶ。**是正前（task 4.2 の基準ログ）にはこのフィールドが 1 行も無い**——欠けていることを「縮退した」と読まない。
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

`monitor`〜`ground` の 8 種が 0 件なら **`wintf::transition=debug` の入れ忘れ**を最初に疑う（§2.2）。`chain` の 0 件は §3.2 のとおり是正前では正常である。`hold` は遷移を含む採取なら 0 件になってはならない（§3.2 の注記＝0 件はゲートが走っていない徴候。`decision=hold` **だけ**が 0 件なのは正常）。`snapshot` は task 5.1 の着地後に点いた観測点だが、**モニタ表が変わったフレームにしか出ない**——遷移を含む採取で 0 件なら同期段を疑う。


### 3.5 整合待ちの札の破れを見る警告語（task 6.5 の検出側・要件 8.1）

**`kind=` のレコードではなく、素の警告行を 1 語だけ数える検査である。** 要件 8.1 が「判定に用いる grep 語」の記録を義務づけているので、本節に置く。

```
整合待ちの札がある窓へ窓書込が到達した
```

| 項目 | 値 |
| --- | --- |
| 意味 | **整合待ちの札（`DpiSyncHold`）が付いている窓へ窓書込が到達した**＝待ち札の適用範囲に漏れがある。整合ゲートの見送りは 4 点（`site=dpi`／`reconcile`／`resnap`／`work-area-resnap`）で覆っているが、5 つ目の窓書込口が増えるとここで鳴る |
| 発行点 | `crates/areka/src/placement/follow/window_move.rs`。`warn!` は **`:682` から始まり**、上の**検査語を含むメッセージ行は `:687`** である（grep の錨としては `:687` が正しい）。同 `:689` に `debug_assert!(false, ..)` が並ぶ（行番号は task 7.5 の是正で `:594`／`:599`／`:601` から移った） |
| ログ target | **`areka::placement::follow::window_move`**（`warn!` に `target=` を与えていないのでモジュールパス既定） |
| 水準 | `warn` |
| **D-ATOM で点くか** | **点く。消灯ではない**（下記） |
| 対象外 | **見送りが覆わない経路は本監視の対象外**（`window_move.rs:678` の条件＝純関数 `deferral_covers_route`・:508）。随伴バルーンの追従（`BalloonFollow`）に加え、**明示操作**——`\![move]`（`MoveCue`）・復元（`Restore`）・バルーンドラッグ解放時の補正（`BalloonLimitRelease`）・route を名乗らないドラッグ——と `SpawnInitial` が対象外である。いずれも設計上そもそも見送らないので、対象にすると正当な到達で偽の警報が出る（task 7.5。是正前は利用者が窓を 1 回ドラッグしただけで debug ビルドが落ちた）。鳴る 7 語は `DpiReproject`／`ReportedSizeReconcile`／`Resnap`／`WorkAreaResnap`／`KeepPositionResize`／`ChainRealign`／`AnchorChange` |

**`release` では panic しない——警告行だけが出る。** `[profile.release]`（リポジトリ直下 `Cargo.toml:95-105`。末尾は `strip = false`）は `debug-assertions` を指定しておらず、既定は off である。ゆえに `debug_assert!` は `release` ビルドで無効化され、本採取のような `release` の実機採取では**この警告行が唯一の徴候**になる。`debug` ビルド（常時テスト）ではその場で落ちるので、**実機採取でだけ静かに通り抜ける形**であり、だからこの grep 語が要る。

**この観測点は消灯していない（§0.3 の但し書きは不要）。** D-ATOM の先頭セグメント `info` が既定水準であり（§2.1 の `info` の行＝「`warn!`／`error!` は常に通る」）、`warn` はそれを上回る。`areka::placement::diag=debug` は `areka::placement::follow::window_move` の接頭辞に**ならない**ので影響しない。

> **較正（この主張を実測で裏づける）**: 同じく `target=` を与えていない兄弟モジュール `areka::placement::measure` の `warn!` が、D-ATOM で採った実機ログに**実際に出ている**（`WARN areka::placement::measure: measure: shell bake で脱落した element…`）。`areka::placement::` 配下の既定 target の `warn` が D-ATOM で通ることの実証である。ゆえに本語の 0 件は「消灯した観測点の 0 件」ではなく、**発生 0 回の根拠として使ってよい**——`[start_preparing]`（§4.4）とはここが違う。

```powershell
$breach = (Select-String -Path $LOG -SimpleMatch '整合待ちの札がある窓へ窓書込が到達した' | Measure-Object).Count
if ($breach -eq 0) { 'ATOM-HOLD-BREACH: PASS' } else { "ATOM-HOLD-BREACH: FAIL ($breach 件)" }
```

**1 件でも出たら本番の欠陥である。** 採取のやり直しでは消えない——`ATOM-SIGNOFF` を FAIL とし、鳴った行の `entity`／`route`／`since_frame` を記録票へ書き写して引受先を決めること（5 つ目の窓書込口を名指しできる唯一の材料である）。**§6.6 の内訳 6 行はこの語を持たない**——本検査は合否の内訳ではなく採取の健全性の前提として §3 側に属するので、判定語は記録票の点灯確認の節へ併記する。

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
8. §3.4・**§3.5**・§4.2・§4.4 を回し、`ATOM-QUOTA` と `ATOM-NO-DRAG` と **`ATOM-HOLD-BREACH`** を記録する。充足していなければ**採り直す**（同じログへ追記しない）。**`ATOM-HOLD-BREACH` の FAIL だけは採り直しでは消えない**——本番の欠陥なので §3.5 に従って記録し、`ATOM-SIGNOFF` を FAIL とする。

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
    量: frames_to_last_write=0 writes=6 path_a_writes=0 sync_stage_writes=0 balloon_same_frame=true(pairs=2) holds=0 chain_realigned=0 ground_diff_max=-48 flush_total_us_max=141179 malformed=0 frames_indeterminate=false
    量(参考): first_write_t_us=199957 last_write_t_us=298444 sum_call_us=140921
    量(窓): scope=0 win_kind=char writes=1 mismatch_frames=0 visualize_to_write_us=208435
    量(見送り窓): なし
    deterministic: 1 件の違反
      - 可視化と書込のフレーム差 1 > 上限 0（scope=0 win_kind=char）
    signoff: 1 件の違反
      - 一括書込の総所要 286000µs > 上限 16667µs
```

- **上限は 2 系統あり、別々に評価して両方が並ぶ。** `deterministic` は回帰テストが固定する決定論の量——起点から最終書込までのフレーム差・**可視化と窓書込のフレーム差**・**随伴バルーンの同一フレーム性**・窓ごとの書込回数・経路 A の件数（札と段の両方）・接地点差・連鎖の回数。`signoff` は**実機専用**の量——可視化から窓書込までの経過（µs）・一括書込の総所要（µs）で、非決定ゆえ回帰テストでは値を固定しない。**どちらか一方の `PASS` を全体の合格と読まない。**
- **フレームを単位に測る量はすべて `deterministic` 側である。** 「可視化と書込のフレーム差」と「随伴バルーンが別フレームで書かれた」は名前が実機の症状に似ているが、`signoff` の下には**出得ない**（`signoff` はフレーム差の上限を 1 つも当てないため）。同じ食い違いを µs で測るのが `signoff` 側の「可視化から書込まで」であり、**両者は別の量である**。§6.6 の 2 行を埋めるとき、違反行がどちらのブロックの下にあったかで系統を判断すること——文言から推測しない。
- **判定量は合否によらず刷られる**（`量:`／`量(参考):`／`量(窓):`／`量(見送り窓):` の各行）。task 4.3 の裁定でこうなった——是正の前後で並べるのは上限の合否ではなく**量そのもの**なので、`PASS` の系統の量が消えると比較の側が毎回生ログから手で起こす羽目になる（task 4.2 で実際に起きた）。欠けている量は番兵 `-` で刷られる（`0` ではない。「測っていない」と「測って 0 だった」は別の事実である）。違反があった量は、それに加えて実測値と上限つきで `- ` の行に並ぶ。
- 実機専用の上限（`Bounds::signoff` の 2 値）は task 4.3 が確定させた。**確定値は両量とも `16667` µs（= 1/60 秒）**であり、根拠は**採取機に依らない**——画面に提示される 1 フレームはリフレッシュレートが 60Hz を下回らない限り高々 1/60 秒なので、これを超えた食い違い（および 1 フレームを超える一括書込）は**どの機械でも必ず 1 枚以上の提示フレームをまたぐ**。要件 4.2／4.5 を µs で言い直した形である。全文は確定台帳 `mechanism-ledger.md` §4（L9）。**リフレッシュレートが 60Hz を下回る表示で採ったときだけ**、この上限は厳しすぎる側へ倒れる——そのときは実測周期を添えて台帳で再裁定する（**目視に合わせて緩めない**・§6.5）。
- 整合待ち（`kind=hold`）を含む遷移だけがフレーム差の許容に待ちフレーム数を足せる。この許容の正本は**本番の整合ゲートの上限**（`crates/areka/src/placement/dpi_sync.rs` の `DPI_SYNC_HOLD_MAX_FRAMES`）であり、判定器の `HOLD_FRAME_ALLOWANCE` はそれを参照する（task 5.4 で二重定義を解消済み）。

### 6.3 判定量を生ログから起こす（記録用）

判定器が刷る判定量（§6.2）が正であり、本表は**その裏取りの手順**である（判定器そのものが壊れたときに気づけるよう、生ログからの起こし方を残す）。数字が食い違ったら判定器の側を疑うこと。遷移ごとに次を集計する（§4.2 のスクリプトで得た遷移の区切りをそのまま使う）。

| 記録する量 | 生ログからの起こし方 |
| --- | --- |
| 起点から最終書込までのフレーム差 | 起点行の `frame=` と、その遷移の最後の `kind=write` 行の `frame=` の差 |
| 窓ごとの書込回数 | その遷移の `kind=write` を `scope=` と `win_kind=` で数える |
| 経路 A の件数 | その遷移の `kind=write` のうち `origin=dpi-suggested` の件数と、`stage=sync` の件数（**両方**。食い違えば札か段のどちらかが壊れている） |
| 随伴の同一フレーム性 | キャラ窓の `kind=write` の `frame=` と、同一スコープのバルーンの `origin=BalloonFollow` の `kind=write` の `frame=` が一致するか |
| 可視化と窓書込の食い違い（フレーム） | 当該窓の `kind=surface stage=visualize` の `frame=` と、当該窓の `kind=write` の `frame=` の差（見送り窓は除く）＝`deterministic` 側の量 |
| 可視化と窓書込の食い違い（時間） | 同一フレームの `kind=surface stage=visualize` の `t_us` と、当該窓の `kind=write` の `t_us` の差（µs）＝`signoff` 側の量。窓ごとに遷移区間内の**最大**を採る |
| 一括書込の総所要 | `kind=flush stage=end` の `total_us` |
| 一括書込がバッチで投入されたか | その遷移の `kind=write stage=flush` の `in_batch=` を数える（**全件 `true`** が期待。1 件でも `false` なら同じ区間の `DeferWindowPos batch unavailable` の `WARN` を読み、縮退の理由語（`BeginDeferWindowPos-failed`／`DeferWindowPos-failed`／`EndDeferWindowPos-failed`）を記録する。`total_us` を是正前後で比べるときは、まず両者が同じ投入形であることをここで確かめる）。**task 7.2 で増えた量**なので、是正前のログにこのフィールドは 1 行も無い |
| 縮退したときの書込の実施ログ | `[guarded_set_window_pos] Calling SetWindowPos` の行を数える（**指令 1 件につき 1 行**が両経路の不変条件。`via="DeferWindowPos"` ならバッチ、`via="SetWindowPos"` なら1 本ずつ。**破棄されたバッチの指令は 1 行も残らない**——残っていれば実施していない書込を数えている）|
| 接地点差 | `kind=ground` の `diff`（負＝浮き） |
| 連鎖の解き直し回数 | その遷移の `kind=chain stage=realigned` の件数（**task 5.6 の着地で点灯**。是正前ログとの差分には使えない＝§3.2） |

### 6.4 フレーム番号が一様に 0 の系列は「1 フレームで完了」と読まない

`frame=0` は tick が始まる前の縮退値である。**系列の `frame` が全部 0 のログは、フレーム差が 0 なのではなく判定不能である**（要件 8.5）。判定器はこの形を検出して `フレーム差が判定不能（一様に 0／読めない）` という違反を立てる。この違反が出たログで「有界フレーム数以内だった」と書いてはならない——**観測基盤の刻印が壊れている**ので、原因を直してから採り直す。

### 6.4.1 「測れなかった」を合格の側へ丸めない（要件 4.6 の裁定・2026-08-21 改訂）

判定器が `判定対象の量が欠けている: …` を立てたときは、**それを雑音として退けてはならない**。とくに**起点フレームに `reason=k-unchanged` が出た窓**がこの形で立つことがある——判定器は見送り窓の除外を `reason=invisible` だけで駆動するので（要件 4.6 の裁定の注記・実装は `transition_judge.rs` の `summarize`）、遷移時点に `k-unchanged` を出した窓は除外されず、書込を持っていれば採点へ入るからである。

**2026-08-21 の再採取で実際に起きた**: 遷移 #5（起点 `frame=37375`）の `scope=0` バルーンが書込 1 件を持ちながら同フレームの再表示は `k-unchanged` で、`mismatch_frames_per_window` と `visualize_to_write_us` の 2 量が測れなかった。当初の裁定はこの形を「合否は変わらない」としていたが、**それは偽であると実測が示した**（要件 4.6 の注記は改訂済み・反証は確定台帳 §10.7 ⑶）。

ゆえに**採取者は 1 件ずつ次を行う**:

1. その窓が**遷移時点で本当に再導出結果を持たなかったのか**を生ログで確かめる（当該フレームの `kind=surface` の行と、直前の可視化の有無を見る）。
2. 当たるなら **「4.6 の窓ゆえ未測定」と根拠つきで記録票へ明記**する（フレーム番号・scope・窓種別・理由語を添える）。
3. 当たらないなら**不合格**とする。

なお `kind=surface` の行は scope も窓種別も運ばない——⑵ で書く scope・窓種別は §3.3 の写像（`target_id` の偶数＝キャラ窓／奇数＝バルーン窓、スコープは `target_id / 2`）を経由して起こすこと。

**⑵ を踏んでも機械判定は `PASS` にならない**（判定器は無改変なので `Unmeasured` の違反は残る）。⑵ は「4.6 の窓だったと判った」ことを記録に残す手続きであって、合格への通路ではない——§6.6 の合否は違反 0 件を要求するので、この遷移を含む採取は機械判定としては不合格側に立つ。**その状態で GO を出すか否かは、記録票の根拠を読んだ開発者の裁定**である。

要件 8.5 の「消灯した観測点を『発生 0 回』の根拠に用いない」と同じ規律である——**測れなかったことは、合格でも不合格でもなく「まだ判っていない」**であり、判っていないまま GO を出さない。

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

**機械判定と目視所見が食い違う場合は合格としない。** 食い違いの向きは 3 つある。加えて、**食い違いに見えて食い違いではない**組み合わせが 1 つあり、取り違えやすいので同じ表の 4 行目に置く。**1〜3 行目は不合格であり、4 行目も合格ではない**（合否は別の事由で FAIL へ倒れる）。

**向きを決めるときは合否を丸めず、系統ごとの PASS／FAIL を見ること。** 判定は 2 系統（決定論側と実機専用側）に割れて刷られ、**2 系統の違反は同時に並び得る**（§6.2 の出力例がその形である）ので、「機械が何と言ったか」を 1 語へ潰すと行を取り違える。

**分岐は 3 つの問いを上から順に当てるだけで決まる。**

1. **目視で跳ねが無いか** — 無くて機械が FAIL なら **2 行目**。無くて機械も PASS なら食い違いは無い（表は当たらない＝合格側）。
2. **決定論系統が 1 件でも違反を立てているか**（症状に対応する量かどうかを問わない） — 立てているなら **4 行目**。
3. **症状に対応する量で違反を立てた系統があるか** — 無ければ **1 行目**、実機専用系統だけがその量で違反を立てているなら **3 行目**。

上から順に当てるので行は必ず 1 つに定まる。**2 と 3 の順序を入れ替えてはならない**——決定論系統の違反は、症状に対応する量でなくても「機械は症状ありと言っている」ことを意味するからである（4 行目の意味）。

| 組み合わせ（4 行目だけは食い違いではない） | 意味 | 採るべき行動 |
| --- | --- | --- |
| 判定 PASS（**決定論系統は PASS**・**症状に対応する量では どの系統も違反なし**）／ 目視で跳ねあり | **どの系統も症状に対応する量を持っていない**＝判定量が症状を捕まえていない（実機専用系統が**症状と別の量**で違反していてもこの行である——分岐の正本は上の 3 つの問いであって、この見出しの語ではない） | 上限か判定量の側を疑い、確定台帳へ「判定に載らない症状」として登記する |
| 判定 FAIL ／ 目視で跳ねなし | 上限が実機の性質に対して厳しすぎるか、観測が壊れている | 上限の根拠を確定台帳で見直す。**目視に合わせて上限を緩めてはならない**（観測装置を被検体に合わせて曲げる行為に当たる） |
| 判定 FAIL（**実機専用系統だけ**）／ 決定論系統は PASS ／ 目視で跳ねあり | **機械も目視も正しく、測っている量が違う。** 症状に対応する量は実機専用系統が現に捉えて上限超を宣言しており、決定論系統は最初から別の量（フレームを単位とする量）を測っている | **判定器にも上限にも手を入れない**（どちらにも欠陥が無い）。**症状に対応する量を持つ系統と、その量の名前を記録票で名指しし、その系統の未達として引受先の spec へ渡す。** あわせて確定台帳へ「判定に載らない症状」として登記する |
| 決定論系統が FAIL（**症状に対応する量かどうかを問わない**）／ 目視で跳ねあり | **食い違いではない**——機械も「症状あり」と言っている。残るのは「機械が名指しした量が、目視の症状と同じものか」だけである | `AGREEMENT` は **`一致`** と書く。**分類はこの 4 行目のままで、上の 3 行へ移さない**（行動だけを 1 行目から借りることはある——下記）。**ただし合格ではない**——`ATOM-SIGNOFF` はどの系統の違反でも FAIL へ倒れる。**違反した量が目視のどの症状に対応するのかを記録票で書き分ける**こと。目視の跳ねに対応する量が誰も違反を立てていないなら、**その症状は 1 行目の形で残っている**ので、**分類は 4 行目のまま 1 行目の行動だけを併せて履行する** |

> **3 行目・4 行目の裁定（task 7.4・2026-08-22。根拠は確定台帳 §11.5、実測は task 7.3 の採取 `atom-73-signoff-1` と task 4.2 の基準採取）**
>
> - **1 行目との境目**（読み違えると 3 行目が 1 行目に化ける）。**決定論系統が PASS であることを確かめたうえで**、1 行目が「判定量が症状を捕まえていない」と断定できるのは、**症状に対応する量で違反を立てた系統が 1 つも無い**ときだけである。**実機専用系統がその量で違反を立てているなら 3 行目**であり、そこで判定器を疑うのは誤診になる。
> - **4 行目を足した理由（task 7.4 のレビューが掘り当てた穴）。** 3 行目までは「決定論系統は PASS」を条件に持つので、**決定論系統が症状と別の項目で FAIL しながら目視で跳ねが出る**組み合わせがどの行にも入らなかった。これは机上の形ではない——⑴ §6.2 の出力例は決定論違反と実機専用違反を**同時に**並べており、⑵ **task 4.2 の基準採取が現にその形だった**（接地点差 −48px の決定論違反〔`ground_diff_abs_max` は決定論側だけが当てる上限である〕と、目視の跳ね・浮きが同居した）。その 4.2 が記録した `AGREEMENT` は **`一致`** であり、7.3 が `食い違い` へ転じた理由は「**是正で決定論系統が PASS になった**」ことである（確定台帳 §3.9・§11.3）。ゆえに 4 行目は**食い違いの 4 つ目の向きではなく、食い違いでないことを明示する行**として置く——3 行目の条件を「症状に対応する量では違反していない」へ緩めると、**4.2 の確定記録（一致）を後から食い違いへ書き換えてしまう**。
> - **3 行目の `AGREEMENT` は `食い違い` と書く**（§6.6 の語 `一致|食い違い` を増やさない。4 行目は `一致` である）。3 行目では、機械が症状を名指しできたかどうかは目視と噛み合っていない（決定論系統は 0 を出す）ので食い違いであり、しかも合否の結論は既に両者とも不合格の側で揃う。**語を増やすと §6.6 の 7 行・記録票・要件 8 の注記を同時に直すことになり、確定済みの採取結果を後から書き換えてしまう。**
> - **判定器は 1 行も変えない。** 3 行目が足すのは説明と行動の語彙だけであり、判定の挙動も上限 16,667µs（確定台帳 L9）も動かさない。
> - **実測（この向きの初例）**: task 7.3 の採取では決定論系統が 8 遷移とも PASS（`mismatch_frames` は 32 窓すべて 0＝フレームを単位とする量として正しく 0）である一方、実機専用系統の `visualize_to_write_us` は **32 件すべてが上限 16,667µs 超**（210,329〜306,301µs）で FAIL を宣言した。合否は `ATOM-SIGNOFF: FAIL`・`AGREEMENT: 食い違い` であり、未達は引受先 `areka-P0-present-write-coherence` へ渡してある（確定台帳 §11.3〜§11.5）。

### 6.6 合否の書き方

```
ATOM-SIGNOFF: PASS|FAIL
  ATOM-QUOTA:        PASS|FAIL (transitions=N low2high=N high2low=N scopes=…)
  ATOM-NO-DRAG:      PASS|FAIL
  DETERMINISTIC:     PASS|FAIL (違反 N 件)
  SIGNOFF-BOUNDS:    PASS|FAIL (違反 N 件・上限 16667µs＝確定値・台帳 L9)
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
| 点灯確認 | §3.4 の 10 行の出力そのまま ＋ **§3.5 の `ATOM-HOLD-BREACH:` の 1 行**（要件 8.1 の grep 語。0 件でも必ず書く——この観測点は D-ATOM で点いているので 0 件が発生 0 回の根拠になる＝§3.5 の較正） |
| 充足 | `ATOM-QUOTA:` の 1 行＋遷移一覧（フレーム・old・new・スコープ） |
| 無効化チェック | `ATOM-NO-DRAG:` の 1 行＋「窓へ触れていない」旨の宣言 |
| 判定 | ランナーの `--nocapture` 出力全文 |
| 判定量 | ランナーの出力に含まれる（§6.2 の `量:` 各行）。§6.3 で生ログから起こした値と食い違ったら**その旨も**書く |
| 目視所見 | §6.5 の様式（遷移ごと） |
| 合否 | §6.6 の 7 行 |

---

## 8. 本書のメンテ規約

- 観測レコードの語（`kind=`・`stage=`・フィールド名）と、ランナーの入口の語（環境変数名・観測 target・行頭タグ・Report の 2 系統名と合格語・ランナーのテスト名）、および **§6.2 の Report 出力例に並ぶ違反行がどちらの上限系統に属するか**は、**`crates/areka/src/placement/transition_signoff_procedure_tests.rs` が本書を読んで一致を検査している**（§0.2 の検査 ⑴〜⑷）。発行側の語を変えるなら、同じコミットで本書も直すこと。とりわけ §6.2 の出力例へ違反行を足す・動かすときは、その系統で実際に出得る違反かをテストが確かめるので、通してから報告すること。
- §3.2 の「発行点が未着地」の表は **task 5.1／5.4／5.6 の着地で空になり、§3.2 は「0 件の読み方」へ書き替えた**（10 種すべて §3.1 にある）。以後に観測点を足したら**同じ要領で §3.1 へ行を足す**こと。移し忘れると、点いている観測点を「出ないもの」として扱い続けることになる。task 5.1（`kind=snapshot`＝T9）・task 5.4（`kind=hold`＝T10）・task 5.6（`kind=chain`）はいずれも移動済みで、**残りは 0 種である**（§3.2:183 が「0 件の読み方」として書いているとおり）。
- §6.2 の実機専用の上限は task 4.3 が確定値（`16667` µs）へ差し替え済みである。以後この値を動かすなら、確定台帳 `mechanism-ledger.md` §4（L9）の根拠を先に書き換え、判定器の定数（`crates/areka/src/placement/transition_judge_verdict.rs` の `VISUALIZE_TO_WRITE_US_MAX`／`FLUSH_TOTAL_US_MAX`）と本書の 3 箇所（§6.2 の出力例・§6.2 の上限の項・§6.6 の `SIGNOFF-BOUNDS` 行）を同じコミットで揃えること。**実測に合わせて緩めるのは禁じられている**（§6.5）。
- file:line の参照は本書作成時点（群 1〜3 着地）のものである。ずれたら本書を直す——ずらしたまま放置すると、本書は読めるが辿れない文書になる。
