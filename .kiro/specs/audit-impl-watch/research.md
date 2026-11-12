# ギャップ分析: audit-impl-watch

> 2026-11-13・枝 `claude/kiro-start-audit-impl-watch-3784e8`（HEAD `005cc6ef`＝`main` の `4863cb66`＋spec の初期化 1 コミット。`git fetch` 後の `origin/main` も `4863cb66`＝取り込み済み・`crates/areka-impl-watch/` の差分 0）。
> 物差しは ponytail のプラグインの `ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.3/skills/ponytail-audit/SKILL.md`。5.3.2 の同じファイル・marketplaces の写しとバイト単位で同じ＝`diff -q` で差なし）。
> 作法のうち 2 つを外した（要件 1.3・1.4）: 「最大 20 件」は外して全件を載せる。「読まなかった所（Not checked）」は残さない（範囲の 40 ファイルを全部読んだ・§1 の表）。
> 読み方: 本番のコード 12 ファイルと `Cargo.toml` は、こちらで全行を読んだ。テスト 27 ファイルは 3 本の読み手（読むだけのサブエージェント）に分けて全行を読ませ（読み手も、受け持つテストが試す本番のファイルを全行読んだ）、出た指摘をここでまとめて重なりを 1 件に寄せ、番号を振り直した。載せた指摘は、こちらでファイルと行を開いて中身を確かめた。確かめて誤りだったもの（読み手の「`merged` が 2 つのリポジトリの机を空ける場合のテストが無い」＝`cli_commands_tests.rs:160-178` に在る）は落とし、§4 に書いた。

## 1. 範囲の測り直し（要件 1.2）

| | 起票のとき（main `e3c2cbe4`） | 要件の段（main `4863cb66`） | 今（HEAD `005cc6ef`） |
|---|---|---|---|
| ファイルの数 | 40 | 40 | **40**（Rust 39＋`Cargo.toml`。fixture のファイルは無い） |
| コードの行 | 3,759 | 3,759 | **3,759**（12 ファイル） |
| テストの行 | 11,945 | 11,945 | **11,945**（27 ファイル） |
| `Cargo.toml` | — | 25 | **25** |
| 合計 | — | 15,729 | **15,729** |

