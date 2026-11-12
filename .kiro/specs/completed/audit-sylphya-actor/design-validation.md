# 設計の検証: audit-sylphya-actor

- 日付: 2026-11-06
- 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C18 の行）
- 照らし合わせたコード: `crates/areka/src/write_failure.rs`・`alert.rs`・`boot_resolve.rs`・`crates/areka-sylphya/src/persist/io.rs`・`crates/areka-nar/src/install.rs`・`crates/log-capture-kit/tests/temp_path_guard_test.rs`・`with_default_guard_test.rs`・`crates/log-capture-kit/src/capture.rs`・完了した `areka-P0-sylphya` と `areka-P0-write-failure-notice` の要件

## まとめ

設計は 22 件の扱いを 1 件ずつ、直し方・縛るテスト・タスクの束まで決めていて、このまま実装に進める細かさがある。コードと照らし合わせた主な主張（範囲の外で 0 か所の削除・既定つきの口の足し算・錠の形・一時のパスの入口に触れないこと）は、どれも事実と合っていた。ただし、#2 の縛るテストが範囲の外の番人のテストを赤にする点が、設計の「範囲の外のファイルは 1 つも変えない」と食い違っている。これと、#1 の帰結を開発者に確かめる点の 2 つを、設計の討議で決める必要がある。

## 確かめた事実（頼まれた 4 点）

### (a) `PersistOutcome::Degraded` を `crates/areka` がどう扱うか

- 起動の前（`boot_resolve.rs` の `write_session_mark` ほか）は `Err(failure)` を返し、`write_failure::stop_boot` が記録 `memory_write_failed` を残して告知（ダイアログ）を出し、起動を止める。走っている間は `write_failure::fatal` が `quit_app` で areka を終える。設計の「失敗の扱い」の書き方は事実どおり。
- 「メッセージボックスは無粋・失敗は emo2 の台詞で」（09-26）とはぶつからない。書けないことを致命にしてダイアログで告げるのは、完了した `areka-P0-write-failure-notice` の要件の討議・議題 1（10-16・開発者「致命的では？ダイアログだと思いますが。」）で決まった形で、09-26 の決まり（終わるときの最後の手段）に収まっている。この spec は告知の仕組みを変えず、理由の文を渡すだけ。
- ただし、完了した `areka-P0-sylphya` の要件 6.7 とは字面でぶつかる（下の重要な問題 1）。

### (b) #3 の錠と、`temp_path_guard_test.rs`・`areka-nar` の棚の錠

- `temp_path_guard_test.rs` が探すのは一時のフォルダの入口の 2 つの綴りだけ。錠のパスは宛先から組み、`temp_dir` も `TempPath` も使わないので当たらない。今の `io.rs` にも入口は無い（テストの `TempPath` 3 か所だけ）。要件 6.3 は守られる。
- `areka-nar/src/install.rs` の `open_shelf_lock` は `read`・`write`・`create`・`share_mode(0)`・`custom_flags(FILE_FLAG_DELETE_ON_CLOSE)`、定数は `0x0400_0000` を値で書く。設計の形はこれと同じ。違うのは、棚の錠が待たずに断るのに対し、こちらは 10 ms ごとに最大 1 秒取り直す点で、理由（記憶の保存は断ると致命になる）も筋が通っている。

### (c) 「範囲の外で 0 か所」の削除

- `flat_global`・`is_all_absent`・`PersistPut`・`KeyParseError`・`persist_outcome`・`ledger_key_determinism`・`spike_ui_pump` は、3 つのクレートの外（`crates`・`tools`・`doc`・`.claude`）で 0 か所だった。
- `SylphyaMsg` は `crates/areka/src/placement/zorder_property_deferral_tests.rs` が使うが、組むのは `SylphyaMsg::Set` だけで、`PersistPut` の欄を消しても壊れない。`MirrorImage` は外の 3 ファイルが `MirrorImage::empty()` で使うだけで、欄の字面の組み立ては無い。
- `run_bounded` は外にも同じ名前の関数が 5 つあるが、どれも別の関数（`spawn.rs` の非公開のものとは無関係）。
- 外から 3 つのクレートのファイルをパスの文字列で読む物（`ukadoc-survey` の `vocab/dotted.rs`・`zorder_property_deferral_tests.rs` の `vocab/` の公開の定数の名簿）は、どれも消す・変える物に当たらない。

### (d) `PersistIo` の既定つきの口

- 3 つのクレートの外にある `PersistIo` の実装は 19 個（設計は 20 と書くが、数え方の違い程度）。どれも `read`・`commit`（一部 `withholds`）だけを書いている。既定つきの口を足すだけなので、書き直さずにコンパイルが通る。`lock_for_update` という名前は、今はツリーのどこにも無い。

## 重要な問題（2 件）

### 問題 1: #1 の帰結が、完了した `areka-P0-sylphya` の要件 6.7 と字面でぶつかる

