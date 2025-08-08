use tree_sitter::{Language, Parser, Tree, Node};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

use crate::ParseResponse;

extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_javascript() -> Language;
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_python() -> Language;
}

pub struct CustomParser {
    parsers: HashMap<String, Parser>,
    languages: HashMap<String, Language>,
}

impl CustomParser {
    pub fn new() -> Result<Self> {
        let mut parser = Self {
            parsers: HashMap::new(),
            languages: HashMap::new(),
        };

        // Initialize supported languages
        unsafe {
            parser.languages.insert("rust".to_string(), tree_sitter_rust());
            parser.languages.insert("javascript".to_string(), tree_sitter_javascript());
            parser.languages.insert("typescript".to_string(), tree_sitter_typescript());
            parser.languages.insert("python".to_string(), tree_sitter_python());
        }

        // Create parsers for each language
        for (lang_name, language) in &parser.languages {
            let mut tree_parser = Parser::new();
            tree_parser.set_language(language.clone())?;
            parser.parsers.insert(lang_name.clone(), tree_parser);
        }

        Ok(parser)
    }

    pub fn parse(&mut self, content: &str, language: &str) -> Result<ParseResponse> {
        let parser = self.parsers.get_mut(language)
            .ok_or_else(|| anyhow!("Unsupported language: {}", language))?;

        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse content"))?;

        let root_node = tree.root_node();
        let ast = self.node_to_string(&root_node, content, 0);
        let symbols = self.extract_symbols(&root_node, content, language);
        let errors = self.extract_errors(&tree);

        Ok(ParseResponse {
            ast,
            symbols,
            errors,
        })
    }

    fn node_to_string(&self, node: &Node, source: &str, depth: usize) -> String {
        let indent = "  ".repeat(depth);
        let mut result = format!("{}{}[{}:{}]", 
            indent, 
            node.kind(), 
            node.start_position().row, 
            node.start_position().column
        );

        if node.child_count() == 0 {
            // Leaf node - include text content
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if !text.trim().is_empty() && text.len() < 50 {
                    result.push_str(&format!(" \"{}\"", text.replace('\n', "\\n")));
                }
            }
        }

        result.push('\n');

        // Add children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                result.push_str(&self.node_to_string(&child, source, depth + 1));
            }
        }

        result
    }

    fn extract_symbols(&self, node: &Node, source: &str, language: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        self.collect_symbols(node, source, language, &mut symbols);
        symbols
    }

    fn collect_symbols(&self, node: &Node, source: &str, language: &str, symbols: &mut Vec<String>) {
        match language {
            "rust" => {
                match node.kind() {
                    "function_item" | "impl_item" | "struct_item" | "enum_item" | "trait_item" => {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                symbols.push(format!("{}:{}", node.kind(), name));
                            }
                        }
                    }
                    _ => {}
                }
            }
            "javascript" | "typescript" => {
                match node.kind() {
                    "function_declaration" | "class_declaration" | "interface_declaration" => {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                symbols.push(format!("{}:{}", node.kind(), name));
                            }
                        }
                    }
                    _ => {}
                }
            }
            "python" => {
                match node.kind() {
                    "function_definition" | "class_definition" => {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                symbols.push(format!("{}:{}", node.kind(), name));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_symbols(&child, source, language, symbols);
            }
        }
    }

    fn extract_errors(&self, tree: &Tree) -> Vec<String> {
        let mut errors = Vec::new();
        let root = tree.root_node();
        self.collect_errors(&root, &mut errors);
        errors
    }

    fn collect_errors(&self, node: &Node, errors: &mut Vec<String>) {
        if node.is_error() {
            errors.push(format!("Parse error at {}:{}", 
                node.start_position().row, 
                node.start_position().column
            ));
        }

        if node.is_missing() {
            errors.push(format!("Missing node at {}:{}", 
                node.start_position().row, 
                node.start_position().column
            ));
        }

        // Check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_errors(&child, errors);
            }
        }
    }
}

// Global parser instance
static mut PARSER: Option<CustomParser> = None;

pub fn parse_content(content: &str, language: &str) -> Result<ParseResponse> {
    unsafe {
        if PARSER.is_none() {
            PARSER = Some(CustomParser::new()?);
        }

        let parser = PARSER.as_mut().unwrap();
        parser.parse(content, language)
    }
}

/// AST optimization for better parsing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ASTOptimization {
    pub enabled: bool,
    pub level: u8,
}

/// Lexical enhancement for better token analysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LexicalEnhancement {
    pub enabled: bool,
    pub features: Vec<String>,
}
