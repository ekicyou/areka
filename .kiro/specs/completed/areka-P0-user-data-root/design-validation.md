# 設計の検証: areka-P0-user-data-root

> 実施 2026-10-16・この枝（main `0bb51d42` の上・`c2a01782` の design.md）。対話なしで実施した（`/kiro-design` のサブエージェント）。
> 入力: `spec.json`（language=ja）・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）・`.claude/skills/kiro-validate-design/rules/design-review.md`。
> design.md が名前で指すコードは、Grep と Read で照らし合わせた（下の「コードとの照合」）。

## 設計レビューの要約

design.md は、要件 1〜10 のすべてを部品と口に結び付けている（Requirements Traceability の表に抜けは無い）。「書く経路の表」で書く物の行き先をすべて挙げ、ウェーブの約束（`main.rs`・`boot_resolve.rs`・`roots/edit.rs` に触らない）も守る形にした。そのまま実装へ進める出来である。ただし 1 か所で、確定した要件どうしの食い違い（要件 3.7・3.8 と 4.1）を、設計が要件に無い例外を足して埋めている。この件と、UI スレッドでの写し・純粋な関数に紛れ込むプロセスの状態の 2 件を、設計の討議で決める必要がある。

## コードとの照合（design.md の主張の確かめ）

| design.md の主張 | 照合の結果 |
|---|---|
| `roots/resolve.rs` の `resolve_roots_from(env, memory, exe_dir)` は純粋で、既定の腕は exe_dir 1 つ、記憶の腕は `decode_memory`、重複除去と `found = is_dir()` を経る | 合う。第 4 引数を足しても `ResolvedRoots` の欄は変わらない |
| `boot_config::exe_location()` は `OnceLock` で 1 回だけ解く | 合う。しかもリンクを辿る（`follow_exe_links`）。winget の `Links\areka.exe` から起動されても、控えの `.db` がある本当の入れ先を見られる |
| `default_app_profile_dir()` を替えれば、全部の読み手がそろう | 合う。呼び手は `resolve_boot`・`main.rs`（`BootContext`）・`ghost_session.rs`・`shiori_host.rs`・`ghost_boot_options`。どれも同じ関数を通る |
| ログ（`log_history::init`）は `resolve_boot` より前に初期化される | 合う（`main.rs`）。`establish` の記録は落ちない |
| `resolve_boot_from` の後に `resolve_balloon_for_ghost` と `ConfigInputs` が `ghost.dir` から決まる | 合う。写しのフォルダを入れれば、後ろはすべて写しになる |
| ゴーストを起こすのは、最初の起動と `ghost_switch::boot_into`（`switch_to`・`switch_to_default`）の 2 経路だけ | 合う（`boot_into(` の呼び手は 2 つ）。フックの漏れは無い |
| `CatalogDestinations::answer` は `install_root` の根を答える。起動中のゴーストは `running_root` | 合う。答えは `PathBuf` で、写しの中に置ける |
| `update/desk.rs` の `resolve` は、バルーンを `here.balloon_dir` と `list_balloons` から引く。ゴーストの `updateother` は無い | 合う |
| `roots/edit.rs` の `apply` は `ResolvedRoots` を組んで `log_roots` を呼ぶ | 合う。欄を足さない判断は正しい |
| `menu/roots_frame.rs` の `remove_item` は、根が 2 つ以上なら子を全部選べる形にする | 合う。書ける根の子だけを選べない形にする余地がある |
| `areka-mcp-bridge` の `Config::from_values` は、置き場の決め方を自前で持つ | 合う。`winget` のマニフェストでリンクを張るのは `areka.exe` だけなので、中継の `current_exe` の親は本当の入れ先になる |
| ログはファイルに書かない。`PersistScope::Balloon` の根は本番では `None` | 合う。実行ファイルの隣にログやバルーンの記憶はできない（要件 4.1・4.5 の前提） |
| winget の控え `Areka.Areka.Portable__DefaultSource.db` | 完了 spec の実測（`winget-local-check.md`）に在る |

## 重大な議題（3 件）

