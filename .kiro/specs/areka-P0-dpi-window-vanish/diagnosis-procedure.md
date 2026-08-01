# 診断手順書（areka-P0-dpi-window-vanish）

> 対象要件: **1.4 / 1.5 / 1.6 / 1.8 / 1.9 / 5.5**
> 対象タスク: 4.1（本書の作成）・**4.5（本書に従う実機 2 セッション採取＝開発者ゲート）**・7.4（是正後の再サインオフ）
> 成果の登記先: `diagnosis-report.md`（確定台帳。本書は**手順**のみを定め、結論は書かない）

本書は「第三者が本書だけを読んで、同じ設定でゴーストを起動し、**DPI 変化通知の受理回数を機械的に数えて**セッションの充足可否を判定できる」ことを完了条件とする。

---

## 0. 大前提（読む前に必ず）

### 0.1 採取は「是正未投入」のビルドで行う（順序の絶対制約）

タスク 4.5 の 2 セッションは **Phase A 完了・S1/S2/S3 是正未投入のコミット**でビルドしたバイナリで採取する。是正を投入すると消失の実機再現自体が起きなくなり、Q1〜Q4 の確定材料が永久に失われる（tasks.md 冒頭の絶対制約・design.md「実機サインオフ > 順序制約」）。

採取に用いたコミット SHA を、ログ保存先の `meta.txt` と `diagnosis-report.md` の双方へ必ず記録する。

### 0.2 本書の判定語は**実コードから転記した実測値**である

本書に載る grep 判定語・フィールド名・レコード書式は、すべて `crates/` の出力点から直接転記し、`EnvFilter` 実濾過で点灯を確認したものである（§8 に検証記録）。**推測で書いた語は 1 つも無い。** 逆に、コードが出していない語は「未実装」として §3.3 に明示する——それらを「発生 0 回」の根拠に使うことは本書が禁じる（要件 1.5）。

### 0.3 要件 1.5 の中心規則（本書が制度化する唯一の禁止）

> **本手順で有効化されない水準に置かれている観測点は、「発生 0 回」の根拠に用いてはならない。**

2026-07-18 の実機診断は、`trace!` 水準の観測点を `RUST_LOG` で開けないまま「発生 0 回」と読み、「OS 提案矩形による位置書込は反証済み」という**誤結論（偽陰性）**を生んだ。本仕様はその誤りを繰り返さないために、§3 の対応表で **点灯／消灯／未実装**の 3 状態を明示し、消灯・未実装の観測点については「観測できなかった」としか書けないことを定める。

---

## 1. 準備

### 1.1 パス変数（以降すべて絶対パスで扱う）

**記憶〈emo2 実走は絶対パス必須〉**: emo2 fixture を**相対パス**で渡すと `pasta.dll` の LOAD が `0x8007007E` で失敗する。位置引数・env はすべて絶対パスで与えること。

```powershell
# リポジトリ（ワークツリー）ルート。自分の環境の実値へ置き換える。
$REPO    = (git rev-parse --show-toplevel).Replace('/', '\')
# 本ドキュメント作成時点の実値の例:
#   C:\home\maz\git\areka\.claude\worktrees\areka-p0-choice-interact-0e0355

$AREKA   = "$REPO\target\debug\areka.exe"
$GHOST   = "$REPO\crates\pilot\examples\shiori-host-32\fixtures\emo2"
$BALLOON = "$REPO\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku"
```

`$GHOST` 配下に `ghost\master\descript.txt` が実在することを確認する（`crates/areka/tests/emo2_real_run.rs` の前提と同一）。

### 1.2 ビルド（x64 本体 ＋ i686 helper）

```powershell
cargo build -p areka --bin areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item "$REPO\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe" "$REPO\target\debug\" -Force
```

