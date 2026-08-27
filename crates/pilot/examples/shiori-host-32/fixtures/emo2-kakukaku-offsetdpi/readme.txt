areka-P0-balloon-offset-dpi 実機サインオフ用の検証バルーン
==========================================================

`fixtures/emo2-kakukaku-wplimit` の複製である（そちらは `fixtures/emo2/emo2-kakukaku`
の複製）。原本との差分は面別上書き層 2 ファイルの `windowposition` と `install.txt` の
`name`／`directory`、およびこの `readme.txt` だけで、既定層と画像は 1 バイトも違わない。

面別上書き層の中身:

  balloons0s.txt (scope 0 / sakura): windowposition.x,center   ← キーワード指定
  balloonk0s.txt (scope 1 / kero)  : windowposition.x,-190     ← 数値指定（素の追従）

なぜ混成なのか
--------------

本仕様の実機サインオフ手順（`transition_judge_offset_signoff_tests.rs` のモジュール doc
§3）は、必須の手として**両方**を要求する。

  手 2: 素の追従スコープ（キーワード指定でないバルーン）を最低 1 つ含めること
  手 3・4: キーワード指定のバルーンを、素材未消費のまま 1 度遷移させ、その後に消費させること

ところが既存の検体はどちらも片側しか持たない——

  fixtures/emo2/emo2-kakukaku        : 両スコープとも数値指定（キーワードが無い）
  fixtures/emo2-kakukaku-wplimit     : 両スコープともキーワード（素の追従が無い）

したがって**どちらの検体でも手順を満たせない**。前者では判定 ⑷ の母数が空になって
「揃えを 1 度も測れていない」の偽の赤、後者では判定 ⑶ の母数が空になり得て
「低い拡大率側で追随が出ていない」の偽の赤が出る。本検体はこの 2 つを同時に塞ぐために
scope 0 をキーワード・scope 1 を数値にした唯一の組み合わせである。

この穴は 2026-08-28 の実機サインオフ（task 10.1）で、手順どおりに進めようとして
初めて見つかった。手順書は「キーワード指定のバルーンを」と書くだけで、どの検体を
使えばよいかを言っていなかった。手順書 §1 に本検体を名指しする記述を足してある。

scope 1 の y について
---------------------

原本 `emo2-kakukaku` の `windowposition.y,-75` へ戻してある。数値指定の基本位置
（バルーン上端＝キャラ上端）を前提にした値であり、数値指定へ戻す以上こちらが素である。
scope 0 はキーワードのままなので wplimit 側の `windowposition.y,0` を引き継ぐ
（理由は `../emo2-kakukaku-wplimit/readme.txt` を参照）。

原本を書き換えず別ディレクトリへ分けたのは、`fixtures/emo2` を読む既存テストが
原本の数値指定 x=266 / x=-190 を期待値に持つためである。`fixtures/emo2` ツリーの外
（兄弟ディレクトリ）に置いてあるので、ゴーストツリーの列挙にも影響しない。

使い方: 起動時に argv[2]（バルーンルート）としてこのディレクトリの絶対パスを渡す。
手順の全体は `crates/areka/src/placement/transition_judge_offset_signoff_tests.rs` の
モジュール doc を参照。
