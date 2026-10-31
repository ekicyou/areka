# 設計の検証: popup-menu-residue

- 検証した日: 2026-10-31
- 対象: `design.md`（commit `93591a07`）・`requirements.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md` のウェーブ C14 の ⑦）
- 判定: **GO**

## まとめ

設計は、13 件の仕分けを 1 つの表にまとめ、直す 6 件（件 3〜6・8・9）を今あるコードの形の延長だけで片づける。新しい部品は失敗の種類の型 `ShowError` 1 つだけで、残りは記録の文の書き換えとテストと文書である。設計が頼るコードの事実は、下の「確かめた事実」のとおり今の main と合っていた。大きな食い違いは無い。直しておくとよい点が 3 つある（どれも実装を止めない）。

## 確かめた事実（設計の主張と今のコード）

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| `Opener::from_sender` はテストのビルドだけに在る | `crates/areka/src/readme/opener.rs` 165〜169 行（`#[cfg(test)] pub(crate) fn from_sender`） | 合う |
| `serve` は開く専用のスレッドの中身そのもので、受信端が閉じるまで回って戻る | 同 129〜133 行・`Opener::spawn` が同じ `serve` を呼ぶ | 合う |
| `opener` は `readme` の非公開の子で、`FakeOs` はテストのビルドだけ | `crates/areka/src/readme.rs` 218〜221 行（`mod opener;`・`#[cfg(test)] mod opener_test_support;`）・`FakeOs` は `#[derive(Default)]`・`calls: Vec<OsCall>` | 合う（`#[cfg(test)] pub(crate) use` の 2 行で見せられる） |
| `GhostSession::for_test` が在る | `crates/areka/src/ghost_session.rs` 582〜584 行（`#[cfg(test)]`・送出端と根だけ）・`GhostSlot` は `pub(crate)` | 合う |
| `boot_runtime` は `wire_readme` まで済ませ、`GhostSlot` と `Opener` は据えない | `emo2_boot/runtime_boot.rs` 220〜222 行・`Opener::spawn` を呼ぶのは `register_readme_drain` だけ | 合う |
| `dola` は 1 つの cue を全部の受け口へ配ってから次の cue へ進む | `crates/dola/src/cue/runtime.rs` 233〜244 行（cue ごとに `for sink in self.sinks.iter_mut()`） | 合う。受け口は同じスレッドで順に呼ばれるので、`Leave` を受け取った時点で説明書の要求は受信端に在る |
| 記録の捕まえはテストのスレッドだけ | `crates/log-capture-kit/src/capture.rs` 66 行（`with_default`） | 合う（取り出し 1 回の間だけ捕まえる 3.2 の数え方が成り立つ） |
| 説明書の受け口の記録は 7 行で、`event=` を持つのは 1 行だけ | `emo2_boot/readme_cue.rs`（`ReadmeCueSink:` が 7 か所・`open_external_rejected` だけ） | 合う |
| `show` は組み立ての失敗と表示の失敗を同じ型で返す・`ShowResult` を使うのは `trigger.rs` とそのテストだけ | `menu/win32.rs` 29〜69 行・`menu/trigger.rs` 397 行・`trigger_show_tests.rs` 153〜167 行（`exits()` は 4 つ） | 合う |
| `round_trip` の 3 行の文と関数の説明が「終了系列（Fault）へ」と書く | `crates/areka-kanade/src/actor.rs` 738〜771 行 | 合う |
| 照会の道は `round_trip_request` を通り、失敗を `ResourceOutcome::Failed` で返すだけ | `actor_resources.rs` 42〜47 行・89 行 | 合う |
| kanade の通しのテストの足場（`spawn_harness`・`without_boot_greeting`・`QuitPolicy::PerTalk`・`join_bounded`・`recorded()`） | `tests/kanade/common/` と `close_test_handshake_tests.rs` 165 行の無言終了のテスト | 合う。今の通しのテストはスコープ 0 だけで、期待値は `on_close`／`onclose_get_index` から作っている |
| 台帳の検査は空の `owner` を数えない | `crates/ukadoc-survey/tests/consistency/spec_checks.rs` 98 行（`if entry.owner.is_empty()` で飛ばす） | 合う。腕 c と腕 f は空の欄を見ない |
| `areka-P0-popup-menu-minimal` の `owner_count` は 10 で、4 行を空にすると 6 | 台帳を数えた: shiori 8・assets 1・sakura-script 1・property 0 | 合う（10 → 6） |
| 4 行の備考の古い段落（「本仕様の裁定」「2026-09-18 の時点で…」） | `doc/ukadoc-coverage/ledger/shiori.toml` 12468〜12515 行ほか | 合う |
| `spine.rs` のコメント「現在は 13 本」と `runtime_sinks` の `Box::new` の数 | `emo2_boot/spine.rs` 752 行・`runtime_boot.rs` の `runtime_sinks`（13） | 合う |
| 件 1・2・8 の確かめ済みのテストが在る | `trigger_flow_tests.rs` 677 行・`readme_tests.rs` 350 行・`trigger_wired_tests.rs` 44 行 | 合う |
| 渡し先の 2 本が生きている | `.kiro/specs/popup-menu-shell-look/brief.md`・`.kiro/specs/extra-character-windows/brief.md`（どちらも `completed/` の外） | 合う（渡す日の main でもう一度見る） |

