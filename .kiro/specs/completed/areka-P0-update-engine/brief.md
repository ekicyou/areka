# Brief: areka-P0-update-engine

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票。`areka-P0-network-update`（台帳 #16）の Approach ①②⑤（定義ファイル・取得・後始末）を**単独の spec として切り出した**（台帳 #51）。前例は `areka-P0-nar-install`（エンジン）と `areka-P0-ghost-install`（製品への結線）の分け方である。
> 切り出しの理由は 2 つ。⑴ #16 の想定タスクが 24〜30 本で 1 spec の上限（20 本）を超えた。⑵ エンジンの入力は「`homeurl` の文字列」と「対象フォルダのパス」だけで、**根の配置・切替・インストールのどれも待たずに今日着手できる**。α の直列経路の外で最も長い仕事を先に走らせられる。
> 正典の要点（`ukadoc:dev_update`／`ukadoc:manual_update`／`ukadoc:spec_update_file:*`）は親 brief（`.kiro/specs/areka-P0-network-update/brief.md` の Current State）が正本。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: ゴーストの作者が出した修正を受け取りたい第三者（α の利用者の一周の 5 歩目）。

ネットワーク更新に当たる実装は 0 である。`git grep -l` で `WinHttp|WinInet` は 0 ファイル、`updates2|updates\.txt` は 0 ファイル、`OnUpdate` は 0 ファイル。`delete\.txt` は `crates/areka-parsers/src/package/validation_tests.rs` の 1 ファイルだけで製品コードは 0（同じ書式の検索が `OnClose` で 63 ファイルに当たることを確かめてある）。`homeurl` は語彙だけが在り（`crates/areka-sylphya/src/vocab/shiori_resource.rs`・`dotted.rs`）、値を読む経路は無い。

## Desired Outcome

完了時に、**ネットワークに出ない決定論テストだけで**次が言い切れている。

1. `updates2.dau`／`updates.txt` を読める（行の形・拡張フィールド・無効な行 3 種・URL 符号化の判定・`charset` あり／なし）。
2. 定義ファイルとローカルの木から**差分**（取得が要るファイルの一覧）を MD5 で導ける。
3. 「定義ファイル → 差分 → 取得 → 照合 → 確定」の一周が、偽の取得口で回る。
4. **確定は全部入るか、1 つも入らないかのどちらかである。** 途中で失敗したら対象フォルダは無傷で、失敗の理由と、どこまで進んだかが `error!` に残る（記憶 areka-log-first-no-silent-failure）。
5. `delete.txt`／`delete[数字].txt` を安全に適用できる（対象フォルダの外・親への上り・絶対パスを拒む 3 種）。
6. 本物の取得口（WinHTTP）が 1 つ在り、実機で 1 周できる。
7. 結果が「結線する側が `OnUpdate*` の Ref を組み立てるのに足りる形」で返る（進んだ段・ファイル名・件数・失敗の理由）。**イベントは送らない。**

## Approach

**新しいクレート**（仮に `crates/areka-update/`）に閉じる。ワークスペースの `members` は `crates/*` の一括指定なので、根の `Cargo.toml` に行を足さずに済む。`windows` の機能 `Win32_Networking_WinHttp` は、`crates/dola/Cargo.toml` がしているのと同じく**クレート自身の `Cargo.toml` で足す**（根の機能一覧に触らない＝同じウェーブの他 spec と共有 0 を保つため）。

- 取得は `HttpFetch` の境界（`get(url) -> bytes`）。常時テストは偽実装だけを使う（roadmap「制約」＝ネットへ出るテストを常時テストに入れない）。
- 確定は `areka-nar` と同じ「作業フォルダに組んでから宛先と入れ替える」形。**同じ仕組みを 2 つ作らない**——`areka-nar` の確定の部品を公開して使えるか、要件段階でまず確かめる。

要件段階で決めること（裁定候補）:

- **MD5 の出どころ**。⑴ `md-5`（RustCrypto・MIT OR Apache-2.0・`deny.toml` の許可リスト内・棚卸⑭の仮裁定 5 で「承認待ち」）／⑵ OS の CNG（`Win32_Security_Cryptography`・依存 0）／⑶ 自前（`areka-nar` が `crc32.rs` を自前で持つのと同じやり方・依存 0）。**推すのは ⑵**——MD5 は今さら自前で書くものではなく、取得を OS（WinHTTP）に任せるのと筋が揃い、依存の承認そのものが要らなくなる。
- **定義ファイルの既定 charset**（棚卸⑭の裁定候補 ⑸＝Shift_JIS 固定を推す）。

## Scope

- **In**: 定義ファイル 2 形式の読み手／差分計算／`HttpFetch` の境界・偽実装・WinHTTP 実装／一時フォルダへの取得と照合／全か無かの確定／`delete.txt`／結果の型。
- **Out**: `OnUpdate*`・`OnUpdateOther*` の送出と Ref・`OnUpdateProcessExec`・`useorigin1`／台本の入口（`\![updatebymyself]`・`\![update,…]`・`\![updateother,…]`・`\![execute,install,url,…]`）／メニュー登記／更新後の読み直し／**網羅台帳（`doc/ukadoc-coverage/ledger/*.toml`）の状態の更新**——以上はすべて親 spec `areka-P0-network-update` が持つ。`updates2.dau` の書き出し・定期自動更新・本体の更新は α 後。

## Boundary Candidates

- 定義ファイルの読み手と差分（純関数）
- 取得の境界（`HttpFetch` と 2 つの実装）
- 確定と後始末（ファイル操作）

## Out of Boundary

- kanade・`emo2_boot`・`main.rs`・メニュー（1 行も触らない）
- `areka-nar` の振る舞い（確定の部品を公開するだけなら可・振る舞いは変えない。**その場合 `areka-P0-nar-install-hardening` と `crates/areka-nar/src/` を共有するので、どちらが先に触るかを着手時に決める**）

## Upstream / Downstream

- **Upstream**: なし。完了 `areka-P0-nar-install`（確定の形の前例）の上に建つ。
- **Downstream**: `areka-P0-network-update`（本エンジンを kanade の相・台本・メニューへ結線する）。

## Existing Spec Touchpoints

- **Extends**: なし（新規クレート）。
- **Adjacent**: 同じウェーブの他 spec との共有は、実測で次の 2 つだけ。⑴ `.kiro/steering/tech.md`（依存を足した場合の登記。CNG を選べば足さない）。⑵ `THIRD-PARTY-NOTICES.md`（`cargo about` の生成物。ワークスペースのクレートが増えると 1 行増える）——**競合したら手で直さず再生成する**（記憶 both-branches-same-wrong-count-merges-silently）。

## Constraints

- 規模 **M**（タスク 12〜14 本）。**要件と設計は Fable**（第三者のフォルダを書き換える仕事で、全か無かの確定と失敗時の保全が主題。正典の行の形の読み取りも判断が要る）。
- ネットへ出るテストを常時テストに入れない。実機の 1 周はローカルの HTTP（`crates/pilot` の example で足りる）か開発者の配布サーバ。
- 常時テストは x86 を避け、偽境界で純 x64 の決定論にする。
- ライセンスの関門（`cargo deny check` → `cargo about generate`）を依存の増減があれば必ず通す。
- `[profile.release]` は `opt-level='z'`——実機の 1 周はこの設定で走る。
- 1 ファイル 1,000 行。
