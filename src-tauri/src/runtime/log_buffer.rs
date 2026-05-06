use std::collections::VecDeque;

use crate::models::RuntimeLogLine;

#[derive(Debug, Clone)]
pub struct LogBuffer {
    capacity: usize,
    lines: VecDeque<RuntimeLogLine>,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            lines: VecDeque::with_capacity(capacity),
        }
    }

    pub fn append(&mut self, line: RuntimeLogLine) {
        if self.capacity == 0 {
            return;
        }

        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }

        self.lines.push_back(line);
    }

    pub fn recent(&self) -> Vec<RuntimeLogLine> {
        self.lines.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{RuntimeLogLine, RuntimeLogStream};

    use super::LogBuffer;

    #[test]
    fn keeps_only_the_most_recent_lines_when_capacity_is_reached() {
        let mut buffer = LogBuffer::new(2);
        buffer.append(sample_line("one"));
        buffer.append(sample_line("two"));
        buffer.append(sample_line("three"));

        let lines = buffer.recent();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line, "two");
        assert_eq!(lines[1].line, "three");
    }

    fn sample_line(line: &str) -> RuntimeLogLine {
        RuntimeLogLine {
            project_id: "project".into(),
            stream: RuntimeLogStream::Stdout,
            line: line.into(),
            partial: false,
            timestamp: "2026-04-16T12:00:00Z".into(),
        }
    }
}
