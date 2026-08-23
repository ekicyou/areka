//! 常駐 probe。発行点の interest を `sometimes` に合成し、他スレッドの先着で `never` が
//! 焼き付く経路を閉じる（プロセス寿命で冪等に確立する）。