`default_helper_exe_path()`（`crates/areka/src/main.rs:155-161`）は **実行ファイル隣**の `shiori-host32-helper.exe` を解決するため、`target\debug\` へ置くこと（記憶〈workspace test は i686 host-32 成果物が要る〉）。

helper が無くてもキャラ窓は出るが、**バルーン窓は発話が来るまで出ない**——要件 2.5（バルーン消失がキャラ随伴か独立か）の判別材料が採れないため、helper は必須扱いとする。

### 1.3 実機構成の要件

- **物理 2 台以上のモニタ**で、**拡大率が異なる**こと（少なくとも 125% と 200% 相当＝`dpi=120` と `dpi=192`）。
- 仮想スクリーン座標に**負座標**が現れる配置（副モニタを主モニタの左／上に置く）が望ましい——実機の消失事象は screen 座標 3200 超・負座標混在の環境で観測された。
- 診断中は**画面ロック・スリープ・リモートデスクトップ接続を行わない**（モニタ列挙が変わり、セッション固定の `MonitorSnapshot` と実構成が食い違って別事象を混入させる）。

### 1.4 profile ディレクトリを**セッションごとに新品**にする

位置永続（`ghost.dat` 相当＝sylphya profile）が前回の画面外位置を復元すると、初期配置が非決定になり採取が汚れる。セッションごとに空ディレクトリを与える。

```powershell
$PROFILE_DIR = "$env:LOCALAPPDATA\areka-diag\profile-$(Get-Date -Format yyyyMMdd-HHmmss)"
New-Item -ItemType Directory -Force -Path $PROFILE_DIR | Out-Null
```

これは `AREKA_PROFILE_DIR`（`main.rs:173`）で切り替わる。**位置永続との相互作用そのものは本 spec の対象外**（`areka-P0-position-persist` の所有）ゆえ、意図的に無効化して観測する。

---

## 2. 起動設定（第三者が同一手順を再実行できる粒度）

### 2.1 ログ directive（**D-BASE**・要件 1.4/1.5 の中核）

```
info,areka::placement::diag=debug,wintf::ecs::window=debug,wintf::ecs::layout::systems::monitor_systems=debug,wintf::ecs::drag=trace
```

各セグメントの存在理由と、**省くと何が消灯するか**:

| セグメント | 開く観測点 | 省くと |
| --- | --- | --- |
| `info` | 既定水準（`main.rs:262` のフォールバックと同値）。`warn!`／`error!` は常に通る | — |
| `areka::placement::diag=debug` | `[diag.monitor_snapshot]`・`[diag.monitor]`・`[diag.window_move]` | 要件 1.1/1.2 の全証跡が消える |
| **`wintf::ecs::window=debug`** | `WM_DPICHANGED` 受理・提案位置の実施可否・**`guarded_set_window_pos`** | §2.2 参照（**最重要**） |
| `wintf::ecs::layout::systems::monitor_systems=debug` | wintf 側モニタ列挙行（`handle`・`work_area`） | D12 の「両側 grep 突合」が成立しない |
| `wintf::ecs::drag=trace` | ドラッグ開始／終了（`debug`）＋**毎イベントのカーソル座標（`trace`）** | 要件 2.3（マウス移動量と窓移動量の対応の数値評価）が不能 |

### 2.2 `wintf::ecs::window=debug` を落としてはならない（design.md:476 の記載は誤り）

design.md「成果物 > diagnosis-procedure.md ①」が例示する
`RUST_LOG=info,wintf::ecs::window_proc=debug,wintf::ecs::drag=debug,areka::placement::diag=debug`
は、**窓位置書込の共通経路 `guarded_set_window_pos` を点灯させない**。

- `guarded_set_window_pos` の target は `wintf::ecs::window::command`（`crates/wintf/src/ecs/window/command.rs:97`）。
- `EnvFilter` の target 照合は**素の文字列前方一致**であり、`wintf::ecs::window_proc` は `wintf::ecs::window::command` の接頭辞に**ならない**。
- 逆に `wintf::ecs::window` は `wintf::ecs::window::command` と `wintf::ecs::window_proc::window_pos` の**両方**の接頭辞になる（＝1 セグメントで両方開く）。

これはタスク 1.3 でミューテーション実測済みであり、`crates/wintf/src/ecs/window_proc/window_pos.rs` の in-source 檻（`window_pos_write_path_is_visible_at_debug` / `..._is_silent_under_default_info_filter`）が固定している。**本書の D-BASE が正であり、design.md:476 の例は使わないこと。** 入れ忘れは、要件 1.5 が排除しようとしている 2026-07-18 の偽陰性をそのまま再生産する。

### 2.3 終了時ログ（要件 6.2/6.3）を見るときの拡張 directive（**D-TEARDOWN**）

```
<D-BASE>,areka::placement::follow=debug,areka::emo2_boot::frame=debug,areka::placement::spawn=debug
```

`[despawn-skip]` 判定語と、レジストリ掃除の成立行は **D-BASE では消灯している**（§3.2）。ただし要件 6.2 の**否定側**（「破棄済み窓に対する `warn!` 以上が出ない」）は `warn!`／`error!` が `info` 水準で通るため **D-BASE のままで判定できる**。肯定側（`[despawn-skip]` が実際に出ている）を確認したいときだけ D-TEARDOWN を使う。

### 2.4 有界自動終了（要件 1.6）

環境変数 **`AREKA_APP_SMOKE_EXIT_MS`**（`crates/areka/src/main.rs:830` の `SMOKE_EXIT_ENV`）。値はミリ秒の非負整数。指定 ms 後に起動窓（ダミー窓／ゴースト窓）を `despawn` して終了する（`main.rs:757-790`）。空・非数値・負値は**ゲート OFF**（自動終了しない）。

手動セッションでは **`900000`（15 分）** を既定とする。理由:

- 受理回数の下限（§5）を人手の操作で踏破するには数分を要する。
- 番犬を持たない直接起動ゆえ、`emo2_real_run.rs` の `SMOKE_EXIT_MS`（3000）や番犬締切（120s）とは**別物**である。混同しない。
- 自動終了は `GhostWindowMarker` の despawn を通るため、**終了時ログ（要件 6.2/6.3）を決定論的に発生させる**という副次的な利点がある。

### 2.5 ログ保存先

**リポジトリ配下へ生ログを置かない**（巨大かつ実機固有・コミット対象外）。

```powershell
$RUN     = Get-Date -Format yyyyMMdd-HHmmss
$LOGDIR  = "$env:LOCALAPPDATA\areka-diag\$RUN"
New-Item -ItemType Directory -Force -Path $LOGDIR | Out-Null
```

- セッション① → `$LOGDIR\session1-drag.log`
- セッション② → `$LOGDIR\session2-osdpi.log`
- 併せて `$LOGDIR\meta.txt` に**コミット SHA・ビルドプロファイル・モニタ構成（台数と拡大率）・OS ビルド・採取日時**を書く。

`diagnosis-report.md` には**該当行の引用のみ**を転記し、生ログのパスと SHA を併記する。

### 2.6 起動コマンド（絶対パス・そのまま貼れる形）

```powershell
$env:RUST_LOG                = "info,areka::placement::diag=debug,wintf::ecs::window=debug,wintf::ecs::layout::systems::monitor_systems=debug,wintf::ecs::drag=trace"
$env:AREKA_APP_SMOKE_EXIT_MS = "900000"
$env:AREKA_PROFILE_DIR       = $PROFILE_DIR

