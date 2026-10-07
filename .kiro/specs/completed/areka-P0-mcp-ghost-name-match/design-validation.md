# 設計の検証: areka-P0-mcp-ghost-name-match

> 2026-10-05・本ブランチ（`fb2b51fa`）の design.md を、requirements.md・research.md・steering と実物のコードに照らして確かめた。コードは「何の定義か」で指す。

## 設計の要約

照合の判断 `resolve`（`crates/areka/src/mcp/resolve.rs`）と、起動中のゴーストを読む `active` の 2 か所だけを直し、振り分け `dispatch`（`mcp/mod.rs`）と各ツールには触らない、という筋は実物と合っている。空文字・空白だけに特別な腕を置かず「前後の空白を除くと何にも当たらない」1 つの規則で `Cannot find` にする形も、要件 2.4（0 体を含む）を余さず満たす。実装へ進めてよい。直したほうがよい点は 1 つ（要件 1.4 の `sakura.name2` を、外れたら赤になるテストが 1 本も無い）。

## 実物で確かめたこと

- **字面の数**: `ActiveGhost {` を組んでいる所は 13 ファイル・17 か所。約束の外の 10 ファイル・12 か所（`get_active_ghost_list_tests.rs` 2・`get_log_tests.rs` 2〔見本 `emily` と `nameless_ghost_matches_records_named_by_its_full_path`〕・残る 8 ファイル各 1）、約束の内は `resolve.rs` 1・`resolve_tests.rs` 3（`named`・`unnamed`・`listed_value_is_full_path_without_trailing_separator_when_name_absent` の `trailing`）・`mcp_tests.rs` 1（見本 `ghost`）。設計の表と一致する。型は `pub(crate)` なので crate の外の字面は無い。
- **空白の除き方**: descript の値を読む `parse_kv`（`crates/areka-parsers/src/kv/parse.rs`）はキーと値に `str::trim` を使う。設計が名前の照合に同じ `str::trim` を使うので、除く文字の範囲（半角の空白・タブ・全角の空白を含む）が食い違わない（要件 2.1）。
- **本体側名の在りか**: 構造体 `GhostNames`（`crates/areka-parsers/src/package/model.rs`）が `name`・`sakura_name`・`sakura_name2`・`kero_name` を持ち、`GhostSession::names()` がそれを返す。`active` で `sakura_name` を取れば parsers も `ghost_session.rs` も触らずに済む。
- **省略と空文字の区別**: プロトコル側の `optional_string`（`crates/areka-mcp/src/tools/mod.rs`）は `args.get(key).and_then(Value::as_str)` で、欄なし・`null` は `None`、空文字は `Some("")` のまま届く。`resolve` の `None | Some("")` の腕から `Some("")` を外すだけで要件 2.4・3.1〜3.2 が分かれる。
- **`get_log`**: `answer`（`get_log.rs`）は `Some("")` を解決の前に `CANNOT_FIND` で断り、ほかは `Omitted::Reject` で解決して失敗を `CANNOT_FIND` に読み替え、成功なら `listed_value` で絞る。新しい規則でも答えは変わらず、空白だけ（`"   "`）は解決へ渡って `CANNOT_FIND` になる。注釈だけ直す設計で足りる。
- **書き換える古いテスト**: `resolve_tests.rs` の 5 本、`mcp_tests.rs` の `get_expression_table_omitted_is_not_active_even_with_one_ghost`（今は `[None, Some("")]` を回す）、`get_log_tests.rs` の `unmatched_empty_and_no_ghost_are_cannot_find`（今は `"emily/phase4.5"` を外れとし、空文字の注釈に「解決へ渡すと 0 体で Specified ghost is not active」とある）。設計の書き換え表と一致する。
- **survey §7.4**: 表の 8 行を設計の判断の流れに当てると、どれも同じ答えになる。前後に空白のあるフルパス（` C:\…\emo2\ `）は、`same_path` の正規化が末尾の区切りを空白の手前で落とせないので、渡された文字列のままで比べれば外れる（要件 2.3）。
- **実機確認の検体**: 配布の `emo2.nar` の descript は `name,えも？？`・`sakura.name,むらさき`・`kero.name,エモ` で、`sakura.name2` の行は無い。`name` に半角の英字が無いので `えも2DEBUG` へ書き替える設計の段取り（要件 6.2）は正しい。
- **本物の単位のリグ**: `SwitchRig` の `copy_ghost` は `name` をフォルダ名に、`sakura.name` を「フォルダ名のさくら」に書き替え、`kero.name` は検体の `エモ` のまま残す。設計の `real_unit_resolves_by_sakura_name_and_ascii_case` が当てる値（`Aのさくら`・`a`・`kero_name`）は実物にある。