🔴 **重大な議題 1**: 「置き場を先頭へ ▸」で実行ファイルの隣の根を上へ動かすと、要件 4.1 の「0」が破れる。設計は、要件に無い例外を足してこれを埋めている。
**懸念**: 要件 3.8 では、書ける根を使う形でも、実行ファイルの隣の根を先頭へ選べる。要件 3.7 では、以後の新しく入れる物（と `.nar-work`）はその根へ入る。design は、このとき写すのをやめてその場で書く形にした（写す判断の流れ・決定 8）。そして「書く経路の表」の例外に「先頭へ動かした後」を足した。けれども要件 4.1 が数に入れない例外は、`AREKA_ROOT` だけである。
**影響**: winget の利用者向けでこれを選ぶと、後から入れた物・同梱のゴーストの記憶・SHIORI の書き残しが入れ先のフォルダに書かれ、次の上げ直しで消える。これは、この spec が直そうとしている症状そのものである。機械の全員向け（`Program Files`）では、インストールも記憶の書き込みも失敗する。説明書には「選んだ置き場に新しく入れる物が入る」とだけ載るので、利用者はこの代償を知らずに選べてしまう。
**提案**: どちらかに決める。（a）書ける根を使う形で、その理由が winget の入れ先か書けないのときは、「置き場を先頭へ ▸」の子のうち実行ファイルの隣の根を選べない形にする。要件 3.8 に 1 文を足し、`front_item` で塞ぐ。`decide_front` は変えない。（b）今の設計のままにする。その場合は、要件 4.1 の例外にこの場合を書き足し、説明書に「winget で入れた形で実行ファイルの隣を先頭にすると、上げ直しで消える」と書く。推しは（a）。直しはメニューだけで済み、要件 4.1 の「0」がそのまま保たれる。
**Traceability**: 要件 3.7・3.8・4.1・8.1・10.8
**Evidence**: design.md「写す判断（要件 4.3〜4.9）」の注記・「書く経路の表」の例外・Open Questions 2・research.md 決定 8

🔴 **重大な議題 2**: 切替とバルーンの更新で、ゴースト・バルーンのフォルダ全体を UI スレッドで同期に写す。
**懸念**: `boot_into` のフックは、窓を閉じた後に、UI スレッドで `copy_tree_atomic` を走らせる。`update/desk.rs` の `resolve` も UI スレッドで走る。写すのは同梱の物 1 つにつき一生に 1 回だが、大きさに上限は無い。書けない場所へ展開した zip では、管理者が置いた大きいゴーストも写す対象になる。開発者の方針は「ゴーストの大きさを甘く見ない・上限で断らず UI スレッドの外へ出す」（10-05）で、これと向きが逆である。
**影響**: 初回の切替では、ゴーストの窓が消えたまま、写し終わるまで UI が固まる（メニュー・他の窓・MCP の受け口も止まる）。実測で長いと分かってから背景へ出すと、`ghost_switch.rs`（917 行・上限の 1,000 行に近い）の段の作りを後から変えることになる。
**提案**: 討議で次のどちらかに決める。（a）今の設計のまま同期で写す。ただし、`user_data_copied` の `ms` に実測の閾値（例: 1 秒）を決めておき、超えたら背景へ出す追跡を起票する。（b）最初から写しを背景へ出す。予約の段で写し、写し終えてから `switch_to` を投函する。写すのは予約の時点にし、`boot_into` には「写した先」を渡すだけにすれば、`ghost_switch.rs` の段は増えない。最初の起動の写し（窓を開く前）とインストールの写し（背景のスレッド）は今のままでよい。
**Traceability**: 要件 4.3・4.4・4.7
**Evidence**: design.md「SwitchHook・InstallHook・UpdateHook」・「Performance & Scalability」・Open Questions 1・research.md 決定 5

