#[derive(Debug)]
pub struct QueueFamily {
    pub index: u32,
    pub priorities: Vec<f32>,
}

impl QueueFamily {
    pub(crate) fn merge_queues(a: &mut Vec<QueueFamily>) {
        // If the queue families are the same, merge them
        let mut i = 0;
        while i < a.len() {
            let mut j = i + 1;
            while j < a.len() {
                if a[i].index == a[j].index {
                    // TODO: Merge priorities
                    a.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
}
