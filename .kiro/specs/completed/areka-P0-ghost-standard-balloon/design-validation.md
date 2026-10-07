# 設計の検証レポート: areka-P0-ghost-standard-balloon

- 検証日: 2026-10-05
- 対象: 同じフォルダの `design.md`（`requirements.md`・`research.md` と突き合わせ）
- やり方: 設計の文をそのまま信じず、設計が名指しするソースとテストと道具を読んで確かめた。ビルドとテストは回していない。

## まとめ

設計は、今の 3 つの部品（読み手 `catalog.rs`・純粋な鎖 `boot_resolve.rs`・入力を集める口 `boot_config.rs`）をそのまま伸ばす形で、要件 1〜5・7 の中身はソースと食い違わない。足す分岐は少なく、記録の出口も表で尽くされている。

ただし、網羅台帳まわり（要件 6.4）に **設計のままでは `cargo test -p ukadoc-survey` が赤になる抜け** が 1 つある。直す場所ははっきりしていて、鎖や読み手の設計には響かない。

**判定: GO（条件つき）** — 下の「重大な指摘 1」を設計の討議で設計に書き足してから、タスク分けへ進む。

## 重大な指摘（1 件）

### 重大な指摘 1: 台帳の 2 行の状態と担当を変えると、`briefing.md` と `roadmap-draft.md` も直さないと検査が赤になる

**何が問題か**

設計は台帳 `assets.toml` の `balloon` を `implemented`、`default.balloon.path` を `degraded` にし、担当（`owner`）を本 spec にする。そして `doc/ukadoc-coverage/briefing.md` を「境界の外」に置き、`roadmap-draft.md` には触れていない（design.md「Out of Boundary」・D6・「文書と台帳」）。理由は「状態を変えても束の帰属と数は動かない」である。

束の帰属と数についてはそのとおりだが、`ukadoc-survey` の整合のテストは、束のほかに次の 2 つを台帳から数え直して突き合わせている。

1. **ページ別の状態の数**（`crates/ukadoc-survey/tests/consistency/briefing_arms.rs` の `distribution_findings`）。`briefing.md` の `[[barrier]]` の `page = "descript_ghost"` の行は今 `implemented = 16`・`degraded = 0`・`absent = 58`。2 行を変えると台帳の数え直しは 17・1・56 になり、3 つの数が食い違う。
2. **担当の行き先**（同 `spec_checks.rs` の `owner_destination_findings` と `owner_count_findings`）。台帳の空でない担当は、`roadmap-draft.md` の `[[spec]]` の名前か、`briefing.md` の `[[owner_completed]]` の名前のどちらかに無ければ赤。`doc/ukadoc-coverage/` の下の `.md` に `ghost-standard-balloon` の綴りは今 0 か所である。

**先例**

完了 `areka-P0-mouse-drag-events`（squash `1bb449ae`）は、台帳の 2 行を実装済みへ移したとき、`briefing.md` の `[[barrier]]` の数 2 つと、`roadmap-draft.md` の `[[spec]]` の 1 行（`owner_count = 2`）・`[briefs].count`・`snapshot_on`・説明の段落・段階の表の行を同じコミットで直している。

**影響**

設計どおりに実装すると、実装の順 6 の「`cargo test -p ukadoc-survey` を通す」が通らない。直そうとすると設計が「境界の外」と書いたファイルを触ることになり、実装者とレビューアが境界違反かどうかで止まる。

**直し方の案**

- 設計の「This Spec Owns」と File Structure Plan に次を足し、「Out of Boundary」から `briefing.md` を外す（`linkage.md` は外のままでよい）。
  - `doc/ukadoc-coverage/briefing.md`: `[[barrier]]` の `descript_ghost` の 3 つの数（16 → 17・0 → 1・58 → 56。実装時に数え直した値を書く）。
  - `doc/ukadoc-coverage/roadmap-draft.md`: `[[spec]]` に本 spec の 1 行（`owner_count = 2`・束は 2 行が属する「既定で着せる吹き出し・読む経路が無い」・段階とウェーブは先例と同じ決め方）と、`[briefs].count`・`snapshot_on`・説明の段落。
- 手を減らす別の案: 2 行の `owner` を空のままにすれば `roadmap-draft.md` は触らずに済む（担当の検査は空でない担当だけを見る）。その場合も `briefing.md` の 3 つの数は要る。先例は担当を登記しているので、どちらにするかは討議で決める。
- 実装の順 6 に「`briefing.md`・`roadmap-draft.md` の数を直す」を足す。

