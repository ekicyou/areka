# 設計の確かめ: install-before-boot

> 2026-10-31・`/kiro-validate-design`（対話なし・設計の段の確かめ）。読んだ物: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` のウェーブ C14 の行）。コードの突き合わせ: `install/procedure.rs`・`install/startup.rs`・`install/desk.rs`・`install/judge.rs`・`install/names.rs`・`boot_config.rs`・`boot_resolve.rs`・`emo2_boot/ghost_switch.rs`・`alert.rs`・`smoke_exit.rs`・`main.rs`・`crates/wintf/src/runtime/mod.rs`・`crates/real-run-kit/src/launch.rs`。

## まとめ

設計は、今の手続き `run_order` を変えずに起動の前の口（`BootPorts`）を 1 つ足し、起動の解決には「起こすゴーストのフォルダ名」を 1 つ渡すだけにしている。層の向きも、今のコードとの合い方も良い。コードについて書いてあることは、ほとんど今のコードのとおりだった。直すべき点は 3 つあるが、どれも形を作り直す話ではなく、タスクを作る段で直せる。**判定は GO**。

## 突き合わせた主な点（コードの読み）

| 設計に書いてあること | コード | 結果 |
|---|---|---|
| `run_order` は口 `InstallPorts`（8 つ）だけで外とやりとりする | `procedure.rs` の `trait InstallPorts`（`begin_archive`・`raise`・`ghost_facts`・`ask_terms`・`install_elsewhere`・`overwrite_running`・`record`・`warn_skipped`） | 合う |
| `Raised` に腕を足しても、全部の腕で分ける所は `send` だけ | 本体で `Raised` を腕ごとに分けるのは `send`（`Raised::Closed => Err`）だけ。`update/procedure.rs` の `Raised` は別の型 | 合う |
| `Unsent` では `OnInstallComplete` へ続かない | `archive_steps` は `raised == Raised::NoReply` だけを見る | 合う |
| `ghost_facts` が空の素性なら `accept` つきは `Refused`・`accept` の無いシェルと追加ファイルは `AcceptMissing` | `judge_accept`（`sakura_name: None`・`install_accept: []` では一致しない）・`destination_of`（`folder: None` なら `Elsewhere`） | 合う |
| `resolve_boot_with` は根の解決の結果を引数で受ける | `boot_config.rs` の `resolve_boot_with(resolved: Result<ResolvedRoots, RootError>, …)` | 合う |
| `GhostRoute` を全部の腕で分ける `match` は無い | 読み手は `== Argv`・`== Switched`・`matches!(Argv \| Pointed)`・`?route` だけ | 合う |
| `CallGhost` の段は記憶の段と同じ引き当てで書ける | `resolve_ghost` の記憶の段は `find(listed, memory)`（`same_folder_name`） | 合う |
| `main.rs` は 986 行・呼ぶ所を書き換えても増えない | 986 行。`resolve_boot(&opts.boot)` は 213 行目。新しい呼び出しは 1 行に収まる。`use` の並びも行数は変わらない | 合う |
| `readonly_tests.rs` の走査は新しい呼び出しでも当たる | 走査の綴りは `resolve_boot(&`。`install::startup::resolve_boot(&opts.boot, …)` に含まれる | 合う |
| DPI の設定は `WinApp::with_exit_policy` の中で、成否どちらも 1 行残す | `runtime/mod.rs` の `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`。`wintf` は `pub use runtime::*`。`Win32_UI_HiDpi` はワークスペースで有効 | 合う |
| `SwitchAcceptCount` の読み手は `install::startup` だけ | 書くのは `ghost_switch.rs` の `accept_switch`、読むのは `startup.rs` だけ | 合う |
| `names::update` は起動の前なら値を載せず、起こしたゴーストへは `names::reseed` が載せる | `update` は `current_runtime` が無ければ `published=false`。`reseed` は `rewire_runtime_consumers` から呼ばれる | 合う |
| `real-run-kit` の `Launch::Empty`・`before_spawn` | `launch.rs` に在る | 合う |
| ウェーブ C14 の約束: `ghost_session.rs` に触らない | 触るファイルの表に無い。`ghost_session.rs` は `StartupInstall` も `SwitchAcceptCount` も読まない | 守れる |
| テストの 37 か所の呼び出し（`resolve_boot_with`） | **違う**（下の 2 つ目） | 違う |

**ウェーブ C14 の重なり**: 設計が触るファイルは、棚卸㉛で測った ② の並び（`install/{startup,worker,desk}.rs`・`boot_config.rs`・`main.rs`・`ghost_switch.rs`・実機の試験・`dist/README.txt`）より多い。足されたのは `boot_resolve.rs` と兄弟のテスト 2 本・`install/procedure.rs`・`install/mod.rs`・`procedure_branch_tests.rs`・`desk_tests.rs`・新しい `boot_config_callghost_tests.rs`・`crates/wintf/src/runtime/mod.rs`・`doc/pasta-real-run.md`。同じウェーブのほかの 6 本の並びと比べると、どれとも重ならない（④ は wintf の `com/wic.rs` と `tests/com/wic_test.rs` で、`runtime/mod.rs` とは別のファイル）。重なりは 0 のままなので問題ではないが、着地のときにロードマップの ② の行へ足しておくとよい。

## 直すべき点（3 つ）

### 1. 起動の前の画面が、人の見ていない走りで止まったままになる

**何が問題か**: 2026-10-31 に `alert::raise` は、有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）が立つ走りでも画面を出さないように直った（「人が見ていない走りで、箱が閉じられるのを待つと、プロセスが終わらない」）。設計の `BeforeBootEnv.alert_suppressed` は `alert::suppressed()`（`AREKA_NO_ALERT` だけ）を読む。起動の前は、自動終了の仕掛けがまだ立っていない（立つのは `WinApp` の後）。そのため、`AREKA_NO_ALERT` を付けずに `AREKA_APP_SMOKE_EXIT_MS` だけで回した走りで、利用条件つきの書庫を `--install` に渡すと、`main` のスレッドが利用条件の画面で止まり、いつまでも終わらない。起動の後に入れていた今は、画面は背景のスレッドに出て、自動終了がアプリを畳むので、こうはならない。つまり、この spec で新しく生まれる止まり方である（`real-run-kit` は `AREKA_NO_ALERT=1` を付けるので、実機の試験は止まらない）。この spec の一番の使い手は「空の根から 1 行で回す AI のエージェント」なので、当たりやすい。

**影響**: 自動の走りが終わらず、走らせた側が失敗に気づけない（要件 1.8「入れ終わった後に今と同じ起動を続ける」・要件 6.4 の「黙って捨てない」の趣旨に当たる）。

**直し方の案**: `resolve_boot` が作る `BeforeBootEnv` を `alert_suppressed: alert::suppressed() || smoke_exit::smoke_exit_ms().is_some()` にする（起動の前の道だけ・1 行）。抑止なら利用条件は今の手続きどおり拒否になる（要件 2.4）。要件 2.4 の「今の決まりで画面を出さないとき」を「終わりの告知と同じ決まり」と読んでよいかは、設計の討議で決める。決めないなら、説明書（`dist/README.txt`）に「自動の走りでは `AREKA_NO_ALERT` も付ける」と書く。

**場所**: 設計の「StartupFlow」の `BeforeBootEnv`・「BootPorts」の `ask_terms`・`warn_skipped` の段。

### 2. 起動の解決の入口が 2 つになり、片方は誰も呼ばない（テストの呼び出しの数も違う）

**何が問題か**: 設計では、本番は `install::startup::resolve_boot` → `install::startup::resolve_boot_from` → `boot_config::resolve_boot_with_call` と呼ぶ。一方で、新しい形の `boot_config::resolve_boot(resolved, request, call_ghost, app_profile_dir)` を「本番の薄い口」として残している。この通りに作ると `boot_config::resolve_boot` を呼ぶ所が無くなり、使われない関数になる（コンパイラが dead code を警告する）。本番の道が 2 本あるように読める。また、「`resolve_boot_with` はテストだけの口に下ろし、テストの 37 か所の呼び出しは動かさない」とあるが、今のコードでは `resolve_boot_with` を呼ぶのは `boot_config.rs` の中の 2 か所（`resolve_boot` とテスト用の `boot_config::resolve_boot_from`）だけ。テストが呼ぶのは `boot_config::resolve_boot_from` で、約 20 か所（`main_config_input_tests.rs` など）。さらに `resolve_boot_from` という名前が `boot_config`（テスト用）と `install::startup`（本番とテスト）の 2 か所にできて、紛らわしい。

**影響**: 作る人が、どの口を本番の道にするかで迷う。警告が残る。「37 か所」を信じてテスト用の口を作ると、要らない口が増える。

**直し方の案**: `boot_config::resolve_boot` は消す（本番の入口は `install::startup::resolve_boot` だけにする）。`resolve_boot_with` は `resolve_boot_with_call` に名前を変えるだけにし、テスト用の `boot_config::resolve_boot_from` はそれに `None` を渡す 1 か所の直しにする（テストの約 20 か所は動かさない）。`install::startup` の側の中身の関数は、`boot_config` と紛れない名前（たとえば `resolve_boot_before_with`）にする。「Allowed Dependencies」の `boot_config::{resolve_boot, …}` も合わせて直す。

**場所**: 設計の「BootConfigCall」・「StartupFlow」の Service Interface・「Modified Files」の `boot_config.rs` の行・「Allowed Dependencies」。

### 3. DPI の立て方と「initial_install.nar だけ」の組が、決定論のテストに映らない

**何が問題か**: 決定論のテストは、どれも `alert_suppressed: true` で回す。設計では、`ensure_process_dpi_awareness` を「告知が抑止されていないときだけ」呼ぶので、その枝はテストで一度も通らない。確かめは「実機で 1 回」だけになる。DPI の扱いは、前に「設定の失敗が見えず `WM_DPICHANGED` が 0 件」の不具合を出した所で、後の直しで呼ぶ順が崩れても、テストは緑のまま気づけない。また、要件 9.5 の「`initial_install.nar` だけ」（`callghost` が渡され、ゴーストは `initial_install.nar` だけで入り、`--install` ではゴーストが入らない）の組が、テストの表に無い。`decide_call_ghost` に `--install` の終わり方だけを渡す形なので、形の上では正しいが、要件 9.5 の字のとおりには固まっていない。

**影響**: 要件 2.2・2.3（DPI 追従を壊さずに出す）と要件 9.5 が、テストで固定されていない。

**直し方の案**: ⑴ `BootPorts` が画面を出す前に `ensure_process_dpi_awareness` を呼ぶことを、静的な構造の確かめ（`before_boot_ports.rs` の中で、`ensure_process_dpi_awareness` の呼び出しが `ask_yes_no`・`warn_ok` より前にあることを読むテスト。`readonly_tests.rs` の走査と同じ形）で固める。または、画面を出す関数を `BootPorts` に差し込めるようにして、偽の画面で呼ぶ順を確かめる。⑵ 「起動の前の道 → 起動の解決」のテストに、「`initial_install.nar` でゴーストが入り、`--install` ではゴーストが入らず、`callghost` あり → `startup_callghost_skipped reason=no_ghost_installed` で、今の選び方で選ぶ」の 1 本を足す。

**場所**: 設計の「Testing Strategy」の Unit と Integration・「DpiOnce」。

## 良い所

1. **今の手続きを変えずに起動の前へ持っていく形**: 手続き `run_order` の段・判定・記録の語には触らず、口を 1 つ足して `Raised::Unsent` の 1 腕を加えるだけで、「送っていないのに『送りました』と書かない」を守っている。`run_order` が途中でやめる 2 つの条件（`Closed`・`ghost_facts` が `None`）を口の決まりとして書き出していて、作る人が間違えにくい。`accept` の照合も、空の素性を渡すだけで今の `judge_accept` がそのまま要件 2.5 の答えを出す。新しい判定のコードが要らない。
2. **層の向きと約束の守り方**: `boot_config` と `boot_resolve` は `install` を知らず、「起こすゴーストのフォルダ名」を 1 つ受けるだけにしている。`main.rs` は呼ぶ所 1 か所の書き換えで行数を増やさない。ウェーブ C14 の約束（`ghost_session.rs` に触らない・`install/desk_overwrite_tests.rs` に触らない）も守れる。消す語・消すテストを、読む物ごとに grep で数えてある。

## 判定

**GO**。

**理由**: 直すべき 3 点は、どれも形を作り直す話ではない。1 つ目は起動の前の旗 1 行（または説明書の 1 文）、2 つ目は入口の整理と名前、3 つ目はテストを 2 本足す話で、設計の討議で決めてタスクに入れれば足りる。層の向き・依存・ウェーブの約束・要件の対応表は揃っていて、コードについて書いてあることも、テストの呼び出しの数のほかは今のコードのとおりだった。

**次の段**:
- 設計の討議（`/kiro-design-discussion install-before-boot`）で、上の 3 点を議題にする。特に 1 つ目は、要件 2.4 の読み方を決める。
- 決めたことを設計へ入れた後、`/kiro-spec-tasks install-before-boot` でタスクを作る。
- 着地のとき、ロードマップのウェーブ C14 の ② の行に、増えた触るファイル（`boot_resolve.rs`・`install/procedure.rs`・`install/mod.rs`・`crates/wintf/src/runtime/mod.rs`・`doc/pasta-real-run.md` など）を足す。