### ウェーブ C14 の ⑦ との照らし合わせ

- C14 の ⑦ の行が挙げるファイルは `menu/{trigger,win32,captions}.rs`・`menu/trigger_show_tests.rs`・`emo2_boot/readme_cue.rs`・kanade の `actor.rs`（文だけ）・`actor_resources.rs` である。設計はこれに兄弟のテストと文書（`readme_cue_tests.rs`・`runtime_boot_tests.rs`・`readme.rs`・`actor_resources_tests.rs`・`close_test.rs`・新しいテストのファイル 2 本・網羅台帳・`roadmap-draft.md`・brief 2 本）を足し、`actor_resources.rs` は触らないと書く。足したファイルは、同じウェーブの兄弟 6 本が挙げる触るファイルと重ならない（① は `consumer_ledger*`・`sakura-script.toml`、② は `install/`・`boot_config.rs`・`main.rs`、③ は `placement/`・`ghost_session.rs`・`emo2_boot/{mod,assets}.rs`、④ は emo-atlas と wintf、⑤ は emo-text、⑥ は areka-actor と `log-capture-kit`）。
- 約束（件 11・12・13 をこのウェーブで取らない）は守られている。設計は 3 件を brief へ写すか、完了時に起票するだけで、メニューを開く所・表示・問う名前の表のコードに触らない（9.4）。
- 気にかけておく所: ③ の案 A は `ghost_session.rs` を、⑥ は `log-capture-kit` の `capture.rs` を触りうる。本 spec はどちらも読むだけ（`GhostSession::for_test`・`capture_lines`）なので、合流で文字の競合は起きないが、向こうが関数の形を変えるとビルドが止まる。合流の後のテストで気付ける。

## 直しておくとよい点（3 つ）

### 1. 件 13 を渡したことを記録する時と場所が、実装のタスクの外にある

- **困ること**: 件 13 は「本 spec の完了時の棚卸で `/kiro-discovery` が起票する」と決めた（9.2）。そのため、渡し先の spec の名前・渡した日・生きていることの確かめ（1.3）・本 spec の brief への記入（9.3）は、実装のタスクが終わった後、`/kiro-complete` の中でしか書けない。設計は「最後のタスクが表を実物に合わせて書き直す」と「確かめの手順 4」に分けて書いているが、件 13 だけはタスクの段で埋まらないことを明記していない。
- **なぜ大事か**: 実装の後の `/kiro-validate-impl` が、1.3・1.5・9.3 を「件 13 の渡し先の名前が無い」として赤にするか、逆に誰も書かないまま閉じるおそれがある。
- **直し方**: タスクの段で、最後のタスクは件 13 を「完了時に起票して渡す（名前と日付は `/kiro-complete` の棚卸で書く）」の形で残すこと、`/kiro-complete` の棚卸がアーカイブの前に、起票した spec の名前と日付を `design.md` の表と本 spec の brief の件 13 の節へ書くことを、タスクの完了の条件に書く。
- **要件**: 1.3・1.5・9.2・9.3
- **設計の場所**: 「13 件の仕分けの記録」の件 13 の行と表の下の注・「確かめの手順」4

### 2. kanade のテストの足場の決まりと、書き下した期待値の食い違いを、テストの中に書いていない

