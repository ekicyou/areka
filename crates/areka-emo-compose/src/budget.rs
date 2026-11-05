//! 広がりの予算（指摘 3）: 入れ子をたどる走査が 1 回の範囲でたどってよい面の数を数え、
//! 尽きたら入るのを断る。菱形の参照（1 つの面が同じ面を何度も insert する）で広がりが
//! 指数になっても、走査が固まらないようにする。深さではなく「たどった面の総数」を数える。

use crate::world::Records;

/// 1 回の走査の範囲でたどってよい面の数。
/// 仮の値: 3.2 で「1 フレーム（16.667 ms）で回りきれる面の数」を測って置き直す。
pub(crate) const VISIT_BUDGET: usize = 1 << 20;

/// 走査 1 回の範囲の残りの数と、報告済みの印。
#[derive(Debug)]
pub(crate) struct Budget {
    limit: usize,
    left: usize,
    reported: bool,
}

impl Budget {
    /// 記録を出す予算。本番の口は [`VISIT_BUDGET`] を、テストは小さな数を渡す。
    pub(crate) fn new(limit: usize) -> Self {
        Self::with_records(limit, Records::Emit)
    }

    /// 記録の出し分けがある走査の口。`Records::Quiet` なら尽きても `error!` を出さない。
    pub(crate) fn with_records(limit: usize, records: Records) -> Self {
        Self {
            limit,
            left: limit,
            reported: !records.emits(),
        }
    }

    /// 面に 1 つ入る前に呼ぶ。残りがあれば減らして true。尽きていれば、範囲で初めての
    /// ときだけ `error!`（`walk`＝走査の名前・`root`＝根の面・予算）を出して false。
    pub(crate) fn enter(&mut self, walk: &'static str, root: u32) -> bool {
        if self.left > 0 {
            self.left -= 1;
            return true;
        }
        if !self.reported {
            self.reported = true;
            tracing::error!(
                target: "areka_emo_compose",
                walk,
                root,
                budget = self.limit,
                "入れ子の広がりが予算を越えた: まだ入っていない枝を捨てる"
            );
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Budget;
    use crate::log_capture::capture_logs;
    use crate::world::Records;

    /// 予算 3 は 3 つ目まで入り、4 つ目から断る。`error!` は範囲で 1 行だけ。
    #[test]
    fn budget_of_three_enters_three_then_refuses_with_one_error() {
        let mut entered = Vec::new();
        let out = capture_logs(|| {
            let mut budget = Budget::new(3);
            for _ in 0..5 {
                entered.push(budget.enter("plan", 7));
            }
        });
        assert_eq!(entered, [true, true, true, false, false]);
        assert_eq!(
            out.matches("level=ERROR").count(),
            1,
            "error! は 1 行: {out}"
        );
        assert!(out.contains("target=areka_emo_compose"), "{out}");
        assert!(out.contains("walk=\"plan\""), "走査の名前: {out}");
        assert!(out.contains("root=7"), "根の面: {out}");
        assert!(out.contains("budget=3"), "予算の値: {out}");
    }

    /// 黙らせる組み方では、同じく断るが記録は出さない。
    #[test]
    fn quiet_budget_refuses_without_records() {
        let mut entered = Vec::new();
        let out = capture_logs(|| {
            let mut budget = Budget::with_records(1, Records::Quiet);
            entered.push(budget.enter("hit_import", 0));
            entered.push(budget.enter("hit_import", 0));
        });
        assert_eq!(entered, [true, false]);
        assert!(out.is_empty(), "記録は出さない: {out:?}");
    }
}
