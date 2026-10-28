# Brief: ghost-material-metadata

## Problem

- **演劇の作者**（pasta で書くノベル）: 演劇は、任意のゴーストの任意のシェルを役者に借りる（2026-10-28 議題 4 の開発者の追記）。けれど areka には、ゴースト・シェル・バルーンを「どれか」と指す安定した宛名が無い。借りてよいか、シェルが背景か役者かトークの場所か、作者は誰でどこで配っているかも、読める所が無い。
- **素材の作者**: 自分のシェルを、ゴーストに入れずに単体で公開したい人がいるかもしれない（開発者「シェルだけ公開したい人がいるかもしれないので」）。
- **利用者**: 演劇が借りたシェルの作者や配布の場所を知る手立てが無い。
- **開発者**（2026-10-28 `/kiro-discovery`・M1「素材 ID とメタデータ契約」）。原文と決めたこと（議題 5・6・7・11）は `doc/ideas/novel-platform.md`。

## Current State

起票のときの実測（main `d6054e4d`）:

- **正典の descript.txt**（ukadoc）に、ゴースト・シェル・バルーンのどれにも次のキーが既にある。
  - `id,ID名`: 半角英数（SSP は全文字を許す）。「今のところ起動時のコンソールのような表示に出てくる以外の用途はない」。
  - `craftman`・`craftmanw`: 作者の名前。`craftmanurl,URL`: 作者の URL。`homeurl`: ネットワーク更新の更新先。
  - 手本: プラグインの `id` は GUID で「一度決定したら変更してはならない」。
