//! # Output Renderer System
//! 
//! Rich output rendering for different MIME types and data formats.

use super::*;
use std::collections::HashMap;

/// Output renderer for rich notebook outputs
#[derive(Debug)]
pub struct OutputRenderer {
    renderers: HashMap<String, Box<dyn MimeRenderer>>,
}

impl OutputRenderer {
    /// Create new output renderer
    pub fn new() -> Self {
        let mut renderer = Self {
            renderers: HashMap::new(),
        };
        
        // Register default renderers
        renderer.register_renderer("text/plain".to_string(), Box::new(TextRenderer));
        renderer.register_renderer("text/html".to_string(), Box::new(HtmlRenderer));
        renderer.register_renderer("application/json".to_string(), Box::new(JsonRenderer));
        renderer.register_renderer("image/png".to_string(), Box::new(ImageRenderer));
        renderer.register_renderer("image/jpeg".to_string(), Box::new(ImageRenderer));
        renderer.register_renderer("image/svg+xml".to_string(), Box::new(SvgRenderer));
        
        renderer
    }

    /// Register a MIME type renderer
    pub fn register_renderer(&mut self, mime_type: String, renderer: Box<dyn MimeRenderer>) {
        self.renderers.insert(mime_type, renderer);
    }

    /// Render cell output
    pub async fn render(&self, output: &CellOutput) -> Result<RenderedOutput> {
        let mut rendered_data = HashMap::new();
        
        // Render each MIME type in the output
        for (mime_type, data) in &output.data.mime_bundle {
            if let Some(renderer) = self.renderers.get(mime_type) {
                let rendered = renderer.render(data).await?;
                rendered_data.insert(mime_type.clone(), rendered);
            } else {
                // Fallback to raw data
                rendered_data.insert(mime_type.clone(), RenderResult {
                    content: data.to_string(),
                    metadata: HashMap::new(),
                });
            }
        }
        
        Ok(RenderedOutput {
            output_type: output.output_type.clone(),
            rendered_data,
            execution_count: output.execution_count,
            timestamp: output.timestamp,
        })
    }

    /// Get preferred MIME type for display
    pub fn get_preferred_mime_type(&self, available_types: &[String]) -> Option<String> {
        // Priority order for display
        let priority = vec![
            "text/html",
            "image/svg+xml",
            "image/png",
            "image/jpeg",
            "application/json",
            "text/plain",
        ];
        
        for mime_type in priority {
            if available_types.contains(&mime_type.to_string()) {
                return Some(mime_type.to_string());
            }
        }
        
        available_types.first().cloned()
    }

    /// Check if MIME type is supported
    pub fn supports_mime_type(&self, mime_type: &str) -> bool {
        self.renderers.contains_key(mime_type)
    }

    /// Get all supported MIME types
    pub fn get_supported_mime_types(&self) -> Vec<String> {
        self.renderers.keys().cloned().collect()
    }
}

/// Rendered output with all MIME types
#[derive(Debug, Clone)]
pub struct RenderedOutput {
    pub output_type: OutputType,
    pub rendered_data: HashMap<String, RenderResult>,
    pub execution_count: Option<u32>,
    pub timestamp: DateTime<Utc>,
}

/// Result of rendering a specific MIME type
#[derive(Debug, Clone)]
pub struct RenderResult {
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Trait for MIME type renderers
#[async_trait::async_trait]
pub trait MimeRenderer: Send + Sync + std::fmt::Debug {
    async fn render(&self, data: &serde_json::Value) -> Result<RenderResult>;
}

/// Text renderer for plain text
#[derive(Debug)]
pub struct TextRenderer;

#[async_trait::async_trait]
impl MimeRenderer for TextRenderer {
    async fn render(&self, data: &serde_json::Value) -> Result<RenderResult> {
        let content = match data {
            serde_json::Value::String(s) => s.clone(),
            _ => data.to_string(),
        };
        
        Ok(RenderResult {
            content,
            metadata: HashMap::new(),
        })
    }
}

/// HTML renderer
#[derive(Debug)]
pub struct HtmlRenderer;

#[async_trait::async_trait]
impl MimeRenderer for HtmlRenderer {
    async fn render(&self, data: &serde_json::Value) -> Result<RenderResult> {
        let content = match data {
            serde_json::Value::String(s) => s.clone(),
            _ => format!("<pre>{}</pre>", data.to_string()),
        };
        
        Ok(RenderResult {
            content,
            metadata: HashMap::new(),
        })
    }
}

/// JSON renderer with syntax highlighting
#[derive(Debug)]
pub struct JsonRenderer;

#[async_trait::async_trait]
impl MimeRenderer for JsonRenderer {
    async fn render(&self, data: &serde_json::Value) -> Result<RenderResult> {
        let content = serde_json::to_string_pretty(data)
            .unwrap_or_else(|_| data.to_string());
        
        Ok(RenderResult {
            content,
            metadata: HashMap::new(),
        })
    }
}

/// Image renderer for PNG/JPEG
#[derive(Debug)]
pub struct ImageRenderer;

#[async_trait::async_trait]
impl MimeRenderer for ImageRenderer {
    async fn render(&self, data: &serde_json::Value) -> Result<RenderResult> {
        let content = match data {
            serde_json::Value::String(base64_data) => {
                // Validate base64 data
                if base64_data.len() > 0 {
                    format!("data:image/png;base64,{}", base64_data)
                } else {
                    "Invalid image data".to_string()
                }
            },
            _ => "Invalid image format".to_string(),
        };
        
        Ok(RenderResult {
            content,
            metadata: HashMap::new(),
        })
    }
}

/// SVG renderer
#[derive(Debug)]
pub struct SvgRenderer;

#[async_trait::async_trait]
impl MimeRenderer for SvgRenderer {
    async fn render(&self, data: &serde_json::Value) -> Result<RenderResult> {
        let content = match data {
            serde_json::Value::String(svg_content) => {
                // Validate SVG content
                if svg_content.contains("<svg") {
                    svg_content.clone()
                } else {
                    format!("<svg>{}</svg>", svg_content)
                }
            },
            _ => "<svg><text>Invalid SVG data</text></svg>".to_string(),
        };
        
        Ok(RenderResult {
            content,
            metadata: HashMap::new(),
        })
    }
}

impl RenderedOutput {
    /// Get rendered content for a specific MIME type
    pub fn get_content(&self, mime_type: &str) -> Option<&str> {
        self.rendered_data.get(mime_type).map(|r| r.content.as_str())
    }

    /// Get the best available content for display
    pub fn get_best_content(&self) -> Option<&str> {
        // Priority order
        let priority = vec![
            "text/html",
            "image/svg+xml", 
            "image/png",
            "application/json",
            "text/plain",
        ];
        
        for mime_type in priority {
            if let Some(content) = self.get_content(mime_type) {
                return Some(content);
            }
        }
        
        // Return first available
        self.rendered_data.values().next().map(|r| r.content.as_str())
    }

    /// Get all available MIME types
    pub fn get_mime_types(&self) -> Vec<&String> {
        self.rendered_data.keys().collect()
    }

    /// Check if output has specific MIME type
    pub fn has_mime_type(&self, mime_type: &str) -> bool {
        self.rendered_data.contains_key(mime_type)
    }
}

impl Default for OutputRenderer {
    fn default() -> Self {
        Self::new()
    }
}
