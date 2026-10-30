# animated-surface-log-flood の確かめの記録

## 作る前の測り（タスク 1・要件 6.2・6.3・6.6・6.7・設計討議の決め 1）

コードを変えずに、ぱすたの面 1002 のコマ替えで、表示の適用 1 回にかかる時間を段ごとに測った。

### 条件

- 日時: 2026-10-30。release 版 12:03:42〜12:05:12Z、debug 版 12:05:36〜12:07:06Z（どちらも 90 秒）。
- コード: `904845747`（タスク生成のコミット。areka のコードは直す前のまま）。
- 机: 進行役が排他テストの机とひとりの机の両方を持った（test・sole）。走行の前の `tools/perf/check-quiet.ps1`（既定の値: 20 秒・閾値 10%）の判定は、両版とも `verdict=QUIET`。機械ぜんたいの CPU は、release 版の前が平均 4.0%（最大 8.4%）、debug 版の前が平均 3.1%（最大 6.9%）。
- 機械: 論理 CPU 22。
- 起こし方: 手順書 `doc/pasta-real-run.md` 4 節の起動の塊のとおり。`RUST_LOG` には `areka_emo_present=debug` を足した。`AREKA_APP_SMOKE_EXIT_MS=90000` で、両版とも自分で終わった（終了コード 0）。ゴーストは `--ghost` に絶対パスで渡した。記憶は空（`OnFirstBoot`）で、起動の後は何も操作していない。`PASTA_DEBUG` は消した（9276 の待受の持ち主は走行の前に 0 件）。
- 書庫: pasta-in-windows の sha256 は `4d2b3efa9f3cd561b0dc3ec3e140e4fdd7b73551fd88f8ef9ba986ca1c206eb9`。
- 置き場: `<w>\target\animated-surface-log-flood\pre\`（根は `root\`、記録は `logs\pre-<版>.*`、静かさの確かめは `logs\quiet-pre-<版>.txt`、5 秒ごとの値は `logs\pre-<版>.health.csv`）。
- 数え方:
  - 対象は面 0 の perf の行（`perf(apply_show): 段階別計時`）のうち `surface_id=1002` の行だけ。どの行も `cache_hit=false` だった。
  - 中央値は段ごとに取った。合成の割合は、行ごとの `t_compose_us / t_total_us` の中央値。
  - areka の CPU は 1 コアに対する割合（5 秒あたりの CPU 秒 ÷ 5）。

### 段ごとの所要（面 1002・µs の中央値）

| 版 | 行数 | 面 1002 の時間（Z） | 頻度 | 合成 `t_compose_us` | マスク `t_mask_us` | 表示の記録 `t_upload_us` | 全体 `t_total_us` | 合成の割合 |
|---|---|---|---|---|---|---|---|---|
| release | 1,210 | 12:03:47〜12:05:11 | 14.3 回／秒 | 1,028 | 203 | 430 | 2,944 | 37% |
| debug | 499 | 12:05:41〜12:06:15 | 14.4 回／秒 | 3,222 | 3,623 | 454 | 8,702 | 38% |

- debug 版のぱすたは、12:06:16Z 以降に面 1000 へ替わった（台本の流れによる。面 1000 も動く絵で、723 行・毎秒 8.1 回、合成 11,223 µs・全体 17,579 µs・合成の割合 66%）。上の表の debug 版は、面 1002 が出ていたおよそ 34 秒だけの値である。
- release 版は、最初の 5 秒（面 1001・73 行）の後、終わりまで面 1002 のままだった。

### 適用 1 回の所要を areka の CPU と比べる

| 版 | 表示の適用の全体（1 コアに対して） | うち合成 | 走行中の areka の CPU（中央値） | 走行中の機械ぜんたいの CPU（中央値） | 直す前の基準（件 25・既定の水準・6 分） |
|---|---|---|---|---|---|
| release | 2.94 ms × 14.3 回／秒 ≒ 4.2% | 1.03 ms × 14.3 ≒ 1.5% | 14%（最初の 6 行＝およそ 30 秒を除いた 10 行） | 4% | 20% |
| debug | 8.70 ms × 14.4 回／秒 ≒ 12.5% | 3.22 ms × 14.4 ≒ 4.6% | 50%（面 1002 のあいだの 7 行） | 8% | 69% |

- 合成は、表示の適用の中では最大の段の 1 つである（全体の 37〜38%）。それでも、areka の CPU の全体から見ると、release 版で 1.5 ポイント、debug 版で 4.6 ポイントしかない。
- 表示の適用ぜんたいを 0 にしても、下がるのは release 版で 4.2 ポイント、debug 版で 12.5 ポイントまで。areka の CPU の残り（release 版で 10 ポイントほど、debug 版で 37 ポイントほど）は、perf の行が測る段の外で使われている。設計の判断 D4 で本 spec が縮めないとした wintf の全面の描き直し、動く絵のコマを進める処理、記録を書く処理などが含まれるはずだが、内訳はこの測りでは分からない。
- この測りでは、直す前のコードのままでも、areka の CPU が基準より低かった（release 版 14%／基準 20%、debug 版 50%／基準 69%）。理由として考えられるのは、走行の長さ（90 秒と 6 分）、機械ぜんたいの負荷（件 25 の中央値 15〜16% と今回の 4〜8%）、debug の水準の記録の有無の違い。どれが効いたかは確かめていない。同じ条件で回し直したときのばらつき（数ポイント）は、合成を縮めて見込める下げ幅（release 版で最大 1.5 ポイント）と同じ大きさになる見込みである。

### 判断: 進む（開発者の決め: 案 a）

- 測った時点の判断は「仰ぐ」だった（理由は下の 4 項）。開発者は案 a を選んだ（2026-10-30）。要件 6.2 の基準を、件 25 と同じ条件で、直す前のコードのまま取り直す。合成の矩形化はそのまま進める。取り直しは次の節に書く。

- 合成を変わった矩形だけにしても、下がるのは最大でも release 版 1.5 ポイント・debug 版 4.6 ポイント（1 コアに対して）である。これは、回し直しのばらつきと基準の条件の違いに埋もれる大きさである。要件 6.2 の「両版とも直す前より下がる」を、合成を縮めたことの効果として示す見込みは薄い。
- 一方で、今回の直す前の値は、すでに両版とも基準（20%・69%）を下回った。基準とただ比べるだけなら、直さなくても合格に見えてしまう。要件 6.2 の比べ方（基準をどの条件で取り直すか）にも決めが要る。
- debug 版では、マスク（3.6 ms）が合成（3.2 ms）より大きい。release 版では、合成・マスク・表示の記録の 3 段を合わせても 2.9 ms で、CPU の多くは perf の行の外にある。
- 以上から、タスク 2 以降には進まず、開発者に決めを仰いだ。示した候補は、(a) 要件 6.2 の基準を同じ条件で取り直す、(b) マスクと GPU への上げも矩形にする、(c) wintf の全面の描き直しに触る、の 3 つ。

## 直す前の基準（同じ条件で取り直し）

### 開発者の決め（2026-10-30）

- 案 a を採る。要件 6.2 の直す前の基準を、タスク 5.2 と設計の「何もしていないあいだ（要件 6）」と同じ条件（件 25 の測り方）で、直す前のコードのまま、debug 版と release 版で 1 回ずつ取り直す。合成の矩形化はそのまま進める。
- 取り直した値を、タスク 5.4 が比べる基準にする。

### 手順

- コード: コミット `a90cdc067`。木の中のコードは `904845747` と同じ（`git diff --stat 904845747 a90cdc067 -- . ':!.kiro'` が空）。areka.exe（debug 版・release 版）と i686 helper は、タスク 1 で `904845747` から作ったものを使った。
- 置き場: `<w>\target\animated-surface-log-flood\baseline-debug\` と `baseline-release\`。根は `pre\root` を写して毎回作り直し、記憶（`profile` とゴーストの中の `profile`）は空にした。
- 起こし方: 手順書 `doc/pasta-real-run.md` 4 節の起動の塊のまま。
  - `RUST_LOG` は手順書の既定の値（`info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug,areka::input_events=debug`）で、`areka_emo_present=debug` は足していない。
  - `AREKA_APP_SMOKE_EXIT_MS=360000`（6 分）。`PASTA_DEBUG` は消した。
- 測り方:
  - 5 秒ごとに、areka の WorkingSet・主の窓の `Responding`・5 秒あたりの CPU 秒・機械ぜんたいの CPU を `logs\baseline-<版>.health.csv` に書いた。
  - CPU は 1 コアに対する割合（5 秒あたりの CPU 秒 ÷ 5）。最初の 60 秒は数えない（両版とも 60 行を数えた）。
- 机: 進行役が排他テストの机とひとりの机の両方を持った（test・sole）。走行の前の `tools/perf/check-quiet.ps1` の判定は、両版とも `QUIET`（`logs\quiet-baseline-<版>.txt`）。
- 取り直しは 2 回回した。1 回目は外からの入力が入ったので基準に使わない。2 回目を基準にする（下の「2 回目」）。

### 1 回目（2026-10-30・邪魔が入った・基準に使わない）

- 記録は `<w>\target\animated-surface-log-flood\disturbed-1\`（`baseline-debug\`・`baseline-release\`）へ写して残した。2 回目の走行で `baseline-*` の中身は作り直したので、1 回目の記録は写しの方だけにある。
- 順: debug 版（12:24:22〜12:30:23Z・pid 35788）、続けて release 版（12:30:47〜12:36:48Z・pid 26648）。

#### 結果

| 版 | 測りの窓（Z） | areka の CPU（中央値／最大） | WorkingSet（最小〜最大・中央値） | 答えなくなった回数 | panic・ERROR | 終了コード | 機械ぜんたいの CPU（中央値／最大） | 窓の中の面 |
|---|---|---|---|---|---|---|---|---|
| debug | 12:25:23〜12:30:19 | 66.4% ／ 96.4% | 198〜201 MB・198 MB | 0 | 0・0 | 0 | 9.7% ／ 40.1% | 面 1000 だけ（面 1002 は 0 秒） |
| release | 12:31:48〜12:36:43 | 28.4% ／ 67.7% | 185〜187 MB・187 MB | 0 | 0・0 | 0 | 8.2% ／ 76.2% | 面 1000 だけ（面 1002 は 0 秒） |

#### 要件 6.4（面 1002 が出ていたか）の確かめ: 両版とも満たさない

- シェルの対象（面 0）の INFO の表示の行で、測りの窓の前の最後の行は、両版とも `surface_id=1000` だった。窓の中の INFO の表示の行もすべて面 1000（debug 版 4,423 行・release 版 4,486 行）。窓の中に面 1002 の行は無い。
- 面の替わり（面 0 の INFO の表示の行）:
  - debug 版: 面 1001（12:24:24Z）、面 1002（12:24:28Z）、面 1000（12:24:37.423Z）。その後は面 1001 と面 1000 を何度か行き来し、12:24:38.506Z 以降は面 1000 のまま。
  - release 版: 面 1001（12:30:48Z）、面 1002（12:30:53Z）、面 1000（12:31:01.648Z）。その後は同じく行き来し、12:31:02.746Z 以降は面 1000 のまま。
- 面 1000 へ替わった直前には、両版とも起動から 14 秒ほどで、シェルの「セキュリティボール」の当たりのクリックが届いていた。
  - debug 版: 12:24:37.367Z の `mouse_event_sent id="OnMouseUp"` と `id="OnMouseClick" scope=0 region="セキュリティボール"`。その前に `cursor_answer … scene="Down"`（12:24:37.231Z）がある。
  - release 版: 12:31:01.625Z に同じ並び。
- このクリックに答えて、ぱすたの台本（`steady_talk origin="OnMouseClick"`）が流れ、面 1000 になった。
- このクリックは areka の外からの入力である（測りの道具は画面を操作していない）。誰が押したかは記録からは分からない。
- 面 1000 も動く絵の面である（タスク 1 の debug 版で、面 1000 の perf の行は毎秒 8.1 回、合成 11,223 µs）。面 1002 とは負荷が違うので、この 2 本の値は面 1002 の基準として使えない。

#### release 版: 邪魔が入った・基準に使わない

上の面の件とは別に、測りの窓の中に外からの操作が入った。

- 12:32:21.734Z（日本時間 21:32:21）: SSTP（`label=SSTP(Local,Auth)`）で台本 `\0\![bind,まばたき,ふつう,0]\![bind,まばたき,閉じるだけ,0]\e` が届いた。
- 12:32:30.859Z: 同じく SSTP で `\0\![bind,まばたき,ふつう,1]\e` が届いた。
- 12:33:24.508Z: `mouse_event_sent id="OnMouseUp"` と `id="OnMouseClick" scope=1 region="窓"` が届き、`steady_talk talk_id=7 origin="OnMouseClick"` の台本が流れた。
- 別のセッションの知らせでは、「ぱすた開発」のセッションが MCP 経由で台本を送り、記録 #79 の OnMouseClick が 21:31 ごろに起きた。上の行はこの知らせと合う。
- `health.csv` で、この時間帯の areka の CPU は 12:32:13〜12:33:28Z に 30〜55%（12:33:08Z に 55%）。窓ぜんたいの中央値 28.4% より高めの区間で、12:33:33Z 以降は 20〜21% に下がった。
- 判断: release 版の値は基準に使わない。release 版は取り直す。

#### debug 版: 窓の中に外からの操作は無い（ただし要件 6.4 を満たさない）

- 測りの窓（12:25:22Z 以降）の中に、SSTP・MCP の台本、`mouse_event_sent`、`input_events` の行は無い。外からのクリックは、窓の前（12:24:37Z）の 1 回だけ。
- 窓の中で流れた台本は、ぱすた自身の `OnSecondChange`（12:29:36Z・12:29:58Z）だけ。
- それでも、窓のあいだずっと面 1000 だったので、要件 6.4 を満たさない。debug 版も、面 1002 の基準としては使えない。

#### 目の重ね（seriko）の様子（release 版の記録から読めること）

- release 版の seriko の行は 560 行。そのうち `loop 抽選発火` が 79 行、`loop 停止（負 surface でベース復帰）` が 79 行で、数がそろっている。debug 版は 449 行で、それぞれ 70 行。
- release 版の `まばたき` の bind の適用は 13 行。12:33:24Z に `閉じるだけ`（id 1401）が on、12:34:49Z にもう一度 on になった。その後に `閉じるだけ` を off にする行は、記録の終わり（12:36:48Z）まで無い。最後の `ふつう`（id 1400）は 12:35:12Z の on。
- 記録の上では、終わりの時点で `ふつう` と `閉じるだけ` の両方が on のまま残っている。画面で目がどう見えていたかは確かめていない。

#### 1 回目から分かったこと

- 両版とも、面 1002 で取り直す必要があった。release 版は外からの操作も入ったので、どちらにしても取り直す。
- 起動から 14 秒ほどのクリックは、開発者が押したものだった（開発者の確かめ・2026-10-30）。2 回目の前に、開発者はゴーストに触らないこと、「ぱすた開発」のセッションは SSTP・MCP・クリックを送らないことを約束した。

### 2 回目（2026-10-30・基準）

- 手順・コード・置き場・机は 1 回目と同じ。道具も同じで、根と記憶は回すたびに作り直した。机は進行役が test と sole の両方を持った。
- 順: debug 版（13:03:08〜13:09:09Z・pid 18504）、続けて release 版（13:09:33〜13:15:33Z・pid 32028）。
- 走行の前の check-quiet は、両版とも `QUIET`。機械ぜんたいの平均は debug 版の前 3.7%（最大 9.1%）、release 版の前 3.2%（最大 7.0%）。

#### 結果

| 版 | 測りの窓（Z） | areka の CPU（中央値／最大） | WorkingSet（窓の中の 60 行） | 答えなくなった回数 | panic・ERROR | 終了コード | 機械ぜんたいの CPU（中央値／最大） | 窓の中の面 |
|---|---|---|---|---|---|---|---|---|
| debug | 13:04:09〜13:09:04 | **47.4%** ／ 61.9% | 193 MB のまま（最小も最大も 193 MB） | 0 | 0・0 | 0 | 5.9% ／ 12.1% | 面 1002 だけ |
| release | 13:10:33〜13:15:29 | **13.1%** ／ 26.5% | 176 MB のまま（最小も最大も 176 MB） | 0 | 0・0 | 0 | 4.0% ／ 9.7% | 面 1002 だけ |

- 記録の行数は、debug 版 5,338 行（WARN 69）、release 版 5,326 行（WARN 69）。
- 5 秒ごとの値（WorkingSet の並びを含む）は、`logs\baseline-<版>.health.csv` にある。

#### 要件 6.4（面 1002 が出ていたか）の確かめ: 両版とも満たす

- 面 0 の INFO の表示の行の面の替わりは次のとおり。どちらも、面 1002 になった後は、記録の終わりまで別の面の INFO の表示の行が無い。
  - debug 版: 面 1000（13:03:09.988Z）、面 1001（13:03:10.028Z）、面 1002（13:03:14.982Z）。
  - release 版: 面 1001（13:09:34.263Z）、面 1002（13:09:39.259Z）。
- 測りの窓の前の最後の INFO の表示の行は、両版とも `surface_id=1002`。窓の中の INFO の表示の行は、すべて面 1002（debug 版 4,296 行・release 版 4,290 行）。

#### 外からの入力が無いことの確かめ: 両版とも無い

- 記録の全体（起動からの最初の 60 秒も含む）で、次の行がどれも 0 件だった: `OnMouse`、`SSTP`、`mouse_event_sent`、`areka::input_events`、`event="steady_talk"`（起動の後に台本が流れた行。起動の台本の終わりの `steady_talk_done` は除く）。
- `areka_mcp` の行は、待受を始めた行と閉じた行の 2 行だけ。

#### 目の重ね（seriko）の様子（記録から読めることだけ）

- 両版とも、seriko の行は 74 行。`loop 抽選発火`・`loop 停止` は 0 行、`bind 適用` は 7 行。
- `まばたき` の bind の適用は 0 行。走行のあいだに、まばたきの on／off は一度も替わっていない。

#### 判断: この 2 本を基準にする

- 2 回目の 2 本は、要件 6.4 を満たし、外からの入力も無い。タスク 5.4 は、この値と比べる: **debug 版 47.4%・release 版 13.1%**（1 コアに対する割合の中央値）。
- 件 25 の基準（debug 版 69%・release 版 20%）よりかなり低い。件 25 との違いとして記録から言えるのは、次の 2 つだけ。
  - 機械ぜんたいの負荷: 件 25 は中央値 15〜16%、今回は 4〜6%。
  - 面の出し方: 件 25 では面 1001 の行が 71〜74 行あった。今回は、面 1002 になった後に別の面の行が無い。
  - どちらが効いたかは確かめていない。

## タスク 4.2: 例・手順書・互換の設計書の注記の食い違い（2026-10-30）

要件 2.6・8.6 の確かめ。文言 `apply(ShowSurface)`・「表示成立点 info」・`show.rs:` を、完了した spec（`.kiro/specs/completed`）と `target` を除いて grep した。表示の行は `crates/areka-emo-present/src/presenter/show.rs:480`（`apply(ShowSurface): 表示・マスクを更新`）。最初の表示と状態の替わりは INFO、コマ送りだけは DEBUG。

### 直したもの（直す前の食い違い 4 件）

- `doc/COMPAT_ARCHITECTURE.md` 178 行目の行の位置の参照 `show.rs:194`/`:221`（`target.rs:31`・`visibility.rs:69` も同じ表の項）を、今の行（可視化のゲート `show.rs:341`・可視化 `:402`、`target.rs:32`、`visibility.rs:81`）へ直した。
- `.kiro/steering/roadmap.md` 522 行目の `presenter/show.rs:402` を `:480` へ直した。
- `doc/pasta-real-run.md` の「`apply(ShowSurface)` を外して読む」注記（314 行目）を、着地後の姿（コマ送りだけの適用は既定の水準で出ない・外さずに読む・すべて見たいときは `areka_emo_present=debug` を足す）へ書き直した。`RUST_LOG` の値（127・141 行目）は変えていない。
- `doc/pasta-real-run.md` の表（183 行目）の本 spec の項を、着地後の確かめ（同じ足し方の注記つき）へ直した。

### 直しが要らないと確かめたもの

- 例 `crates/areka/examples/`（`collision-probe.rs`・`emo-present.rs`・`emo-present/state.rs`・`emo-present/systems.rs`・`window-placement.rs`）の `apply(ShowSurface)` の注記は、どれも「最初の表示を駆動する」「ログの欄（`k_ratio` など）を見る」だけを言い、水準を書いていない。例が出す行は最初の表示（INFO のまま）なので、直しは要らない。例の `RUST_LOG` の既定は `info` のままで、読める。
- `crates/areka-emo-present/src/presenter_perf_log_tests.rs` の「表示成立点 info!」は、最初の表示の行（INFO のまま）を固定するテストの文言で、今も正しい。`presenter_refresh_and_log_tests.rs` の「表示成立点で info レベル」も最初の表示について言っていて正しい。
- 性能の道具（`tools/perf/` の判定・順位付け・見本）は文言で行を拾い水準を見ない。注記はタスク 4.1 で合わせ済み。`doc/ukadoc-coverage/ledger/sakura-script.toml` の言及は error! の別の行（未装着ターゲット）の話で、表示の行ではない。
- `.kiro/steering/roadmap-history.md` 2568 行目の `show.rs:462` → `:402` は、棚卸㉙の時点の直しの記録（履歴）。過去の事実の記録なので書き換えない。

### 本 spec 自身の文書の扱い

- 本 spec の `brief.md`・`requirements.md`・`design.md`・`research.md`・`design-validation.md` などは、直す前の姿（`show.rs:402`・「毎適用 INFO」）を問題として書いた履歴の文書なので、食い違いの数に入れず、書き換えていない。
- `verification.md` の既存の節も書き換えていない。

### 結果

- 直した後の食い違い: **0 件**（行の位置の参照は `show.rs:480` を指す表示の行の位置と合い、水準の主張はどれも最初の表示＝INFO・コマ送りだけ＝DEBUG と合う）。
- 再確認の grep: `show.rs:` の参照は `COMPAT_ARCHITECTURE.md`（今の行）・`roadmap.md`（`:480`）・`roadmap-history.md`（履歴）の 3 か所だけ。

## 件 26: 止まったまばたきの重ね絵が残る件（タスク 3.7・要件 9）

### 直す前の失敗するテスト（要件 9.3）

見本はぱすたの animation1400 と同じ形（`bind+random`・pattern0 が閉じた目 1410・pattern1 が半目 1411・pattern2 が `-1`）。

- seriko の側: `crates/areka-seriko/src/looper_tests.rs` の `blink_ending_in_minus_one_leaves_no_frame_in_the_command_after_stop`。直す前から**緑**。`-1` の停止は、1400 の欄を指令から外していた（seriko は正しい）。
- 表示の側: `crates/areka-emo-present/src/presenter_blink_stop_tests.rs` の `stopped_blink_leaves_no_closed_eye_in_the_picture`。直す前は**赤**。欄の無い指令で成立した絵が、まばたきの重ねの無い絵と違い、「0 周目: `-1` で外した後の絵に閉じた目が残る」で落ちた（127 行目の判定）。
- 合成の側（原因の所の兄弟）: `crates/areka-emo-compose/src/plan_bind_random_tests.rs`。直す前の `cargo test -p areka-emo-compose --lib bind_random_tests` は 1 本が**赤**で、もう 1 本は緑。
  - `bind_random_without_frame_stacks_no_pattern0`: 欄の無いとき、命令列が `["eyebase.png", "closed.png"]` になった（期待は `["eyebase.png"]`）。閉じた目の pattern0 が静的に積まれていた。
  - `bind_random_with_frame_draws_the_frame`: 再生中はそのコマ（1410・1411）を描く。直す前から緑（直しで変わらないことの檻）。
  - どちらのテストも、純 `bind` の 1300（目の土台）が bind に入っていれば pattern0 を積むことを同じ導出で確かめる（空同士の一致でない）。

### 原因の場所

- emo-compose の `crates/areka-emo-compose/src/plan.rs` の `flatten_surface`。`bind+random` も bind 種として「有効 bind」の id に入り（`is_bind_interval`）、その id にコマが無いときは pattern0 を静的に積んでいた。ぱすたの animation1400 と 1401 は pattern0 が閉じた目（1410）なので、seriko が `-1` で欄を外した後も閉じた目が残った。
- 本 spec の境界の外だったので止まって報告し、開発者の決め（2026-10-31・案 1）で `plan.rs` の 1 か所だけを本 spec で直すことにした。

### 直した 1 か所

- `plan.rs` の `flatten_surface` の、コマの無い id の pattern0 を静的に積む道（pattern0 を引いた直後）に、`Interval::BindRandom` なら積まずに次へ進む条件を 1 つ足した。純 `bind`・`is_bind_interval`・`is_bind_animation`・`nesting.rs` 130 行目・合流の id の集め方・外形の求め方は変えていない。
- ほかに足したのは、`derive_ops` の説明の 1 文と、テストの接続宣言 1 つ（`plan.rs` の末尾）だけ。

### 直した後のテスト

- `cargo test -p areka-emo-compose`: 401 本すべて緑（新しい 2 本を含む）。golden のテスト（`golden_tests_*`）は 1 本も変わらず緑。emo2 のまばたき（1400 は pattern0 を持たない `bind+random`・1403 は `bind`）は、もともと pattern0 を静的に積まないので、命令列が変わらない。
- `cargo test -p areka-emo-present`: 357 本すべて緑（`stopped_blink_leaves_no_closed_eye_in_the_picture` と `stopped_blink_beside_a_moving_picture_leaves_no_closed_eye_on_the_rect_path` を含む）。
- `cargo test -p areka-seriko`: 本体 455 本ほか、すべて緑（`blink_ending_in_minus_one_leaves_no_frame_in_the_command_after_stop` と `blink_stop_beside_a_moving_picture_drops_only_the_blink_slot` を含む）。

### 動く絵と並ぶまばたき（矩形の重ね直しの道・レビュー 1 回目の指摘）

欄の空の指令はいちからの合成（`compose_into`）を通るので、上の表示の側の 1 本だけでは矩形の重ね直し（`compose_into_reusing`・タスク 3.3）を通らない。同じ面に純 `random` の動く絵（1500）を足し、その欄が残る止まり方を 2 本で確かめた。

- seriko の側: `blink_stop_beside_a_moving_picture_drops_only_the_blink_slot`。1500 が 1500 → 1501 とコマを替えるあいだに、まばたきが 閉じる → 半目 → 外す と進み、止まった後の指令は 1400 の欄だけを欠いて 1500 の欄（1501）が残る。seriko はもともと正しいので、直す前から緑の檻。
- 表示の側: `stopped_blink_beside_a_moving_picture_leaves_no_closed_eye_on_the_rect_path`。止まった後の指令は 1500 の欄だけを運び、半目の絵を下敷きに矩形で重ね直される。その絵が「1500 のそのコマだけ・まばたきの重ね無し」の絵（`native_golden_with`）と同じことを、1 周目（控えに外れる）と 2 周目（控えに当たる）で判定する。空の道で緑にしないため、止まった絵の適用で重ね直しの範囲の行が 1 周目は `partial` 1 件、2 周目は 0 件（控えに当たる）であることも数える。見本の前提として、止まった絵に 1500 のコマの画素が載ることも確かめる。
- 赤の確かめ: `plan.rs` の条件を一時的に `if false && matches!(...)` にして `cargo test -p areka-emo-present --lib blink_stop` を回すと、2 本とも「0 周目: `-1` で外した後の絵に閉じた目が残る」で**赤**（矩形の道の 1 本も落ちる）。元のファイルへ戻し、控えと `cmp` で 1 バイトも違わないことを確かめてから緑に戻ることを見た。

### 持ち越し

- `nesting.rs` の `bind_targets`／`visible_parts` は、`bind+random` の pattern0 の先をまだ見える部品として数える（開発者へ上げる・本タスクでは直さない）。
- `cargo fmt --all -- --check`: 差なし。`cargo clippy -p areka-emo-compose --lib -- -D warnings`・`cargo clippy -p areka-emo-present --no-deps --lib -- -D warnings`: 指摘なし。

### 実機の確かめ（未）
