# 設計の検証: startup-install-option

> 2026-10-29・本ブランチ `a92c271e`。`kiro-validate-design` の手順（分析 → 大事な問題 → よい所 → GO／NO-GO）で、`design.md` の主張を今のコードで確かめた。設計・要件・research は直していない。

## まとめ

設計は、判断を新しい `install/startup.rs` へ寄せ、今あるファイルには呼ぶ行だけを足す形で、同じウェーブの約束（`ghost_session.rs`・`ghost_switch.rs` に触らない）と 1,000 行の上限と「依存を足さない」を守っている。設計が拠り所にしたコードの事実は、下の表のとおりすべて今のコードと合っていた。残る問題は、取り消せない消去のテストの穴 1 つと、記録と振る舞いの書き漏らし 2 つで、どれも設計の討議で数行の足しで直せる。**GO**。

## コードで確かめたこと

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| kanade は `basewareversion` の答えの後に定常到達の知らせを 1 件出す（本物の sink か `LogSink` かに関わらない） | `crates/areka-kanade/src/schedule/boot.rs` 125〜135 行目（`Phase::BootVersion` + `Notified` → `Action::Notice(KanadeNotice::Steady)`）。知らせの受け口 `KanadeNoticeRx` は本物の結線と `LogSink` の起動で同じ 1 つ（`emo2_boot/frame.rs` 171〜173 行目・`emo2_boot/mod.rs` 702 行目） | 合う |
| 初回の起動の定常到達も `ghost_switch::on_notice` → `install::desk::on_steady` を通る | `emo2_boot/frame.rs` 205〜218 行目（`Stopped` 以外を `on_notice` へ）・`ghost_switch.rs` 491〜519 行目（予約の有無に関わらず腕の末尾で `install::desk::on_steady`） | 合う |
| 窓口は最初の定常到達より前に在る | `install::register`（`install/mod.rs` 123〜125 行目）は `ghost_session::register_systems` から、`app.run()` の前に呼ばれる | 合う |
| `DeskAsk::OrderDone` を形で読む所は 4 か所 | `desk_tests.rs` 250・262 行目、`worker_tests.rs` 344・398 行目。ほかは `desk.rs` 292 行目と `worker.rs` 55・71 行目だけ（`update` の `OrderDone` は別の型） | 合う |
| `parse_tests.rs` の書き換えは 4 か所 | 111〜118 行目（2 行）・285〜287 行目・438〜444 行目・512〜527 行目。578 行目は `ParseNote` を直接作るだけで表に依らず、直さなくてよい。`callghost`・`deletesource`・`"--install"` を読むテストはほかに無い | 合う |
| `InstallOrigin` で分かれる `match` は無い | `install/` に `match origin` の分岐なし | 合う |
| 入れた先は `InstalledItem::place`（フォルダの絶対パス）・`kind` は `"ghost"` | `procedure.rs` 443 行目（`element.path.display()`）・`judge.rs` 140〜147 行目 | 合う |
| `run_order` は `Abandoned` のところで並びを短くして返す | `procedure.rs` 119〜127 行目 | 合う |
| 起動中のゴーストへの上書きは、展開を終えてから背景のスレッドへ返る | `overwrite.rs` の段（預かった → 頼んだ → 展開した → 定常到達で返す）・`procedure.rs` 239〜245 行目。`OrderDone` は展開の後に来るので、`deletesource` が展開の前に書庫を消すことは無い | 合う |
| メニューの「ゴースト」の頼み方 | `menu/ghost_frame.rs` 53〜58 行目（`GhostSpec::Folder`・`raise_event: true`・`ChangeOrigin::Manual`・`boot_event: None`） | 合う |
| exe の本当の場所の隣 | `roots/user_root.rs` 159 行目 `exe_dir()` → `boot_config::exe_location()`（リンクを辿る・1 回だけ解く） | 合う |
| `Drop` の記録は、ふつうの道でもテストでも余計に出ない | 本番: 最初の定常到達で `pending` を取り出すので、ふつうに動いた走りでは出ない。`--help`・`--offline-dump` は持ち物を作る前に戻る（`main.rs` 177〜183 行目）。`main.rs` で `process::exit` を使うのは `--offline-dump` だけ。起動の解決の失敗は `main` の `return Err`（210〜219 行目）で、ローカルの持ち物が落ちて 1 件出る。テスト: 今のテストの World に `StartupInstall` は無い。`smoke_exit::arm` は World を `Weak` で持つだけで、World が落ちるのを妨げない | 確か |
| 行数 | `main.rs` 981・`ghost_switch.rs` 926・`ghost_session.rs` 935・`desk.rs` 560・`parse.rs` 645・`desk_tests.rs` 766・`worker_tests.rs` 532・`parse_tests.rs` 674 | 合う（足しても上限の内） |
| 試験の部品 | `vendors/sample_ghost/konnoyayame.nar` 820,368 バイト・`temp-path-kit` の `TempPath::under_target`・`log-capture-kit`・`real-run-kit` は在り、`crates/areka` の開発用の依存に入っている | 合う |
| `dist/README.txt` の 62・79 行目 | 62 行目が「まだ扱わない」の文・79 行目が readonly の「断る物」 | 合う |

## 大事な問題（3 つ）

### 問題 1: 消すのは起動の依頼の書庫だけ、をテストで固定していない

