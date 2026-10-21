# Brief: areka-P0-sylphya-sqlite-kvs

> 出どころ（2026-10-21 開発者・`/kiro-discovery`）: 「areka-sylphyaで行っている永続化について、sqliteベースへの移行を検討せよ。画面などで高速に参照されうる揮発変数（rustコードで直接管理）、get/set propertyの値として呼ばれるが、揮発してよい変数（sqliteのインメモリDBで管理）、永続化が必要な変数（sqliteのファイルDBで管理）、これらを統一的なインターフェースで操作可能なKVSとせよ。また、現行では管理階層として「アプリ層」「ゴースト層」「シェル層」「バルーン層」を持つこととする。これらの改修は、wintfやdolaなどの基礎整備と同じフェーズで実施することとする。」
> 同日の答え: 層の意味＝**重ねて上書き**（バルーン→シェル→ゴースト→アプリの順に最初に見つかった値）・ファイルの置き場＝**各層の今の置き場**（今の `sylphya.toml` と同じフォルダ）・手で書く使い方＝**MCP の set で書く**（`mcp-set-property` へ分けた）・速い層＝**型付きの取っ手で直に読み、同じ値を名前でも読み書きできる**（ECS の値そのものは ECS に残す）・作り方＝**案 A（番人のアクターが接続 1 本＋ATTACH）**・分け方＝**2 本**（本 spec → `mcp-set-property`）・版＝**v0.2.0**。

## Problem

areka の値の持ち方が、置き場によってばらばらである。覚えておく値は `sylphya.toml` に、SHIORI の SetProperty で入る値はアクターの持つ写しに入る。窓の位置のような速く読む値は ECS にあり、名前では読めない。覚えておける鍵は型の決まった 9 種類だけで、新しい設定（例: `shell-balloon-scale-setting` の表示倍率）を足すたびに `PersistKey` の列挙・TOML の表・正規名の変換の 3 か所を書き足すことになる。

同じアプリ層のファイルには、ゴーストごとのアクターと、ホストの受け口のアクターと、UI スレッドの直書きの 3 者が書く。TOML を丸ごと書き直す形なので、2 者が同時に書くと片方が消える（`boot_resolve.rs` の注・`multi-ghost-runtime` が避けて通っている穴）。4 つの層の名前（`PersistScope` の App・Ghost・Shell・Balloon）はもうあるが、読むと全部が同じ `dotted_global` に入る。層どうしの上書きの順は決まっておらず、バルーンの層には置き場も無い。

## Current State

- **crate**: `crates/areka-sylphya`（約 7.5k 行・テスト込み）。依存は thiserror・tracing・toml・`areka-actor`。sqlite はワークスペースのどこにも無い。
- **読み**: `SharedMirror`（`mirror.rs:33,105`）の `RwLock<Arc<MirrorImage>>` を写し替える形。`SylphyaReader` の `resolve_flat`／`resolve_dotted`／`talk_snapshot`（`reader.rs:50,80,114`・トークごとに凍る）。値は全部文字列。大小文字を区別する。
- **書き**: `SylphyaPublisher`（`actor.rs:463`）の `publish_static`／`publish_shiori`／`set`／`persist_put`（投げっぱなし）／`barrier`／`close`。判断は純関数 `SylphyaCore::apply`（`actor.rs:260`）。`classify_set`（`actor.rs:140`）が SET を RuntimeCommand（予約だけ・配線なし）／NotSettable／StoreWrite（メモリーだけ）に分ける。
- **覚える値**: `PersistKey`（`persist/mod.rs:123`）＝WindowPos・BalloonOffset・BootCount・VanishCount・LastGhost・LastBalloon・LastShell・LastRunning・BasewareRoots。正規名は `areka.*`（`to_canonical_key` :166）。ファイルは `<root>/sylphya.toml`（`format-version = 1`・`persist/format.rs`）。書くたびに読んで混ぜて丸ごと書く（`save_scope` :276）。tmp に書いて rename（`persist/io.rs:77`）。失敗は `PersistOutcome::Degraded` から利用者への知らせへ流れる（完了 `write-failure-notice`）。
- **置き場**: アプリ＝`areka_user_data::app_profile_dir`（`<user_root>/profile/areka`）・ゴースト＝`<shiori.dir>/profile/areka/`・シェル＝`<shell.dir>/profile/areka/`（`areka-ghost/src/sylphya_wiring.rs`）・バルーン＝無し。
- **アクターを通らない読み書き**: `boot_resolve.rs:353,429`（前回の名前・走行中の印）・`placement/persist.rs:289`（位置を起動時に直接読む）・`roots/{mod,edit}.rs`（置き場の並び）が `load_scope`／`save_scope` を直に呼ぶ。
- **アクターの数**: ゴースト 1 体につき 1 本（`runtime.rs:568-640`）＋ホストの受け口のアプリ層専用の 1 本（`shiori_host.rs:199`）。
- **試験の偽物**: `FakePersistIo`（`io.rs:124`・`fail_next_commit`／`fail_next_read`）。`readonly-mode`（v0.1.0）は `PersistIo` を「書かない」ものに差し替えて止める作り。