- 数え方: brief と同じく `_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた（`wc -l`）。
- 1,000 行の近く: `src/plan_desk_tests.rs` 924・`src/cli_args_tests.rs` 871・`src/wait_loop_tests.rs` 776。本番のコードの最大は `src/plan.rs` 648。
- 読んだファイルと行（全行）:

| 読み手 | ファイル（行） | 行 |
|---|---|---|
| こちら（本番） | `Cargo.toml`（25）・`src/main.rs`（35）・`src/error.rs`（108）・`src/home.rs`（75）・`src/state.rs`（313）・`src/plan.rs`（648）・`src/plan_turns.rs`（394）・`src/cli.rs`（406）・`src/cli_args.rs`（563）・`src/presence.rs`（118）・`src/status.rs`（362）・`src/store.rs`（422）・`src/wait.rs`（315） | 3,784 |
| A（入口のテスト） | `src/cli_args_tests.rs`（871）・`src/cli_commands_tests.rs`（700）・`src/cli_desk_tests.rs`（263）・`src/cli_test_support.rs`（132）・`src/cli_tests.rs`（204）・`src/cli_wait_tests.rs`（430）・`src/error_tests.rs`（148）・`src/home_tests.rs`（278）・`src/main_layering_tests.rs`（247） | 3,273 |
| B（判断の核のテスト） | `src/plan_clear_tests.rs`（187）・`src/plan_desk_tests.rs`（924）・`src/plan_liveness_tests.rs`（462）・`src/plan_reclaim_tests.rs`（611）・`src/plan_stop_tests.rs`（698）・`src/plan_test_support.rs`（261）・`src/plan_turns_tests.rs`（486） | 3,629 |
| C（口と待ちと実機のテスト） | `src/presence_tests.rs`（266）・`src/state_tests.rs`（189）・`src/status_tests.rs`（661）・`src/store_ports_tests.rs`（742）・`src/store_test_support.rs`（144）・`src/store_tests.rs`（663）・`src/wait_loop_tests.rs`（776）・`src/wait_test_support.rs`（30）・`src/wait_tests.rs`（491）・`tests/real.rs`（752）・`tests/real/mod.rs`（329） | 5,043 |

## 2. この範囲がしていること・想定した負荷

- **何をするか**: 並んで走る実装セッション（Claude のセッション）へ、3 種類の机（実装の机・排他テストの机・マージの机）を貸し出すコンソールアプリ `areka-impl-watch.exe`。各セッションがスキルから Bash で直に呼ぶ。
  - 判断の核 `plan::apply(state, cmd, caller, now, alive)`（回収 → コマンド → 番の決め直し）は純粋で、ファイルも時計もログも触らない（構造テスト `main_layering_tests.rs` が見張る）。番の決まりは子 `plan_turns.rs`。
  - 口: 置き場所（環境変数 `AREKA_IMPL_WATCH_HOME`・`home.rs`）・居る印（`alive/<識別>.<種類>.lock` を握りっぱなしにする・`presence.rs`）・`state.json`（`state.lock` の排他＋一時ファイルからの置き換え書き・版 2・`store.rs`）・読み物 `status.md`（`status.rs`）・`impl-watch.log`。
  - 待ち（見張り・`--wait`・再開）は「番が来たら終わるコマンド」で、1 秒ごとに状態ファイルの更新時刻と大きさを見て、変わったか 10 回目ごとに読み直し、30 回目ごとに周期の一回り（`tick`）を呼ぶ（`wait.rs`）。
  - 外への約束はコマンド・引数・終了コード・端末へ出す文・`--help`・状態ファイルの形式と版・置き場所の下のファイルの名前（正本 `doc/impl-watch.md`）。呼び手はスキル `kiro-impl`・`kiro-complete`・`kiro-next`・`release`・`update`・`kiro-watch-clear`・置き方の道具 `tools/impl-watch-install.ps1`。areka の実行時コードからは参照されない。
- **想定した負荷**: マシン 1 台・並んで走るセッション 8 本前後（ウェーブの上限）。各セッションが見張り 1 つと待ち 1〜2 つを持ち、1 秒ごとに状態ファイルの更新時刻を見る。状態を変える呼び出しは 1 回ごとに `state.lock` を短く取る。この負荷で、どの読み手も「必ず直す」に当たる壊れ方を見つけなかった。

## 3. 指摘の一覧（全 17 件）

組の名は audit の 3 つ: **Must fix（必ず直す）**・**Should fix（直したほうがよい）**・**Nice to have（できれば）**。番号は 3 つの組を通して 1 から（以後の段はこの番号で指す・要件 1.5）。

- 数: **Must fix 0 件・Should fix 7 件・Nice to have 10 件**。本番のコードの振る舞いに関わるのは #1・#8・#9 の 3 件だけで、残りの 14 件はテストとコメント。
- 「外への約束」の欄: 直しが外への約束（コマンド・引数・終了コード・端末へ出す文・`--help`・状態ファイルの形式とその版・置き場所の下のファイルの名前）を変えるか。**「変わる」は C19 のあいだ取れない**（要件 6.4・6.5＝議題）。「—」は変わらない。
- 「範囲外」の欄: 直しが `crates/areka-impl-watch/` の外のファイルに及ぶとき、そのファイル。**どの指摘の直しも `crates/areka/` と `Cargo.lock` には及ばない**（C19 の約束 ⑴・⑷。依存も足さない）。
- 「残す候補」の欄: 直さずに残してよい 3 種類（⑴ 開発者の裁定・完了した spec の決め／⑵ わざと置いた先送りの印／⑶ SSP・ukadoc の正典に関わる振る舞いの変更）に当たるかもしれないもの。決めるのは要件の討議（要件 2.2）。「—」は当たらない。⑶ に当たるものは無い（開発の道具で、SSP・ukadoc とは関わらない）。

### 3.1 Must fix（0 件）

無い。判断の核（`plan.rs`・`plan_turns.rs`）・状態の型（`state.rs`）・入口（`cli.rs`・`cli_args.rs`）・口（`store.rs`・`presence.rs`・`status.rs`・`home.rs`）・待ち（`wait.rs`）のどれにも、想定した負荷で誤った結果・データの消失・止まりを生む所は見つからなかった（確かめた中身は §4）。

### 3.2 Should fix（7 件）

| # | 場所 | 何が壊れるか（具体的な場合） | いちばん小さい直し | 外への約束 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 1 | `src/state.rs:79`（`Participant::id`）・`src/plan.rs:355`・`:359`（`Unstop`）・`:574`（`resume` の出来事）・`:543`（`join` が書く） | **参加者の識別が 2 か所（表の鍵と記録の欄 `id`）に在り、`unstop` と再開の出来事だけが欄の方を見る**。ほかの判断（回収・停止要請・番・`status`・待ちの判定）は全部、表の鍵を見る。欄が鍵と食い違う状態ファイル（例 `"participants": { "a": { "status": "stopped" } }`＝欄 `id` が欠けて `""`）では、`status` は `a stopped` と出すのに、`unstop --id a` は `nobody to unstop`（終了コード 3）で戻せず、名指しなしの `unstop` は戻すが出来事は `Unstopped { id: "" }`・結果は `unstopped n=1` でログの識別が空になる。番の決め直しの再開（`Resumed`）も同じく識別が空になる。今は欄を書くのが `join` だけなので、手で書き替えた（手順書 7 節は「手で書き替えない」）ファイルでしか起きない。テストは無い | `Unstop` は `state.participants.iter_mut()` の鍵で比べ、`resume` は識別を引数で受けて出来事に載せる（欄を読む所を 0 にする・3 か所）。欄の無い参加者の `unstop --id` のテストを 1 本。欄そのものを消すかは議題（下の「外への約束」） | **—**（鍵で比べる直しだけなら）。欄 `id` を消すなら **変わる**＝状態ファイルの形（版 2 のまま消すと、古い実行ファイルが新しいファイルを読んで欄を `""` で埋め、古い側の `unstop --id` が当たらなくなる。版を上げると置き方の道具が `clear` を呼び、走っている待ち・見張りが全部終わる＝要件 7.3） | 欄を消すなら `doc/impl-watch.md`（状態ファイルの形の説明があれば） | — |
| 2 | `src/plan_turns.rs:290-311`（`grant_one` のひとり占めの枝）・`:304`（`if request.desks.has_test()`）・`:324-331`（`grantable` のひとり占めの枝） | **`--desk sole` だけの申し込みが番を受ける場合を、どのテストも通らない**。`DeskSet::Sole` がテストに出るのは、断られる申し込み・`return`・並べ替えの表だけ（grep: `plan_turns_tests.rs:317・319・357・394・410`・`plan_desk_tests.rs:919`・`cli_desk_tests.rs:143`〔断り〕）。全探索（`plan_liveness_tests.rs` の字母は `request(id, Sole)` を含む）が見るのは不変（待ちの輪が無い・走っている印の組・固定点・協力的に進めれば全部空く）だけで、「渡した机の組が申し込んだ組と同じ」は見ない。304 行目を `if true` に変えると、`--desk sole` の番で空いている排他テストの机まで持ち主にするが、テストは全部緑のまま。そのとき、後から参加して停止要請を受けた者の `request --desk test` は（決まり 6 はひとり占めが待たれている間だけなので）直ちに渡るはずなのに、ひとり占めの持ち主が全部返すまで待たされる | `state_with(["a","b"])` で b が a のために止まった後、`request("a", Sole)` が番を受けて `state.test` が `None` のままであること、続けて参加した D の `request("d", Test)` が同じ呼び出しで渡ることを見るテストを 1 本（または `check_invariants` に「持ち主の組は番の出来事 `DeskGranted.desks` と同じ」を 1 行） | — | — | — |
| 3 | `src/main_layering_tests.rs:148-170`（`files_that_write_to_the_terminal_have_ascii_only_literals`） | **端末へ出す文の ASCII の見張りが、見るファイルの一覧を手で持ち、一覧が全部かを確かめない**。モジュールの doc は「`main.rs` のほかの本番のファイル全部」と書き、今は 11 ファイルで合っている（Glob で確かめた）。`src/lockwait.rs` のような本番のファイルを足して `"待ちが切れた"` を端末へ出しても、この見張りは緑のまま（入口のテストの `call()` の ASCII の確かめが、たまたまその道を通るときだけ捕まる） | 同じテストの中で `src/` を `read_dir` し、`_tests.rs`・`_test_support.rs` で終わらず `main.rs` でもない名前の集まりが一覧の名前と同じことを `assert_eq!`（8 行ほど） | — | — | — |
| 4 | `src/cli_test_support.rs:21-46`（`sample`・`line`）・`src/cli_args_tests.rs:33-63`（`args`・`sample`・`line` の写し） | **引数を組むテストの支えが 2 組あり、入口の側の組はほとんど使われない**。`cli_test_support.rs` の `sample`・`line` の使い手は `cli_tests.rs:79`・`:142` の `line("watch", &[Id, Repo])` の 2 回だけ（grep）で、10 個の引数の表は `watch --id a1 --repo areka` を作るためだけに在る。`cli_args_tests.rs` は同じ 3 つの関数を写して持ち、その注（37-38 行目）は「`--desk` を含むので別に持つ」と書くが、入口の側の `--desk` の値はどこでも使われていない。`--desk` の見本の値を変えると片方だけが変わる | `cli_test_support.rs` の `sample`・`line`（と 10 行目の `Opt` の読み込み）を消し、`cli_tests.rs` の 2 か所は `args(&["watch", "--id", "a1", "--repo", "areka"])` に。`cli_args_tests.rs` の `args` は入口の側のものを読み込む（約 −30 行） | — | — | — |
| 5 | `src/cli_commands_tests.rs:56-58` と `src/cli_desk_tests.rs:43-45`（`not_applied`）・`src/cli_wait_tests.rs:61-65` と `src/cli_desk_tests.rs:48-51`（`waits`）・`src/cli_desk_tests.rs:174-175`（状態ファイルを `std::fs::write` で直に書く） | **入口のテストに、中身が同じ支えが 2 つずつある**。`not_applied`（`not applied:` の文の組み立て）と `waits`（状態ファイルの待ちの記録の一覧）は、同じ中身が 2 ファイルに在る。`not applied:` の綴りを変えると 2 か所を揃えて直す。`cli_desk_tests.rs:174-175` は、双子のテスト（`cli_wait_tests.rs:170`）が使う支え `put_state` を使わずに書く | 2 つを `cli_test_support.rs`（同じ組の `done` が在る）へ移し、174-175 行目を `put_state` に（約 −10 行） | — | — | — |
| 6 | `src/plan_desk_tests.rs:41-57`（`merge`・`merged`）・`src/plan_liveness_tests.rs:24-49`（`merge`・`merged`・`load_running`）・`src/plan_stop_tests.rs:25-31`（`load_granted`）・`:68`（`resumed`）・`:84-91`（`load_running`）・`src/plan_turns_tests.rs:14-20`（`granted`）・`:433`・`:459-464`（閉じた関数）・ほかに手で書いた同じ組み立て（`plan_desk_tests.rs:419-425・827`・`plan_reclaim_tests.rs:164・221-227`・`plan_stop_tests.rs:226-230・478-484`・`plan_clear_tests.rs:99`・`plan_turns_tests.rs:250`）・`src/plan_liveness_tests.rs:176-181`（`holds_merge`） | **判断の核のテストに、コマンドと出来事の組み立てが 5〜6 か所ずつ写されている**。`Command::Merge`・`Command::LoadRunning` に欄が 1 つ増えると、5〜6 か所を揃えて直す。`plan_liveness_tests.rs` の `holds_merge` は本番の非公開の `plan_turns.rs:219-222` の写しで、このテストは `turns` の子なので `super::holds_merge` を直に呼べる。`plan_desk_tests.rs` は 1,000 行の番人まで残り 76 行 | `merge`・`merged`・`load_running`・`granted`・`resumed` を `plan_test_support.rs`（`request`・`give_back` の隣）へ移し、写しと手書きを置き換える。`holds_merge` の写しを消して `super::holds_merge` に（約 −60 行） | — | — | — |
| 7 | `const T0: u64 = 1_791_000_000` が 4 か所（`plan_test_support.rs:10`・`status_tests.rs:12`・`wait_test_support.rs:7`・`store_test_support.rs:19`）・`entries` が 3 か所（`cli_test_support.rs:100`・`home_tests.rs:16`・`presence_tests.rs:25`・中身は同じ）・`names` が 2 か所（`store_test_support.rs:114`・`cli_test_support.rs:110`）・`home_in` が 2 か所（`presence_tests.rs:18`・`store_test_support.rs:42`）・`read_state` が 2 か所（`cli_test_support.rs:124`・`store_test_support.rs:129`） | **クレートのテスト全体で、同じ支えがモジュールごとに写されている**。テストの起点の時刻やフォルダの一覧の取り方を変えると、3〜4 ファイルを揃えて直す（揃え損ねると、時刻の綴りを比べるテスト〔`status_tests.rs:13` の `T0_UTC`〕だけが別の起点で動く） | クレートに `#[cfg(test)]` のモジュールを 1 つ（例 `src/test_common.rs`・`main.rs` に `#[cfg(test)] mod test_common;`）置いて `T0`・`entries`・`names`・`home_in` を持たせ、写しを消す（約 −40 行）。モジュールの読み込みの向きの見張り（`main_layering_tests.rs`）に新しいモジュールの扱いを足すかは設計で確かめる（§8） | — | — | — |