- **何が問題か**: 完了した `areka-P0-sylphya` の要件 6.7 は「永続化に起因するいかなる失敗（保存失敗・読取失敗・**形式異常**）が発生しても…ゴーストの起動・実行を停止させない」と書いている。#1 の直しの後は、形式の異常（壊れた TOML・版の違い）と読みの失敗が `Degraded` になり、`crates/areka` の今の扱いで起動が止まるか、areka が終わる。要件の討議の ⑴ の確かめ（`research.md` §3 の #1）は要件 6.3・6.4 だけを挙げ、6.7 に触れていない。後の `areka-P0-write-failure-notice`（10-16）が「書けない＝致命」と決めたのは書き込みの I/O の失敗で、読めない物を守るためにわざと書かない場面まで含めたかは、記録からは読めない。
- **なぜ効くか**: 要件 2.4（裁定を覆す直しは直さずに議題にする）に当たるかもしれない。利用者から見ると、手で直し損ねた 1 文字や、ウイルス対策が一瞬つかんだ読みの失敗で、ゴーストが起きない・走っている途中で落ちる、に変わる。さらに、告知の決まった行「直し方: この置き場を書ける状態にしてください。…」（`alert.rs` の `MEMORY_WRITE_FIX`）は、この場面では外れている（置き場は書ける。直すのはファイルの中身）。
- **どう直すか**: 設計の討議で、要件 6.7 を名指して、この帰結でよいかを開発者に確かめる（`research.md` §10.5 の 1 に、根拠として 6.7 を足す）。よいと決まったら、⑴ 理由の文に具体的な直し方（「sylphya.toml を直すか、別の名前へ退けてから起動し直してください」）を入れて、決まった行の外れを埋める。⑵ 読みの失敗（`Err(e)`）は一時の失敗のことがあるので、`Degraded` にする前に短く取り直すかを一緒に決める。
- **要件**: 2.2 ⑴・2.4・3.1・5.2
- **設計の場所**: 「#1 使えない記憶のファイルを上書きしない」・「失敗の扱い」の「利用者から見た結果」

### 問題 2: #2 の縛るテストが範囲の外の番人を赤にし、「範囲の外の変更は 0」と食い違う

- **何が問題か**: 新しい `crates/areka-actor/tests/ui_span_test.rs` は、自前の受け手を `tracing::subscriber::set_default` で据える。ところが `crates/log-capture-kit/tests/with_default_guard_test.rs` は、`crates/` の下の `.rs` を全部読み、`set_default(`・`with_default(`・`set_global_default(` の素の呼び出しが、共有のクレートの `src/` と例外の表（5 件・件数の定数 `ALLOWED_DIRECT_CALLS_COUNT = 5`）の外に 1 件でもあれば赤にする。`log-capture-kit` の `capture` の受け手は区間を数えない（`new_span` は固定の番号・`enter`／`exit` は何もしない）ので、番人に従ったまま区間の出入りを見る道は今のツリーに無い。
- **なぜ効くか**: このままでは全体テスト（要件 7.4）が赤になる。直すには `with_default_guard_test.rs` の例外の表に 1 行足し、件数を 6 にする必要がある。これは設計の「範囲の外のファイルは 1 つも変えない」と、T11 の確かめ「`git diff --stat … -- crates/log-capture-kit/tests/` が空」に反する。別名で呼んで番人をすり抜ける形は、番人の目的を壊すので採れない。
- **どう直すか**: 設計の討議で扱いを決める。推奨は「例外の表に 1 行（理由: 区間の出入りを数えるには受け手を据えるしかなく、共有の捕捉は区間を運ばない）を足し、件数を 6 にする」。このファイルは `placement-crate-split` が触る `log-capture-kit` のテスト 2 つ（`file_length_guard_test.rs`・`temp_path_guard_test.rs`）には入らないので、要件 6.2 には当たらない。決めたら、設計の「ファイルの構成」・「範囲の外」・「要件の対応」の 6.7 の行・T11 の差分の確かめ（`crates/log-capture-kit/tests/` を、触ってよい 1 ファイルを除いた形に狭める）を直す。範囲の外のファイルに手を入れるので、要件 6.6 のとおり、そのファイルを持つセッションが走っていれば申し合わせる。
- **要件**: 4.1・6.2・6.6・6.7・7.4・7.5
- **設計の場所**: 「#2 UI アクターの記録の区間」の縛るテスト・「ファイルの構成」・「仕上げの確かめ」の 4

## 軽い気づき（重要な問題ではない）

- #3 の「プロセスの中の待ちも有界」は少し言い過ぎ。プロセスの中の錠で待つ時間は、先の書き手のファイルの錠の待ち（最大 1 秒）に、その書き手の読みと書き（`sync_all` を含む・上限なし）を足したもの。UI スレッドの直の保存（`boot_resolve.rs`・`roots/edit.rs`）が、書き手のアクターの遅い書き込みを待つことがある。言い方を直せば足りる。
- 外の `PersistIo` の実装の数は、今の測りで 19（設計は 20）。

## 設計のよい所

1. **公開の形の約束を、コードの事実で裏づけている。** 消す 3 つは外で 0 か所、足す 3 つは既定つきの口か新しい名前で、外の 19 の実装も呼び手も書き換えずに済む。錠は `areka-nar` の棚の錠と同じ形で依存を足さず、一時のパスの入口にも触れない。今回照らし合わせた範囲では、どれも事実と合っていた。
2. **縛るテストが「直しを外すと赤」で組まれている。** #1・#3・#4・#5 のどれも、直しを外したときに何が起きて赤になるかを書き、門で順を作って決定論にしている。待ちが止まる形を、期限つきで待って赤で終わる形に直してあるのもよい。

## 判定

- **判定: GO（条件つき）**
- **理由**: 根本の形のずれは無く、要件はすべて受け持ちが決まっていて、実装の道もはっきりしている。2 つの問題はどちらも小さく閉じた直し（設計の文の直しと、番人の表の 1 行）で、設計の討議で決めれば済む。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-sylphya-actor`）で、問題 1（要件 6.7 との関係と理由の文）と問題 2（番人の例外の表に 1 行足すこと）を決める。
  2. 決まった形を `design.md` に反映する（ファイルの構成・範囲の外・要件の対応・T11 の差分の確かめ）。
  3. `/kiro-spec-tasks audit-sylphya-actor` でタスクを作る。