& $AREKA $GHOST $BALLOON 2>&1 | Tee-Object -FilePath "$LOGDIR\session1-drag.log"
```

- 位置引数は `argv[1]=ghost_root`・`argv[2]=balloon_root`（`resolve_config_inputs`・`main.rs:130-143`）。
- `2>&1 | Tee-Object` で標準出力・標準エラーを 1 本にまとめて保存しつつ画面でも見る。
- セッション②では出力先ファイル名だけを `session2-osdpi.log` に変える（**env は 1 文字も変えない**——2 セッションの差は操作だけであるべき）。

---

## 3. 観測点 × ログ target × 水準 対応表（要件 1.5）

### 3.1 D-BASE で**点灯する**観測点

| # | 判定語（行に必ず現れる文字列） | ログ target | 水準 | 出所 |
| --- | --- | --- | --- | --- |
| O1 | `[diag.monitor_snapshot] context=monitor_snapshot count=` | `areka::placement::diag` | `debug` | `placement/diag.rs:273,322` ← `main.rs:633`（**要件 1.1 の正典出力点**・D12） |
| O2 | `[diag.monitor_snapshot] context=prepare_ghost_windows count=` | 同上 | `debug` | 同上 ← `placement/mod.rs:363`（配置準備の列挙点） |
| O3 | `[diag.monitor] index= handle= bounds= work_area= dpi= primary=` | 同上 | `debug` | `placement/diag.rs:278-289` |
| O4 | `[diag.window_move] route= entity= kind= scope= x= y= w= h= dpi=` | 同上 | `debug` | `placement/diag.rs:294-313,334` ← `placement/follow.rs:1168-1170`（単一ライター成功時） |
| O5 | `[initialize_layout_root] Creating Monitor entity` （`handle=`・`work_area=`） | `wintf::ecs::layout::systems::monitor_systems` | `debug` | `layout/systems/monitor_systems.rs:39-55` |
| O6 | `WM_DPICHANGED`（`suggested_left/top/right/bottom` を持つ受信行） | `wintf::ecs::window_proc::window_pos` | `debug` | `window_proc/window_pos.rs:302-313` |
| O7 | **`[WM_DPICHANGED] DPI component directly updated (Changed<DPI>)`**（`entity=`・`old_dpi_x=`・`new_dpi_x=`） | 同上 | `debug` | `window_proc/window_pos.rs:325-332` ＝ **§5 の受理計数の対象行** |
| O8 | `[WM_DPICHANGED] suggested position write decision`（`entity=`・`hwnd=`・**`policy=`**・`applied=`・`suggested_left=`・`suggested_top=`） | 同上 | `debug`（旧 `trace!` から是正・要件 1.3） | `window_proc/window_pos.rs:392-403`（**タスク 5.1 着地後**。`policy=` 新設・`applied` は `decision.is_some()` の分岐結果） |
| O9 | `[guarded_set_window_pos] Calling SetWindowPos`（`hwnd=`・`x=`・`y=`・`cx=`・`cy=`・`flags=`） | `wintf::ecs::window::command` | `debug`（旧 `trace!` から是正） | `window/command.rs:97-102` |
| O10 | `[WM_WINDOWPOSCHANGED]`（`is_echo=`・`has_dpi_ctx=`・`client_xy=`） | `wintf::ecs::window_proc::window_pos` | `debug` | `window_proc/window_pos.rs:80-90` |
| O11 | `[start_preparing] DragState -> Preparing (with capture)`（`entity=`・`hwnd=`） | `wintf::ecs::drag::state` | `debug` | `drag/state/mod.rs:246-252` |
| O12 | `[update_dragging] JustStarted -> Dragging with WindowDragContext`（`entity=`・`hwnd=`・`move_window=`） | 同上 | `debug` | `drag/state/mod.rs:333-340` |
| O13 | `[drag] Dragging started` / `[drag] Dragging ended` / `[drag] Dragging cancelled` | 同上 | `debug` | `drag/state/mod.rs:274,427,480` |
| O14 | `[DragEvent] Dispatching`（`start_x/y`・`current_x/y`・`delta_x/y`） | `wintf::ecs::drag::dispatch` | **`trace`** | `drag/dispatch.rs:357-365` ＝ **要件 2.3 の数値源** |
| O15 | `[DragStartEvent] Dispatching` / `[DragEnd] Direct Arrangement.offset sync` | 同上 | `info` | `drag/dispatch.rs:190,301` |
| O16 | `dpi reproject: WindowPos.size 未確定（窓生成前）`（`entity=`） | `areka::emo2_boot::frame` | **`warn`** | `frame.rs:923-926`（**タスク 5.2 着地後**。DPI 相の位置再射影が現寸を読めず打ち切った縮退＝1 行でも出たら当該窓の接地点は保証されない） |

### 3.2 D-BASE では**消灯する**観測点（D-TEARDOWN で点灯）

| # | 判定語 | ログ target | 水準 | 備考 |
| --- | --- | --- | --- | --- |
| X1 | `[despawn-skip]`（追従層） | `areka::placement::follow` | `debug` | `follow.rs:839`（`resize_window_to`）・`follow.rs:1524`（`resize_window_keep_position`）。定数は `diag.rs:92` |
| X2 | `[despawn-skip] dpi reconcile:` / `[despawn-skip] resnap:` / **`[despawn-skip] dpi reproject:`**（フレーム層） | `areka::emo2_boot::frame` | `debug` | `frame.rs:1140,1344,920`（3 つ目は**タスク 5.2 着地後**＝DPI 相の位置再射影の消費点。O16 と対で読む——同じ「寸が読めない」でも破棄済みは正常終了系ゆえ `debug`、実在窓は真の異常ゆえ `warn`） |
| X3 | `placement: ゴースト窓レジストリから scope エントリを除去`（`scope=`・`char_window=`・`balloon_window=`） | `areka::placement::spawn` | `debug` | `spawn.rs:124-130`。**scope→entity 対応表の別解**（§5.2 の代替源） |

> **規則の適用**: D-BASE のみで採ったログについて「`[despawn-skip]` が 0 行だった」と書いてはならない（**消灯しているだけ**）。要件 6.2 の判定は「`warn!` 以上が 0 行」という**否定側**で行う（`warn` は `info` で通る＝D-BASE で観測可能）。

### 3.3 **どの directive でも 0 行になる**観測点（未実装・Phase C 待ち）

| 語 | なぜ出ないか | いつ出るようになるか |
| --- | --- | --- |
| `ClampX` / `VisibilityVerdict` 関連の `warn!` | `guard_visibility`（`follow.rs:1417`）は**意図的に無ログ**の純関数で、まだどこからも呼ばれていない（`#[allow(dead_code)]`） | タスク **6.1**（キャラ窓）／**6.2**（バルーン窓）で配線＋`warn!` |
| `NearestFallback` の `warn!` | `work_area_for_window_with_origin`（`follow.rs:1315`）は判別を返すが、水準昇格は消費側の責務で未配線 | タスク **6.1** |
| ~~`policy=` フィールド~~ | ~~未配線~~ | **タスク 5.1 で配線済み＝本表から退役**（現在は O8 に必ず載る。値語彙は §3.4） |
| ~~`applied=false`~~ | ~~`applied` は定数~~ | **タスク 5.1 で分岐化済み＝本表から退役**（ゴースト窓では `false`・非ゴースト窓では `true`。§3.4） |
| `route=SpawnInitial` / `route=Restore` | 語彙のみ予約・未配線（`diag.rs:120-125`・D13 帰結⑷） | **未定**——5.1 は spawn へ `ExternalAuthority` を付与しただけで **route の配線は行わなかった**（spawn は単一ライター `enqueue_window_set_pos` を通らず entity 組立時に `WindowPos` を直接持たせるため。`diag.rs` の `#[allow(dead_code)]` も存置）。位置復元（`Restore`）は position-persist spec の所有。**本 spec では出ないままである**——0 行を「配線漏れ」と読まないこと |
| ドラッグ由来の `[diag.window_move]` | ドラッグ経路は `enqueue_window_set_pos(..., route=None)` で呼ぶ（`follow.rs:290,383`）＝**設計上レコードを出さない** | 予定なし（§4 の対応表で wintf 側から観測する） |

> **`applied` の読み方は 2026-08-01（タスク 5.1 着地）で反転した。** 採取ログが**どちらのビルドのものか**を先に確定してから読むこと。
>
> - **5.1 着地前（Phase A/B・セッション①および②-b）**: `applied` は定数（`let applied = true;`）であり、`applied=true` は「OS 提案位置を書くと**決定した**」証拠ではなく「**分岐がまだ存在しない**」ことの表示にすぎない。これを「政策判断が働いた」と読むのは要件 1.5 が禁じる誤読と同型である（§2.3.3 の実機 84/84 はこの意味の `true` である）。
> - **5.1 着地後（タスク 7.4 の再サインオフ）**: `applied` は `dpi_suggested_position_decision` の**実際の分岐結果**である。`applied=true` は「政策が `ApplyPosition`／未付与だったので書いた」という**正常な政策判断**であり、非ゴースト窓では**そう出るのが正しい**。**これを「分岐が無い」と読むと再サインオフの判定が反転する**。分岐の有無は `applied` ではなく **同一行の `policy=`** で判別すること（§3.4）。

### 3.4 O8 の `policy=` 値語彙（**タスク 5.1 着地後**・要件 1.5）

`policy=` は判断の**根拠**を名指しするフィールドで、値は次の 4 種に限られる（`window_pos.rs:384-391` の網羅 match ＝腕が増えればコンパイラが指摘する）。**引用符は付かない**（`policy=ExternalAuthority` の形。他フィールドと grep の当たり方を揃えるため `format_args!` で出している）。

