# 設計の検証: areka-P0-write-failure-notice

> 2026-10-16・本ブランチ（main `0bb51d42` の上に要件と設計を積んだ所）のコードと、`areka-P0-user-data-root` のブランチ `claude/areka-p0-user-data-root-12e486`（`1f5844b5`）の design.md に照らして検証した。対話をせずに出した結論（非対話の検証）。design.md・requirements.md・research.md・spec.json は直していない。

## 1. まとめ

設計は、今のコードの形（`quit_app` と `FirstExit`・`run()` の後の後始末・告知の場面・`boot_with_kanade_stop` の派生の前例）にそのまま乗り、書き込みの失敗を「書き手を起こすときに渡す受け口」と「直の書き込みの戻り値」の 2 か所で拾って 1 つの机へ集める。コードで確かめた主張はすべて合っており、実装へ進める（GO）。ただし、隣の `user-data-root` との接続 1 点、OS のセッションの終了の扱い 1 点、要件 7.2 の「写しの後」の判定の届き方 1 点は、設計の討議で決めておく必要がある。

## 2. コードで確かめた主張

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| 投函の書き込みは 9 か所 | `persist_put` の呼び手: `boot_resolve.rs`（4）・`emo2_boot/ghost_switch.rs` の `record_steady_memory`（1）・`install/desk.rs`（1）・`placement/persist.rs`（1）・`roots/edit.rs`（1）・`areka-ghost/src/prop_sink.rs`（1） | 合う（9） |
| UI スレッドの直の書き込みは 4 つ | `save_scope` の直の呼び手は `boot_resolve.rs` の `save_app`（`write_session_mark`・`write_switch_drop`・`clear_session_mark` の 3 つが使う）と `roots/edit.rs` の `apply` | 合う（4） |
| `boot_with_origin` の本番の呼び口は 2 か所 | `ghost_session.rs`（倒れ先）・`emo2_boot/mod.rs`（`wire_emo2_boot`）。どちらも `world: &mut World` を持つ | 合う |
| `spawn_ghost_sylphya` は `boot_with_origin` の中で呼ぶ | `areka-ghost/src/runtime.rs` | 合う |
| App スコープだけの書き手は投函を持たない | `shiori_host.rs` の `spawn_sylphya` の 2 か所に `persist_put` は 0 | 合う |
| `PersistSave` を出すのは `PersistPut` だけ | `areka-sylphya/src/actor.rs` の `SylphyaCore` | 合う。返信の既定 `unwrap_or(PersistOutcome::Degraded)` は形が変わるので直す所（設計に書いてある） |
| `PersistKey` は 9 種・`PersistScope` は 4 種 | `areka-sylphya/src/persist/mod.rs` | 合う（`remembered_text` の網羅の表と一致） |
| `ExitOrigin` は今 7 種・`fault_of` は網羅の match | `app_exit.rs` | 合う。`AlertScene` と `ExitOrigin` は `Clone + PartialEq + Eq` なので、`PersistFailure` の derive と食い違わない |
| 告知は今 5 場面・題名「areka を起動できません」 | `alert.rs` の `AlertScene` と `TITLE` | 合う |
| `session_mark_verdict` の「きれいな終わり」の群 | `main.rs`（`Quit` 系・`Escape`・`Smoke`・`OsClose`・`SessionEnd`） | 合う |
| `IoPhase` は 4 種・`Read` は `.nar` を読む所だけ | `areka-nar/src/error.rs`・`lib.rs`（`Read` は `read_archive_bytes` だけ。`Stage` は作業フォルダと組み上げ。中身はすでに読み終えている） | 合う。振り分けで「書庫の破損を `fileio` と取り違える」道は無い |
| 根が無いインストールは `IoPhase::Stage` | `install/destination.rs` | 合う |
| `main.rs` は 971 行 | `wc -l` | 合う（足す量の見積もり約 +14 行で 985 行前後） |
| 「書きました」の文を照らすテストは 0 | テストと `crates/areka/tests` を「書きました」「書き換えました」「Ref6/7 付き」「前回落ちたとして」で検索 | 合う（0 件） |
| 書き手のアクターは降ろすときに join される | `runtime.rs` の `sylphya_handle.join()` | 合う（降ろす途中の失敗は受け手に残り、`take_receiver` で拾える） |
| `AREKA_PROFILE_DIR` は `default_app_profile_dir()` が先に読む | `boot_config.rs` | 合う（実機の方向 ⑤ の作り方が成り立つ） |

