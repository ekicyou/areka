# Brief: ghost-alias-file

> 起票: 2026-10-23（棚卸㉙・完了 spec の残り物から。出どころは `.kiro/specs/completed/areka-P0-dot-sakura-specimen/research.md` 11 行目の ⑴「別名の専用ファイル `alias.txt`（`sakura.surface.alias` を含む）を areka は読まない・担当の spec が無い」と、同 spec の `verification.md` 242 行目「`alias.txt` の別名は、面が出ていないので症状を見ていない」）。
> 版の段は「いつか」・区分 B（正典の別名が黙って落ち、どっとさくらの台本の面の指定が別の番号のまま解かれる見込み。実機の症状はまだ見ていない）。

## Problem

- **利用者**: 古い作りのシェルは、面の番号の付け替え（別名）を、シェルのフォルダの `alias.txt` か `surfaces.txt` の `sakura.surface.alias` に書く。areka はどちらも読まないので、台本が別名の番号や名前で面を指すと、違う面が出るか、面が出ない。SSP は別名どおりに出す。
- **どっとさくら（検体）**: `vendors/sample_ghost/dot_sakura.nar` のシェル 3 つ（`master`・`sakura100`・`sakura50`）は、どれも `alias.txt`（135 バイト）を持つ。中身は `sakura.surface.alias` が `9→7`・`21→6`・`23→6`・`26→20`・`44→0`、`kero.surface.alias` が `13→11`・`110→10`・`111→11`・`112→12`。シェル `master` に `surface9.png`・`surface21.png`・`surface23.png`・`surface26.png`・`surface44.png`・`surface13.png`・`surface110〜112.png` は無い＝台本が `\s[9]` と書くと、SSP は面 7 を出し、areka は無い面 9 を探す見込み。

## Current State

- **正典（ukadoc・2026-10-23 に MCP で引いた）**:
  - 「UKADOC Project シェル」: 「alias.txt 旧仕様。sakura(kero).aliasブレスを記述するためのテキストファイル。現在はsurfaces.txtに機能が統合されている。」＝今の正典でも、シェルのフォルダの `alias.txt` は読む対象で、中身は `surfaces.txt` の別名のブレスと同じ書き方。
  - 「\s[ID番号]」: 「surfaces.txtのsurface.aliasまたはnameで定義された文字列をIDの代わりに使用できる。」
  - 「name,定義名」（2.8.24）: 「サーフェスの名前。surface.aliasブレスと同様に扱われ、\s[]タグでIDの代わりに使用できる。」
  - 別名のブレスの書き方そのもの（`sakura.surface.alias`・`kero.surface.alias`）は surfaces.txt のページの説明の文にだけ出て、項目の見出しは無い（網羅台帳 `assets.toml` 8181 行目の注記と同じ）。別名の名前が数字のとき（どっとさくらの `9→7`）に、番号より別名を先に引くかは書いていない＝SSP で測る。
