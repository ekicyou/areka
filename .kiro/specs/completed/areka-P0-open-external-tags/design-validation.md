# 設計の検証: areka-P0-open-external-tags

- 対象: `design.md`（2026-10-05 生成・`316bd260`）と `requirements.md`・`research.md`・`brief.md`・`.kiro/steering/`
- やり方: `kiro-validate-design` の手順（分析 → 重大な問題 → 良い点 → 進めるか）。対話なしで判定した。
- 引用の照合: 設計が行番号つきで引いた実物の行（`decode.rs:35`・`:203`・`:270`・`:272`、`compile.rs:181`、`readme_cue.rs:84`、`readme.rs:27`・`:154-158`・`:165-198`、`emo2_boot/mod.rs:491`・`:492`・`:717-719`、`menu/mod.rs:313`、`ghost_session.rs:61`、`mcp/mod.rs:6`、`log_history.rs:113`）は、すべて字句どおりに一致した。

## まとめ

設計は、`\j` を読み込みの腕 1 本で汎用の `\!` の運び手へ写し、既に結線されている説明書の受け口を開く系全体へ広げ、OS を呼ぶのを開く専用の 1 本のスレッドに閉じる形で、要件 1〜10 の 56 項目をすべて部品へ割り当てている。同じウェーブの約束（`main.rs`・`emo2_boot/mod.rs` に触らない・`CueCommand` を足さない・`decode.rs` は腕 1 本・`compile.rs` に触らない）は設計の上では守られている。気になるのは中身の誤りではなく、並走の `mcp-author-tools` との重なりが設計に書かれていないことと、常時テストで本物の OS を呼ばないことが「誰も送らない」という約束だけで守られていることの 2 点である。

## 確かめたこと

### 数と引用

- 受け取り手の表の総数の檻は今 15（`consumer_ledger.rs` の `canonical_builds_without_duplicate`）。本 spec が足すのは 6 行（`open` の 5 組＋運搬名 `\j`）。要件 9.2 の 5 組に `\j` を足した 6 は、要件 1.5（`\j` も同じ受け取り手）と合っている。
- 記録の振り分けの表は今「info の行を振り分ける 13 行」（`doc/ssp-mcp/log-convention.md` と `RULES` の実数も 13）。1 行足して 14 にする設計は正しい。
- 網羅台帳の 6 行は実在する（`_5cj_5bID_5d:1`・`open_2cfile`・`open_2cbrowser`・`open_2cexplorer`・`open_2ceditor`・`open_2cmailer`）。`degraded` 2 行・`implemented` 4 行は要件 9.1 と一致。
- 要件の対応表は 56 行（1.1〜10.6）で、抜けはない。
- `ShellExecute` の綴りは今 `readme.rs` と `readme_tests.rs` だけにある（`crates/` 全体を検索）。`Command::new` などほかの起こし方も `crates/areka/src` に無い。見張りのテストは成り立つ。
- `Win32_UI_Shell`・`Win32_System_Com` は根の `Cargo.toml` で有効。`log-capture-kit`・`temp-path-kit` は `areka` の dev 依存にある。`resolve_switch_target`・`GhostSpec::Name`・`shell_candidates(&Path)`・`balloon_candidates(&BasewareRoot)`・`list_ghosts`・`BasewareRoot`（`PartialEq` あり）・`GhostSlot`（NonSend・`ghost_dir()` あり）・`BootContext`（Resource・`root` あり）は設計の書いた形で実在する。
- `balloon-lifecycle-events` 側の文書にも本 spec との「数え直し」の約束が載っている（相手の worktree の requirements・design で確認）。

### 規則の順番（先に当たったものが勝つ所）

- `classify` の `\j` の分け方（`http(s)://` → `mailto:` → `file:///` → それ以外）は互いに重ならない。
- 受け口は `("open","readme")` を先に拾い、残りを `classify` へ渡す。`classify` は `readme` に `None` を返すので、二重に拾うことはない。
- 受け取り手の表は「選別子なしの登記を先に引く」規則。`("\\j", None)` は他の `\j` の登記と衝突しない。`open` には選別子なしの登記が無いので、`("open","help")`・`("open", None)` は担当なしのまま。
- 記録の振り分けは上から当てる。足す `areka::readme`（下も含む）の前にある行（`areka` は完全一致、`areka::install` など）は `areka::readme::opener` に当たらないので、横取りされない。

### スレッドと World の借用