### 3.3 Nice to have（10 件）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 外への約束 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 8 | `src/store.rs:203-210`（`summary`＝`status`）・比べる所 `:151-179`（`change` は `state.lock` の中で `status.md` を書く） | **`status` が `state.lock` の外で `status.md` を書くので、同時に走った変更の読み物を古い中身で上書きしうる**。X が `status` で状態 S1 を読む → Y が `merged` で排他を取り S2 と `status.md`(S2) を書いて放す → X が S1 から組んだ `status.md` で置き換える。`state.json` は正しいが、`status.md` は次の変更か次の `status` まで古い。見張りの終わりの 2 行目 `details: <status.md>` が読ませる停止要請の理由が、そのあいだ載っていないことがある（端末の 1 行目 `stop requested by <識別>` は正しい） | `summary` は `state.lock` を 1 回だけ待たずに試し（`try_lock`）、取れたら中で読み直してから `status.md` を書く。取れなければ書かない（排他を持つ変更が自分で書く）。読むことは今どおり排他を取らない | —（`status` が読み物を書き直さない時があるだけ。文・終了コード・ファイルの名前は同じ） | `doc/impl-watch.md:576`（「`status.md` の中身が古い」の節の文） | **⑴**: 完了 `areka-P0-impl-watch` の design.md 567 行目「`summary`（`status`）は排他を取らずに `status.md` を置き換えるので、同時に走った変更の読み物を、少し古い内容で上書きすることがある（次の変更か、もう一度の `status` で直る。正本は状態ファイル）」＝わかっていて受け入れた限界。手順書 6 節（574-576 行目）にも同じ文 |
| 9 | `src/store.rs:344-354`（`replace`）・`:406-410`（`write_synced` の `sync_all`）・`:334-340`（`write_status`） | **状態を変える 1 回ごとに、`state.lock` を握ったままディスクへの確定（`sync_all`）を 2 回する**。2 回目は読み物 `status.md` の分で、`status.md` は次の変更か `status` で作り直す読み物なので、確定しても守るものが無い。8 本が同時に `watch` を始めると（`tests/real.rs` の `simultaneous_changes_are_all_kept` の形）、16 回の確定の後ろに並ぶ（8 回で済む）。上限 10 秒（`LOCK_LIMIT`）には遠く届かないので、速さの改善だけ | 読み物の一時ファイルは `sync_all` を呼ばずに書く（`replace` に確定するかの引数を 1 つ足すか、読み物だけ `fs::write` で一時ファイルを書いて名前を替える） | — | — | — |
| 10 | `src/store_ports_tests.rs:63-65`（`lossy`） | **本番の `store.rs:401-403` の `lossy` の写し**。`ports_tests` は `store` の子なので、非公開の `super::lossy` を直に呼べる（grep で `fn lossy` はこの 2 か所だけ） | 3 行を消して `super::lossy` を使う | — | — | — |
| 11 | `src/state_tests.rs:143-146`（`a_file_without_version_does_not_read_as_the_current_version`） | **コードが頼っていない性質を見るテスト**。`State::default().version != VERSION` を見るが、版の無いファイルは `store::parse`（`store.rs:379-382`）が形を読む前に `NoVersion` で断るので、`Default` が変わっても振る舞いは変わらない。その断りは `store_tests.rs` の `json_of_the_wrong_shape_is_broken_too`（`{}`・`{ "participants": {} }`）がもう見ている | 4 行を消す | — | — | — |
| 12 | `src/home_tests.rs:111-121`（`a_relative_value_is_refused_before_anything_is_created`・`a_rooted_value_without_a_drive_is_refused_before_anything_is_created`）と `:130-132` | **2 本のテストが、3 本目の最初の 2 行と同じ**。2 本は `values_landing_in_a_temp_folder_are_refused_and_create_nothing(&RELATIVE)`・`(&ROOTED)` を 1 回ずつ呼ぶだけで、`the_spellings_named_by_the_design_are_refused_and_create_nothing` が同じ 2 つを較正として先に呼ぶ。後退は 3 本とも同じ所で落ちる | 2 本（11 行）を消す | — | — | — |
| 13 | `src/cli_tests.rs:51-70`（`a_usage_error_reads_and_writes_nothing`）・`src/cli_commands_tests.rs:457-485`（`a_malformed_clear_repo_is_2_and_creates_or_changes_nothing`）・`src/cli_desk_tests.rs:249-263`（`a_malformed_desk_is_2_and_reads_or_writes_nothing`） | **「使い方の誤りは何も読み書きしない」を 3 本（`--wait` 付きを数えると 4 通り）で見る**。`dispatch` は使い方の誤りで置き場所を決める前に返る（`cli.rs:83-90`）ので、誤りの種類によらず同じ道。誤りの文は `cli_args_tests.rs:842-870`（`clear --repo`）・`:700-756`（`--desk`）がもう文字どおり見ている | `clear --repo` と `--desk` の誤りを 1 つずつ `cli_tests.rs:55-60` の並びへ足し、2 本を消す（約 −40 行）。`cli_commands_tests.rs` の「状態の在るときも変えない」の半分は、`cli_tests.rs` の並びを状態の在る置き場所でも回す形で残す | — | — | — |
| 14 | `src/cli_args_tests.rs:541-547`（`merge` の行を見る）と `:764`・`:619-636`（`usage_notes_of_the_resume_waits_…`）と `:758-790`（`usage_shows_the_merge_request_return_and_loadrunning_lines_of_the_design`） | **`--help` の行を見るテストが重なる**。`merge` の名前の行は 543 行目と 764 行目で 2 度見る。2 本の「名前の行を探して次の行を比べる」テストは同じ形 | 1 本の表のテストにまとめ、541-547 行目の重なりを消す（約 −20 行） | — | — | — |
| 15 | `src/plan_liveness_tests.rs:390-393`・`:401-404`（`serde_json::to_string` の前後の比べ）・`src/plan_stop_tests.rs:145-149`（理由の比べ）・`src/plan_desk_tests.rs:244`（`state.waits[0].id`）・`src/plan_test_support.rs:255-261`（`fake_presence_answers_present_unless_told_absent`） | **落ちようのない確かめ**。390-404 行目は、直前の `assert_not_applied` がもう `!changed`（`*state != before`＝状態の全部の比べ）を見ているので、状態だけで決まる JSON の比べは独りでは落ちない。145-149 行目と 244 行目は、直前の状態の全部の比べ（期待の状態に同じ値が入っている）の後に一部をまた比べる。255-261 行目はテストの支えの偽物を試すテストで、偽物が壊れれば何十本ものテストが先に落ちる | 該当の行とテスト 1 本を消し、`…_byte_for_byte` の名を直す | — | — | — |
| 16 | `src/plan_turns_tests.rs:103-121`（`replan_twice_changes_nothing_the_second_time`）・`src/plan_liveness_tests.rs:360` と `:366-382`（`overlapping_scene` と `overlapping_scene_grants_every_request_through_the_cooperative_run`）・`src/plan_turns_tests.rs:457-486`（`recording_a_running_load_test_needs_both_desks_free`）と `src/plan_stop_tests.rs:433-470`・`:520-545` | **ほぼ写しのテスト**。`replan_twice…` の固定点は、全探索の `check_invariants`（`plan_liveness_tests.rs:221-228`・同じ形の状態に深さ 2 で届く）と多くのテストの末尾の `assert_fixed_point` が見ている。`overlapping_scene(&["L","U","M"])` と 366-382 行目は同じ場面を組み、後者は渡る順の確かめだけを足す。走っている負荷テストの記録の「両方空き → 記録・片方でも持ち主 → 断り」は 3 か所に分かれている | `replan_twice…` を消す・`overlapping_scene` に渡る順の確かめを入れて 366-382 行目を消す・記録の 3 か所を「両方空き・test だけ持たれる・sole だけ持たれる・両方持たれる」の 1 本の表のテストに（約 −50 行） | — | — | — |
| 17 | `src/error.rs:61`（`Damage::WrongShape` の doc「版は 1 だが欄の形が合わない」）・`src/plan.rs:1`（「規則 1〜7」） | **コメントが今のコードと合わない**。状態ファイルの版は 2（`state.rs:24`）で、`WrongShape` は版 2 のファイルの形が合わないとき。`plan.rs` の「規則 1〜7」は、子 `plan_turns.rs:5-17` の「番の決まり 1〜6」とも、手順書 1 節の「決まり 1〜11」とも数が合わず、どの表を指すか読めない | `error.rs:61` を「版は合うが欄の形が合わない」に。`plan.rs:1` は番号を落とす（「番の決まり（`plan_turns.rs`）・回収・番の決め直し・出来事」） | — | — | — |