| 値 | 意味 | 同行の `applied` | 実機で出るはずの窓 |
| --- | --- | --- | --- |
| `ExternalAuthority` | 位置権威が areka 配置系にある窓＝**OS 提案位置を書かない** | `false` | **ゴースト窓（キャラ・バルーンの全 scope）** |
| `unset` | component 未付与＝後方互換の既定（従来どおり書く） | `true` | 非ゴースト窓（examples・将来の通常窓） |
| `ApplyPosition` | 明示的に既定を宣言した窓（`unset` と同義の判断） | `true` | 現状 areka 本体では付与箇所なし |
| `unreachable` | **政策を読めなかった**（World 借用の再入・entity 破棄）。従来挙動へフォールバックする | `true` | 通常は 0 行。出たら「宣言が無かった（`unset`）」と**混同しないこと**——読めなかっただけであり、`unset` と同じ語で報告すると事後の突合が偽の結論を作る（要件 1.5） |

> **`unreachable` を `unset` と数え合わせてはならない。** ゴースト窓の受理が `unreachable` を報告していたら、それは「外部権威が効いた」でも「宣言が無い」でもなく**観測そのものの失敗**であり、その受理は計数から除外して原因を先に潰すこと。

---

## 4. 経路（誰が位置を書いたか）× target 対応表（要件 2.4・D13 申し送り）

`areka::placement::diag` の `route=` 語彙は **9 種**（`diag.rs:115-151`・`PlacementRoute::ALL` が 9 件であることを in-source 檻が固定）。**ただしドラッグと OS 直書きは diag target を通らない。**片方の target だけを grep すると、実機で最も疑わしい 2 経路をまるごと取り逃す。

| 実トリガ | `areka::placement::diag` 側 | wintf 側の痕跡 | 割当点 |
| --- | --- | --- | --- |
| spawn 初期配置 | （**未配線**・`SpawnInitial` 予約） | O9 | — |
| 位置永続の復元 | （**未配線**・`Restore` 予約） | O9 | — |
| アンカー変化 | `route=AnchorChange` | O9 | `follow.rs:1032` |
| 毎フレーム再スナップ | `route=Resnap` | O9 | `frame.rs:1270` |
| **DPI 変化の位置再射影（`dpi_phase` 限定）** | `route=DpiReproject` | O6/O7/O8 ＋ O9 | `frame.rs:870`（寸の再導出結果あり）／`frame.rs:937`（**タスク 5.2 着地後**＝再導出結果なしで**現寸のまま位置だけ**再射影する経路。`reproject_char_window_at_current_size`） |
| **報告回収（drain 相・初回表示の k₀ 補正を含む）** | `route=ReportedSizeReconcile` | O9 | `frame.rs:1152` |
| バルーン位置据置きリサイズ | `route=KeepPositionResize` | O9 | `follow.rs:1578` |
| バルーン随伴 | `route=BalloonFollow` | O9 | `follow.rs:523,771` |
| `\![move]` スクリプト明示移動 | `route=MoveCue` | O9 | `follow.rs:753` |
| **ユーザーのドラッグ** | **出ない（`route=None`）** | **O11〜O15 ＋ O9** | `follow.rs:290,383` |
| **OS 提案位置の直書き（`WM_DPICHANGED` ③）** | **出ない（areka を通らない）** | **O8 ＋ O9** | `window_pos.rs:369-379` |

### 4.1 セッション②で「DPI 由来」と数えてよいのは `DpiReproject` **だけ**（D13）

`ReportedSizeReconcile` は drain 相の報告回収であり、**`Changed<DPI>` に依存しない**（初回表示の k₀ 補正がここに landing する＝`frame.rs:1144-1146`・`diag.rs:140-145`）。DPI 変化ゼロの起動でも出る。これを DPI 由来と数えると、セッション②の突合に偽陽性が丸ごと混入する。

同様に `MoveCue`（`\![move]`）・`BalloonFollow`（キャラ確定後の随伴）は DPI 由来ではない。

> **タスク 5.2 着地後の読み替え（`DpiReproject` の件数は増え得る）**: 5.2 の是正で、`Changed<DPI>` のキャラ窓は**寸の再導出結果が得られなかった走行でも**位置の射影を一度通るようになった（`frame.rs:937`）。ゆえに `route=DpiReproject` のレコードは「**寸を伴う書込**」（従来経路）と「**寸は前寸のまま位置だけ直した書込**」の 2 種を含む。**書式は変わらない**（`w=`／`h=` には常に実寸が載る＝`w=-` にはならない）ため §5 の計数規則・トークン境界は無改変だが、①（5.1 以前のビルド）との件数比較を「悪化」と読まないこと。5.2 以後に増えた分は**接地点規約の復元**であって新たな暴走ではない。判別が要る場合は、同一 entity の直前レコードと `w=`／`h=` が一致していれば後者（位置のみの是正）である。

### 4.2 target をまたぐ結合キー

| つなぎたいもの | キー | 出所 |
| --- | --- | --- |
| scope ↔ 窓 entity | `entity=` | O4（`kind=char`／`kind=balloon` と `scope=` を同一行に持つ） |
| areka レコード ↔ wintf の DPI 受理 | `entity=` | O4 ↔ O7/O8（同じ `Debug` 表現＝`{index}v{generation}`。`diag.rs` の檻 `window_move_record_entity_uses_debug_rendering_of_wintf_logs` が固定） |
| entity ↔ `HWND` | `entity=` と `hwnd=` を**同一行**に持つ行 | **O8**（`entity=… hwnd=HWND(0x…)`）・**O11/O12**（`entity=… hwnd="0x…"`） |
| ドラッグのカーソル ↔ 実際の窓書込 | `hwnd=` | O11/O12（`hwnd="0x{:X}"` 大文字 16 進・引用符付き）↔ O9（同じ `hwnd="0x{:X}"` 書式） |

> **`hwnd` の書式は target で異なる**: O9・O11・O12 は `format!("0x{:X}", ...)`（文字列ゆえ引用符付き・**大文字**16 進）、O6・O8 は windows-rs の `HWND` の `Debug`（`HWND(0x0)` 形式・**小文字**16 進）。単純一致で結合すると外れる。**DPI 側の結合は `entity=` で行うこと**（書式差の影響を受けない）。

> **座標系の注意**: O9 の `x=`/`y=` は `SetWindowPos` へ渡した**ウィンドウ座標**、O4 の `x=`/`y=` は areka が確定して `WindowPos` へ写した値である。ゴースト窓は枠なしゆえ実機では一致するはずだが、**一致を前提にせず**、系統的なずれが観測されたら `diagnosis-report.md` に事実として記録すること。

---

## 5. 充足条件と 2 段 grep 規則（要件 1.9）

### 5.1 充足条件は経過時間ではなく**DPI 遷移回数**

各セッションについて、**キャラ窓の各 scope × 各方向（低 DPI→高 DPI・高 DPI→低 DPI）× 3 回以上**の DPI 遷移をログから数えられること。合計の下限は **キャラ窓の実在数 × 2 方向 × 3**（キャラ 2 体なら 12・1 体なら 6）。

