# ギャップ分析: default-balloon-selfmade

> 2026-10-29 `/kiro-validate-gap`（main `932d2c30` の上のワークツリー `claude/kiro-start-default-balloon-selfmade-3aaac2`）。要件（requirements.md）が書いた事実を引き直し、要件が回してきた「`StayseeBalloon` を綴るファイルの仕分け」をここで行う。SSP は動かしていない（測るのは要件の討議で、開発者の許しを取ってから）。決めごとはせず、選び方と材料だけを書く。

## 1. まとめ

- **既定の差し替えは小さい**。本番コードで既定の名前を持つのは `crates/areka/src/boot_resolve.rs` 34 行目の定数 1 つで、参照は段 6（325〜326 行目）だけ（今も変わらない）。ただし、既定の名前に頼るテストは要件が挙げた 4 本でなく **6 本**（`boot_config_roots_tests.rs`・`boot_config_option_tests.rs` が増える）。うち `boot_config_option_tests.rs` は綴りが組み立ての 1 か所だけで、答えの比べには出てこないが、既定が外れると無作為の段の `no_pick` が panic して赤になる。
- **絵を描く道具は `areka-emo-atlas` の example に置くのがほぼ一択**。`image`（`png`・`webp` の機能）と `sample-ghost-kit`（dev）を両方持つのはこの crate だけで、ほかへ置くと `Cargo.lock` が変わる（要件 8.3 に反する）。動く WebP は前例 `gen_animated_samples.rs` の組み方（可逆の `VP8L` を `VP8X`・`ANIM`・`ANMF` に包む）がそのまま使えるが、今は 8×8 決め打ちなので、大きさを引数にする手直しが要る。
- **議題 3（絵の形式）に効く新しい事実**: 動かない WebP は今 WIC（Windows の読み手）で読んでいて、Windows に WebP の拡張が無い機械では読めない（`balloon-webp-names` の brief）。中身を固定するテストの焼き込み（`bake`）も WIC を通る。`.webp` を選ぶと、`balloon-webp-names` が「動かない WebP を `image` で読む」まで着地しないと、既定バルーンの面が機械によって出ない。
- **`validrect.right,0` は、正の絶対値で書けば読み手によらず同じ所に解ける**。areka は「0 以上はそのまま・負は反対の辺から」と読む（`region.rs` の `resolve_coord`）。あいまいなのは 0 だけで、`validrect.right,326`（面の幅）のような正の値や `-N` は SSP でも areka でも同じ意味になる見込み（SSP の測りで確かめる）。
- **検体として畳む所に 1 つ落とし穴**: 畳む道具 `fold-samples` は `git ls-files` で追跡しているファイルだけを入れる。さらに見張り `sample_path_guard_test.rs` は、登記した名前について `vendors/sample_ghost/<名前>/` という綴りを `.rs` に書くことを禁じる。絵の道具の出力先と「元の木（descript.txt など）をどこに追跡して持つか」を設計で決める必要がある（4.3 節）。

## 2. 今の作りの調べ（要件ごとの対応表）