型合わせの一覧の漏れ（重くない）: `areka-ghost/src/sylphya_wiring.rs` の中のテスト（`spawn_ghost_sylphya(ScopeRoots::default())`）も引数を足す。同じファイルを直すので、タスクの段で拾えば足りる。

## 3. 重い問題（3 件）

### 重い問題 1: `user-data-root` との接続 2 点が、向こうの生成済みの設計と食い違っている

- **問題**: ⑴ この設計は `roots/edit.rs` の `apply` を「戻り値 `()` のまま、書けなかった結果は中で閉じる」と決めたが、向こうの design.md は「着地の後の `apply` の戻り値（書けなかった結果を返す形）をそのまま返す」（向こうの要件 3.9 の行・約束の外で触るファイルの表）と書いている。⑵ 書けるかの確かめについて、この設計は「`write_failure::probe_dir`（消えるファイルを作って閉じる）1 つにし、先に着地した方の物を後の方が使い、2 つ作らない」と書くが、向こうは葉のクレート `areka-user-data` に「何も作らない（ハンドルを開いて閉じるだけ）」別の確かめ（`exe_dir_writability`）を置く設計で、中継（`areka-mcp-bridge`）からも使うため `areka` の中の部品は使えない。確かめ方（ファイルを一瞬作るか・作らないか）と置き場（`areka` か葉のクレートか）が両方で違う。
- **影響**: このまま両方を実装すると、確かめの部品が 2 つになり、同じフォルダに対して「書ける」「書けない」の答えが割れうる（例: 向こうは権限だけを見て通し、こちらは作れずに止める）。`apply` は、向こうが最後のタスクで「戻り値を返す」形を探して空振りする。
- **提案**: ⑴ `apply` の形（`()` のまま・中で `fatal` へ渡す）は今の設計でよい。進行役が向こうのセッションへ「`apply` は `()`。呼び手は戻り値を受けず、書けなかった結果は致命の道へ渡る」と送り、向こうの要件 3.9 の行を合わせてもらう。⑵ 確かめの部品は、どちらに置くかを討議で 1 つに決める。案: 共有できる葉のクレート（`areka-user-data`）に「無ければ在るいちばん深い親を選ぶ」確かめを 1 つ置き、こちらは `write_failure::probe_dir` をその薄い呼び出しにする。確かめ方は、要件 2.9（作って残さない）を満たしつつ読み取り専用の媒体も拾える「消えるファイル」の形に寄せるのがよい。こちらが先に着地するなら、向こうが葉のクレートへ移す、と順を決める。
- **要件**: 2.1・2.9（`user-data-root` との合意）・Boundary Context「Adjacent expectations」
- **設計の場所**: 「`user-data-root` への引き渡し: `roots/edit.rs` の `apply` の形」の「確かめの部品」・Components「致命の机」の `probe_dir`

### 重い問題 2: OS のセッションの終了の中で書けなかったとき、要件の「0」から外れる道が、要件に無い例外として残っている

- **問題**: 設計の Error Strategy は「OS のセッションの終了の中 → 告げない（今どおり）・失敗の記録は今どおり」とし、`take_receiver` も OS の終了を処理済みなら `None` を返す。`session_end.rs` の `settle_session_mark` の呼び出しは戻り値を使わない（設計も「直さない」と書く）。この道では、印を消せなかった失敗も降ろす途中の失敗も、告知へ渡らず、新しい記録 `memory_write_failed` も出ず、終了コードも決めていない。
- **影響**: 要件 1.1 は「どの書き込みでも告知へ渡す・記録だけで終わる失敗の道を 0」、要件 2.3 は「終わるときに印を消せないときは告知」と書いており、OS の終了の例外を持たない。告知を出さないこと自体は今の決まり（告知は OS の終了を塞ぐ）に合うが、要件にその例外が無いまま設計だけで外すと、検証の段で「要件 2.3 を満たしていない」と読まれる。また、この道だけ `memory_write_failed` が出ないと、記録を引く道具が見落とす。
- **提案**: 討議で次を決める。⑴ OS のセッションの終了の中では告知を出さない、を要件 2.3 の例外として要件へ書き足す（今の SHIORI の失敗と同じ扱い）。⑵ その道でも `memory_write_failed` を 1 件ずつ残す（`session_end.rs` で `settle_session_mark` の 2 つ目と受け手の残りを記録する 2〜3 行。`write_failure` に記録だけの小さな口を置けば `session_end.rs` の行は増えない）。⑶ 終了コードはこの道では問わない（OS が終わらせる）と書く。
- **要件**: 1.1・1.2・2.3・3.4
- **設計の場所**: Error Handling「Error Strategy」の最後の行・Components「致命の机」の `take_receiver`・File Structure Plan「Modified Files」の `session_end.rs` の注

