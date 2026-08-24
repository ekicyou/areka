# 着手前インベントリの再計測手順（要件 2.1・4.1）

本書は `areka-P0-test-cage-determinism` の着手時に requirements.md の Introduction インベントリを
現在値へ更新するために使った計測コマンドを、そのまま再実行できる形で残したものである。
**同じコミットで同じコマンドを実行すれば同じ数値が出る**ことを担保するのが本書の役割で、
数値の解釈（何を是正するか）は requirements.md 側に置く。

## 0. 実行前提

| 項目 | 値 |
|---|---|
| 計測日 | 2026-08-23 |
| ブランチ | `claude/areka-p0-test-cage-determinism-dad056` |
| HEAD | `b9f7936e` |
| `origin/main` | `327e7fd3`（**HEAD の先祖**＝取り込み漏れ無し。`main` 取り込みコミットは `76384c83`） |
| 前回計測 | 2026-08-22・`f6b81078`（requirements.md の旧数値の出所） |
| シェル | Git Bash（`rg` 14.1.1・`comm`・`awk`・`find`・`wc`）。リポジトリ**ルート**で実行する |
| 走査範囲 | `crates/` 配下の `*.rs` のみ。`target/`・`vendors/` は `rg` の既定除外と `find` の `-not -path` で外す |
| **区切り文字の注意** | Windows では `rg -l` が **`crates\areka\…`（円記号区切り）**、`git grep -l` が **`crates/areka/…`（斜線区切り）** を出す。両者を `diff`／`comm` で突き合わせるコマンドは、必ず `rg` 側を `tr '\134' '/'` で正規化すること。怠ると全件が差分として並び、**変わっていないものが変わったと読める**（§1-b がこの形）。数を数えるだけのコマンドには影響しない。なお `--path-separator /` は MSYS が裸の `/` をパス展開してしまい `rg` がエラーになるので使えない |

着手条件（改善ループの成果が `main` へマージ済みで、さらに新しい `main` が無いこと）の確認:

```bash
git fetch origin main
git rev-parse origin/main                              # → 327e7fd3b66ae7429d0ed19899ea5a611e5b47a6
git merge-base --is-ancestor origin/main HEAD && echo ANCESTOR   # → ANCESTOR
```

以降、`WD` / `MK` は次の 2 つのファイル集合を指す（コマンド中ではプロセス置換で毎回作り直す）。

```bash
# WD: 捕捉 subscriber を自分で差しているファイル
rg -l 'with_default\(' crates --glob '*.rs' | sort
# MK: 硬化の印（interest キャッシュの作り直し・常駐 probe・常駐 keeper）を持つファイル
rg -l 'rebuild_interest_cache|ensure_interest_probes|install_interest_keeper' crates --glob '*.rs' | sort
```

---

## 1. 捕捉 subscriber を差すファイルと硬化の印の有無（要件 2.1・2.2）

### 1-a. 総数

```bash
rg -l 'with_default\(' crates --glob '*.rs' | wc -l                      # → 40
rg --no-filename -o 'with_default\(' crates --glob '*.rs' | wc -l        # → 62
```

| 値 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|
| `with_default(` を持つファイル | **40** | 40 | 増減なし |
| `with_default(` の呼出総数 | **62** | 62 | 増減なし |

### 1-b. 硬化済み／未硬化の切り分け

判定規則は機械的に 1 点＝「`with_default(` を持ち、**同一ファイル内に**硬化の印
（`rebuild_interest_cache` / `ensure_interest_probes` / `install_interest_keeper` のいずれか）が
現れるか」。

