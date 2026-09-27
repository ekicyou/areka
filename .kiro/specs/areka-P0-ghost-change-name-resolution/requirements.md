# Requirements Document

> 本文の実測は **2026-09-27・本ブランチ**（main `55a2a1fd`＝棚卸⑱の直後。`ghost-shell-balloon-switch` PR#192 の着地後で、名前解決に関わるソースは brief の「2026-09-27 棚卸⑱の再測定」と同じ土台）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 要件 9 の裁定 1〜7 は、正典が沈黙している点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: `.nar` を入れた直後に「入れたゴーストに交代します」と言うゴーストの利用者。里々 wiki の定石（`satori:ゴースト切り替え`「インストールしたゴーストに即チェンジまたは呼び出し（ＳＳＰ専用）」）は `OnInstallComplete` から `\![change,ghost,lastinstalled]` を出す。完了 `ghost-shell-balloon-switch` だけでは `lastinstalled` は「該当なし」で黙って（`warn!` だけで）何も起きず、**入れて替える α の一周がゴーストの台本からは完結しない**。

**今の状態**: 同じ経路の仲間として、`\+`（ランダムに他のゴーストへ切替）・`\_+`（次のゴーストへ切替）・`\![change,ghost,random|sequential]` も今日は何も起きない。`\+`／`\_+` は字句解析で角括弧なしのタグに切れ、転記の既定の腕で「意味を持たない生の綴り」になり、台本の組み立ての受け皿が捨てる。`random`／`sequential`／`lastinstalled` は名指しの名前としてそのまま目録と突き合わされ、一致しないので「該当なし」に落ちる。「最後にインストールしたゴースト」の記録はどこにも無い。

**何を変えるか**: `\![change,ghost,random|sequential|lastinstalled]` と `\+`／`\_+` を、完了 `ghost-shell-balloon-switch` が作った名指しの切替の経路へ**名前を解いて渡す**。`random`＝今のゴースト以外から 1 体・`sequential`＝目録の並びの次（末尾なら先頭）・`lastinstalled`＝同じプロセスの中で最後に入れたゴースト。`lastinstalled` の受け皿（プロセス内の記録と書く口）を本仕様が用意し、`ghost-install` はそこへ書くだけ。切替の握手・降ろして起こし直す仕組み・kanade には触らない。

> 起票: 2026-09-26 `/kiro-discovery` 再入（棚卸⑰）で `ghost-shell-balloon-switch` から切り出し。2026-09-27（棚卸⑱）に main `5a232d2f` で再測定し、置き換える腕の所在（`crates/areka/src/emo2_boot/ghost_switch.rs` の `resolve_switch_target`／`request_ghost_switch`）と `\+`／`\_+` の写し方（`decode_bare` の 2 腕）を確かめた。

## Introduction

### 誰が困っているか

α の利用者（第三者）と、ゴーストの作者。作者の台本が正典どおり `\![change,ghost,lastinstalled]`・`\+`・`\_+` を書いても、今日の areka では**何も起きない**（利用者から見える変化は 0・ログに `warn!` か `debug!` が 1 件残るだけ）。

### いま何が起きているか（2026-09-27 実測）

