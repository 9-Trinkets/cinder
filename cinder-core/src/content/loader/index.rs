use std::collections::HashMap;

pub fn build_index<T, F>(items: &[T], id: F) -> HashMap<String, usize>
where
    F: Fn(&T) -> &str,
{
    items
        .iter()
        .enumerate()
        .map(|(index, item)| (id(item).to_string(), index))
        .collect()
}