- **areka の目録**: `crates/areka-ghost/src/catalog.rs` の `list_ghosts`・`list_balloons`・`list_shells`。素性の型 `Identity`（112〜138 行）は 7 項目＝`folder`・`name`・`craftman`・`craftmanw`・`id`・`readme`・`has_thumbnail`。コメント「7 項目以外は足さない（要件 2.9）」＝完了 `baseware-root-layout` の要件 2.9（目録の素性に `sakura.name` を足さない）。**`craftmanurl` は持たない**。`homeurl` は別の関数（同 328 行付近）がネットワーク更新のために読む。
- **install.txt**: `crates/areka-nar/src/manifest.rs` の `InstallManifest` は `kind`・`name`・`directory`・`accept`・`existing`・`companions`・`warnings`・`charset_declared` だけ。ID・作者・ライセンスの欄は無い。利用条件の本文（`terms.txt`／`terms.md`）は `crates/areka/src/install/terms.rs` が画面に出すだけ。
- **バルーンを探す順の前例**: 完了 `ghost-inner-balloon` は「ゴーストの中の `balloon\<名>\` → 置き場の `balloon\<名>\`」の順に探す。`dev-folder-alias` は「エイリアス → 根の並びの上から」。
- **シェルの単体の置き場は無い**: SSP では、シェルは必ずどこかのゴーストの中に入る（`type,shell` は入れ先のゴーストを持つ）。
- **差し替え・借用・席・役割の考えは 0**（brief を cameo・swappable・slot・role で引いて 0 件。コードの `GhostSlot` は「動いているゴーストを入れる箱」で別物）。

## Desired Outcome

- ゴースト・シェル・バルーン・演劇を、素材 ID（`種別:id`）で指せる。同梱の素材は親の ID を前に付ける（`shell:foo/default`）。単体のフリー素材は前に何も付けない。
- 演劇の中で短く書いた ID が、決まった探す順で 1 つに決まる。当たらなければ既定へ戻し、「未導入」と記録して、取得先があれば示す。
- 作者・配布の場所・支援先・ライセンス・シェルの役割（character・background・stage）・「借りないで」の印・「演劇である」の印・演劇の席の要件を、descript.txt から読める。
- 目録の素性がこれらを持ち、メニューや About から作者の配布の場所・支援先へ案内できる。
- SSP との互換は壊れない（SSP は知らないキーを読み飛ばす）。
- 既存の素材（ID も新しいキーも無い）は今のまま動き、ID はフォルダ名で補われる。

## Approach

- **キー**（議題 6）: 正典のキー（`id`・`craftman`・`craftmanw`・`craftmanurl`）をそのまま使う。足りない分だけを areka の独自キーとして descript.txt に足す。頭に areka の印を付けて正典の名前と衝突させない（名前は要件の段で決める・たとえば `areka.` で始める）。席の要件は、正典の `surface*` ブレスと同じ書き方で演劇の descript に並べ、名付けも同じ型にする。別のファイルは作らない。「提供する能力」は使う側が現れるまで外す。開発者「まああとの議論で変わるかもしれないけど」。
- **ID の規則**（議題 7）: この spec が先に決め、外のリポジトリの U1（利用統計の仕様と登記簿・`doc/ideas/usage-stats.md`）は後でそれを写して、世界で 1 つであることの保証と偽装への手当て（登記簿）を足す。id の元は descript の `id`、空ならインストール先のフォルダ名（install.txt の `directory`）。手元で同じ ID が 2 つあれば、決まった順で最初の 1 つに決め、かぶりを記録する。作者向けの文書に「一度決めたら変えない」と書く。
- **探す順**（議題 7）: 要件の段で決める。開発者の叩き台（演劇 {play} から {shell} を探すとき）:
  1. `/play/{play}/shell/{shell}/`
  2. `/free-shell/{shell}/`
  3. `/ghost/{shell}/shell/master/`
  親まで書いた ID（`shell:{ghost}/{shell}`）は探さずにそれだけを指す（ゴーストの既定でないシェルを借りる道）。
- **借りてよいか**（議題 5）: オプトアウト。どのシェルも借りられ、作者が「借りないで」の印を書いたものだけ借りない。借りたシェルの作者表記（`craftman`・`craftmanurl`）を演劇の中で見られるようにする。利用者による差し替えも同じ規則。演劇の作者が「この席は替えさせない」と錠をかける側は要件の段で詰める。利用者が普段のゴーストでバルーンやシェルを替えるのは今のまま制限しない。

## Scope

- **In**: 素材 ID の形と導き方・探す順・かぶりと未導入の扱い・descript の独自キー（ライセンス・支援先・role・「借りないで」・「演劇である」・席の要件）・目録の素性の拡張（`craftmanurl` と足したキー・完了 `baseware-root-layout` の要件 2.9 の改め）・作者の配布の場所・支援先への案内・単体のフリーシェルの置き場（`/free-shell/`）とそのインストール・作者向けの書き方の文書・決定論のテスト。
- **Out**: 席に役者を立てて描くこと（`shell-multi-compose`）・演劇のパッケージの形と起動（pasta の依頼書の後・roadmap の受け口の節）・世界で 1 つであることの保証と登記簿（外の U1）・利用統計へ渡すこと（`areka-usage-stats`）・`pasta_check validate` の検査（pasta の持ち物・下の申し送り）。

## Boundary Candidates

- descript の独自キーの読み手（`areka-parsers` の descript・`areka-ghost` の目録）。
- 素材 ID の導き方と探す順（目録の上の解決の関数）。
- 単体のフリーシェルのインストール（`areka-nar`・`crates/areka/src/install/`）。
- 案内の入口（メニュー・About）。

## Out of Boundary

- シェルの中身の解析（surfaces.txt・SERIKO）。
- ネットワーク更新の決まり（`homeurl` の扱いは今のまま）。

## Upstream / Downstream

- **Upstream**: 働きの上の前提は無い。`audit-ghost-catalog`（v0.1.0 の後の隙間・目録の見直し）が目録を先に触るなら、その着地の後に。
- **Downstream**: `shell-multi-compose`（役者のシェルの宛名・role）・`areka-usage-stats`（素材 ID で集める）・演劇のパッケージ（受け口の節）・外の U1（ID の規則を写す）・pasta の P2（検査）。

## Existing Spec Touchpoints

- **Extends**: 完了 `baseware-root-layout` の要件 2.9（目録の素性を 7 項目に限る決まり）を改める。
- **Adjacent**: `install-package-type`（いつか・`type,package`＝単体のフリーシェルの置き場と近い）・`property-catalog-lists`（ghostlist などの一覧を目録から出す）・完了 `ghost-inner-balloon`（探す順の前例）・`dev-folder-alias`（探す順の先頭のエイリアス）。

## Constraints

- 区分 A（開発者が起票）・版は新しい段「ノベルゲーム基盤」（2026-10-28 議題 1・3）。
- 規模の見立て: M〜L（12〜20 タスク）。Opus。
- **pasta への相談（議題 11）**: この spec がウェーブに載ったら、要件の段の最初に、areka のセッションから pasta（SHIORI のエンジン・`C:\home\maz\git\pasta`）のセッションへ相談し、P2（`pasta_check validate` で新しいキーをどこまで検査するか）を決める。開発者「spec が次のウェーブに載ったタイミングで、areka セッションから pasta セッションを呼び出す形で相談して決めることにしましょう」。
  - 別のリポジトリにセッションを立てられるのは、開発者がその場所を言葉で指したときだけ。pasta のセッションが動いていれば直接手紙で相談する。動いていなければ、`/kiro-start` のときに開発者が「pasta のセッションと相談して」と添えるか、先に pasta のセッションを開いてもらう。
  - 相談の中身: 足すキーの一覧と書式・`pasta_check validate` に足す範囲・`pasta_check release` は手を入れずにキーが .nar に載ること。
- SSP との互換を壊さない（正典のキーの意味を変えない）。
- 失敗の経路は記録を残す（ID のかぶり・未導入・読めないキーは `warn!` を 1 行）。