## Desired Outcome

- **1 つの口で 3 つの層**: 呼ぶ側は鍵の名前と層の名前だけで get／set する。値がどこに住むか（速い層・メモリーの sqlite・ファイルの sqlite）は**鍵の表**が決め、呼ぶ側は選ばない。表に無い鍵（SHIORI の SetProperty の自由な鍵）はメモリーの sqlite に住む。
  - **速い層**: Rust のコードは型付きの取っ手でロックなしに直に読む（画面などの毎フレームの読み）。同じ値を名前で get／set しても同じ値に届く。
  - **メモリーの sqlite**: 揮発してよい property の値。終了で消える。
  - **ファイルの sqlite**: 覚えておく値。
- **4 つの層の重ね読み**: 同じ鍵を読むと、バルーン→シェル→ゴースト→アプリの順に最初に見つかった値を返す。書くときは層を名指しする。「全体の既定＋ゴーストごとの上書き」を、各機能が自前で組まずに書ける。
- **書き手が何人でも消えない**: アプリ層のファイルにゴーストのアクターが何本書いても、1 つも消えない（sqlite のトランザクションと待ちで守る）。
- **今の利用者の記憶が消えない**: 初めて起動したとき、各層の `sylphya.toml` を一度だけ取り込む。取り込んだ元のファイルは消さずに名前を変えて残す。取り込みに失敗したら記録を残し、元のファイルに触らずに既定値で動く。
- **今の振る舞いを保つ**: トークごとの凍った写し・書けなかったときの利用者への知らせ・`readonly-mode` の「1 つも書かない」・決定論の試験。

## Approach

**案 A＝番人のアクターが接続を 1 本持つ。** 今の sylphya のアクター（1 本のスレッド・`apply` の純関数）が rusqlite の接続を 1 本だけ持つ。そこへ 4 つの層のファイルと揮発用の `:memory:` を ATTACH し、重ね読みは SQL 1 回で引く。get も set もアクターへの問い合わせ（同期の返事）にする。トークの凍った写しは、トークの始めに 1 回の問い合わせで作る。速い層はアクターを通らない型付きの取っ手（今の `SharedMirror` と同じ写し替えの形）で持ち、名前での get／set もこの取っ手へ届ける。

