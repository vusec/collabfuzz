use super::interface::{dfsan_get_label_info, dfsan_label};
use std::collections::BTreeSet;

pub fn dfsan_get_base_labels(root_label: dfsan_label) -> BTreeSet<dfsan_label> {
    let mut base_labels = BTreeSet::new();

    let mut seen_labels = BTreeSet::new();
    seen_labels.insert(root_label);

    let mut dag_visit_stack = vec![root_label];
    while !dag_visit_stack.is_empty() {
        let label = dag_visit_stack.pop().unwrap();

        unsafe {
            let label_info = dfsan_get_label_info(label);
            let l1 = (*label_info).l1;
            let l2 = (*label_info).l2;

            if l1 == 0 && l2 == 0 {
                // This is a base label
                base_labels.insert(label);
                continue;
            }

            assert!(l1 != 0 && l2 != 0);

            if !seen_labels.contains(&l1) {
                dag_visit_stack.push(l1);
                seen_labels.insert(l1);
            }
            if !seen_labels.contains(&l2) {
                dag_visit_stack.push(l2);
                seen_labels.insert(l2);
            }
        }
    }

    base_labels
}