- **名指しの切替は動く。** 完了 `ghost-shell-balloon-switch` により、台本の `\![change,ghost,名(,--option=raise-event)]` は `crates/areka/src/emo2_boot/change_cue.rs` の `ChangeCueSink` が名前を無変形で運び、`crates/areka/src/emo2_boot/ghost_switch.rs` の唯一の入口 `request_ghost_switch(world, SwitchRequest)` が目録（`crates/areka-ghost/src/catalog.rs` の `list_ghosts`）を読み、純関数 `resolve_switch_target(entries, &GhostSpec) -> Option<SwitchTarget>` で突き合わせる。`GhostSpec` は `Name(String)`（台本・`descript.txt` の `name` → フォルダ名の順）と `Folder(String)`（メニュー）の 2 通り。`resolve_switch_target` の説明は「比較は大文字小文字を区別し、今のゴーストも除外しない」と明記する。
- **特別な名前は「該当なし」に落ちる。** `random`／`sequential`／`lastinstalled` はそのまま `GhostSpec::Name("random")` などになり、目録に同名のゴーストが無い限り `warn!(event = "ghost_switch_unknown")`・`SwitchVerdict::NotFound`（降ろさず kanade へ何も送らない）。既存テスト `ghost_switch_tests.rs` の `unknown_names_warn_once_and_send_nothing` が `["Nobody", "random", "lastinstalled"]` の 3 つを「該当なし」として固定している。
- **`\+`／`\_+` は捨てられる。** 字句解析は `\_+` を 1 単位の角括弧なしタグにする（`lexer_bare_tag_tests.rs` の `two_char_underscore_tags_consume_as_single_unit`）。転記 `crates/areka-parsers/src/sakura/decode.rs` の `decode_bare` は `e`・`c`・`-`・`n`・`0`/`h`・`1`/`u`・`f` だけを写し、それ以外は既定の腕 `decode_passthrough_bare` で `Raw` にする。`Raw` は `crates/areka-sakura/src/compile.rs` の受け皿の既定の腕が `debug!` を残して捨てる。同じ `decode_bare` に、裸の `\f` を角括弧付きと同じ受け皿（`Font { args: [] }`）へ載せる前例がある。
- **今のゴーストは入口で分かる。** `request_ghost_switch` は `BootContext` の `current.ghost.folder: Option<String>` を読んでいる（argv でベースウェアの根の外のゴーストを起こしたときは `None`）。
- **目録の並びはフォルダ名の昇順。** `list_ghosts` の説明は「フォルダ名の昇順」で、右クリックメニューの「ゴースト」枠（`crates/areka/src/menu/ghost_frame.rs` の `ghost_frame_item`）もその並びのまま子を出す。利用者が見る並びと同じ。
- **「最後にインストールしたゴースト」の記録は無い。** 製品コードに `lastinstalled` は 0 件（上の既存テストの 1 件だけ）。書く側（`ghost-install`）はまだ無い。
- **乱数の前例。** `crates/areka/src/boot_resolve.rs` の `pick_index(n)` が起動時のゴースト／バルーンの「ランダム」（`GhostRoute::Random`・`ghost_picked_randomly`）を新規依存なしで引いている。
- **二重要求。** `request_ghost_switch` は「予約が在れば `warn!(ghost_switch_busy)`」を最初に判定する（本仕様で変えない）。
- **台帳。** `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\+`（`_5c_2b`）・`\_+`（`_5c__2b`）は `absent`・owner 空・B2。`\![change,ghost,…]` の行は `implemented`・owner `areka-P0-ghost-shell-balloon-switch` で、備考が「正典の特別な名前 random／sequential／lastinstalled の解決は areka-P0-ghost-change-name-resolution の持ち場」と本仕様を名指しし、`same-feature` で `\+`・`\_+` を指す。`doc/COMPAT_ARCHITECTURE.md` §8 に `random`／`sequential`／`lastinstalled` の行は無い。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [`\![change,ghost,ゴースト名(,--option=raise-event)]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1) | 「ゴースト名をrandomにするとランダムチェンジ(\+と同様)。sequentialにするとシーケンシャルチェンジ(\_+と同様)。実行後、該当ゴーストがいなかった場合は無視される。」「ゴースト名をlastinstalledにすると最後にインストールしたゴーストに切り替え。ただしSSPを一度終了した場合は無効。」「ゴースト名の後に--option=raise-eventとすると、メニューから切り替え操作をした場合と同じくOnGhostChangingが通知される。（略）指定しない場合は通知されない。」 | 3 つの特別な名前は同じ命令の引数。`random`＝`\+`、`sequential`＝`\_+` と**同様**（別名）。該当なしは無視。`lastinstalled` はプロセスの中だけの記録でよい（永続化しない）。`--option=raise-event` は特別な名前でも同じに効く。 |
| [`\+`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_2b:1) | 「ランダムに他のゴーストに切り替わる。このスクリプトによる切り替えではSHIORIイベントOnGhostChangingは通知されない。」 | 「**他の**ゴースト」＝今のゴーストは候補から外す。`OnGhostChanging` は送らない（＝`raise-event` 無しの名指しと同じ）。 |
| [`\_+`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c__2b:1) | 「次の順番のゴーストに切り替わる(シーケンシャルチェンジ)。ゴーストエクスプローラで下側にいるゴーストに切り替わる。リストの一番下だった場合、リストの一番上のゴーストに切り替わる。このスクリプトによる切り替えではSHIORIイベントOnGhostChangingは通知されない。」 | 「リスト」の次、末尾なら先頭。areka にゴーストエクスプローラは無いので「リスト」＝目録の並び（メニューの「ゴースト」枠と同じ・要件 9 裁定 1）。`OnGhostChanging` は送らない。 |
| [`OnInstallComplete`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallComplete:1)（参考） | 「インストールが正常終了した際に発生。」 | 里々 wiki の定石はこのイベントの応答で `\![change,ghost,lastinstalled]` を出す。送るのは `ghost-install`（本仕様の外）。本仕様は、その台本が届いたときに解けるようにする。 |
| [里々 wiki「ゴースト切り替え」](https://soliton.sub.jp/satori/?%E3%82%B4%E3%83%BC%E3%82%B9%E3%83%88%E5%88%87%E3%82%8A%E6%9B%BF%E3%81%88#ma90e679)（参考） | 「速攻ゴースト切替 …\![change,ghost,lastinstalled]」 | α で「入れて替える一周」を台本から完結させる根拠。 |

正典が沈黙している点（並びの定義・今のゴーストが目録に無いときの「次」・目録が 1 体だけのときの `sequential`・特別な名前と同名のゴーストがいるときの優先・`lastinstalled` の記録を切替に使ったあと残すか・記録したゴーストが目録から消えていたとき）は areka の裁量として要件 9 で決め、`doc/COMPAT_ARCHITECTURE.md` §8 に記す。SSP の挙動を実測して合わせることはしない。

### 何を変えるか

**特別な名前を名指しへ解いて、名指しの経路へ渡す。** 台本の `\![change,ghost,random|sequential|lastinstalled]` と `\+`／`\_+` が、完了 `ghost-shell-balloon-switch` の唯一の入口で目録と突き合わされる前に、具体的なゴースト（フォルダ）へ解かれる。解けたあとは名指しの切替と 1 本の同じ経路（`OnGhostChanging` の有無・降ろす・起こす・失敗したら既定ゴーストへ戻す）を通り、kanade には触らない。`lastinstalled` の記録はプロセスの中だけに置き、書く口を 1 つ用意して `ghost-install` に渡す。判断の分岐は偽の目録と注入した乱数で決定論テストに固定し、実機で `\+` による emo2 → 他のゴーストの切替を 1 周見る。

## Boundary Context

- **In scope**:
  - `\+`／`\_+` の綴りを `\![change,ghost,random]`／`\![change,ghost,sequential]` と同じ切替要求にすること（転記の段で別名として写す・意味づけはしない・`OnGhostChanging` は送らない）。
  - 台本の `\![change,ghost,名]` の名前が `random`／`sequential`／`lastinstalled` のときの解決（今のゴースト以外から 1 体・目録の並びの次・最後に入れたゴースト）と、解けないときの無視＋`warn!`。
  - `lastinstalled` の受け皿＝プロセスの中だけの記録（永続化しない）と、それを書く口 1 つ（`ghost-install` が呼ぶ）。
  - 解けた名前を名指しの切替の経路（`SwitchRequest` の唯一の入口）へ渡すこと。`--option=raise-event` と出どころ（台本＝`automatic`）はそのまま通す。
  - 網羅台帳 `sakura-script.toml` の `\+`・`\_+`（実装済み・owner＝本仕様）と `\![change,ghost,…]` の備考（「本仕様の持ち場」→「解決済み」）、`doc/COMPAT_ARCHITECTURE.md` §8 の裁量の行、`.kiro/steering/roadmap.md` の行。
  - 決定論テスト（偽の目録・注入した乱数・手で入れた記録）と、既存の「該当なし」テストの縮め（`random`／`lastinstalled` を該当なしの例から外す）。実機サインオフ（`\+` で emo2 → 別のゴースト）。
- **Out of scope**:
  - シェル・バルーンの `random`／`lastinstalled`（`\![change,shell|balloon,…]`＝`areka-P0-shell-balloon-switch` の議題 ⑷。正典 `\![change,shell]` の `lastinstalled` の読み方も同 spec が裁定する）。
  - `\![call,ghost,…]`（`lastinstalled`／`random` を含む・多重ゴースト・α 後）。
  - `lastinstalled` を**書く側**（`ghost-install`＝インストール完了時に受け皿へ書く・`OnInstallComplete` を送る）。本仕様は受け皿と書く口を用意するだけで、本番で書く呼び手は持たない（本仕様の完了時点で記録は常に「無い」）。
  - `sequential` の順を SSP の「ゴーストエクスプローラの下側」に合わせること（areka にゴーストエクスプローラは無い）。
  - 切替の握手・降ろして起こし直す仕組み・失敗時に既定ゴーストへ戻すこと・二重要求の扱い・メニューの「ゴースト」枠（完了 `ghost-shell-balloon-switch`。本仕様はそのまま使う）。
  - `random` の乱数の質（暗号学的な強さは要らない・偏りの検定はしない）。
- **Adjacent expectations**:
  - 完了 `ghost-shell-balloon-switch` の「解決できない名前」の腕（唯一の入口の中の突き合わせ）を本仕様が広げる。入口の形（`SwitchRequest`・`GhostSpec`・`SwitchVerdict`）と kanade へ送る要求の形は変えない。`GhostSpec` に腕を足す形を採るなら `ghost-install` の brief へ申し送る（同 spec が `GhostSpec` の形を前提にしている）。
  - `areka-P0-ghost-install`（後続）は、本仕様の書く口へ「入れたゴーストのフォルダ名」を 1 行で書くだけで `\![change,ghost,lastinstalled]` が動くことを期待する。書く口の名前と引数は設計で決め、完了時に `ghost-install` の brief へ申し送る。
  - `areka-P0-shell-balloon-switch`・`areka-P0-session-mark-residue`（同じウェーブで並走）とは触るソースを重ねない（brief の Constraints）。本仕様が触るのは名前解決の入口のファイルとその兄弟テスト、転記の `decode_bare` とその兄弟テストに閉じる。台帳 `sakura-script.toml` は別の行、生成物 `report/*.md` は手で直さず、後に main へ入る側が生成器で作り直す。
  - 完了 `areka-P0-baseware-root-layout` の目録（`list_ghosts`＝フォルダ名の昇順・`GhostEntry`）を読むだけで、目録の形は変えない。同 spec の裁定「列挙の並びは判断に使わない」は、`sequential` に限り本仕様の裁定 1 が上書きする（並びを「次」の判断に使う。§8 に記す）。

## Requirements

### Requirement 1: `\+` と `\_+` は `\![change,ghost,random]`／`\![change,ghost,sequential]` の別名として届く

**Objective:** As a ゴーストの作者, I want 正典の短い綴り `\+`・`\_+` がそのまま動くこと, so that SSP 向けに書いた台本を書き換えずに済む

#### Acceptance Criteria

1. When 再生中の台本が `\+` の位置に達する, the areka shall `\![change,ghost,random]` と同じ切替要求（`random` の解決・`OnGhostChanging` を送らない・出どころ＝台本）を出す。
2. When 再生中の台本が `\_+` の位置に達する, the areka shall `\![change,ghost,sequential]` と同じ切替要求（`sequential` の解決・`OnGhostChanging` を送らない・出どころ＝台本）を出す。
3. The areka shall `\+`／`\_+` を、字句と転記の段で角括弧付きの `\![change,ghost,random|sequential]` と同じ受け皿へ載せる**別名の転記**として扱い、新しい種類の命令を作らない（`\!` の命令は汎用の運び手 1 本の原則）。意味（誰が消費するか）は転記の段では決めない。
4. The areka shall `\+`／`\_+` を「意味を持たない生の綴り」のまま台本の組み立てへ流す経路を残さない（今日の `Raw` 落ちを閉じる）。`\+` の直後の本文（例 `\+こんにちは`）は本文として残る（`\+` が消費する文字は 1 文字だけ・完了 `sakura-bare-tag-lexer` の切れ目は変えない）。
5. The areka shall `\+`／`\_+` に引数の形（`\+[…]`）を新設しない。

### Requirement 2: `random` は今のゴースト以外から 1 体を選ぶ

**Objective:** As a ゴーストの作者, I want `\![change,ghost,random]`／`\+` で他のゴーストへ替わること, so that 正典どおり「ランダムに他のゴーストに切り替わる」

#### Acceptance Criteria

1. When 名前 `random` の切替要求が届く, the areka shall 目録のゴーストのうち**今のゴーストを除いた**ものを候補とし、その中から 1 体を選んで名指しの切替の経路へ渡す。
2. When 候補が 2 体以上ある, the areka shall どの候補も選ばれ得るようにする（特定の 1 体に固定しない）。
3. If 候補が 0 体（目録が今のゴーストだけ、または空）, then the areka shall 切替を無視し、`warn!` を 1 件残し、ゴーストを降ろさず `OnGhostChanging` も送らない（利用者から見える変化は 0）。
4. While 今のゴーストが目録に無い（ベースウェアの根の外のゴーストを引数で起こした）, when 名前 `random` の切替要求が届く, the areka shall 目録の全ゴーストを候補とする（除くものが無い）。
5. When `\![change,ghost,random,--option=raise-event]` が届く, the areka shall 選んだ 1 体を切替先として、名指しと同じく `OnGhostChanging`（Ref0〜3 は選んだ切替先の値）を送る切替を行う。`\+` と `--option` 無しの `random` は送らない。
6. The areka shall 候補からの選び方を、乱数を外から与えて決定論テストで固定できる形にする（本番の乱数の出所は設計で決める・新しい依存は足さない）。

### Requirement 3: `sequential` は目録の並びで「次」のゴーストへ替える

**Objective:** As a ゴーストの作者, I want `\![change,ghost,sequential]`／`\_+` で順番に隣のゴーストへ替わること, so that 正典どおり「次の順番のゴースト」へ切り替わる

#### Acceptance Criteria

1. When 名前 `sequential` の切替要求が届く, the areka shall 目録の並び（フォルダ名の昇順＝右クリックメニューの「ゴースト」枠と同じ並び）で今のゴーストの**次**のゴーストを切替先として名指しの切替の経路へ渡す。
2. When 今のゴーストが目録の末尾である, the areka shall 目録の先頭のゴーストを切替先とする（正典「リストの一番下だった場合、リストの一番上のゴーストに切り替わる」）。
3. While 今のゴーストが目録に無い（ベースウェアの根の外のゴーストを引数で起こした）, when 名前 `sequential` の切替要求が届く, the areka shall 目録の先頭のゴーストを切替先とする（要件 9 裁定 2）。
4. While 目録が今のゴースト 1 体だけである, when 名前 `sequential` の切替要求が届く, the areka shall 今のゴースト自身を切替先とし、完了 `ghost-shell-balloon-switch` 要件 1.8 のとおり降ろして起こし直す（要件 9 裁定 3）。
5. If 目録が空である, then the areka shall 切替を無視し、`warn!` を 1 件残す。
6. When `\![change,ghost,sequential,--option=raise-event]` が届く, the areka shall 名指しと同じく `OnGhostChanging` を送る切替を行う。`\_+` と `--option` 無しの `sequential` は送らない。
7. The areka shall `sequential` の「次」の判断に目録の並び以外（前回の切替の履歴・起動順）を使わない。

### Requirement 4: `lastinstalled` は同じプロセスの中で最後に入れたゴーストへ替える

**Objective:** As a α の利用者, I want `.nar` を入れた直後の「入れたゴーストに交代します」がそのまま動くこと, so that 入れて替える一周がゴーストの台本から完結する

#### Acceptance Criteria

1. The areka shall 「最後にインストールしたゴースト」の記録を**プロセスの中だけ**に 1 つ持ち、ファイルにも記憶（プロパティ）にも書かない（正典「SSPを一度終了した場合は無効」）。
2. The areka shall その記録を書く口を 1 つ用意し、入れたゴーストのフォルダ名を受け取る（呼び手は `ghost-install`・本仕様では本番の呼び手を持たない）。When 書く口が 2 度以上呼ばれる, the areka shall 最後の 1 件だけを保つ。
3. When 名前 `lastinstalled` の切替要求が届き、記録があり、記録のゴーストが目録にある, the areka shall そのゴーストを切替先として名指しの切替の経路へ渡す。
4. If 名前 `lastinstalled` の切替要求が届いたが記録が無い（このプロセスで何も入れていない）, then the areka shall 切替を無視し、`warn!` を 1 件残し、ゴーストを降ろさず `OnGhostChanging` も送らない。
5. If 記録はあるが、そのゴーストが目録に無い（入れたあとにフォルダが消された）, then the areka shall 切替を無視し、`warn!` を 1 件残す（要件 9 裁定 4）。
6. The areka shall 記録を、切替に使ったあとも・別のゴーストへ切り替わったあとも消さない（プロセスが終わるまで有効・要件 9 裁定 5）。
7. When 記録のゴーストが今のゴースト自身である, the areka shall 無視せず、完了 `ghost-shell-balloon-switch` 要件 1.8 のとおり降ろして起こし直す。
8. When `\![change,ghost,lastinstalled,--option=raise-event]` が届く, the areka shall 名指しと同じく `OnGhostChanging` を送る切替を行う。

### Requirement 5: 解けた名前は名指しと同じ 1 本の経路を通る

**Objective:** As a 開発者, I want 名前解決が切替の経路の手前の 1 段で完結すること, so that 切替の握手・降ろして起こし直す仕組み・kanade に手を入れずに済み、並走する spec と触るソースが重ならない

#### Acceptance Criteria

1. The areka shall `random`／`sequential`／`lastinstalled` の解決を、切替要求の唯一の入口（完了 `ghost-shell-balloon-switch` 要件 1.1）の中の「目録と突き合わせる」段で行い、解けた結果を名指し（フォルダ名）として同じ経路へ渡す。経路を 2 本にしない。
2. The areka shall 解決の判断を、目録の項目・今のゴーストのフォルダ名・`lastinstalled` の記録・乱数だけを入力とする純粋な関数で行う（ファイルの読み書きと切り離す＝決定論テストの前提）。
3. The areka shall 解けたあとの切替（`OnGhostChanging` の有無と Ref0〜3・降ろす・起こす・切替先が起きなければ既定ゴーストへ戻す・二重要求の無視）を完了 `ghost-shell-balloon-switch` の形のまま使い、kanade・切替の握手・降ろして起こし直す仕組みに変更を加えない。
4. The areka shall 特別な名前の比較を大文字小文字を区別して行い、正典の綴り（`random`・`sequential`・`lastinstalled`）だけを特別な名前とする（`Random`・`RANDOM` は名指しの名前）。
5. While 目録に `random`・`sequential`・`lastinstalled` と同じ名前（`descript.txt` の `name` またはフォルダ名）のゴーストがいる, when その名前の切替要求が届く, the areka shall 特別な名前として解決する（そのゴーストを名指ししない・要件 9 裁定 6）。
6. The areka shall 解決の段を、切替の途中（予約が在る）の判定より**後**に置く（今日どおり、切替中の 2 通目は解決せずに `warn!` で無視する）。
7. The areka shall 右クリックメニューの「ゴースト」枠からの切替（フォルダ名の名指し）に特別な名前の解決をかけない（メニューの指し方はフォルダ名だけ）。

### Requirement 6: 記録（ログ）と判断の固定

**Objective:** As a 開発者, I want 無視した理由がログで見分けられ、判断の分岐がすべて決定論テストで固定されていること, so that 「何も起きない」が仕様どおりか欠陥かを実機を待たずに判別できる

#### Acceptance Criteria

1. If 特別な名前が解けなかった（`random` の候補 0・`sequential` の目録空・`lastinstalled` の記録なし・記録のゴーストが目録にない）, then the areka shall それぞれを見分けられる `warn!` を 1 件だけ残す（同じ事象で 2 件出さない・黙って落ちる経路を残さない）。
2. When 特別な名前が解けた, the areka shall 何を何へ解いたか（名前・切替先のフォルダ名・`sequential` なら今の位置）を `info!` または `debug!` に 1 件残す。
3. The areka shall 要件 2〜5 の分岐（候補 0・候補 1・候補 2 以上と乱数の値・末尾→先頭・今のゴーストが目録にない・1 体だけ・記録なし・記録あり・記録のゴーストが消えた・`raise-event` の通り方・同名のゴーストとの優先・切替中の 2 通目）を、偽の目録と注入した乱数と手で入れた記録による決定論テストで固定する。
4. The areka shall 要件 1 の別名の転記（`\+`／`\_+` → 角括弧付きと同じ受け皿・直後の本文が残る）を転記の兄弟テストで固定する。
5. The areka shall 既存テスト `unknown_names_warn_once_and_send_nothing` の「該当なし」の例から `random`・`lastinstalled` を外し（`Nobody` だけ残す）、それらの判断を本仕様の新しいテストへ移す。
6. The areka shall `lastinstalled` の切替を、本番の書く側が無いため、記録を手で入れた World の決定論テストで確かめる（実機では確かめない＝`ghost-install` の実機一周へ申し送る）。

### Requirement 7: 台帳と文書

**Objective:** As a 互換の網羅を追う開発者, I want 台帳と裁量の記録が実装と一致すること, so that `\+`・`\_+`・特別な名前の状態を台帳だけで読める

#### Acceptance Criteria

1. The areka shall 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\+`・`\_+` の行を実装済み（owner＝本仕様）に改め、備考の「壊れ方」を実装後の振る舞い（何へ解かれるか・解けないときの `warn!`）へ書き換える。
2. The areka shall 同台帳の `\![change,ghost,…]` の行の備考「random／sequential／lastinstalled の解決は areka-P0-ghost-change-name-resolution の持ち場」を「解決済み（本仕様）」へ改める。`shell-balloon-switch` が触る `\![change,shell|balloon,…]` の行には触らない。
3. The areka shall 生成物（`doc/ukadoc-coverage/report/*.md`）を手で直さず、生成器で作り直す（並走する spec と同時に変わる数は、後に main へ入る側が作り直す）。
4. The areka shall `doc/COMPAT_ARCHITECTURE.md` §8 に、要件 9 の裁定（`sequential` の並び・今のゴーストが目録にないとき・1 体だけのとき・同名のゴーストとの優先・`lastinstalled` の記録の寿命と消えたゴースト）を 1 行ずつ記す。
5. The areka shall `.kiro/steering/roadmap.md` の本仕様の行を完了へ改める。

### Requirement 8: 実機サインオフ

**Objective:** As a α の利用者, I want `\+` で実際に別のゴーストへ替わるのを見ること, so that 決定論テストが隠す配線の欠陥を実機で炙り出す

#### Acceptance Criteria

1. When 検体（emo2 と、目録にある他のゴースト 1 体以上）で `\+` を含む台本が再生される, the areka shall emo2 を降ろして他のゴーストを起こし、ログに解決の記録（要件 6.2）と切替の要求の記録（完了 `ghost-shell-balloon-switch` の `ghost_switch_requested`）を残す。
2. When 実機サインオフを行う, the areka shall 判定の分岐のログの level（`debug!`／`trace!`）まで開けて記録を取り、結果を spec の下（`signoff.md`）に残す。
3. The areka shall `lastinstalled` の実機確認を `ghost-install` の実機一周へ申し送る（要件 6.6）。

### Requirement 9: 暫定裁定（要件ディスカッションで覆せる）

**Objective:** As a 開発者, I want 正典が沈黙している点を 1 か所にまとめておくこと, so that 討議で覆したときに直す要件が分かる

#### Acceptance Criteria

1. **`sequential` の並び＝目録の並び（フォルダ名の昇順）。** The areka shall SSP の「ゴーストエクスプローラの下側」の代わりに、利用者が右クリックメニューで見る並びをそのまま使う（要件 3.1）。完了 `baseware-root-layout` の「列挙の並びは判断に使わない」は `sequential` に限り上書きする。
2. **今のゴーストが目録に無いときの「次」＝先頭。** The areka shall 引数でベースウェアの根の外のゴーストを起こしたときの `sequential` を目録の先頭へ解く（要件 3.3）。
3. **目録が 1 体だけのときの `sequential`＝自分自身（起こし直し）。** The areka shall 正典「一番下なら一番上」を文字どおりに当て、無視ではなく自分自身への切替（完了 `ghost-shell-balloon-switch` 要件 1.8 の読み直しの代用）とする（要件 3.4）。`random` は「他の」と書かれているので同じ状況では無視（要件 2.3）。
4. **記録のゴーストが目録から消えていたら無視。** The areka shall 消えたゴーストを起こそうとしてから失敗する形（既定ゴーストへ戻る）にせず、解決の段で「該当なし」として `warn!` で止める（要件 4.5・正典「該当ゴーストがいなかった場合は無視される」の読み）。
5. **`lastinstalled` の記録は使っても消えない。** The areka shall 記録をプロセスの終わりまで保ち、同じ記録で何度でも切り替えられるようにする（要件 4.6・正典は「SSP を一度終了した場合は無効」としか言わない）。
6. **特別な名前は目録の同名より優先。** The areka shall `random`・`sequential`・`lastinstalled` と同じ名前のゴーストを台本から名指しできない形を受け入れる（要件 5.5・正典が引数の語を予約している以上、避けられない）。
7. **`random` の 1 体の選び方は候補の一様な選択。** The areka shall 候補に重みを付けず、乱数を外から与えられる形にする（要件 2.2・2.6）。