- **採った理由**: 書き手が 1 人なので順番が崩れず、書いた値がすぐ読める。今のアクターの形にそのまま乗る。毎フレーム読む値は速い層が受け持つので、property の読み書きが往復 1 回になっても困らない。
- **採らなかった案**: 読む側もそれぞれ接続を持つ形（WAL と共有のメモリー DB・往復は無くなるが、接続の数と共有キャッシュの扱いが増え、書いた値をすぐ読めるかも考えることになる）。今の写し（`MirrorImage`）を読む口に残し、sqlite は保存だけに使う形（メモリーの sqlite と写しが同じ役を二重に持ち、開発者の 3 つの層の形から外れる）。TOML のまま層の抽象だけ足す形（sqlite へ移す依頼から外れる）。
- **依存**: `rusqlite`（`bundled`・MIT）。`tech.md` の依存の表・`THIRD-PARTY-NOTICES.md`・`deny.toml` の確かめを、依存を足すタスクの中で行う。
- **鍵の表**: 今の `PersistKey` の 9 種類と `vocab/dotted.rs` の表（`DOTTED_ROOTS`・`GENERIC_PROP_NAMES`・`SET_EFFECTIVE`）を 1 つの表へまとめる。鍵ごとに「住む層（速い／メモリー／ファイル）・書いてよいか・既定値」を書く。新しい覚える鍵は表に 1 行足すだけにする。

## Scope

- **In**:
  - KVS の口（鍵＋層での get／set・重ね読み・鍵の表）と、3 つの住み場所の実装。
  - rusqlite の導入と、ファイルの形（表の形・形の版の番号・形の版が上がったときの直し方）。
  - 各層のファイルの置き場（今の `sylphya.toml` の隣）。バルーンの層の置き場の決め（今は無い・バルーンのフォルダの `profile\areka`）。
  - `sylphya.toml` から一度だけ取り込む道。元のファイルは名前を変えて残す。
  - アクターを通らない今の読み書き（`boot_resolve.rs`・`placement/persist.rs`・`roots/`）を新しい口へ移す。書き手が何人いても消えないことの試験。
  - 書けなかったときの知らせ（`PersistFailure` の道）と `readonly-mode` の「書かない」の継ぎ目を新しい口へ移す。
  - 速い層の型付きの取っ手（今ある値の中で、名前でも読む必要があるものだけを載せる）。
  - 決定論の試験と、実機（実 emo2）で前回の位置・前回のゴーストが新しいファイルから戻ることの確かめ。
- **Out**:
  - MCP から名前で書く口（`mcp-set-property`）。
  - ECS の値そのもの（`wintf::ecs::WindowPos` など）を KVS へ移すこと（開発者「ECS の値そのものは ECS に残す」）。
  - `currentghost.*`・`system.*` などの SSP の property の値を流し込むこと（`currentghost-property-tree` ほかの持ち主の spec）。
  - 大小文字を区別しない読み（`property-name-case-fold`）。
  - pasta の `save.json`・MCP の橋渡しのキャッシュ（`areka-mcp-bridge/src/cache.rs`）など、sylphya を通らない保存。
  - 値に型を持たせること（今どおり文字列で持つ）。

## Boundary Candidates

- KVS の口と鍵の表（呼ぶ側が見る面・住み場所を隠す）
- 住み場所 3 つ（速い層の取っ手・メモリーの sqlite・ファイルの sqlite）
- 層の重ね読みと、層ごとのファイルの置き場
- TOML から一度だけ取り込む道（一度きりで、後で消せる）
- アクターを通らない今の読み書きの移し替え（`areka` 本体と `areka-ghost` の側）

## Out of Boundary

- MCP の道具の追加（`mcp-set-property`）
- property の値を集めて流し込む spec 群（`currentghost-property-tree`・`currentghost-property-others`・`system-property-values`・`zorder-property`）
- 複数のゴーストを同時に動かす仕組み（`multi-ghost-runtime`。本 spec は書き手が何人でも消えない土台だけを渡す）
- 設定画面（windows-reactor 待ち）

## Upstream / Downstream

