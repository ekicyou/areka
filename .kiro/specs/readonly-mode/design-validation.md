# 設計の検証: readonly-mode

> 2026-10-23・`kiro-validate-design` の手順（Analysis → Critical Issues → Strengths → GO/NO-GO）を対話なしで実施。
> 入力: `spec.json`（language=ja）・`requirements.md`・`design.md`・`research.md`・`.kiro/steering/`（product・tech・structure・logging・focus）。
> 設計が言い切っているコードの事実は、本ブランチのコードを Grep／Read で確かめた（下の「確かめた事実」）。

## 設計の検証のまとめ

設計は「保存の口 `FsPersistIo` の中で控える」「World の中の入口は資源 `ReadOnlyRun`」「起動の前の守りはプロセスの値を引数で受ける純粋な関数」の 3 つの継ぎ目に分けており、今の層と依存の向きを崩していない。言い切っているコードの事実（作り手 7 か所・`PersistOutcome` を網羅で読むのは `boot_resolve.rs` の 3 か所だけ・入口の関数・字の檻）はどれも今のコードと合っていた。直すべき点は 3 つあるが、どれも設計の形を変えずに直せる小さなものなので、実装へ進めてよい。

## 確かめた事実（設計の言い切りとコード）

| 設計の言い切り | 確かめた結果 |
|---|---|
| 本番の `FsPersistIo` の作り手は 7 か所・5 ファイル | 合う（`shiori_host.rs:206`・`roots/mod.rs:28`・`roots/edit.rs:282`・`boot_resolve.rs:353/432`・`placement/persist.rs:307`・`areka-ghost/src/sylphya_wiring.rs:116`） |
| `PersistOutcome` を網羅の `match` で読むのは `boot_resolve.rs` の 3 か所だけ | 合う（`roots/edit.rs:281`・`actor.rs:671` は `if let Degraded` だけ。返事の `PersistOutcome` を受けるのは試験だけ） |
| 記憶のファイル `sylphya.toml` を読み書きする道は保存の口だけ | 合う（`PERSIST_FILE_NAME` の使い手は `persist/mod.rs` だけ・`crates/areka/src` の本番の直の書き込みは `install/fetch_url.rs`〔一時フォルダ〕だけ） |
| `install::submit` が積む前に判定・`update::submit` → `judge` が段を移す前に判定 | 合う（`install/mod.rs:68`・`update/mod.rs:128/150`。投げ込み・窓口の取り出し・メニューはどれも `submit` を通る） |
| `InstallCueSink::new` の引数を変えなければ字の檻に触らない | 合う（`zorder_wiring_tests.rs` は `runtime_boot.rs` の `InstallCueSink::new(crate::install::desk::raw_sender(world))` の字を固めている） |
| 更新の総括の送り物は `refs.rs` で組む・送らない 7 語の檻がある | 合う（`summary_refs`・`UNSENT_WORDS: [&str; 7]`。ふだんは `Current`→`OnUpdateResult`・`Other`→`OnUpdateResultEx` の片方だけを送る） |
| `keep_session_mark` は印を読み直して予告する | 合う（`boot_resolve.rs:557`） |
| `runtime.rs` の 629 行目が `std::fs::create_dir_all(ghost_root)` | 合う |
| `main.rs` は 978 行・`--offline-dump` の戻りの後に `resolve_boot` | 合う（足し 5 行前後で 1,000 行の内） |

## Critical Issues（3 件）