- **何が**: `on_order_done` は「出どころが `StartupArgs`・`InitialInstall` でなければ何もしない」と書いてあるが、テストの段（結線）に、メニュー・台本・窓への投げ込みの依頼が `Installed` で終わっても書庫が消えないことを確かめる行が無い。`StartupInstall` が World に在り、起動の引数の控え（`deletesource` 真）が残っているときに、ほかの出どころの `OrderDone` が先に届く場合も無い。
- **なぜ困るか**: 消去は取り消せない。出どころの見分けを 1 行でも取り違えると、利用者が窓へ投げ込んだ NAR が黙って消える。決定論のテストで全部の判断を固定する、という決まりにも届かない。
- **直し方**: `desk_startup_tests.rs` に 2 行足す。⑴ 控えが残っている World で `Menu`・`Script`・`WindowDrop` の `OrderDone { ends: [Installed] }` を 1 つずつ送り、写しが消えず、控えも残ること。⑵ `InitialInstall` の `OrderDone` は `exe_dir` の隣のパスだけを消し、控えの旗（`delete_source` 偽でも）に左右されないこと。
- **要件**: 3.5・3.6・9.3・9.4
- **設計の所**: 「StartupInstall」の「口ごとの振る舞い」の `on_order_done`、「Testing Strategy」の「結線」

### 問題 2: 終了で、起動の引数の依頼の後始末（消す・切り替えを頼む）を黙って捨てる

- **何が**: `app.run()` が戻ってから終了を始める（`main.rs` 337〜348 行目）。その後は `drain` が回らないので、積んだ・走っている起動の引数の依頼の `OrderDone` は捌かれない。設計は「消さない（安全側）」と決めているが、`discard_for_exit` が記録するのは `pending`（まだ出していない）だけで、出した後の控え `args_followup` が残っているとき（`deletesource`・`callghost` が立っている）は何も記録しない。手続きの記録には「入った」（`install_done`）が残るのに、書庫が消えなかった理由がどこにも出ない。
- **なぜ困るか**: 要件 7.2 は「この spec で足す道の失敗を、どれも理由の分かる記録に残す」。試験で「入ったのに NAR が残った」ときに、記録から追えない。
- **直し方**: `startup::discard_for_exit` で `args_followup` も取り出し、`delete_source` か `call_ghost` が立っていれば warn 1 件（例 `startup_followup_dropped`・`reason=exit`・語と書庫の並び）を残す。結線のテストに「依頼を積んだ後に終了の後始末を呼ぶと 1 件出て、その後に `OrderDone` を送っても消さず頼まない」を足す。
- **要件**: 3.1・7.2・2.8
- **設計の所**: 「StartupInstall」の `discard_for_exit`、「Open Questions / Risks」の 2

### 問題 3: `callghost` とゴースト自身の切り替えの、ふつうの順の扱いが設計に書かれていない

- **何が**: research の調べ 2 で、ゴーストが `OnInstallCompleteEx` の応えに `\![change,ghost,lastinstalled]` を書いたとき、多くは `OrderDone` が先に UI スレッドへ届くと分かっている。そのときは `callghost` が先に受け付けられ、ゴーストの切り替えは入口で `ghost_switch_busy`（warn）になり、送り出す側には応えのトークの途中で `OnGhostChanging`（`manual`）が届く。`design.md` の「Open Questions」には A → B → A の見分けしか無く、この「ふつうの順」が書かれていない。テストにも無い。
- **なぜ困るか**: 要件 5.3 は「ゴーストの選択を上書きしない」。ふつうの順では、切り替え先が同じでも、ゴースト自身の切り替えが warn で捨てられ、`OnGhostChanging` の Reference1 が `manual` になる。実機の試験（要件 9.8）で、この warn が毎回出る見込みがある。どちらが先かは決まらないので、決めを文書に残さないと、実装とレビューで読み方が分かれる。
- **直し方**: 設計の討議で次を決めて `design.md` に書く。推す形は「今の形のまま（待ちを置かない）。`OrderDone` の後に来たゴーストの切り替えは『依頼の手続きの間』の外として扱い、warn `ghost_switch_busy` は今の入口の記録のまま受け入れる」。そのうえで決定論のテストに 1 行（`callghost` を頼んだ後に台本の切り替えが来ると `Busy` になり、`callghost` の頼みは 1 回だけ）を足し、説明書の `callghost` の項に「ゴーストが自分で切り替えるときは、そちらより先に切り替わることがある」と書く。
- **要件**: 5.3・5.7・9.5・9.8
- **設計の所**: 「`callghost` の判断」の流れ図、「`decide_call_ghost` の決まり」、「Open Questions / Risks」の 1

## よい所

1. **約束を守る形の選び方**: 定常到達を、今ある `install::desk::on_steady`（どの定常到達でも呼ばれる口）で受け、持ち物を `Option::take` で 1 回だけ取り出す。これで `ghost_session.rs`・`ghost_switch.rs` に 1 行も触らず、時刻の待ちも置かずに「最初の定常到達で 1 回だけ」が状態の形で決まる。初回の起動・`LogSink` の起動でも同じ道を通ることをコードで確かめてある。
2. **取り消せない消去を純粋な関数 1 つに閉じた**: 消す判断は `sources_to_delete(archives, ends)` だけで決まり、終わり方の 5 つ・短い並び・同じパスの重ねを 1 つずつテストで固定する。`deletesource` と `initial_install.nar` が同じ関数を使い、消す道具は `remove_file`（フォルダは消せない）だけ。readonly・受け付けられなかったときは控えを置かないので、消す所に届かない。

## 判定

- **判定**: **GO**
- **理由**: 既存の仕組みとの食い違い・要件の大きな穴・行数と並走の約束の破れは無い。上の 3 つは、テスト 2〜3 行・記録 1 件・文書の決め 1 つの足しで、形を変えずに直せる。
- **次にやること**: 設計の討議（`/kiro-design-discussion startup-install-option`）で問題 1〜3 を議題にし、決めを `design.md` に入れる。その後 `/kiro-spec-tasks startup-install-option`。