- `ShellExecuteW` と fs の解決（実在・フォルダか・目録の読み取り）は開く専用のスレッドだけが行う。UI スレッドで行う fs は、今もある説明書のファイル 1 つの有無の確認だけ。台本のスレッドの受け口は分類して送るだけ。
- COM の初期化（STA）は開く専用のスレッドの最初に 1 度だけ行う。
- メニューの「説明書」も `submit` を通るので、`popup-menu-residue` が挙げた「World を借りている間の `ShellExecuteW`」はこの設計で消える。

### 同じウェーブの約束

- `main.rs`・`emo2_boot/mod.rs`・`compile.rs`・dola への変更は 0。`emo2_boot/mod.rs` が綴るのは型名 `ReadmeRequest`・`ReadmeCueSink::new`・`resolve_path`・`wire_readme` の署名だけで、設計はそれらを変えない。
- `decode.rs` は `"f"` の腕の次に `"j"` の腕を 1 本足すだけ。定数は `model.rs` に置き `sakura/mod.rs` の公開の行へ足す（約束の外のファイルだが、禁止されてはいない）。

## 重大な問題

### 🔴 重大な問題 1: 並走の `mcp-author-tools` が同じ 2 つのファイルを変える予定で、設計に調整の記述が無い

- **問題**: roadmap の C4 の約束は「⑮（本 spec）の `decode.rs`・`compile.rs`・`consumer_ledger.rs`・`readme.rs` は C4 のほかの誰も触らない」「⑨（`mcp-author-tools`）は `consumer_ledger.rs` に触らない」と書く。ところが `mcp-author-tools` の design.md（タスク生成済み）は、⑴ `decode.rs` の `Raw` を作る 4 か所（`decode_passthrough_tag` を含む＝本 spec が `"j"` の腕を足すすぐ隣）などが「印」を一緒に返す形に変える、⑵ `consumer_ledger.rs` に 2 行足して総数の檻を 15 → 17 にし、`#![allow(dead_code)]` を外し、表と 8 つの受け口（`ReadmeCueSink` を含む）の選別の一致を見るテスト `consumer_ledger_agreement_tests.rs` を新しく足す、と書いている。本設計の「Revalidation Triggers」と要件 9.3 の約束は `balloon-lifecycle-events` しか挙げていない。
- **影響**: 後から main へ入る側で、⑴ `decode_tag` の腕の形（印を返すなら `"j"` の腕も合わせる）、⑵ 総数（15＋6＋2＋1＝24 になるはず）とモジュールの doc の「15 行」、⑶ 一致のテストの見本（本 spec の 6 行それぞれに「受け口へ届く」見本が要る。`\j` の見本は URL でなければ受け口が断るので届かない）が食い違い、赤になるか、手で数を書いて誤る。
- **提案**: 設計ディスカッションで、⑴ `mcp-author-tools` のセッションへ重なりを知らせ（開発者経由か SendMessage）、着地の順を決める、⑵ 本設計の「Revalidation Triggers」と要件 9.3 の相手に `mcp-author-tools` を足す、⑶ タスクに「main を取り込んだ後、総数・doc の行数・一致のテストの見本（`\j` は `http://` の見本）を実物から数え直して直す」を明記する。
- **対応する要件**: 9.2・9.3・1.5（と brief の「同じウェーブ C4 の約束」）
- **設計の場所**: 「Boundary Commitments → Revalidation Triggers」・「表と記録の振り分け（要約）→ ConsumerLedger」・「JumpTagArm」

### 🔴 重大な問題 2: 常時テストで本物の OS を呼ばないことが「誰も要求を送らない」ことだけに頼っている

- **問題**: 設計は `register_readme_drain` の中で `Opener::spawn()`（本物の `WindowsShell` と COM の初期化を持つスレッド）を起こす。`register_readme_drain` を呼ぶ `ghost_session::register_systems` は、本番だけでなくテストの組み立て（`emo2_boot/ghost_switch_test_support.rs`・`ghost_session_restart_tests.rs`）も呼ぶ。設計は「起きる本物のスレッドには要求を 1 件も送らない」と書くだけで、仕組みとしての歯止めが無い。
- **影響**: 今は台本に開く系のタグを含むテストが無いので実害は無い。しかし後続（`choice-script-prefix` の `script:`、`mcp-kanade-tools` の `sakurascript`、`link-context-copy`・`balloon-link-hover`）が同じ組み立てで台本を流すテストを書くと、開発者の机で本物のブラウザ・エクスプローラー・実行ファイルが起き、要件 10.1 の「実際に起こす回数 0」が黙って破れる。見張りのテスト（`ShellExecute` の綴りの場所）では捕まらない。
- **提案**: テストのビルドでは本物の OS を呼べない形にする。例: `Opener::spawn` を `#[cfg(test)]` では「呼ばれた `OsCall` を記録して `error!` を残すだけの偽物」で起こす、または `register_readme_drain` が使う OS の作り口を差し替えられるようにする。あわせて「テストのビルドで本物の `WindowsShell` が作られない」ことを 1 本のテストで固定する。
- **対応する要件**: 10.1・10.6
- **設計の場所**: 「ReadmeDrain（`readme.rs` の変更）」の `register_readme_drain` と Implementation Notes の Validation・「Opener」の `Opener::spawn`

