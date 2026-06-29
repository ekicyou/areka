# LLM参照を禁じる。このドキュメントは人間用です。LLMは参考にせず、書き込みも禁止


/kiro-discovery
## メッセージループとウィンドウ起動を**wintf-winmsg-executor**にベースに置き換える検討

現在、メッセージループはほぼ自作であるが、非同期関数の実行ができるメッセージループ作成クレート`wintf-winmsg-executor`を用意したので置き換えを実施せよ。

### 置き換えの同期

フォーク元である`winmsg-executor`は、Windows低レベルレイヤーに関する深い知見に基づいた洗練されたコードであり、私たち開発者の思い付きのようなコードより正しい挙動が得られると判断する。

また、UIスレッド上などで簡便に非同期コードが実行可能であり、利用上のトラブル余地も極めて小さい。wintfのコードベースがこれ以上膨らむ前に、置き換えを行うべきであると判断した。


### 必要と思われる仕様

+ `wintf-winmsg-executor`クレートの技術概要を`wintf-winmsg-executor`の実装ソースコードレベルで知見調査・記録し、置き換え可能か検討、置き換え時にどこを修正すべきかを判断する仕様
+ 置き換え可能と判断したら、実際に置き換えを行う仕様

### `wintf-winmsg-executor`の主な調査領域
+ doc.rsより
  + https://docs.rs/wintf-winmsg-executor/latest/wintf_winmsg_executor/
  + https://docs.rs/wintf-winmsg-executor/latest/wintf_winmsg_executor/util/struct.Window.html
    + fn new_ex（実装ソースコードも調査対象）
    + fn new_checked_ex（実装ソースコードも調査対象）
    + ソースコードの内部関数「get_instance_handle」この関数は現在未公開だが、次のバージョンで pub fn util::get_instance_handle()に変更します。
    + Windowを作るとき、`util::Window::new_ex`または`new_checked_ex`を使うこと。
  + https://docs.rs/wintf-winmsg-executor/latest/wintf_winmsg_executor/fn.block_on.html
  + https://docs.rs/wintf-winmsg-executor/latest/wintf_winmsg_executor/fn.spawn_local.html

### 検討すべき内容
+ ECSのtick起動をどうすべきか。ecs ticksを非同期タスクとして予約し、60Hz起動スレッドからウェイクアップしてもらう？多分、現在のやりかた（起動メッセージをpopする）方法より洗練されるはず。rust非同期関数のウェイクアップ作法ってあるのか？C#ならTaskCompletionSourceを使うべきシチュエーション。tokio依存は避けたい。event_listenerクレートがよさそうですね。

