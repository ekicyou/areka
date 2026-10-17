# 設計の検証: areka-P0-emily-ghost-verification

- 実施日: 2026-10-17（ワークツリーの HEAD `065c97a1`）
- 入力: 同じフォルダの `spec.json`（`language: ja`・`phase: design-generated`）・`requirements.md`・`design.md`・`research.md`、steering の `product.md`・`tech.md`・`structure.md`（1 ファイル 1,000 行の目安）
- やり方: 設計が名指しした場所を、ソースで読んで突き合わせた（ビルド・テスト・実機は回していない）。人に尋ねずに判定した。

---

## 1. まとめ

設計は、断っている所（`plan.rs` の `companion_placement` の 1 か所）・ダイアログを出す所（`procedure.rs` の `archive_steps` の 1 か所）・窓の数を決める所（`placement/config.rs` の `detect_scopes` の 1 か所）を、どれも「全部の呼び手が通る根」で押さえていて、ソースと食い違う主張はほぼ無い。直しの量は小さく、層の向き（`areka-nar` → 手続き → 本物の口 → `alert.rs`）も今の形を崩さない。気になるのは、直書きの数（14・7）の洗い出しに漏れがあることと、えみりの台本の `\p[2]` の道が調べ残しのまま実機へ回っていることの 2 つで、どちらもタスクの段で手当てできる。

---

## 2. ソースとの突き合わせ（主張ごと）

| 設計の主張 | ソース | 結果 |
|---|---|---|
| 断っているのは `companion_placement`（167〜199 行）の 1 か所だけ | `crates/areka-nar/src/plan.rs` 167 行・181 行（`files.is_empty() && dirs.is_empty()` で `CompanionSourceMissing`）。`build_plan`（85〜96 行）が `?` で全体を断る | 合う |
| 今の拒否の鍵は常に `{key}.source.directory` と綴る（だから `source_written` が要る） | `plan.rs` 182 行 `format!("{}.source.directory", companion.key)` | 合う |
| `InstallOutcome` を値で組むのは `areka-nar` の中だけ | 組むのは `install.rs` 469 行の 1 か所。外は読むだけ（`sample-ghost-kit` の `lib_tests.rs` 426 行・`devroot.rs` 372 行は戻り値を捨てる） | 合う |
| `Companion` を値で組むのは `areka-nar` の中だけ | `manifest.rs`・`manifest_tests.rs`・`manifest_companion_tests.rs` だけ（`manifest_companion_reading_tests.rs` は組んでいない＝設計の「変えるファイル」の 1 つは触らずに済む見込み） | 合う（軽い差） |
| 飛ばした同梱の置き場に 1 バイトも触れない | `lib.rs` の `place`（192〜216 行）は計画の列の配置ごとにだけ根を尋ね作業フォルダを掘る。計画に入れなければ根も掘らない | 合う |
| 手続きの口 `InstallPorts` は今 7 つ | `procedure.rs` 42〜65 行 | 合う |
| 利用条件の画面は背景のスレッドから `alert::ask_yes_no` | `worker.rs` 185 行 | 合う |
| 門は書き終えた時点で手放す・背景のスレッドは join しない | `worker.rs` 204〜211 行・`desk.rs` 494 行（「取っ手は持たない（落としても join しない）」） | 合う。ダイアログが開いたままでも終了を待たせない |
| `alert.rs` の `unsafe` は `MessageBoxW` に閉じる・`suppressed()` で抑える | `alert.rs` 13 行・252〜260 行・283 行・318 行 | 合う |
| `failure_word` はワイルドカードの腕を持たない | `judge.rs` 121〜136 行 | 合う（消し忘れはビルドが止める） |
| `build_placement_config` の呼び手は 3 か所 | `placement/mod.rs` 406 行・`reseed.rs` 62 行・`emo2_boot/frame/switch.rs` 448 行 | 合う |
| `detect_scopes` が `char{n}.` から scope 2 以上を足す | `config.rs` 144〜164 行・`char_scope_of` 168〜175 行 | 合う |
| panic の道は `debug_assert!(scope <= 1)` の 2 か所 | `input_events/mod.rs` 398 行・`menu/captions.rs` 110 行。ほかの `scope >= 2` の分岐（`measure.rs` 187 行・`areka-emo-present/src/balloon.rs` 160 行）は記録か縮退だけ | 合う（窓が在る形で見た場合。3 節の 2 を参照） |
| 起動の絵の側は `[0, 1]` | `emo2_boot/mod.rs` 289〜291 行 | 合う |
| 期待を変える既存のテスト 3 本 | `config.rs` 570 行・`config_limit_tests.rs` 85 行・`reseed_tests.rs` 195 行（固定入力は 126〜141 行の `char2.`） | 合う |
| 登記表は 7 行・直書きの 7 | `sample-ghost-kit/src/lib.rs` 99〜135 行。`lib_tests.rs` 6・117・124・178・180・184〜198・411〜412 行 | 合う（3 節の 1 に 1 か所の紛らわしさ） |
| `emily4.nar` の `install.txt` と最上位 | `balloon.directory,emily4`・`balloon.source.directory,balloon`。最上位は `ghost/`・`shell/`・`delete.txt`・`install.txt`・`readme.txt`・`thumbnail.png`・`updates2.dau`（`balloon/` 無し） | 合う |
| `crates/areka/src/main.rs` は 1,000 行ちょうど・触らない | 1,000 行 | 合う |