### 重い問題 3: 要件 7.2 の「確かめは `user-data-root` の写しの後に走る」を、実機の方向 ⑤ では確かめられない

- **問題**: 設計は、確かめが `resolve_boot`（写しを含む）の後に走る並びを「実機の方向 ⑤ が受け持つ」とし、単体の `check_boot_dirs` のテストは「渡された置き場しか見ない」ことだけを判定する。ところが方向 ⑤ は `AREKA_ROOT` に検体の根を渡す形で、向こうの設計（要件 3.4・`listed_user_root()`）は「`AREKA_ROOT` が在るときは書ける根を足さず、写さない」。つまり方向 ⑤ では写しが起きず、「写しの後に確かめる」並びは通らない。
- **影響**: 要件 7.2 の判定の 1 つ（写す前の元の置き場を見ないこと）を、どのテストも実際には赤にできない。後で誰かが確かめの呼び出しを `resolve_boot` の前へ動かしても、テストは緑のまま。
- **提案**: 並びそのものを判定する検査を足す。案 ⑴: `main.rs` の本文を `include_str!` で読み、`resolve_boot(` の後に `check_boot_dirs(` があり、`WinApp::with_exit_policy` より前にあることを確かめる構造の検査（このリポジトリに前例がある・兄弟のファイル）。案 ⑵: 確かめの入口を「`resolve_boot` の結果（`ConfigInputs`）を受け取る関数」にして、型で「写しの後の値しか渡せない」形にする。どちらでも `main.rs` の行は増えない。方向 ⑤ の説明からは「写しの後の並びもここで通る」の一文を外す。
- **要件**: 7.2・2.1
- **設計の場所**: Testing Strategy「単体（areka）」の `check_boot_dirs`・「実機（要件 7.7）」の最後の文

## 4. 良い所

- **失敗の拾い方が 1 本にまとまっている**: 書き手のアクターに受け口を「起こすときに」渡す形（`spawn_sylphya_with_failures`・`boot_with_origin` の引数）にしたので、投函の 9 か所にも talk のスレッドの `prop_sink` にも手を入れずに、起動のトークの最中の書き込みまで取りこぼさない。`SylphyaInit` と `GhostBootOptions` に欄を足さない（構築点を変えない）のも、`boot_with_kanade_stop` の前例どおりで、変更の範囲が小さい。
- **今の終わり方の形をそのまま使っている**: 途中の失敗は `quit_app` → `FirstExit`（最初の出所だけ）→ `run()` の後の後始末 → 告知、という SHIORI の失敗と同じ道に乗り、新しい出所 1 つと場面 1 つを足すだけで「告知は 1 回だけ」（要件 2.5）が今の仕組みで保たれる。インストールの語も `IoPhase` の網羅の match で振り分け、`areka-nar` に触れない。

## 5. 判定

- **判定: GO**
- **理由**: 今の層の境と終わり方の形に合っており、要件 1〜9 の各項目に部品と判定のテストが当たっている。コードで確かめた主張に誤りは無い。上の 3 件は、どれも部品の形を変えずに討議で決めて書き足せる（向こうのセッションとの合わせ・要件の例外の書き足し・並びの検査 1 本）。
- **次にすること**:
  1. `/kiro-design-discussion areka-P0-write-failure-notice` で上の 3 件を決める（重い問題 1 は `user-data-root` のセッションと合わせる）。
  2. 決まったことを design.md（と、重い問題 2 を例外として足すなら requirements.md）へ反映する。
  3. `/kiro-spec-tasks areka-P0-write-failure-notice` でタスクを作る。タスクの段で、`sylphya_wiring.rs` の中のテストの型合わせと、`main.rs` の行数（上限 1,000）の見張りを拾う。