- Lean（消せる行の見込み）: **約 −270 行（ほぼ全部テスト）・依存 −0**。

## 4. 指摘にしなかったもの（確かめた結果）

- **`merged` が 2 つのリポジトリの机を同時に空ける場合のテストが無い（読み手 B）**: 誤り。`cli_commands_tests.rs:160-178` が、`zeta` と `areka` の両方の持ち主の `merged` で `merged repo=areka\nmerged repo=zeta` と、空いた後の持ち主 0 を見ている。最初の 1 つで止める変更はこのテストで落ちる。判断の核の側（`plan_desk_tests.rs:588-630`）は 1 つ持ち・1 つ待ちの形。
- **Windows の装置名（`con`・`nul`・`aux`・`com1`）を `--id` にする**: 形の決まり（`cli_args.rs:102-107`）を通り、居る印は `alive\con.watch.lock` になる。この機械（Windows 11・10.0.26300）で `con.watch.lock` ほか 4 つを作って排他で握り、普通のファイルとしてできることを確かめた（スクラッチの一時フォルダで）。Windows 10 では装置名に化けうるが、想定した負荷（開発者の機械 1 台）の外なので指摘にしない。
- **端末の文の逃がしの重なり**: `cli.rs:302`（`merged repo=`）・`:330-333`（`clear --repo`）は、引数の形の決まりで ASCII のはずの値を `escape_path` に通す。値は状態ファイルから来る（手で直したファイルからは何でも来うる）ので、`status.rs:368-370`・`wait.rs:185-187` と同じ方針で、要らない逃がしではない。`queued merge repo=`（`cli.rs:288`）は打たれた引数（形の決まりを通った値）から組むので逃がさない＝方針どおり。
- **`waiting` が居る印を握る前に `store::open` を呼ぶ（`cli.rs:159-160`）**: `open` が触るのは `impl-watch.log` だけで、状態ファイル・ロック・読み物には触らない（`cli_wait_tests.rs:232` が確かめる）。doc の「居る印を握るのがいちばん先」は状態について言っていて、振る舞いの食い違いは無い。
- **判断の核の決まり**（手順書 1 節の 1〜11）: 決まり 4（全部渡せるときだけ）・5（先の申し込みのために取っておかない・同点は行列の順＝安定な並べ替え）・6（待たれている間だけ・test だけの申し込みだけ）・7（止まった者には渡さない・止まった持ち主の机は空く）・8（候補が在る間はマージを出さない・バグ優先）・9（マージの持ち主と待ち手は止めない）・10（記録は停止要請を出さない・取り消しは出し直される）を、`grantable`・`grant_one`・`replan`・`need_stop` の各枝で照らし合わせた。食い違いは無い。`test,sole` の「sole は渡せるが test が他人に持たれる」は、状態の不変から起きないことも確かめた（読み手 B）。
- **回収**: 呼んだ識別は同じ呼び出しで回収しない・見張りを立て直すまでの印の在る間は回収しない・「作業中」でない参加者は回収しない（手順書 6 節 500・518 行目に限界として書いてある）。終わりかけの見張りの抹消の直前に再開が入る競り（`plan.rs:392-401`）にもテストが在る。
- **状態ファイルの読み書き**: 排他 → 時計 → 読み → 退避のログ → 判断 → 書き → 出来事のログ → 読み物、の順。書けなかった変化はログに渡さない・退避は前の退避を上書きしない・知らない版は読まず動かさず失敗・OS の読みの失敗を「壊れた」と見なさない・置き換えの失敗は一時ファイルを消す。どれもテストが在る（読み手 C）。
- **記録の無い失敗の道**: `time()`・`with_state`・`clear`・`read_only`・`fingerprint`・`say` は `error!` を出してから `Err` を返す。わざと失敗にしない道（読み物・周期の一回り・抹消・確かめられなかった探り）は `warn!` を残す。
- **ログと退避のファイルが増え続ける**: `impl-watch.log` は切り詰めない・退避のファイルは残る。手順書 7 節（589 行目）に「切り詰めはしない・大きくなったら、誰も走らせていないときに消してよい」と書いてあり、1 行は数百バイトで、想定した負荷では月に MB の桁。指摘にしない。
- **待ちのたびの周期の一回り**（30 秒ごと・待ち 1 つにつき 1 回）: 8 本×2 つの待ちで 30 秒に 16 回の短い排他。想定した負荷で問題にならない。
- **時刻の文字化**（`status.rs:387-407`）: うるう年・世紀・2038 年・2106 年でテストが在る。
- **1,000 行**: どのファイルも越えていない（最大 924）。
- **テストの置き場所と子プロセス**: 一時の根は全部 `TempPath::under_target`（`target\test-roots\`）・`tests/real/mod.rs` の `Desk::command` は必ず `AREKA_IMPL_WATCH_HOME` をその下の Windows 形の絶対パスで上書きする。常時テストは置き場所を引数で渡し、環境変数を変えない。1 秒の眠りは `refuse_poll` が panic に変えるので、常時テストは眠らない（例外は排他の競りを見る `concurrent_changes_from_several_threads_are_all_kept` の 10 ms の試し・上限 10 秒）。
- **`Cargo.toml`**: `serde`・`serde_json` の版をクレート側に書くのは、根の `[workspace.dependencies]` に無いからで、`crates/dola/Cargo.toml` と同じ形（根を grep して確かめた）。

### わざと置いた先送りの印（⑵ の候補）

- `shortcut:`・`ponytail:` の注釈、`#[allow(dead_code)]` の語彙の口は、範囲に 1 つも無い（grep）。⑵ に当たる指摘は無い。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今のコード・道具 | ギャップ |
|---|---|---|
| 1（洗い出し） | 本書 §1〜§4 | 済み（17 件・全 40 ファイル） |
| 2（討議で確定） | — | **Missing**: 討議で #1 の欄の扱い・#8 の ⑴ を決める（§7） |
| 3（直し切る） | 指摘の場所は全部 `crates/areka-impl-watch/` の中 | **Constraint**: #8 を直すなら `doc/impl-watch.md:576` の文も直す（範囲の外・C19 で触ってよい＝`crates/areka/` でも 6.3 のファイルでもない）。#1 で欄を消すなら外への約束（状態ファイル）＝要件 6.5 |
| 4（テストで縛る） | 判断の核は純粋で、偽の生死の口・偽の時計・偽の眠りがそろっている（`plan_test_support.rs`・`store_test_support.rs`・`wait_test_support.rs`） | バグ・危ない所の直しは #1（欄の無い参加者の `unstop`）・#8（取れなかった `try_lock` で書かない）にテストを 1 本ずつ。#2・#3 はテストそのものを足す指摘 |
| 5（守る決まり） | 1,000 行・`error!`＋`Err`・ASCII の文 | **Constraint**: `plan_desk_tests.rs` 924 行＝足すなら新しいファイルへ（#6 の直しで減る）。#7 の新しいモジュールは `main.rs` の読み込みの向きの doc に載せるか確かめる |
| 6（範囲の外・C19） | どの直しも `crates/areka/`・`Cargo.lock`・`runtime-module-untangle` のファイル・`vendors/` に触らない | **Constraint**: 外への約束を変える直しは #1 の欄を消す場合だけ（取らなければ 0 件） |
| 7（机で動いているもの） | 実機テスト `tests/real.rs` は置き場所を `target\` の下へ上書きする | 直しの確かめで手で走らせるときも、置き場所を `target\` の下へ上書きする。版を上げる直しは無い（#1 の欄を消して版を上げる案を取らない限り） |
| 8（仕上げの確かめ） | 全体テスト `tools/test-all.ps1`・実機テスト `cargo test -p areka-impl-watch --test real -- --ignored`（約 30 秒・排他テストの机が要る＝手順書 1 節の判定の基準） | かけ直しの audit・既定のブランチとの差分の確かめは検証のタスクで |

## 6. 実装の進め方の案

- **案 A（今のファイルの中で直す）**: 指摘ごとに今のファイルを直す。#7 だけはクレートのテストの支えのモジュールを 1 つ足すか、写しを 1 つのモジュール（例 `store_test_support.rs`）へ寄せて他から `crate::store::test_support::…` で読む（新しいファイルを足さない）。
  - ✅ 新しいファイルが無いか 1 つ・形が今のまま。❌ #7 を寄せる先が「口の支え」になり、`home_tests`・`presence_tests` から読む向きが層の向き（`home`／`presence` → `store`）と逆に見える。
- **案 B（クレートのテストの共通のモジュールを新しく置く）**: `src/test_common.rs`（`#[cfg(test)]`）に `T0`・`entries`・`names`・`home_in`・`read_state` を置き、各モジュールの写しを消す。入口・判断の核のテストの支え（#4〜#6）は、今の支えのファイルの中で寄せる。
  - ✅ 層の向きを崩さない・写しが 0 になる。❌ ファイルが 1 つ増え、`main.rs` の `mod` と読み込みの向きの doc・見張りの扱いを決める要がある。