## 良い点

1. **規則を 1 つの純粋な関数に寄せた**: 受け口（実行時）と `link_destinations`（取り出し）が同じ `classify` だけを呼ぶので、要件 8.6 の「2 つの規則が食い違わない」が構造で守られ、後続 2 本は規則を持たずに済む。分類の表が断る入力（引数なし・3 形以外の `\j`・`headline`／`plugin`）まで 1 枚で書かれていて、テストの表にそのまま写せる。
2. **OS を待つのを 1 本のスレッドに閉じた**: UI スレッドは文脈を写して送るだけ、台本のスレッドは分類して送るだけにしたことで、要件 7.5（止めない）・7.8（順番どおり）・7.1（入口は 1 か所）を同時に満たし、今ある「メニューから World を借りたまま `ShellExecuteW`」も解消する。`script-impact-tiers` が同意の窓を差し込む場所（`submit`）もはっきりしている。

## 細かい指摘（進める妨げではない）

- 要件 4.4 は「名前の引き方は `\![change,ghost,名前]` と同じ」と書くが、`\![change,ghost,…]` は `random`・`sequential`・`lastinstalled` も解く（`NameResolution`）。設計はこれを解かない（§8 に裁量として登記）。ディスカッションで要件の文言を「名指しの引き方（`descript` の `name` → フォルダ名）は同じ」に揃えるか確認するとよい。
- `decode.rs` の `_` の腕のすぐ上の注記（「subset 外タグ（`\i` `\j` 等）はタスク 4.2 のパススルー領分。」）は、角括弧つきの `\j[…]` を読むようになると古くなる（裸の `\j` を書く `decode_bare` 側の注記は、裸の `\j` が今のまま `Raw` なので正しいまま）。「他の行は変えない」約束と両立させるなら、タスクで注記の 1 行だけ直すかを決めておく。
- 設計は「`ShellExecuteW` と `CoInitializeEx` を綴るのは `readme/os_port.rs` だけ」と書くが、`CoInitializeEx` は既に `emo2_boot/assets_tests.rs` などのテストにある。見張るのは `ShellExecute` だけなので、文言を「本 spec の中では」に絞るとよい。
- `OpenJob` の文脈（ゴースト名・フォルダ）は台本を出した時点でなく UI で取り出した時点の `GhostSlot` から写す。ゴーストの切り替えの直前の台本では、まれに切り替え後のゴーストの名前・フォルダで解かれうる。取り出しは毎 tick なので実害は小さいが、Risks に 1 行あるとよい。
- 入口で捨てたときの `open_external_dropped` は取り決めの target でないので、ゴースト名では絞り込めない（Monitoring の「`ghost_name` で絞り込める」は解決と OS の失敗の行だけに当てはまる）。
- `ShellExecuteW`（`Ex` でない形）が、関連付けの無いファイルや見つからない名前で OS 側の窓を出すかどうかは research の残件どおり実機で確かめる（要件 7.6 との関係）。

## 判定

- **判定: GO**
- **理由**: 設計の中身は要件 1〜10 を漏れなく満たし、引用・数・規則の順番・スレッドの切り分け・ウェーブの約束のいずれにも誤りは見つからなかった。重大な問題の 2 点は、並走の spec との調整とテストの歯止めの追加で、設計の形を変えずにディスカッションとタスクで直せる。
- **次の手順**:
  1. 設計ディスカッション（`/kiro-design-discussion areka-P0-open-external-tags`）で重大な問題 1・2 と細かい指摘を扱う（問題 1 は `mcp-author-tools` 側への知らせを含む）。
  2. 決まったことを design.md（Revalidation Triggers・ReadmeDrain の Validation）と要件 9.3 に反映する。
  3. `/kiro-spec-tasks areka-P0-open-external-tags` でタスクを作る。