> **改訂（2026-07-31・タスク 4.6 着地に伴う・Req 1.9 改訂）**: 改訂前は計数対象を **O7 のみ**（`WM_DPICHANGED` の受理）としていた。しかしセッション② 1 回目の実機採取で、**OS 表示設定からの拡大率変更では `WM_DPICHANGED` が 1 度も届かない**ことが判明し（S4・`diagnosis-report.md` §2.7）、この定義では下限が**構造的に到達不能**だった。タスク 4.6 は `WM_DPICHANGED` 非依存の再導出駆動路を通したので、**計数は「キャラ窓の DPI が実際に遷移したこと」を経路によらず数える**形へ改める。セッション①の判定（`PASS` 44/12）はこの改訂でも変わらない（O7 のみで下限を超えているため）。

- 計数対象行は次の **2 種の和**である。同一の遷移が両方に現れることはない（4.6 の等値ガードにより、`WM_DPICHANGED` で更新済みなら再導出は書込ゼロで抜ける）。

| # | 判定語 | 経路 | ログ target |
| --- | --- | --- | --- |
| **O7** | `[WM_DPICHANGED] DPI component directly updated`（`entity=`・`old_dpi_x=`・`new_dpi_x=`） | OS からの DPI 変化通知（ドラッグでのモニタ跨ぎ等） | `wintf::ecs::window_proc::window_pos` |
| **O16** | `[detect_display_change_system] Redriving window DPI from updated Monitor (no WM_DPICHANGED required)`（`entity=`・`handle=`・`center=`・`old_dpi_x=`・`old_dpi_y=`・`new_dpi_x=`・`new_dpi_y=`） | 表示構成変更を契機とする再導出（Req 7.3・タスク 4.6 新設） | `wintf::ecs::layout::systems::monitor_systems`（`:476`） |

  両方とも **D-BASE の `RUST_LOG` で点灯する**（O16 の target は既収録＝追加語不要）。**両方のフィールド名が `entity=`・`old_dpi_x=`・`new_dpi_x=` で揃えてある**ため、第 2 段の正規表現は 1 本で両方を拾える。

- **セッション②では併せて次の 2 語を必ず確認すること**（充足条件ではないが、`0/6` だったときの切り分けに要る）:
  - `[SetProcessDpiAwarenessContext] DPI awareness set`（成功・`info!`）／`... failed`（失敗・`warn!`）——target `wintf::runtime`・D-BASE の大域 `info` で点灯。**`WM_DPICHANGED` が 0 件である機序の第一次切り分けはここでしかできない**（Req 7.4）
  - `[detect_display_change_system] Display configuration change applied` の **`windows_redriven=N`**——「モニタ表は更新されたが窓が 1 つも駆動されなかった」を 1 行で切り分けられる
- **`[detect_display_change_system] Updating Monitor entity` に `old_dpi=`／`new_dpi=`／`old_work_area=`／`new_work_area=`／`old_bounds=`／`new_bounds=`／`old_primary=`／`new_primary=` が載る**（4.6 新設）。モニタ表が実際に何を反映したかを実機ログだけで復元できる。
- 対象は**キャラ窓の entity のみ**。バルーン窓（`kind=balloon`）の受理を混ぜると計数が壊れる。
- 方向は**同一行の `old_dpi_x` と `new_dpi_x` の大小比較**で機械判定する（`new > old` = 低→高、`new < old` = 高→低、`new == old` = **どちらにも数えない**）。
- 「15 分回した」「何回もまたいだ」は充足の根拠にならない。**数えた数字だけが根拠である。**

### 5.2 第 1 段: scope → キャラ窓 entity の対応表を作る

O4 のレコードから、`kind=char` の行だけを取り、`scope=` と `entity=` の組を一意化する。

```powershell
$log = "$LOGDIR\session2-osdpi.log"

$map = Select-String -Path $log -SimpleMatch '[diag.window_move]' |
  ForEach-Object { $_.Line } |
  ForEach-Object {
    if ($_ -match 'entity=(\d+v\d+) kind=char scope=(\d+) ') {
      [pscustomobject]@{ scope = [int]$Matches[2]; entity = $Matches[1] }
    }
  } | Sort-Object scope, entity -Unique

$map | Format-Table -AutoSize
```

- 正規表現が `entity=… kind=char scope=… ` を**この順で連続**して要求しているのは、`window_move_record_line`（`diag.rs:303-312`）のフィールド順が固定だからである（in-source 檻がリテラル固定）。
- **表が空になったら手順の失敗である**（`areka::placement::diag=debug` の入れ忘れ）。空の表を根拠に「受理 0 回」と結論してはならない。
- セッション②（ドラッグ禁止）でも表は必ず作れる——起動直後の drain 相 k₀ 補正が `route=ReportedSizeReconcile` のレコードを出すため（§4.1）。
- **代替源**: D-TEARDOWN で採っていれば X3（`spawn.rs:124`）の 1 行に `scope=`・`char_window=`・`balloon_window=` が揃う。第 1 段の独立確認に使える。
- entity 値（`{index}v{generation}`）は**実行ごとに変わる**。セッションごとに第 1 段からやり直すこと。

### 5.3 第 2 段: 当該 entity の受理行を数え、同行の新旧 DPI で方向を決める

```powershell
$acc = Select-String -Path $log -Pattern 'DPI component directly updated|Redriving window DPI from updated Monitor' |
  ForEach-Object { $_.Line } |
  ForEach-Object {
    if ($_ -match 'entity=(\d+v\d+).*?old_dpi_x=(\d+).*?new_dpi_x=(\d+)') {
      [pscustomobject]@{ entity = $Matches[1]; old = [int]$Matches[2]; new = [int]$Matches[3] }
    }
  }

$result = foreach ($m in $map) {
  $rows = @($acc | Where-Object { $_.entity -eq $m.entity })
  $up   = @($rows | Where-Object { $_.new -gt $_.old }).Count
  $down = @($rows | Where-Object { $_.new -lt $_.old }).Count
  [pscustomobject]@{
    scope    = $m.scope
    entity   = $m.entity
    'low2high' = $up
    'high2low' = $down
    total    = $rows.Count
    ok       = ($up -ge 3 -and $down -ge 3)
  }
}

$result | Format-Table -AutoSize
$sum = ($result | Measure-Object -Property total -Sum).Sum
$verdict = if ((@($result | Where-Object { -not $_.ok }).Count -eq 0) -and $sum -ge 12) { 'SESSION-QUOTA: PASS' } else { 'SESSION-QUOTA: FAIL' }
"$verdict (total=$sum)"
```

**合否判定語**（`diagnosis-report.md` へそのまま転記する）:

- `SESSION-QUOTA: PASS` — 全 scope で `low2high >= 3` かつ `high2low >= 3`、合計 12 回以上。
- `SESSION-QUOTA: FAIL` — 上記を満たさない。**このセッションは要件 2.6 の「再現しない」の根拠に用いてはならない。** 操作を追加して採り直す。

> 実行例の 1 行（本書作成時に `EnvFilter` 実濾過で採取した実出力）:
> `2026-07-31T05:33:43.443522Z DEBUG wintf::ecs::window_proc::window_pos: [WM_DPICHANGED] DPI component directly updated (Changed<DPI>) entity=3v0 old_dpi_x=96 old_dpi_y=96 new_dpi_x=192 new_dpi_y=192`