- **案 C（組ごとに進める）**: 本番に触る 3 件（#1・#8・#9）とコメント（#17）を先に、テストの寄せ（#4〜#7・#10〜#16）を後に。#2・#3 のテストの足しは本番の直しと同じ段。
  - ✅ 本番の振る舞いの直しとテストの寄せが混ざらず、レビューしやすい。❌ テストの寄せの段で、本番の直しの段に足したテストもまた寄せ直すことがある。
- **規模の見込み**: S（1〜3 日の相当）。タスクは **10〜12 本**の見込み（例: #1 / #2＋#3 / #8 / #9 / #17 / #4＋#5＋#13＋#14 / #6＋#15＋#16 / #7＋#10＋#11＋#12 / かけ直しの audit / 全体テスト・実機テスト・差分の確かめ）。**20 本を越えないので、2 本目の spec へ切る要は無い**（要件 2.6）。
- **危うさ**: 低い。判断の核は純粋で偽物の口がそろい、本番に触る直しは 3 件で、どれも局所。#8 は排他の扱いを変えるので、`tests/real.rs` の同時の変更のテストを実機テストで回して確かめる。

## 7. 討議に渡す議題の種（要件 2）

1. **#1 の欄 `Participant::id` を消すか**: 推す案＝鍵で比べる直し（3 か所）だけにして、欄は残す（外への約束を変えない・C19 で取れる）。欄を消す案は状態ファイルの形が変わる＝要件 6.5 の議題（版 2 のまま消すと古い実行ファイルとの混在で `unstop --id` が当たらない・版を上げると置き方の道具の `clear` で走っている待ちと見張りが全部終わる＝要件 7.3）。
2. **#8（`status` が排他の外で `status.md` を書く）は ⑴ に当たるか**: 完了 `areka-P0-impl-watch` の design.md 567 行目と手順書 576 行目が「わかっていて受け入れた限界」として書いている。直す（`try_lock` で取れたときだけ書く・`doc/impl-watch.md:576` の文も直す）か、⑴ として残すかの裁定が要る。直しても外への約束は変わらない。
3. **#9（読み物の確定を外す）**: 速さだけの改善で、想定した負荷では効きが小さい。指摘は全部直す方針なので直す前提だが、`replace` に引数を足すか、読み物だけ別の書き方にするかは設計で決める。
4. **#7 の寄せ方**（§6 の案 A／B）: 新しいテストの共通のモジュールを置くか、今の支えのファイルへ寄せるか。
5. **#2 の足し方**: 名指しのテスト 1 本か、全探索の不変に「渡した机の組＝申し込んだ組」を 1 行足すか（後者は全探索の時間をほぼ変えずに、組の取り違え全般を捕まえる）。
6. 外への約束を変える指摘は、#1 で欄を消す案を取らない限り **0 件**。`crates/areka/`・`Cargo.lock`・C19 で `runtime-module-untangle` が書き換えるファイル・`vendors/` に及ぶ直しは **0 件**。範囲の外に及ぶのは #8 を直す場合の `doc/impl-watch.md:576` だけ（スキル・置き方の道具・`structure.md` には及ばない）。

