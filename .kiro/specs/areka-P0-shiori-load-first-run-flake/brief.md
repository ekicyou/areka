# Brief: areka-P0-shiori-load-first-run-flake

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の実機の確かめ〔タスク 9.2〕で 1 回だけ見た件。範囲の外の問題として、完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 9.2 の ⑦ と、その回の記録（下の Current State に要る行を写した。記録の置き場 `target\` は git に残らない）。**見たのは 1 回だけで、再現していない。原因は確かめていない。** 区分 B（バグ・観察）。

## Problem

emo2 を新しい写しから起こした実機の走行が、1 回だけ、SHIORI（pasta.dll）の LOAD の失敗で終わった。同じ手順でゴーストの写しを作り直して走らせたら通り、その後は起きていない。

- **利用者**: もし利用者の機械でも起きるなら、「入れたばかりのゴーストが、最初の 1 回だけ起動に失敗する」形になる（起きるかどうかは分かっていない）。
- **開発者**: areka の側の記録には「LOAD が成功の返事を返さなかった」までしか残らず、pasta.dll がなぜ断ったのかが分からない。実機の確かめを自動で回すと、理由の分からない赤が 1 回混ざる。

## Current State

2026-10-10 に見た 1 回（`seriko-trigger-intervals` のブランチ・debug 版の `areka.exe`・開発機 1 台）。

- **手順**: 同梱の emo2 をワークツリーの `target\` の下へ写し（毎回、前の写しを消してから写し直す）、`surfaces.txt` の末尾に検体の面を足し、`boot.lua` を差し替えて、`areka.exe` をそのフォルダの絶対パスで起こす。有界の自動終了つき。MCP の `dump_surface`・`dump_balloon` を別のプロセスから繰り返し呼んでいた。同じ日に、同じ手順の走行を前後で 5 本通している（どれも exit 0）。
- **失敗した回の記録**（areka の標準出力と、32bit の仲介 `shiori-host32-helper` の標準エラー）:
  - 起動は 11:49:30.53（UTC）。11:49:31.2〜31.9 に絵の焼き込み・seriko の表・文字の層の装着まで進んだ。
  - 仲介の標準エラー: `[helper] SHIORI 初期化の入口: loadu` → `[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse` → `Error: Error { code: HRESULT(0x80004005), message: "エラーを特定できません" }`。
  - 11:49:32.733 `ERROR shiori-actor: SHIORI 接続確立に失敗 … event="connect_failed" reason=SHIORI の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: [0]`。
  - 11:49:32.734 `ERROR kanade: SHIORI 呼出失敗——終了系列（Fault）へ event="shiori_failed"`。
  - 11:49:32.775 `ERROR areka::alert: [alert] 利用者へ告げます event="alert" scene=ShioriFault { … kind: ConnectFailed … }`。
  - 11:49:32.82 に MCP の待受を閉じて終了。走らせた側の測りでは、起動から 3.6 秒で終了コード 1。
- **残っていないもの**:
  - pasta.dll の側の記録。pasta.dll は、ゴーストのフォルダの `ghost\master\profile\pasta\` の下に自分のスクリプトの展開先（`pasta_scripts`）・`logs`・`cache`・`save` を作る。失敗した回のそのフォルダは、次の走行の前に写しごと消して作り直したので残っていない。
  - 走らせた側（Claude のセッション）は、失敗の直後に「アクセスが拒否されました (os error 5)」という文言を、pasta.dll が `profile\pasta\pasta_scripts` を展開する所の記録として見た、と書き残している。その文言を含むファイルは残っていない＝**手がかりであって、確かめた事実ではない**。
  - areka の記憶の置き場（その回は `AREKA_PROFILE_DIR` で走行ごとのフォルダへ向けていた）は空だった（書く所まで進んでいない）。
- **areka の側の伝え方は働いている**: 接続の失敗は記録に `ERROR` 3 行で残り、利用者へ告げる経路（`alert`）も通っている。黙って失敗した経路ではない。分からないのは、DLL の中で何が起きたか。
- **疑っているが確かめていない所**:
  - 新しい写しでは、pasta.dll が最初の LOAD でスクリプトを展開する。その書き込みが、何かと取り合った（直前の走行の `areka.exe` か仲介のプロセスがまだ終わりきっていなかった／写しを作る Python がまだファイルを掴んでいた／ウイルス対策の走査が、写したばかりのファイルを掴んでいた）。
  - 通った 5 本と失敗した 1 本の手順の違いは見つけていない。
- 近い過去の件: 完了 `areka-P0-ghost-session-test-load-flake` は、負荷の下で名前替え（`areka-nar` の確定）が OS に断られる揺れを「試し直す」形で直した。あちらは areka 自身の書き込みで、こちらは SHIORI の DLL の中の書き込みなので、同じ直しは当てられない。

## Desired Outcome

次のどれかで閉じる。

- 再現の条件が分かり、areka の側で防げるなら防ぐ（例: 前のゴーストのプロセスが終わりきるのを待ってから LOAD する、など）。
- areka の側では防げない（DLL の中・OS・ウイルス対策の側）と分かったら、その根拠を記録して閉じる。そのとき、利用者に見える形（最初の 1 回だけ失敗しうる）が残るなら、既定ゴーストの台詞で伝わることを確かめる。
- 再現しないなら、試した回数と条件を記録して閉じる。
- どの場合も、**次に同じ赤が出たら原因が読める**ようにしておく（失敗した回のゴーストの写しを消さずに残す手順・仲介が拾える情報が在れば記録に足す）。

## Approach

確かめが本体。

1. 新しい写しからの最初の起動を、同じ手順で数十回くり返して、失敗の割合を数える（1 回は数秒。窓が出るので、負荷を掛ける検査と同じ時間には回さない）。失敗した回は、ゴーストの写しを消さずに別の名前で残し、`ghost\master\profile\pasta\logs` を読む。
2. 取り合いの相手を 1 つずつ外して比べる: 前の走行の終わりから次の起動までを空ける／写した直後に待つ／MCP の呼び出しを止める。
3. 仲介（`crates/shiori-host32-helper`）が LOAD の失敗のときに残せる情報が在るかを読む（今は `LoadReturnedFalse` と HRESULT だけ。SHIORI の `load` は真偽しか返さないので、増やせるのは OS の最後のエラーくらいの見込み）。
4. 結果で、上の Desired Outcome のどれで閉じるかを決める。

## Scope

- **In**: 再現の試み・失敗した回の証拠の残し方・原因の切り分け・（areka の側で防げると分かったとき）その直し・（防げないとき）記録。
- **Out**: pasta.dll の中の直し（別のリポジトリ）。SHIORI の失敗の伝え方そのもの（完了 `areka-P0-shiori-fault-notice` で確定。変えない）。書けない場所に在るときの伝え方（`write-failure-notice`）。置き場を入れ先の外へ出すこと（`user-data-root`）。

## Boundary Candidates

- 実機の走行の手順（写しの作り方・前の走行との間隔）と、失敗した回の証拠の残し方。
- 仲介が LOAD の失敗で残す情報（`crates/shiori-host32-helper/src/shiori_proxy.rs`）。
- ゴーストの起こし直し・切替のとき、前の SHIORI のプロセスの終わりと次の LOAD の順（areka の側で防げると分かったときだけ）。

## Out of Boundary

- emo2 の台本・pasta の振る舞い。
- 負荷の下のテストの揺れ（`areka-test-threads-av`・`test-wait-marker-gaps`）。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: emo2 を新しい写しから起こす実機の確かめすべて（`mcp-kanade-tools` ほか、MCP で実機を回す spec）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**（読んで、重なりが無いことを確かめた）:
  - `emily-ghost-verification`・`shiori4-api`: どちらも「SHIORI の `load` へ渡すフォルダのパスの形（区切りの混在・末尾の区切りなし）」を疑い先・議題の候補に持つ。今回は同じ形のパスで 5 本通っているので、別の件と見ている。再現の調べでパスの形が効いていたら、そちらへ渡す。
  - `test-roots-under-target`: テストの一時フォルダを `target\` の下へ寄せる件。今回の走行の根は初めから `target\` の下で、決まりどおり。
  - `install-live-target-hazards`: 使っている最中のフォルダへの上書きを実測する件。「掴まれているファイルへ書く」という形は近いが、あちらはインストールの名前替え、こちらは DLL の中の書き込み。
  - `write-failure-notice`: areka 自身が書けないときの伝え方。SHIORI の DLL が書けないときは含まない。
  - `emo2-real-run-wrap-timeout`: emo2 の実走の別の赤（待ちの時間切れ）。原因が違う。

## Constraints

- 段は**バグ**・区分 B。見たのは 1 回＝まず再現を確かめる。
- 実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。emo2 の実走は絶対パスかつ短いパス。
- 止めてよいプロセスは、自分が起こしたと確かめられるものだけ。
- 実機の窓が出る検査なので、負荷を掛ける検査と同じ時間には回さない。くり返しの走行は、回す前に所要時間を伝える。
- 規模の見込み: XS〜S（確かめと記録だけ 1〜3・areka の側の直しが要ると分かれば 4〜7 タスク）。
- 要件定義のモデル: Opus。