```bash
# 硬化済み
comm -12 <(rg -l 'with_default\(' crates --glob '*.rs' | sort) \
         <(rg -l 'rebuild_interest_cache|ensure_interest_probes|install_interest_keeper' crates --glob '*.rs' | sort) | wc -l   # → 16
# 未硬化
comm -23 <(rg -l 'with_default\(' crates --glob '*.rs' | sort) \
         <(rg -l 'rebuild_interest_cache|ensure_interest_probes|install_interest_keeper' crates --glob '*.rs' | sort) | wc -l   # → 24
# 印を持つファイル総数（`with_default(` を持たない消費側 2 ファイルを含む）
rg -l 'rebuild_interest_cache|ensure_interest_probes|install_interest_keeper' crates --glob '*.rs' | wc -l                      # → 18
```

| 値 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|
| 硬化済みファイル | **16** | 16 | 増減なし |
| 未硬化ファイル | **24** | 24 | 増減なし |
| 印を持つファイル総数 | **18** | 18 | 増減なし |

未硬化 24 ファイルの**集合そのもの**が前回と一致することも確認済み。下記は**何も出力せず終了コード 0**
を返す（`rg` 側だけ `tr '\134' '/'` を通すのが要点＝§0 の区切り文字の注意。これを省くと 24 件すべてが
差分として並び、集合は変わっていないのに「変わった」と読める偽の食い違いになる）:

```bash
diff <(comm -23 <(rg -l 'with_default\(' crates --glob '*.rs' | tr '\134' '/' | sort) \
                <(rg -l 'rebuild_interest_cache|ensure_interest_probes|install_interest_keeper' crates --glob '*.rs' | tr '\134' '/' | sort)) \
     <(comm -23 <(git grep -l 'with_default(' f6b81078 -- 'crates/**/*.rs' | sed 's/^f6b81078://' | sort) \
                <(git grep -l -e 'rebuild_interest_cache' -e 'ensure_interest_probes' -e 'install_interest_keeper' f6b81078 -- 'crates/**/*.rs' | sed 's/^f6b81078://' | sort))
echo $?   # → 0（出力は空）
```

較正（この検査が素通りでないことの確認）: 左辺を 1 件減らす `| tail -n +2` を挟むと
`0a1` と `> crates/areka-emo-present/src/presenter/timing_tests.rs` が出て、終了コードが 1 になる。

### 1-c. 硬化済み側の呼出数（**2026-08-22 の記載誤りを是正**）

```bash
comm -12 <(rg -l 'with_default\(' crates --glob '*.rs' | sort) \
         <(rg -l 'rebuild_interest_cache|ensure_interest_probes|install_interest_keeper' crates --glob '*.rs' | sort) \
| while IFS= read -r f; do rg -o 'with_default\(' "$f"; done | wc -l   # → 16
```

| 値 | 2026-08-23 | 2026-08-22 の記載 | 判定 |
|---|---|---|---|
| 硬化済み 16 ファイルの `with_default(` 呼出 | **16**（1 ファイル 1 呼出） | 28 | **記載誤り**（下記） |

`f6b81078` に対して同じ計測をしても **16** で、前回計測時点でも 16 だった。したがって
requirements.md の「28 呼出」は 2026-08-22 の**記録上の誤り**であり、期間中の増減ではない。
`with_default`（括弧無し＝説明文中の言及を含む）で数えても 55、全ファイルでも 135 なので、
28 を再現する数え方は見つからない。今回の更新で **16** に是正した。

検算: 62（総数）− 16（硬化済み）= 46 = 17（未硬化のヘルパ定義 1 呼出 × 17 ファイル）＋ 29（直書き）。

### 1-d. 未硬化 24 の内訳（ヘルパ定義側／直書き側）

未硬化ファイルは「ヘルパ定義が 1 本だけ＝呼出 1」と「直書き＝呼出 2 以上」で機械的に分かれる。

```bash
comm -23 <(rg -l 'with_default\(' crates --glob '*.rs' | sort) \
         <(rg -l 'rebuild_interest_cache|ensure_interest_probes|install_interest_keeper' crates --glob '*.rs' | sort) \
| while IFS= read -r f; do c=$(rg -c 'with_default\(' "$f"); if [ "$c" -ge 2 ]; then echo "$c $f"; fi; done
```

出力（2026-08-23）:

```
3 crates\areka-emo-present\src\presenter\timing_tests.rs
5 crates\areka-emo-present\src\presenter\transition_record_tests.rs
6 crates\areka-emo-present\src\presenter_perf_log_tests.rs
7 crates\areka-emo-present\src\presenter_refresh_and_log_tests.rs
2 crates\areka-emo-text\src\layout_cursor_tests.rs
3 crates\areka-emo-text\src\state_cue_apply_tests.rs
3 crates\areka\src\shiori_demo.rs
```

| 値 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|
| 直書きファイル | **7** | 7 | 増減なし |
| 直書きの呼出 | **29** | 29 | 増減なし |
| ヘルパ定義側（名前付き 10＋別名 7） | **17** | 17 | 増減なし |

### 1-e. 硬化方式の内訳（要件 1.5 の削除対象）

```bash
rg -l 'fn ensure_interest_probes' crates --glob '*.rs' | wc -l      # → 8
rg -l 'fn install_interest_keeper' crates --glob '*.rs' | wc -l     # → 3
rg -l 'set_global_default' crates --glob '**/tests/**/*.rs'         # → 2 ファイル
```

| 値 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|
| 常駐 probe 方式の複製（定義） | **8** | 8 | 増減なし |
| 常駐 keeper 方式（定義＝3 crate） | **3** | 3 | 増減なし |
| 統合テストの一回限り全スレッド捕捉 | **2**（`areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs`・`areka-seriko/tests/loop_integration.rs`） | 2 | 増減なし |

### 1-f. 誤った説明文の候補母集団（要件 2.4）

最終判定は文意の読み取りが要るため機械では確定しない。母集団の絞り込みだけを機械化する。

```bash
# 捕捉サイト 40 ファイルのうち「スレッドローカル」の語を含むもの
rg -l 'with_default\(' crates --glob '*.rs' \
| while IFS= read -r f; do rg -q 'スレッドローカル' "$f" && echo "$f"; done | wc -l   # → 26（未硬化 13＋硬化済み 13）
# 誤った主張に近い狭い言い回し（2 行以内に「干渉しない」「並行テスト安全」等）
rg -U -l --glob '*.rs' 'スレッドローカル[^\n]*\n?[^\n]*(干渉しない|並行テスト.{0,3}安全|並行実行でも.{0,6}安全)' crates | wc -l   # → 11
```

狭いパターンは `choice_drain.rs` / `balloon_test_support.rs` / `frame_test_support.rs` の
「最小複製」系の言い回しを取りこぼすので、**是正時は 26 ファイルの側を母集団として読む**こと。

---

## 2. 派生ヘルパの定義位置（要件 2.3）

```bash
rg -n --glob '*.rs' 'fn (capture_logs|capture_logs_flow|capture_under_filter|with_log_cage|count_warns|resolve_counting_warns|capture)\s*[(<]' crates | sort
```

2026-08-23 の出力は **30 定義／29 ファイル**（`areka-seriko/src/actor_test_support.rs` のみ 2 本）。
requirements.md が挙げる 17 の定義位置（名前付き 10・別名 7）と行番号を突き合わせた結果、
**`crates/areka/src/emo2_boot/adapter.rs` だけが `:383` → `:388`（+5 行）へずれていた**。
他 16 件の行番号は一致（`spine.rs:525`・`frame_test_support.rs:122`・`frame_chain_finalize_tests.rs:241`・
`move_cue_move_severity_log_tests.rs:43`・`talk_lifecycle_tests.rs:97`・`balloon_test_support.rs:140`・
`choice_drain.rs:182`・`table.rs:209`・`dpi_helpers_tests.rs:345`・`draw_test_support.rs:61`・
`actor_runtime_frame_tests.rs:53`・`areka-emo-text/src/sink.rs:170`・`region.rs:400`・`wrap.rs:114`・
`writing.rs:128`・`areka-ghost/src/sink.rs:224`）。同じずれは説明文の参照 `adapter.rs:358-359` にも
かかり、現在は `:363-364`。**同じ `adapter.rs:363-395`／`:358-359` の参照は design.md:155（`## File Structure Plan` の `### Modified Files`）にも載っている**ので、①の移行タスクに入る際はそちらも +5 行ずれている前提で読むこと（design.md の改訂は本タスクの範囲外）。

呼出規模:

```bash
rg --no-filename -o 'capture_logs\(' crates --glob '*.rs' | wc -l          # → 238
rg -l 'capture_logs\(' crates --glob '*.rs' | wc -l                        # → 62
rg --no-filename -o 'capture_logs_flow\(' crates --glob '*.rs' | wc -l     # → 18
rg --no-filename -o 'capture_under_filter\(' crates --glob '*.rs' | wc -l  # → 96
rg --no-filename -o 'capture_events\(' crates --glob '*.rs' | wc -l        # → 7
```

| 値 | 2026-08-23 | 2026-08-22 の記載 | 判定 |
|---|---|---|---|
| `capture_logs(` 呼出 | **238** | 238 | 増減なし |
| `capture_logs(` を含むファイル | **62** | 64 | **記載誤り**（`f6b81078` でも 62。今回 62 へ是正） |
| `capture_logs_flow(` 呼出 | **18** | 18 | 増減なし |
| `capture_under_filter(` 呼出 | **96** | 96 | 増減なし |

---

## 3. 反復回数固定の待機（要件 4.1）

```bash
rg -n --glob 'spine*.rs' '^\s*for\s+(_|now)\s+in\s' crates/areka/src/emo2_boot/
rg -n 'SPIN_WAIT|fn spin_wait_until' crates/areka/src/emo2_boot/spine.rs
```

1 本目の出力（2026-08-23）:

```
crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs:369:    for now in [1000u64, 2000, 3000, 4000, 5000] {
crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs:372:        for _ in 0..5_000 {
crates/areka/src/emo2_boot/spine_display_tests.rs:410:    for now in 1_000_000u64..1_000_000 + 5_000 {
```

3 件のうち `:369` は注入時刻の階段（打ち切り条件ではない）で、**是正対象の待機は `:372` と `:410` の 2 件**。

| 値 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|
| 反復回数固定の待機 | **2**（`spine_seriko_loop_tests.rs:372-375`・`spine_display_tests.rs:410-414`） | 2（同じ行） | 増減なし |
| `spine.rs` 本体の壁時計上限なしループ | **0**（`spin_wait_until` :358 は `SPIN_WAIT`=30 秒 :329 で有界） | 0（同じ行） | 増減なし |

---

## 4. 表示更新の失敗点（要件 5.1）

```bash
rg -n 'fn upload|struct SwapChainPresenter' crates/areka-emo-present/src/chain.rs
awk 'NR>=185 && NR<=241 && /\?;/ {print NR": "$0}' crates/areka-emo-present/src/chain.rs
awk 'NR>=185 && NR<=241 && /\?;/' crates/areka-emo-present/src/chain.rs | wc -l   # → 7
```

失敗を返し得る行（2026-08-23）: `200`(ResizeBuffers) / `203`(source_tex 再作成) / `204`(staging 再作成) /
`211`(source_tex→Resource) / `228`(GetBuffer(0)) / `231`(backbuffer→Resource) / `238`(Present(0))。

| 値 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|
| `upload` の失敗点 | **7**（`chain.rs:185-241`） | 7（同じ範囲） | 増減なし |
| `SwapChainPresenter` の定義行 | **`chain.rs:122`** | :122 | 増減なし |
| 保持側 | **`presenter/target.rs:73`**（`Option<SwapChainPresenter>`） | :73 | 増減なし |
| 観測点の早期 return | **`presenter/show.rs:306-310`** | :306-310 | 増減なし |

---

## 5. テスト直列化の錠の呼出（要件 7.2）

```bash
rg -c 'lock_self_initiated_for_test\(' crates --glob '*.rs' | sort
rg --no-filename -o 'lock_self_initiated_for_test\(' crates --glob '*.rs' | wc -l   # → 22（定義 1 行を含む）
rg -n 'fn lock_self_initiated_for_test' crates --glob '*.rs'                        # → command.rs:104
```

出力（2026-08-23）:

```
crates\wintf\src\ecs\window\command.rs:3                       （定義 :104 ＋ 実呼出 :961 / :973）
crates\wintf\src\ecs\window\command_batch_tests.rs:5
crates\wintf\src\ecs\window\command_transition_tests.rs:4
crates\wintf\src\ecs\window_proc\window_pos_tests.rs:5
crates\wintf\src\ecs\window_proc\window_pos_transition_tests.rs:5
```

| 値 | 2026-08-24 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|---|
| 実呼出（22 − 定義 1） | **21** | 21 | 21 | 増減なし |
| ファイル数 | **5** | 5 | 5 | 増減なし |
| 定義位置 | **`command.rs:104`** | :104 | :104（`:76` から移動済み） | 増減なし |
| カウンタの形 | **`thread_local!` の `Cell<i32>`（`command.rs:49`・`:70`）** | 同じ | 同じ（`76384c83` で取り込み済み） | 増減なし |

要件 7 は分岐⒝（`draw-load-parity` 着地済み）で確定＝7.2 が実施対象、という前回の判断が
現在値でも成立する。

### 5-a. タスク 7.1 の判定用の再計測（2026-08-24・HEAD `79527213`）

`rg -c` は doc コメント中の参照も数える（`command_threadlocal_tests.rs:19` が「錠を意図的に
取らない」と述べている 1 件）。**実呼出だけ**を数えるときは実行行の形で当てる:

```bash
# 実呼出のみ（21）。ファイル別の行番号も出る
rg -n 'let _serialized = .*lock_self_initiated_for_test\(\)' crates --glob '*.rs'
rg -n 'let _serialized = .*lock_self_initiated_for_test\(\)' crates --glob '*.rs' | wc -l  # → 21

# 陳腐化した説明文（要件 2.4 の対象・4 件）。母集団は「プロセス共有」の語を含む行
rg -n 'プロセス共有' crates --glob '*.rs'
```

出力（2026-08-24）:

| ファイル | 実呼出 | 行 |
|---|---|---|
| `crates/wintf/src/ecs/window/command.rs` | 2 | :961, :973 |
| `crates/wintf/src/ecs/window/command_batch_tests.rs` | 5 | :322, :402, :466, :542, :637 |
| `crates/wintf/src/ecs/window/command_transition_tests.rs` | 4 | :302, :372, :408, :426 |
| `crates/wintf/src/ecs/window_proc/window_pos_tests.rs` | 5 | :44, :284, :318, :622, :651 |
| `crates/wintf/src/ecs/window_proc/window_pos_transition_tests.rs` | 5 | :192, :222, :299, :399, :519 |
| **合計** | **21**（兄弟テスト 4 本＝19） | |

「プロセス共有」の語は `crates/**/*.rs` に **30 行 / 21 ファイル**あるが、その大半は起床旗・
スレッド名簿・空 `PatternState` 等の**別の**共有物を正しく説明している。自発書込カウンタを
指していて**かつ現在形で誤っている**のは次の 4 件だけ（7.2 の是正対象）:

```
crates/wintf/src/ecs/window/command_batch_tests.rs:25
crates/wintf/src/ecs/window/command_transition_tests.rs:28
crates/wintf/src/ecs/window_proc/window_pos_tests.rs:40
crates/wintf/src/ecs/window_proc/window_pos_transition_tests.rs:21
```

同じカウンタを指していても `command.rs:77`・`command_threadlocal_tests.rs:35`・
`areka/src/emo2_boot/frame_harness_tests.rs:10` は「**だった頃**／**是正前**」の**過去形**で
書かれており誤りではない（`command.rs` は非接触の裁定下でもある）。**語で数えて 30 件を
「是正対象」と読むと 26 件を余計に壊す**ので、必ず 1 件ずつ現在形かどうかを見ること。

判定結果とその根拠は requirements.md の「申し送り台帳 ⑴」に登記した。

---

## 6. 1,000 行超のファイル（要件 10.2）

```bash
find crates -name '*.rs' -not -path '*/target/*' -not -path '*/vendors/*' -exec wc -l {} + \
| grep -v ' total$' | awk '$1 > 1000' | sort -rn
find crates -name '*.rs' -not -path '*/target/*' -not -path '*/vendors/*' -exec wc -l {} + \
| grep -v ' total$' | awk '$1 > 1000' | wc -l    # → 11
```

出力（2026-08-23・行数降順）:

```
1618 crates/areka-emo-present/src/cache_tests.rs
1374 crates/areka-emo-compose/src/plan_ops_tests.rs
1336 crates/areka-seriko/src/actor_bind_loop_tests.rs
1255 crates/areka/src/emo2_boot/frame_transition_branch_tests.rs
1227 crates/areka/src/placement/follow/window_move.rs
1129 crates/areka-ghost/tests/ghost/inproc_e2e_test.rs
1081 crates/areka-emo-present/src/presenter/budget_tests.rs
1043 crates/areka-seriko/src/bind.rs
1039 crates/areka/src/placement/transition_judge_tests.rs
1037 crates/areka/src/placement/transition_judge_verdict_tests.rs
1006 crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs
```

| 値 | 2026-08-23 | 2026-08-22 | 差 |
|---|---|---|---|
| 1,000 行超のファイル | **11** | 11 | 増減なし（**行数もファイル名も `f6b81078` と完全一致**） |

この 11 件がそのまま要件 10.2 の例外表の初期値になる。

---

## 7. まとめ

改善ループ（`draw-load-parity`・PR#118）の `main` マージを取り込んだ後も、
**本仕様が数えている 6 系統の数値は 1 つも動いていない**。`draw-load-parity` は
`command.rs` のカウンタをスレッド局所へ移しただけで、ログ捕捉の仕組み・待機・
表示更新・ファイル行数のいずれにも触れていない（着手時の突合として想定していた
「二重作業」は発生しなかった）。

一方で、前回の記録には**再現できない数値が 2 件**あった（増減ではなく記録の誤り）:

| 項目 | 旧記載 | 正しい値 | 根拠 |
|---|---|---|---|
| 硬化済み側の `with_default` 呼出 | 28 | **16** | §1-c。`f6b81078` でも 16 |
| `capture_logs(` を含むファイル | 64 | **62** | §2。`f6b81078` でも 62 |

さらに `adapter.rs` のヘルパ定義行が `:383` → `:388`（説明文は `:358-359` → `:363-364`）へ
ずれていた（§2）。requirements.md 側はいずれも現在値へ更新済み。
