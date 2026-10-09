# Brief: areka-P0-shiori4-api

> 起票: 2026-10-10（`/kiro-discovery`・開発者「SHIORI4（COM の SHIORI）の API 整備の spec はあるか。いまは最低限の決まりだけ作っていて、どこかの段階で着手しないといけない」）。
> **区分 C・台帳の段は「その他」**（開発者）。**着手は、実装がある程度整備された段階で**。細かい調査と仕様の検討はそのときに行う（開発者「そこで仕様を検討するのがよい」「取り掛かりたいと思ったときに調査すべき」）。この brief は、着手のときの出発点として、いまの実情と開発者の望みを書き留めるもの。

## Problem
areka の SHIORI の口には、areka 独自の正準形式（呼び名は SHIORI4。COM の `IShiori` の境界を流れるもの）と、従来の SHIORI/3.0 のテキスト（呼び名は SHIORI3）の 2 つがある。設計の方針は「本体と進行役は SHIORI4 だけを話し、SHIORI3 への変換は過去互換のアダプタが受け持つ」だが、いまは最低限の決まりしか作っておらず、実際の経路はこの形になっていない。

- 本番の経路は SHIORI4 を通らない。本体は番号付きの Reference の並びを渡し、32bit ホストの側が SHIORI/3.0 のテキストを組み立てている。
- SHIORI の側からベースウェアへ問い合わせる口（シンク系インターフェース。プロパティの読み書きなど）は、COM の面には定義があるが、本番ではどこにもつながっていない。
- SHIORI4 の中身の形式は、紙の上の契約と、実際に流れているものが食い違っている。第三者はどちらで SHIORI を書けばよいかを文書から決められない。

## Current State
2026-10-10 の調査（main `dbb3c758` の後・コードは読んだだけ）。着手のときに調べ直すこと。

**3 つの層が食い違っている**

| 層 | 決まっていること | 実体 |
|---|---|---|
| COM の境界 | `IShioriFactory`・`IShiori`・`IShioriHost` の 3 本。流れる文字列は中身を問わない HSTRING | `crates/shiori-abi/`（実装とテストがある） |
| 中身の契約（紙） | JSON-RPC 2.0 の封筒・意味名の引数・446 項目のカタログ。版は `contract_version = "0.x"` | `doc/shiori/fragments/`（TOML だけ。読むコードは無い） |
| 実際に流れているもの | SHIORI/3.0 のテキストを、そのまま HSTRING に入れたもの | `crates/areka-ghost/src/shiori_inproc.rs`・`crates/shiori4-testdll/` |

**経路**

- 本番は `ShioriWiring::Helper` に固定（`crates/areka/src/boot_config.rs`・`ghost_session.rs`・`emo2_boot/mod.rs`）。進行役（kanade）の共通の口 `ShioriBackend`（`crates/areka-kanade/src/shiori/real.rs`）が、イベント名と番号付きの Reference の並びを受け、`crates/shiori-host32-host/src/shiori3.rs` が SHIORI/3.0 のテキストを組み立てて 32bit の helper へ送る。`IShiori` はこの経路に現れない。
- SHIORI4 と SHIORI3 を変換するアダプタは無い。`crates/shiori-host32-host/src/client.rs` に「`IShiori` への写像点は型で示すだけで実装しない」という注釈があるだけ。
- `IShiori` を通るのは `ShioriWiring::InProc`（x64 の DLL を同じプロセスに読む経路）だけで、使うのはテスト用の DLL。ゴーストの descript.txt から選ぶ道は無い。

**シンク系インターフェース（SHIORI → ベースウェア）**

- COM の面には `IShioriHost` の 4 つがある: `Raise`（SHIORI からの自発の通知）・`Complete`（遅れて返す応答）・`GetProperty`・`SetProperty`（`crates/shiori-abi/src/interface.rs`）。
- 実装は 3 つあり、どれも本番に届いていない。
  - `ShioriHostSink`（`crates/areka/src/shiori_host.rs`）: プロパティは sylphya につながっているが、環境変数で有効にするデモからしか使われない。`Raise` は受けて溜めるだけで、上へ配らない。
  - `InProcHost`（`crates/areka-ghost/src/shiori_inproc.rs`）: プロパティは孤立した表。`Raise` は警告を出して捨てる。`Complete` は常に断る。
  - 本番の Helper の経路: 口が無い。
- `GetProperty` の名前の決まりは「ドット区切りのパス」とだけ書いてある。`Raise` に渡すものが何か（さくらスクリプトか、イベントか）は決まっていない。

**そのほか、最低限のまま残っているもの**

- 版や能力を確かめ合う口が無い。IID は「開発用・リリースで凍結」と注釈がある。
- 紙の契約は古いメソッド名（`Request`）のまま。応答の形・共通の欄（`Sender`・`SecurityLevel` など）の置き場・`Notify` の写し方が決まっていない。
- 第三者向けの仕様書・単独で建てられる見本・検査の道具が無い。`shiori-abi` は `publish = false`。
- 完了済みの spec（`shiori-com`・`shiori-protocol`・`shiori-reference`・`shiori4-test-ghost`・`host32-shiori-load`）が「下流」「別仕様」「M2 の予約」と書いて送り出した宿題の引受先が、spec として存在しない。

## Desired Outcome
着手のときに要件として確かめ直す。いまの時点で開発者が望んでいる形は次のとおり。

- **経路が「areka → SHIORI4 → SHIORI のホスト」になっている**。本体と進行役は SHIORI4 だけを話し、SHIORI/3.0 への変換はホストの側のアダプタが受け持つ。いま本体からホストへの経路でやっていることを、この形に整える。
- **シンク系インターフェースが本番で使える**（いま特に欲しいもの）。SHIORI の側から、プロパティの読み書きなどをベースウェアへ問い合わせられる。
- SHIORI4 の中身の形式が 1 つに決まり、紙の契約と実装が一致している。

