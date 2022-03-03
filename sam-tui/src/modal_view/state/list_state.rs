use fzy_rs::has_match;

use crate::modal_view::state::Value;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct ListState<V: Value> {
    pub filter_query: ListFilter,
    values: Vec<V>,
    marked_values: HashSet<V>,
    pub current_displayed_values: Vec<V>,
    pub highlighted_line: Option<usize>,
}

impl<V: Value> ListState<V> {
    pub fn new(list: Vec<V>) -> Self {
        let cursor = list.first().map(|_| 0);
        ListState::<V> {
            values: list.clone(),
            current_displayed_values: list,
            marked_values: HashSet::default(),
            highlighted_line: cursor,
            filter_query: ListFilter::default(),
        }
    }
    pub fn displayed_values(&self) -> Vec<(bool, &V)> {
        self.current_displayed_values
            .iter()
            .map(|v| (self.marked_values.contains(v), v))
            .collect()
    }

    pub fn up(&mut self) {
        self.highlighted_line =
            self.highlighted_line
                .map(|cursor| if cursor > 0 { cursor - 1 } else { cursor });
    }

    pub fn down(&mut self) {
        self.highlighted_line = self.highlighted_line.map(|cursor| {
            if cursor < self.current_displayed_values.len() - 1 {
                cursor + 1
            } else {
                cursor
            }
        });
    }

    pub fn mark(&mut self) -> Option<bool> {
        let value = self
            .highlighted_line
            .and_then(|cursor| self.current_displayed_values.get(cursor));

        if let Some(v) = value {
            if self.marked_values.contains(v) {
                self.marked_values.remove(v);
                Some(false)
            } else {
                self.marked_values.insert(v.clone());
                Some(true)
            }
        } else {
            None
        }
    }

    pub fn mark_all(&mut self) {
        for value in &self.current_displayed_values {
            if !self.marked_values.contains(&value) {
                self.marked_values.insert(value.clone());
            }
        }
    }

    pub fn entr(&mut self) -> Option<bool> {
        let value = self
            .highlighted_line
            .and_then(|cursor| self.current_displayed_values.get(cursor));

        if let Some(v) = value {
            if self.marked_values.contains(v) {
                Some(false)
            } else {
                self.marked_values.insert(v.clone());
                Some(true)
            }
        } else {
            None
        }
    }

    pub fn update_filter(&mut self, c: char) {
        self.filter_query.push_back(c);
        self.update_display_and_highlight()
    }

    pub fn remove_last_char_from_filter(&mut self) {
        self.filter_query.pop();
        self.update_display_and_highlight()
    }

    pub fn search_filter(&self) -> &str {
        self.filter_query.as_ref()
    }

    fn filtered_view(&self) -> Vec<V> {
        let mut filters = Vec::with_capacity(self.values.len());
        let pat = self.filter_query.as_ref().as_bytes();
        for v in &self.values {
            let text = v.text().as_bytes();
            if has_match(pat, text) {
                filters.push(v.clone());
            }
        }
        filters
    }

    pub fn marked_values(self) -> HashSet<V> {
        self.marked_values
    }

    fn update_display_and_highlight(&mut self) {
        self.current_displayed_values = self.filtered_view();
        self.highlighted_line = if let Some(cursor) = self.highlighted_line {
            if cursor >= self.current_displayed_values.len() {
                if self.current_displayed_values.len() > 0 {
                    Some(0)
                } else {
                    None
                }
            } else {
                Some(cursor)
            }
        } else {
            if self.current_displayed_values.len() > 0 {
                Some(0)
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ListFilter(String);

impl AsRef<str> for ListFilter {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl ListFilter {
    pub fn push_back(&mut self, c: char) {
        self.0.push(c)
    }
    pub fn pop(&mut self) {
        self.0.pop();
    }
    pub fn take(self) -> String {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::modal_view::state::mocks::MockValue;

    use super::ListState;

    #[test]
    fn test_navigation() {
        let mut list = ListState::<MockValue>::new(vec![
            MockValue::new(1, "one"),
            MockValue::new(2, "two"),
            MockValue::new(3, "three"),
            MockValue::new(4, "four"),
            MockValue::new(5, "five"),
        ]);
        list.up();
        list.down();
        list.down();
        list.up();
        list.mark();
        list.update_filter('o');
        list.down();
        list.mark();
        assert!(list.marked_values.contains(&MockValue::new(2, "two")));
        assert!(list.marked_values.contains(&MockValue::new(4, "four")));
    }
    #[test]
    fn test_marks() {}
}