## 直すべき点

### 1. 要件 1.4 の `sakura.name2` を守るテストが、どこにも無い

- **何が困るか**: 純粋なテスト `other_names_do_not_resolve` の入力は「`sakura.name2` に当たる別の綴り」だが、`ActiveGhost` は `sakura.name2` を持たないので、この入力はただの外れの名前（今の `eight_with_a_wrong_name_cannot_find` の `Someone else` と同じ）でしかない。`active` が後で `sakura_name2` も読むように変わっても、このテストは緑のまま。本物の単位のテストは `kero_name` だけを当て、検体の emo2 にも `copy_ghost` の書き替えにも `sakura.name2` の行が無いので、そこでも当たらない。つまり要件 1.4 の半分（`sakura.name2`）は「正しく動くが固定されていない」。
- **なぜ大事か**: 要件 1.4 と要件 5.1 は `sakura.name2` の不一致を固定するよう求めている。テストの名前と表が「固定した」と読めるのに、実際は外れたら赤になる檻が無い。
- **どう直すか**: `real_unit_resolves_by_sakura_name_and_ascii_case` の中で、`SwitchRig::new` の後・`rig.boot("A")` の前に、`rig.cfg("A").ghost_root` の下の `ghost\master\descript.txt` へ `sakura.name2,…` の 1 行を足してから起こし、その値で `CANNOT_FIND` になることを `kero_name` と並べて判定する（リグは変えない・足すのはテストの中だけ。`GhostNames` は `#[non_exhaustive]` なので、テストで字面を組んで純粋に試す道は無い）。純粋なテスト `other_names_do_not_resolve` は「名前・本体側名でない文字列は外れ」の意味に名前と表の説明を直す。
- **要件**: 1.4・5.1
- **design.md の場所**: 「Testing Strategy」の「照合の判断」の足すテストの表（`other_names_do_not_resolve`）と「本物の単位で通す」

## よい点

- **規則を 1 つに保った**: 空文字・空白だけに専用の腕を置かず、「名前は前後を除いて半角の英字の大小を畳んで比べる・フルパスは渡されたまま `same_path`」の 2 つの比べ方だけで survey §7.4 の全行と要件 2.4 の 0 体の場合まで説明がつく。変えるのは `resolve.rs` の 2 関数と型 1 つで、9 本のツールの答えが 1 か所でそろう。新しい依存も新しい失敗の種類も無い。
- **並走の衝突の扱いがはっきりしている**: `ActiveGhost` の字面を組むファイルを全数（10 ファイル・12 か所）挙げ、同じウェーブの ⑥ `mcp-get-status`・⑧ `mcp-dump-images-residue`・⑨ `mcp-author-tools` と重なる所を表に出し、「後から着地する側が 1 行足す・足し忘れはコンパイルが落ちる」で見落としが起きない形にしている。roadmap の C4 の約束（⑦ が触るファイル）とも一致する。

## 判定

**GO**

- **理由**: 判断の置き場・比べ方・省略と空文字の分け方・書き換えるテストの数え・触るファイルの全数が実物と一致し、要件 1〜6 のすべてに実現の手段がある。直すべき点 1 はテストを 1 行足して 1 本の名前を直すだけで、設計の筋を変えない。
- **次の一歩**: 設計ディスカッションで直すべき点 1 を決め（推し: 本物の単位のテストで `sakura.name2` を足して当てる）、design.md の「Testing Strategy」へ反映してから `/kiro-spec-tasks areka-P0-mcp-ghost-name-match` へ進む。
