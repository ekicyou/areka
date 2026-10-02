# 設計の検証: areka-P0-alpha-release-signoff

> 2026-10-01・`kiro-validate-design`（対話なしで実行）。読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・`.kiro/steering/`（`product.md`・`workflow.md`・`roadmap.md` の該当の行）。設計の期待を支える file:line は、ワークツリーの HEAD `d85dfade` で読み直して確かめた（下の「確かめた主張」）。

## まとめ

設計は「新しいコードを書かず、既存の道具（`tools/package-alpha.ps1`・`tools/test-all.ps1`）の上に説明書・検体・文書 2 本だけを載せる」形で、要件 1〜7 の全条項に担い手と手順が当たっている。期待を支える出どころを読み直したところ、決め手になる主張はすべてソースと一致した。直すべき点は、同じ走行に複数の項目が入るのに記録の件数をどの範囲で数えるかが決まっていないことなど 3 つで、どれも受入記録の書き方の補いで済み、設計の骨組みは変わらない。

## 確かめた主張（決め手になるもの）

| 設計の主張 | 確かめた結果 |
|---|---|
| 起動記録が無ければ `OnFirstBoot` が最優先で、`Halted` でも `OnBoot` を送らない | 一致。`crates/areka-kanade/src/schedule/boot.rs` の `boot_root` は `config.first_boot` を最初に見て、`Plain`・`Halted` は根なし（`OnBoot` だけ） |
| `Halted` の `OnBoot` は Ref6＝`halt`・Ref7＝落ちたゴーストの名前 | 一致。`crates/areka-kanade/src/schedule/events.rs` の `on_boot` が 6 個まで空で埋めてから `halt` と名前を足す。`crates/areka/src/main.rs` の `first_boot_origin` が印の値を `Halted { ghost_name }` にする |
| Ref7 は空にならない | 一致。読み手は `crates/areka-sylphya/src/persist/format.rs` の `last_running: read_last("running").filter(...)` で空を捨て、書き手は `crates/areka/src/boot_resolve.rs` の `running_name`（`name`、無ければフォルダ名） |
| 切替の後の定常で印が今のゴーストへ書き換わる | 一致。`crates/areka/src/emo2_boot/ghost_switch.rs` の `record_steady_memory` が `session_mark_steady` を出して印を投函し、その後に `ghost_switch_done` が出る（投函は書き手への送り出しで、書き込みは非同期） |
| 窓の位置を保存するのは掴んで離したときだけ | 一致。`char_pos_entries` の呼び手は `crates/areka/src/placement/follow/drag_follow.rs` の「char DragEnd 保存」だけ。保存先はゴーストのスコープ（`placement/persist.rs` の `persist_entries`） |
| 新しい `emo2.nar` は `halt` の台詞を持ち、`emo2-kakukaku` の `homeurl` を消している | 一致。ghost_dev `e2df8cb` の `release/emo2/emo2.nar` は 4,586,381 バイト・md5 `3f5d8777…`・111 項目・全項目 deflate。`boot.lua` は「Reference6 が `halt` かつ Reference7 が空でない」で「起動halt」へ、`boot.pasta` に `＊起動halt` が 3 つ。`install.txt` に `balloon.directory,emo2-kakukaku` が残り、`emo2-kakukaku/descript.txt` に `homeurl` の行は無い |
| 利用条件のファイル名 | 一致。`crates/areka/src/install/terms.rs` の `TERMS_FILES`＝`terms.txt`・`terms.md`。`R_POST_and_KOMAINU.nar` と `claudia.nar` はどちらも持たない（一周で画面は出ない） |
| シェル・バルーンの切替で `OnShellChanged`・`OnBalloonChange` を送る | 一致。`crates/areka/src/emo2_boot/frame/switch.rs` の `raise_changed` の 2 か所 |
| 配布スクリプトの版・7 桁・`dirty`・許可表・判定 7 と 8 | 一致。`tools/package-alpha.ps1` の `$SCRIPT_VERSION`・`git rev-parse --short=7`・`git status --porcelain` の行数・`$allowed`・判定 7／8 |
| メニューの枠 7 つの並び・`derive_scopes` が `[0, 1]` 固定・既知の制限の裏付けの定数（7 日・更新のオプション 3 つ） | 一致 |
| 付随の確認で台詞を足す `R_POST_and_KOMAINU` の辞書 | 一致。`ghost/master/dic02_Event.txt` に `＊OnShellChanged` が在り、`readme.txt` も同梱している（`\![open,readme]` で開くものが在る） |

細かな食い違い（判定に影響しない）: 設計の `file_drop.rs:79` は `crates/areka/src/input_events/file_drop.rs`、`boot_resolve.rs:402` の `session_mark_cleared` は `clear_session_mark` の中の数行下。受入記録に写すときは「何の定義行か」で指せば足りる。

## 直すべき点（3 つ）

### 1. 同じ走行に複数の項目が入るのに、記録の件数をどの範囲で数えるかが決まっていない

- **問題**: 検証項目表の証跡のいくつかは「件数」で書かれているが、走行 A2・A3 には同じ記録を出す操作が複数入っている。走行全体で数えると期待が偽になる。
  - 項目 5「`ghost_switch_done` が 3 件」→ A2 は項目 13 の前に `claudia` へもう 1 度替えるので、走行全体では 4 件。
  - 項目 8「`OnGhostChanged`／`OnBoot` が 0 件」→ A3 は冒頭に項目 13 の `OnBoot` が在り、項目 8 の後に `R_POST_and_KOMAINU` へ替える `OnGhostChanged` も在る。
  - 項目 6 の `skin_switch_done` は、項目 10 の準備（もう 1 度 `second`・バルーンの切替）と混ざる。
  - 項目 4「切替の行が 0 件」も、A2 の後半の切替と混ざる。