🔴 **Critical Issue 1**: 試験の根の「下か」の判定が文字列の頭の一致だと、並んで走る別の試験へ漏れる
**Concern**: 控える範囲の判定を「小文字・区切り `\` にそろえた文字列」で行うと書いてあるが、根の名前は `temp_path_kit` の `areka-{label}-{pid}-{serial}` で、同じ label の `…-1` は `…-10`〜`…-19`・`…-100` の文字列の頭と一致する。`SwitchRig` の検体のように同じ label を多くの試験が使うので、読み取りだけの試験が立てた `WithholdGuard` が、並んで走るふつうの試験（書けることや致命の机へ届くことを見る試験）の書き込みを黙って控え、たまにだけ赤になる。
**Impact**: 決定論の試験という要件 10 の前提と「並んだ試験への漏れなし」の約束が崩れる。揺れる赤は原因を追いにくい。
**Suggestion**: 範囲の判定はパスの部品の単位で行う（登録する根の鍵の末尾に `\` を足して頭の一致を見るか、そろえた `PathBuf` の `starts_with`）。`withhold_tests.rs` に「`…-1` を登録しても `…-10` は書く」試験を 1 つ足す。
**Traceability**: 10.1・10.2・10.5（決定論のテスト）・2.1
**Evidence**: design.md「PersistWithhold」の Responsibilities（鍵と範囲の判定）・State Management・Unit Tests 1

🔴 **Critical Issue 2**: 読み取りだけのアプリの記憶の置き場が、呼ぶたびにファイルを見て決め直され、記録も毎回出る
**Concern**: `readonly_app_profile_dir` は `migrate_source`（フォルダが在るかを見る）を通り、`app_memory_read_beside` を info に残す。これを `default_app_profile_dir()` から選ぶので、1 回の走りで少なくとも 3 回（`boot_config.rs:203` の起動の解決・`main.rs:216` の確かめ・`main.rs:258` の起動の文脈）呼ばれ、同じ info が繰り返し出る。今の `layout()` と `exe_location()` は「途中で置き場が変わらないように」1 度だけ決めて覚えている（`boot_config.rs:144` の説明）が、読み取りだけの置き場はその決まりの外になる。別の areka が同じ時にふつうに起きて写しを終えると（`CopyDone::Raced` を今のコードが扱っている＝並ぶ起動は想定の内）、走りの途中で置き場が隣から書ける根へ変わり、控えの鍵も別のパスに分かれる。
**Impact**: 要件 4.5 の記録が重なって読みにくい。要件 1.6（走りの途中で変えない）・4.3（写した後に読むのと同じ値で起こす）の約束が並ぶ起動のときに崩れうる。`areka-user-data` の `lib.rs` の関数は「純粋」と書いてあり、そこへ FS を見る関数が並ぶ。
**Suggestion**: 読み取りだけの置き場は `layout()` と同じく 1 度だけ決めて覚える（`establish_readonly` の答えに写す元を持たせるか、`boot_config` に `OnceLock` を 1 つ置く）。`app_memory_read_beside` はその 1 度だけ出す。`readonly_app_profile_dir` は判断（純粋）と FS を見る部分を分け、判断を試験する。
**Traceability**: 1.6・4.3・4.5・11.2
**Evidence**: design.md「UserRootReadonly」の Responsibilities（`readonly_app_profile_dir`・`default_app_profile_dir()` の選び分け）・Monitoring の表

🔴 **Critical Issue 3**: 保存の口の外に FS に触る関数を足し、sylphya の「FS に触るのは `FsPersistIo` だけ」の決まりと食い違う
**Concern**: `persist/io.rs` の説明は `FsPersistIo` を「areka-sylphya でファイルシステムに触れる唯一の型（R8.3）」と書いている。設計は `persist/withhold.rs` に自由な関数 `ensure_root_dir`（範囲の外で `std::fs::create_dir_all`）を足すので、この決まりが黙って崩れる（説明の文も古くなる）。
**Impact**: 完了 spec（`areka-P0-sylphya`）の決まりと今の説明が合わなくなり、後の spec が「FS に触る口は 1 つ」を前提に読み違える。直しは小さいが、残すと設計の言い切り（「`profile\areka` を作る持ち主が保存の口 1 つになる」）とも合わない。
**Suggestion**: `ensure_root_dir` を `FsPersistIo` の関連関数（`FsPersistIo::ensure_root_dir`）にして `persist` から公開するか、`io.rs` の説明を「FS に触るのは `io.rs` の `FsPersistIo` と `ensure_root_dir` だけ」に書き直す。どちらでも `runtime.rs` の差分は 1 行のまま。
**Traceability**: 2.5・11.5
**Evidence**: design.md「PersistWithhold」の Service Interface（`persist/withhold.rs` の `ensure_root_dir`）・「GhostRuntimeDirHook」

## Design Strengths

1. **継ぎ目の選び方が今の形に合っている**: 記憶の 13 種をそれぞれの書き手で止めず、保存の口 `FsPersistIo` の `read`／`commit` の中で控えるので、作り手 7 か所にも `placement/persist.rs`・`sylphya_wiring.rs` にも触らない（ウェーブ C11 の ④・⑥ との重なりが最小）。`PersistOutcome::Withheld` を `Saved`／`Degraded` と別の値にしたので、致命の机・告知・終了コードの道（`write-failure-notice`）に入らないことが型で決まり、網羅の `match` は `boot_resolve.rs` の 3 か所だけ（確かめ済み）。
2. **入口の断りが窓口の状態に触れない**: インストールは `install::submit` の頭、更新は `judge` の `Closing` の後で断り、段・預かり・背景のスレッドに触れないので「忙しいで残らない」（要件 7.6）が作りで守られる。URL の取り込みは `with_fetch` の既存の差し口を使って `new` の引数を変えず、`zorder_wiring_tests.rs` の字の檻を壊さない。後から足される入口（`startup-install-option`）も `submit` を通る限り同じ断りを受け、通らない入口は Revalidation Triggers に挙げてある。

## 補足（重大ではないが設計の討議で触れてよい点）

- 更新の断りは `Current`（`\![updatebymyself]`・`\![update,…]`）でも `OnUpdateResultEx` と `OnUpdateResult` の 2 件を送る。SSP で字まで測れたのは `\![updateother]` だけで、`\![updatebymyself]` は数えられなかった（requirements.md の測りの表）。要件 7.5 が確定しているので設計はそれに従っているが、ふだんの areka は `Current` で `OnUpdateResult` だけを送るので、討議で一言確かめておくとよい。
- 起動の前の守りのうち、プロセスの値 `readonly::is_on()` を読む配線（`layout()`・`default_app_profile_dir()`・`writable_copy`・`InstallCueSink::new`・`main` の `check_boot`・`roots::pick`）は、試験では純粋な関数として引数で検められるだけで、配線そのものは檻に入らない（「配線は再テストしない」の方針どおり）。要件 10.2 の前後比べはこれらを部品として順に呼ぶ形である。

## Final Assessment

- **Decision**: GO
- **Rationale**: 層・依存の向き・既存の型と字の檻との合い方に根の食い違いは無く、要件 1〜11 はどれも部品と試験に結び付いている。上の 3 件は設計の形を変えずに、タスクの段か設計の討議で直せる。
- **Next Steps**:
  1. 設計の討議（`kiro-design-discussion`）で Critical Issue 1〜3 の直し方を決め、design.md へ映す。
  2. その後 `/kiro-spec-tasks readonly-mode` でタスクを作る。Issue 1 の「部品の単位の判定と `…-1`／`…-10` の試験」、Issue 2 の「1 度だけ決める」をタスクの完了条件に入れる。
