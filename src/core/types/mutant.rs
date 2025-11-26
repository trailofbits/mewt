use console::style;
use similar::{ChangeTag, TextDiff};

use crate::types::Target;
use crate::types::config::colors_enabled;

#[derive(Debug, Clone)]
pub struct PartialMutant {
    pub byte_offset: u32,
    pub line_offset: u32,
    pub old_text: String,
    pub new_text: String,
}

#[derive(Debug, Clone)]
pub struct Mutant {
    pub id: i64,
    pub target_id: i64,
    pub byte_offset: u32,
    pub line_offset: u32,
    pub old_text: String,
    pub new_text: String,
    pub mutation_slug: String,
}

impl Mutant {
    pub fn from_partial(partial: PartialMutant, target: &Target, slug: &str) -> Self {
        Self {
            id: 0,
            target_id: target.id,
            byte_offset: partial.byte_offset,
            line_offset: partial.line_offset,
            old_text: partial.old_text,
            new_text: partial.new_text,
            mutation_slug: slug.to_string(),
        }
    }

    pub fn get_lines(&self) -> (u32, u32) {
        // Count newline characters without trimming so we capture true span
        let newline_count = self.old_text.bytes().filter(|&b| b == b'\n').count() as u32;
        // Convert stored 0-based line_offset to 1-based for display
        let start_line_one_based = self.line_offset + 1;
        (start_line_one_based, start_line_one_based + newline_count)
    }

    /// Formats a mutant for display
    /// - Includes line number or range information
    /// - Shows the full line(s) before and after the mutation
    /// - Collapses indentation from newlines into single spaces
    /// - Replaces newlines with the literal string "\n"
    /// - Highlights removed parts in red and added parts in green
    pub fn display(&self, target: &Target) -> String {
        // Extract the full line(s) from the target's source text
        let lines = self.get_lines();
        let source_lines: Vec<&str> = target.text.lines().collect();
        let mut original_full_lines = String::new();
        for line_num in lines.0..=lines.1 {
            if let Some(line) = source_lines.get((line_num - 1) as usize) {
                if !original_full_lines.is_empty() {
                    original_full_lines.push('\n');
                }
                original_full_lines.push_str(line);
            }
        }

        // Apply the mutation using the precise byte-level method
        let mutated_full_lines = if let Ok(mutated_content) = target.mutate(self) {
            // Find the corresponding text in the mutated content
            // Calculate byte position of the start of the line range
            let line_start_byte = target
                .text
                .lines()
                .take((lines.0 - 1) as usize)
                .map(|line| line.len() + 1) // +1 for newline
                .sum::<usize>();

            // Calculate how the mutation changed the byte positions
            let mutation_byte_change = self.new_text.len() as i64 - self.old_text.len() as i64;
            let mutation_start = self.byte_offset as usize;

            // Find the end position of the original lines
            let original_line_end_byte = if lines.1 >= target.text.lines().count() as u32 {
                target.text.len()
            } else {
                target
                    .text
                    .lines()
                    .take(lines.1 as usize)
                    .map(|line| line.len() + 1)
                    .sum::<usize>()
                    .saturating_sub(1) // Remove final newline
            };

            // Calculate the new end position in the mutated content
            let mutated_line_end_byte = if mutation_start >= original_line_end_byte {
                // Mutation is after our lines, no change to our range
                original_line_end_byte
            } else if mutation_start < line_start_byte {
                // Mutation is before our lines, adjust the entire range
                (original_line_end_byte as i64 + mutation_byte_change) as usize
            } else {
                // Mutation is within our lines, adjust from mutation point
                (original_line_end_byte as i64 + mutation_byte_change) as usize
            };

            let mutated_bytes = mutated_content.as_bytes();
            if line_start_byte < mutated_bytes.len() && mutated_line_end_byte <= mutated_bytes.len()
            {
                String::from_utf8_lossy(&mutated_bytes[line_start_byte..mutated_line_end_byte])
                    .to_string()
            } else {
                // Fallback if byte calculations fail
                original_full_lines.replace(&self.old_text, &self.new_text)
            }
        } else {
            // Fallback if mutation fails
            original_full_lines.replace(&self.old_text, &self.new_text)
        };

        // Function to format text: collapse indentation to single spaces and escape newlines
        let format_text = |text: &str| {
            text.trim()
                .lines()
                .map(|line| {
                    // Replace leading whitespace with a single space if the line isn't empty
                    if line.trim().is_empty() {
                        line.to_string()
                    } else {
                        let trimmed = line.trim_start();
                        if trimmed == line {
                            line.to_string()
                        } else {
                            format!(" {trimmed}")
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("\\n")
        };

        let formatted_original = format_text(&original_full_lines);
        let formatted_mutated = format_text(&mutated_full_lines);

        // Always use word diff
        let diff = TextDiff::configure()
            .algorithm(similar::Algorithm::Patience)
            .timeout(std::time::Duration::from_millis(100))
            .diff_unicode_words(&formatted_original, &formatted_mutated);

        // Format the diff; optionally disable colors
        let colors_enabled = colors_enabled();
        let mut original_highlighted = String::new();
        let mut mutated_highlighted = String::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    if colors_enabled {
                        original_highlighted.push_str(&style(change.value()).red().to_string());
                    } else {
                        original_highlighted.push_str(change.value());
                    }
                }
                ChangeTag::Insert => {
                    if colors_enabled {
                        mutated_highlighted.push_str(&style(change.value()).green().to_string());
                    } else {
                        mutated_highlighted.push_str(change.value());
                    }
                }
                ChangeTag::Equal => {
                    original_highlighted.push_str(change.value());
                    mutated_highlighted.push_str(change.value());
                }
            }
        }

        let line_display = if lines.0 == lines.1 {
            format!("Line {}", lines.0)
        } else {
            format!("Lines {}-{}", lines.0, lines.1)
        };
        format!(
            "[{} {}] {}: '{}' -> '{}'",
            self.mutation_slug, self.id, line_display, original_highlighted, mutated_highlighted
        )
    }
}
