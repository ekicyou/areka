# 設計の検証: audit-emo-present

- 日付: 2026-11-05
- 対象: `.kiro/specs/audit-emo-present/design.md`（要件 9 の決めどおりに 43 件を直す設計）
- 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・`.kiro/steering/`（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C17 の節）・`kiro-validate-design/rules/design-review.md`
- 進め方: 対話なし（下請けとして実行）。設計の主張のうち、外れると実装が壊れるものをコードで確かめた（読むだけ）。

## 1. まとめ

設計は 43 件すべてに直しを持ち、要件 9 の決め（残すのは `CLICKWAIT_FAMILY` だけ・`build_shell_target_with_boxes` は可視性だけ・`surfaces.txt` の解析の回数は触らない）を外していない。コードで確かめた数（`attach_target` の呼び手・例外表の件数・`cache_tests.rs` の削減量）はどれも設計と合っており、そのまま実装に進める。下の 3 点は小さく、タスクを組む段か設計の討議で片づく。

## 2. コードで確かめたこと

| 主張 | 確かめた結果 |
|---|---|
| `.attach_target(` を呼ぶファイルは 46 本（範囲の外 17・中 29・約 80 か所） | 合う。46 本＝外 17（`crates/areka` 11＝`src/` 8・実行例 3、`crates/areka-emo-text` 5、`crates/pilot` 1）＋中 29。中は 81 か所 |
| `pilot-balloon-asset-swap/swap.rs` は `Result` を残して `world` だけ外す | 合う。`.attach_target(world, ..).map_err(..)` を自前の `attach` の `Result` で包んでいる |
| `cache_tests.rs`（1,301 行）は消すだけで 1,000 行を切る | 切れる見込み。9 番の 4 本（約 204 行）・10 番の 3 本（約 100 行）・29 番の写し（66〜96 行・約 31 行）・`transparent_surface`（5 行）で約 340 行減り、約 960 行になる。番人は `lines > 1000` で判定する |
| 例外表の件数 `OVER_LIMIT_ALLOWED_COUNT` 9→8・`ALLOWED_COUNT` 19→18・`PROCESS_UNIQUE_COUNT` 16→15 | 合う。`balloon_test_support.rs` の項目は「プロセス識別子で一意化」の種別。`temp_path_guard_test.rs` の自己検査の 1 件（`inproc_fixture.rs`）は替えなくてよい |
| `file_length_guard_test.rs` の自己検査の 1 件を `pilot-clickthrough-alpha-toggle/main.rs` へ | 表に載っている。ただし 1,006 行で上限との差が 6 行しかない（下の議題 2） |
| テストの支えは新しいファイルなしで寄せられる | 寄せられる。`shell_target.rs` は既に `pub(crate) mod test_support`。`presenter.rs:162`・`balloon.rs:795` の `mod test_support` を `pub(crate)` にすれば、`cache`・`balloon_marker`・`balloon_definitions` の子から届く |
| 1 番は `TempPath::new` で `target\test-roots` の下へ | 合う。`TempPath::new` と `under_target` は同じ置き場。`TempDir` の `path` 欄を直に読む呼び手は無い |
| 3 番の型の替えで範囲の外の呼び手は変位を見ない | 合う。外で `PresentError` を名指すのは `emo2_boot/{mod.rs, assets.rs}`・`switch_finish_tests.rs`（`TargetNotAttached` を作るだけ）・`frame/wiring.rs`（バルーンの読み込みと関係ない）だけ。emo-text の fixture は `expect`・`unwrap_or_else`・`let Ok .. else` で受ける |
| 7 番で狭める名前は外で使われていない | 合う。外で名指すのは `nesting_fixture_tests.rs:4` のコメントだけ（9.5 で触らない）。40 番の公開の包み（`resolve_marker_pair`・`load_scope_balloon_model`）は残る |
| 5 番の `set_if_neq` | 使える。wintf の `HitTest` は `PartialEq` を持つ |
| 42 番で欄を私有にしても外は壊れない | 合う。外で `ScalePolicy` を使うのは `ScalePolicy::new(..)` だけで、構造体の字面で組む所は無い |
| `PresentError` は `#[non_exhaustive]` | 合う（`command.rs:38`） |
| `doc/`・`docs/` を名指す行・`Cargo.lock`・`crates/areka-emo-text/` へのファイル追加 | どれも無い。直すファイルに `doc/` を名指す行は無い。37 番は差分 0 を `git diff --exit-code -- Cargo.lock` で確かめる手順がある。emo-text は既存の 5 ファイルを直すだけ |
| `audit-parsers-sakura` との重なり | 守られている。`shell_target.rs` は可視性とコメントだけで、`surfaces.txt` の解析の呼び出し（390〜392・440 行）と署名には触らない。後に着地する方が合わせる取り決めも書いてある |
| 43 件それぞれに直し、バグ・危ない所に守るテスト | 43 行とも直しがある。バグ・危ない所（1〜6・38・40・ほか）には守るテストか、型や番人が守る旨が書いてある。39 番は出どころを 1 つにして食い違いを無くす形で、テストが要らない理由も書いてある |