- **areka**:
  - surfaces.txt の読み手 `crates/areka-parsers/src/shell/decode.rs`（598 行）の `dispatch_block`（115 行目）は、別名のブレスとして `kero.surface.alias` だけを振り分ける（122 行目）。`sakura.surface.alias` は知らないブレスに落ちる。
  - シェルを読む所 `crates/areka-emo-present/src/shell_target.rs`（855 行）は `surfaces.txt` の 1 つだけを読む（61 行目の定数・327 行目）。`alias.txt` を開く所は `crates/` のどこにも無い。
  - 別名の表は 1 つで、キャラクターの区別が無い（`crates/areka-emo-compose/src/world.rs` 38 行目「`kero.surface.alias` の alias キー → 順序付き id リスト」）。本体と相方で同じ表を引く。
  - 面の指定を解く `crates/areka-seriko/src/resolve.rs`（314 行）の `SurfaceResolver::resolve` は、先に数字として読み、数字なら別名を見ずにその番号を返す。別名を引くのは数字でないときだけで、番号が複数なら先頭を返す（決定論を優先した既存の決め）。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` 5998〜5999 行目の注記: ⑴ 番号を複数書いても先頭だけ ⑵ 本体側の綴り `sakura.surface.alias` は crates のどこにも無い ⑶ `alias.txt` を読む道が無い。`name,定義名` の行（7256 行目）は absent・持ち主なし。

## Desired Outcome

- シェルのフォルダの `alias.txt` を読み、`surfaces.txt` の別名のブレスと同じに扱う（両方にあるときの勝ち負けは SSP で測って決める）。
- `sakura.surface.alias` を読み、本体の面の指定は本体の表、相方は相方の表で解く（キャラクターごとの表）。
- 別名の名前が数字のときの扱い（番号より別名が先か）を SSP と同じにする。どっとさくらで `\s[9]` が面 7 になることを確かめる。
- surface ブレスの `name,定義名` を同じ表に載せる。
- 決定論のテストで「`alias.txt` だけ」「両方にある」「本体と相方で同じ名前が違う番号」「数字の別名」「`name`」を固定する。

## Approach

- 最初の段は SSP で測る（開発者の機械の SSP の MCP）: 数字の別名と番号の勝ち負け・`alias.txt` と `surfaces.txt` の両方にあるとき・`char2.surface.alias` のような 3 人目からの綴りを SSP が読むか。
- `alias.txt` は `surfaces.txt` と同じ読み手に通す（書き方が同じ）。表はキャラクターの番号ごとに持ち、面の指定を解く所へキャラクターの番号を渡す。

## Scope

- **In**: シェルのフォルダの `alias.txt` の読み・`sakura.surface.alias`・キャラクターごとの別名の表・数字の別名の扱い・`name,定義名`・網羅台帳の行（5998 行目の項目・`name` の行）・決定論のテスト・どっとさくらでの実機の確かめ。
- **Out**: ゴーストのフォルダの `ghost/master/alias.txt`（MAKOTO の `makoto,` の付け替え。下の境目）・番号を複数書いたときに毎回どれかを選ぶこと（今の「先頭」の決めを変えない。変えるなら要件の段の議題）・`surfaces**.txt`（SSP 専用の分けた surfaces・議題）。

## Boundary Candidates

- シェルの設定の読み（`crates/areka-parsers/src/shell/decode.rs` の振り分け・`shell_target.rs` が読むファイル）
- 別名の表の持ち方（`crates/areka-emo-compose/src/world.rs`・`fold.rs`）
- 面の指定の解き方（`crates/areka-seriko/src/resolve.rs` と、解く道具を作る `crates/areka/src/emo2_boot/assets.rs`）

## Out of Boundary

- **`makoto-dll-host` との境目**: 同 brief（38 行目と 100 行目）が扱う `alias.txt` は、ゴーストのフォルダ `ghost/master/alias.txt` の `makoto,別名.dll`（MAKOTO の DLL の付け替え・ukadoc には無く materia の原典の決まり）。本 spec はシェルのフォルダ `shell/<名前>/alias.txt` の面の別名だけを読み、`ghost/master/alias.txt` は開かない。同じファイル名でも置き場所と中身が違う。
- 面の絵の読み方・合成の決まりは変えない。

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-shell-parse`（surfaces.txt の読み手）・完了 `areka-P0-dot-sakura-specimen`（検体）。
- **Downstream**: `shiori2-protocol`（どっとさくらの台詞が出るようになって初めて、別名の崩れが実機で見える）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-shell-parse` の別名の読みを広げる。
- **Adjacent**: `makoto-dll-host`（上の境目）・`seriko-pattern-calls`（`shell/decode.rs`・`world.rs`）・`shell-mouse-cursor`（`shell/decode.rs`・`shell_target.rs`）・`config-parse-diagnostics`（`shell/decode.rs`）・`extra-character-windows`（3 人目からのキャラクターの表）・`animated-surface-log-flood`・`dpi-realign-remembered-chain`・`placement-measure-bake-once`（`shell_target.rs`）。

## Constraints

- 1 ファイル 1,000 行（`shell_target.rs` 855＝足す分は新しいファイルへ）。
- 決定論のテストは必達。実機の根と一時フォルダはワークツリーの `target\` の下だけ。
- 区分を B にした理由: 正典に書いてある別名が黙って落ち、検体（どっとさくら）の別名がちょうど「無い面の番号を在る面へ付け替える」形なので、静的に読むだけで崩れが見込める。実機の症状は、どっとさくらの台詞が出ていない（`shiori2-protocol` の前）ので見ていない。

## 2026-10-23 棚卸㉙の測り

- **触るファイル**: `crates/areka-parsers/src/shell/{decode.rs 598, model.rs}` と兄弟のテスト・`crates/areka-emo-present/src/shell_target.rs`（855）か新しい兄弟のファイル（`alias.txt` を読む）・`crates/areka-emo-compose/src/{world.rs 412, fold.rs 375}`・`crates/areka-seriko/src/resolve.rs`（314）・`crates/areka/src/emo2_boot/assets.rs`（キャラクターごとの解く道具）・台帳 `assets.toml` の 2 行。
- **規模**: S〜M（6〜9 タスク。SSP の測り 1・読み 2・キャラクターごとの表 2・数字の別名 1・`name` 1・実機 1）。
- **先に要るもの**: 働きの上では無い。どっとさくらでの実機の確かめは `shiori2-protocol` の後（台詞が出ないと `\s[9]` が流れない）。同じウェーブに置けない相手（v0.1.0）: `seriko-pattern-calls`（`shell/decode.rs`・`world.rs`）・`shell-mouse-cursor`（`shell/decode.rs`・`shell_target.rs`）・`animated-surface-log-flood`・`dpi-realign-remembered-chain`・`placement-measure-bake-once`（`shell_target.rs`）。`consumer_ledger.rs` の列・emo-text・kanade の `schedule/events.rs`・`input_events/`・パーサの `balloon/` には触らない。SSP での測りは開発者の機械の SSP。
- **優先度の区分**: B（上の Constraints の理由）。
- **要件定義のモデル**: Opus（表をキャラクターごとに分けるのは、今の表の持ち方を広げるだけ）。
- **議題の候補**: ⑴ 数字の別名と番号のどちらを先に引くか（SSP で測る）。⑵ `alias.txt` と `surfaces.txt` の両方にあるときの勝ち負け（SSP で測る）。⑶ `surfaces**.txt`（同じ「読むファイル」の所・持ち主なし）も入れるか。⑷ 3 人目からの綴り（`char2.surface.alias` など）を読むか（SSP で測る・`extra-character-windows` との分け方）。⑸ 番号を複数書いたときの「先頭」の決めをそのままにするか。
