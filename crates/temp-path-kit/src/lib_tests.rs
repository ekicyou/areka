//! [`TempPath`](super::TempPath) の自己テスト（要件 12.1）。
//!
//! 主張は 3 本——同一プロセス内で 2 度作っても名前が衝突しない／破棄で実体が消える／
//! プロセス識別子が違えば別名になる。3 本目は別プロセスを起こしては決定論的に確かめられ
//! ないので、名前を組み立てる内部関数 [`compose_name`](super::compose_name) に識別子を
//! 引数で渡して直接確かめる。**その内部関数を実際に作る側も通っている**ことは
//! [`production_path_goes_through_compose_name`] が固定する（内部関数だけを試して
//! 本番経路が別の式で組んでいたら、テストは何も測っていない）。
//!
//! 併せて「違う名前になる」の主張が恒真でないことの較正を対で置く——**同じ札・同じ識別子・
//! 同じ連番なら同じ名前になる**ことを確かめれば、比較そのものが常に不一致を返す壊れた
//! 道具ではないと分かる。

use super::*;

/// 較正: 3 つの入力がすべて同じなら**同じ名前**になる。
///
/// 以降の `assert_ne!` 群が「何を入れても違う名前を返す」壊れた式の上に立っていないことを
/// 示す対である（この 1 本が無いと不一致の主張は恒真でも通る）。
#[test]
fn same_inputs_compose_the_same_name() {
    assert_eq!(compose_name("cage", 4242, 7), compose_name("cage", 4242, 7));
}

/// プロセス識別子だけが違えば別名になる（要件 12.1・プロセス間の一意性）。
#[test]
fn different_process_id_composes_a_different_name() {
    let mine = compose_name("cage", 4242, 7);
    let neighbour = compose_name("cage", 4243, 7);
    assert_ne!(
        mine, neighbour,
        "識別子が違うのに同じ名前を返している（プロセス間で奪い合う）"
    );
}

/// 連番だけが違えば別名になる（同一プロセス内の一意性の素）。
#[test]
fn different_serial_composes_a_different_name() {
    assert_ne!(compose_name("cage", 4242, 7), compose_name("cage", 4242, 8));
}

/// 札だけが違えば別名になる（別のテスト群の一時パスと混ざらない）。
#[test]
fn different_label_composes_a_different_name() {
    assert_ne!(compose_name("cage", 4242, 7), compose_name("kit", 4242, 7));
}

/// 同一プロセス内で 2 度作っても名前が衝突しない（連番が効いている）。
#[test]
fn two_temp_paths_in_one_process_do_not_collide() {
    let first = TempPath::new("collide-check");
    let second = TempPath::new("collide-check");

    assert_ne!(
        first.path(),
        second.path(),
        "同一プロセス内で同じ名前を配っている（連番が効いていない）"
    );
    // 非空虚性: 両方が実在していなければ「違う」だけでは何も担保していない。
    assert!(first.path().is_dir(), "1 つ目が実体を持っていない");
    assert!(second.path().is_dir(), "2 つ目が実体を持っていない");
}

/// 破棄で実体が消える（`Drop` の再帰削除が効いている）。
#[test]
fn dropping_removes_the_directory_and_its_contents() {
    let kept;
    let child;
    {
        let dir = TempPath::new("drop-check");
        kept = dir.path().to_path_buf();
        child = dir.child("nested.txt");
        std::fs::write(&child, "中身").expect("一時ディレクトリへ書けるはず");

        // 非空虚性: 消える前に実在していたことを先に固定する。作れていなければ
        // 「消えた」は無条件に成り立ってしまう。
        assert!(kept.is_dir(), "破棄前にディレクトリが無い");
        assert!(child.is_file(), "破棄前にファイルが無い");
    }

    assert!(!kept.exists(), "破棄してもディレクトリが残っている");
    assert!(!child.exists(), "破棄しても中身が残っている");
}

/// `child` は配られたディレクトリの下を指す（宛先の種類を増やさない形）。
#[test]
fn child_points_under_the_handed_directory() {
    let dir = TempPath::new("child-check");
    let target = dir.child("descript.txt");

    assert_eq!(target.parent(), Some(dir.path()));
    assert_eq!(
        target.file_name().and_then(|n| n.to_str()),
        Some("descript.txt")
    );
    // 返すのはパスだけで実体は作らない。
    assert!(!target.exists(), "`child` が実体を作っている");
}

/// 実際に作る側が [`compose_name`](super::compose_name) を通っている。
///
/// 内部関数の性質をいくら試しても、本番経路が別の式で名前を組んでいたら自己テストは
/// 何も測っていない。実配置の名前から連番を読み戻し、同じ内部関数で**逐語再現**できる
/// ことで両者が同一の式であることを固定する。
#[test]
fn production_path_goes_through_compose_name() {
    let dir = TempPath::new("link-check");
    let actual = dir
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .expect("配られたパスは末尾要素を持つはず")
        .to_owned();

    let serial: u32 = actual
        .rsplit('-')
        .next()
        .expect("名前は `-` 区切りの末尾に連番を持つはず")
        .parse()
        .expect("末尾は連番（10 進数）のはず");

    assert_eq!(
        actual,
        compose_name("link-check", std::process::id(), serial),
        "実配置の名前を内部関数で再現できない（本番経路が別の式で組んでいる）"
    );
    // 較正: 識別子を 1 つずらせば再現しないこと＝上の一致が恒真でないこと。
    assert_ne!(
        actual,
        compose_name("link-check", std::process::id().wrapping_add(1), serial),
        "識別子を変えても同じ名前になる（名前が識別子を含んでいない）"
    );
}