| 要件 | 使える既存の物 | 足りない物・注意 | 区分 |
|---|---|---|---|
| 1 絵柄を選ぶ | 前例 `areka-P0-app-icon`（要件の段で絵を選ぶ）・`target\` の下を使う決まり | 下書きの道具そのもの（要件 8 の道具の早い版） | 足りない |
| 2 面の絵と寸法 | `image` の PNG／WebP の書き手（`areka-emo-atlas` の依存） | 角丸の箱・しっぽ・半透明の縁を描く処理は無い（新しく書く） | 足りない |
| 3 印の絵 | 動く WebP を組む前例 `crates/areka-emo-atlas/examples/gen_animated_samples.rs` 199〜300 行 | 8×8 決め打ち（`webp_full` の `w: 8, h: 8`・`VP8X` の `u24(8 - 1)`）。77×36 にするには大きさを引数にする | 制約 |
| 4 絵の形式 | 面の名前は `.png` だけ（`crates/areka-emo-present/src/balloon.rs` 45 行目 `FRAME_SUFFIX`）・動く絵は `image` の腕・動かない絵は WIC | `.webp` の名前は `balloon-webp-names` 待ち。動かない WebP は WIC が拡張なしで読めない | 制約 |
| 5 名前と設定 | パーサ `crates/areka-parsers/src/balloon/parse.rs`（知らない鍵は完全一致引きで黙って無視）・`charset,UTF-8` の読み | `use_self_alpha` は SSP の既定が 0（下の 3.4） | 注意 |
| 6 描画範囲 | `crates/areka-emo-text/src/region.rs`（495 行・`real-run-log-noise` の持ち物） | 0 の読みは SSP で未測定 | 不明（測る） |
| 7 説明書 | `StayseeBalloon` の `readme.txt` の前例 | 文面は新しく書く | 足りない |
| 8 道具と手順 | `gen_animated_samples.rs`・`fold-samples.rs`・`sample-ghost-kit` の `fold_tree`／`NarBuilder`（公開） | 置き場と元の木の持ち方（4 節） | 足りない |
| 9 権利の記録 | `dist/README.txt` の同梱物の節・`vendors/sample_ghost/README.md` の出どころの節 | 新しい項 | 足りない |
| 10 検体の登記 | 登記表 `SAMPLES`（`crates/sample-ghost-kit/src/lib.rs` 132〜196 行）・`fold-samples` | 数のテストが 4 か所（5 節） | 足りない |
| 11 既定の差し替え | 定数 1 つ・`tools/package.ps1` 4 か所 | 既定に頼るテストは 6 本（3.1 節） | 足りない |
| 12 確かめ | `staysee_balloon_fixture` 一式（10 ファイル・計 2,872 行）・拡大率 1.0／1.25／2.0 | 新しいバルーン用の同じ形のテスト | 足りない |
| 13 守ること | 1,000 行の番人・見張り `sample_path_guard_test.rs` | 見張りが道具の綴りを縛る（4.3） | 制約 |

### 2.1 引き直した事実（要件の「要件生成時に引き直した事実」との突き合わせ）

- 定数 `DEFAULT_BALLOON_FOLDER` は 34 行目・参照は 325〜326 行目の段 6 だけ: **合っている**（`boot_resolve.rs` は 753 行）。
- 既定の名前を期待するテスト 4 本の綴りの数 15・6・3・4: **合っている**。ただし既定に頼るテストはほかに 2 本ある（3.1 節）。
- `tools/package.ps1` の 467・496・504・570 行目: **合っている**（752 行）。464〜467 行目で `emo2` と `StayseeBalloon` を窓口 `nar-sample-path` から引く。
- 登記表は 10 行（`StayseeBalloon` は `Vendored`）: **合っている**。
- `staysee_balloon_fixture` はテーマ別 8 ファイル＋`test_support.rs`＋本体: **合っている**。
- 文書の行（根の `README.md` 48 行目・`dist/README.txt` 379〜384 と 457〜460 行目・`vendors/sample_ghost/README.md` 21・32 行目・`product.md` 81 行目・`structure.md` 426 行目）: **合っている**。ほかにも既定として書く所がある（6 節）。

## 3. 調べの本体

### 3.1 `StayseeBalloon` を綴るファイルの仕分け（要件 11.3 が回してきた件）

`crates/` の下で `StayseeBalloon` を綴る `.rs` は 27 ファイル。テストはどれも定数を使わず、名前を文字で書いている。次の 3 つに分けた。

- **A＝直さないと赤になる**（既定の段が `StayseeBalloon` に当たることを前提にしている）
- **B＝直さなくても緑のままだが、「既定のバルーン」の意味で綴っている**（直すのが筋。直さないと、テストの名前やコメントが嘘になる）
- **C＝ある 1 つのバルーンの名前として使っているだけ**（直さない）

#### A: 直さないと赤になる（6 ファイル）

| ファイル | 行 | 何に頼っているか |
|---|---|---|
| `crates/areka/src/boot_resolve_tests.rs` | 302〜311（`balloon_default_staysee_when_listed_among_many`）・356〜366（`balloon_companion_not_listed_warns_then_default`） | 答えが `BalloonRoute::Default`＋`StayseeBalloon` |
| `crates/areka/src/boot_resolve_balloon_tests.rs` | 261〜276（`companion_first_missing_goes_to_default_not_second`）・283〜303（`companion_with_separator_is_not_rewritten`） | 同上 |
| `crates/areka/src/boot_config_balloon_tests.rs` | 21・31（組み立てで必ず `StayseeBalloon` を置く）・158〜165 | 答えが `Default`。組み立てを使うほかのテストも、descript などが当たらない場合は既定の段に頼り、外れると `no_pick` が panic する |
| `crates/areka/src/main_config_input_tests.rs` | 182（組み立て）・250〜256・277〜283・445〜451 | 答えが `Default`＋`StayseeBalloon`、起動の文脈のバルーンが `StayseeBalloon` |
| `crates/areka/src/boot_config_roots_tests.rs`（**要件に無い**） | 95〜122（`default_ghost_and_balloon_are_adopted_from_the_lower_root`）・166〜176 | 答えが `Default` で、下の根の `StayseeBalloon` のフォルダ |
| `crates/areka/src/boot_config_option_tests.rs`（**要件に無い**） | 26〜38（組み立てで `["StayseeBalloon", "b1"]`） | 答えの比べには出てこないが、ゴーストがバルーンを指さず、バルーンが 2 つなので段 6 に頼る。外れると段 7 の `no_pick` が panic する（`fn no_pick` は 13 行目） |

#### B: 緑のままだが、意味は「既定」（直すのが筋）

| ファイル | 行 | 中身 |
|---|---|---|
| `boot_resolve_tests.rs` | 6（モジュールの説明）・258・268（記憶・同梱が既定より先、の相手として並べる）・284〜300（`balloon_only_staysee_is_only_not_default`＝「既定の名前 1 つだけでも経路は `Only`」） | 名前を替えなくても通るが、「既定より先」を見るテストの相手が既定でなくなる |
| `boot_resolve_balloon_tests.rs` | 106〜111（`all_hit`「既定のバルーンも在る」）・160〜165（`companion_wins_over_default`） | 同上 |
| `boot_config_roots_tests.rs` | 131〜135・149〜153（同梱・記憶が既定より先） | 同上 |
| `crates/areka/src/boot_config_copy_tests.rs` | 56〜62（説明「バルーン `b1`・`b2`・既定を置き」） | どのゴーストも `default.balloon.path` を持ち、段 3 で当たるので既定は使われない |

#### C: 名前として使うだけ（直さない）

| ファイル | 中身 |
|---|---|
| `crates/areka/src/emo2_boot/ghost_switch_tests.rs`・`ghost_switch_fallback_tests.rs`・`crates/areka/src/menu/ghost_frame_tests.rs` | 起動の文脈を手で組む（経路の札は `Default` だが、解決は通らない）。名前は何でも通る。札と名前が「今の既定」の写しなので、揃えるかは好み（B に寄せてもよい） |
| `crates/areka/src/menu/balloon_frame_tests.rs` | メニューの「バルーン」枠の並びと印。大小の違う同名（`stayseeballoon`）の見分けにも使う |
| `crates/areka/src/shell_balloon_switch_session_balloon_tests.rs`・`shell_balloon_switch_session_update_tests.rs` | 検体 `StayseeBalloon` を窓口から引いて「2 つ目のバルーン」として使う（絵の大きさの違いで新旧を見分ける）。検体は残るので、このまま |
| `crates/areka-ghost/src/catalog_test_support.rs`・`catalog_tests.rs` | Shift_JIS の偽のバルーン（コメントに「偽の既定バルーン」とあるが、目録のテスト） |
| `crates/areka-sylphya/src/persist/format.rs`（説明の例）・`persist_tests.rs` | 記憶の値の例 |
| `crates/sample-ghost-kit/src/lib.rs`・`lib_tests.rs` | 検体の登記の行（残す。数のテストは 5 節） |
| `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` と `staysee_balloon_fixture/` | 中身を固定するテスト（要件 10.5 で変えない）。ただし本体の説明 9 行目「`StayseeBalloon` は areka の**既定バルーン**」は、議題 7 の答えで嘘になる（変えないと決めた以上、注記で済ませるか設計で決める） |
| `crates/pilot/examples/pilot-balloon-asset-swap/`（`main.rs`・`swap.rs`・`README.md`） | 先進坑の検体 A。そのまま |

**A の直し方の案**（設計で決める）:

- 案 1: 新しい名前の文字に置き換える（今の書き方のまま）。次に既定を替えるときも同じ 6 本を直す。
- 案 2: `crate::boot_resolve::DEFAULT_BALLOON_FOLDER` を使う形にする（`boot_config_option_tests.rs` は既に `DEFAULT_GHOST_FOLDER` をそう使っている）。次からは定数 1 か所で済む。ただし「定数の値を間違えても気付かない」ので、新しい名前を文字で比べるテストを 1 本だけ残す必要がある。

### 3.2 `validrect` の読み（areka と ukadoc）

- **areka**: `crates/areka-emo-text/src/region.rs` 420〜435 行。`resolve_coord(v, extent)` は `v >= 0` ならそのまま、負なら `extent + v`。成分が書かれていないときは左と上は 0、右と下は面の幅と高さへ縮める（そのつど `debug!`「未指定座標成分を既定辺へ縮退する」。毎フレーム出うる記録で、`real-run-log-noise` が直す対象）。だから `validrect.right,0` は「左端から 0」になり、幅 0 に潰れる。
- **ukadoc**（`descript_balloon`）:
  - `validrect.right`: 「テキスト描画範囲の右位置。」既定値の欄は「不明」。<https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#validrect.right%2C%E5%BA%A7%E6%A8%99>。`left`・`top`・`bottom` も既定値は「不明」。
  - 4 つとも脚注 *1 が付く。脚注の本文は ukadoc の MCP の索引に載っていない（引けなかった）。`region.rs` の説明は脚注を「マイナス座標はベース画像の右下からの相対」と引いている。**0 をどう読むかは脚注にも書かれていない見込み**＝SSP で測る理由。
  - `wordwrappoint.x`: 「未指定の場合は validrect.right まで書けるものとして扱う」。
  - `origin.x`: 既定は「横書き=validrect.left」。
- **読み手によらず同じ所に解ける書き方**: あいまいなのは 0 だけ。
  - 正の絶対値（例: `validrect.right,326`＝面の幅）。本体側の面はどれも幅 326 なので、全体の `descript.txt` に 1 つ書けば足りる。
  - 負の値（例: `validrect.right,-14`）。手本の `wordwrappoint.x,-20`／`-30` と同じ書き方になる。
  - 書かない（areka は面の幅へ縮めるが、毎フレームの `debug!` が出る。SSP の既定は「不明」）。
  - `left`・`top` の 0 は、どちらの読みでも左端・上端になる見込み（0 を「書いていない」と読む場合も、既定が左端・上端なら同じ）。SSP で `right` と合わせて確かめる。
- 手本の値を当てはめた見込み（面 0・326×169・areka の読み）: `bottom,-20` は 149、`origin` (14,11) と `wordwrappoint.x,-30`（面別）＝296 は、`right` を 326 か `-N`（N≤30）にすれば範囲の内に入る。相方側（326×96）は `balloonk0s.txt` の `validrect.bottom,-14` で 82。

### 3.3 `windowpos` と `windowposition`（議題 4）

- ukadoc の鍵の名前は `windowposition.x`・`windowposition.y`（既定 0）と `windowposition.limit`（既定 1）。「SSPに限り、descript.txtにも書ける」。<https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#windowposition.x%2C%E5%BA%A7%E6%A8%99>
- 手本の `windowpos.x,0`・`windowpos.y,0` は**正典の鍵の名前ではない**。areka のパーサは `windowposition.*` を読み（`crates/areka-parsers/src/balloon/model.rs` の `with_windowposition_raw`）、`windowpos.*` は完全一致引きで黙って捨てる。
- 値が 0＝既定なので、「書かない」と「`windowposition.x,0` と書く」は同じ意味になる。

### 3.4 そのほかエンジンの事実

- **`use_self_alpha`**: areka は宣言が無ければ「α<255 の画素が在れば α を使う」独自の決まり（`tech.md` 163 行目）。正典の既定は 0（α を捨てて左上の色を抜く）。**SSP で α どおりに見せるには `use_self_alpha,1` を書く必要がある**（議題 3 で PNG を選んだとき特に効く）。`StayseeBalloon` も 1 と書いている（`dist/README.txt` 402 行目）。
- 面 0 は必須（`resolve_balloon_faces`・無ければ `error!` で失敗）。面別の設定は面 0 だけ通る。
- `.pna` が在ると `warn!` が 1 行出る（要件 2.4 の「置かない」で避けられる）。
- 印（`arrow*`・`online*`・`sstp`）と `balloonc*` は読まない。知らない絵のファイルが在っても警告は出ない（列挙は接頭辞と `.png` で選ぶだけ）。
- `onlinemarker.filename` は「online と設定すると、online0・online1・online2…の順に画像を読み込み」、`onlinemarker.interval` は「50ミリ秒未満は設定できない」・既定 500 ミリ秒（ukadoc）。1 枚の動く絵で持つのは areka の拡張（議題 5・`balloon-markers` の brief 145〜149 行目に申し送り済み）。
- 名前の突き合わせ `find_balloon`（`boot_resolve.rs` 144 行目）は `same_folder_name`＝大小の違いを無視する作り（要件 5.2 の前提どおり）。

### 3.5 WebP を書けるか（外の依存の調べ）

- `areka-emo-atlas` の依存 `image = 0.25.10`（機能 `png`・`webp`）。`image::codecs::webp::WebPEncoder::new_lossless` で可逆の WebP を書け、`PngEncoder` で α 付き PNG を書ける。**新しい依存は要らない**。
- 動く WebP は `image` に書き手が無いので、前例は 1 コマずつ可逆で書いた `VP8L` を手で `VP8X`・`ANIM`・`ANMF` に包む（`gen_animated_samples.rs`）。繰り返し回数 0＝無限。コマの位置は偶数の制約がある（全面のコマを (0,0) に置けば問題ない）。
- 同じ版の `image`・`image-webp` なら同じバイト列になる（前例の説明 5 行目）。`image-webp` は根の `Cargo.toml` 92〜96 行目で git の固定コミットへ差し替えてあり、`webp_pin_tests.rs` が見張る。**版を上げるとバイト列が変わりうる**（要件 8.2 の「入力」に版を含めて考える）。
- 読み戻し（要件 8.9）: 動く絵の読み手 `crates/areka-emo-atlas/src/decode/image_arm.rs` の `read_frames` は `pub(crate)`。同じ crate の中のテストからなら直接呼べる。外からなら公開の焼き込みの道（`bake_with_limits`）を通す。

## 4. 作り方の選択肢

### 4.1 絵の道具の置き場

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| **A: `areka-emo-atlas` の example（推し）** | `crates/areka-emo-atlas/examples/<道具>.rs`（前例 `gen_animated_samples.rs` の隣） | `image` と `sample-ghost-kit`（dev）が両方使える・読み戻しのテストを同じ crate の中に置ける・`Cargo.lock` が変わらない | 前例の動く WebP の組み方は example 同士で共有できない（写すか `#[path]` で読む）。1,000 行の上限に注意（前例は 476 行） |
| B: `sample-ghost-kit` の example | 畳む道具の隣 | 畳む所と近い | `image` を持っていない＝足すと `Cargo.lock` の依存の並びが変わる（要件 8.3 に反する） |
| C: 新しい crate | 道具だけの crate | 責務が分かれる | ワークスペースに足すと `Cargo.lock` が変わる（要件 8.3 に反する） |

