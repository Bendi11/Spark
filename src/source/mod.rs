use std::ops::Range;

use slotmap::DenseSlotMap;

pub mod span;

slotmap::new_key_type! {
    /// ID used to access file data in a [SourceFiles] collection
    pub struct FileId;
}

/// A map of [FileId]s to file data, used for displaying diagnostics using span data
pub struct SourceFiles {
    map: DenseSlotMap<FileId, FileData>,
}

/// In-memory structure containing the full contents of a source file, with amortization data to
/// speed location queries for use with diagnostics and metadata tracking the file source
struct FileData {
    name: String,
    text: String,
    lines: Vec<usize>,
}

impl FileData {
    /// Get the line number of the given byte position in the file
    fn line_of_offset(&self, offset: usize) -> usize {
        self.upper_bound_offset(offset).1
    }
    
    /// Get the byte range for the text contained in a given line number
    fn range_of_line(&self, line: usize) -> Result<Range<usize>, FileDataQueryError> {
        let start = *self.lines.get(line).ok_or_else(|| FileDataQueryError::LineTooLarge {
            err: line,
            max: self.line_max(),
        })?;
        
        let end = self.lines.get(line + 1).copied().unwrap_or(self.text.len());

        Ok(
            Range {
                start,
                end,
            }
        )
    }
    
    /// Get the column number of the given byte offset in the file
    fn column_of_offset(&self, offset: usize) -> usize {
        offset - self.upper_bound_offset(offset).0
    }
    
    /// Get the first line entry less than or equal to the given byte position.
    /// Returns (byte position of newline, line number)
    fn upper_bound_offset(&self, offset: usize) -> (usize, usize) {
        match self.lines.binary_search(&offset) {
            Ok(line) => (offset, line),
            Err(pos) => {
                let line = pos - 1;
                (self.lines[line], line)
            }
        }
    }
    
    /// Get the last line number of this file
    fn line_max(&self) -> usize {
        self.lines.len() - 1
    }
    
    /// Create a new in-memory file from the given file name and contents
    fn new(name: String, text: String) -> Self {
        let first_line = std::iter::once(0);
        let newlines = text
            .char_indices()
            .filter_map(|(idx, ch)| (ch == '\n').then_some(idx));

        let lines = first_line
            .chain(newlines)
            .collect();

        Self {
            name,
            text,
            lines,
        }
    }
}

impl<'a> codespan_reporting::files::Files<'a> for &'a SourceFiles {
    type FileId = FileId;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        Ok(&self.map[id].name)
    }

    fn source(&'a self, id: Self::FileId) -> Result<Self::Source, codespan_reporting::files::Error> {
        Ok(&self.map[id].text)
    }

    fn line_index(&'a self, id: Self::FileId, byte_index: usize) -> Result<usize, codespan_reporting::files::Error> {
        Ok(self.map[id].line_of_offset(byte_index))
    }

    fn line_range(&'a self, id: Self::FileId, line_index: usize) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
        self.map[id].range_of_line(line_index).map_err(Into::into)
    }
}

#[derive(Debug, thiserror::Error)]
enum FileDataQueryError {
    #[error("Line number {} too large for file with {} lines", err, max)]
    LineTooLarge {
        max: usize,
        err: usize,
    },
}

impl From<FileDataQueryError> for codespan_reporting::files::Error {
    fn from(value: FileDataQueryError) -> Self {
        match value {
            FileDataQueryError::LineTooLarge { err, max } => Self::LineTooLarge { given: err, max, },
        }
    }
}