## 3. 議題（3 件・どれも実装を止めるものではない）

### 議題 1: 範囲の外のコメントに「`attach_target` は `&mut World` を要する」が残る
- **何が問題か**: 6 番で `attach_target` から World の引数を外すと、`crates/areka/examples/emo-present.rs:111-115` と `crates/areka/examples/emo-present/systems.rs:40` の「`attach_target`／`apply` は `&mut World` を要する」が半分古くなる。設計が直す行として挙げているのは `frame/attach.rs` の 182・389 行だけで、この 2 か所は表に無い。
- **なぜ効くか**: かけ直しの audit は `crates/areka-emo-present/` だけを見るので、範囲の外の古いコメントはどの検査にも引っかからず残る（要件 2.3 の「呼び出し元まで直し切る」に届かない）。
- **直し方**: 6 番の「範囲の外に及ぶ所」の表のうち `emo-present/systems.rs` と `emo-present.rs` の行に「`&mut World` を要するのは `apply` だけ、と言い直す」を足す。T2 の境界に `crates/areka/examples/emo-present.rs` を足す。
- **要件**: 2.3・7.1
- **設計の場所**: 「範囲の外に及ぶ所（要件 7.1）」の表の 6 番の行

### 議題 2: 番人の自己検査が外して試す 1 件の余裕が 6 行しかない
- **何が問題か**: 自己検査が外して試す 1 件の移し先 `crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs` は 1,006 行で、上限との差は 6 行。7 行減らす直しが入るだけで、自己検査（`violations[0].lines > LINE_LIMIT`）と「今も上限を越えている」の確かめが同時に赤になる。
- **なぜ効くか**: 兄弟の見直しや後の spec がその実行例を少しでも削ると、この spec が選んだ 1 件のせいで関係のない番人が赤になる。表のほかの項目は 1,037〜1,468 行で余裕が大きい。
- **直し方**: C17 の兄弟が触らない項目のうち余裕の大きいもの（例: `crates/areka-seriko/src/actor_bind_loop_tests.rs` 1,375 行・`crates/areka/src/emo2_boot/frame_transition_branch_tests.rs` 1,271 行）を選ぶ。pilot を残すなら、余裕が 6 行しかないことを選んだ理由と一緒に記録に残す。
- **要件**: 7.7・6.1
- **設計の場所**: 「守る決まり」の例外表の段・「範囲の外に及ぶ所」の 10 番の行

### 議題 3: 2 番で、届かない失敗の枝を本番の `expect` にすること
- **何が問題か**: `show_target` の「再通過の後に登録が消えた」「表示は成り立っているのに装着が無い」の 2 枝を `expect` に替える。今の本番のコードではどちらも届かない（`apply_show` は登録を消さず、装着を作ってから `current_surface_id` を立てる）。ただし決まりは「panic は致命の時だけ・ほかは `error!`＋`Err`」である。
- **なぜ効くか**: v0.2.0 の 6 本がこの上に装着を外す道を足すと、前提が崩れたときに記録と `Err` ではなく UI スレッドの panic で落ちる。
- **直し方**: `replace.rs` の `expect("直上で登録済み")` という前例に合わせ、`expect` の文言に前提（どの不変条件か）を書くことを決めとして確かめる。前提を v0.2.0 の 6 本へ渡す「Revalidation Triggers」に 1 行足す。決まりを厳しく読むなら、`error!`＋原因どおりの変位（`NotShown` など）で返す案もある。
- **要件**: 6.3・2.5
- **設計の場所**: 「指摘ごとの直し」の 2 番の行・「失敗の型」の「2 番の `expect` 2 つ」

## 4. よい所

- **形が変わる直しを先に置いた直列の順**: 2・3・4・6（型と署名）を T1〜T3 に置き、範囲の外の呼び手を先に着地させてから、後のテストの直しを新しい型の上で書く。同じファイルを何度も触るタスクを直列にしたので、取り違えの危うさが小さい。17 本で 20 以下にも収まる。
- **数が確かめられている**: 呼び手の数・例外表の件数・消すものの使い手の有無が `crates/` 全体を検索した数で書かれており、コードと合っていた。兄弟と同じ件数の行がぶつかったときは数え直して合わせる、という手当て（両方の枝が同じ誤った数を書いても git は黙って通す件への備え）まで書いてある。

## 5. 判定

- **判定: GO**
- **理由**: 43 件すべてに要件 9 どおりの直しがあり、守る決まり（1,000 行・`Cargo.lock`・`doc/`・emo-text・`audit-parsers-sakura` の持ち物）を外していない。数の主張はコードと合っていた。議題の 3 件は、表に 1〜2 行足すか決めを確かめるだけで済む。
- **次の手**: 設計の討議（`/kiro-design-discussion audit-emo-present`）で上の 3 件を決め、`/kiro-spec-tasks audit-emo-present` へ進む。