---

## 3. 重い気付き（3 つまで）

### 気付き 1: 直書きの数「14」「7」の洗い出しが、ソース全体の検索で閉じていない

- **何が問題か**: 設計の「変えるファイル」は 14 の場所を `error.rs`・`lib_vocabulary_tests.rs`・`COMPAT_ARCHITECTURE.md` 287 行・`error_tests.rs` の 69 行と 289 行に絞っている。ソース全体を検索すると、ほかに次が残る。
  - `crates/areka-nar/src/error_tests.rs` 79〜85 行: テスト `all_kinds_has_fourteen_entries` が `ALL_KINDS.len()` を 14 と比べる（関数名にも 14）。
  - `crates/areka/src/install/judge_tests.rs` 176 行: 注釈「14 種を 1 つずつ組み」。
  - `doc/ukadoc-coverage/ledger/shiori.toml` 3445 行: `OnInstallFailure` の備考「areka-nar の拒否の 14 種」。
  - `crates/areka/src/install/procedure.rs` 5 行: 冒頭の注釈「口 `InstallPorts` の 7 つだけ」（口を 1 つ足すので 8 になる）。
  - `crates/sample-ghost-kit/src/lib_tests.rs` 5 行の「主張は 7 本」は検体の数ではなく主張の数で、研究の記録は直す場所に数え、設計は数えていない。どちらに読むかが書かれていない。
- **なぜ効くか**: テストの 2 つはビルドかテストが赤で教えるが、台帳の備考と注釈は何も教えない。要件 8.4 は本 spec の後の台帳に偽の記述を 0 件と求め、要件 2.3 は直書きの 7 を 0 か所と求めている。手書きの数を手で探すと、同じ数の取りこぼしが黙って残る。
- **手当て**: タスクに「`14 種`・`14 変種`・`fourteen`・`7 つ`・`seven` をソースと文書の全体で検索し、出た行を全部、直すか直さない理由つきで片付ける」という 1 行を入れる。上の 5 か所を設計の「変えるファイル」に足す。`lib_tests.rs` 5 行は「主張の数」なので、`emily4` の 1 本を主張に足すなら 8 本へ、足さないなら 7 本のままと決めて書く。
- **要件**: 1.7・2.3・7.4・8.2・8.4
- **設計の場所**: 「File Structure Plan」の「Modified Files」（`error_tests.rs`・`lib_vocabulary_tests.rs`・`judge_tests.rs`・`procedure.rs`・`doc/ukadoc-coverage/ledger/`）

### 気付き 2: 窓を絞った後の、台本の `\p[2]` の道が調べ残しになっている