### 4.2 書庫（`.nar`）までの道

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| 1: 道具が書庫まで作る | 道具が絵と、道具の中に持った文（`descript.txt` など。定数か `include_str!`）から、公開の `fold_tree`／`NarBuilder` で `vendors/sample_ghost/<名前>.nar` を直に書く | 手順が 1 本・元の木を追跡しない（README の「展開した木は追跡しない」に合う） | 往復の確かめ（`fold-samples` の `--check`）は追跡した木が要るので使えない＝道具の側で同じ確かめを持つ |
| 2: 元の木を別の場所で追跡し、`fold-samples` で畳む | 例えば道具の隣のフォルダに `<名前>/`（文と絵）を置いて追跡し、`fold-samples --from <その場所>` | 今の手順と道具がそのまま使える・文を人が直しやすい | 絵が木と書庫で 2 重に入る（数十 KB 程度の見込み）。フォルダ名が書庫の名前になる |
| 3: README の手順どおり（いったん `vendors/sample_ghost/<名前>/` で追跡→畳む→外す） | 今の検体と同じ | 前例どおり | 文の元が書庫の中にしか残らず、作り直しのたびに取り出す手間がある（要件 8.7 の手順が重くなる） |

### 4.3 落とし穴（どの案でも）

- 見張り `crates/log-capture-kit/tests/sample_path_guard_test.rs` の形 ⑵ は、登記した名前について `vendors/sample_ghost/<名前>/` と `vendors/sample_ghost/<名前>"` の綴りを `.rs` で禁じる（139〜146 行目）。道具が展開形をそこへ書く作りにすると、登記した時点で赤になる。書庫 `vendors/sample_ghost/<名前>.nar` を指す綴りは当たらない。
- `fold-samples` は追跡しているファイルだけを入れる（`git ls-files`・0 件なら止まる）。`target\` の下の作りかけは畳めない。
- `vendors/sample_ghost/` は `.gitattributes` が `* -text`（改行を変えない）。案 2 の元の木を別の場所に置くなら、同じ手当て（UTF-8・改行を固定する）が要る。根の `.gitignore` は `.png`・`.webp`・`.txt` を除外していない（確かめた）。

### 4.4 混ぜ方（推しの組み合わせ）

道具は案 A。書庫までは、案 1（道具が書庫まで書き、往復の確かめも道具が持つ）か案 2（元の木を追跡して今の `fold-samples` を使う）のどちらか。案 1 は文を道具の中に持つので「1 か所を直せば作り直せる」、案 2 は「今の手順の再利用」が強み。どちらも要件 13.1（本番コードの変更は定数と登記表だけ）に収まる（example とテストは本番コードでない）。

## 5. 登記と数のテスト（要件 10.1）

10 → 11 で直す所（`crates/sample-ghost-kit/src/lib_tests.rs`・783 行）:

- 137〜157 行（`current_rows_are_registered_as_vendored`）: Vendored の並び 9 → 10（メッセージ「今の 9 行」も）。
- 209 行: `SAMPLES.len()` が 10。
- 263〜276 行（`unknown_sample_fails_with_all_ten_known_names`）: 名前の並び 10 → 11（テストの名前の `ten` も）。
- 440〜446 行: バルーンの種類の並びに新しい名前を足す。
- 499〜503 行（`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`）: 10。
- 文書: `vendors/sample_ghost/README.md` 9 行目「今の 9 本」と 13〜23 行目の表、`structure.md` 426 行目「10 本」。
- 見張り `sample_path_guard_test.rs` は登記表から走査語を組むので、直す所は無い。

## 6. 配布の道具と文書

### 6.1 `tools/package.ps1`

- 464〜467 行: `Read-SamplePaths` で窓口 `nar-sample-path` から検体を引く（新しい名前を同じ形で 1 行足す）。
- 496 行: 必須の項目（`balloon/<名前>/descript.txt`）。
- 504 行: `balloon/` 直下の許可の表。
- 569〜570 行: 写す行。
- 議題 7 で `StayseeBalloon` を外すなら、467・496・504・570 行のすべてから外す（要件 11.5）。
- 注意: 配布物の初回起動の確かめ（638〜643 行）は `emo2` の同梱 `emo2-kakukaku`（経路 `Companion`）を見るだけで、**既定の段は配布物の確かめでは通らない**。既定の段の当たりは単体テストだけが見ている。

### 6.2 既定と書いている文書（要件 11.7 の列挙の外も含む）

| 場所 | 中身 | 要件の列挙 |
|---|---|---|
| 根の `README.md` 48 行 | 「既定のバルーンは CC0 の StayseeBalloon」 | あり |
| `dist/README.txt` 376〜386 行（同梱しているバルーン）・457〜460 行（同梱物とライセンス） | 名前・フォルダ・作者・条件 | あり |
| `dist/README.txt` 402 行 | `use_self_alpha` の説明で「同梱している Balloon for Staysee Syncfield は 1 と書いてあり」 | **無い**（議題 7 で外すなら嘘になる） |
| `vendors/sample_ghost/README.md` 21・32 行 | 「areka の既定バルーン」 | あり |
| `.kiro/steering/product.md` 81 行 | 「既定バルーンは CC0 の StayseeBalloon に確定」 | あり |
| `.kiro/steering/structure.md` 426 行 | 検体の顔ぶれ・「areka の既定バルーン・CC0」 | あり |
| `.kiro/steering/tech.md` 166 行 | 「配布物へ入れてよいのは CC0 の既定バルーン StayseeBalloon のように…」 | **無い**（言い回しの例。直すかは設計で） |
| `doc/COMPAT_ARCHITECTURE.md` 381 行 | 段の並びの説明「既定（`StayseeBalloon`）」 | **無い** |
| `doc/pasta-real-run.md` 56〜84・160 行 | 実機の根を「配布物と同じ形」に組む手順で `StayseeBalloon` を写す | **無い**（配布物の形を写す手順なので、議題 7 の答えに合わせて直す候補） |
| `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` 9 行 | 説明「areka の既定バルーン」 | 要件 10.5 で変えない（3.1 の C） |
| `.kiro/specs/balloon-markers/brief.md` 61・69 行 | 「既定バルーン（Staysee）」 | 要件 3.7 の申し送りで触る |

完了の spec の文書と `roadmap.md`／`roadmap-history.md` は直さない（要件 11.8・roadmap は `/kiro-next` の持ち物）。

## 7. 議題 7（`StayseeBalloon` を配布物から外す）の材料

- 記憶の段で指す先が無いとき: `warn!`（`last_balloon_not_found`・`boot_resolve.rs` 274 行目）を 1 件残して次の段へ進む（`boot_config_roots_tests.rs` 166〜176 行目が「記憶の先が無い→既定」を固定）。emo2 なら段 4（同梱 `emo2-kakukaku`）、バルーンを持たないゴーストなら段 6 で新しい既定に当たる。止まらない。
- **調べが要る（Research Needed）**:
  - zip を前の版のフォルダへ上書きで展開した利用者の手元には、古い `balloon/StayseeBalloon/` が残り、記憶の段が当たり続ける（配布物から外しても見えなくならない）。
  - winget で上げたとき、前の版のフォルダの中身が消えるか残るか。
  - 書ける根（`%LOCALAPPDATA%\areka`・`crates/areka/src/roots/user_root.rs` の `copy_for_write`）へ、バルーンのフォルダが写される場面があるか（写されていれば、配布物から外しても書ける根に残る）。

## 8. 規模と危うさ

- **規模: M**（8〜12 タスク）。差し替えは小さいが、絵の道具（描く・動く WebP を組む・書庫まで）と、`staysee_balloon_fixture` と同じ形の中身のテスト（前例は 10 ファイル 2,872 行）が重い。`validrect` の読みを areka の側で直すことになれば +1〜2（ただし `real-run-log-noise` の後）。
- **危うさ: 中**。技術は既知（前例あり・新しい依存なし）。不確かなのは、⑴ 議題 3 で `.webp` を選んだときの `balloon-webp-names` への待ち（動かない WebP の読み手の差し替えまで含めて）、⑵ SSP の `validrect.right,0` の読み、⑶ 並走する spec との同じファイル（`boot_resolve.rs` を作り直す `multi-ghost-runtime`・`shell-companion-balloon`、`region.rs` の `real-run-log-noise`、`dist/README.txt` の `startup-install-option`・`bridge-store-user-root`、根の `README.md` の `winget-release-automation`）。

## 9. 設計の段へ持ち越すこと

### 9.1 推しの形

- 道具は `areka-emo-atlas` の example。動く WebP の組み方は前例を大きさの引数つきで写す。
- 中身のテストは `crates/areka-emo-text/tests/` に `staysee_balloon_fixture` と同じ形で足す（検体は窓口から名前で引く）。読み戻しのテスト（要件 8.9）は `areka-emo-atlas` の中に置く（`read_frames` が `pub(crate)` のため）。
- `validrect` は正の絶対値か負の値で書く（0 を書かない）。`use_self_alpha,1` を書く。`windowpos.*` は持たない（または正典の名前 `windowposition.*` で 0）。
- 既定の差し替えは最後のタスク・直前に main を取り込む（要件 11.9）。

### 9.2 調べが要ること（Research Needed）

1. SSP の `validrect.right,0`（と `left`・`top` の 0）の読み（要件の討議の最初・開発者の許しを取ってから）。
2. SSP が `.webp` の名前の面・印を読むか（議題 3・5 の説明書の書き方に効く）。
3. `balloon-webp-names` の着地の見込みと、動かない WebP を `image` で読む作りが入るか（議題 3 で `.webp` を選ぶ場合の前提）。
4. 7 節の 3 点（上書き展開・winget・書ける根への写し）。
5. 新しいバルーンを流し込んだときの、拡大率 1.25・2.0 での折り返しの期待値（`staysee_balloon_fixture/scale.rs` の手計算の形に合わせる）。

### 9.3 設計で決めること（要件の討議で設計へ回した）

1. 絵の道具の置き場（4.1・推しは `areka-emo-atlas` の example）。
2. 書庫（`.nar`）までの道（4.2 の案 1〜3）と、元の木（`descript.txt` など）をどこで持つか（4.3 の見張りと `fold-samples` の決まりに当たらない形）。
3. 仕分け A の 6 本の直し方（3.1 の案 1＝新しい名前を文字で書く／案 2＝定数を参照し、名前を文字で比べるテストを 1 本だけ残す）。C のうち手で組んだ `Default` の札をそろえるか。
4. 動く WebP の読み戻しのテスト（要件 8.9）の置き場（3.5・推しは `areka-emo-atlas` の中）。
5. 配布物の確かめ（`tools/package.ps1` 638〜643 行）に既定の段の当たりを足すか（6.1）。
6. `staysee_balloon_fixture_test.rs` 9 行目の説明「既定バルーン」の扱い（要件 10.5 で中身は変えない。注記で済ませるか）。
## 10. 設計の段（2026-10-29 `/kiro-spec-design -y`）

### 10.1 調べの範囲と使った物

- 種類: **既存の作りへの足し（Extension）**＝軽い調べ。新しい依存が無いので外の調べ（Web）はしていない。
- 読んだ物: `kiro-spec-design` の決まり（設計の原則・軽い調べ・まとめの 3 つの目・設計の門）、steering の決まり（ファイルの 1,000 行・テストの置き方・窓口）、下書きの道具 `target\balloon-draft\src\main.rs`、前例 `crates/areka-emo-atlas/examples/gen_animated_samples.rs`、`crates/sample-ghost-kit/examples/fold-samples.rs`・`src/nar_writer.rs`（`fold_tree`・`NarBuilder`・`install_txt`）・`src/lib.rs`（`SAMPLES`）、見張り `crates/log-capture-kit/tests/sample_path_guard_test.rs`、`staysee_balloon_fixture` 一式、`crates/areka-emo-atlas/src/decode.rs`・`decode/wic_arm.rs`・`decode/image_arm.rs`、`crates/areka-emo-text/src/actor_attach.rs`（警告の条件）、`tools/package.ps1`、`doc/pasta-real-run.md`、SSP に入っている手本 `C:\wintools\ssp\balloon\bottle\`（読むだけ）。
- 使ったスキル: `kiro-spec-design`（その決まりのファイル 5 つ）だけ。絵柄は要件の討議で決まっているので、絵作りのスキルは使っていない。

### 10.2 手本 `bottle` の値（SSP の `balloon\bottle\`・Shift_JIS で読んだ）

- `descript.txt`（`charset` の行なし）: `type,balloon`・`name,ボトル（でふぉ改）`・`id,bottle`・`craftman,MAz`・`origin.x,14`・`origin.y,11`・`validrect.left,0`・`top,0`・`right,0`・`bottom,-20`・`wordwrappoint.x,-20`・`wordwrappoint.y,0`・`font.name,ＭＳ ゴシック`・`font.height,12`・`font.color` 0/0/0・`cursor.blendmethod,mergepennot`・`cursor.style,square`・`cursor.brush.color` 22/143/231・`cursor.pen.color` 8/8/194・`cursor.font.color` 201/255/255・`arrow0.x,303`・`arrow0.y,10`・`arrow1.x,303`・`arrow1.y,-20`・`onlinemarker.x,4`・`onlinemarker.y,-39`・`sstpmarker.x,10`・`sstpmarker.y,-15`・`sstpmessage.x,20`・`sstpmessage.y,-17`・`sstpmessage.font.name,ＭＳ Ｐゴシック`・`sstpmessage.font.height,12`・`sstpmessage.font.color` 56/163/163・`number.font.name,Times New Roman`・`number.font.height,14`・`number.font.color` 32/76/120・`number.xr,-25`・`number.y,-18`・`communicatebox.x,8`・`communicatebox.y,21`・`communicatebox.width,385`・`communicatebox.height,20`。
- 面別の設定: `balloons0s.txt`（origin 14,11・`windowpos` 0,0・wordwrappoint -30,0・`onlinemarker.x,3`・`sstpmarker.x,10`・`sstpmessage.x,20`）、`balloons1s.txt`（origin 23,11・`windowpos` 0,0・wordwrappoint -20,0・`onlinemarker.x,12`・`sstpmarker.x,19`・`sstpmessage.x,29`）、`balloonk0s.txt`（origin 14,14・`validrect.bottom,-14`・`windowpos` 0,0・wordwrappoint -30,0）。`balloons2s.txt`・`balloons3s.txt`・`balloonk1s.txt` も在る（要件 5.5 の値）。`balloonc*s.txt` は無い。
- 絵のファイル: `balloons0`〜`3`・`balloonk0`〜`1`・`balloonc0`〜`3`・`arrow0`〜`1`・`sstp`・`online.png` と `online0`〜`online15`。
- この値は要件 5.4〜5.6 の値と合っている。設計の `texts.rs` はこれを写し、`charset`・`name`・`id`・`craftman`・`use_self_alpha`・`validrect.right` だけを替え、`windowpos.*` を落とす。

### 10.3 新しく分かったこと

- **動く WebP は公開の読み口で外から読める**: `areka_emo_atlas::WicDecoderArm` は `ElementDecoder::probe_animation`（先頭の署名で APNG と WebP を見分ける `sniff`）と `decode_frames`（`image` の腕）を実装していて、`AnimatedImage`・`AnimationFrame`・`LoopCount` も公開されている。ギャップ分析 3.5 の「`read_frames` は `pub(crate)`」は、この公開の道で回避できる。COM の初期化（MTA）が要る（`staysee_balloon_fixture/bake.rs` と同じ）。
- **6.5 の 3 つの警告は装着の口で出る**: 潰れ（`actor_attach.rs` の `note_region_collapse`・条件は `TextRegion::is_collapsed`）、折り返し基準（`warn_coarse_wrap_threshold`・条件は `wrap_threshold() > inline_limit()`）、origin（`region.rs` 452 行目あたりの解決の中・宣言を捨てる）。純粋なテストでは、同じ条件が偽であることで確かめられる。実機は起動の記録を grep する。
- **端末の文を ASCII に保つ所**: Windows の `std::io::Error` の `Display` は OS の日本語の文を含む。道具は `ErrorKind` と OS のエラー番号だけを出す。
- **`hypot` は環境に依る**: 下書きは `f32::hypot` を使っているが、これは環境の数学ライブラリの答えで、機械によって最後の桁が違いうる。`sqrt` は正しく丸められるので、道具は `sqrt(x*x + y*y)` で書く（要件 8.2）。
- **一時のファイルの置き場**: 要件 13.8 は一時のファイルを `target\` の下だけに置くことを求める。書き出し先の隣に一時名を置く形は使えないので、`target\gen-default-balloon\` に書いてから名前を替える（同じドライブ）。
- 要件 7.3 の「要件 3.6 と 4.3」の 4.3 は在らない番号。文脈から入力の箱（要件 2.7）を指すと読んだ（設計の Requirements Traceability の注）。

### 10.4 まとめの 3 つの目（設計の synthesis）

- **まとめられるもの**: 面 10 枚と印 3 種と動く印のコマは、どれも「距離関数を 4×4 の標本で塗る」1 つの描き方の場合分け（下書きと同じ）。描き方の関数は 1 つにし、面と印は表で回す。文のファイル 9 つも「行の並び → CRLF の UTF-8」の 1 つの形。
- **作るか借りるか**: 書庫は `NarBuilder`、CRLF の行は `install_txt`、PNG と VP8L は `image`、動く絵の読み戻しは `WicDecoderArm` を借りる。作るのは描き方（下書きの写し）と動く WebP の包み（前例の写し・大きさを引数に）だけ。`fold-samples` は使わない（展開形を追跡しないため・9.3 の決め 2）。
- **削ったもの**: 道具の展開した木の書き出し（窓口で取り出せる）、配布物の確かめへの既定の段の追加（9.3 の決め 5）、`areka-emo-atlas` の中の読み戻しのテスト（中身のテストへまとめた）、`gen_animated_samples.rs` の作り直し（写すだけにし、前例の検体のバイト列を動かさない）、`staysee_balloon_fixture` のテーマ別ファイルの説明の直し（入口の注記 1 段だけ）。

### 10.5 9.3 の 6 件の決め（詳しくは design.md「設計で決めたこと」）

| 件 | 決め | 選ばなかった案 |
|---|---|---|
| 1 道具の置き場 | `areka-emo-atlas/examples/gen_default_balloon/`（main と子 3 つ） | `sample-ghost-kit` の example・新しい crate（`Cargo.lock` が変わる） |
| 2 書庫までの道 | 道具が `NarBuilder` で直に書く・文は道具の中の行の並び・`--check` で同じかを見る | 元の木を別に追跡して `fold-samples`（絵が 2 重・改行の手当て）・README の手順で一時に追跡（作り直しが重い） |
| 3 既定に頼るテスト | 定数を参照し、名前を文字で比べるテストを 1 本残す・C は変えない | 新しい名前を文字で 6 本に書く（次の差し替えでまた 6 本直す） |
| 4 読み戻しの置き場 | 中身のテストの `online.rs`（公開の `WicDecoderArm` で書庫の中の絵を読む） | `areka-emo-atlas` の中（`read_frames` を直に呼ぶ・保管物でなく作りたての絵になる） |
| 5 配布物の確かめ | 既定の段の当たりは足さない | バルーンを持たないゴーストを配布物に足す（配布物の形が変わる） |
| 6 Staysee のテストの説明 | 入口の説明に注記 1 段 | 直さない（嘘が残る）・テーマ別ファイルまで直す（要件 10.5 から遠い） |

### 10.6 危うさ

- `image`・`image-webp` の版が上がると書庫のバイト列が変わりうる → 道具の `--check` で気付き、作り直す（設計の Revalidation Triggers）。
- 並走の spec と同じファイル（`boot_resolve.rs` のテスト・`dist/README.txt`・根の `README.md`）→ 差し替えを最後の 1 タスクにまとめ、直前に main を取り込む。
- 折り返しの期待値を手で導く所（`wrapping.rs`）で、折り返しの決まりの読み違えが起こりうる → 導き方を説明に書き、実測に合わせて緩めない（`staysee_balloon_fixture/wrapping.rs` と同じ規律）。
