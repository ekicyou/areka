# Brief: areka-P0-emily-ghost-verification

> 2026-10-08 `/kiro-discovery` で起票（開発者「SSP から取り出した、えみりゴーストがうまく動かないらしい。まあ SSP 専用ゴーストでいろいろ問題が出る可能性が高いが。何とか動くところまでは持っていきたい」）。着手の区分は **A（開発者が起票）**。

## Problem

SSP に同梱されている既定ゴースト「Emily/Phase4.5」（えみり・`emily4`）を areka で動かすと、うまく動かない。えみりは SSP の作者陣が SSP のために作ったゴーストで、SSP 専用の作りを多く持つ。areka の検体（emo2・里々と YAYA のテンプレート・claudia）はどれも「SSP 以外でも動くように書かれた」ゴーストなので、SSP 前提のゴーストで何が崩れるかを、areka はまだ一度も確かめていない。

## Current State

- 検体 `vendors/sample_ghost/emily4.nar` は 2026-10-08 に受け取ったバイト列のまま置いた（sha256 と出どころ・ライセンスは `vendors/sample_ghost/README.md` の「`emily4.nar` の出どころとライセンス」節）。**登記表 `SAMPLES`（`crates/sample-ghost-kit/src/lib.rs`）にはまだ載せていない**ので、`SampleRoot::acquire("emily4")` と `nar-sample-path emily4` はまだ使えない。
- **第一の壁＝インストールが断られる**。`install.txt` が `balloon.directory,emily4`・`balloon.source.directory,balloon` を名乗るのに、書庫に `balloon/` が無い（SSP ではバルーンが別の `balloon/emily4/` にあり、ゴーストのフォルダだけを畳むと抜ける）。`areka-nar` は `crates/areka-nar/src/plan.rs` の `companion_placement` で `RefuseReason::CompanionSourceMissing` を返し、インストール全体を断る（利用者の目には `crates/areka/src/install/judge.rs` の `failure_word` で `InvalidType` の語になる）。ukadoc の `*.source.directory` の項は、取り出し元が無いときの扱いを書いていない。
- 書庫の中身のうち、areka がつまずきそうなもの（2026-10-08 に書庫を開いて数えた。areka の対応の有無は要件の段で確かめる）:
  - SHIORI は `yaya.dll`（**i386＝32bit**）・`shiori.version,SHIORI/3.0`。32bit の道（shiori-host32）を通る。辞書は YAYA の `aya_*.dic` 31 本＋`system/` 5 本＋Lilith（`aya_lilith*.dic`・`lilith_config.txt`）。
  - キャラクターが 3 人（`sakura.name,Emily`・`kero.name,Teddy`・`char2.name,Emilio`・`char2.seriko.defaultsurface,200`・シェルの `char2.*`／`char3.*`／`char4.*` の配置の指定）。3 人目以降の窓は `extra-character-windows`（未着手）の持ち分。
  - シェルの PNG 115 枚（`surface*.png` 93・`element*.png` 21・`thumbnail.png` 1）のうち α を持つのは 14 枚だけ（色の型 6）。残り 101 枚は α の無い絵（型 2＝69・型 3＝32）で、**`.pna` が 27 枚**添えてある（基本の面 `surface0000`・`surface0010`・`surface0200` も型 2＋`.pna`）。areka は `.pna` を使わない（2026-10-05 裁定）ので、α の無い絵は左上 1 画素の抜き色で透過になる。
  - `surfaces.txt`: アニメーション 65 本（`sometimes` 22・`never` 14・`runonce` 11・`bind` 10・`always` 8）・`option,exclusive` 14・pattern の描画メソッド `overlay` 169・`bind` 10・`base` 2・element の `overlay` 67・**`replace` 6**（`draw-methods-canon` の持ち分）・`sakura.surface.alias`／`kero.surface.alias`・`surfacetable.txt`。`collisionex` は 0。
  - シェルの `descript.txt`: 着せ替え（`sakura.bindgroup100.*`・`sakura.menuitem0`）・オーナードローのメニューの絵（`menu.*.bitmap.filename`。areka は OS 標準のメニューなので使わない）・`*.balloon.alignment,none`。
  - ゴーストの `descript.txt`: `balloon,emily4`・`x-ssp.run_delete_txt,1`・`install.accept`・`updateurl`（SSP のサーバー）・`sstp.allowunspecifiedsend,1`・`icon`・`name.allowoverride,0`。最上位に `delete.txt`・`updates2.dau`・`thumbnail.png`。
