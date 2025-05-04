use regex::Regex;
use tracing::info;

#[derive(Debug, Clone)]
pub struct ChunkingOptions {
    pub chunk_size: usize,
    pub overlap: usize,
    pub strategy: ChunkingStrategy,
}

#[derive(Debug, Clone)]
pub enum ChunkingStrategy {
    FixedSize,
    Sentence,
    Paragraph,
}

impl Default for ChunkingOptions {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            overlap: 100,
            strategy: ChunkingStrategy::FixedSize,
        }
    }
}

pub struct TextChunker {
    sentence_regex: Regex,
    paragraph_regex: Regex,
}

impl TextChunker {
    pub fn new() -> Self {
        Self {
            sentence_regex: Regex::new(r"[.!?]+[\s]+").unwrap(),
            paragraph_regex: Regex::new(r"\n\s*\n").unwrap(),
        }
    }

    pub fn chunk_text(&self, text: &str, options: &ChunkingOptions) -> Vec<String> {
        info!("Chunking text with strategy: {:?}", options.strategy);
        match options.strategy {
            ChunkingStrategy::FixedSize => self.chunk_by_size(text, options),
            ChunkingStrategy::Sentence => self.chunk_by_sentence(text, options),
            ChunkingStrategy::Paragraph => self.chunk_by_paragraph(text, options),
        }
    }

    fn chunk_by_size(&self, text: &str, options: &ChunkingOptions) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut start = 0;
        
        while start < text.len() {
            let end = (start + options.chunk_size).min(text.len());
            
            // Try to find a good breaking point (space or punctuation)
            let mut actual_end = end;
            if actual_end < text.len() {
                while actual_end > start && !text[actual_end..].starts_with(char::is_whitespace) {
                    actual_end -= 1;
                }
                if actual_end == start {
                    actual_end = end; // If no good break point found, use the original end
                }
            }
            
            let chunk = text[start..actual_end].trim().to_string();
            if !chunk.is_empty() {
                chunks.push(chunk);
            }
            
            start = actual_end.saturating_sub(options.overlap);
        }
        
        chunks
    }

    fn chunk_by_sentence(&self, text: &str, options: &ChunkingOptions) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        for sentence in self.sentence_regex.split(text) {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }
            
            if current_chunk.len() + sentence.len() + 1 <= options.chunk_size {
                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
                current_chunk.push_str(sentence);
            } else {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                }
                current_chunk = sentence.to_string();
            }
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        // Add overlap between chunks
        if options.overlap > 0 && chunks.len() > 1 {
            let mut overlapped_chunks = Vec::with_capacity(chunks.len());
            overlapped_chunks.push(chunks[0].clone());
            
            for i in 1..chunks.len() {
                let prev_chunk = &chunks[i-1];
                let current_chunk = &chunks[i];
                
                // Find the last sentence in the previous chunk
                let last_sentences: Vec<&str> = self.sentence_regex
                    .split(prev_chunk)
                    .filter(|s| !s.trim().is_empty())
                    .collect();
                
                if let Some(last_sentence) = last_sentences.last() {
                    let mut new_chunk = last_sentence.trim().to_string();
                    new_chunk.push(' ');
                    new_chunk.push_str(current_chunk);
                    overlapped_chunks.push(new_chunk);
                } else {
                    overlapped_chunks.push(current_chunk.clone());
                }
            }
            
            chunks = overlapped_chunks;
        }
        
        chunks
    }

    fn chunk_by_paragraph(&self, text: &str, options: &ChunkingOptions) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        for paragraph in self.paragraph_regex.split(text) {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }
            
            if current_chunk.len() + paragraph.len() + 2 <= options.chunk_size {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(paragraph);
            } else {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                }
                current_chunk = paragraph.to_string();
            }
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        // Add overlap between chunks
        if options.overlap > 0 && chunks.len() > 1 {
            let mut overlapped_chunks = Vec::with_capacity(chunks.len());
            overlapped_chunks.push(chunks[0].clone());
            
            for i in 1..chunks.len() {
                let prev_chunk = &chunks[i-1];
                let current_chunk = &chunks[i];
                
                // Find the last paragraph in the previous chunk
                let last_paragraphs: Vec<&str> = self.paragraph_regex
                    .split(prev_chunk)
                    .filter(|p| !p.trim().is_empty())
                    .collect();
                
                if let Some(last_paragraph) = last_paragraphs.last() {
                    let mut new_chunk = last_paragraph.trim().to_string();
                    new_chunk.push_str("\n\n");
                    new_chunk.push_str(current_chunk);
                    overlapped_chunks.push(new_chunk);
                } else {
                    overlapped_chunks.push(current_chunk.clone());
                }
            }
            
            chunks = overlapped_chunks;
        }
        
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_size_chunking() {
        let chunker = TextChunker::new();
        let text = "This is a test. This is another test. This is a third test.";
        let options = ChunkingOptions {
            chunk_size: 20,
            overlap: 5,
            strategy: ChunkingStrategy::FixedSize,
        };
        
        let chunks = chunker.chunk_text(text, &options);
        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].len() <= 20);
        assert!(chunks[1].len() <= 20);
        assert!(chunks[2].len() <= 20);
    }

    #[test]
    fn test_sentence_chunking() {
        let chunker = TextChunker::new();
        let text = "This is a test. This is another test. This is a third test.";
        let options = ChunkingOptions {
            chunk_size: 30,
            overlap: 10,
            strategy: ChunkingStrategy::Sentence,
        };
        
        let chunks = chunker.chunk_text(text, &options);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains("This is a test"));
        assert!(chunks[1].contains("This is a third test"));
    }

    #[test]
    fn test_paragraph_chunking() {
        let chunker = TextChunker::new();
        let text = "This is a test.\n\nThis is another test.\n\nThis is a third test.";
        let options = ChunkingOptions {
            chunk_size: 30,
            overlap: 10,
            strategy: ChunkingStrategy::Paragraph,
        };
        
        let chunks = chunker.chunk_text(text, &options);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains("This is a test"));
        assert!(chunks[1].contains("This is a third test"));
    }
}