- **何が問題か**: 研究の記録の調べ物 2（窓の無い scope への台詞を areka がどう扱うか・会話の終わりを待つ側が止まらないか）は、設計の段の答え（研究の記録 10.1 節）に答えが無い。設計の「panic の道は `debug_assert!` の 2 か所だけ」は、scope 2 以上の窓が作られている形で見たもの。絞った後は「台本が scope 2 を名指すが、窓もバルーンも無い」形になる。えみりは起動と終了の辞書に `\p[2]` を 50 回、メニューの辞書に 36 回持つ（研究の記録の 4.2 の行）。
- **なぜ効くか**: 要件 4.1（起動の台詞）・4.2（待っても台詞が出続ける）・4.5（終了の台詞の後にきれいに終わる・終了コード 0）は、この道が止まらないことに乗っている。止まれば実機の最初の走行で見つかり、要件 6.1 で本 spec の中で直すことになり、要件 6.4 の 20 タスクの見積もりが走行の後まで読めない。なお、起動の絵の側は今も `[0, 1]` しか作らないので、絞りで台詞の道が今より悪くなることは無い見込み（今も scope 2 には絵とバルーンの的が無い）。
- **手当て**: 次の 2 つのどちらかをタスクの段で選んで書く。⑴ 2 人のゴーストに `\p[2]` を含む台本を流し、会話が最後まで進み、終了のイベントの台詞の後に終わることを決定論のテスト 1 本で確かめる（既存の会話の固定入力が使えるなら安い）。⑵ 実機の検証の「見立ての照合」の先頭に見立て 3 を置き、最初の走行で真っ先に確かめる項目と書く。設計の「panic の道」の段落に「窓が在る形で見た。絞った後の `\p[2]` の道は ⑴ か ⑵ で確かめる」と 1 文足す。
- **要件**: 4.1・4.2・4.5・4.7・6.4・6.7
- **設計の場所**: 「areka / placement」の「scope の絞り」の責務と制約・「実機の検証の手順」の走行 5・「検証の記録」の見立ての照合

---

## 4. 良い所

1. **直す所を、全部の呼び手が通る 1 か所に置いている。** インストールの扱いは `build_plan` の同梱の 1 本の道（ゴースト・シェル・無印・番号付きの全部）、ダイアログは `archive_steps`（メニュー・台本・投げ込み・網・起動中のゴーストの上書きの全部）、窓の数は `detect_scopes`（3 つの呼び手の全部）。どれもソースで確かめた。症状の出た道だけを塞ぐ形になっていない。
2. **新しい部品を最小にし、テストの継ぎ目を値と口で作っている。** 飛ばした事実は `SkippedCompanion` の値で運び、ダイアログの求めは口 `warn_skipped` で数える。`areka-nar` は記録と値だけ、画面は `alert.rs` の 1 か所だけで、偽の口 `FakePorts` が本物の `areka-nar` を通したまま件数と本文を見られる。依存の追加は 0、`main.rs` は触らない。

---

## 5. 軽い気付き（タスクの段で拾えば足りる）

- `crates/areka/src/input_events/input_events_tests.rs` は 890 行。6.7 の 1 本を足すときに 1,000 行の番人（`file_length_guard_test.rs`）に近づく。長くなるなら兄弟のテストへ置く。
- 6.7 のテストが「直す前に赤」になるには、窓（`CharWindowMarker`）の scope を手で決めず、固定入力から `build_placement_config` が返した scope の全部で作る必要がある（今の `input_events_tests.rs`・`trigger_wired_tests.rs` は scope を手で書いて窓を作っている）。設計の文はそう読めるが、タスクにはそのことをはっきり書く。
- 設計と研究の記録の「2.6 MB の展開」は書庫の大きさ（2,677,923 バイト）で、展開すると約 3.8 MB（260 エントリ）。テストの時間の見立てに少し効くだけ。
- `placement/mod.rs` の `shared_test_support` は `mod`（非公開）で接続されている（895〜896 行）。設計の「`pub(crate)` でなければそこだけ広げる」のとおり、広げる必要がある。同じ `placement/mod.rs` 77 行の `pub(crate) mod test_support` を使う手もある。

---

## 6. 判定

- **判定: GO**
- **理由**: 層の向き・直す場所・テストの継ぎ目は今の仕組みと合っていて、要件 1〜8 のどれにも道が引かれている。上の気付き 1・2 は設計の形を変えず、タスクに数行を足せば閉じる。

### 次にやること

- 設計の討議（`kiro-design-discussion`）で、気付き 1（数の洗い出しを検索で閉じる・`lib_tests.rs` 5 行の読み方）と気付き 2（`\p[2]` の道を決定論のテストで先に見るか、実機の最初の項目にするか）を決める。
- その後 `/kiro-spec-tasks areka-P0-emily-ghost-verification` でタスクを作る。