**対応する要件**: 6.4（台帳の 2 行）・6.5・6.7（同じ検査を通す）

**設計の該当箇所**: 「Boundary Commitments」の Out of Boundary の 4 つ目／「設計の決め」D6／「文書と台帳」の網羅台帳の節の箇条書き 1 つ目と 3 つ目／「実装の順」6

## 確かめて問題が無かった点

依頼で名指しされた確認点を、ソースを読んで確かめた結果である。

| 確認点 | 結果 | 根拠 |
|---|---|---|
| `BalloonInputs.listed` を `&[catalog::BalloonEntry]` に変えて他の呼び手が壊れないか | 壊れない | `BalloonInputs` を組むのは本番で `boot_config.rs` の 2 か所（引数の腕は `&[]`）、テストで `boot_resolve_tests.rs` の補助関数 `balloon` の 1 か所だけ。`resolve_balloon_for_ghost` の呼び手は `resolve_boot_from` と `ghost_switch.rs` の `boot_into` の 2 つで、署名は変わらない |
| 無作為の段の添字と並び | 変わらない | 今も `list_balloons` の戻りをフォルダ名へ落としただけの列で、件数と並び（フォルダ名のバイト順）は同じ。`[only]` の腕も型が変わるだけ |
| `BalloonRoute` に腕を足して他が壊れないか | 壊れない | 本番で腕を見るのは `LastUsed::record` の `== BalloonRoute::Argv` だけ（`Descript` は引数以外なので記憶が書かれる＝要件 3.9）。腕を全部並べる `match` は無い |
| 「最初の 1 個は無印か `balloon0` か無し」で要件 1.1〜1.6・1.14 を尽くすか | 尽くす | 探索は最初に無い番号で止まるので 3 通りしか無い。空の無印＋`balloon0` → 無し、無印なし＋空の `balloon0`＋`balloon1` → 無し、`balloon1` だけ → 無し、のどれも設計の表に行が在る。`parse_kv` は空の値の行を鍵ごと残す（落とすのは `lowercased` の側）ので、「行は在るが値が空」は設計の言う読み方で見分けられる |
| 既存の同梱のテストがそのまま通るか | 通る見込み | `companion_balloon_reads_one_key`（無印と `balloon0` の両方 → 無印）・`empty_values_count_as_absent`（空の無印だけ → 無し）は新しい決まりでも同じ答え |
| `balloon` の名前 → フォルダ名の決まりが `resolve_skin_target` と同じか | 同じ | `SkinSpec::Name` の最後の腕は「`name` が一致する最初の候補 → 無ければフォルダ名の一致」で、候補は同じ `list_balloons` の並び。同着・`name` とフォルダ名の取り合いも同じ答えになる。`random`・`lastinstalled` だけが違い、設計はそこを突き合わせから外している（要件 2.8） |
| descript.txt が読めないときの記録がちょうど 1 件か | 1 件 | `standard_balloon_keys` が `master_descript_keys` を 1 回だけ呼ぶ。`resolve_balloon_for_ghost` の中に同じファイルの読み手は他に無い |
| 「記憶で決まった解決でも、読めないファイルの記録は出る」は要件 5.6・3.2 と食い違わないか | 食い違わない | 5.6 は「指定が書かれていない段」の話で、読めないのは 5.4 が記録を求める別の場合。今の同梱の読み方と同じ扱い。3.2 は引数でバルーンを渡した腕の話で、その腕は読み手を呼ばない |
| 切替の記録を足しても、起動の「バルーンを決めました」は 1 件のままか | 1 件のまま | `balloon_resolved` を出すのは `resolve_boot_from` の 1 か所で差分 0 行。新しい記録の文面「切替先のバルーンが決まりました」は `tools/package.ps1` の目印「バルーンを決めました」を含まない |
| `route=Companion` の目印（`tools/package.ps1`・`smoke_boot_loop_exit.rs`）が保たれるか | 保たれる | 検体 `emo2` の descript.txt に `balloon`・`default.balloon.path` は無く（書庫の中身を確かめた。`claudia`・`konnoyayame`・`R_POST_and_KOMAINU` も同じ）、`install.txt` は無印の `balloon.directory` なので、新しい段は当たらず `Companion` のまま |
| 台帳の `implemented` と証拠の置き方 | 通る | 検査が証拠を求めるのは `implemented` だけ（`check/content.rs` の `check_evidence`）。設計の 2 つの URL は台帳の id（`balloon_2c_30d0_30eb_30fc_30f3_540d:1`・`default.balloon.path_2c_30d1_30b9:1`）と合い、`Identity` の欄と同じ置き方。`degraded` に証拠の行が在っても赤にする検査は無い |
| 1 ファイル 1,000 行 | 収まる | `ghost_switch.rs` 891 行・`boot_resolve.rs` 544 行・`boot_config.rs` 492 行・`catalog.rs` 377 行。テストは新しい兄弟ファイルへ分ける設計 |
| 突き合わせのテストが `emo2_boot` を使えるか | 使える | `shell_balloon_resolve`・`shell_balloon_switch` は `pub(crate) mod`、`resolve_skin_target`・`SkinSpec`・`SkinCandidate` も `pub(crate)`。使うのはテストだけなので、本番の依存の向きは変わらない |