**討議の結果（2026-11-13・開発者「承認」＝3 件とも推奨案で確定）**: 議題 1＝#8 は直す（`try_lock`・⑴ として残さない）。議題 2＝#1 は鍵で比べる直しだけ・欄 `id` は残す。議題 3＝着地の後に置き方の道具で置き直す（要件 8.8）。直さずに残す指摘は 0 件（要件 9）。

## 8. 調べが要るもの（Research Needed）

- **読み手の側の一瞬の失敗**: 待ちのループは、読み（`read_only`）と更新時刻の取得（`fingerprint`）の OS の失敗で待ちごと終了コード 1 で終わる（`wait.rs:269-271` が決めとして書く）。書き手の名前の付け替えは「読む側が開いている一瞬」のために 5 回試す（`store.rs:356-365`）が、逆向き（書き手が名前を付け替える一瞬に読み手の `fs::read`／`fs::metadata` が失敗するか）は確かめていない。今の Rust の `fs::rename`（Windows）が POSIX の意味の付け替えを使うかで答えが変わる。想定した負荷で起きると確かめられたら指摘になる（今は具体的な場合を作れていないので指摘にしない）。
- **#7 の新しいモジュールと見張り**: `main_layering_tests.rs` の読み込みの向きの表（`main.rs:7-9` の doc）に `#[cfg(test)]` のモジュールをどう扱うか（表に載せない・載せる）。
- **#8 の `try_lock`**: `state.lock` を取った `status` が中で読み直すと、`status` の「排他を取らない」（完了 spec の要件）に触れるかを、完了 `areka-P0-impl-watch` の requirements.md で確かめる。