- **困ること**: kanade の統合テストの足場は「期待する References を定数で書かず、`areka_kanade::events::*` から作る」と決めている（`tests/kanade/common/common_recording.rs` 30〜31 行の説明）。要件 7.3 は、作る関数そのものの誤りにも気付けるように、`["user","1","1"]` と書き下すことを求める。設計はこの例外を選んでいるが、新しいテストのファイルの説明でそれを断ることを書いていない。
- **なぜ大事か**: タスクのレビュー（`kiro-review`）や次の開発者が、足場の決まりに合わせて `on_close` から作る形へ「直して」しまうと、7.3 が狙った檻が消える。
- **直し方**: `close_test_scope_references_tests.rs` の冒頭の説明に「足場の決まりの例外。要件 7.3 により、期待値を `events::on_close` から作らずに書き下す（作る関数の誤りにも気付けるように）」と 1〜2 行書くことを、設計かタスクに足す。
- **要件**: 7.3・7.4
- **設計の場所**: 「終了のスコープの通しのテスト」の Responsibilities & Constraints

### 3. 往復の関数の新しい説明が、呼び手を 2 つしか挙げない

- **困ること**: 設計が書く新しい説明は「会話の道は Unloading{Fault} へ倒し、照会の道は `ResourceOutcome::Failed` として返すだけ」である。しかし `round_trip` を通る呼び手は、ほかに終了（`round_trip_unload`）・翻訳（`actor_translate.rs` の `round_trip_raw`・`translate_failed` で終了へ進む）・外からのイベント（`actor_sstp.rs` の説明が、送出点は `round_trip_request` だと書く）がある。2 つだけを挙げると、それで全部だと読めてしまい、6.5 の「事実に合わせる」に半分しか届かない。
- **なぜ大事か**: 件 6 の根は「往復の記録が、呼び手のすることを言い切っていた」ことである。説明が別の形で同じことを言い切ると、次に道が増えたときにまた古くなる。
- **直し方**: 説明を「失敗の後どうするかは呼び手が決める（例: 会話と翻訳の道は終了へ進み、照会の道は失敗として返すだけ）」のように、例であることが分かる形にする。あわせて、6.4 のテストは送る段の失敗（`shiori_send_failed`）だけを見るので、残り 2 行（`shiori_reply_dropped`・`shiori_reply_timeout`）の文に「終了」が残っていないことを、実装のタスクの確かめ（文字列の検索 1 回）に入れる。
- **要件**: 6.1・6.5
- **設計の場所**: 「往復の記録」の文の表の下の注

### 軽微な点（直さなくても進められる）

- 「`win32.rs` は記録を出さない」と書くが、今の `show` は `debug!` を 2 行（`menu_foreground_refused`・`menu_null_post_failed`）出す。失敗の記録を呼び手に任せる、という意味なら正しいので、タスクの段で読み違えないように言い方を「失敗の記録は呼び手が出す」に揃えるとよい。
- 説明書の受け口の接頭辞 `[readme]` は、隣の受け口（`[ChangeCueSink]`・`[InstallCueSink]`）の形とは違う。要件 5.1 が説明書の道（`readme.rs` の `[readme]`）に揃えると決めたので、このままでよい。

## よい所

1. **事実に足を置いている**: 済みの 4 件を、場所とテストの名前で示し、確かめた日の main でもう一度見る手順を持つ。台帳の `owner_count` も手で引き算せず数え直す。設計が頼るコードの主張は、確かめた限りすべて今の main と合っていた。
2. **通しのテストが本物の部品を通る**: 件 3 は、偽物を SHIORI と OS の 2 つの端だけに置き、解析・配送・受け口・取り出し・`submit`・`serve`・`execute` を本番の関数のまま通す。待ちは `Leave` の合図で終わり、`dola` の配送の順（確かめ済み）に支えられているので、時間に頼らない決定論のテストになる。プロセス共有の `REFUSED` を使わないので、並走するテストと混ざらない。

## 判定

- **GO**
- **理由**: 既存の層と取り決め（記録は呼び手が出す・往復の失敗の扱いは呼び手が決める・テストは兄弟に置く）を守り、10 本の要件のすべてに部品とテストが当たっている。上の 3 点は文書とタスクの書き方で片づく小さな点で、設計の形を変えない。
- **次の手順**: 設計の討議（`kiro-design-discussion`）で上の 3 点を議題にし、決まった分をタスクの段へ持ち込む。その後 `/kiro-spec-tasks popup-menu-residue`。