## 軽い指摘（討議で拾うかどうかを決める）

1. **要件 7.2 の「同梱が唯一・既定・無作為に勝つ」**。設計のテスト 3 は「同梱が既定に勝つ」だけを挙げる。唯一は候補が 1 個なので勝ち負けが起きないが、無作為（既定のバルーンが無く候補 2 個以上）に勝つ場面は書かれていない。既存の `boot_resolve_tests.rs` に同じ場面が在るならそれを名指しし、無ければ 1 場面足す。
2. **突き合わせのテスト（設計のテスト 6）の候補の作り方**。`BalloonEntry` の列と `SkinCandidate` の列を別々に手で書くと、片方だけ書き間違えても気付けない。1 つの `BalloonEntry` の列から `balloon_candidates` と同じ写し方（`folder`・`name`）で `SkinCandidate` を作る、と設計に書いておくとよい。
3. **切替のテスト 9・10 の記憶**。切替先は検体の複製なので、複製の中に `profile/areka/sylphya.toml` の `[last] balloon` が残っていると記憶の段が勝って赤になる。テストの準備で記憶のファイルが無いことを確かめる（または消す）と書いておく。
4. **「descript.txt が読めない」（要件 5.4）へ届く道**。起動も切替も、相手のゴーストは `list_ghosts` の列挙から来る。列挙は読めない descript のゴーストを除く（同じ名前の記録 `catalog_descript_unreadable` を出して）ので、`standard_balloon_keys` が読めない descript に出会うのは、引数でゴーストを渡した起動か、列挙の後に読めなくなった場合だけである。読み手のテスト 1 本で足りるが、設計の Error Handling の表にこの道を 1 行書いておくと、後から「届かない分岐では」と疑われない。
5. **切替の記録の件数の場面**。設計は「切替先を起こせず既定へ戻すと 2 件（切替先で 1 件・既定で 1 件）」と書くが、テストの場面には無い。要件 5.8 は「決まったとき 1 件」なので、戻す場面を足すか、足さない理由（既存の戻すテストに記録の件数の判定を足すと配線の踏み直しになる、など）を書く。
6. **`catalog.rs` の `companion_balloon` の説明**。今の説明の「番号付きの鍵は読まない」も直す対象である（設計は冒頭の説明だけを挙げている）。

## 設計の良い点

1. **鎖を純粋な関数のまま保ち、ファイルを読むのは読み手だけにしている**。値の検査（`..`・絶対パス・区切り）をわざわざ書かず、「列挙のフォルダ名は 1 段の名前なので完全一致で当たらない」ことに任せたので、記録の出口が増えず、要件 1.15・2.4・5.2 が足すコード 0 行〜数行で満たされる。
2. **配布物の検査と起動の煙テストが見る目印（`route=Companion`・「バルーンを決めました」）を先に調べ、起動の記録のコードを差分 0 行に保った**。切替の記録を別の名前・別の文面にした理由が、道具の実物に基づいている。

## 最終判定

**GO（条件つき）**

- 理由: 鎖・読み手・切替の記録の設計はソースと合っており、要件 1〜5・7 の ID はすべて部品とテストの場面に対応している。見つかった抜けは台帳まわりの文書 2 つの数と行で、直す場所と先例がはっきりしている。
- 次の一歩: 設計の討議（`/kiro-design-discussion areka-P0-ghost-standard-balloon`）で「重大な指摘 1」を設計に書き足し、軽い指摘 1〜6 を拾うかどうかを決める。その後 `/kiro-spec-tasks areka-P0-ghost-standard-balloon` へ進む。