## Approach
**決めていない。着手のときに調べて決める。** いま分かっている分かれ目だけ書き留める。

- SHIORI4 の中身を何にするか（紙の契約どおり JSON-RPC にするか、別の形にするか）。
- 32bit の SHIORI/3.0 の DLL に対して、シンク系インターフェースをどう届けるか。従来の DLL には呼び返す口が無いので、プロパティの読み取りは台本を経由する道（`property-query-channels`）と、プロセス間の輸送（`property-ipc-transport`）のどちらか、または両方になる。
- 共通の口 `ShioriBackend` を SHIORI4 の形へ替えるか、既定の実装を持つメソッドを足して広げるか。実装は 20 か所（製品 3・テストの偽物 17）。
- `Raise`・遅れて返す応答・版と能力の確認を、この spec に入れるか、後へ回すか。
- x64 の SHIORI4 の DLL を descript.txt から選べるようにするか（roadmap の予約「pasta の native x64／`IShiori` in-proc」と同じ話）。

規模は 1 spec の上限（20 タスク）を超える見込み。**着手のときに要件の段で切り出す**（先に分けて起票しない）。切り出しの候補は「シンク系インターフェースの本番配線」「経路の組み替え」「中身の形式の確定と文書」。

## Scope
- **In**（着手のときに確かめ直す）:
  - 経路の組み替え（本体・進行役 → SHIORI4 → ホストの側のアダプタ → SHIORI/3.0）。
  - シンク系インターフェースの本番配線（少なくともプロパティの読み書き）。
  - SHIORI4 の中身の形式の確定と、紙の契約（`doc/shiori/fragments/_shared.toml` の封筒）との一致。
  - 古くなった注釈の掃除（`main.rs`・`shiori-abi`・`shiori_inproc.rs` に残る、済んだタスクを指す注釈）。
- **Out**: 下の「Out of Boundary」。

## Boundary Candidates
- COM の面（`shiori-abi`: メソッド・IID・HRESULT）。
- 中身の形式（封筒・要求・応答・エラー）と、その組み立て・読み取り。
- SHIORI4 ⇄ SHIORI3 のアダプタ（`shiori-host32-host` の x64 の側）。
- シンク系インターフェースの受け側（プロパティは sylphya・通知は進行役）。
- 進行役の共通の口 `ShioriBackend` と、その実装 20 か所。
- 第三者向けの文書と見本。

## Out of Boundary
- 32bit の helper と IPC の中身（`shiori-host32-helper`・`shiori-host32-ipc`）の作り替え。helper は SHIORI の中身を知らないバイト列の中継のまま。プロパティの輸送のためにタグを足すなら `property-ipc-transport` の仕事。
- SHIORI/3.0 の要求へヘッダを足す仕事（`script-security-level`・`mcp-shiori-query`）。
- MAKOTO の DLL（`makoto-dll-host`。同 brief は「x64 の `IShiori` 版は作らない」と明記）。
- pasta の上流を SHIORI4 に対応させること（別リポジトリ）。
- `shiori-abi` の crates.io への公開。要るかどうかは、形式が決まってから別に判断する。
- SSP を動かして挙動を確かめること。

## Upstream / Downstream
- **Upstream**: 完了 `shiori-com`（COM の面）・`shiori-protocol`／`shiori-protocol-split`（紙の契約）・`shiori4-test-ghost`（テスト用の DLL と InProc の経路）・sylphya（プロパティ）。
- **Downstream**: x64 の SHIORI4 の DLL を本番で読む仕事（roadmap の予約）・第三者が SHIORI4 で SHIORI を書くこと。

## Existing Spec Touchpoints
- **Extends**: なし（完了済みの spec の宿題を引き受ける）。
- **Adjacent**（同じファイルを触る＝同時に走らせない）:
  - roadmap の直列の列「host32-host」＝ `script-security-level` → `mcp-shiori-query` → `makoto-dll-host` → `makoto-reload-directives` → `property-ipc-transport`。触るのは `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/{runtime.rs, shiori_inproc.rs}`・kanade の `shiori/real.rs`。この spec はこの列の後ろに並ぶ。
  - `property-ipc-transport`（据え置き）: `IShioriHost::GetProperty`／`SetProperty` をプロセス間で運ぶ道。シンク系インターフェースと主題が重なる。着手のときに、どちらが何を持つかを決め直す。
  - `property-query-channels`: 台本を経由するプロパティの読み取り。
  - roadmap の予約「pasta の native x64／`IShiori` in-proc（本番に使うときは InProc の SHIORI を外から終わらせる手を決める）」。

## Constraints
- **優先度**: 区分 C・段「その他」（開発者・2026-10-10）。夢よりは上。急がない。
- **着手の条件**: 実装がある程度整備されてから。着手のときに、この brief の「Current State」を調べ直す。
- **1,000 行の上限に近いファイル**（2026-10-10）: `crates/areka-kanade/src/` の `msg.rs` 926・`actor.rs` 900・`schedule/mod.rs` 955・`schedule/steady.rs` 950、`crates/areka/src/main.rs` 950・`emo2_boot/spine.rs` 937、`crates/areka-ghost/src/runtime_tests.rs` 986、`crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` 1135（すでに超えている）。
- **host-32 は後付け**。進行役（kanade）の変更は、ホストのための特別扱いを足す向きでなく、切り離す向きでだけ行う。
- **紙の契約の正本は `doc/shiori/fragments/`**。契約を別のファイルへ二重に書かない。
- **32bit で建つ範囲**は `shiori-host32-*` と `shiori-abi` だけ。
- 挙動の意味は ukadoc から決める（SSP の実測に頼らない）。
