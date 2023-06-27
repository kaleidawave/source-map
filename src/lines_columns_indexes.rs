/// Contains the byte indexes of when line starts
#[derive(Clone, Debug)]
pub struct LineStarts(pub(crate) Vec<usize>);

impl LineStarts {
    /// Implementation copied from [codespan-reporting](https://docs.rs/codespan-reporting/0.11.1/codespan_reporting/)
    pub fn new(source: &str) -> LineStarts {
        Self(
            std::iter::once(0)
                .chain(source.match_indices('\n').map(|(i, _)| i + 1))
                .collect(),
        )
    }

    pub fn byte_indexes_on_same_line(&self, pos1: usize, pos2: usize) -> bool {
        debug_assert!(pos1 <= pos2);
        self.0
            .windows(2)
            .find_map(|w| {
                let range = w[0]..=w[1];
                range.contains(&pos1).then_some(range)
            })
            .expect("could not find splits for pos1")
            .contains(&pos2)
    }

    pub fn byte_indexes_crosses_lines(&self, pos1: usize, pos2: usize) -> usize {
        debug_assert!(pos1 <= pos2);
        let first_line_backwards = self.get_index_of_line_pos_is_on(pos1);
        let second_line_backwards = self.get_index_of_line_pos_is_on(pos2);
        second_line_backwards - first_line_backwards
    }

    pub fn byte_indexes_on_different_lines(&self, pos1: usize, pos2: usize) -> bool {
        self.byte_indexes_crosses_lines(pos1, pos2) > 0
    }

    pub(crate) fn get_index_of_line_pos_is_on(&self, pos: usize) -> usize {
        let backwards_index = self
            .0
            .iter()
            .rev()
            .position(|index| pos >= *index)
            .expect("pos1 out of bounds");

        (self.0.len() - 1) - backwards_index
    }
}

#[cfg(test)]
mod tests {
    use super::LineStarts;

    fn get_source() -> String {
        std::fs::read_to_string("README.md").expect("No README")
    }

    #[test]
    fn split_lines() {
        let source = get_source();

        let line_starts = LineStarts::new(&source);
        let expected_lines = source.lines().collect::<Vec<_>>();
        let mut actual_lines = Vec::new();

        let mut iterator = line_starts.0.into_iter();
        let mut last = iterator.next().unwrap();
        for part in iterator {
            let value = &source[last..part];
            let value = value.strip_suffix("\n").unwrap();
            let value = value.strip_suffix("\r").unwrap_or(value);
            actual_lines.push(value);
            last = part;
        }

        assert_eq!(expected_lines, actual_lines);
    }

    #[test]
    fn byte_indexes_crosses_lines() {
        let source = get_source();

        let line_starts = LineStarts::new(&source);

        let start = 100;
        let end = 200;
        let lines_in_between = source[start..end].chars().filter(|c| *c == '\n').count();

        assert_eq!(
            line_starts.byte_indexes_crosses_lines(start, end),
            lines_in_between
        );
    }

    #[test]
    fn byte_indexes_on_same_line() {
        let source = get_source();

        let line_starts = LineStarts::new(&source);

        let start = 100;
        let end = start
            + source[start..]
                .chars()
                .take_while(|c| *c == '\n')
                .map(|c| c.len_utf16())
                .sum::<usize>();

        assert!(line_starts.byte_indexes_on_same_line(start, end));
    }
}