🔴 **重大な議題 3**: 純粋な関数とテストの経路に、プロセスに 1 つの形（`layout()` → `establish`）の読み出しが紛れ込む。
**懸念**: `report::log_roots` が `listed_user_root()` を呼ぶ形にすると、`resolve_boot_from` のテストと `edit.rs` のテストが通るたびに、本物の `establish` が走る。`establish` は、プロセスの環境変数（`AREKA_USER_ROOT`）を読み、書ける根を作り、記憶を引き継ぐ。`update/desk.rs` の `resolve`（今は値だけの関数で、兄弟テストがある）と `CatalogDestinations::answer` も、中で `writable_copy`（＝`CopyContext::current()`）を呼ぶ形になる。開発者が実機の確かめのために `AREKA_USER_ROOT` を開けた端末（要件 2.3・10.8 の手順）で `cargo test` を回すと、テストが書ける根を作り、記録の欄（`user_root`）も変わる。
**影響**: 要件 10.6（値を注入できる形・テストは環境に左右されない）と、決定論テストの方針に反する。結果が端末の環境変数しだいで変わるので、揺れるテストの原因になりうる。
**提案**: プロセスの形は、薄い入口だけで読む。入口は `resolve_boot`・`resolve_targets`（`here()` で `CopyContext` を組んで渡す）・`install_into` の呼び手（`CatalogDestinations` に `CopyContext` を持たせる）・`boot_into` の 4 つ。`resolve`・`answer`・`log_roots_with` は値を受け取る。`log_roots` の今の口は `edit.rs` のためだけに残し、本番の経路でだけ `listed_user_root()` を読む。テストのプロセスで `establish` が走らないことを、1 本のテストで確かめる（例: `layout()` を通らない経路だけでテストが閉じる）。
**Traceability**: 要件 10.6・10.7・3.6・4.4
**Evidence**: design.md「RootsReportExt」・「SwitchHook・InstallHook・UpdateHook」・UserRootContext の State Management（「テストのプロセスでは layout() は今の形になる」という前提）

## 設計の良いところ

1. **書く経路を 1 つの口に集めた**: 写す判断を `plan_copy`（純粋）と `writable_copy`（1 つの口）に集め、フックを 4 か所に 1〜数行ずつ置いた。形の判断と記憶の置き場も、共有の葉のクレート `areka-user-data` にまとめた。こうして、areka と中継の食い違い（今の `from_values` が前例）を構造の上で無くした。コードとの照合でも、ゴーストを起こす経路は 2 つ、記憶の置き場の読み手は 1 つの関数に集まっていることを確かめた。漏れの無さを「書く経路の表」で示した点も良い。
2. **約束と既存の型を尊重した**: `main.rs`・`boot_resolve.rs` には触らない。`ResolvedRoots`・`BootContext` に欄を足さない（`OnceLock` の口から読む）。約束の外の `roots/edit.rs` は最後のタスクに置き、`write-failure-notice` の着地の後の `apply` に合わせる。前例の型（`exe_location` の `OnceLock`・`areka-nar` の「作業フォルダで組んで rename」・`catalog_roots` の隠れ）をそのまま使い、新しい外部依存も 0 にした。

## 小さな気付き（議題にしない・タスクで直せばよい）

- `CatalogDestinations::answer` が写しの根を答えるときは、並びの綴り（`roots` の中の書ける根の `dir()`）をそのまま返す。`areka-nar` は作業フォルダを根の綴りの完全一致でまとめるため（`destination.rs` の冒頭の注記）。
- `resolve_boot` の「`default_app_profile_dir()` が先に `layout()` を呼ぶ」という順の説明は、`OnceLock` なのでどちらが先でも同じになる。説明は省いてよい。
- `ghost_switch.rs` は 917 行。フックの 3〜5 行は入るが、議題 2 で（b）を選ぶなら、写しの段を別ファイルへ出す。
- 書けるかの確かめで `std::fs::OpenOptions` に `access_mode` を渡すと、read／write を指定しなくても開ける（`access_mode` が優先される）。実装のときに、`PermissionDenied` の対応を実機の `Program Files` で 1 度確かめる。

## 最終判断

**判断: GO**（重大な議題 3 件を設計の討議で決めることが条件）

**理由**: 骨格（共有の葉のクレート・1 つの口・`OnceLock`・並びへの差し込み）は、既存のアーキテクチャと steering に沿う。名前で指したコードの事実とも合う。3 件はどれも、局所の直し（メニュー 1 か所・写す時機・値の渡し方）で解ける。作り直しは要らない。

**次の手順**:
1. 設計の討議（`kiro-design-discussion`）で議題 1〜3 を決め、design.md に反映する。議題 1 で（a）を選ぶなら、要件 3.8 にも 1 文を足す。
2. 反映の後、`/kiro-spec-tasks areka-P0-user-data-root` でタスクを作る。「置き場を先頭へ ▸」（`roots/edit.rs`・`menu/*`）は最後のタスクに置き、`write-failure-notice` の着地を待つ。