- **影響**: 受入記録を書く段で「数が合わない」か「都合のよい範囲で数えた」になり、要件 3.5（根拠を時刻つきで逐語に引く）と 2.1 の合否が後から辿れなくなる。
- **直し方**: 受入記録 §5 と §6 に「各項目の件数は、その項目の操作の始まりから次の項目の操作の始まりまでの区間で数える」と書く。区間の境目は、開発者が操作の直前に `runs.txt` へ 1 行（項目の番号と時刻）を足すか、操作で必ず出る記録の行（例: メニューの選択の行）の時刻で決め、§7 に区間の始まりと終わりの時刻を項目ごとに書く。
- **関わる要件**: 2.1（項目 4・5・6・8）・3.5
- **設計の箇所**: 「検証項目表」の「証跡の取り方」の列・「走行の手順」の A2・A3

### 2. 項目 13 の強制終了の合図と、期待する名前の取り方

- **問題**: 設計は A2 の終わりで「`claudia` へ替えて定常に入るのを待つ（`ghost_switch_done`）→ 強制終了」とし、A3 で「`session_mark_found` の `ghost` が落としたゴーストの名前」を見る。だが、印に書かれるのはフォルダ名の `claudia` ではなく `descript.txt` の `name` で、書き込みは書き手へ投函するだけの非同期である（`record_steady_memory` の `persist_put`）。
- **影響**: 「期待する名前」を手で `claudia` と書くと、正しく動いていても名前が合わず不合格に見える。投函の直後に止めると、まれに 1 つ前のゴーストの名前が残り、原因の分からない食い違いになる。
- **直し方**: 強制終了の合図を `ghost_switch_done` ではなく `session_mark_steady` の行（`ghost=` が切り替えたゴーストの名前）にし、その行が出てから数秒置いてから止める。期待する名前はその行の `ghost=` の値を逐語で写し、A3 の `session_mark_found` の `ghost=` と `OnBoot` の参照の 8 番目をそれと比べる、と §5 と §6 に書く。
- **関わる要件**: 2.3・3.9・6.4
- **設計の箇所**: 「走行の手順」の A2・「検証項目表」の項目 13・「Error Handling」の安全弁の行

### 3. 説明書の「内部の言葉を出さない」の機械の確かめが、新しい欄だけに当たっている

- **問題**: 要件 4.8 は説明書の全文に掛かるが、設計で機械の確かめ（語の探し方）を当てるのは §6.2 を写す新しい欄だけで、ほかの欄は §10 の突き合わせの表（行ごとの裏付け）に任されている。今の説明書の「■ .nar の入れ方」には spec 名（`alpha-release-signoff`）がそのまま書かれており、まさにこの種の語が残りうる。
- **影響**: 欄を書き足す仕上げの途中で、spec 名・イベントの名前（括弧の外）・環境変数の名前などが紛れても気付く手段が目視だけになる。zip を組んだ後に見つかると、要件 1.3 の組み直しになる。
- **直し方**: §10 に「説明書の全文に語の探し方を当てて 0 件」を足す。探す語は少なくとも spec 名の形（`areka-`・`alpha-` で始まる名前）・crate やソースの名前・`AREKA_`・`RUST_LOG`・`event=`・括弧の外の `On` で始まるイベントの名前。較正として、仕上げる前の説明書（57 行目の spec 名）に当てると 1 件以上当たることを確かめる。
- **関わる要件**: 4.8・4.7
- **設計の箇所**: 「§6.2 の写し方」の「写せたことの確かめ方」・「Testing Strategy」の「文書の機械の確かめ」

## 良いところ

1. **走行の順序が、観測の前提を正しく組み立てている。** 項目 13 を項目 8 より前に置いて「zip の `emo2` が話した」と言えるようにし、A3 の `emo2` が A1・A2 で起動記録を持つので `OnFirstBoot` ではなく `OnBoot` が送られる（台詞の条件が成り立つ）ことまでソースで詰めてある。項目 12 は窓を掴まずに観測し、項目 10 の前提（既定でないゴースト・シェル・バルーンと掴んで離した位置）を A3 で作る、と前後の依存がすべて手順に落ちている。
2. **期待の出どころを実物で固めている。** 期待の裏付けが「何の定義行か」つきの file:line で、今回読み直した決め手の主張はすべて一致した。新しい `emo2.nar` も配布物そのものを開いて、`halt` の台詞・`homeurl` の削除・`install.txt`・判定 7 の 2 本・圧縮の方式まで確かめてあり、差し替えでつまずく見込みが低い。

## 判定

**GO**

- **理由**: 要件 1〜7 の全条項に担い手と手順があり、決め手になる主張はソースと一致し、製品のコードと配布スクリプトに手を入れない範囲も守られている。上の 3 つは受入記録の書き方の補いで、設計の骨組みや段取り（バグの PR の着地と main の取り込みを待ってから zip を組む）を変えない。
- **次の段**: 設計の討議（`/kiro-design-discussion areka-P0-alpha-release-signoff`）で上の 3 つを扱い、受け入れたものを設計の「検証項目表」「走行の手順」「§6.2 の写し方」に反映してから、`/kiro-spec-tasks areka-P0-alpha-release-signoff` でタスクを分解する。開発者の段取りどおり、タスク分解まで進めたところで止め、バグ `balloon-reappear-short-talk` の PR が main に入ってから main を取り込んで実装へ進む。