### 5.4 grep のトークン境界（**必読**・誤計数の温床）

| 落とし穴 | 正しいアンカー |
| --- | --- |
| `w=-` は `w=-12` の**接頭辞**（`-` は「値なし」の番兵・`diag.rs:98`。負の座標・寸と衝突する） | 後続の空白まで含める（`w=- ` / 正規表現 `w=-(?=\s)`）。`dpi=-` は行末ゆえ `dpi=-$` |
| `entity=1v1` は `entity=1v10`・`entity=1v11` の接頭辞 | 後続の空白を含める（§5.2/5.3 の正規表現は次フィールド名まで要求している） |
| `scope=1` は `scope=10` の接頭辞 | 同上（`scope=(\d+) ` と空白まで） |
| `[diag.window_move]` などの `[` `]` は正規表現メタ文字 | `Select-String -SimpleMatch` ／ `rg -F` を使う |
| `(Changed<DPI>)` の `<` `>` も同様 | 同上 |
| route 名同士の接頭辞衝突 | **無い**（9 語の全 36 対を目視で確認済み。近接語 `Restore`／`Resnap`／`ReportedSizeReconcile` も 3〜4 文字目で分岐する）。ただし**檻が固定しているのは語数と相異のみ**（`placement_route_all_covers_nine_distinct_variants`）で、接頭辞非衝突は檻に入っていない——将来 `Resnap2` のような変種を足すと檻は緑のまま本節の grep アンカー保証だけが静かに壊れる。route 語彙を増やす際は本行を再検証すること |

---

## 6. 実機採取の 2 セッション（要件 1.8）

**2 本は独立したプロセス起動**とし、ログを別ファイルへ保存する。env は §2.6 と**完全に同一**にし、差は操作のみとする（差が env にもあると、どちらの差が効いたか事後に分離できない）。

### 6.1 セッション①: ドラッグによるモニタ跨ぎ**のみ**

**目的**: Q1（暴走か操作どおりか）・Q3（ドラッグ以外の経路か）の切り分け材料、要件 2.3 の数値評価。

1. `$LOGDIR\session1-drag.log` へ保存する形で §2.6 のコマンドを実行する。
2. 起動直後、キャラ 2 体（むらさき=scope 0・えも=scope 1）とバルーンが表示されるまで待つ。
3. **OS の表示設定には一切触れない**（触ったら §6.2 との切り分けが壊れる＝セッション破棄）。
4. scope 0 のキャラ窓を掴んで、**拡大率の異なるモニタ境界を跨いで**移動する。低 DPI→高 DPI と高 DPI→低 DPI を**各 3 往復以上**。
5. scope 1 のキャラ窓についても同じ操作を行う。
6. ドラッグ中はバルーンが随伴していることを目視し、**消えた瞬間があればその時刻を `meta.txt` へ秒単位で記録**する（ログの突合起点になる）。
7. 自動終了を待つ（または全キャラ窓を閉じる）。**プロセスを強制終了しない**——終了時ログ（要件 6.2）が採れなくなる。
8. §5 の 2 段 grep を実行し、`SESSION-QUOTA:` を記録する。

**このセッションでのみ観測できるもの**: O11〜O15（ドラッグ）。要件 2.3 は O14 の `delta_x`/`delta_y`（カーソル移動量）と、直後の O9 の `x=`/`y=`（窓の実移動量）を `hwnd` で結合して数値比較する。

### 6.2 セッション②: **ドラッグ禁止**・OS 設定側から DPI 変更のみ

**目的**: 要件 2.4 の「ドラッグ以外の経路」を単独で観測する。ドラッグが混ざると、書き手の名指しが原理的にできない。

1. `$LOGDIR\session2-osdpi.log` へ保存する形で §2.6 のコマンドを実行する（**新しい `$PROFILE_DIR`** を与える）。
2. **ゴースト窓を一切マウスで掴まない。** ドラッグはもちろん、ゴースト上での左ボタン押下も行わない（`[start_preparing]` が 1 行でも出たらこのセッションは無効＝採り直し）。
3. 「設定 > システム > ディスプレイ > 拡大縮小」で、**ゴースト窓が載っているモニタ**の拡大率を切り替える。125% ↔ 200% の往復を **3 回以上**（＝低→高 3 回・高→低 3 回）。
   - 両 scope が同一モニタに載っていれば、1 回の変更で両 scope の窓が同時に `WM_DPICHANGED` を受理する＝往復 3 回で 12 回の受理に達する。
   - scope が別モニタに分かれている場合は、**両方のモニタで**往復 3 回ずつ行う。
4. 各変更のあと、キャラとバルーンが見えているかを目視し、消えたら時刻を `meta.txt` へ記録する。
5. 自動終了を待つ。
6. §5 の 2 段 grep を実行し、`SESSION-QUOTA:` を記録する。
7. **無効化チェック**: `Select-String -Path $log -SimpleMatch '[start_preparing]'` が 0 行であることを確認する。判定語 `SESSION2-NO-DRAG: PASS` / `FAIL`。

### 6.3 消失痕跡の判定（両セッション共通）

§3.3 のとおり **`ClampX`／`NearestFallback` の `warn!` は Phase A のビルドにまだ存在しない**。したがって消失痕跡は、**幾何の事後突合**で判定する（これは Phase C 以後も有効な、より原理的な判定である）。

1. `[diag.monitor]`（O3・`context=monitor_snapshot` の組）から work area 矩形の集合を作る。
2. `[diag.window_move]`（O4）の各行から `(x, y, w, h)` を取る。`w=-`／`h=-` の行は**移動専用書込**ゆえ、同一 entity の直近の既知寸を持ち越す。
3. 矩形 `(x, y, x+w, y+h)` が**どの work area とも交差しない**行があれば、それが「真の不可視」への遷移の証跡である。書き手は同じ行の `route=` が名指しする。
4. ドラッグ・OS 直書き由来の位置は O4 に出ない（§4）。O9 の `x=`/`y=` を `hwnd` で結合して同じ突合を行う。
5. 判定語（`diagnosis-report.md` へ転記）:
   - `VANISH-TRACE: NONE` — 全 work area 非交差の矩形が 1 件も無い。
   - `VANISH-TRACE: FOUND route=<route 語 or drag or os-suggested> entity=<…> rect=<l,t,r,b>` — 発見。要件 2.2 の「真の不可視／可視領域内の見落とし」の判別結論を併記する。

> 要件 2.6 の適用: **`SESSION-QUOTA: PASS` の 2 セッション**の双方で `VANISH-TRACE: NONE` のときに限り「再現しない」と結論できる。そのとき除外できるのは**実機でしか確定できない残余仮説に対する追加修正のみ**であり、静的構造証跡で確定した S1〜S3′ の是正と回帰檻は**除外しない**。

### 6.4 終了時静穏の確認（要件 6.2/6.3）

自動終了後のログ末尾について:

- **D-BASE のまま**判定できる: 破棄済み窓に対する `WARN`／`ERROR` が 0 行であること。
  判定語 `TEARDOWN-SILENCE: PASS` / `FAIL`。
- **肯定側**（`[despawn-skip]` が実際に出ている・他 scope が処理を継続している）を見たい場合のみ、D-TEARDOWN で追加の 1 回を回す。**D-BASE のログに `[despawn-skip]` が 0 行なのは消灯しているだけであり、何の根拠にもならない**（§3.2）。

### 6.5 S1 是正の実機成立判定（**タスク 7.4 専用**・5.1 着地後のビルドでのみ使う）

> **Phase A/B の採取（セッション①・②-b）にこの節を適用してはならない**——当時 `applied` は定数であり、以下の判定はすべて偽陰性になる（§3.3・§3.4）。
>
> **前提: 当該セッションが `SESSION-QUOTA: PASS`（§5）であること。** 下の手順 1 は「O8 の件数が O7（DPI 受理）と同数」を求めるが、これは **`0 == 0` でも成立してしまう**——DPI 受理が一度も起きていない死んだセッションでも合否判定語が `PASS (external=0/0, boxstyle_warn=0, x_divergence=0)` と綺麗に埋まる。**`N = 0` は `PASS` ではなく `N/A` とすること。** 受理回数の下限を踏破していないセッションを是正成立の根拠に用いない（要件 1.9／2.10）。

S1 是正は「ゴースト窓の DPI 受理で OS 提案位置が**書かれない**」ことで成立する。**0 件を数える判定は単独では偽陰性と区別が付かない**（点灯していないだけかもしれない）ため、**必ず肯定側の相方と対で読む**。

**手順**（§5.2 の第 1 段で作った「scope → キャラ窓 entity」の対応表を前提とする）:

1. **肯定側**: ゴースト窓 entity の O8 行を数える。全件が `policy=ExternalAuthority` かつ `applied=false` であること。件数はその entity の **O7（DPI 受理）と同数**であること（＝受理のたびに政策判断が下っている＝観測点が点灯している証拠）。
2. **否定側**: 同じ受理に対応する O9（`[guarded_set_window_pos]`）に、O6 の `suggested_left`／`suggested_top` と一致する `x=`／`y=` が **1 件も現れない**こと（§7.1 の手順 3 の反転）。
3. **非退行の対照**: 非ゴースト窓が存在する走行では、その O8 が `policy=unset`・`applied=true` を報告し、O9 に提案座標が現れること（Per-Monitor v2 の標準応答の維持）。
4. **相方の消滅**: `[WM_WINDOWPOSCHANGED] DPI center correction skipped: BoxStyle not found`（`warn!`・target **`wintf::ecs::window_proc::dpi_helpers`**・D-BASE の大域 `info` に載る）が **0 件**であること。セッション①では DPI 受理と同数（84 件）出ていた。5.1 で `DpiChangeContext` がゴースト窓では立たなくなり、`correct_position_for_dpi_center_preserve`（`window_proc/dpi_helpers.rs:96`）が冒頭 `:104` で打ち切るため当該 `warn!`（`:110`）へ到達しなくなる。
   - **⑴と⑷は対で読む**。`applied=false` が受理と同数（肯定側の点灯）でありながら `BoxStyle not found` が 0 件（否定側）である、という**組**が S1 是正の実機成立である。⑷だけが 0 件なら「そもそも DPI 受理が起きていない」だけかもしれず、根拠にならない（要件 1.5）。
5. **X 差の消滅**（§2.5.1 の指紋との対比）: セッション①で観測された二重ライターの差分は「Y が定数・**X が可変（−861〜+861）**」だった。X の可変性が S1 の指紋ゆえ、是正後は **O9 と `[diag.window_move]` の X が一致し、861px 級の X 差が 0 件**になる。

**合否判定語**（`diagnosis-report.md` §4 へ転記）:

- `S1-SOURCE-CUT: PASS (external=N/N, boxstyle_warn=0, x_divergence=0)` — 1〜5 がすべて成立。`N` はゴースト窓の DPI 受理件数。
- `S1-SOURCE-CUT: FAIL <項番>` — 不成立。とりわけ `policy=unreachable` が 1 行でも出ていたら**その受理は計数から除外**し、原因（World 借用の再入・entity 破棄）を先に潰す（§3.4）。
- `S1-SOURCE-CUT: N/A` — 採取ビルドが 5.1 着地前である（§0.1 の順序制約の確認漏れを疑う）、**または `N = 0`**（ゴースト窓の DPI 受理が 1 件も無い＝上記の前提を満たさない）。

---

## 7. 実機サインオフ（要件 5.5・決定論化できない残余）

決定論檻に入れられないのは次の 2 項だけである。両方とも「有界自動終了 ＋ ログ照合」で合否を出す（記憶〈実機サインオフは有界 auto-exit ＋ログ grep〉）。

### 7.1 残余 A: **OS が実際に提示する提案矩形**

決定論檻は提案矩形を**注入**するため、OS が実際に何を提示するかは実機でしか判らない。

**確認手順**（セッション①②のどちらでも成立するが、②が本命）:

1. O6（`WM_DPICHANGED` 受信行）から `suggested_left`・`suggested_top`・`suggested_right`・`suggested_bottom` の実値を取り出す。
2. 同じ受理に対応する O8（`[WM_DPICHANGED] suggested position write decision`）の `suggested_left=`／`suggested_top=` と一致することを確認する。
3. 直後の O9（`[guarded_set_window_pos] Calling SetWindowPos`）の `x=`／`y=` が、その `suggested_left`／`suggested_top` と**一致する**ことを確認する（＝OS 提案位置が実際に窓へ書かれている＝S1 の実機痕跡）。`flags=` に `SWP_NOSIZE` が立っている（位置のみ）ことも見る。
4. 提案矩形の `left` が、**変化後のモニタ側の座標系へ変位している**ことを確認する（モニタ跨ぎ相当の X 変位が実在することの実測）。

**合否判定語**:

- `RESIDUE-A-SUGGESTED-RECT: PASS` — 上記 1〜4 がすべて確認でき、提案矩形の実値がログから復元できた。
- `RESIDUE-A-SUGGESTED-RECT: FAIL` — O6／O8／O9 のいずれかが欠落しており実値を復元できない（＝手順の設定ミスを疑う。`wintf::ecs::window=debug` の入れ忘れが第一容疑）。

> **是正後（タスク 7.4）の再サインオフでは判定が反転する**: S1 是正後、ゴースト窓では O8 が `policy=ExternalAuthority`・`applied=false` を報告し、O9 に提案座標が現れないことが PASS 条件になる（非ゴースト窓では従来どおり）。**Phase A の採取でこれを期待してはならない**——§3.3 のとおり Phase A の `applied` は定数 `true` である。反転後の完全な判定手順と合否判定語は **§6.5**（`S1-SOURCE-CUT`）が持つ。本節（残余 A）は「OS が実際に何を提示したか」の復元が目的であり、**是正後も手順 1・2・4 はそのまま有効**（提案矩形の実値は O6／O8 から復元できる＝書かないことと記録しないことは別である）。手順 3 だけが §6.5 の否定側へ反転する。