- SSP は開発者の機械の `C:\wintools\ssp\` に入っており、`balloon\emily4\`（41 ファイル）もそこにある。

## Desired Outcome

- えみりが areka で**起動して喋り、撫でと右クリックメニューと終了まで通る**（「何とか動く」の線）。見た目が SSP と同じであることは求めない。
- 動かなかった原因が 1 つずつ記録され、areka 側で直せるものは直し、areka の方針で受けないもの（`.pna`・オーナードローのメニューなど）はそう書かれている。範囲の外の未対応は全部起票されている（開発者方針「実機で areka の未対応のためにうまくいかなかった件は、範囲外でもすべて起票」）。
- `emily4` が登記表 `SAMPLES` に載り、`SampleRoot::acquire("emily4")` で取れる。

## Approach

検証を主にした spec。まず「インストールの断り」の扱いを決め、検体を登記して実機で起こし、崩れた所を記録の上から順に潰す。

1. **インストールの扱いを決める**（要件の段の議題）。案は 2 つ——(a) 取り出し元が無い同梱バルーンは**その 1 つだけを飛ばして記録を出し**、ゴーストは入れる（えみりはゴーストの `balloon,emily4` が解けず、`ghost-standard-balloon` の決まりで既定バルーンに落ちる）。(b) 今の断りを正しいとし、検体のほうを SSP の `balloon\emily4\` を足した形に直す（受け取ったバイト列から外れる・バルーンのライセンスは書庫の `readme.txt` に書かれていない）。ukadoc の語と、SSP の作者のゴーストを「SSP から取り出した」形の nar が出回る現実で裁定する。
2. 登記表に 1 行足し、直書きの検体数（`crates/sample-ghost-kit/src/lib_tests.rs` の `SAMPLES.len()` の 7 が 2 か所・文書注釈の 7 が 3 か所・`unknown_sample_fails_with_all_seven_known_names` の名前の一覧と関数名）を 8 に直し、`vendors/sample_ghost/README.md` の表の「未登記」の注記を外す。
3. 実機で起こす（debug 版・`RUST_LOG` を判定の分岐まで開ける・MCP の `dump_surface`／`dump_balloon`／`get_log` で撮って記録）。落ちる・出ない・止まる所を 1 つずつ根本で直し、決定論テストを添える。
4. 範囲の外の未対応は `/kiro-discovery` で起票し、この spec では台帳に名前を書くだけにする。

## Scope
- **In**: 取り出し元の無い同梱バルーンの扱いの裁定と実装・`emily4` の登記・えみりの実機の検証（起動・会話・撫で・メニュー・終了）・その途中で見つかった areka 側の不具合のうち、えみりを「何とか動く」線へ持っていくのに要るものの修正・見つかったものの一覧（直した／方針で受けない／起票した）。
- **Out**: SSP との見た目の完全一致・`.pna` の対応（受けない裁定済み）・オーナードローのメニュー・3 人目以降のキャラクター窓（`extra-character-windows`）・`replace` ほかの描画メソッド（`draw-methods-canon`）・えみりの辞書（YAYA の台本）の書き換え。

## Boundary Candidates
- インストールの計画（`areka-nar` の同梱バルーンの取り出し）
- 検体の登記（`sample-ghost-kit`）
- 実機の検証の記録と、そこから出る個別の修正（どの層に入るかは見つかってから）

## Out of Boundary
- 見つかった未対応のうち、えみりを起こすのに要らない機能の実装（起票だけ）
- SSP の挙動の実測を根拠にした実装（意味論は ukadoc から取る方針）

## Upstream / Downstream
- **Upstream**: `nar-install`・`install-companion-reading`・`ghost-standard-balloon`（✅ 10-07・ゴーストの `balloon` 指定が解けないときに既定へ落とす決まり）・`self-alpha-declaration`（✅・α の無い絵の扱い）・shiori-host32（32bit YAYA）
- **Downstream**: 3 人キャラクターの検体が要る `extra-character-windows`・`replace` を描く `draw-methods-canon`・着せ替え（bindgroup）の残件

## Existing Spec Touchpoints
- **Extends**: なし（完了 spec は直さない）
- **Adjacent**: `extra-character-windows`（char2 の窓）・`draw-methods-canon`（`replace`）・`install-live-target-hazards`（インストールの計画を触る）・`seriko-trigger-intervals`（アニメーションの起動の型）

## Constraints
- 実機の根と検体はワークツリーの `target\` の下だけ（`C:\` 直下・`C:\tmp` は不可）。emo2 と同じく絶対パスで起こす。
- 32bit の SHIORI なので、debug 版の `areka.exe` の隣に i686 の helper が要る。
- 検体のライセンスは CC BY-NC 4.0＝**areka の配布物へ同梱しない**（`tools/package.ps1` は emo2 だけを入れるので今は触らない）。
- 「何とか動く」までに見つかる不具合の数が読めない。規模の見立ては要件の段で測り直す（上限は 1 spec 20 タスク・見込みは M）。直す数が膨らむなら、一覧を作って起票へ回す線を要件の段で引く。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 起票（10-08）の後に着地したのは `balloon-lifecycle-events`・`wintf-tooltip`・`ghost-session-test-load-flake`。本文の指し先はすべて実物と合っている。
  - 検体は未登記のまま。`crates/areka-nar/src/plan.rs` の `companion_placement` が、取り出し元の無い同梱でインストール全体を断る。検体数の直書きの 7 は `crates/sample-ghost-kit/src/lib_tests.rs` に 2 か所。
  - 32bit の YAYA の道は、検体 `claudia` で実機まで通っている（`ghost-standard-balloon` の実機の記録）＝えみりの次の壁にはならない見込み。全体テストの後は debug の helper が x64 版に替わる罠がある（`dev-helper-x64-clobber`）。
- 検証そのものが触るファイル:
  - インストールの扱い（案 (a) の場合）: `crates/areka-nar/src/plan.rs`（307）と新しい兄弟のテスト（`plan_tests.rs` は **919 行**なので足さない）・要れば `crates/areka/src/install/judge.rs`。案 (b) なら 0。
  - 登記: `crates/sample-ghost-kit/src/{lib.rs, lib_tests.rs}`・`vendors/sample_ghost/README.md`。
  - 記録: この spec のフォルダの中の検証の記録（文書だけ）。
  - 直しがどのファイルに入るかは、起こしてみるまで分からない。
- 規模: 8〜12 タスク（下の線を引いた場合）。
- 先に要るもの: なし（今すぐ着手できる）。ファイルの重なりは、`install-live-target-hazards`（`install/judge.rs`）・`ghost-inner-balloon`（`areka-nar` の `plan.rs`・向こうが書庫に鍵を足す場合）。`shell-companion-balloon`・`balloon-canon-residue`・`network-update-canon-order`・`baseware-root-list` とは 0。
- 優先度の区分: A（開発者「えみりゴーストがうまく動かないらしい」「何とか動くところまでは持っていきたい」）。
- 要件定義のモデル: Fable（インストールの断りの裁定・どこまでをこの spec で直すかの線）。
- 分割の案（ほかの列を止めないための線）: この spec は「起こして、確かめて、一覧にして、起票する」まで。この spec の中で直すのは次の 2 つだけにする。
  - ⑴ 自分の持ち場（インストールの計画・検体の登記）の直し。
  - ⑵ 1 タスクで済み、同じウェーブのほかの spec が触らないファイルだけで閉じる直し（上限 4 件）。
  - それ以外は、えみりの記録を根拠に 1 件ずつ起票する（区分は A を引き継ぐ）。「何とか動く」の最後の確かめは、起票した直しが着地した後に、この spec が残す実機の手順をもう一度回して行う。
- 見つけた穴・古くなった記述:
  - 登記の行に同梱バルーン `emily4` を書くと、検体のパスの綴りの番人（`crates/log-capture-kit/tests/sample_path_guard_test.rs`）の見張る語に `emily4` が加わる。`crates/areka/src/mcp/` のテスト 13 本が `C:\ssp\ghost\emily4` を書いている（今の綴りは当たらない見込み。登記のときに番人を回して確かめる）。
  - 最初に出す面の番号は 0 と 10 の決め打ちで（`crates/areka/src/emo2_boot/assets.rs`）、descript の `*.seriko.defaultsurface` はどこも読んでいない。えみりの表情の出だしが違って見えたら、ここが原因の候補。
  - roadmap の台帳の行の「`install-live-target-hazards` と同時に走らせない」は、重なるファイルが `install/judge.rs` の 1 本だけ＝どちらかが触らない約束をすれば並べられる。

## 2026-10-10 棚卸㉓の申し送り

- **YAYA が起きないときに疑う所の候補: SHIORI の `load` へ渡すフォルダのパスの形**。areka が渡すパスは `…\ghost/master` の形（区切りが混ざり、末尾に区切りが無い）。出どころは、ゴーストの根に `ghost/master` をつなぐ所（`crates/areka-parsers/src/package/resolve_shell.rs` の定数 `GHOST_MASTER`）。同じプロセスで読む道（`crates/areka-ghost/src/shiori_inproc.rs`）も、32 ビットの補助プログラムの道（`crates/shiori-host32-helper/src/shiori_proxy.rs` の `init_bytes`）も、形を整えずにそのまま渡す。正典はパスの形を決めていない。
- 検体 `claudia`（32 ビットの YAYA）はこの形で動いているので、最初に疑う所ではない。辞書やファイルが見つからない形で止まったら候補に入れる。
- 直すと SHIORI へ渡すバイト列がすべてのゴーストで変わる＝この spec の中の小さな直しとしては扱わず、開発者に聞く（`shiori4-api` の議題の候補にも同じ事実を書いた）。
- 上の再測定の「descript の `*.seriko.defaultsurface` はどこも読んでいない」を引き直した。製品コードでこの綴りが出るのは、プロパティの名前の表（`crates/areka-sylphya/src/vocab/dotted.rs` の `SET_EFFECTIVE`）だけで、descript から最初の面を読む所は 0。