- **Upstream**: `audit-sylphya-actor`（v0.1.1・同じ crate を見直して直す＝その着地の後に着手）・`readonly-mode`（v0.1.0・「書かない」の継ぎ目）・`multi-ghost-runtime`（v0.1.0 の余力の枠・着地していればアプリ層の書きの集め方に合わせる）・`property-name-case-fold`（v0.1.0・鍵の綴りの扱い）。
- **Downstream**: `mcp-set-property`（本 spec の名前での set の上に乗る）・`shell-balloon-scale-setting`（v0.4.0・表示倍率を鍵の表に 1 行足し、重ね読みで「全体の既定＋ゴーストごと」を引く）・`currentghost-property-tree` ほかの property の値の spec（速い層とメモリーの層へ値を流す）・予約「設定画面と設定ファイル」。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `sylphya`・`position-persist`・`write-failure-notice`・`baseware-root-list`・`ghost-shell-balloon-switch`・`sylphya-set-ledger` が作った形を置き換える）
- **Adjacent**: `audit-sylphya-actor`（同じ crate）・`readonly-mode`（`persist/`・`boot_resolve.rs`・`roots/`）・`multi-ghost-runtime`（アプリ層の書き手）・`property-name-case-fold`・`dpi-realign-remembered-chain`（`placement/persist.rs`）・`shell-balloon-scale-setting`

## Constraints

- v0.2.0（wintf・dola の基幹の整備と同じ段）。ただし wintf・dola の改造と触るファイルは重ならない（`areka-sylphya`・`areka` 本体の `boot_resolve.rs`・`placement/persist.rs`・`roots/`・`areka-ghost` の `sylphya_wiring.rs`・`runtime.rs`）。ウェーブに入れるときは、v0.2.0 の他の spec と触るファイルの重なりが 0 であることを示す。
- 利用者の記憶を 1 つも失わない（winget で入れ直しても消えない決まり＝完了 `user-data-root`）。取り込みは元のファイルを壊さない。
- 書けなかった道は必ず記録を残し、利用者へ知らせる（ログ無しの失敗の道は作らない）。
- 1 ファイル 1,000 行の上限・試験は兄弟のファイル・決定論（`thread::sleep` で待たない）。
- 実機の根と一時フォルダはワークツリーの `target\` の下。
- **起票のときの確かめ（2026-10-21・使い捨ての試作を 3 つの target で組んで動かした）**: rusqlite 0.40.2／libsqlite3-sys 0.38.2（2026-08-08・SQLite 3.53.2 を同梱・MIT）は保守されている。`bundled` は x64・ARM64・i686 の MSVC で警告なく組める。この repo の `deny.toml` で `cargo deny check licenses advisories` が通る（勧告 0 件）。ATTACH の上限は 10（使うのは 5）。日本語のフォルダ名も、ATTACH の引数として渡せば通る。重ね読みの `UNION ALL … ORDER BY 層 LIMIT 1` は期待どおりの層を返した。要件と設計で扱う注意:
  - 2 つ以上のファイルにまたがる 1 回のトランザクションは、マシンが落ちたときにファイルの間で不揃いになりうる（sqlite.org の ATTACH の説明）＝**1 回のトランザクションでは 1 つの層だけに書く**。
  - 複数のプロセスが書くので、書くトランザクションは `BEGIN IMMEDIATE` で始める（読んでから書く形は `busy_timeout` が待たずに SQLITE_BUSY を返す）。ATTACH はトランザクションの外でしか打てない。
  - WAL はローカルのディスクだけで動き、フォルダに `-shm` を書ける必要がある。ジャーナルの形はファイルごとに決める（`PRAGMA <schema>.journal_mode`）。書けないフォルダ（読み取り専用の置き場・`readonly-mode`）では WAL を使わない開き方にする。
  - SQLite の Windows の層はパスを 260 文字（MAX_PATH）までに制限する。深いゴーストのフォルダで当たりうるので、当たったときの扱い（記録と知らせ・ビルドの旗 `SQLITE_WIN32_MAX_PATH_CHARS`）を決める。Unicode として正しくない Windows のパスは開けない（`InvalidPath`）。
  - 確かめていないこと: ARM64 での実行・複数プロセスの書き込みの負荷試験・ネットワークの共有フォルダ・260 文字を超えるパス。
- 規模の見立て: 18〜24 タスク（口と表 4〜5・sqlite の住み場所と形 4〜5・取り込み 3・直の読み書きの移し替え 3〜4・知らせと書かない継ぎ目 2・速い層 2〜3・実機 1）。20 を超えても、これ以上は削らない。