### 7.2 残余 B: **実モニタ列挙**

合成モニタでは実機の handle・work area・混在 DPI の実配置を再現できない。

**確認手順**:

1. O1（`context=monitor_snapshot`）の `count=N` が、実機の実際のモニタ台数と一致することを確認する。
2. 各 O3（`[diag.monitor]`）の `dpi=` が、OS の拡大率 × 96 ÷ 100 と一致することを確認する（125%→120・150%→144・200%→192）。
3. O1 群（`context=monitor_snapshot`）と O2 群（`context=prepare_ghost_windows`）の `[diag.monitor]` 行が**同値**であることを確認する（D12 の共有語彙 grep 突合。食い違えば、配置が読む権威と準備時の列挙が乖離しているという**それ自体が発見**である）。
4. wintf 側 O5（`[initialize_layout_root] Creating Monitor entity`）の `handle=`・`work_area=` が O3 と一致することを確認する（フィールド名は両側で共有語彙）。
5. `work_area` が `bounds` より小さい（タスクバー分）ことと、負座標・3200 超座標が忠実転写されていることを確認する。

**合否判定語**:

- `RESIDUE-B-MONITOR-ENUM: PASS` — 1〜5 すべて一致。
- `RESIDUE-B-MONITOR-ENUM: FAIL <項番>` — 不一致。不一致の実値を `diagnosis-report.md` へ引用する。

---

## 8. 本書の判定語が実際に点灯することの検証記録（タスク 4.1 実施時）

`RUST_LOG` の推測は本 spec が排除しようとしている偽陰性そのものを生むため、**D-BASE は実際に `EnvFilter` で濾して点灯を確認した**（`crates/wintf/src/ecs/test_support.rs` の `capture_under_filter` ／ `crates/areka/src/placement/test_support.rs` の実濾過ハーネス）。確認は一時的な probe テストで行い、確認後に撤去した（本 spec は本番コードを変更しない）。

D-BASE で実際に得られた出力（抜粋・書式はこのとおり）:

```
DEBUG areka::placement::diag: [diag.monitor_snapshot] context=prepare_ghost_windows count=1
DEBUG areka::placement::diag: [diag.monitor] index=0 handle=1 bounds=0,0,1920,1080 work_area=0,0,1920,1040 dpi=120 primary=true
DEBUG areka::placement::diag: [diag.window_move] route=SpawnInitial entity=1v0 kind=char scope=0 x=0 y=0 w=1 h=1 dpi=96
DEBUG wintf::ecs::layout::systems::monitor_systems: [initialize_layout_root] Creating Monitor entity handle=305419896 bounds_left=-1920 bounds_top=0 bounds_right=0 bounds_bottom=1200 work_area=-1920,0,0,1160 dpi=192 is_primary=false
DEBUG wintf::ecs::window_proc::window_pos: WM_DPICHANGED hwnd=HWND(0x0) dpi_x=192 dpi_y=192 scale_x=2.00 scale_y=2.00 suggested_left=3210 suggested_top=140 suggested_right=3810 suggested_bottom=620
DEBUG wintf::ecs::window_proc::window_pos: [WM_DPICHANGED] DPI component directly updated (Changed<DPI>) entity=3v0 old_dpi_x=96 old_dpi_y=96 new_dpi_x=192 new_dpi_y=192
DEBUG wintf::ecs::window_proc::window_pos: [WM_DPICHANGED] suggested position write decision entity=3v0 hwnd=HWND(0x0) applied=true suggested_left=3210 suggested_top=140
DEBUG wintf::ecs::window::command: [guarded_set_window_pos] Calling SetWindowPos hwnd="0x0" x=3210 y=140 cx=0 cy=0 flags=SET_WINDOW_POS_FLAGS(21)
DEBUG wintf::ecs::drag::state::…: [drag] …            （debug 水準の点灯を確認）
TRACE wintf::ecs::drag::dispatch 相当: [DragEvent] … （trace 水準の点灯を確認）
```

併せて、既定水準（`info`）では O8・O9 が **1 行も出ない**ことも確認した（診断専用のまま＝要件 1.7）。

> **上の抜粋は 4.1 実施時（Phase A）の書式である。** O8 の行は **タスク 5.1 で `policy=` が加わり、`applied` が分岐結果になった**。5.1 着地後の実測書式は次のとおり（in-source 檻 `s1_decision_line_reports_external_authority_and_applied_false`／`_unset_policy_and_applied_true` がリテラルを固定している）:
>
> ```
> DEBUG wintf::ecs::window_proc::window_pos: [WM_DPICHANGED] suggested position write decision entity=3v0 hwnd=HWND(0x0) policy=ExternalAuthority applied=false suggested_left=2400 suggested_top=800
> DEBUG wintf::ecs::window_proc::window_pos: [WM_DPICHANGED] suggested position write decision entity=3v0 hwnd=HWND(0x0) policy=unset applied=true suggested_left=2400 suggested_top=800
> ```
>
> `policy=` の値に**引用符が付かない**ことも檻が固定している（`format_args!` 経由。素の `&str` として渡すと `policy="unset"` になり、同行の他フィールドと grep の当たり方が変わる＝§5.4 のトークン境界の罠と同型）。

> **本書のメンテ規約**: 出力書式は in-source 檻がリテラル固定している（`diag.rs` の `record_tags_are_fixed`・`monitor_record_line_carries_every_field`・`window_move_record_line_carries_every_field` ほか）。書式を変えれば檻が赤になる。**檻を直すときは本書の判定語も同じコミットで直すこと**——直さなければ本書が静かに嘘になる。

---

## 9. 採取後に `diagnosis-report.md` へ渡すもの（受け渡し契約）

本書は手順のみを定める。以下を**そのまま**レポートへ転記する（結論の記述はレポート側の仕事）。

| 項目 | 形式 |
| --- | --- |
| 採取ビルド | コミット SHA ・プロファイル |
| モニタ構成 | O1/O3 の全行（実値） |
| セッション①充足 | `SESSION-QUOTA: PASS|FAIL (total=N)` ＋ scope 別 `low2high`/`high2low` 表 |
| セッション②充足 | 同上 ＋ `SESSION2-NO-DRAG: PASS|FAIL` |
| 消失痕跡 | `VANISH-TRACE: NONE` または `FOUND …`（両セッション分） |
| 終了時静穏 | `TEARDOWN-SILENCE: PASS|FAIL` |
| 残余 A | `RESIDUE-A-SUGGESTED-RECT: PASS|FAIL` ＋ 提案矩形の実値 1 例 |
| 残余 B | `RESIDUE-B-MONITOR-ENUM: PASS|FAIL` |
| 生ログ | 保存パス（リポジトリ外）・引用した行番号 |

要件 2.9 の「S1〜S3′ の実機痕跡の有無」も本書の判定語で埋まる（痕跡が無くても**静的構造証跡による確定は取り消さない**）。
